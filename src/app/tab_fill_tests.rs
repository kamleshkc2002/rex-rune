//! M2.8 first slice: local, default-off toolbar sizing, not tab icon policy.
use super::pane_verb_tests::{Camera, framed_terminal, rect};
use super::*;
use crate::app::panel::SettingsSection;
use rex_linux::{
    connection::activity::TerminalProgress,
    download::{State, Transfer},
    settings::interface::ToolbarBehavior,
};

fn settle(rt: &mut Runtime<Workspace>) {
    // Layout feedback, then selected-tab reveal on the next layout (as the app polls).
    for _ in 0..3 {
        rt.update(|s, tx| {
            crate::app::poll(s, 0., tx);
        });
        for _ in 0..60 {
            rt.frame(1. / 60.);
        }
    }
}

fn element(rt: &Runtime<Workspace>, key: &str) -> rune::ElementInfo {
    rt.elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("missing {key}"))
}

fn extra_tabs(ws: &mut Workspace) {
    let view = ws.hosts[0].model.views.get_mut("s").unwrap();
    let template = view.windows[0].clone();
    for i in 0..8 {
        let mut tab = template.clone();
        tab.window_id = Some(format!("extra-{i}"));
        tab.label = Some(format!("a-very-long-project-name-and-terminal-{i}"));
        view.windows.push(tab);
    }
}

fn download(ws: &mut Workspace) {
    let mut transfer = Transfer::new(
        1,
        "s".into(),
        "block:b".into(),
        "~/archive.tar.gz".into(),
        std::env::temp_dir(),
        1,
    );
    transfer.name = Some("archive.tar.gz".into());
    transfer.state = State::Receiving {
        received: 42,
        hint: 100,
    };
    ws.hosts[0].model.downloads.push(transfer);
}

fn fill(rt: &mut Runtime<Workspace>, on: bool) {
    rt.update(|s, _| {
        s.settings.interface.tabs_fill_available_space = on;
        s.reveal_tab_again = true;
    });
    settle(rt);
}

fn chrome(rt: &Runtime<Workspace>) -> Vec<[f32; 4]> {
    ["identity", "commands", "new-tab", "settings", "close-app"]
        .map(|key| rect(rt, key))
        .to_vec()
}

#[test]
fn tab_fill_uses_only_spare_width_and_off_restores_baseline() {
    let _families = rune::text::with_families("Inter", "Inter");
    for size in [(1024., 700.), (640., 480.)] {
        for downloading in [false, true] {
            let (mut ws, host) = tabbed_workspace("split");
            let (sender, sent) = flume::unbounded();
            framed_terminal(&mut ws, "block:b", &sender, b"unchanged");
            framed_terminal(&mut ws, "block:c", &sender, b"unchanged");
            // Both an owned pane and a peer-owned pane keep exactly the same grid.
            ws.terminals[&ws.term_key("block:b")]
                .connection
                .model
                .lock()
                .unwrap()
                .owner = true;
            if downloading {
                download(&mut ws);
            }
            let mut rt = runtime_at(ws, size);
            settle(&mut rt);
            assert_eq!(rect(&rt, "tab:single")[2], 96.);
            assert_eq!(rect(&rt, "tab:split")[2], 96.);
            let baseline = [
                rect(&rt, "tab:single"),
                rect(&rt, "tab:split"),
                rect(&rt, "tab-track"),
                rect(&rt, "title-drag"),
            ];
            let controls = chrome(&rt);
            let panes = [
                rect(&rt, "pane:block:b"),
                rect(&rt, "pane:block:c"),
                rect(&rt, "grid:block:b"),
            ];
            let area = rt.state.area;
            let chip = downloading.then(|| rect(&rt, "download-chip"));
            host.try_iter().for_each(drop);
            sent.try_iter().for_each(drop);
            fill(&mut rt, true);
            assert_eq!(chrome(&rt), controls);
            assert_eq!(rect(&rt, "title-drag")[2], 48.);
            assert_eq!(rt.state.area, area);
            assert_eq!(
                [
                    rect(&rt, "pane:block:b"),
                    rect(&rt, "pane:block:c"),
                    rect(&rt, "grid:block:b")
                ],
                panes
            );
            if let Some(before) = chip {
                let after = rect(&rt, "download-chip");
                assert_eq!(&after[1..], &before[1..]);
                assert_eq!(after[0] + after[2] + 8., rect(&rt, "title-drag")[0]);
                assert_eq!(
                    element(&rt, "download-chip").rect,
                    element(&rt, "download-chip").visible
                );
            }
            let strip = rect(&rt, "tabs")[2];
            let expected = ((strip - 2.) / 2.).max(96.);
            for key in ["tab:single", "tab:split"] {
                // Layout rounds edges to device pixels, not the floating-point share.
                assert!(
                    (rect(&rt, key)[2] - expected).abs() <= 1.,
                    "{size:?} {key}: {:?}, expected {expected}",
                    rect(&rt, key)
                );
            }
            assert_eq!(
                rect(&rt, "close-app")[0] + rect(&rt, "close-app")[2],
                size.0
            );
            assert!(host.is_empty() && sent.is_empty() && rt.state.claimed.is_empty());
            fill(&mut rt, false);
            assert_eq!(
                [
                    rect(&rt, "tab:single"),
                    rect(&rt, "tab:split"),
                    rect(&rt, "tab-track"),
                    rect(&rt, "title-drag")
                ],
                baseline
            );
            std::fs::remove_file(&rt.state.settings_path).ok();
        }
    }
}

