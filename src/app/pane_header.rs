//! When a pane has a header and what its trailing buttons do: Split right, Split down, Zoom
//! and Close, each on the pane the header belongs to, focused or not.
//!
//! Settings → Interface → Pane headers decides where the header is. Auto follows the density
//! (every pane in Comfortable, a lone one included, which stays edge to edge under its
//! header; never in Compact), Always puts one on every pane, On hover floats it over the top
//! of the pane the pointer rests on, and Hidden draws none. A header that floats never moves the terminal
//! under it: the grid keeps its rows, so nothing is resized on the server.
use super::{
    panel::{Intent, PaneAction},
    splits::SplitWay,
    theme::*,
    widgets::glyph,
    workspace::Workspace,
};
use rex_rune::{
    layout,
    settings::{PaneDensity, PaneHeader, interface::ToolbarBehavior},
};
use rexrs::api::session::Window;
use rune::{Cursor, El, Icon, SpringPair, Transition, Tx, el, row, text};
use serde_json::{Value, json};
use std::time::Duration;

/// Floats the header on every pane that has no row, until it is run again.
pub const TOGGLE: &str = "client.pane_headers.toggle";

/// A header button's square, and the icon in it.
pub const BUTTON: f32 = 24.;
const ICON: f32 = 14.;
/// The band along a pane's top edge where the pointer brings an On hover header out.
pub const REVEAL: f32 = 36.;
/// How long the pointer rests in the band before the header shows, and how long after it left
/// the header stays. A pass on the way to the toolbar shows nothing.
const REVEAL_AFTER: Duration = Duration::from_millis(90);
const HIDE_AFTER: Duration = Duration::from_millis(220);
/// An action that cannot run now keeps its place at this opacity.
pub const UNAVAILABLE: f32 = 0.35;
/// Below this width a header keeps Zoom and Close only; the pane menu has the splits.
const ROOMY: f32 = 200.;

/// Gentle extra fading, not the disabled-button cue. Hover restores enabled icons.
pub const FADE: f32 = 0.85;

/// Only the name and ordinary enabled icons use this; never a header/container opacity.
pub fn header_opacity(s: &Workspace, block: &str) -> f32 {
    let i = &s.settings.interface;
    if (i.fade_inactive_pane_headers && s.focused_block().as_deref() != Some(block))
        || (i.fade_pane_headers_in_inactive_window && !s.window_focused)
    {
        FADE
    } else {
        1.
    }
}

/// Headers brought out without a setting change. Never saved.
#[derive(Default)]
pub struct Reveal {
    /// The pane whose top edge the pointer rests on, in On hover.
    pub hover: Option<String>,
    /// Reveal pane headers is on: every pane without a header row floats one.
    pub pinned: bool,
}

/// How a pane's header is drawn now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    None,
    /// A row above the terminal.
    Row,
    /// Over the terminal's top edge.
    Floating,
}

/// One pane and nothing zoomed.
fn single(window: &Window) -> bool {
    window.zoomed_block_id.is_none() && layout::placements(window, None).0.len() == 1
}

/// Whether Zoom has anything to do in this tab: it zooms one pane of several, or restores
/// the zoomed one. The only pane of a tab already has the whole of it. The header's button,
/// the pane menu and Commands all follow this.
pub fn can_zoom(window: &Window) -> bool {
    !single(window)
}

/// Whether `window` holds the last pane of the session on screen: one tab, with one pane.
/// Its header's Close is not offered, as in Rex for macOS; the pane menu, Commands and the
/// tab's own menu still close it.
fn last_pane(s: &Workspace, window: &Window) -> bool {
    single(window) && s.model().view().is_some_and(|v| v.windows.len() == 1)
}

/// Whether this tab's panes carry a header row above their terminal. The tab no longer
/// matters (a lone pane has one too); it stays in the signature for the callers that have it.
pub fn rows(s: &Workspace, _window: &Window) -> bool {
    match s.settings.pane_header {
        PaneHeader::Always => true,
        // A lone pane has one too, as in Rex for macOS: Split right and Split down are
        // where the pointer expects them before there is anything to split from.
        PaneHeader::Auto => s.settings.pane_density == PaneDensity::Comfortable,
        PaneHeader::OnHover | PaneHeader::Hidden => false,
    }
}

