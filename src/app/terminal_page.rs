//! Terminal display and selection settings. Previews reuse the real grid renderer.
use super::{
    panel::Intent,
    settings_kit::{
        card, disclosure, footnote, page, preview_well, section, select_row, setting, stepper,
        value, value_popup,
    },
    term::{
        self,
        preview::{Run, Sample, plain},
    },
    theme::*,
    workspace::Workspace,
};
use rex_linux::settings::terminal::{CursorStyle, SelectionColor, TerminalConfig};
use rune::El;

/// What the previews draw in: the colours panes draw in now, and the terminal's default
/// text, cursor and a few of its palette's colours.
pub(super) struct Inks {
    colors: term::TermColors,
    text: u32,
    cursor: u32,
    /// The prompt's directory, its branch, and a warning: ANSI blue, magenta and yellow of
    /// the declared theme, or the chrome's nearest tokens when it names none.
    path: u32,
    branch: u32,
    caution: u32,
}

impl Inks {
    pub(super) fn of(s: &Workspace) -> Self {
        let theme = s.term_theme.as_ref();
        let ansi = |index: u8, otherwise: u32| {
            theme
                .and_then(|t| t.palette.iter().find(|(i, _)| *i == index))
                .map_or(otherwise, |(_, color)| *color)
        };
        let text = theme.and_then(|t| t.foreground).unwrap_or(ink());
        Self {
            colors: s.term_colors(),
            text,
            cursor: theme.and_then(|t| t.cursor).unwrap_or(text),
            path: ansi(4, accent()),
            branch: ansi(5, accent2()),
            caution: ansi(3, warning()),
        }
    }

    fn sample<'a>(&self, lines: Vec<Vec<Run<'a>>>) -> Sample<'a> {
        Sample {
            lines,
            text: self.text,
            cursor_color: self.cursor,
            ..Sample::default()
        }
    }
}

/// A preview well showing `sample` as the terminals set their text now.
pub(super) fn preview(
    s: &Workspace,
    key: &str,
    label: &str,
    inks: &Inks,
    sample: &Sample<'_>,
) -> El {
    let m = term::metrics(s.settings.font_size);
    let (_, height) = term::preview::size(sample, &m);
    preview_well(
        key,
        label,
        inks.colors.background,
        height,
        term::preview::sample(&format!("{key}-sample"), sample, &m, &inks.colors),
    )
}

/// A switch for one of the terminal settings, saved as soon as it flips.
fn toggle(key: &str, label: &str, on: bool, field: fn(&mut TerminalConfig) -> &mut bool) -> El {
    rune_kit::Switch::new(on)
        .label(label.to_owned())
        .hide_label()
        .on_toggle(move |s: &mut Workspace| {
            let value = field(&mut s.settings.terminal);
            *value = !*value;
            s.save();
        })
        .build(kit())
        .key(key.to_owned())
}

/// Regular, bold and italic text, as a build's output has them.
pub(super) fn typography_sample(inks: &Inks) -> Sample<'static> {
    inks.sample(vec![
        vec![
            plain("~/rex ").color(inks.path),
            plain("main ").color(inks.branch),
            plain("$ cargo build && cargo test"),
        ],
        vec![
            plain("warning").color(inks.caution).bold(),
            plain(": unused variable: `frame`"),
        ],
        vec![
            plain("Finished").bold(),
            plain(" dev profile "),
            plain("(1.24 s)").italic(),
        ],
    ])
}

fn cursor(s: &Workspace, inks: &Inks) -> El {
    let config = &s.settings.terminal;
    let mut sample = inks.sample(vec![vec![plain("~/rex ").color(inks.path), plain("$ ")]]);
    sample.cursor = Some(((8, 0), config.cursor_style.shape()));
    section(
        "Cursor",
        [
            card([
                preview(
                    s,
                    "cursor-preview",
                    "Cursor preview: the chosen cursor at a terminal prompt.",
                    inks,
                    &sample,
                ),
                select_row(
                    "cursor-style",
                    "Cursor style",
                    "",
                    &CursorStyle::ALL.map(CursorStyle::name),
                    CursorStyle::ALL
                        .iter()
                        .position(|style| *style == config.cursor_style)
                        .unwrap_or_default(),
                    |s, i, _| {
                        s.set_cursor_style(CursorStyle::ALL[i.min(CursorStyle::ALL.len() - 1)])
                    },
                ),
                setting(
                    "Force style",
                    "Ignore programs' requests to change the cursor's shape.",
                    toggle("cursor-force", "Force style", config.cursor_force, |c| {
                        &mut c.cursor_force
                    }),
                ),
            ]),
            footnote(
                "The cursor is steady and never blinks. A pane without the keyboard shows a hollow block.",
            ),
        ],
    )
}

fn keyboard(s: &Workspace) -> El {
    section(
        "Keyboard",
        [card([setting(
            "Show key sequence hints",
            "The keys pressed so far of a multi-key shortcut, in the toolbar.",
            toggle(
                "sequence-hints",
                "Show key sequence hints",
                s.settings.terminal.key_sequence_hints,
                |c| &mut c.key_sequence_hints,
            ),
        )])],
    )
}

/// The choice at `index` of a selection colour's list.
fn selection_color(index: usize) -> SelectionColor {
    SelectionColor::ALL[index.min(SelectionColor::ALL.len() - 1)]
}

