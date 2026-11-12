//! M2.8 colour only: cached glyphs, unchanged chrome/terminal geometry and input.
use super::pane_verb_tests::{Camera, framed_terminal, rect};
use super::tab_icon_tests::{element, five_programs, settle};
use super::*;
use crate::app::{activity::tab_programs, tab_icons::*, theme::*};
use rex_rune::settings::{ChromeTheme, interface::TabIconColor};

const MODES: [TabIconColor; 4] = [
    TabIconColor::Always,
    TabIconColor::ActivePane,
    TabIconColor::ActiveTab,
    TabIconColor::Never,
];

fn glyph_color(rt: &Runtime<Workspace>, key: &str) -> rune::Color {
    let root = element(rt, key);
    rt.elements()
        .iter()
        .find(|e| {
            !e.exiting && e.path.starts_with(&format!("{}/", root.path)) && e.text_color.is_some()
        })
        .unwrap_or_else(|| panic!("no glyph below {key}"))
        .text_color
        .unwrap()
}

#[test]
fn complete_colour_truth_table_and_focused_dedup_fallback_counts() {
    for (mode, expected) in MODES.into_iter().zip([
        [true, true, true, true],
        [false, false, false, true],
        [false, false, true, true],
        [false, false, false, false],
    ]) {
        for ((on_screen, focused), expected) in
            [(false, false), (false, true), (true, false), (true, true)]
                .into_iter()
                .zip(expected)
        {
            assert_eq!(
                in_color(mode, on_screen, focused),
                expected,
                "{mode:?} {on_screen} {focused}"
            );
        }
    }
    let _families = rune::text::with_families("Inter", "Inter");
    let (mut ws, _) = tabbed_workspace("split");
    five_programs(&mut ws);
    let model = &mut ws.hosts[0].model;
    model.programs.insert("block:d".into(), "bash".into()); // same glyph as focused zsh
    model.programs.insert("block:f".into(), "nvim".into());
    let mut tab = model.views["s"].windows[1].clone();
    for focus in [Some("block:b"), Some("block:e"), None, Some("missing")] {
        tab.focused_block_id = focus.map(str::to_owned);
        for count in [0, 1, 5] {
            let icons = icons_of(&tab_programs(model, &tab), count);
            let first = if focus == Some("block:e") {
                "rex-git"
            } else {
                "rex-terminal"
            };
            assert_eq!(
                icons.first().map(|i| i.icon.name),
                (count > 0).then_some(first)
            );
            let names: std::collections::HashSet<_> = icons.iter().map(|i| i.icon.name).collect();
            assert_eq!(names.len(), icons.len());
            assert_eq!(icons.len(), if count == 5 { 4 } else { usize::from(count) });
        }
    }
    for programs in [
        vec![],
        vec![None],
        vec![Some("mystery".into()), Some("zsh".into())],
    ] {
        for count in [0, 1, 5] {
            let icons = icons_of(&programs, count);
            assert_eq!(icons.len(), usize::from(count > 0));
            if count > 0 {
                assert_eq!(icons[0].icon.name, "rex-terminal");
                assert_eq!(tint(icons[0].icon), 0xa1edac);
            }
        }
    }
}