/// A tab with one pane, not zoomed, draws it edge to edge: no card or margin. Its header, if
/// it has one, is a row across the top of it. With headers always shown it is a card like any
/// other.
pub fn full_bleed(s: &Workspace, window: &Window) -> bool {
    single(window) && s.settings.pane_header != PaneHeader::Always
}

/// Room kept above a lone pane's header row while the toolbar has no row of its own. The
/// header is then the top of the window, and the toolbar's reveal edge (and the window's
/// resize edge within it) must stay clear of its buttons: a pointer resting on Close would
/// otherwise bring the toolbar out over it, with Close app where Close pane was.
pub fn clearance(s: &Workspace, window: &Window) -> f32 {
    if full_bleed(s, window)
        && rows(s, window)
        && s.settings.interface.toolbar != ToolbarBehavior::Always
    {
        super::toolbar_reveal::EDGE
    } else {
        0.
    }
}

pub fn placement(s: &Workspace, window: &Window, block: &str) -> Placement {
    if rows(s, window) {
        Placement::Row
    } else if s.header_reveal.pinned
        || s.settings.pane_header == PaneHeader::OnHover
            && s.header_reveal.hover.as_deref() == Some(block)
    {
        Placement::Floating
    } else {
        Placement::None
    }
}

/// Whether Reveal pane headers has anything to bring out: some pane, here or in another
/// tab, can be without a header row.
pub fn can_reveal(s: &Workspace) -> bool {
    match s.settings.pane_header {
        PaneHeader::Always => false,
        // In Comfortable every pane already has its row.
        PaneHeader::Auto => s.settings.pane_density != PaneDensity::Comfortable,
        PaneHeader::OnHover | PaneHeader::Hidden => true,
    }
}

impl Workspace {
    /// Reveal pane headers, and the same again to put them back.
    pub fn toggle_pane_headers(&mut self) {
        self.header_reveal.pinned = !self.header_reveal.pinned && can_reveal(self);
    }
}

fn zoomed(window: &Window, block: &str) -> bool {
    window.zoomed_block_id.as_deref() == Some(block)
}

/// The quiet word beside the name of a pane that fills its tab while the others wait.
pub fn zoomed_chip(window: &Window, block: &str) -> Option<El> {
    zoomed(window, block).then(|| {
        el().key(format!("pane-zoomed:{block}"))
            .h(18.)
            .px(7.)
            .center()
            .radius(9.)
            .fill(white(KEYCAP_FILL))
            .help("The other panes of this tab are hidden until this one is restored.")
            .child(text("Zoomed").font(medium(10.5)).color(rgb(keycap_ink())))
    })
}

/// A header button: no fill at rest, a rounded one under the pointer, which also lights the
/// icon. It never keeps Rune focus, so the next key still reaches the terminal. With no
/// `action` it cannot run now: it stays in its place, dim, and says why.
fn button(
    key: String,
    icon: &'static Icon,
    label: &str,
    help: String,
    tone: (rune::Color, f32),
    destructive: bool,
    action: Option<(String, PaneAction)>,
) -> El {
    let (rest, fade) = tone;
    let b = row()
        .key(key)
        .role("button")
        .label(label.to_owned())
        .size(BUTTON, BUTTON)
        .center()
        .radius(6.)
        .continuous_corners()
        .help(help);
    let Some((block, action)) = action else {
        return b.child(glyph(icon, ICON, rest).opacity(UNAVAILABLE));
    };
    let (hover, press) = if destructive {
        (rgb(danger()).alpha(0.22), rgb(danger()).alpha(0.3))
    } else {
        (white(0.09), white(0.13))
    };
    b.cursor(Cursor::Pointer)
        .hover(hover)
        .press(press)
        .preserve_focus_on_press(true)
        .tab_index(-1)
        .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| {
            if let Some(target) = s.target(&block) {
                s.perform(Intent::Pane(target, action), tx);
            }
            tx.blur();
        })
        .child(glyph(icon, ICON, rest.alpha(rest.a * fade)).group_color(rgb(ink())))
}

