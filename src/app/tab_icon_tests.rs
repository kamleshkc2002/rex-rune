//! M2.8 cached monochrome icons; preferences never touch terminals or the server.
use super::pane_verb_tests::{Camera, framed_terminal, rect};
use super::*;
use crate::app::{
    activity::{tab_activity, tab_programs},
    panel::SettingsSection,
    tab_icons::*,
};
use rex_linux::connection::activity::{ProgramState, StatusRecord};
use rex_linux::settings::interface::ToolbarBehavior;

fn settle(rt: &mut Runtime<Workspace>) {
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
fn names(programs: &[Option<String>], count: u8) -> Vec<&'static str> {
    icons_of(programs, count)
        .into_iter()
        .map(|i| i.icon.name)
        .collect()
}
#[test]
fn resolved_glyph_dedup_cap_unknown_and_empty() {
    let programs = [
        "nvim", "vim", "bash", "unknown", "", "claude", "codex", "git", "cargo", "ssh",
    ]
    .map(|p| Some(p.to_owned()));
    assert_eq!(
        names(&programs, 5),
        [
            "rex-vim",
            "rex-terminal",
            "rex-claude",
            "rex-codex",
            "rex-git"
        ]
    );
    for count in 0..=5 {
        assert_eq!(icons_of(&programs, count).len(), usize::from(count));
        assert_eq!(leading_width(usize::from(count)), f32::from(count) * 23.);
    }
    assert_eq!(icons_of(&programs, 255).len(), 5);
    assert_eq!(names(&[], 1), ["rex-terminal"]);
    assert!(icons_of(&[], 0).is_empty());
    assert_eq!(names(&[None, Some("mystery".into())], 5), ["rex-terminal"]);
}
#[test]
fn focused_first_server_order_and_exact_legacy_status_source() {
    let (mut ws, _host) = tabbed_workspace("split");
    let model = &mut ws.hosts[0].model;
    model.programs.insert("block:b".into(), "nvim".into());
    model.programs.insert("block:c".into(), "cargo".into());
    let tab = model.views["s"].windows[1].clone();
    assert_eq!(
        tab_programs(model, &tab),
        [Some("nvim".into()), Some("cargo".into())]
    );
    model.activity.insert(
        "block:c".into(),
        [(
            "c".into(),
            StatusRecord {
                state: ProgramState::Blocked,
                app: Some("claude".into()),
                ..Default::default()
            },
        )]
        .into(),
    );
    model.activity.insert(
        "block:b".into(),
        [
            (
                "working".into(),
                StatusRecord {
                    state: ProgramState::Working,
                    app: Some("git".into()),
                    ..Default::default()
                },
            ),
            (
                "error".into(),
                StatusRecord {
                    state: ProgramState::Error,
                    app: Some("codex".into()),
                    ..Default::default()
                },
            ),
        ]
        .into(),
    );
    assert_eq!(tab_activity(model, &tab).program.as_deref(), Some("codex"));
    assert_eq!(
        names(&tab_programs(model, &tab), 5),
        ["rex-codex", "rex-claude"]
    );
    assert_eq!(names(&tab_programs(model, &tab), 1), ["rex-codex"]);
    // Legacy: no app on the deciding record uses foreground, not a lower record's app.
    model
        .activity
        .get_mut("block:b")
        .unwrap()
        .get_mut("error")
        .unwrap()
        .app = None;
    assert_eq!(tab_activity(model, &tab).program.as_deref(), Some("nvim"));
    assert_eq!(names(&tab_programs(model, &tab), 1), ["rex-vim"]);
    let mut tab = tab;
    tab.focused_block_id = Some("block:c".into());
    assert_eq!(
        names(&tab_programs(model, &tab), 5),
        ["rex-claude", "rex-vim"]
    );
    for focus in [None, Some("missing".into())] {
        tab.focused_block_id = focus;
        assert_eq!(
            tab_programs(model, &tab)[0],
            tab_activity(model, &tab).program
        );
        assert_eq!(
            names(&tab_programs(model, &tab), 5),
            ["rex-vim", "rex-claude"]
        );
    }
    tab.layers.clear();
    assert_eq!(names(&tab_programs(model, &tab), 5), ["rex-terminal"]);
    std::fs::remove_file(&ws.settings_path).ok();
}
/// Five cached pane records on the inactive tab; no client-side layout mutation.
fn five_programs(ws: &mut Workspace) {
    let model = &mut ws.hosts[0].model;
    let tab = &mut model.views.get_mut("s").unwrap().windows[1];
    tab.layers[0].blocks = ["block:b", "block:c", "block:d", "block:e", "block:f"]
        .map(|id| rexrs::api::session::LayerBlock {
            block_id: Some(id.into()),
            ..Default::default()
        })
        .to_vec();
    for (block, program) in ["block:b", "block:c", "block:d", "block:e", "block:f"]
        .into_iter()
        .zip(["zsh", "claude", "nvim", "git", "cargo"])
    {
        model.programs.insert(block.into(), program.into());
    }
}
fn grid(rt: &Runtime<Workspace>) -> Vec<rune::ElementInfo> {
    let root = element(rt, "grid:block:a");
    rt.elements()
        .into_iter()
        .filter(|e| {
            !e.exiting && (e.path == root.path || e.path.starts_with(&format!("{}/", root.path)))
        })
        .collect()
}
fn same_grid(before: &[rune::ElementInfo], after: &[rune::ElementInfo]) {
    assert_eq!(before.len(), after.len());
    for (a, b) in before.iter().zip(after) {
        assert_eq!(
            (&a.key, &a.text, a.rect, a.fill, a.text_color, a.opacity),
            (&b.key, &b.text, b.rect, b.fill, b.text_color, b.opacity)
        );
    }
}
#[test]
fn count_matrix_keeps_grids_commands_labels_and_removes_the_whole_gap() {
    let _families = rune::text::with_families("Inter", "Inter");
    for size in [(1024., 700.), (640., 480.)] {
        for fill in [false, true] {
            for sidebar in [false, true] {
                let (mut ws, host) = tabbed_workspace("single");
                five_programs(&mut ws);
                ws.settings.interface.tabs_fill_available_space = fill;
                ws.settings.vertical_tabs = sidebar;
                if sidebar {
                    ws.sidebar.toggle();
                }
                // Match the real sidebar preference path before measuring count-only
                // effects. Otherwise a delayed window-size save starts its prefetch.
                ws.sync_prefetch();
                let (sender, sent) = flume::unbounded();
                framed_terminal(&mut ws, "block:a", &sender, b"\x1b[31mred\x1b[0m unchanged");
                ws.terminals[&ws.term_key("block:a")]
                    .connection
                    .model
                    .lock()
                    .unwrap()
                    .owner = fill;
                let mut rt = runtime_at(ws, size);
                settle(&mut rt);
                rt.update(|s, _| {
                    for t in s.terminals.values_mut() {
                        t.draw_anyway(
                            std::time::Instant::now() + std::time::Duration::from_secs(10),
                        );
                    }
                });
                settle(&mut rt);
                let before = grid(&rt);
                assert!(before.iter().any(|e| e.text.is_some()));
                let area = rt.state.area;
                let pane = rect(&rt, "pane:block:a");
                let prefix = if sidebar {
                    "sidebar-tab:0:s:split"
                } else {
                    "tab:split"
                };
                let name = if sidebar {
                    "sidebar-tab-name:0:s:split"
                } else {
                    "tab-name:split"
                };
                let icon = if sidebar {
                    "sidebar-tab-icon:0:s:split"
                } else {
                    "tab-icon:split"
                };
                let baseline_label = element(&rt, prefix).label;
                host.drain();
                sent.drain();
                for count in [0, 1, 5, 1] {
                    rt.update(|s, _| {
                        s.settings.interface.tab_icon_count = count;
                        s.reveal_tab_again = true;
                    });
                    settle(&mut rt);
                    assert_eq!(element(&rt, prefix).label, baseline_label);
                    assert_eq!(has(&rt, icon), count != 0);
                    for i in 1..5 {
                        assert_eq!(has(&rt, &format!("{icon}:{i}")), i < usize::from(count));
                    }
                    let pill = rect(&rt, prefix);
                    let text = rect(&rt, name);
                    if count > 0 {
                        let last = if count == 1 {
                            icon.to_owned()
                        } else {
                            format!("{icon}:{}", count - 1)
                        };
                        let last = rect(&rt, &last);
                        assert_eq!(text[0] - last[0] - last[2], 8.);
                        assert_eq!(last[2], 15.);
                    } else if !fill || sidebar {
                        assert_eq!(text[0] - pill[0], if sidebar { 10. } else { 14. });
                    }
                    assert_eq!(rect(&rt, "pane:block:a"), pane);
                    assert_eq!(rt.state.area, area);
                    same_grid(&before, &grid(&rt));
                    assert!(
                        host.is_empty() && sent.is_empty() && rt.state.claimed.is_empty(),
                        "{size:?} fill={fill} sidebar={sidebar} count={count}: host={:?} terminal={:?} claimed={:?}",
                        host.try_iter()
                            .map(|c| match c {
                                HostCommand::Request { method, .. } => method,
                                c => format!("{:?}", std::mem::discriminant(&c)),
                            })
                            .collect::<Vec<_>>(),
                        sent.len(),
                        rt.state.claimed
                    );
                    if !sidebar {
                        assert!(rect(&rt, "title-drag")[2] >= 48.);
                    }
                }
                // Reveal the target first: at minimum width the inactive pill can be
                // outside the viewport, and an offscreen synthetic press is not a menu test.
                rt.update(|s, _| {
                    s.hosts[0]
                        .model
                        .views
                        .get_mut("s")
                        .unwrap()
                        .active_window_id = Some("split".into());
                    s.reveal_tab_again = true;
                });
                settle(&mut rt);
                secondary_click(&mut rt, prefix);
                assert!(
                    rt.state.tab_targeted(0, "split"),
                    "{size:?} fill={fill} sidebar={sidebar}"
                );
                std::fs::remove_file(&rt.state.settings_path).ok();
            }
        }
    }
}
#[test]
fn actual_label_search_stepper_preview_save_relaunch_are_local() {
    let _families = rune::text::with_families("Inter", "Inter");
    for size in [(1024., 700.), (640., 480.)] {
        let (mut ws, host) = tabbed_workspace("single");
        let (sender, sent) = flume::unbounded();
        framed_terminal(&mut ws, "block:a", &sender, b"no input");
        ws.perform(Intent::Settings, &mut rune::Tx::default());
        let mut rt = runtime_at(ws, size);
        rt.update(|s, _| {
            s.settings_nav.search.text = "Show tab icons".into();
            s.settings_search_changed();
        });
        let hit = &rt.state.settings_nav.hits()[0];
        assert_eq!(hit.label, "Show tab icons");
        assert_eq!(hit.page, SettingsSection::Interface);
        rt.update(|s, tx| {
            s.settings_nav.search.text.clear();
            s.settings_search_changed();
            tx.scroll_center("tab-icons-larger");
        });
        settle(&mut rt);
        host.drain();
        sent.drain();
        for (key, expected) in [
            ("smaller", 0),
            ("smaller", 0),
            ("larger", 1),
            ("larger", 2),
            ("larger", 3),
            ("larger", 4),
            ("larger", 5),
            ("larger", 5),
        ] {
            let key = format!("tab-icons-{key}");
            assert_eq!(element(&rt, &key).rect, element(&rt, &key).visible);
            click(&mut rt, &key);
            settle(&mut rt);
            assert_eq!(rt.state.settings.interface.tab_icon_count, expected);
            assert_eq!(
                Settings::load(&rt.state.settings_path)
                    .unwrap()
                    .interface
                    .tab_icon_count,
                expected
            );
            assert_eq!(has(&rt, "tab-icons-preview-icon"), expected > 0);
            for i in 1..5 {
                assert_eq!(
                    has(&rt, &format!("tab-icons-preview-icon:{i}")),
                    i < expected
                );
            }
        }
        assert_eq!(relaunch(&rt.state).settings.interface.tab_icon_count, 5);
        click(&mut rt, "tab-icons-preview");
        assert!(host.is_empty() && sent.is_empty() && rt.state.claimed.is_empty());
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}
#[test]
fn icon_counts_preserve_visibility_modes_and_sidebar_fallback() {
    let _families = rune::text::with_families("Inter", "Inter");
    for mode in [
        ToolbarBehavior::Always,
        ToolbarBehavior::OnHover,
        ToolbarBehavior::Hidden,
    ] {
        for count in [0, 1, 5] {
            let (mut ws, host) = tabbed_workspace("single");
            five_programs(&mut ws);
            ws.settings.interface.toolbar = mode;
            ws.settings.interface.tab_icon_count = count;
            ws.settings.vertical_tabs = true;
            ws.sidebar.toggle();
            let mut rt = runtime_at(ws, (640., 480.));
            settle(&mut rt);
            assert_eq!(has(&rt, "toolbar"), mode == ToolbarBehavior::Always);
            assert_eq!(has(&rt, "sidebar-tab-icon:0:s:split"), count > 0);
            rt.update(|s, tx| {
                s.perform(Intent::Sidebar, tx);
                if mode != ToolbarBehavior::Always {
                    s.toggle_toolbar();
                }
            });
            settle(&mut rt);
            assert!(has(&rt, "tab-track"));
            assert_eq!(has(&rt, "tab-icon:split"), count > 0);
            assert!(host.is_empty());
            std::fs::remove_file(&rt.state.settings_path).ok();
        }
    }
}
#[test]
fn counts_keep_overflow_downloads_status_and_menu_navigation() {
    use rex_linux::{
        connection::activity::TerminalProgress,
        download::{State, Transfer},
    };
    let _families = rune::text::with_families("Inter", "Inter");
    for size in [(1024., 700.), (640., 480.)] {
        for fill in [false, true] {
            for count in [0, 1, 5] {
                let (mut ws, host) = tabbed_workspace("single");
                five_programs(&mut ws);
                ws.settings.interface.tab_icon_count = count;
                ws.settings.interface.tabs_fill_available_space = fill;
                let model = &mut ws.hosts[0].model;
                let view = model.views.get_mut("s").unwrap();
                let template = view.windows[1].clone();
                for i in 0..8 {
                    let mut tab = template.clone();
                    tab.window_id = Some(format!("extra-{i}"));
                    tab.label = None;
                    view.windows.push(tab);
                }
                model.titles.insert(
                    "block:b".into(),
                    (
                        "long generated program title with no saved tab label".into(),
                        None,
                    ),
                );
                model
                    .progress
                    .insert("block:c".into(), TerminalProgress::Paused(Some(42)));
                let mut transfer = Transfer::new(
                    1,
                    "s".into(),
                    "block:a".into(),
                    "~/archive.tar.gz".into(),
                    std::env::temp_dir(),
                    1,
                );
                transfer.name = Some("archive.tar.gz".into());
                transfer.state = State::Receiving {
                    received: 42,
                    hint: 100,
                };
                model.downloads.push(transfer);
                let mut rt = runtime_at(ws, size);
                settle(&mut rt);
                assert!(has(&rt, "previous-tabs") && has(&rt, "next-tabs"));
                assert!(has(&rt, "download-chip"));
                assert_eq!(rect(&rt, "title-drag")[2], 48.);
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
                        .active_window_id = Some("extra-7".into());
                });
                settle(&mut rt);
                let last = element(&rt, "tab:extra-7");
                let strip = rect(&rt, "tabs");
                assert_eq!(last.visible[2], strip[2].min(last.rect[2]));
                assert_eq!(last.rect[0] + last.rect[2], strip[0] + strip[2]);
                assert!(
                    last.label
                        .as_deref()
                        .unwrap()
                        .contains("long generated program title")
                );
                assert_eq!(
                    element(&rt, "tab-word:extra-7").text.as_deref(),
                    Some("Paused 42%")
                );
                assert!(has(&rt, "tab-progress:extra-7"));
                assert_eq!(has(&rt, "tab-icon:extra-7:4"), count == 5);
                if fill {
                    let floor =
                        (28. + leading_width(usize::from(count)) + 40. + 8. + 10. * 7.).max(96.);
                    assert!(last.rect[2] >= floor);
                }
                host.drain();
                let [x, y, w, h] = last.visible;
                rt.secondary_down(x + w / 2., y + h / 2.);
                rt.secondary_up(x + w / 2., y + h / 2.);
                rt.frame(1. / 60.);
                assert!(rt.state.tab_targeted(0, "extra-7"));
                press(&mut rt, Key::Escape);
                rt.update(|s, _| {
                    let target = s.tab_target(0, "s", "extra-7").unwrap();
                    s.set_tab_muted(&target, true);
                });
                settle(&mut rt);
                assert!(has(&rt, "tab-muted:extra-7"));
                assert!(!has(&rt, "tab-word:extra-7") && !has(&rt, "tab-progress:extra-7"));
                assert!(host.is_empty());
                std::fs::remove_file(&rt.state.settings_path).ok();
            }
        }
    }
}