#[test]
fn modes_change_only_real_glyph_colours_not_geometry_content_or_channels() {
    let _families = rune::text::with_families("Inter", "Inter");
    for size in [(1024., 700.), (640., 480.)] {
        for sidebar in [false, true] {
            for fill in [false, true] {
                for count in [0, 1, 5] {
                    let (mut ws, host) = tabbed_workspace("split");
                    five_programs(&mut ws);
                    ws.settings.interface.tab_icon_count = count;
                    ws.settings.interface.tabs_fill_available_space = fill;
                    ws.settings.vertical_tabs = sidebar;
                    if sidebar {
                        ws.sidebar.toggle();
                    }
                    ws.sync_prefetch();
                    ws.hosts[0].model.progress.insert(
                        "block:c".into(),
                        rex_rune::connection::activity::TerminalProgress::Paused(Some(42)),
                    );
                    let (sender, sent) = flume::unbounded();
                    for block in ["block:b", "block:c"] {
                        framed_terminal(&mut ws, block, &sender, b"\x1b[31mred\x1b[0m unchanged");
                        let t = ws.terminals.get_mut(&ws.term_key(block)).unwrap();
                        t.connection.model.lock().unwrap().owner = block == "block:b";
                    }
                    let mut rt = runtime_at(ws, size);
                    settle(&mut rt);
                    rt.update(|s, _| {
                        for t in s.terminals.values_mut() {
                            t.draw_anyway(
                                std::time::Instant::now() + std::time::Duration::from_secs(60),
                            );
                        }
                    });
                    settle(&mut rt);
                    let before: Vec<_> = rt.elements().into_iter().filter(|e| !e.exiting).collect();
                    // Inter's advances differ from a terminal cell, so the renderer
                    // intentionally emits one text segment per character here.
                    let grid = element(&rt, "grid:block:b");
                    let content: String = before
                        .iter()
                        .filter(|e| e.path.starts_with(&format!("{}/", grid.path)))
                        .filter_map(|e| e.text.as_deref())
                        .collect();
                    assert!(content.starts_with("red unchanged"), "{content:?}");
                    host.drain();
                    sent.drain();
                    for mode in MODES {
                        rt.update(|s, _| s.settings.interface.tab_icon_color = mode);
                        settle(&mut rt);
                        let after: Vec<_> =
                            rt.elements().into_iter().filter(|e| !e.exiting).collect();
                        assert_eq!(before.len(), after.len());
                        for (a, b) in before.iter().zip(&after) {
                            assert_eq!(
                                (
                                    &a.path, &a.key, &a.text, &a.label, &a.help, a.rect, a.fill,
                                    a.opacity
                                ),
                                (
                                    &b.path, &b.key, &b.text, &b.label, &b.help, b.rect, b.fill,
                                    b.opacity
                                ),
                                "{size:?} sidebar={sidebar} fill={fill} count={count} mode={mode:?}"
                            );
                            if !a.path.contains("tab-icon:") {
                                assert_eq!(a.text_color, b.text_color, "non-icon {}", a.path);
                            }
                        }
                        for (tab, active) in [("split", true), ("single", false)] {
                            let key = if sidebar {
                                format!("sidebar-tab-icon:0:s:{tab}")
                            } else {
                                format!("tab-icon:{tab}")
                            };
                            let tab = rt.state.hosts[0].model.views["s"]
                                .windows
                                .iter()
                                .find(|t| t.window_id.as_deref() == Some(tab))
                                .unwrap();
                            let icons =
                                icons_of(&tab_programs(&rt.state.hosts[0].model, tab), count);
                            assert_eq!(has(&rt, &key), count > 0);
                            for (i, icon) in icons.iter().enumerate() {
                                let expected = match mode {
                                    TabIconColor::Always => tint(icon.icon),
                                    TabIconColor::ActiveTab if active => tint(icon.icon),
                                    TabIconColor::ActivePane if active && i == 0 => tint(icon.icon),
                                    _ => {
                                        if active {
                                            ink()
                                        } else {
                                            muted()
                                        }
                                    }
                                };
                                let key = if i == 0 {
                                    key.clone()
                                } else {
                                    format!("{key}:{i}")
                                };
                                assert_eq!(glyph_color(&rt, &key), rgb(expected));
                            }
                        }
                        assert!(host.is_empty() && sent.is_empty() && rt.state.claimed.is_empty());
                        if !sidebar {
                            assert!(rect(&rt, "title-drag")[2] >= 48.);
                        }
                    }
                    std::fs::remove_file(&rt.state.settings_path).ok();
                }
            }
        }
    }
}