/// The buttons at a header's trailing end. `lit` is the focused pane's header.
pub fn actions(s: &Workspace, window: &Window, block: &str, width: f32, lit: bool) -> El {
    let rest = rgb(muted()).alpha(if lit { 1. } else { 0.7 });
    let fade = header_opacity(s, block);
    let hint = |label: &str, action: &str, args: Value| {
        let keys = s.shortcut_detail(action, &args);
        if keys.is_empty() {
            label.to_owned()
        } else {
            format!("{label} · {keys}")
        }
    };
    let on = |action: PaneAction| Some((block.to_owned(), action));
    let mut cluster = row().key(format!("pane-actions:{block}")).center_y();
    if width >= ROOMY {
        cluster = cluster
            .child(button(
                format!("pane-split-right:{block}"),
                &SPLIT_RIGHT,
                "Split right",
                hint("Split right", "pane.split", json!({"direction": "right"})),
                (rest, fade),
                false,
                on(PaneAction::Split(SplitWay::Right)),
            ))
            .child(button(
                format!("pane-split-down:{block}"),
                &SPLIT_DOWN,
                "Split down",
                hint("Split down", "pane.split", json!({"direction": "down"})),
                (rest, fade),
                false,
                on(PaneAction::Split(SplitWay::Down)),
            ));
    }
    let zoom = if zoomed(window, block) {
        button(
            format!("pane-zoom:{block}"),
            &RESTORE,
            "Restore pane",
            hint("Restore pane", "pane.zoom", Value::Null),
            (rest, fade),
            false,
            on(PaneAction::Zoom),
        )
    } else if !can_zoom(window) {
        // The only pane already has the whole tab.
        button(
            format!("pane-zoom:{block}"),
            &ZOOM,
            "Zoom pane",
            "Zoom pane · this tab has one pane".into(),
            (rest, fade),
            false,
            None,
        )
    } else {
        button(
            format!("pane-zoom:{block}"),
            &ZOOM,
            "Zoom pane",
            hint("Zoom pane", "pane.zoom", Value::Null),
            (rest, fade),
            false,
            on(PaneAction::Zoom),
        )
    };
    let close = if last_pane(s, window) {
        button(
            format!("pane-close:{block}"),
            &CLOSE,
            "Close pane",
            "Close pane · the session's only pane".into(),
            (rest, fade),
            true,
            None,
        )
    } else {
        button(
            format!("pane-close:{block}"),
            &CLOSE,
            "Close pane",
            hint("Close pane…", "pane.close", Value::Null),
            (rest, fade),
            true,
            on(PaneAction::Close),
        )
    };
    cluster.child(zoom).child(close)
}

/// A floating header's surface: the popover colour over the terminal, with a hairline below
/// it. Inside a card it sits within the border and follows the card's top corners.
pub fn floating(header: El, carded: bool) -> El {
    let (edge, radius) = if carded {
        (1., CARD_RADIUS - 1.)
    } else {
        (0., 0.)
    };
    el().key("pane-header-float")
        .pin(Some(edge), Some(edge), Some(edge), None)
        .corners([radius, radius, 0., 0.])
        .continuous_corners()
        .fill(rgb(popover()))
        .box_shadow(shadow(0., 6., 16., black(0.35)))
        .transition(Transition::opacity().springs(SpringPair::new(SNAPPY, SNAPPY)))
        .child(header)
        .child(
            el().pin(Some(0.), None, Some(0.), Some(0.))
                .h(1.)
                .fill(white(0.09)),
        )
}

/// The band along a pane's top edge that brings its On hover header out and keeps it while
/// the pointer is on it. It takes no presses: until the header shows, the terminal under the
/// band works as everywhere else.
pub fn reveal_band(block: &str) -> El {
    let id = block.to_owned();
    el().key(format!("pane-header-band:{block}"))
        .pin(Some(0.), Some(0.), Some(0.), None)
        .h(REVEAL)
        .on_hover_change(move |s: &mut Workspace, inside: bool, _: &mut Tx| {
            if inside {
                s.header_reveal.hover = Some(id.clone());
            } else if s.header_reveal.hover.as_deref() == Some(id.as_str()) {
                s.header_reveal.hover = None;
            }
        })
        .hover_delays(REVEAL_AFTER, HIDE_AFTER)
}