#[test]
fn cached_updates_keep_pane_order_and_sidebar_status_whole() {
    use rex_linux::connection::activity::TerminalProgress;
    let _families = rune::text::with_families("Inter", "Inter");
    let (mut ws, host) = tabbed_workspace("single");
    five_programs(&mut ws);
    ws.settings.interface.tab_icon_count = 5;
    ws.settings.vertical_tabs = true;
    ws.sidebar.toggle();
    ws.sync_prefetch();
    let mut rt = runtime_at(ws, (640., 480.));
    settle(&mut rt);
    let helps = |rt: &Runtime<Workspace>| {
        (0..5)
            .filter_map(|i| {
                let key = if i == 0 {
                    "sidebar-tab-icon:0:s:split".to_owned()
                } else {
                    format!("sidebar-tab-icon:0:s:split:{i}")
                };
                rt.elements()
                    .into_iter()
                    .find(|e| !e.exiting && e.key.as_deref() == Some(&key))
                    .map(|e| e.help)
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        helps(&rt),
        ["zsh", "claude", "nvim", "git", "cargo"].map(|p| Some(p.into()))
    );
    host.drain();
    rt.update(|s, _| {
        let m = &mut s.hosts[0].model;
        m.programs.insert("block:d".into(), "vim".into());
        m.progress
            .insert("block:c".into(), TerminalProgress::Paused(Some(42)));
    });
    settle(&mut rt);
    assert_eq!(
        helps(&rt),
        ["zsh", "claude", "vim", "git", "cargo"].map(|p| Some(p.into()))
    );
    let word = rt
        .elements()
        .into_iter()
        .find(|e| !e.exiting && e.text.as_deref() == Some("Paused 42%"))
        .unwrap();
    let row = rect(&rt, "sidebar-tab:0:s:split");
    assert!(
        word.rect[0] >= row[0] && word.rect[0] + word.rect[2] <= row[0] + row[2],
        "word={:?} row={row:?}",
        word.rect
    );
    assert!(has(&rt, "tab-progress:split"));
    rt.update(|s, _| {
        let m = &mut s.hosts[0].model;
        m.programs.insert("block:d".into(), "lazygit".into());
        m.views.get_mut("s").unwrap().windows[1].focused_block_id = Some("block:c".into());
    });
    settle(&mut rt);
    assert_eq!(
        helps(&rt),
        ["claude", "zsh", "lazygit", "cargo"].map(|p| Some(p.into()))
    );
    assert!(host.is_empty() && rt.state.claimed.is_empty());
    std::fs::remove_file(&rt.state.settings_path).ok();
}

/// Run separately in real fonts, never in a parallel font-global capture run.
#[test]
fn tab_icons_draw() {
    let Some(mut camera) = Camera::new() else {
        return;
    };
    let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
    let _families = rune::text::with_families("Inter", "IBM Plex Mono");
    let (mut ws, _host) = tabbed_workspace("single");
    five_programs(&mut ws);
    let (sender, _sent) = flume::unbounded();
    framed_terminal(
        &mut ws,
        "block:a",
        &sender,
        b"$ cargo test\r\nAll tests passed.",
    );
    framed_terminal(
        &mut ws,
        "block:b",
        &sender,
        b"$ git status\r\nWorking tree clean.",
    );
    framed_terminal(
        &mut ws,
        "block:c",
        &sender,
        b"$ cargo check\r\nChecking private fixture",
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
    for count in [0, 1, 5] {
        rt.update(|s, _| {
            s.settings.interface.tab_icon_count = count;
            s.settings.interface.tabs_fill_available_space = false;
        });
        settle(&mut rt);
        camera.snap(&mut rt, &format!("icons-{count}-toolbar"));
        rt.update(|s, _| {
            s.settings.interface.tabs_fill_available_space = true;
        });
        settle(&mut rt);
        camera.snap(&mut rt, &format!("icons-{count}-fill"));
        rt.update(|s, _| {
            s.settings.vertical_tabs = true;
            s.sidebar.toggle();
        });
        settle(&mut rt);
        camera.snap(&mut rt, &format!("icons-{count}-sidebar"));
        rt.update(|s, _| {
            let m = &mut s.hosts[0].model;
            m.views.get_mut("s").unwrap().windows[1].label =
                Some("long project name with program activity".into());
            m.progress.insert(
                "block:c".into(),
                rex_linux::connection::activity::TerminalProgress::Paused(Some(42)),
            );
        });
        settle(&mut rt);
        camera.snap(&mut rt, &format!("icons-{count}-sidebar-status"));
        rt.update(|s, _| {
            s.sidebar.toggle();
            s.settings.vertical_tabs = false;
            s.hosts[0]
                .model
                .views
                .get_mut("s")
                .unwrap()
                .active_window_id = Some("split".into());
        });
        settle(&mut rt);
        camera.snap(&mut rt, &format!("icons-{count}-active-status"));
        rt.update(|s, tx| {
            s.hosts[0]
                .model
                .views
                .get_mut("s")
                .unwrap()
                .active_window_id = Some("single".into());
            s.hosts[0].model.views.get_mut("s").unwrap().windows[1].label = Some("dev".into());
            s.hosts[0].model.progress.clear();
            s.perform(Intent::Settings, tx);
            tx.scroll_center("tab-icons-larger");
        });
        settle(&mut rt);
        camera.snap(&mut rt, &format!("icons-{count}-settings"));
        press(&mut rt, Key::Escape);
        settle(&mut rt);
    }
    std::fs::remove_file(&rt.state.settings_path).ok();
}
