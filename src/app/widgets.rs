//! Building blocks on the Midnight tokens: chrome icon buttons, pill buttons, list rows with a
//! sliding highlight, scroll thumbs and hint lines.
use super::{theme::*, workspace::Workspace};
use rune::{Color, Cursor, El, Icon, Live, LiveText, ScrollPos, Tx, el, icon, row, text};

pub fn glyph(i: &'static Icon, size: f32, color: Color) -> El {
    icon(i, size).color(color)
}

/// Official artwork, shared by the identity chip and empty state. Keep the same
/// footprint while the image worker decodes; the parent supplies its semantics.
pub fn app_logo(s: &Workspace, size: f32) -> El {
    let element = el().size(size, size);
    if let Some(image) = &s.logo {
        element.pixels(image.pixels())
    } else {
        element
    }
}

/// A round icon control in the bar or a card: never keeps Rune focus, so the next key still
/// reaches the terminal, and never sits in the Tab order.
pub fn chrome_button(
    key: impl Into<String>,
    i: &'static Icon,
    help: &str,
    size: f32,
    f: impl Fn(&mut Workspace, &mut Tx) + 'static,
) -> El {
    row()
        .key(key.into())
        .role("button")
        .label(help.split(" · ").next().unwrap_or(help).to_owned())
        .size(size, size)
        .center()
        .radius(9.)
        .continuous_corners()
        .cursor(Cursor::Pointer)
        .hover(white(0.08))
        .press(white(0.12))
        .preserve_focus_on_press(true)
        .tab_index(-1)
        .help(help.to_owned())
        .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| {
            f(s, tx);
            tx.blur();
        })
        .child(glyph(i, 16., rgb(muted())))
}

#[derive(Clone, Copy, PartialEq)]
pub enum Pill {
    Primary,
    Neutral,
    Danger,
    Warning,
}

/// A pill button: accent gradient (primary), chip (neutral) or tinted (danger, warning). Shows a
/// 2pt accent focus ring outside its shape when focused from the keyboard.
pub fn pill(
    key: &str,
    label: &str,
    kind: Pill,
    height: f32,
    f: impl Fn(&mut Workspace, &mut Tx) + 'static,
) -> El {
    let (fg, e) = match kind {
        Pill::Primary => (
            rgb(on_accent()),
            row()
                .linear_gradient(
                    1.9,
                    [
                        (0., rgb(accent())),
                        (1., rgb(mix(accent2(), accent(), 0.55))),
                    ],
                )
                .box_shadow(shadow(0., 6., 18., rgb(accent()).alpha(0.25)))
                .box_shadow(inset(1., white(0.35))),
        ),
        Pill::Neutral => (rgb(ink()), chip(row()).hover(white(0.09))),
        Pill::Danger => (
            rgb(danger()),
            row()
                .fill(rgb(danger()).alpha(0.1))
                .border(1., rgb(danger()).alpha(0.35))
                .hover(rgb(danger()).alpha(0.16)),
        ),
        Pill::Warning => (
            rgb(warning()),
            row()
                .fill(rgb(warning()).alpha(0.1))
                .border(1., rgb(warning()).alpha(0.4))
                .hover(rgb(warning()).alpha(0.18)),
        ),
    };
    e.key(key.to_owned())
        .role("button")
        .label(label.to_owned())
        .h(height)
        .px(if height < 28. { 10. } else { 14. })
        .center()
        .radius(height / 2.)
        .continuous_corners()
        .cursor(Cursor::Pointer)
        .focusable(true)
        .focus_ring(2., 2., rgb(accent()))
        .on_press_tx(f)
        .child(
            text(label.to_owned())
                .font(medium(if height < 28. { 11.5 } else { 13. }))
                .color(fg),
        )
}

pub struct Row<'a> {
    pub key: String,
    /// The surface's own highlight id, so it slides within one list only.
    pub surface: &'a str,
    pub label: &'a str,
    pub detail: &'a str,
    pub height: f32,
    pub check: Option<bool>,
    pub followup: bool,
    pub enabled: bool,
    pub selected: bool,
    /// Shown at the end in place of the detail, while the row is highlighted.
    pub actions: Option<El>,
    /// Columns drawn after the label in place of the detail, with the widest the label may
    /// be. The label then keeps its own width up to that, and the columns give way first.
    pub columns: Option<(El, f32)>,
}

/// The sliding selection: a childless background that flies to the next selected row, with no
/// entrance or exit of its own (so two never show). Fill **and** outline, never fill alone.
pub fn highlight(id: &str, radius: f32) -> El {
    el().pin(Some(0.), Some(0.), Some(0.), Some(0.))
        .radius(radius)
        .continuous_corners()
        .fill(white(0.09))
        .border(1., white(0.06))
        .matched(id.to_owned())
        .transition(no_presence())
        .motion(SNAPPY)
}

