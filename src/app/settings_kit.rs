//! What every Settings page is built from, so pages share one rhythm and one set of controls.
//!
//! - [`page`] stacks [`section`]s. A section is a heading, one or more [`card`]s and, when a
//!   group needs explaining, a [`footnote`] under them.
//! - A [`card`] holds rows and draws the divider between each pair itself: a page never
//!   places a divider.
//! - Rows: [`setting`] (a label, at most one sentence of help, and a control in the trailing
//!   slot), [`select_row`] (a setting whose control is a [`value_popup`]), [`disclosure`]
//!   (the whole row is a button that opens a form or a sub-page) and [`row_frame`] for a row
//!   with its own content.
//! - Controls: [`segmented`] and [`segmented_fixed`], [`value_popup`], [`stepper`],
//!   [`button`], and the [`value`], [`mono_value`] and [`state_chip`] read-outs.
//!
//! A popup opened from a [`value_popup`] is a list panel drawn over Settings and placed
//! against the control (`Panel::anchor_to`, read by `popups::list_popup`).
use super::{
    panel::{Choice, Intent, Panel, PanelKind},
    theme::*,
    widgets::{Pill, glyph, pill},
    workspace::Workspace,
};
use rune::{Cursor, El, Icon, Key, KeyEvent, Shadow, Tx, col, el, row, text};
use rune_kit::IconButton;

/// The Settings dialog, clamped to the window by `settings_view::settings`.
pub const PANEL_WIDTH: f32 = 816.;
pub const PANEL_HEIGHT: f32 = 680.;
/// The section list beside the page.
pub const NAV_WIDTH: f32 = 204.;
/// A row's inset from its card's edges; also the inset of a card's dividers, and of the
/// headings and footnotes outside it.
pub const INSET: f32 = 12.;
/// A row with a label only, and one with a helper line under it.
pub const ROW: f32 = 40.;
pub const ROW_WITH_HELPER: f32 = 48.;
/// Buttons, popup buttons and steppers in a row's trailing slot.
pub const CONTROL: f32 = 26.;
/// One segment of a [`segmented`] control.
pub const SEGMENT: f32 = 120.;
const SEGMENT_HEIGHT: f32 = 28.;

/// A page's sections, top to bottom.
pub fn page(sections: impl IntoIterator<Item = El>) -> El {
    col().w_full().gap(22.).children(sections)
}

/// A group of cards under a heading; `""` for a group that needs no name. Cards, a row of
/// actions and a [`footnote`] are all children.
pub fn section(title: &str, children: impl IntoIterator<Item = El>) -> El {
    let mut body = col().w_full().gap(8.);
    if !title.is_empty() {
        body = body.child(
            text(title.to_owned())
                .role("heading")
                .font(medium(13.))
                .color(rgb(ink()))
                .px(INSET),
        );
    }
    body.children(children)
}

/// Rows on one inset surface, with a hairline between each pair.
pub fn card(rows: impl IntoIterator<Item = El>) -> El {
    let mut card = col()
        .w_full()
        .radius(CARD_RADIUS)
        .continuous_corners()
        .fill(white(0.04));
    for (index, row_el) in rows.into_iter().enumerate() {
        if index > 0 {
            card = card.child(
                row()
                    .w_full()
                    .px(INSET)
                    .child(el().role("separator").grow(1.).h(1.).fill(white(0.06))),
            );
        }
        card = card.child(row_el);
    }
    card
}

/// Small print under a card: what the group above does, in a sentence or two.
pub fn footnote(words: impl Into<String>) -> El {
    text(words.into())
        .font(ui(11.5))
        .color(rgb(muted()))
        .wrap()
        .px(INSET)
}

/// [`footnote`] for a path, a command or another value to copy.
pub fn mono_footnote(words: impl Into<String>) -> El {
    text(words.into())
        .font(mono(11.))
        .color(rgb(muted()))
        .wrap()
        .selectable(true)
        .px(INSET)
}

/// The frame every card row shares: full width, the card's inset, content centred on the
/// row. `helper` says whether the row carries a second line.
pub fn row_frame(helper: bool) -> El {
    row()
        .w_full()
        .min_h(if helper { ROW_WITH_HELPER } else { ROW })
        .px(INSET)
        .py(6.)
        .gap(16.)
        .center_y()
}

