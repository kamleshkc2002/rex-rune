//! Settings → Terminal's font picker, its Advanced font settings page, and what the
//! typography settings do to a pane. The pickers list `font_picker::TEST_FAMILIES`, not this
//! machine's fonts.
use super::pane_verb_tests::{Camera, framed_terminal, rect};
use super::*;
use crate::app::{
    font_picker::{FontPick, FontPurpose},
    fonts::Family,
    panel::SettingsSection,
    settings_nav::{Page, SubPage},
    term,
};
use rex_rune::{
    connection::Command,
    settings::typography::{BoldItalic, FontStyle, SpacingUnit, Typography},
};

/// Where a test opens Settings: a section's own page, or Advanced font settings.
#[derive(Clone, Copy)]
enum Place {
    Section(SettingsSection),
    AdvancedFont,
}
const TERMINAL: Place = Place::Section(SettingsSection::Terminal);
const TERMINAL_PAGE: Page = Page::Section(SettingsSection::Terminal);
const ADVANCED: Page = Page::Sub(SubPage::AdvancedFont);

const MONO: [&str; 4] = [
    "DejaVu Sans Mono",
    "Hack Nerd Font Mono",
    "IBM Plex Mono",
    "JetBrainsMono Nerd Font",
];
const ALL: [&str; 9] = [
    "DejaVu Sans Mono",
    "Hack Nerd Font Mono",
    "IBM Plex Mono",
    "Inter",
    "JetBrainsMono Nerd Font",
    "Noto Color Emoji",
    "Noto Sans",
    "Noto Sans CJK JP",
    "Symbols Nerd Font",
];

fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
}

/// Settings open on `section` in a window that fits the dialog.
fn settings_on(section: Place, prepare: impl FnOnce(&mut Workspace)) -> Runtime<Workspace> {
    let mut ws = workspace_hosts(2);
    prepare(&mut ws);
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    match section {
        Place::AdvancedFont => ws.open_advanced_fonts(&mut rune::Tx::default()),
        Place::Section(section) => ws.open_section(section),
    }
    let mut rt = runtime_at(ws, (1440., 1000.));
    // The app's own frame hook: it scrolls a list's selection into view.
    rt.poll = Some(crate::app::poll);
    settle(&mut rt);
    rt
}

fn element(rt: &Runtime<Workspace>, key: &str) -> rune::ElementInfo {
    rt.elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("missing {key}"))
}

fn tap(rt: &mut Runtime<Workspace>, key: &str) {
    rt.update(|_, tx| tx.scroll_into_view(key));
    settle(rt);
    click(rt, key);
    settle(rt);
}

fn pick(rt: &Runtime<Workspace>) -> Option<FontPick> {
    match rt.state.panel.as_ref()?.kind {
        PanelKind::Fonts(pick) => Some(pick),
        _ => None,
    }
}

fn on_settings(rt: &Runtime<Workspace>) -> bool {
    matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::Settings)
    )
}

/// The families the open picker has built rows for, top to bottom.
fn listed(rt: &Runtime<Workspace>) -> Vec<String> {
    let mut rows: Vec<(f32, String)> = rt
        .elements()
        .into_iter()
        .filter(|e| !e.exiting && e.role == "menuitem" && e.path.contains("menu-surface"))
        .map(|e| (e.rect[1], e.label.unwrap_or_default()))
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    rows.into_iter().map(|(_, label)| label).collect()
}

/// Press the picker's row for `family`.
fn choose_family(rt: &mut Runtime<Workspace>, family: &str) {
    let find = |rt: &Runtime<Workspace>| {
        rt.elements()
            .into_iter()
            .find(|e| !e.exiting && e.role == "menuitem" && e.label.as_deref() == Some(family))
            .unwrap_or_else(|| panic!("{family} is not listed"))
    };
    // The list is as tall as its side of the control allows: scroll to a row further down.
    let list = element(rt, "results-scroll").rect;
    for _ in 0..40 {
        let row = find(rt);
        if row.visible[3] >= row.rect[3] - 0.5 {
            break;
        }
        let below = row.rect[1] + row.rect[3] > list[1] + list[3];
        rt.wheel(
            list[0] + list[2] / 2.,
            list[1] + list[3] / 2.,
            if below { -40. } else { 40. },
        );
        for _ in 0..12 {
            rt.frame(1. / 60.);
        }
    }
    let row = find(rt).rect;
    rt.pointer_down(row[0] + row[2] / 2., row[1] + row[3] / 2.);
    rt.pointer_up(row[0] + row[2] / 2., row[1] + row[3] / 2.);
    settle(rt);
}

fn type_text(rt: &mut Runtime<Workspace>, text: &str) {
    for c in text.chars() {
        press(rt, Key::Text(c.to_string()));
    }
    settle(rt);
}

/// Choose `label` from the popup of the control keyed `control`.
fn choose(rt: &mut Runtime<Workspace>, control: &str, label: &str) {
    tap(rt, control);
    let row = rt
        .elements()
        .into_iter()
        .find(|e| !e.exiting && e.role == "menuitem" && e.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("no {label} in the list of {control}"))
        .rect;
    rt.pointer_down(row[0] + row[2] / 2., row[1] + row[3] / 2.);
    rt.pointer_up(row[0] + row[2] / 2., row[1] + row[3] / 2.);
    settle(rt);
}