/// A list row. The keyboard highlight follows the pointer, so exactly one row is lit.
pub fn list_row(
    r: Row<'_>,
    index: usize,
    on_press: impl Fn(&mut Workspace, &mut Tx) + 'static,
) -> El {
    let fg = rgb(if r.enabled { ink() } else { muted() });
    let detail = if r.selected {
        rgb(ink()).alpha(0.75)
    } else {
        rgb(muted())
    };
    let mut e = row()
        .key(r.key)
        .role("menuitem")
        .label(r.label.to_owned())
        .w_full()
        .h(r.height)
        .px(9.)
        .gap(8.)
        .center_y()
        .help_when_truncated(format!("{} · {}", r.label, r.detail));
    // The highlight's slot is always there, so the row's other children keep their places
    // (and don't animate) as it comes and goes.
    e = e.child(if r.selected {
        highlight(&format!("sel:{}", r.surface), 7.)
    } else {
        el().pin(Some(0.), Some(0.), Some(0.), Some(0.))
    });
    if r.enabled {
        e = e
            .cursor(Cursor::Pointer)
            .on_hover(move |s: &mut Workspace| {
                if let Some(p) = &mut s.panel {
                    p.selected = index;
                }
            })
            .on_press_tx(on_press);
    }
    if let Some(current) = r.check {
        e = e.child(el().w(14.).center().child(if current {
            glyph(&CHECK, 14., rgb(accent()))
        } else {
            el()
        }));
    }
    let label = text(r.label.to_owned())
        .font(if r.selected { medium(13.) } else { ui(13.) })
        .color(fg)
        .truncate();
    let (label, columns) = match r.columns {
        // A box as wide as the label, so the row never shrinks it; the label is cut only
        // once it reaches the box's limit.
        Some((columns, limit)) => (
            row()
                .max_w(limit)
                .child(label.help_when_truncated(r.label.to_owned())),
            Some(columns),
        ),
        None => (label.grow(1.).shrink(), None),
    };
    e = e.child(label);
    match r.actions.filter(|_| r.selected) {
        Some(actions) => e
            .child(actions)
            .child(el().pin(Some(0.), Some(0.), Some(0.), Some(0.))),
        None => e
            .child(columns.unwrap_or_else(|| text(r.detail.to_owned()).font(ui(11.)).color(detail)))
            .child(el().w(14.).child(if r.followup {
                glyph(&RIGHT, 13., rgb(muted()))
            } else {
                el()
            })),
    }
}

/// A small icon button inside a list row: pressing it never presses the row.
pub fn row_action(
    key: String,
    icon: &'static Icon,
    help: &str,
    on_press: impl Fn(&mut Workspace, &mut Tx) + 'static,
) -> El {
    el().key(key)
        .role("button")
        .label(help.to_owned())
        .help(help.to_owned())
        .size(22., 22.)
        .center()
        .radius(6.)
        .hover(white(0.1))
        .press(white(0.14))
        .cursor(Cursor::Pointer)
        .preserve_focus_on_press(true)
        .tab_index(-1)
        .on_press_tx(on_press)
        .child(glyph(icon, 13., rgb(ink()).alpha(0.75)))
}

/// A list's scroll thumb and its direction-aware hint, both driven from the scroll offset
/// without rebuilding the view.
#[derive(Clone)]
pub struct ScrollFx {
    pub thumb: Live<[f32; 4]>,
    pub hint: LiveText,
}
impl Default for ScrollFx {
    fn default() -> Self {
        Self {
            thumb: Live::new([0.; 4]),
            hint: LiveText::new(""),
        }
    }
}

impl ScrollFx {
    pub fn watch(
        &self,
        width: f32,
        viewport: f32,
        hint: impl Fn(f32, f32) -> String + 'static,
    ) -> impl Fn(ScrollPos) + 'static {
        let (thumb, label) = (self.thumb.clone(), self.hint.clone());
        move |p: ScrollPos| {
            let content = viewport + p.max[1];
            let h = (viewport * viewport / content.max(1.))
                .max(16.)
                .min(viewport);
            let y = if p.max[1] > 0. {
                p.offset[1] / p.max[1] * (viewport - h)
            } else {
                0.
            };
            thumb.set([width - 6., y, 4., h]);
            label.set(hint(p.offset[1], p.max[1]));
        }
    }

    pub fn thumb_el(&self) -> El {
        el().absolute(0., 0.)
            .size(4., 16.)
            .radius(2.)
            .fill(white(0.22))
            .live_rect(&self.thumb)
    }

    pub fn hint_el(&self) -> El {
        row().h(24.).px(12.).center_y().child(
            text("More sessions and hosts below")
                .live_text(&self.hint)
                .font(ui(11.))
                .color(rgb(muted()))
                .grow(1.),
        )
    }
}