/// A row's words: its label and, when the label alone doesn't say it, one helper sentence.
/// The words give way to the trailing slot: a label too long for what is left (a host's
/// name, an endpoint) is cut short with its whole text as a tooltip, and never runs under
/// the control.
pub fn words(label: &str, detail: &str) -> El {
    let mut words = col().grow(1.).shrink().min_w(0.).gap(1.).child(
        text(label.to_owned())
            .font(ui(13.))
            .color(rgb(ink()))
            .truncate()
            .help_when_truncated(label.to_owned()),
    );
    if !detail.is_empty() {
        words = words.child(
            text(detail.to_owned())
                .font(ui(11.5))
                .color(rgb(muted()))
                .wrap(),
        );
    }
    words
}

/// A labelled setting: its words on the left and its control in the trailing slot, which
/// ends at the card's inset on every row.
pub fn setting(label: &str, detail: &str, control: El) -> El {
    row_frame(!detail.is_empty())
        .child(words(label, detail))
        .child(control)
}

/// A read-only value in a row's trailing slot.
pub fn value(value: impl Into<String>) -> El {
    text(value.into())
        .font(ui(13.))
        .color(rgb(muted()))
        .selectable(true)
}

/// [`value`] for a commit, a path, an id or a command.
pub fn mono_value(value: impl Into<String>) -> El {
    text(value.into())
        .font(mono(12.))
        .color(rgb(muted()))
        .selectable(true)
}

/// How a [`state_chip`] reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChipTone {
    /// In place and as it should be: a check and plain words.
    Good,
    /// Not there, with nothing wrong: an outline and `MUTED` words.
    Off,
    /// Needs the user: `WARNING`, always with the word.
    Attention,
}

/// What something is now, in a word or two on a small rounded rectangle. A read-out and
/// never a button, which is why it isn't a capsule; `why` is its tooltip, for a word that
/// needs a sentence.
pub fn state_chip(key: &str, label: &str, tone: ChipTone, why: &str) -> El {
    let (ink_color, chip) = match tone {
        ChipTone::Good => (rgb(ink()), row().fill(white(0.07))),
        ChipTone::Off => (rgb(muted()), row().border(1., white(0.1))),
        ChipTone::Attention => (rgb(warning()), row().fill(rgb(warning()).alpha(0.12))),
    };
    let mut chip = chip
        .key(key.to_owned())
        .role("status")
        .label(label.to_owned())
        .h(20.)
        .px(7.)
        .gap(3.)
        .center()
        .radius(6.)
        .continuous_corners();
    if !why.is_empty() {
        chip = chip.help(why.to_owned());
    }
    if tone == ChipTone::Good {
        chip = chip.child(el().ml(-3.).child(glyph(&CHECK, 13., rgb(accent()))));
    }
    chip.child(text(label.to_owned()).font(medium(11.)).color(ink_color))
}

/// Return presses a focused control as Space does: Settings has no default button to take it.
fn on_return(e: El, action: impl Fn(&mut Workspace, &mut Tx) + 'static) -> El {
    e.on_key(move |s: &mut Workspace, event: &KeyEvent, tx: &mut Tx| {
        (event.key == Key::Enter).then(|| action(s, tx)).is_some()
    })
}

/// A button in a row's trailing slot, or in a row of actions under a card.
pub fn button(
    key: &str,
    label: &str,
    kind: Pill,
    action: impl Fn(&mut Workspace, &mut Tx) + Clone + 'static,
) -> El {
    on_return(pill(key, label, kind, CONTROL, action.clone()), action)
}

/// A button that can't be used now: drawn dimmed, out of the Tab order, with the reason as
/// its tooltip.
pub fn unavailable(key: &str, label: &str, why: &str) -> El {
    let e = pill(key, label, Pill::Neutral, CONTROL, |_, _| {})
        .opacity(0.4)
        .focusable(false);
    if why.is_empty() {
        e.inert()
    } else {
        e.help(why.to_owned())
    }
}

/// A stepper: − value +, the value in a fixed slot so the buttons don't move as it changes.
pub fn stepper(
    key: &str,
    value: String,
    down: impl Fn(&mut Workspace, &mut Tx) + Clone + 'static,
    up: impl Fn(&mut Workspace, &mut Tx) + Clone + 'static,
) -> El {
    let k = kit();
    row()
        .gap(8.)
        .center_y()
        .child(on_return(
            IconButton::new(&MINUS)
                .label("Smaller")
                .on_press_tx({
                    let down = down.clone();
                    move |s: &mut Workspace, tx: &mut Tx| down(s, tx)
                })
                .build(k)
                .key(format!("{key}-smaller")),
            down,
        ))
        .child(
            row()
                .w(52.)
                .center()
                .child(text(value).font(medium(13.)).color(rgb(ink()))),
        )
        .child(on_return(
            IconButton::new(&PLUS)
                .label("Larger")
                .on_press_tx({
                    let up = up.clone();
                    move |s: &mut Workspace, tx: &mut Tx| up(s, tx)
                })
                .build(k)
                .key(format!("{key}-larger")),
            up,
        ))
}