#[test]
fn the_font_picker_opens_under_its_button_on_monospaced_with_the_font_in_use_selected() {
    let mut rt = settings_on(TERMINAL, |_| {});
    let before = rt.state.settings.font_fallbacks.clone();
    assert_eq!(
        element(&rt, "font-name").text.as_deref(),
        Some("JetBrainsMono Nerd Font"),
        "the row names the saved family"
    );
    tap(&mut rt, "font-choose");
    assert_eq!(
        pick(&rt),
        Some(FontPick {
            purpose: FontPurpose::Terminal,
            mono: true
        })
    );
    // Monospaced lists only families fontconfig calls fixed pitch, and the bundled one.
    assert_eq!(listed(&rt), MONO);
    let entries = rt.state.entries();
    let current: Vec<&str> = entries
        .iter()
        .filter(|e| e.current)
        .map(|e| e.label.as_str())
        .collect();
    assert_eq!(current, ["JetBrainsMono Nerd Font"], "only it is checked");
    assert_eq!(rt.state.panel.as_ref().unwrap().selected, 3);

    // Against the button, over the page, which stays where it was; the field has the keys.
    let (button, list) = (rect(&rt, "font-choose"), rect(&rt, "menu-surface"));
    let gap = list[1] - (button[1] + button[3]);
    assert!((0. ..=8.).contains(&gap), "{list:?} under {button:?}");
    assert!(
        (list[0] + list[2] - button[0] - button[2]).abs() < 1.,
        "trailing edges meet: {list:?} and {button:?}"
    );
    assert!(has(&rt, "dialog") && element(&rt, "dialog").inert);
    assert!(focused(&rt).unwrap().contains("popup-search"));

    // Tab lists every family, and the selection stays on the font in use.
    press(&mut rt, Key::Tab);
    settle(&mut rt);
    assert_eq!(pick(&rt).map(|p| p.mono), Some(false));
    assert_eq!(listed(&rt), ALL);
    assert_eq!(rt.state.panel.as_ref().unwrap().selected, 4);
    assert!(focused(&rt).unwrap().contains("popup-search"));

    // Typing searches the scope on screen, whatever the case.
    type_text(&mut rt, "NOTO");
    assert_eq!(
        listed(&rt),
        ["Noto Color Emoji", "Noto Sans", "Noto Sans CJK JP"]
    );
    press(&mut rt, Key::Tab);
    settle(&mut rt);
    assert!(listed(&rt).is_empty());
    assert_eq!(
        element(&rt, "font-none").text.as_deref(),
        Some("No monospaced fonts match. Tab lists all fonts.")
    );
    // The segments press too, and leave the keyboard in the field.
    tap(&mut rt, "font-scope:all");
    assert_eq!(listed(&rt).len(), 3);
    assert!(focused(&rt).unwrap().contains("popup-search"));
    tap(&mut rt, "font-scope:mono");
    assert!(listed(&rt).is_empty());

    // Escape closes the picker only, and the keyboard goes back to the button.
    press(&mut rt, Key::Escape);
    settle(&mut rt);
    assert!(on_settings(&rt));
    assert!(
        focused(&rt).is_some_and(|path| path.contains("font-choose")),
        "{:?}",
        focused(&rt)
    );
    assert_eq!(rt.state.settings.font_family, "JetBrainsMono Nerd Font");
    assert_eq!(
        rt.state.settings.font_fallbacks, before,
        "picker cancellation changes no preferences"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn choosing_a_font_saves_it_for_the_next_launch_and_asks_no_server() {
    let mut ws = workspace_hosts(2);
    let requests = capture_host(&mut ws, 0);
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    ws.open_section(SettingsSection::Terminal);
    let mut rt = runtime_at(ws, (1440., 1000.));
    settle(&mut rt);
    assert!(shown(
        &rt,
        "JetBrainsMono Nerd Font isn't installed, so terminals use IBM Plex Mono (bundled)."
    ));

    tap(&mut rt, "font-choose");
    type_text(&mut rt, "hack");
    assert_eq!(listed(&rt), ["Hack Nerd Font Mono"]);
    press(&mut rt, Key::Enter);
    settle(&mut rt);
    assert!(on_settings(&rt), "back on the page it was opened from");
    assert_eq!(rt.state.settings_section, SettingsSection::Terminal);
    assert_eq!(rt.state.settings.font_family, "Hack Nerd Font Mono");
    assert_eq!(
        element(&rt, "font-name").text.as_deref(),
        Some("Hack Nerd Font Mono")
    );
    // The row's helper is one short sentence; what is drawn until then is the footnote's.
    assert!(shown(&rt, "Applies when you reopen the client."));
    assert_eq!(
        element(&rt, "font-drawn").text.as_deref(),
        Some(
            "Hack Nerd Font Mono is saved and applies when you reopen the client. Until then terminals use IBM Plex Mono (bundled)."
        )
    );
    let notices: Vec<_> = rt.state.notices.iter().map(|n| n.text.as_str()).collect();
    assert_eq!(
        notices,
        ["Hack Nerd Font Mono saved. Reopen the client to use it."]
    );
    let saved = Settings::load(&rt.state.settings_path).unwrap();
    assert_eq!(saved.font_family, "Hack Nerd Font Mono");
    assert!(saved.font_fallbacks.is_empty());

    // It is the checked one now; choosing it again says nothing new.
    tap(&mut rt, "font-choose");
    assert_eq!(rt.state.panel.as_ref().unwrap().selected, 1);
    press(&mut rt, Key::Enter);
    settle(&mut rt);
    assert_eq!(rt.state.notices.len(), 1);
    // Back to the family this launch started with: nothing is pending, so no notice.
    tap(&mut rt, "font-choose");
    choose_family(&mut rt, "JetBrainsMono Nerd Font");
    assert_eq!(rt.state.settings.font_family, "JetBrainsMono Nerd Font");
    assert_eq!(rt.state.notices.len(), 1);
    assert!(
        requests.try_iter().next().is_none(),
        "font preferences are entirely local"
    );
    std::fs::remove_file(&rt.state.settings_path).unwrap();
}

#[test]
fn a_proportional_font_in_use_opens_the_picker_on_all_fonts() {
    let mut rt = settings_on(TERMINAL, |ws| ws.settings.font_family = "Noto Sans".into());
    tap(&mut rt, "font-choose");
    assert_eq!(pick(&rt).map(|p| p.mono), Some(false));
    assert_eq!(listed(&rt), ALL);
    assert_eq!(rt.state.panel.as_ref().unwrap().selected, 6);
    // A family that isn't installed at all is in neither list: Monospaced, nothing checked.
    let mut rt = settings_on(TERMINAL, |ws| ws.settings.font_family = "MonoLisa".into());
    tap(&mut rt, "font-choose");
    assert_eq!(pick(&rt).map(|p| p.mono), Some(true));
    assert!(rt.state.entries().iter().all(|e| !e.current));
    assert_eq!(rt.state.panel.as_ref().unwrap().selected, 0);
}

#[test]
fn a_long_list_builds_only_the_rows_near_the_window_and_reveals_the_font_in_use() {
    let mut rt = settings_on(TERMINAL, |ws| {
        ws.fonts.catalog = Some(
            (0..3000)
                .map(|i| Family::new(&format!("Family {i:04}"), i % 10 == 0))
                .collect(),
        );
        ws.settings.font_family = "Family 2500".into();
    });
    tap(&mut rt, "font-choose");
    assert_eq!(rt.state.entries().len(), 300, "the monospaced tenth");
    press(&mut rt, Key::Tab);
    settle(&mut rt);
    assert_eq!(rt.state.entries().len(), 3000);
    assert_eq!(rt.state.panel.as_ref().unwrap().selected, 2500);
    let built = listed(&rt);
    assert!(built.len() < 120, "{} of 3000 rows were built", built.len());
    // The font in use is in view, inside the list.
    let inside = |rt: &Runtime<Workspace>, family: &str| {
        let view = rect(rt, "results-scroll");
        rt.elements()
            .into_iter()
            .find(|e| !e.exiting && e.role == "menuitem" && e.label.as_deref() == Some(family))
            .is_some_and(|row| {
                row.rect[1] >= view[1] - 1. && row.rect[1] + row.rect[3] <= view[1] + view[3] + 1.
            })
    };
    assert!(inside(&rt, "Family 2500"), "{built:?}");
    let list = rect(&rt, "menu-surface");
    assert!(
        list[3] < 450.,
        "about ten rows plus the family sample: {list:?}"
    );
    let [_, _, _, thumb] = rt.state.switcher_fx.thumb.get();
    assert!(thumb > 0., "a thumb says there is more");

    // Page Down moves a screenful and keeps the selection in view; arrows one row.
    press(&mut rt, Key::PageDown);
    press(&mut rt, Key::Down);
    settle(&mut rt);
    assert_eq!(rt.state.panel.as_ref().unwrap().selected, 2511);
    assert!(inside(&rt, "Family 2511"));
    for _ in 0..80 {
        press(&mut rt, Key::PageDown);
    }
    settle(&mut rt);
    assert_eq!(rt.state.panel.as_ref().unwrap().selected, 2999);
    assert!(inside(&rt, "Family 2999"));
    press(&mut rt, Key::Enter);
    settle(&mut rt);
    assert_eq!(rt.state.settings.font_family, "Family 2999");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn advanced_font_settings_is_a_page_of_terminal_that_back_and_escape_leave() {
    let mut rt = settings_on(TERMINAL, |_| {});
    let selected = |rt: &Runtime<Workspace>| element(rt, "settings-section:terminal").fill.a > 0.;
    assert!(selected(&rt));
    tap(&mut rt, "open-advanced-font");
    assert_eq!(rt.state.settings_page(), ADVANCED);
    assert!(shown(&rt, "Advanced font settings"));
    assert!(selected(&rt), "Terminal stays selected in the section list");
    for key in [
        "settings-back",
        "advanced-font-preview",
        "line-height-larger",
        "char-spacing-larger",
        "char-spacing-unit",
        "bold-italic",
        "fallback-add",
    ] {
        assert!(has(&rt, key), "{key} is on the page");
    }
    assert!(
        !has(&rt, "font-choose"),
        "the Terminal page is not under it"
    );

    // Escape goes back to Terminal, Settings still open, the keyboard on the row.
    press(&mut rt, Key::Escape);
    settle(&mut rt);
    assert!(on_settings(&rt));
    assert_eq!(rt.state.settings_page(), TERMINAL_PAGE);
    assert!(
        focused(&rt).is_some_and(|path| path.contains("open-advanced-font")),
        "{:?}",
        focused(&rt)
    );
    // Return on the row opens it again; Back leaves it as Escape does.
    press(&mut rt, Key::Enter);
    settle(&mut rt);
    assert_eq!(rt.state.settings_page(), ADVANCED);
    // The row went with its page: the keyboard is on Back, which Return presses.
    assert!(
        focused(&rt).is_some_and(|path| path.contains("settings-back")),
        "{:?}",
        focused(&rt)
    );
    press(&mut rt, Key::Enter);
    settle(&mut rt);
    assert_eq!(rt.state.settings_page(), TERMINAL_PAGE);
    assert!(focused(&rt).is_some_and(|path| path.contains("open-advanced-font")));
    tap(&mut rt, "open-advanced-font");
    tap(&mut rt, "settings-back");
    assert_eq!(rt.state.settings_page(), TERMINAL_PAGE);
    assert!(on_settings(&rt));
    // On a section's own page Escape closes Settings, as it always has.
    press(&mut rt, Key::Escape);
    settle(&mut rt);
    assert!(rt.state.panel.is_none());

    // Another section, then Terminal again: the section's own page, not the one under it.
    let mut rt = settings_on(Place::AdvancedFont, |_| {});
    tap(&mut rt, "settings-section:interface");
    tap(&mut rt, "settings-section:terminal");
    assert_eq!(rt.state.settings_page(), TERMINAL_PAGE);
    assert!(has(&rt, "font-choose"));
}

/// What a pane's worker was told its size is, in order: (columns, rows, cell width, cell
/// height).
fn resizes(sent: &flume::Receiver<HostCommand>) -> Vec<(u16, u16, u16, u16)> {
    sent.try_iter()
        .filter_map(|c| match c {
            HostCommand::Terminal {
                command: Command::Resize(cols, rows, w, h),
                ..
            } => Some((cols, rows, w, h)),
            _ => None,
        })
        .collect()
}

#[test]
fn line_height_and_character_spacing_change_the_rows_and_columns_a_server_is_told() {
    let _families = rune::text::with_families("Inter", "Inter");
    let (mut ws, _host) = tabbed_workspace("single");
    let (sender, sent) = flume::unbounded();
    framed_terminal(&mut ws, "block:a", &sender, b"$ ");
    // This client sizes the pane.
    for term in ws.terminals.values_mut() {
        term.connection.model.lock().unwrap().owner = true;
    }
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    ws.open_advanced_fonts(&mut rune::Tx::default());
    let mut rt = runtime_at(ws, (1440., 1000.));
    settle(&mut rt);
    let key = rt.state.term_key("block:a");
    let pane = rt.state.terminals[&key].rect;
    // Rows/columns use the exact local cell; only the wire cell dimensions round up.
    let expected_resize = |(cw, ch): (f32, f32)| {
        (
            (pane[2] / cw).floor() as u16,
            (pane[3] / ch).floor() as u16,
            cw.ceil() as u16,
            ch.ceil() as u16,
        )
    };
    let plain = *resizes(&sent).last().expect("the pane's size");
    assert_eq!(plain.3, 21);
    assert_eq!(plain, expected_resize(rt.state.metrics().cell));

    // +20%: four steps of the stepper.
    for _ in 0..4 {
        tap(&mut rt, "line-height-larger");
    }
    assert_eq!(rt.state.settings.terminal.font.line_height, 20);
    assert_eq!(rt.state.metrics().cell.1, 26.);
    let told = resizes(&sent);
    assert_eq!(told.len(), 4, "one size for each step: {told:?}");
    let tall = told[3];
    assert_eq!(tall.3, 26, "the cell's new height in points");
    assert_eq!(tall, expected_resize(rt.state.metrics().cell));
    assert!(tall.1 < plain.1, "fewer rows: {plain:?} then {tall:?}");
    // The preview's three lines are three of the new rows.
    let sample = rect(&rt, "advanced-font-preview-sample");
    assert!((sample[3] - 3. * 26.).abs() < 0.5, "{sample:?}");

    // Two points of character spacing: wider cells, so fewer columns, and the same rows.
    tap(&mut rt, "char-spacing-larger");
    tap(&mut rt, "char-spacing-larger");
    let told = resizes(&sent);
    assert_eq!(told.len(), 2, "{told:?}");
    let wide = told[1];
    assert_eq!(wide, expected_resize(rt.state.metrics().cell));
    assert_eq!((wide.1, wide.3), (tall.1, tall.3), "the same rows");
    let font = rt.state.settings.terminal.font.clone();
    assert_eq!(font.spacing, 2);
    let before = term::metrics_for(15., &Typography::default()).cell.0;
    let after = term::metrics_for(15., &font).cell.0;
    assert!((after - before - 2.).abs() < 1e-3);
    assert!((pane[2] / after).floor() < (pane[2] / before).floor());

    // A step at the end of the range changes nothing and tells the server nothing.
    rt.update(|s, _| s.edit_typography(|font| font.line_height = 100));
    settle(&mut rt);
    resizes(&sent);
    tap(&mut rt, "line-height-larger");
    assert_eq!(rt.state.settings.terminal.font.line_height, 100);
    assert!(resizes(&sent).is_empty());

    let saved = Settings::load(&rt.state.settings_path)
        .unwrap()
        .terminal
        .font;
    assert_eq!((saved.line_height, saved.spacing), (100, 2));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_pane_this_client_does_not_size_is_never_told_a_size() {
    let (mut ws, _host) = tabbed_workspace("single");
    let (sender, sent) = flume::unbounded();
    framed_terminal(&mut ws, "block:a", &sender, b"$ ");
    let mut rt = runtime(ws);
    settle(&mut rt);
    rt.update(|s, _| {
        s.edit_typography(|font| {
            font.line_height = 40;
            font.spacing = 3;
        })
    });
    settle(&mut rt);
    assert!(resizes(&sent).is_empty(), "another client owns its size");
    // It is drawn on the new grid all the same: its cursor is one of the new cells.
    assert_eq!(rt.state.metrics().cell.1, 30.);
    let cursor = rect(&rt, "gblock:a/cursor");
    assert_eq!(cursor[3], 30., "{cursor:?} is a cell tall");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_style_spacing_unit_and_bold_italic_choices_save_and_apply_at_once() {
    let mut rt = settings_on(TERMINAL, |_| {});
    choose(&mut rt, "font-style", "Bold Italic");
    assert_eq!(rt.state.settings.terminal.font.style, FontStyle::BoldItalic);
    let style = rt.state.metrics().style;
    assert_eq!((style.weight, style.italic), (700., true));
    assert!(on_settings(&rt));

    tap(&mut rt, "open-advanced-font");
    // The value says its unit, one name for it everywhere: points are "pt".
    let spacing = |rt: &Runtime<Workspace>| {
        let stepper = element(rt, "char-spacing-smaller").rect;
        rt.elements()
            .into_iter()
            .filter(|e| {
                !e.exiting && e.rect[0] > stepper[0] && (e.rect[1] - stepper[1]).abs() < 12.
            })
            .find_map(|e| e.text.filter(|t| t.ends_with("pt") || t.ends_with('%')))
            .expect("the spacing's value")
    };
    assert_eq!(spacing(&rt), "0 pt");
    assert!(shown(&rt, "Points, from \u{2212}3 to +12."));
    assert!(!shown(&rt, "px"));
    tap(&mut rt, "char-spacing-larger");
    assert_eq!(spacing(&rt), "+1 pt");
    // The two steppers are alone in their rows' trailing slots, so they share both edges.
    let edges = |rt: &Runtime<Workspace>, key: &str| {
        let (less, more) = (
            element(rt, &format!("{key}-smaller")).rect,
            element(rt, &format!("{key}-larger")).rect,
        );
        (less[0], more[0] + more[2])
    };
    assert_eq!(edges(&rt, "line-height"), edges(&rt, "char-spacing"));
    choose(&mut rt, "char-spacing-unit", "Percent");
    let font = rt.state.settings.terminal.font.clone();
    assert_eq!((font.spacing, font.spacing_unit), (1, SpacingUnit::Percent));
    assert_eq!(spacing(&rt), "+1%");
    assert!(shown(&rt, "A share of the character's width"));
    assert_eq!(edges(&rt, "line-height"), edges(&rt, "char-spacing"));
    tap(&mut rt, "char-spacing-larger");
    assert_eq!(rt.state.settings.terminal.font.spacing, 6, "5% a step");
    tap(&mut rt, "char-spacing-smaller");
    tap(&mut rt, "char-spacing-smaller");
    assert_eq!(rt.state.settings.terminal.font.spacing, -4);

    choose(&mut rt, "bold-italic", "Italic only");
    let m = rt.state.metrics();
    assert!(!m.bold && m.italic);
    assert_eq!(rt.state.settings_page(), ADVANCED);

    let saved = Settings::load(&rt.state.settings_path)
        .unwrap()
        .terminal
        .font;
    assert_eq!(
        saved,
        Typography {
            style: FontStyle::BoldItalic,
            line_height: 0,
            spacing: -4,
            spacing_unit: SpacingUnit::Percent,
            bold_italic: BoldItalic::ItalicOnly,
        }
    );
    // The Terminal page's row says what is away from its default.
    press(&mut rt, Key::Escape);
    settle(&mut rt);
    assert!(shown(&rt, "Spacing \u{2212}4% · Italic only"));
    // More than two would not fit the row: they are counted.
    rt.update(|s, _| s.edit_typography(|font| font.line_height = 10));
    settle(&mut rt);
    assert!(shown(&rt, "3 changes"));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn fallback_fonts_are_added_from_all_fonts_and_kept_in_the_order_given() {
    let mut rt = settings_on(Place::AdvancedFont, |_| {});
    assert!(!has(&rt, "fallback:0"), "none to begin with");
    assert!(shown(
        &rt,
        "A change to the list applies when you reopen the client."
    ));

    for family in ["Noto Color Emoji", "Symbols Nerd Font", "Noto Sans CJK JP"] {
        tap(&mut rt, "fallback-add");
        assert_eq!(
            pick(&rt),
            Some(FontPick {
                purpose: FontPurpose::Fallback,
                mono: false
            }),
            "a fallback's picker opens on All fonts"
        );
        assert_eq!(listed(&rt), ALL);
        choose_family(&mut rt, family);
        assert!(on_settings(&rt));
        assert_eq!(rt.state.settings_page(), ADVANCED);
    }
    let order = |rt: &Runtime<Workspace>| rt.state.settings.font_fallbacks.clone();
    assert_eq!(
        order(&rt),
        ["Noto Color Emoji", "Symbols Nerd Font", "Noto Sans CJK JP"]
    );
    assert!(has(&rt, "fallback:2") && !has(&rt, "fallback:3"));
    assert!(shown(
        &rt,
        "The list changed and applies when you reopen the client."
    ));
    assert!(rt.state.notices.is_empty(), "the page says it");

    // The picker can't add a family twice, or the terminal font: both say why.
    tap(&mut rt, "fallback-add");
    let entries = rt.state.entries();
    let unavailable: Vec<(&str, &str)> = entries
        .iter()
        .filter(|e| !e.enabled)
        .map(|e| (e.label.as_str(), e.detail.as_str()))
        .collect();
    assert_eq!(
        unavailable,
        [
            ("JetBrainsMono Nerd Font", "Terminal font"),
            ("Noto Color Emoji", "Added"),
            ("Noto Sans CJK JP", "Added"),
            ("Symbols Nerd Font", "Added"),
        ]
    );
    assert_eq!(
        rt.state.panel.as_ref().unwrap().selected,
        0,
        "the selection starts on a family that can be added"
    );
    choose_family(&mut rt, "Noto Color Emoji");
    assert!(pick(&rt).is_some(), "an added family does nothing");
    press(&mut rt, Key::Escape);
    settle(&mut rt);
    assert_eq!(order(&rt).len(), 3);
    assert!(focused(&rt).is_some_and(|path| path.contains("fallback-add")));

    // Move up, move down and remove; the ends of the list can't move past them.
    tap(&mut rt, "fallback-up:Noto Sans CJK JP");
    assert_eq!(
        order(&rt),
        ["Noto Color Emoji", "Noto Sans CJK JP", "Symbols Nerd Font"]
    );
    tap(&mut rt, "fallback-up:Noto Color Emoji");
    tap(&mut rt, "fallback-down:Symbols Nerd Font");
    assert_eq!(order(&rt).len(), 3);
    assert_eq!(order(&rt)[0], "Noto Color Emoji");
    tap(&mut rt, "fallback-down:Noto Color Emoji");
    assert_eq!(
        order(&rt),
        ["Noto Sans CJK JP", "Noto Color Emoji", "Symbols Nerd Font"]
    );
    tap(&mut rt, "fallback-remove:Noto Color Emoji");
    assert_eq!(order(&rt), ["Noto Sans CJK JP", "Symbols Nerd Font"]);
    assert!(!has(&rt, "fallback:2"));

    // From the keyboard: a family moved to an end keeps the keyboard, on the button that
    // moves it back, and removing one hands the keyboard to the family that takes its place.
    let on =
        |rt: &Runtime<Workspace>, key: &str| focused(rt).is_some_and(|path| path.contains(key));
    while !on(&rt, "fallback-up:Symbols Nerd Font") {
        press(&mut rt, Key::Tab);
    }
    press(&mut rt, Key::Enter);
    settle(&mut rt);
    assert_eq!(order(&rt), ["Symbols Nerd Font", "Noto Sans CJK JP"]);
    assert!(
        on(&rt, "fallback-down:Symbols Nerd Font"),
        "{:?}",
        focused(&rt)
    );
    press(&mut rt, Key::Tab);
    assert!(on(&rt, "fallback-remove:Symbols Nerd Font"));
    press(&mut rt, Key::Enter);
    settle(&mut rt);
    assert_eq!(order(&rt), ["Noto Sans CJK JP"]);
    assert!(
        on(&rt, "fallback-remove:Noto Sans CJK JP"),
        "{:?}",
        focused(&rt)
    );
    press(&mut rt, Key::Enter);
    settle(&mut rt);
    assert!(order(&rt).is_empty());
    assert!(on(&rt, "fallback-add"), "{:?}", focused(&rt));
    for family in ["Noto Sans CJK JP", "Symbols Nerd Font"] {
        tap(&mut rt, "fallback-add");
        choose_family(&mut rt, family);
    }

    let saved = Settings::load(&rt.state.settings_path).unwrap();
    assert_eq!(saved.font_fallbacks, order(&rt));
    assert_eq!(saved.font_family, "JetBrainsMono Nerd Font");
    // The Terminal page counts them.
    press(&mut rt, Key::Escape);
    settle(&mut rt);
    assert!(shown(&rt, "2 fallbacks"));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_fallback_that_is_not_installed_says_so_and_the_terminal_font_leaves_the_list() {
    let mut rt = settings_on(Place::AdvancedFont, |ws| {
        ws.settings.font_fallbacks = vec!["Hack Nerd Font Mono".into(), "Gone Sans".into()];
    });
    assert!(!has(&rt, "fallback-missing:0"));
    assert_eq!(
        element(&rt, "fallback-missing:1").label.as_deref(),
        Some("Not installed")
    );
    // Hack becomes the terminal font: it can't be its own fallback.
    press(&mut rt, Key::Escape);
    settle(&mut rt);
    tap(&mut rt, "font-choose");
    choose_family(&mut rt, "Hack Nerd Font Mono");
    assert_eq!(rt.state.settings.font_family, "Hack Nerd Font Mono");
    assert_eq!(rt.state.settings.font_fallbacks, ["Gone Sans"]);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn font_fallback_on_the_command_line_still_overrides_the_saved_list() {
    let endpoint = "unix:///nonexistent/rex-test.sock";
    let options = Options::parse_from([
        "rex-rune",
        "--endpoint",
        endpoint,
        "--font-fallback",
        "Noto Sans CJK JP",
        "--font-fallback",
        "Inter",
    ]);
    let settings = Settings {
        hosts: vec![Host {
            label: "test".into(),
            endpoint: endpoint.into(),
            remote_control: false,
            collapsed: false,
            shell: None,
        }],
        font_fallbacks: vec!["Noto Color Emoji".into()],
        ..Settings::default()
    };
    // What the launch loads: the flags' list, in the order given, not the saved one.
    assert_eq!(
        options.fonts(&settings),
        (
            "JetBrainsMono Nerd Font",
            &["Noto Sans CJK JP".to_owned(), "Inter".to_owned()][..]
        )
    );
    let handle = rune::App::new(AppSpec::new("Rex", (1024., 700.), placeholder, view)).handle();
    let path = std::env::temp_dir().join(format!("rex-font-override-{}.json", std::process::id()));
    let mut ws = Workspace::new(options, settings, path, handle, None);
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    ws.open_advanced_fonts(&mut rune::Tx::default());
    let mut rt = runtime_at(ws, (1440., 1000.));
    settle(&mut rt);
    // The page lists what is saved, and says this launch uses something else.
    assert_eq!(listed_fallbacks(&rt), ["Noto Color Emoji"]);
    assert_eq!(
        element(&rt, "fallback-override").text.as_deref(),
        Some(
            "This launch was started with --font-fallback, so it uses Noto Sans CJK JP, Inter in place of the list."
        )
    );
    // Editing the list saves the list and never the flags.
    tap(&mut rt, "fallback-add");
    choose_family(&mut rt, "Symbols Nerd Font");
    let saved = Settings::load(&rt.state.settings_path).unwrap();
    assert_eq!(
        saved.font_fallbacks,
        ["Noto Color Emoji", "Symbols Nerd Font"]
    );
    std::fs::remove_file(&rt.state.settings_path).ok();

    // Without the flag nothing is said.
    let rt = settings_on(Place::AdvancedFont, |_| {});
    assert!(!has(&rt, "fallback-override"));
}

#[test]
fn advanced_controls_are_searchable_and_pickers_fit_the_minimum_window() {
    let mut ws = workspace_hosts(1);
    ws.settings.font_family = "A very long missing family with an apostrophe's name".into();
    ws.settings.font_fallbacks = vec!["Gone fallback with a long family name".into()];
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    let mut rt = runtime_at(ws, (640., 480.));
    rt.poll = Some(crate::app::poll);
    settle(&mut rt);
    let mut camera = Camera::new();
    for (query, label, control) in [
        ("line height", "Line height", "line-height-larger"),
        (
            "character spacing",
            "Character spacing",
            "char-spacing-larger",
        ),
        ("render bold", "Render bold and italic", "bold-italic"),
        ("fallback font", "Add a fallback font", "fallback-add"),
    ] {
        rt.update(|s, tx| {
            s.settings_nav.search.text = query.into();
            s.settings_search_changed();
            let at = s
                .settings_nav
                .hits()
                .iter()
                .position(|h| h.label == label)
                .expect("indexed kit row");
            s.open_settings_hit(at, tx);
        });
        settle(&mut rt);
        assert_eq!(rt.state.settings_page(), ADVANCED);
        let r = element(&rt, control).rect;
        assert!(
            r[0] >= 0. && r[0] + r[2] <= 640. && r[1] >= 0. && r[1] + r[3] <= 480.,
            "{control}: {r:?}"
        );
    }
    tap(&mut rt, "fallback-add");
    let r = element(&rt, "menu-surface").rect;
    let header = element(&rt, "settings-header").rect;
    assert!(
        r[1] >= header[1] + header[3] - 1. && r[1] + r[3] <= 480.,
        "{r:?}"
    );
    assert!(focused(&rt).unwrap().contains("popup-search"));
    press(&mut rt, Key::Escape);
    settle(&mut rt);
    assert!(focused(&rt).unwrap().contains("fallback-add"));
    assert_eq!(rt.state.settings_nav.search.text, "fallback font");
    rt.update(|s, _| {
        s.clear_settings_search();
        s.navigate(SettingsSection::Terminal);
    });
    settle(&mut rt);
    tap(&mut rt, "font-choose");
    let r = element(&rt, "menu-surface").rect;
    assert!(
        r[0] >= 0. && r[0] + r[2] <= 640. && r[1] + r[3] <= 480.,
        "{r:?}"
    );
    press(&mut rt, Key::Escape);
    settle(&mut rt);
    if let Some(camera) = &mut camera {
        camera.snap(&mut rt, "font-long-missing-minimum");
    }
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_launch_font_override_is_distinct_from_the_saved_family() {
    let rt = settings_on(TERMINAL, |s| {
        s.terminal_family = Some("Inter");
        s.launch_font = "Inter".into();
    });
    assert!(shown(
        &rt,
        "This launch uses --font Inter instead of the saved family; terminals use Inter."
    ));
    assert_eq!(
        element(&rt, "font-name").text.as_deref(),
        Some("JetBrainsMono Nerd Font")
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

/// The fallback rows' names, top to bottom.
fn listed_fallbacks(rt: &Runtime<Workspace>) -> Vec<String> {
    (0..)
        .map_while(|index| {
            has(rt, &format!("fallback:{index}")).then(|| {
                element_text(rt, &format!("fallback:{index}"))
                    .into_iter()
                    .nth(1)
                    .unwrap_or_default()
            })
        })
        .collect()
}

/// The app's own fonts, for the camera (see `terminal_settings_tests::app_fonts`).
fn app_fonts() -> rune::text::TextSystem {
    let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
    rune::text::TextSystem::new(fonts.files, fonts.families)
}

/// The font settings as drawn: the Typography card, the picker on each scope, and the
/// Advanced font settings page plain and with its settings changed (see `Camera`; run it
/// alone: `-- the_font_settings_draw --test-threads=1`).
#[test]
fn the_font_settings_draw_their_picker_and_pages() {
    let on = Camera::new().is_some();
    if on {
        drop(app_fonts());
    }
    let studio = |section: Place, prepare: fn(&mut Workspace)| {
        let (mut ws, _host) = tabbed_workspace("single");
        let (sender, _sent) = flume::unbounded();
        framed_terminal(
            &mut ws,
            "block:a",
            &sender,
            b"$ cargo test\r\n\x1b[1mrunning 12 tests\x1b[0m\r\n\xe2\x94\x8c\xe2\x94\x80\xe2\x94\x80\xe2\x94\x90 \xe2\x96\x88\xe2\x96\x88\r\n\xe2\x94\x94\xe2\x94\x80\xe2\x94\x80\xe2\x94\x98 \xe2\x96\x88\xe2\x96\x88",
        );
        if on {
            ws.settings.font_family = "IBM Plex Mono".into();
            ws.launch_font = "IBM Plex Mono".into();
            ws.launch_saved_font = "IBM Plex Mono".into();
            ws.terminal_family = Some("IBM Plex Mono");
        }
        prepare(&mut ws);
        ws.perform(Intent::Settings, &mut rune::Tx::default());
        match section {
            Place::AdvancedFont => ws.open_advanced_fonts(&mut rune::Tx::default()),
            Place::Section(section) => ws.open_section(section),
        }
        let size = if std::env::var_os("REX_M26_MINIMUM").is_some() {
            (640., 480.)
        } else {
            (1024., 700.)
        };
        let env = Env::new(true, 2.);
        let mut rt = if on {
            Runtime::with_text(ws, view, size, env, app_fonts())
        } else {
            Runtime::new(ws, view, size, env)
        };
        rt.on_key = Some(crate::app::input::on_key);
        rt.poll = Some(crate::app::poll);
        settle(&mut rt);
        rt
    };
    let snap = |camera: &mut Option<Camera>, rt: &mut Runtime<Workspace>, name: &str| {
        if let Some(camera) = camera {
            camera.snap(rt, name);
        }
    };

    let mut rt = studio(TERMINAL, |_| {});
    let mut camera = Camera::new();
    snap(&mut camera, &mut rt, "font-typography-card");
    tap(&mut rt, "font-choose");
    snap(&mut camera, &mut rt, "font-picker-monospaced");
    press(&mut rt, Key::Tab);
    settle(&mut rt);
    snap(&mut camera, &mut rt, "font-picker-all");
    type_text(&mut rt, "noto");
    snap(&mut camera, &mut rt, "font-picker-search");
    press(&mut rt, Key::Tab);
    settle(&mut rt);
    snap(&mut camera, &mut rt, "font-picker-none");
    let list = rect(&rt, "menu-surface");
    assert!(
        list[1] >= 0. && list[1] + list[3] <= 1400.,
        "the picker is on screen: {list:?}"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();

    let mut rt = studio(TERMINAL, |ws| {
        ws.fonts.catalog = Some(
            (0..600)
                .map(|i| Family::new(&format!("Family {i:03} Mono"), i % 3 == 0))
                .collect(),
        );
        ws.settings.font_family = "Family 300 Mono".into();
    });
    let mut camera = Camera::new();
    tap(&mut rt, "font-choose");
    snap(&mut camera, &mut rt, "font-picker-long");
    std::fs::remove_file(&rt.state.settings_path).ok();

    let mut rt = studio(TERMINAL, |s| {
        s.settings.font_family = "A very long missing family with an apostrophe\'s name".into();
        s.font_catalog();
    });
    snap(&mut Camera::new(), &mut rt, "font-long-missing");
    std::fs::remove_file(&rt.state.settings_path).ok();

    let mut rt = studio(Place::AdvancedFont, |_| {});
    let mut camera = Camera::new();
    snap(&mut camera, &mut rt, "font-advanced");
    tap(&mut rt, "fallback-add");
    snap(&mut camera, &mut rt, "font-fallback-picker");
    std::fs::remove_file(&rt.state.settings_path).ok();

    let mut rt = studio(Place::AdvancedFont, |ws| {
        ws.settings.font_fallbacks = vec![
            "Noto Color Emoji".into(),
            "Symbols Nerd Font".into(),
            "Gone Sans".into(),
        ];
        ws.settings.terminal.font = Typography {
            style: FontStyle::Regular,
            line_height: 20,
            spacing: 20,
            spacing_unit: SpacingUnit::Percent,
            bold_italic: BoldItalic::BoldOnly,
        };
    });
    let mut camera = Camera::new();
    snap(&mut camera, &mut rt, "font-advanced-changed");
    rt.update(|_, tx| tx.scroll_into_view("fallback:2"));
    settle(&mut rt);
    snap(&mut camera, &mut rt, "font-fallback-list");
    tap(&mut rt, "char-spacing-unit");
    snap(&mut camera, &mut rt, "font-spacing-unit");
    press(&mut rt, Key::Escape);
    press(&mut rt, Key::Escape);
    settle(&mut rt);
    snap(&mut camera, &mut rt, "font-typography-card-changed");
    press(&mut rt, Key::Escape);
    settle(&mut rt);
    assert!(rt.state.panel.is_none());
    snap(&mut camera, &mut rt, "font-pane-changed");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

/// The real renderer, selection and forced cursor on a synthetic grid, never a live endpoint.
#[test]
fn the_adjusted_grid_draws_wide_combining_selection_and_forced_cursor() {
    let on = Camera::new().is_some();
    if on {
        drop(app_fonts());
    }
    for (name, adjusted) in [("grid-default", false), ("grid-metrics-20", true)] {
        let (mut ws, _host) = tabbed_workspace("single");
        ws.settings.terminal.cursor_force = true;
        ws.settings.terminal.cursor_style = rex_rune::settings::terminal::CursorStyle::Bar;
        if adjusted {
            ws.settings.terminal.font.line_height = 20;
            ws.settings.terminal.font.spacing = 20;
            ws.settings.terminal.font.spacing_unit = SpacingUnit::Percent;
        }
        let (sender, _sent) = flume::unbounded();
        framed_terminal(&mut ws, "block:a", &sender,
            "┌─────────┬─────────┐ ██▐▌\r\n│ 世界 e\u{301}  │ aligned │\r\n└─────────┴─────────┘\r\n\u{1b}[1mbold\u{1b}[0m \u{1b}[3mitalic\u{1b}[0m $ \u{1b}[2 q".as_bytes());
        let size = if std::env::var_os("REX_M26_MINIMUM").is_some() {
            (640., 480.)
        } else {
            (1024., 700.)
        };
        let mut rt = if on {
            Runtime::with_text(ws, view, size, Env::new(true, 2.), app_fonts())
        } else {
            Runtime::new(ws, view, size, Env::new(true, 2.))
        };
        rt.on_key = Some(crate::app::input::on_key);
        settle(&mut rt);
        let grid = rect(&rt, "grid:block:a");
        let cell = rt.state.metrics().cell;
        let at = |col: f32, row: f32| (grid[0] + col * cell.0, grid[1] + (row + 0.5) * cell.1);
        let (from, to) = (at(2.1, 1.), at(9.8, 1.));
        rt.pointer_down(from.0, from.1);
        rt.pointer_move(to.0 - 5., to.1);
        rt.pointer_move(to.0, to.1);
        rt.pointer_up(to.0, to.1);
        settle(&mut rt);
        let key = rt.state.term_key("block:a");
        let term = &rt.state.terminals[&key];
        assert!(term.has_selection());
        assert_eq!(
            term.index(from.0, from.1, cell) % term.frame.as_ref().unwrap().cols as usize,
            2
        );
        let cursor = rect(&rt, "gblock:a/cursor");
        assert_eq!(cursor[3], cell.1);
        assert!(cursor[2] < cell.0 / 2., "forced bar in the adjusted cell");
        if let Some(mut camera) = Camera::new() {
            camera.snap(&mut rt, name);
        }
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn a_picker_over_settings_stays_under_the_header_and_opens_below_where_six_rows_fit() {
    // A window where the fallback list's Add… button has little room under it.
    let (mut ws, _host) = tabbed_workspace("single");
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    ws.open_advanced_fonts(&mut rune::Tx::default());
    let mut rt = Runtime::new(ws, view, (1024., 700.), Env::new(true, 2.));
    rt.on_key = Some(crate::app::input::on_key);
    rt.poll = Some(crate::app::poll);
    settle(&mut rt);
    tap(&mut rt, "fallback-add");
    let (list, header, add) = (
        element(&rt, "menu-surface").rect,
        element(&rt, "settings-header").rect,
        element(&rt, "fallback-add").rect,
    );
    assert!(
        list[1] >= header[1] + header[3] - 0.5,
        "the picker {list:?} leaves the header {header:?}, its title and Close, in view"
    );
    let close_button = element(&rt, "close-settings").rect;
    assert!(
        list[1] >= close_button[1] + close_button[3]
            || list[0] >= close_button[0] + close_button[2]
            || list[0] + list[2] <= close_button[0],
        "Close {close_button:?} is not under the picker {list:?}"
    );
    assert!(
        list[1] + list[3] <= add[1] || list[1] >= add[1] + add[3],
        "it is against its button, over or under: {list:?} and {add:?}"
    );
    assert!(list[1] + list[3] <= 1400., "and on screen");
    press(&mut rt, Key::Escape);
    settle(&mut rt);
    std::fs::remove_file(&rt.state.settings_path).ok();

    // With room for six rows under the button the picker opens there, as every list does,
    // although there is more room over it.
    let mut rt = settings_on(TERMINAL, |_| {});
    rt.update(|_, tx| tx.scroll_into_view("font-choose"));
    settle(&mut rt);
    tap(&mut rt, "font-choose");
    let (list, button) = (
        element(&rt, "menu-surface").rect,
        element(&rt, "font-choose").rect,
    );
    assert!(
        list[1] >= button[1] + button[3],
        "under its button: {list:?} and {button:?}"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_terminal_font_that_is_not_installed_says_so_on_its_own_row() {
    // The saved family, which this launch asked for, isn't installed: terminals are drawn
    // in the bundled font.
    let mut rt = settings_on(TERMINAL, |_| {});
    assert!(rt.state.terminal_family.is_none());
    let (chip, name, row) = (
        element(&rt, "font-missing"),
        element(&rt, "font-name").rect,
        element(&rt, "font-choose").rect,
    );
    assert_eq!(chip.label.as_deref(), Some("Not installed"));
    let label = rt
        .elements()
        .into_iter()
        .find(|e| !e.exiting && e.text.as_deref() == Some("Terminal font"))
        .expect("the row's label")
        .rect;
    assert!(
        chip.rect[0] >= label[0] + label[2]
            && chip.rect[1] + chip.rect[3] <= name[1] + 1.
            && (chip.rect[1] + chip.rect[3] / 2. - label[1] - label[3] / 2.).abs() < 3.,
        "in the Terminal font row, beside its label: {:?} and {label:?}",
        chip.rect
    );
    // The name has its own line; helper text stays readable beside Choose….
    let helper = rt
        .elements()
        .into_iter()
        .find(|e| !e.exiting && e.text.as_deref() == Some("Applies when you reopen the client."))
        .expect("the row's helper")
        .rect;
    assert!(helper[3] < 20., "one line: {helper:?}");
    // What terminals use instead is right under the card of that row, before the next card.
    let (note, advanced) = (
        element(&rt, "font-drawn").rect,
        element(&rt, "open-advanced-font").rect,
    );
    assert!(
        note[1] > row[1] && note[1] + note[3] <= advanced[1],
        "{note:?} is between the Typography card and Advanced font settings {advanced:?}"
    );
    // A family that is installed, chosen for the next launch, has no warning.
    tap(&mut rt, "font-choose");
    type_text(&mut rt, "hack");
    press(&mut rt, Key::Enter);
    settle(&mut rt);
    assert_eq!(rt.state.settings.font_family, "Hack Nerd Font Mono");
    assert!(!has(&rt, "font-missing"));
    // One fontconfig has no family for, saved in the file, does.
    rt.update(|s, _| s.settings.font_family = "Gone Mono".into());
    settle(&mut rt);
    assert!(has(&rt, "font-missing"));
    std::fs::remove_file(&rt.state.settings_path).ok();
}