#[test]
fn sidebar_on_screen_requires_host_session_and_window_not_colliding_ids() {
    let _families = rune::text::with_families("Inter", "Inter");
    let (source, _) = tabbed_workspace("split");
    let mut ws = workspace_hosts(2);
    let channels = [capture_host(&mut ws, 0), capture_host(&mut ws, 1)];
    for host in &mut ws.hosts {
        host.model.connected = true;
        host.model.sessions = vec![session("s"), session("other")];
        host.model.selected = Some("s".into());
        for id in ["s", "other"] {
            let mut v = source.hosts[0].model.views["s"].clone();
            v.session_id = Some(id.into());
            host.model.views.insert(id.into(), v);
        }
        host.model.programs.insert("block:b".into(), "git".into());
        host.model.programs.insert("block:c".into(), "nvim".into());
    }
    ws.settings.vertical_tabs = true;
    ws.settings.interface.tab_icon_count = 5;
    if !ws.sidebar.is_open() {
        ws.sidebar.toggle();
    }
    ws.sync_prefetch();
    let mut rt = runtime_at(ws, (1024., 700.));
    settle(&mut rt);
    for rx in &channels {
        rx.drain();
    }
    for mode in MODES {
        rt.update(|s, _| s.settings.interface.tab_icon_color = mode);
        settle(&mut rt);
        for host in 0..2 {
            for session in ["s", "other"] {
                let active = host == 0 && session == "s";
                for (i, name) in ["git", "nvim"].into_iter().enumerate() {
                    let key = format!("sidebar-tab-icon:{host}:{session}:split");
                    let key = if i == 0 { key } else { format!("{key}:{i}") };
                    let colored = match mode {
                        TabIconColor::Always => true,
                        TabIconColor::ActiveTab => active,
                        TabIconColor::ActivePane => active && i == 0,
                        TabIconColor::Never => false,
                    };
                    let color = if colored {
                        tint(crate::app::activity::program_icon(Some(name)))
                    } else if active {
                        ink()
                    } else {
                        muted()
                    };
                    assert_eq!(glyph_color(&rt, &key), rgb(color), "{mode:?} {key}");
                }
            }
        }
        assert!(channels.iter().all(|rx| rx.is_empty()) && rt.state.claimed.is_empty());
    }
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn colour_search_anchored_keyboard_save_relaunch_and_inert_count_preview() {
    let _families = rune::text::with_families("Inter", "Inter");
    for size in [(1024., 700.), (640., 480.)] {
        let (mut ws, host) = tabbed_workspace("single");
        let (sender, sent) = flume::unbounded();
        framed_terminal(&mut ws, "block:a", &sender, b"no leaked input");
        ws.perform(Intent::Settings, &mut rune::Tx::default());
        let mut rt = runtime_at(ws, size);
        rt.update(|s, _| {
            s.settings_nav.search.text = "Use colour for tab icons".into();
            s.settings_search_changed();
        });
        let hit = &rt.state.settings_nav.hits()[0];
        assert_eq!(hit.label, "Use colour for tab icons");
        assert_eq!(hit.page, crate::app::panel::SettingsSection::Interface);
        press(&mut rt, Key::Enter);
        settle(&mut rt);
        assert_eq!(
            element(&rt, "tab-icon-color").rect,
            element(&rt, "tab-icon-color").visible
        );
        host.drain();
        sent.drain();
        for (i, mode) in MODES.into_iter().enumerate() {
            click(&mut rt, "tab-icon-color");
            settle(&mut rt);
            assert_eq!(
                rt.state.panel.as_ref().unwrap().anchor_to.as_deref(),
                Some("tab-icon-color")
            );
            let control = rect(&rt, "tab-icon-color");
            let rows: Vec<_> = rt
                .elements()
                .into_iter()
                .filter(|e| !e.exiting && e.role == "menuitem")
                .collect();
            assert_eq!(rows.len(), 4);
            assert_eq!(
                rows.iter()
                    .map(|e| e.label.as_deref().unwrap())
                    .collect::<Vec<_>>(),
                ["Always", "Active pane", "Active tab", "Never"]
            );
            for row in &rows {
                let [x, y, w, h] = row.rect;
                assert!(x >= 0. && y >= 0. && x + w <= size.0 && y + h <= size.1);
                assert!(x < control[0] + control[2] && x + w > control[0]);
            }
            // The kit preselects the saved choice. Navigate from Never, then from
            // the prior index: exercise real popup key routing, not direct apply.
            let current = if i == 0 { 3 } else { i - 1 };
            for _ in i..current {
                press(&mut rt, Key::Up);
            }
            for _ in current..i {
                press(&mut rt, Key::Down);
            }
            press(&mut rt, Key::Enter);
            settle(&mut rt);
            assert!(matches!(
                rt.state.panel.as_ref().map(|p| &p.kind),
                Some(PanelKind::Settings)
            ));
            assert!(focused(&rt).unwrap().contains("tab-icon-color"));
            assert_eq!(rt.state.settings.interface.tab_icon_color, mode);
            assert_eq!(
                Settings::load(&rt.state.settings_path)
                    .unwrap()
                    .interface
                    .tab_icon_color,
                mode
            );
            assert_eq!(relaunch(&rt.state).settings.interface.tab_icon_color, mode);
            for count in [0, 1, 5] {
                rt.update(|s, tx| {
                    s.settings.interface.tab_icon_count = count;
                    tx.scroll_center("tab-icons-preview");
                });
                settle(&mut rt);
                for (active, key) in [
                    (true, "tab-icons-preview-icon"),
                    (false, "tab-icons-preview-inactive"),
                ] {
                    assert_eq!(has(&rt, key), count > 0);
                    for (j, name) in ["zsh", "claude", "nvim", "git", "cargo"]
                        .into_iter()
                        .take(count as usize)
                        .enumerate()
                    {
                        let key = if j == 0 {
                            key.to_owned()
                        } else {
                            format!("{key}:{j}")
                        };
                        let e = element(&rt, &key);
                        assert_eq!(e.rect, e.visible, "{size:?} {key}");
                        let expected = if in_color(mode, active, j == 0) {
                            tint(crate::app::activity::program_icon(Some(name)))
                        } else if active {
                            ink()
                        } else {
                            muted()
                        };
                        assert_eq!(glyph_color(&rt, &key), rgb(expected));
                    }
                }
                assert!(element(&rt, "tab-icons-preview").inert);
                click(&mut rt, "tab-icons-preview");
                assert!(host.is_empty() && sent.is_empty() && rt.state.claimed.is_empty());
                if count == 0 {
                    assert!(rt.elements().iter().any(|e| e.text.as_deref()
                        == Some("No icons are shown while Show tab icons is 0.")));
                }
            }
            rt.update(|_, tx| tx.scroll_center("tab-icon-color"));
            settle(&mut rt);
        }
        // Escape dismisses just the choice and restores its control.
        click(&mut rt, "tab-icon-color");
        press(&mut rt, Key::Escape);
        settle(&mut rt);
        assert!(focused(&rt).unwrap().contains("tab-icon-color"));
        assert_eq!(
            rt.state.settings.interface.tab_icon_color,
            TabIconColor::Never
        );
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

/// The renderer composites translucent white in linear RGB. Test the actual
/// supported surfaces, not an sRGB mix (which understates their brightness).
#[test]
fn glyph_tints_contrast_on_supported_tab_and_preview_surfaces() {
    fn over(base: rune::Color, alpha: f32) -> rune::Color {
        rune::Color {
            r: alpha + base.r * (1. - alpha),
            g: alpha + base.g * (1. - alpha),
            b: alpha + base.b * (1. - alpha),
            a: 1.,
        }
    }
    fn luminance(c: rune::Color) -> f32 {
        c.r * 0.2126 + c.g * 0.7152 + c.b * 0.0722
    }
    for chrome in ChromeTheme::ALL {
        let p = palette_of(chrome);
        for (surface, base) in [
            ("toolbar", rgb(p.ground)),
            ("sidebar", over(rgb(p.ground), 0.025)),
            ("preview", over(rgb(p.popover), 0.04)),
        ] {
            for (state, alpha) in [("plain", 0.), ("hover", 0.05), ("selected", 0.11)] {
                let background = over(base, alpha);
                for program in [
                    "claude", "codex", "opencode", "pi", "nvim", "git", "cargo", "ssh", "zsh",
                ] {
                    let foreground = rgb(tint(crate::app::activity::program_icon(Some(program))));
                    let contrast = (luminance(foreground) + 0.05) / (luminance(background) + 0.05);
                    assert!(
                        contrast >= 3.,
                        "{} {surface} {state} {program}: {contrast}",
                        chrome.name()
                    );
                }
            }
        }
    }
}

/// Real-font captures are run individually, including all supported chrome palettes.
#[test]
fn tab_icon_colours_draw() {
    let Some(mut camera) = Camera::new() else {
        return;
    };
    let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
    let _families = rune::text::with_families("Inter", "IBM Plex Mono");
    let (mut ws, _) = tabbed_workspace("split");
    five_programs(&mut ws);
    ws.settings.interface.tab_icon_count = 5;
    let (sender, _) = flume::unbounded();
    for block in ["block:b", "block:c"] {
        framed_terminal(
            &mut ws,
            block,
            &sender,
            b"$ colour is local chrome\r\n\x1b[31mTerminal red stays red.",
        );
    }
    // Both tabs show five distinct cached programs; inactive is a read-only sample.
    let layers = ws.hosts[0].model.views["s"].windows[1].layers.clone();
    ws.hosts[0].model.views.get_mut("s").unwrap().windows[0].layers = layers;
    ws.hosts[0].model.views.get_mut("s").unwrap().windows[0].focused_block_id =
        Some("block:b".into());
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
    for (palette, chrome) in ChromeTheme::ALL.into_iter().enumerate() {
        for (mode_id, mode) in MODES.into_iter().enumerate() {
            rt.update(|s, _| {
                s.settings.chrome_theme = chrome;
                s.settings.interface.tab_icon_color = mode;
            });
            settle(&mut rt);
            camera.snap(
                &mut rt,
                &format!("palette-{palette}-mode-{mode_id}-toolbar"),
            );
            if mode == TabIconColor::Always {
                let [x, y, w, h] = element(&rt, "tab:single").visible;
                rt.pointer_move(x + w / 2., y + h / 2.);
                settle(&mut rt);
                camera.snap(&mut rt, &format!("palette-{palette}-toolbar-hover"));
                rt.pointer_move(0., 0.);
            }
            rt.update(|s, _| {
                s.settings.vertical_tabs = true;
                s.sidebar.toggle();
            });
            settle(&mut rt);
            camera.snap(
                &mut rt,
                &format!("palette-{palette}-mode-{mode_id}-sidebar"),
            );
            if mode == TabIconColor::Always {
                let [x, y, w, h] = rect(&rt, "sidebar-tab:0:s:single");
                rt.pointer_move(x + w / 2., y + h / 2.);
                settle(&mut rt);
                camera.snap(&mut rt, &format!("palette-{palette}-sidebar-hover"));
                rt.pointer_move(0., 0.);
                rt.update(|s, _| {
                    for (block, program) in ["block:b", "block:c", "block:d", "block:e", "block:f"]
                        .into_iter()
                        .zip(["codex", "opencode", "pi", "ssh", "unknown"])
                    {
                        s.hosts[0]
                            .model
                            .programs
                            .insert(block.into(), program.into());
                    }
                });
                settle(&mut rt);
                camera.snap(&mut rt, &format!("palette-{palette}-remaining-glyphs"));
                rt.update(|s, _| five_programs(s));
            }
            rt.update(|s, _| {
                s.sidebar.toggle();
                s.settings.vertical_tabs = false;
            });
            rt.update(|s, tx| {
                s.perform(Intent::Settings, tx);
                tx.scroll_center("tab-icons-preview");
            });
            settle(&mut rt);
            camera.snap(
                &mut rt,
                &format!("palette-{palette}-mode-{mode_id}-preview"),
            );
            press(&mut rt, Key::Escape);
            settle(&mut rt);
        }
    }
    std::fs::remove_file(&rt.state.settings_path).ok();
}
