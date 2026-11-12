//! Settings → Terminal → Advanced font settings: the line height, the character spacing,
//! which of bold and italic are drawn, and the ordered fallback fonts. The first three apply
//! to every terminal as they change, with a preview above them; the fallback list, like the
//! terminal font, is used when the client is reopened.
use super::{
    panel::Intent,
    settings_kit::{
        ChipTone, button, card, footnote, on_return, page, row_frame, section, select_row, setting,
        state_chip, stepper, words,
    },
    settings_nav::SubPage,
    term::{self, Metrics},
    terminal_page::{Inks, preview, typography_sample},
    theme::*,
    widgets::Pill,
    workspace::Workspace,
};
use rex_rune::settings::typography::{BoldItalic, SpacingUnit, Typography};
use rune::{El, Tx, row, text};
use rune_kit::IconButton;

/// A number with its sign, as an adjustment reads: `+20`, `0`, `−3` (a real minus sign).
fn signed(value: i16) -> String {
    match value {
        0 => "0".to_owned(),
        v if v > 0 => format!("+{v}"),
        v => format!("\u{2212}{}", v.unsigned_abs()),
    }
}

/// What the Advanced font settings row says is set: the settings away from their defaults,
/// and how many fallback fonts there are. More than two would not fit the row, so they are
/// counted instead. Nothing when everything is as it comes.
pub fn summary(s: &Workspace) -> String {
    let font = &s.settings.terminal.font;
    let mut parts = Vec::new();
    if font.line_height() != 0 {
        parts.push(format!("Line height {}%", signed(font.line_height())));
    }
    if font.spacing() != 0 {
        parts.push(format!(
            "Spacing {}{}",
            signed(font.spacing()),
            font.spacing_unit.suffix()
        ));
    }
    if font.bold_italic != BoldItalic::Both {
        parts.push(font.bold_italic.name().to_owned());
    }
    match s.settings.font_fallbacks.len() {
        0 => {}
        1 => parts.push("1 fallback".to_owned()),
        n => parts.push(format!("{n} fallbacks")),
    }
    if parts.len() > 2 {
        format!("{} changes", parts.len())
    } else {
        parts.join(" · ")
    }
}

fn metrics_card(s: &Workspace, inks: &Inks) -> El {
    let font = &s.settings.terminal.font;
    let unit = font.spacing_unit;
    card([
        preview(
            s,
            "advanced-font-preview",
            "Typography preview: regular, bold and italic text as terminals set it.",
            inks,
            &typography_sample(inks),
        ),
        setting(
            "Line height",
            "Added to the height of every row, from \u{2212}20% to +100%.",
            stepper(
                "line-height",
                format!("{}%", signed(font.line_height())),
                |s, _| s.edit_typography(|font| font.step_line_height(-1)),
                |s, _| s.edit_typography(|font| font.step_line_height(1)),
            ),
        ),
        // The value carries its unit, as Line height's does, and the unit has a row of its
        // own: the two steppers are alone in their slots, so they line up on both edges.
        setting(
            "Character spacing",
            "Added to the width of every character.",
            stepper(
                "char-spacing",
                format!("{}{}", signed(font.spacing()), unit.suffix()),
                |s, _| s.edit_typography(|font| font.step_spacing(-1)),
                |s, _| s.edit_typography(|font| font.step_spacing(1)),
            ),
        ),
        select_row(
            "char-spacing-unit",
            "Spacing unit",
            match unit {
                SpacingUnit::Px => "Points, from \u{2212}3 to +12.",
                SpacingUnit::Percent => {
                    "A share of the character's width, from \u{2212}30% to +100%."
                }
            },
            &SpacingUnit::ALL.map(SpacingUnit::name),
            SpacingUnit::ALL
                .iter()
                .position(|choice| *choice == unit)
                .unwrap_or_default(),
            |s, i, _| {
                s.edit_typography(|font| {
                    font.set_spacing_unit(SpacingUnit::ALL[i.min(SpacingUnit::ALL.len() - 1)])
                })
            },
        ),
        select_row(
            "bold-italic",
            "Render bold and italic",
            "A style that isn't drawn shows in the ordinary face.",
            &BoldItalic::ALL.map(BoldItalic::name),
            BoldItalic::ALL
                .iter()
                .position(|choice| *choice == font.bold_italic)
                .unwrap_or_default(),
            |s, i, _| {
                s.edit_typography(|font| {
                    font.bold_italic = BoldItalic::ALL[i.min(BoldItalic::ALL.len() - 1)]
                })
            },
        ),
    ])
}

/// One fallback font: its place in the order, its name, and Move up, Move down and Remove.
/// The buttons are keyed by the family, so the keyboard stays on a font as it moves.
fn fallback_row(s: &Workspace, index: usize, family: &str, count: usize) -> El {
    let k = kit();
    type Act = fn(&mut Workspace, &str, &mut Tx);
    let action = |icon, label: &str, key: String, disabled: bool, act: Act| {
        let (name, again) = (family.to_owned(), family.to_owned());
        let button = IconButton::new(icon)
            .label(format!("{label} {family}"))
            .disabled(disabled)
            .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| act(s, &name, tx))
            .build(k)
            .key(key);
        if disabled {
            button
        } else {
            on_return(button, move |s, tx| act(s, &again, tx))
        }
    };
    let mut line = row_frame(false)
        .key(format!("fallback:{index}"))
        .child(
            text((index + 1).to_string())
                .font(mono(11.5))
                .color(rgb(muted()))
                .w(14.),
        )
        .child(words(family, ""));
    if s.fonts.installed(family) == Some(false) {
        line = line.child(state_chip(
            &format!("fallback-missing:{index}"),
            "Not installed",
            ChipTone::Attention,
            "fontconfig has no family of this name, so it is skipped.",
        ));
    }
    line.child(
        row()
            .gap(2.)
            .center_y()
            .child(action(
                &STEP_UP,
                "Move up",
                format!("fallback-up:{family}"),
                index == 0,
                |s, family, tx| s.move_fallback(family, -1, tx),
            ))
            .child(action(
                &STEP_DOWN,
                "Move down",
                format!("fallback-down:{family}"),
                index + 1 == count,
                |s, family, tx| s.move_fallback(family, 1, tx),
            ))
            .child(action(
                &CLOSE,
                "Remove",
                format!("fallback-remove:{family}"),
                false,
                |s, family, tx| s.remove_fallback(family, tx),
            )),
    )
}

