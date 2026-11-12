//! Settings navigation: the sidebar and its search, sub-pages, and Back and Forward.
use super::*;
use crate::app::panel::SettingsSection;

fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
}

/// Settings as the toolbar's button opens it, at rest.
fn settings(hosts: usize, size: (f32, f32)) -> Runtime<Workspace> {
    let mut ws = workspace_hosts(hosts);
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    let mut rt = runtime_at(ws, size);
    settle(&mut rt);
    rt
}

fn rect(rt: &Runtime<Workspace>, key: &str) -> [f32; 4] {
    rt.elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("missing {key}"))
        .rect
}

fn type_text(rt: &mut Runtime<Workspace>, words: &str) {
    for c in words.chars() {
        press(rt, Key::Text(c.to_string()));
    }
}

fn alt(rt: &mut Runtime<Workspace>, key: Key) {
    rt.key(KeyEvent {
        key,
        mods: Mods {
            alt: true,
            ..Mods::default()
        },
    });
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
}

fn hits(rt: &Runtime<Workspace>) -> Vec<(String, String)> {
    rt.state
        .settings_nav
        .hits()
        .iter()
        .map(|hit| (hit.label.clone(), hit.place.clone()))
        .collect()
}

fn title(rt: &Runtime<Workspace>) -> String {
    element_text(rt, "settings-title").join("")
}

/// Whether the header's Back or Forward can be pressed.
fn enabled(rt: &Runtime<Workspace>, key: &str) -> bool {
    rt.elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("missing {key}"))
        .focusable
}

#[test]
fn about_project_rows_are_searchable_by_their_section_heading() {
    let mut rt = settings(1, (1024., 700.));
    rt.update(|s, _| {
        s.settings_nav.search.text = "project".into();
        s.settings_search_changed();
    });
    let found = hits(&rt);
    for name in ["Repository", "Release notes", "License", "Issues"] {
        assert!(
            found
                .iter()
                .any(|(label, place)| label == name && place == "About"),
            "{name}: {found:?}"
        );
    }
}