fn selection(s: &Workspace, inks: &Inks) -> El {
    let config = &s.settings.terminal;
    let mut sample = inks.sample(vec![vec![
        plain("On branch "),
        plain("feature/tabs").color(inks.branch),
    ]]);
    // "branch feature/tabs": text in two colours, so the cell-colour choices show.
    sample.selection = Some((0, 3, 21));
    let names = SelectionColor::ALL.map(SelectionColor::name);
    let at = |color: SelectionColor| {
        SelectionColor::ALL
            .iter()
            .position(|c| *c == color)
            .unwrap_or_default()
    };
    section(
        "Selection",
        [
            card([
                preview(
                    s,
                    "selection-preview",
                    "Selection preview: selected terminal text in the chosen colours.",
                    inks,
                    &sample,
                ),
                setting(
                    "Copy on selection",
                    "Copy selected text when you release the pointer.",
                    toggle(
                        "copy-on-selection",
                        "Copy on selection",
                        config.copy_on_selection,
                        |c| &mut c.copy_on_selection,
                    ),
                ),
                setting(
                    "Primary selection",
                    "Selecting also sets the primary selection, and a middle-click pastes it.",
                    toggle(
                        "primary-selection",
                        "Primary selection",
                        config.primary_selection,
                        |c| &mut c.primary_selection,
                    ),
                ),
                select_row(
                    "selection-foreground",
                    "Foreground",
                    "",
                    &names,
                    at(config.selection_foreground),
                    |s, i, _| {
                        s.settings.terminal.selection_foreground = selection_color(i);
                        s.save();
                    },
                ),
                select_row(
                    "selection-background",
                    "Background",
                    "",
                    &names,
                    at(config.selection_background),
                    |s, i, _| {
                        s.settings.terminal.selection_background = selection_color(i);
                        s.save();
                    },
                ),
                setting(
                    "Rounded corners",
                    "",
                    toggle(
                        "selection-rounded",
                        "Rounded corners",
                        config.selection_rounded,
                        |c| &mut c.selection_rounded,
                    ),
                ),
            ]),
            footnote(
                "Theme uses the terminal theme's selection colours; Cell foreground and Cell background use each selected cell's own.",
            ),
            footnote("A middle-click goes to a program that tracks the mouse instead of pasting."),
        ],
    )
}

pub fn terminal(s: &Workspace) -> El {
    let inks = Inks::of(s);
    let saved = &s.settings.font_family;
    let active = s.terminal_family.unwrap_or("IBM Plex Mono (bundled)");
    // The row's helper is one sentence. What terminals are drawn in now goes under the card,
    // and only while that isn't the family the row names.
    let drawn = if *saved != s.launch_saved_font {
        Some(format!(
            "{saved} is saved and applies when you reopen the client. Until then terminals use {active}."
        ))
    } else if s.terminal_family.is_none() {
        Some(format!(
            "{} isn't installed, so terminals use {active}.",
            s.launch_font
        ))
    } else {
        None
    };
    let fallbacks = if s.settings.font_fallbacks.is_empty() {
        "Automatic".to_owned()
    } else {
        s.settings.font_fallbacks.join(", ")
    };
    let shells = s.settings.hosts.iter().enumerate().map(|(i, host)| {
        disclosure(
            &format!("host-shell:{i}"),
            &host.label,
            "",
            host.shell.as_deref().unwrap_or("Server default"),
            move |s, tx| s.perform(Intent::HostShell(i), tx),
        )
    });
    page([
        section(
            "Typography",
            std::iter::once(card([
                preview(
                    s,
                    "typography-preview",
                    "Typography preview: regular, bold and italic terminal text.",
                    &inks,
                    &typography_sample(&inks),
                ),
                disclosure(
                    "font-change",
                    "Font family",
                    "Applies when you reopen the client.",
                    saved,
                    |s, tx| s.perform(Intent::Fonts, tx),
                ),
                setting(
                    "Font size",
                    "From 8 to 48 pt, for every terminal.",
                    stepper(
                        "font",
                        format!("{} pt", s.settings.font_size),
                        |s, _| s.change_font(-1.),
                        |s, _| s.change_font(1.),
                    ),
                ),
                setting(
                    "Fallback fonts",
                    "For glyphs the font lacks, before the system's own fallbacks.",
                    value(fallbacks).truncate().max_w(260.),
                ),
            ]))
            .chain(drawn.map(|words| footnote(words).key("font-drawn"))),
        ),
        section(
            "Layout",
            [card([setting(
                "Pane padding",
                "Space around each terminal's content, from 0 to 32 pt.",
                stepper(
                    "pane-padding",
                    format!("{} pt", s.settings.pane_padding),
                    |s, _| {
                        s.settings.pane_padding = (s.settings.pane_padding - 1.).max(0.);
                        s.save();
                    },
                    |s, _| {
                        s.settings.pane_padding = (s.settings.pane_padding + 1.).min(32.);
                        s.save();
                    },
                ),
            )])],
        ),
        section(
            "Colours",
            [
                card([setting(
                    "Theme",
                    "",
                    value_popup(
                        "terminal-theme",
                        Workspace::theme_label(&s.settings.terminal_theme),
                        Intent::ChooseTheme,
                    ),
                )]),
                footnote(
                    "Declared to Rex for panes this client owns or nobody owns. Follow server declares nothing until this client has declared a theme once; from then on it resets those panes to Rex's colours, once each per connection.",
                ),
            ],
        ),
        cursor(s, &inks),
        keyboard(s),
        selection(s, &inks),
        section(
            "Default shell",
            [
                card(shells),
                footnote(
                    "What new sessions, tabs and splits start on each host. Running terminals keep theirs.",
                ),
            ],
        ),
        super::titles::titles(s),
    ])
}
impl Workspace {
    /// The cursor every terminal draws until its program asks for a shape of its own: saved,
    /// and told to each pane's worker, which shows it at once where no program has chosen.
    pub fn set_cursor_style(&mut self, style: CursorStyle) {
        self.settings.terminal.cursor_style = style;
        for term in self.terminals.values_mut() {
            term.set_cursor_style(style.shape());
        }
        self.save();
    }
}