#[test]
fn tab_fill_handles_one_tab_and_no_session_without_moving_window_controls() {
    let _families = rune::text::with_families("Inter", "Inter");
    for size in [(1024., 700.), (640., 480.)] {
        let (mut ws, host) = tabbed_workspace("single");
        ws.hosts[0]
            .model
            .views
            .get_mut("s")
            .unwrap()
            .windows
            .truncate(1);
        let mut rt = runtime_at(ws, size);
        settle(&mut rt);
        let controls = chrome(&rt);
        fill(&mut rt, true);
        assert_eq!(rect(&rt, "tab:single")[2], rect(&rt, "tabs")[2]);
        assert_eq!(rect(&rt, "title-drag")[2], 48.);
        assert_eq!(chrome(&rt), controls);
        assert!(!has(&rt, "next-tabs") && !has(&rt, "previous-tabs"));
        rt.update(|s, _| s.hosts[0].model.selected = None);
        settle(&mut rt);
        assert!(!has(&rt, "tab-track"));
        assert_eq!(chrome(&rt), controls);
        let empty_drag = rect(&rt, "title-drag");
        fill(&mut rt, false);
        assert_eq!(rect(&rt, "title-drag"), empty_drag);
        assert!(host.is_empty());
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn tab_fill_overflow_scroll_reveal_click_and_context_menu() {
    let _families = rune::text::with_families("Inter", "Inter");
    for size in [(1024., 700.), (640., 480.)] {
        let (mut ws, host) = tabbed_workspace("single");
        extra_tabs(&mut ws);
        download(&mut ws);
        ws.settings.interface.tabs_fill_available_space = true;
        let (sender, sent) = flume::unbounded();
        framed_terminal(&mut ws, "block:a", &sender, b"no keys");
        let mut rt = runtime_at(ws, size);
        settle(&mut rt);
        assert!(has(&rt, "previous-tabs") && has(&rt, "next-tabs"));
        assert_eq!(rect(&rt, "title-drag")[2], 48.);
        assert_eq!(rect(&rt, "tab:extra-7")[2], 96.);
        let start = rt.state.tab_pos.get()[0];
        click(&mut rt, "next-tabs");
        settle(&mut rt);
        assert!(rt.state.tab_pos.get()[0] > start);
        click(&mut rt, "previous-tabs");
        settle(&mut rt);
        assert_eq!(rt.state.tab_pos.get()[0], start);
        rt.update(|s, _| {
            s.hosts[0]
                .model
                .views
                .get_mut("s")
                .unwrap()
                .active_window_id = Some("extra-7".into())
        });
        settle(&mut rt);
        assert!(rt.state.tab_pos.get()[0] > start);
        let last = element(&rt, "tab:extra-7");
        let strip = rect(&rt, "tabs");
        // At 640 pt with Downloads and a long identity, a whole 96 pt pill cannot
        // fit. Reveal its trailing edge and all the viewport, keeping both arrows.
        assert_eq!(last.visible[2], strip[2].min(last.rect[2]));
        assert_eq!(last.rect[0] + last.rect[2], strip[0] + strip[2]);
        host.try_iter().for_each(drop);
        sent.try_iter().for_each(drop);
        click(&mut rt, "tab:extra-7");
        secondary_click(&mut rt, "tab:extra-7");
        assert!(matches!(
            rt.state.panel.as_ref().unwrap().kind,
            PanelKind::Window
        ));
        assert_eq!(
            rt.state.panel.as_ref().unwrap().title,
            "a-very-long-project-name-and-terminal-7"
        );
        assert!(
            rt.state
                .entries()
                .iter()
                .any(|e| e.label == "Move left" && e.enabled)
        );
        assert!(
            rt.state
                .entries()
                .iter()
                .any(|e| e.label == "Mute tab" && e.enabled)
        );
        let mute = rt
            .state
            .entries()
            .iter()
            .position(|e| e.label == "Mute tab")
            .unwrap();
        click(&mut rt, &format!("entry:{mute}"));
        assert!(rt.state.tab_muted(0, "extra-7"));
        assert!(rt.state.panel.is_none());
        assert!(sent.is_empty() && rt.state.claimed.is_empty());
        let commands: Vec<_> = host.try_iter().collect();
        assert!(
            matches!(&commands[..], [HostCommand::Visibility(true), HostCommand::Navigate { window: Some(window), .. }] if window == "extra-7"),
            "only visibility and the explicit navigation request"
        );
        settle(&mut rt);
        let [x, y, w, h] = element(&rt, "tab:extra-7").visible;
        rt.secondary_down(x + w / 2., y + h / 2.);
        rt.secondary_up(x + w / 2., y + h / 2.);
        rt.frame(1. / 60.);
        let left = rt
            .state
            .entries()
            .iter()
            .position(|e| e.label == "Move left")
            .unwrap();
        click(&mut rt, &format!("entry:{left}"));
        let commands: Vec<_> = host.try_iter().collect();
        assert!(
            matches!(&commands[..], [HostCommand::Request { method, payload }]
            if method == "session.move_window" && payload["window_id"] == "extra-7"
                && payload["before_window_id"] == "extra-6")
        );
        assert!(sent.is_empty() && rt.state.claimed.is_empty());
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn tab_fill_keeps_labels_status_progress_and_muted_marks() {
    let _families = rune::text::with_families("Inter", "Inter");
    for size in [(1024., 700.), (640., 480.)] {
        let (mut ws, _host) = tabbed_workspace("single");
        ws.settings.interface.tabs_fill_available_space = true;
        ws.hosts[0].model.views.get_mut("s").unwrap().windows[0].label = None;
        ws.hosts[0].model.titles.insert(
            "block:a".into(),
            (
                "generated terminal title longer than the available tab width".into(),
                None,
            ),
        );
        ws.hosts[0]
            .model
            .progress
            .insert("block:c".into(), TerminalProgress::Paused(Some(42)));
        let mut rt = runtime_at(ws, size);
        settle(&mut rt);
        assert_eq!(
            element(&rt, "tab:single").label.as_deref(),
            Some("generated terminal title longer than the available tab width")
        );
        assert_eq!(
            element(&rt, "tab-word:split").text.as_deref(),
            Some("Paused 42%")
        );
        assert!(has(&rt, "tab-progress:split"));
        let word = rect(&rt, "tab-word:split");
        let pill = rect(&rt, "tab:split");
        assert!(word[0] >= pill[0] && word[0] + word[2] <= pill[0] + pill[2]);
        rt.update(|s, _| {
            s.settings.generated_titles = true;
            let model = &mut s.hosts[0].model;
            model.titles.remove("block:a");
            model.programs.insert("block:a".into(), "vim".into());
            model.places.insert(
                "block:a".into(),
                rex_linux::connection::titles::Place {
                    cwd: Some("/home/u/rex".into()),
                    home: Some("/home/u".into()),
                    shell: Some(1),
                },
            );
        });
        settle(&mut rt);
        assert_eq!(
            element(&rt, "tab:single").label.as_deref(),
            Some("vim - ~/rex")
        );
        rt.update(|s, _| {
            let target = s.tab_target(0, "s", "single").unwrap();
            s.set_tab_muted(&target, true);
        });
        settle(&mut rt);
        assert!(has(&rt, "tab-muted:single"));
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn tab_fill_respects_toolbar_modes_and_sidebar_placement() {
    let _families = rune::text::with_families("Inter", "Inter");
    for mode in [
        ToolbarBehavior::Always,
        ToolbarBehavior::OnHover,
        ToolbarBehavior::Hidden,
    ] {
        for size in [(1024., 700.), (640., 480.)] {
            for sidebar in [false, true] {
                let (mut ws, host) = tabbed_workspace("single");
                ws.settings.interface.toolbar = mode;
                ws.settings.vertical_tabs = sidebar;
                if sidebar {
                    ws.sidebar.toggle();
                }
                let mut rt = runtime_at(ws, size);
                settle(&mut rt);
                let pane = rect(&rt, "pane:block:a");
                let sidebar_tab = sidebar.then(|| rect(&rt, "sidebar-tab:0:s:single"));
                fill(&mut rt, true);
                assert_eq!(has(&rt, "toolbar"), mode == ToolbarBehavior::Always);
                assert_eq!(rect(&rt, "pane:block:a"), pane);
                assert_eq!(
                    sidebar.then(|| rect(&rt, "sidebar-tab:0:s:single")),
                    sidebar_tab
                );
                if mode != ToolbarBehavior::Always {
                    rt.update(|s, _| s.toggle_toolbar());
                    settle(&mut rt);
                }
                assert!(has(&rt, "toolbar"));
                assert_eq!(has(&rt, "tab-track"), !sidebar);
                assert_eq!(rect(&rt, "pane:block:a"), pane);
                assert!(host.is_empty() && rt.state.claimed.is_empty());
                if sidebar {
                    rt.update(|s, tx| s.perform(Intent::Sidebar, tx));
                    settle(&mut rt);
                    assert!(has(&rt, "tab-track"));
                    assert_eq!(rect(&rt, "title-drag")[2], 48.);
                    assert!(rt.state.settings.interface.tabs_fill_available_space);
                }
                std::fs::remove_file(&rt.state.settings_path).ok();
            }
        }
    }
}

#[test]
fn tab_fill_switch_is_searchable_saved_and_local_at_both_sizes() {
    let _families = rune::text::with_families("Inter", "Inter");
    for size in [(1024., 700.), (640., 480.)] {
        let (mut ws, host) = tabbed_workspace("split");
        ws.perform(Intent::Settings, &mut rune::Tx::default());
        let mut rt = runtime_at(ws, size);
        rt.update(|s, _| {
            s.settings_nav.search.text = "Tabs fill available space".into();
            s.settings_search_changed();
        });
        let hit = &rt.state.settings_nav.hits()[0];
        assert_eq!(hit.label, "Tabs fill available space");
        assert_eq!(hit.page, SettingsSection::Interface);
        rt.update(|s, tx| {
            s.settings_nav.search.text.clear();
            s.settings_search_changed();
            tx.scroll_center("tabs-fill");
        });
        settle(&mut rt);
        assert_eq!(
            element(&rt, "tabs-fill").rect,
            element(&rt, "tabs-fill").visible
        );
        assert!(
            rt.elements()
                .iter()
                .any(|e| e.text.as_deref() == Some("Only toolbar tabs share the available width."))
        );
        for on in [true, false, true] {
            click(&mut rt, "tabs-fill");
            settle(&mut rt);
            assert_eq!(
                Settings::load(&rt.state.settings_path)
                    .unwrap()
                    .interface
                    .tabs_fill_available_space,
                on
            );
        }
        assert!(
            relaunch(&rt.state)
                .settings
                .interface
                .tabs_fill_available_space
        );
        assert!(host.is_empty() && rt.state.claimed.is_empty());
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

/// Run individually with REX_TEST_SNAP_DIR; real fonts must not leak into parallel tests.
#[test]
fn tab_fill_draw() {
    let Some(mut camera) = Camera::new() else {
        return;
    };
    let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
    let _families = rune::text::with_families("Inter", "IBM Plex Mono");
    let (mut ws, _host) = tabbed_workspace("split");
    let (sender, _sent) = flume::unbounded();
    framed_terminal(
        &mut ws,
        "block:b",
        &sender,
        b"$ cargo test\r\nAll tests passed.",
    );
    framed_terminal(
        &mut ws,
        "block:c",
        &sender,
        b"$ tail -f server.log\r\nListening on private socket",
    );
    let size = if std::env::var_os("REX_M28_MINIMUM").is_some() {
        (640., 480.)
    } else {
        (1024., 700.)
    };
    let mut rt = Runtime::with_text(
        ws,
        view,
        size,
        Env::new(true, 1.),
        rune::text::TextSystem::new(fonts.files, fonts.families),
    );
    rt.on_key = Some(crate::app::input::on_key);
    settle(&mut rt);
    camera.snap(&mut rt, "tabs-off");
    fill(&mut rt, true);
    camera.snap(&mut rt, "tabs-on");
    rt.update(|s, _| {
        extra_tabs(s);
        download(s);
        s.hosts[0]
            .model
            .progress
            .insert("block:c".into(), TerminalProgress::Paused(Some(42)));
    });
    settle(&mut rt);
    camera.snap(&mut rt, "tabs-overflow-downloads");
    fill(&mut rt, false);
    camera.snap(&mut rt, "tabs-overflow-downloads-off");
    fill(&mut rt, true);
    rt.update(|s, tx| {
        s.perform(Intent::Settings, tx);
        tx.scroll_center("tabs-fill");
    });
    settle(&mut rt);
    camera.snap(&mut rt, "tabs-settings");
    std::fs::remove_file(&rt.state.settings_path).ok();
}