fn fallbacks(s: &Workspace) -> El {
    let list = &s.settings.font_fallbacks;
    // The heading names the list; its first row is the way to add to it.
    let rows = std::iter::once(setting(
        "Add a fallback font",
        "Used in order for characters the terminal font doesn't have.",
        button("fallback-add", "Add…", Pill::Neutral, |s, tx| {
            s.open_anchored("fallback-add", Intent::AddFallback, tx)
        }),
    ))
    .chain(
        list.iter()
            .enumerate()
            .map(|(index, family)| fallback_row(s, index, family, list.len())),
    );
    let pending = if *list != s.fonts.saved_fallbacks {
        "The list changed and applies when you reopen the client."
    } else {
        "A change to the list applies when you reopen the client."
    };
    let mut notes = vec![
        footnote(format!(
            "{pending} After the list come the bundled IBM Plex Mono and the system's own fallbacks for other scripts and symbols."
        ))
        .key("fallback-note"),
    ];
    if let Some(launch) = &s.fonts.launch_fallbacks {
        notes.push(
            footnote(format!(
                "This launch was started with --font-fallback, so it uses {} in place of the list.",
                launch.join(", ")
            ))
            .key("fallback-override"),
        );
    }
    section("Fallback fonts", std::iter::once(card(rows)).chain(notes))
}

/// The Advanced font settings page.
pub fn advanced(s: &Workspace) -> El {
    let inks = Inks::of(s);
    page([
        section(
            "Metrics",
            [
                metrics_card(s, &inks),
                footnote(
                    "Every terminal follows at once, and one this client sizes is told its new rows and columns. Box drawing and block characters fill their cells at any setting.",
                ),
            ],
        ),
        fallbacks(s),
    ])
}

impl Workspace {
    /// The cell and text style terminals are drawn with now: the font size and the
    /// typography settings.
    pub fn metrics(&self) -> Metrics {
        term::metrics_for(self.settings.font_size, &self.settings.terminal.font)
    }

    /// The cell changed (the font size, the line height or the character spacing): every
    /// terminal this client sizes is told the rows and columns it now has.
    pub fn resize_terminals(&mut self) {
        let cell = self.metrics().cell;
        for term in self.terminals.values_mut() {
            term.last_size = None;
            term.resize_if_owner(cell);
        }
        // Find also uses cell boundaries; unowned panes receive no resize reply to wake it.
        for key in self.terminals.keys().cloned().collect::<Vec<_>>() {
            self.place_find(&key);
        }
    }

    /// Change the typography settings: saved, and every terminal follows at once.
    pub fn edit_typography(&mut self, edit: impl FnOnce(&mut Typography)) {
        let before = self.settings.terminal.font.clone();
        let cell = self.metrics().cell;
        edit(&mut self.settings.terminal.font);
        if self.settings.terminal.font != before {
            if self.metrics().cell != cell {
                self.resize_terminals();
            }
            self.save();
        }
    }

    /// Show the Advanced font settings page, with the families fontconfig knows read, so a
    /// fallback that isn't installed says so.
    pub fn open_advanced_fonts(&mut self, tx: &mut Tx) {
        self.font_catalog();
        self.open_sub_page(SubPage::AdvancedFont, tx);
    }

    fn fallback_at(&self, family: &str) -> Option<usize> {
        self.settings
            .font_fallbacks
            .iter()
            .position(|f| f.eq_ignore_ascii_case(family))
    }

    /// Take `family` out of the fallback fonts. The keyboard goes to the family that takes
    /// its place, to the one before it at the end of the list, or to Add… when none is left.
    pub fn remove_fallback(&mut self, family: &str, tx: &mut Tx) {
        let Some(index) = self.fallback_at(family) else {
            return;
        };
        let list = &mut self.settings.font_fallbacks;
        list.remove(index);
        let next = match list.get(index).or(list.last()) {
            Some(family) => format!("fallback-remove:{family}"),
            None => "fallback-add".to_owned(),
        };
        tx.focus(next, tx.focus_visible());
        self.save();
    }

    /// Move `family` one place earlier (`-1`) or later (`1`) among the fallback fonts. At
    /// an end of the list the button that brought it there can't be pressed again, so the
    /// keyboard goes to the one that moves it back.
    pub fn move_fallback(&mut self, family: &str, direction: isize, tx: &mut Tx) {
        let Some(index) = self.fallback_at(family) else {
            return;
        };
        let list = &mut self.settings.font_fallbacks;
        let to = index as isize + direction.signum();
        if to < 0 || to as usize >= list.len() {
            return;
        }
        let to = to as usize;
        list.swap(index, to);
        if to == 0 {
            tx.focus(format!("fallback-down:{family}"), tx.focus_visible());
        } else if to + 1 == list.len() {
            tx.focus(format!("fallback-up:{family}"), tx.focus_visible());
        }
        self.save();
    }
}