/// One choice of a [`segmented`] control: a word, and a glyph before it when a picture says
/// it faster.
#[derive(Clone, Copy)]
pub struct Segment<'a> {
    label: &'a str,
    icon: Option<&'static Icon>,
}

impl<'a> Segment<'a> {
    pub fn new(label: &'a str) -> Self {
        Self { label, icon: None }
    }
    pub fn icon(mut self, icon: &'static Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// A segmented control for two or three choices, every segment [`SEGMENT`] wide, so the
/// controls of neighbouring rows line up on both edges.
pub fn segmented(
    key: &str,
    segments: &[Segment<'_>],
    selected: Option<usize>,
    pick: impl Fn(&mut Workspace, usize, &mut Tx) + Clone + 'static,
) -> El {
    segmented_fixed(
        key,
        SEGMENT * segments.len().max(1) as f32,
        segments,
        selected,
        pick,
    )
}

/// [`segmented`] at a width of the caller's choosing, divided equally. It draws in the kit's
/// tokens, as its `SegmentedControl` does, and adds what that one lacks: a leading glyph.
/// Left and Right move between segments; Tab visits the selected one.
pub fn segmented_fixed(
    key: &str,
    width: f32,
    segments: &[Segment<'_>],
    selected: Option<usize>,
    pick: impl Fn(&mut Workspace, usize, &mut Tx) + Clone + 'static,
) -> El {
    let k = kit();
    let c = &k.c;
    let each = width / segments.len().max(1) as f32;
    let hairline = |color| Shadow {
        spread: 0.5,
        color,
        ..Default::default()
    };
    let mut root = row()
        .key(key.to_owned())
        .role("radiogroup")
        .focus_group()
        .w(width)
        .h(SEGMENT_HEIGHT)
        .center_y()
        .radius(k.radius.sm)
        .fill(c.layer_01)
        .box_shadow(hairline(c.border_base));
    if let Some(index) = selected {
        root = root.child(
            el().key("indicator")
                .absolute(each * index as f32, 0.)
                .size(each, SEGMENT_HEIGHT)
                .radius(k.radius.sm)
                .fill(c.bg_base)
                .box_shadow(hairline(c.border_strong))
                .motion(k.motion.control),
        );
    }
    for (index, segment) in segments.iter().enumerate() {
        let on = selected == Some(index);
        let ink = if on { c.text_base } else { c.text_muted };
        let mut label = text(segment.label.to_owned())
            .key("label")
            .font(if on { medium(13.) } else { ui(13.) })
            .color(ink)
            .truncate();
        let mut picture = segment.icon.map(|icon| glyph(icon, 15., ink));
        if !on {
            label = label.group_color(c.text_base);
            picture = picture.map(|p| p.group_color(c.text_base));
        }
        let pick = pick.clone();
        let mut cell = row()
            .key(format!("segment{index}"))
            .role("radio")
            .label(segment.label.to_owned())
            .focusable(true)
            .focus_selected(on)
            .focus_ring(2., 1., c.border_focus)
            .w(each)
            .h(SEGMENT_HEIGHT)
            .px(8.)
            .gap(6.)
            .center()
            .radius(k.radius.sm)
            .hover_group()
            .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| pick(s, index, tx))
            .children(picture)
            .child(label);
        if !on {
            cell = cell.cursor(Cursor::Pointer).hover(c.hover).press(c.pressed);
        }
        root = root.child(cell);
    }
    root
}

/// A popup button: the current value and a round badge with an up-and-down chevron. Pressing
/// it performs `open`, which must open a list panel; the list is drawn over Settings against
/// this control, and focus comes back here when it closes. `key` names the control and is
/// what the list is placed by.
pub fn value_popup(key: &str, value: &str, open: Intent) -> El {
    let control = key.to_owned();
    let press = move |s: &mut Workspace, tx: &mut Tx| s.open_anchored(&control, open.clone(), tx);
    popup_button(key, value, press)
}

fn popup_button(
    key: &str,
    value: &str,
    press: impl Fn(&mut Workspace, &mut Tx) + Clone + 'static,
) -> El {
    on_return(
        row()
            .key(key.to_owned())
            .role("combobox")
            .label(value.to_owned())
            .h(CONTROL)
            .pl(10.)
            .pr(3.)
            .gap(7.)
            .center_y()
            .radius(CONTROL / 2.)
            .continuous_corners()
            .cursor(Cursor::Pointer)
            .hover(white(0.07))
            .press(white(0.11))
            .focusable(true)
            .focus_ring(2., 2., rgb(accent()))
            .on_press_tx(press.clone())
            .child(
                text(value.to_owned())
                    .font(medium(13.))
                    .color(rgb(ink()))
                    .truncate()
                    .max_w(240.),
            )
            .child(
                el().size(20., 20.)
                    .radius(10.)
                    .fill(white(0.09))
                    .center()
                    .child(glyph(&UPDOWN, 15., rgb(muted()))),
            ),
        press,
    )
}

/// A setting chosen from a short list of named values: [`setting`] with a [`value_popup`]
/// that lists `options` with `current` checked. `apply` gets the chosen index once the popup
/// has closed; it is a plain function, so the popup's state stays `Clone` and `Debug`.
pub fn select_row(
    key: &str,
    label: &str,
    detail: &str,
    options: &[&str],
    current: usize,
    apply: fn(&mut Workspace, usize, &mut Tx),
) -> El {
    let (control, title) = (key.to_owned(), label.to_owned());
    let list: Vec<String> = options.iter().map(|o| (*o).to_owned()).collect();
    let press = move |s: &mut Workspace, _: &mut Tx| {
        s.open_choice(
            &control,
            &title,
            Choice {
                options: list.clone(),
                current: Some(current),
                apply,
            },
        )
    };
    setting(
        label,
        detail,
        popup_button(
            key,
            options.get(current).copied().unwrap_or_default(),
            press,
        ),
    )
}

/// A row that is one button: its words, what it is set to now, and a chevron. For a setting
/// edited in a form or on a page of its own.
pub fn disclosure(
    key: &str,
    label: &str,
    detail: &str,
    current: &str,
    open: impl Fn(&mut Workspace, &mut Tx) + Clone + 'static,
) -> El {
    let mut e = row_frame(!detail.is_empty())
        .key(key.to_owned())
        .role("button")
        .label(label.to_owned())
        .cursor(Cursor::Pointer)
        .radius(CARD_RADIUS - 2.)
        .hover(white(0.04))
        .press(white(0.07))
        .focusable(true)
        .focus_ring(2., -2., rgb(accent()))
        .on_press_tx(open.clone())
        .child(words(label, detail));
    if !current.is_empty() {
        e = e.child(
            text(current.to_owned())
                .font(ui(13.))
                .color(rgb(muted()))
                .truncate()
                .max_w(260.),
        );
    }
    on_return(
        e.child(el().ml(-8.).mr(-4.).child(glyph(&RIGHT, 16., rgb(muted())))),
        open,
    )
}

impl Workspace {
    /// Perform `open` (an intent that opens a list panel) for the control keyed `control`,
    /// so the list is placed against it.
    pub fn open_anchored(&mut self, control: &str, open: Intent, tx: &mut Tx) {
        self.perform(open, tx);
        if let Some(panel) = self.panel.as_mut().filter(|p| p.is_list()) {
            panel.anchor_to = Some(control.to_owned());
        }
    }

    /// Open the list of a [`select_row`] against its control.
    pub fn open_choice(&mut self, control: &str, title: &str, choice: Choice) {
        let current = choice.current;
        self.open_panel(PanelKind::Choice(choice), title, &[]);
        let panel = self.panel.as_mut().expect("the choice just opened");
        panel.anchor_to = Some(control.to_owned());
        if let Some(current) = current {
            panel.selected = current;
        }
    }

    /// Take the chosen option of an open [`select_row`] list: close the list, back to the
    /// panel it was opened from, then apply.
    pub(super) fn choose(&mut self, index: usize, tx: &mut Tx) {
        let Some(PanelKind::Choice(choice)) = self.panel.as_ref().map(|p| p.kind.clone()) else {
            return;
        };
        match self.panel.as_mut().and_then(|p| p.back.take()) {
            Some(back) => self.panel = Some(*back),
            None => self.close_panel(tx),
        }
        if index < choice.options.len() {
            (choice.apply)(self, index, tx);
        }
    }
}

/// The panel a list is drawn over, when it was opened from a control in Settings.
pub fn under(panel: &Panel) -> Option<&Panel> {
    panel.anchor_to.as_ref()?;
    panel
        .back
        .as_deref()
        .filter(|back| matches!(back.kind, PanelKind::Settings))
}