#[test]
fn the_sidebar_lists_the_sections_in_groups_behind_their_tiles() {
    let rt = settings(1, (1440., 1000.));
    let elements = rt.elements();
    let rows: Vec<_> = elements
        .iter()
        .filter(|e| {
            !e.exiting
                && e.key
                    .as_deref()
                    .is_some_and(|k| k.starts_with("settings-section:"))
        })
        .collect();
    let keys: Vec<&str> = rows
        .iter()
        .map(|e| {
            e.key
                .as_deref()
                .unwrap()
                .trim_start_matches("settings-section:")
        })
        .collect();
    assert_eq!(
        keys,
        [
            "server",
            "interface",
            "terminal",
            "shortcuts",
            "integrations",
            "notifications",
            "hosts",
            "updates",
            "about"
        ]
    );
    // Four groups: Server; Interface; terminal/host preferences; app information.
    let gaps: Vec<f32> = rows
        .windows(2)
        .map(|pair| pair[1].rect[1] - (pair[0].rect[1] + pair[0].rect[3]))
        .collect();
    for (index, gap) in gaps.iter().enumerate() {
        let wide = matches!(keys[index + 1], "interface" | "terminal" | "updates");
        assert_eq!(
            *gap > 10.,
            wide,
            "the gap before {}: {gap}",
            keys[index + 1]
        );
    }
    // Every row is a 22 pt tile and a label, on one left edge, inside the sidebar.
    let sidebar = rect(&rt, "settings-sidebar");
    let search = rect(&rt, "settings-search-box");
    for row in &rows {
        assert!(
            (row.rect[0] - search[0]).abs() < 0.5 && (row.rect[2] - search[2]).abs() < 0.5,
            "{:?} lines up with the search field {search:?}",
            row.rect
        );
        assert!(row.rect[0] + row.rect[2] <= sidebar[0] + sidebar[2]);
        assert!((row.rect[3] - 32.).abs() < 0.5);
    }
    let tiles = elements
        .iter()
        .filter(|e| {
            !e.exiting
                && e.path.contains("settings-sections")
                && (e.rect[2] - 22.).abs() < 0.5
                && (e.rect[3] - 22.).abs() < 0.5
        })
        .count();
    assert_eq!(tiles, rows.len(), "a tile a section");
    // The search field's band is the page header's: both are centred on one line.
    let header = rect(&rt, "settings-header");
    assert!(
        ((search[1] + search[3] / 2.) - (header[1] + header[3] / 2.)).abs() < 0.5,
        "{search:?} and {header:?}"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn settings_opens_on_its_section_list_and_a_typed_letter_starts_a_search() {
    let mut rt = settings(1, (1440., 1000.));
    assert!(
        focused(&rt).is_some_and(|path| path.ends_with("settings-section:interface")),
        "focus starts on the section list, not {:?}",
        focused(&rt)
    );
    type_text(&mut rt, "fo");
    assert_eq!(rt.state.settings_nav.search.text, "fo");
    assert!(
        focused(&rt).is_some_and(|path| path.contains("settings-search")),
        "the field has the keyboard: {:?}",
        focused(&rt)
    );
    assert!(hits(&rt).iter().any(|(label, _)| label == "Font size"));
    assert!(
        !has(&rt, "settings-section:interface"),
        "the hits take the list's place"
    );
    // Escape ends the search first, and closes Settings only once there is none.
    press(&mut rt, Key::Escape);
    assert_eq!(rt.state.settings_nav.search.text, "");
    assert!(has(&rt, "settings-section:interface"));
    assert!(matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::Settings)
    ));
    // From the empty field, Down goes back to the section list.
    press(&mut rt, Key::Down);
    assert!(
        focused(&rt).is_some_and(|path| path.ends_with("settings-section:interface")),
        "{:?}",
        focused(&rt)
    );
    press(&mut rt, Key::Escape);
    assert!(rt.state.panel.is_none(), "Escape with no search closes");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn typing_sound_lists_notifications_and_enter_opens_it() {
    let mut rt = settings(1, (1440., 1000.));
    click(&mut rt, "settings-search");
    type_text(&mut rt, "sound");
    assert_eq!(
        hits(&rt),
        [
            ("Play sounds".to_owned(), "Notifications".to_owned()),
            ("Notifications".to_owned(), String::new()),
        ]
    );
    assert!(has(&rt, "settings-hit:0"));
    // The new exact setting hit comes first; still verify opening the page hit.
    press(&mut rt, Key::Down);
    press(&mut rt, Key::Enter);
    settle(&mut rt);
    assert_eq!(rt.state.settings_page(), SettingsSection::Notifications);
    assert_eq!(title(&rt), "Notifications");
    assert!(has(&rt, "notify-tabs"), "its page is on screen");
    assert!(!has(&rt, "settings-found"), "a page has no row to mark");
    // The search stays, with the keyboard still in it.
    assert_eq!(rt.state.settings_nav.search.text, "sound");
    assert!(focused(&rt).is_some_and(|path| path.contains("settings-search")));

    // Nothing found says so, and Enter does nothing.
    press(&mut rt, Key::Escape);
    type_text(&mut rt, "zebra");
    assert!(hits(&rt).is_empty());
    assert!(has(&rt, "settings-no-hits"));
    press(&mut rt, Key::Enter);
    assert_eq!(rt.state.settings_page(), SettingsSection::Notifications);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn typing_padding_lists_the_row_and_enter_marks_it() {
    let mut rt = settings(1, (1440., 1000.));
    click(&mut rt, "settings-search");
    type_text(&mut rt, "padding");
    assert_eq!(
        hits(&rt)[0],
        ("Horizontal padding".to_owned(), "Terminal".to_owned())
    );
    // The arrows walk the hits, around the ends.
    let count = hits(&rt).len();
    assert!(count > 1, "the Terminal page is a hit too");
    press(&mut rt, Key::Up);
    assert_eq!(rt.state.settings_nav.selected(), count - 1);
    press(&mut rt, Key::Down);
    assert_eq!(rt.state.settings_nav.selected(), 0);

    press(&mut rt, Key::Enter);
    settle(&mut rt);
    assert_eq!(rt.state.settings_page(), SettingsSection::Terminal);
    assert_eq!(rt.state.settings_section, SettingsSection::Terminal);
    assert_eq!(title(&rt), "Terminal");
    // The row is marked: the tint and its outline, around the row that holds the stepper.
    let mark = rt
        .elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some("settings-found"))
        .expect("the found row's mark");
    let (fill, _) = crate::app::theme::selection(false);
    assert_eq!(mark.fill, fill);
    let helper = "Space to the left and right of the terminal.";
    assert_eq!(
        rt.elements()
            .iter()
            .find(|e| !e.exiting && e.text.as_deref() == Some(helper))
            .unwrap()
            .text_color,
        Some(crate::app::theme::rgb(crate::app::theme::ink())),
        "the highlighted helper stays legible on the selection tint"
    );
    let stepper = rect(&rt, "pane-padding-larger");
    assert!(
        mark.rect[1] < stepper[1] && mark.rect[1] + mark.rect[3] > stepper[1] + stepper[3],
        "{:?} is around the row of {stepper:?}",
        mark.rect
    );
    // The mark leaves by itself.
    rt.update(|s, tx| {
        let later = std::time::Instant::now() + std::time::Duration::from_secs(5);
        assert!(s.poll_settings(later, tx));
    });
    settle(&mut rt);
    assert!(!has(&rt, "settings-found"));
    assert_eq!(rt.state.settings_page(), SettingsSection::Terminal);

    // A pointer picks a hit as Enter does.
    press(&mut rt, Key::Escape);
    type_text(&mut rt, "font size");
    click(&mut rt, "settings-hit:0");
    settle(&mut rt);
    assert_eq!(rt.state.settings_page(), SettingsSection::Terminal);
    let stepper = rect(&rt, "font-larger");
    let mark = rect(&rt, "settings-found");
    assert!(mark[1] < stepper[1] && mark[1] + mark[3] > stepper[1] + stepper[3]);
    assert!(
        focused(&rt).is_some_and(|path| path.contains("settings-search")),
        "the press leaves the keyboard in the field"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn search_reads_the_rows_the_pages_are_built_from() {
    let mut rt = settings(2, (1440., 1000.));
    click(&mut rt, "settings-search");
    // A row from each kind of page: a section's, another module's, a sub-page's.
    for (query, label, place) in [
        ("font size", "Font size", "Terminal"),
        ("zoom", "Interface zoom", "Interface"),
        ("toolbar", "Toolbar visibility", "Interface"),
        ("download folder", "Download folder", "Interface"),
        ("client label", "Client label", "Remote Hosts"),
        (
            "desktop notifications",
            "Desktop notifications",
            "Notifications",
        ),
        ("pane headers", "Pane headers", "Interface"),
    ] {
        rt.update(|s, _| {
            s.settings_nav.search = rune::TextEdit::new(query);
            s.settings_search_changed();
        });
        assert!(
            hits(&rt).contains(&(label.to_owned(), place.to_owned())),
            "{query} finds {label} on {place}: {:?}",
            hits(&rt)
        );
    }
    // A label a page repeats, one per host, is one hit.
    rt.update(|s, _| {
        s.settings_nav.search = rune::TextEdit::new("remote control");
        s.settings_search_changed();
    });
    let found = hits(&rt);
    assert_eq!(
        found
            .iter()
            .filter(|(label, _)| label == "Remote control")
            .count(),
        1,
        "{found:?}"
    );
    // A helper's words find its row, after the rows named for them.
    rt.update(|s, _| {
        s.settings_nav.search = rune::TextEdit::new("osc");
        s.settings_search_changed();
    });
    assert_eq!(
        hits(&rt),
        [(
            "Messages from programs".to_owned(),
            "Notifications".to_owned()
        )]
    );
    // Reading the pages leaves the one on screen as it was.
    assert_eq!(rt.state.settings_page(), SettingsSection::Interface);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn history_starts_again_each_time_settings_opens() {
    let mut rt = settings(1, (1440., 1000.));
    click(&mut rt, "settings-section:terminal");
    assert_eq!(rt.state.settings_page(), SettingsSection::Terminal);
    assert!(enabled(&rt, "settings-back"));
    // Alt with the arrows is Settings' own: it does nothing once Settings is closed.
    click(&mut rt, "close-settings");
    settle(&mut rt);
    assert!(rt.state.panel.is_none());
    alt(&mut rt, Key::Left);
    assert_eq!(rt.state.settings_page(), SettingsSection::Terminal);

    // It reopens on the page it was left on, with nothing to go back to.
    rt.update(|s, tx| s.perform(Intent::Settings, tx));
    settle(&mut rt);
    assert_eq!(title(&rt), "Terminal");
    assert!(!enabled(&rt, "settings-back") && !enabled(&rt, "settings-forward"));
    // Opening on a section (About, from Commands) is not a visit either.
    rt.update(|s, tx| {
        s.close_panel(tx);
        s.perform(Intent::About, tx);
    });
    settle(&mut rt);
    assert_eq!(rt.state.settings_page(), SettingsSection::About);
    assert!(!enabled(&rt, "settings-back"));
    // Nor is opening on Updates, which also starts without the last search.
    click(&mut rt, "settings-search");
    type_text(&mut rt, "font");
    rt.update(|s, tx| {
        s.close_panel(tx);
        s.perform(Intent::Updates, tx);
    });
    settle(&mut rt);
    assert_eq!(rt.state.settings_page(), SettingsSection::Updates);
    assert_eq!(rt.state.settings_nav.search.text, "");
    assert!(has(&rt, "settings-section:updates") && !enabled(&rt, "settings-back"));
    rt.update(|s, tx| {
        s.close_panel(tx);
        s.perform(Intent::About, tx);
    });
    settle(&mut rt);
    // The first visit from there can come back to it.
    click(&mut rt, "settings-section:terminal");
    assert!(enabled(&rt, "settings-back"));
    click(&mut rt, "settings-back");
    assert_eq!(rt.state.settings_page(), SettingsSection::About);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn back_and_forward_restore_scroll_and_a_new_visit_discards_forward() {
    let mut rt = settings(1, (640., 480.));
    assert!(!enabled(&rt, "settings-back") && !enabled(&rt, "settings-forward"));
    click(&mut rt, "settings-section:notifications");
    settle(&mut rt);
    let page = rect(&rt, "settings-scroll:notifications");
    rt.wheel(page[0] + page[2] / 2., page[1] + page[3] / 2., -110.);
    settle(&mut rt);
    assert!(rt.state.settings_nav.offset().get() > 20.);
    // Keep the pointer-navigation target visible as rows are added above Programs.
    rt.update(|_, tx| tx.scroll_center("notify-messages"));
    settle(&mut rt);
    let scrolled = rt.state.settings_nav.offset().get();
    assert!(scrolled > 20., "{scrolled}");
    let before = rect(&rt, "notify-messages");
    click(&mut rt, "settings-section:terminal");
    settle(&mut rt);
    assert_eq!(rt.state.settings_nav.offset().get(), 0.);
    click(&mut rt, "settings-back");
    settle(&mut rt);
    assert_eq!(title(&rt), "Notifications");
    assert!((rt.state.settings_nav.offset().get() - scrolled).abs() < 0.5);
    assert_eq!(rect(&rt, "notify-messages"), before);
    assert!(enabled(&rt, "settings-back") && enabled(&rt, "settings-forward"));

    alt(&mut rt, Key::Right);
    assert_eq!(title(&rt), "Terminal");
    alt(&mut rt, Key::Left);
    settle(&mut rt);
    assert_eq!(rect(&rt, "notify-messages"), before);

    // Pointer navigation is Settings-local, even over a control.
    let control = rect(&rt, "notify-messages");
    let (x, y) = (control[0] + control[2] / 2., control[1] + control[3] / 2.);
    let notifications = rt.state.settings.notifications.program_messages;
    rt.button_down(x, y, rune::PointerButton::Forward);
    rt.button_up(x, y, rune::PointerButton::Forward);
    settle(&mut rt);
    assert_eq!(title(&rt), "Terminal");
    assert_eq!(
        rt.state.settings.notifications.program_messages,
        notifications
    );
    let header = rect(&rt, "settings-header");
    let (x, y) = (header[0] + header[2] / 2., header[1] + header[3] / 2.);
    for expected in ["Notifications", "Interface", "Interface"] {
        rt.button_down(x, y, rune::PointerButton::Back);
        rt.button_up(x, y, rune::PointerButton::Back);
        settle(&mut rt);
        assert_eq!(title(&rt), expected);
        assert!(rt.state.panel.is_some());
    }
    click(&mut rt, "settings-section:about");
    assert!(!enabled(&rt, "settings-forward"));
    alt(&mut rt, Key::Right);
    assert_eq!(title(&rt), "About");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn search_field_keeps_focus_through_history_and_forms_keep_the_search() {
    let mut rt = settings(1, (1024., 700.));
    let saved = std::fs::read(&rt.state.settings_path).ok();
    click(&mut rt, "settings-search");
    type_text(&mut rt, "download folder");
    press(&mut rt, Key::Enter);
    settle(&mut rt);
    assert!(has(&rt, "settings-found") && has(&rt, "download-folder"));
    click(&mut rt, "download-folder");
    assert!(matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::Form(Form::DownloadFolder))
    ));
    // Settings history must not navigate behind a form.
    alt(&mut rt, Key::Left);
    assert!(matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::Form(Form::DownloadFolder))
    ));
    press(&mut rt, Key::Escape);
    settle(&mut rt);
    assert!(matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::Settings)
    ));
    assert_eq!(rt.state.settings_nav.search.text, "download folder");
    assert!(
        focused(&rt).is_some_and(|p| p.contains("settings-search")),
        "returns to the search which kept focus during the pointer press"
    );
    // Escape clears the search even when a page control, not the field, has focus.
    rt.update(|_, tx| tx.focus("download-folder", true));
    press(&mut rt, Key::Escape);
    assert_eq!(rt.state.settings_nav.search.text, "");
    assert!(rt.state.panel.is_some());
    click(&mut rt, "settings-search");
    type_text(&mut rt, "sound");
    press(&mut rt, Key::Enter);
    alt(&mut rt, Key::Left);
    assert_eq!(rt.state.settings_page(), SettingsSection::Interface);
    alt(&mut rt, Key::Right);
    assert_eq!(rt.state.settings_page(), SettingsSection::Notifications);
    assert!(focused(&rt).is_some_and(|p| p.contains("settings-search")));
    assert_eq!(
        std::fs::read(&rt.state.settings_path).ok(),
        saved,
        "search and history don't write preferences"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn search_results_scroll_with_keyboard_and_never_trap_tab() {
    let mut rt = settings(2, (640., 480.));
    click(&mut rt, "settings-search");
    type_text(&mut rt, "e");
    let count = hits(&rt).len();
    assert!(count > 10, "{count}");
    press(&mut rt, Key::Up);
    settle(&mut rt);
    let last = rect(&rt, &format!("settings-hit:{}", count - 1));
    let scroll = rect(&rt, "settings-hits-scroll");
    assert!(
        last[1] >= scroll[1] && last[1] + last[3] <= scroll[1] + scroll[3] + 0.5,
        "{last:?} in {scroll:?}"
    );
    press(&mut rt, Key::Down);
    assert_eq!(rt.state.settings_nav.selected(), 0);
    press(&mut rt, Key::Tab);
    assert!(!focused(&rt).unwrap().contains("settings-search"));
    assert!(focused(&rt).unwrap().contains("dialog"));
    press(&mut rt, Key::Escape);
    assert!(has(&rt, "settings-sections"));
    press(&mut rt, Key::Escape);
    assert!(rt.state.panel.is_none());
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn settings_search_and_history_preserve_the_shortcuts_filter() {
    let mut rt = settings(1, (1024., 700.));
    click(&mut rt, "settings-search");
    type_text(&mut rt, "shortcuts");
    press(&mut rt, Key::Enter);
    settle(&mut rt);
    assert!(has(&rt, "shortcut-filter"));
    click(&mut rt, "shortcut-filter");
    type_text(&mut rt, "ctrl+shift+d");
    assert_eq!(rt.state.shortcut_filter.text, "ctrl+shift+d");
    assert!(
        rt.elements()
            .iter()
            .any(|e| e.text.as_deref() == Some("Split right"))
    );
    alt(&mut rt, Key::Left);
    assert_eq!(rt.state.settings_page(), SettingsSection::Interface);
    alt(&mut rt, Key::Right);
    assert_eq!(rt.state.settings_page(), SettingsSection::Shortcuts);
    assert_eq!(rt.state.shortcut_filter.text, "ctrl+shift+d");
    click(&mut rt, "shortcut-filter");
    press(&mut rt, Key::Escape);
    assert!(
        rt.state.shortcut_filter.text.is_empty(),
        "its own Escape clears its filter"
    );
    assert!(rt.state.panel.is_some());
    assert_eq!(rt.state.settings_nav.query(), "shortcuts");
    press(&mut rt, Key::Escape);
    assert!(rt.state.settings_nav.query().is_empty());
    assert!(rt.state.panel.is_some());
    press(&mut rt, Key::Escape);
    assert!(rt.state.panel.is_none());
    std::fs::remove_file(&rt.state.settings_path).ok();
}

/// Run alone with REX_TEST_SNAP_DIR: real fonts are process-global.
#[test]
fn settings_navigation_draws_at_normal_and_minimum_size() {
    let Some(dir) = std::env::var_os("REX_TEST_SNAP_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let gpu = rune::render::Gpu::headless();
    let (loaded, ready) = std::sync::mpsc::channel();
    let images = rune::ImageCache::new(std::sync::Arc::new(move || {
        let _ = loaded.send(());
    }));
    let logo = images.load(crate::app::APP_ICON);
    ready
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    for (name, scale, size) in [
        ("normal", 1., (1024., 700.)),
        ("minimum", 1., (640., 480.)),
        ("zoomed", 1.3, (1024., 700.)),
    ] {
        let mut ws = workspace_hosts(2);
        ws.settings.ui_scale = scale;
        ws.perform(Intent::Settings, &mut rune::Tx::default());
        let mut rt = Runtime::new(ws, view, size, Env::new(true, 2.));
        rt.state.logo = Some(logo.clone());
        rt.on_key = Some(crate::app::input::on_key);
        // Each runtime owns a different atlas and set of resident render caches.
        let mut painter = rune::render::Renderer::new(&gpu.device, rune::render::OFFSCREEN_FORMAT);
        settle(&mut rt);
        for capture in [
            "interface",
            "notifications",
            "search",
            "found",
            "empty",
            "about",
            "shortcuts",
        ] {
            match capture {
                "notifications" => click(&mut rt, "settings-section:notifications"),
                "search" => {
                    click(&mut rt, "settings-search");
                    type_text(&mut rt, "font");
                }
                "found" => {
                    press(&mut rt, Key::Escape);
                    type_text(&mut rt, "padding");
                    press(&mut rt, Key::Enter);
                }
                "empty" => {
                    press(&mut rt, Key::Escape);
                    type_text(&mut rt, "zebra");
                }
                "about" => {
                    press(&mut rt, Key::Escape);
                    click(&mut rt, "settings-section:about");
                }
                "shortcuts" => {
                    click(&mut rt, "settings-section:shortcuts");
                    click(&mut rt, "shortcut-filter");
                    type_text(&mut rt, "ctrl+shift+d");
                }
                _ => {}
            }
            settle(&mut rt);
            let dialog = rect(&rt, "dialog");
            for e in rt.elements().iter().filter(|e| {
                !e.exiting
                    && (e.path.contains("settings-header") || e.path.contains("settings-sidebar"))
                    && e.rect[2] > 0.
            }) {
                assert!(
                    e.rect[0] >= dialog[0] - 0.5
                        && e.rect[0] + e.rect[2] <= dialog[0] + dialog[2] + 0.5,
                    "{name}/{capture}: {:?} {:?} outside {dialog:?}",
                    e.key,
                    e.rect
                );
            }
            let close = rect(&rt, "close-settings");
            let title = rect(&rt, "settings-title");
            assert!(title[0] + title[2] <= close[0]);
            let (w, h) = ((size.0 * 2.) as u32, (size.1 * 2.) as u32);
            painter.upload_images(&gpu.device, &gpu.queue, &mut rt.images);
            painter.prepare(
                &gpu.device,
                &gpu.queue,
                &mut rt.text.atlas,
                &rt.instances,
                (w, h),
            );
            let target = rune::render::Offscreen::new(&gpu.device, w, h);
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            painter.draw(&mut encoder, &target.view, rt.clear_color());
            target.copy(&mut encoder);
            gpu.queue.submit([encoder.finish()]);
            rune::render::write_png(
                &dir.join(format!("m23-{name}-{capture}.png")),
                w,
                h,
                &target.read(&gpu.device),
            )
            .unwrap();
        }
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}
