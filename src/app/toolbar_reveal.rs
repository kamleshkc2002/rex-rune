//! When the toolbar shows and where it is drawn.
//!
//! Settings → Interface → Toolbar decides: Always keeps its 52 pt row above the panes, On
//! hover gives that row to the panes and floats the toolbar over the top of the window while
//! the pointer rests on the window's top edge, and Hidden draws none. Reveal toolbar floats
//! it from the keyboard in either, until it is run again, so a window without a toolbar row
//! can still be moved, closed and steered. A toolbar that floats never moves the panes under
//! it: no terminal is resized.
use super::{theme::*, workspace::Workspace};
use rex_rune::settings::interface::ToolbarBehavior;
use rune::{Edge, El, SpringPair, Transition, Tx, el};
use std::time::Duration;

/// Floats the toolbar where it has no row, until it is run again.
pub const TOGGLE: &str = "client.toolbar.toggle";

/// The band along the window's top edge where the pointer brings an On hover toolbar out.
/// No taller than the default padding above a terminal's first row, so working on that row
/// never brings it out.
pub const EDGE: f32 = 8.;
/// How long the pointer rests on the edge before the toolbar shows, and how long after it
/// left the toolbar stays. A pass over the edge on the way out of the window shows nothing.
const REVEAL_AFTER: Duration = Duration::from_millis(80);
const HIDE_AFTER: Duration = Duration::from_millis(320);

/// The toolbar brought out without a setting change. Never saved.
#[derive(Default)]
pub struct Reveal {
    /// The pointer rests on the window's top edge, or on the toolbar it brought out.
    pub hover: bool,
    /// Reveal toolbar is on.
    pub pinned: bool,
}

/// How the toolbar is drawn now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    None,
    /// In its own row above the panes.
    Row,
    /// Over the top of the window.
    Floating,
}

pub fn placement(s: &Workspace) -> Placement {
    let reveal = &s.toolbar_reveal;
    match s.settings.interface.toolbar {
        ToolbarBehavior::Always => Placement::Row,
        ToolbarBehavior::OnHover if reveal.hover || reveal.pinned => Placement::Floating,
        ToolbarBehavior::Hidden if reveal.pinned => Placement::Floating,
        ToolbarBehavior::OnHover | ToolbarBehavior::Hidden => Placement::None,
    }
}

/// Whether Reveal toolbar has anything to bring out.
pub fn can_reveal(s: &Workspace) -> bool {
    s.settings.interface.toolbar != ToolbarBehavior::Always
}

/// The height the toolbar's row takes from the panes: nothing unless it is always shown.
pub fn row_height(s: &Workspace) -> f32 {
    if placement(s) == Placement::Row {
        TOOLBAR
    } else {
        0.
    }
}

/// The height the toolbar covers at the window's top now, in its row or floating.
pub fn shown_height(s: &Workspace) -> f32 {
    if placement(s) == Placement::None {
        0.
    } else {
        TOOLBAR
    }
}

/// The space pane cards and the sidebar keep from the window's top edge: none under the
/// toolbar's row, which is their margin, and the outer margin where they start at the edge.
pub fn top_margin(s: &Workspace) -> f32 {
    if placement(s) == Placement::Row {
        0.
    } else {
        MARGIN
    }
}

impl Workspace {
    /// Reveal toolbar, and the same again to put it back.
    pub fn toggle_toolbar(&mut self) {
        self.toolbar_reveal.pinned = !self.toolbar_reveal.pinned && can_reveal(self);
        self.reveal_tab_again = true;
    }

    /// Settings → Interface → Toolbar: saved, and a toolbar that was brought out by hand goes
    /// back with the old choice.
    pub fn set_toolbar(&mut self, toolbar: ToolbarBehavior, tx: &mut Tx) {
        self.settings.interface.toolbar = toolbar;
        self.toolbar_reveal = Reveal::default();
        self.reveal_tab_again = true;
        self.save();
        // The panes take or give the row at once; terminals never animate.
        tx.instant();
    }
}

/// A floating toolbar's surface: the popover colour over the panes with a hairline under it,
/// sliding in from the window's top edge.
pub fn floating(toolbar: El) -> El {
    el().key("toolbar-float")
        .pin(Some(0.), Some(0.), Some(0.), None)
        .h(TOOLBAR)
        .fill(rgb(popover()))
        .box_shadow(shadow(0., 8., 24., black(0.4)))
        .transition(
            Transition::move_edge(Edge::Top)
                .combined(Transition::opacity())
                .springs(SpringPair::new(SNAPPY, SNAPPY)),
        )
        .child(toolbar)
        .child(
            el().pin(Some(0.), None, Some(0.), Some(0.))
                .h(1.)
                .fill(white(0.09)),
        )
}

/// The band along the window's top edge that brings an On hover toolbar out, and keeps it
/// while the pointer is on the toolbar. It takes no presses: until the toolbar shows, what
/// is under the band works as everywhere else.
pub fn band(s: &Workspace) -> El {
    let height = if placement(s) == Placement::Floating {
        TOOLBAR
    } else {
        EDGE
    };
    el().key("toolbar-band")
        .pin(Some(0.), Some(0.), Some(0.), None)
        .h(height)
        .on_hover_change(|s: &mut Workspace, inside: bool, _: &mut Tx| {
            s.toolbar_reveal.hover = inside;
            if inside {
                s.reveal_tab_again = true;
            }
        })
        .hover_delays(REVEAL_AFTER, HIDE_AFTER)
}

/// With no toolbar row, pending-sequence and receiving-download chips float at the top left.
/// This surface exists only while it has visible status; it reserves no terminal space.
pub fn sequence_float(chip: El) -> El {
    el().key("pending-sequence-float")
        .pin(Some(MARGIN), Some(GUTTER), None, None)
        .radius(10.)
        .continuous_corners()
        .fill(rgb(popover()))
        .box_shadow(shadow(0., 4., 16., black(0.45)))
        .transition(Transition::opacity().springs(SpringPair::new(SNAPPY, SNAPPY)))
        .child(chip)
}
