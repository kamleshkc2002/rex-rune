//! Header-only fading: local preferences, rendered boundaries and existing focus policy.
use super::pane_verb_tests::{Camera, framed_terminal, rect};
use super::*;
use crate::app::{pane_header, panel::SettingsSection, theme::*};
use rex_linux::settings::{PaneDensity, PaneHeader};

fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
}

fn element(rt: &Runtime<Workspace>, key: &str) -> rune::ElementInfo {
    rt.elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("missing {key}"))
}

fn subtree(rt: &Runtime<Workspace>, key: &str) -> Vec<rune::ElementInfo> {
    let root = element(rt, key);
    rt.elements()
        .into_iter()
        .filter(|e| {
            !e.exiting && (e.path == root.path || e.path.starts_with(&format!("{}/", root.path)))
        })
        .collect()
}

fn assert_same(a: &[rune::ElementInfo], b: &[rune::ElementInfo]) {
    assert_eq!(a.len(), b.len());
    for (a, b) in a.iter().zip(b) {
        assert_eq!(
            (&a.key, &a.text, a.rect, a.fill, a.text_color, a.opacity),
            (&b.key, &b.text, b.rect, b.fill, b.text_color, b.opacity)
        );
    }
}

fn cues(ws: &mut Workspace) {
    use rex_linux::connection::activity::{ProgramState, TerminalProgress};
    let error = record(ProgramState::Working, true);
    ws.hosts[0]
        .model
        .progress
        .insert("block:c".into(), TerminalProgress::Error(Some(42)));
    ws.hosts[0]
        .model
        .activity
        .insert("block:c".into(), [("".into(), error)].into());
    let key = ws.term_key("block:c");
    let t = ws.terminals.get_mut(&key).unwrap();
    t.connection.model.lock().unwrap().connected = false;
    t.refresh();
}

#[test]
fn header_fading_matrix_changes_only_names_and_enabled_icons() {
    let _families = rune::text::with_families("Inter", "Inter");
    for density in [PaneDensity::Comfortable, PaneDensity::Compact] {
        for header in [
            PaneHeader::Auto,
            PaneHeader::Always,
            PaneHeader::OnHover,
            PaneHeader::Hidden,
        ] {
            for tab in ["single", "split", "zoom"] {
                let (mut ws, host) =
                    tabbed_workspace(if tab == "single" { "single" } else { "split" });
                ws.settings.pane_density = density;
                ws.settings.pane_header = header;
                if tab == "zoom" {
                    ws.hosts[0].model.views.get_mut("s").unwrap().windows[1].zoomed_block_id =
                        Some("block:b".into());
                }
                if header == PaneHeader::OnHover {
                    // The existing delayed-hover path is covered by pane_header_tests.
                    ws.header_reveal.hover = Some(
                        if tab == "single" {
                            "block:a"
                        } else {
                            "block:b"
                        }
                        .into(),
                    );
                }
                let blocks: &[&str] = if tab == "single" {
                    &["block:a"]
                } else if tab == "zoom" {
                    &["block:b"]
                } else {
                    &["block:b", "block:c"]
                };
                let (sender, sent) = flume::unbounded();
                for block in blocks {
                    framed_terminal(
                        &mut ws,
                        block,
                        &sender,
                        b"\x1b[31mred\x1b[0m unchanged\r\nselection",
                    );
                    let key = ws.term_key(block);
                    // Exercise layout feedback on owned panes as well as unowned ones.
                    ws.terminals[&key].connection.model.lock().unwrap().owner = *block != "block:c";
                    let t = ws.terminals.get_mut(&key).unwrap();
                    t.selection = Some((40, 44));
                    std::sync::Arc::make_mut(t.frame.as_mut().unwrap()).selection_rows[1] =
                        Some((0, 4));
                }
                if tab == "split" {
                    cues(&mut ws);
                }
                let mut rt = runtime(ws);
                settle(&mut rt);
                // Channel-backed workers cannot return a frame at the requested size.
                // Advance the initial-snapshot deadline explicitly, not wall-clock retries.
                rt.update(|s, _| {
                    for t in s.terminals.values_mut() {
                        t.draw_anyway(
                            std::time::Instant::now() + std::time::Duration::from_secs(10),
                        );
                    }
                });
                settle(&mut rt);
                for focused in ["block:b", "block:c"] {
                    for window_focused in [true, false] {
                        // Compare toggles at identical focus: the existing cursor intentionally
                        // hollows on focus loss, and native focus-in intentionally claims panes.
                        rt.update(|s, _| {
                            s.window_focused = window_focused;
                            if tab != "single" {
                                s.hosts[0].model.views.get_mut("s").unwrap().windows[1]
                                    .focused_block_id = Some(focused.into());
                            }
                            s.settings.interface.fade_inactive_pane_headers = false;
                            s.settings.interface.fade_pane_headers_in_inactive_window = false;
                        });
                        settle(&mut rt);
                        let grids: Vec<_> = blocks
                            .iter()
                            .map(|b| subtree(&rt, &format!("grid:{b}")))
                            .collect();
                        assert!(
                            grids
                                .iter()
                                .all(|grid| grid.iter().any(|e| e.text.is_some())),
                            "inspect actual terminal cells, not empty containers"
                        );
                        let headers: Vec<_> = blocks
                            .iter()
                            .filter(|b| has(&rt, &format!("pane-header:{b}")))
                            .map(|b| (*b, subtree(&rt, &format!("pane-header:{b}"))))
                            .collect();
                        sent.drain();
                        host.drain();
                        let claimed = rt.state.claimed.clone();
                        for pane in [false, true] {
                            for window in [false, true] {
                                rt.update(|s, _| {
                                    s.settings.interface.fade_inactive_pane_headers = pane;
                                    s.settings.interface.fade_pane_headers_in_inactive_window =
                                        window;
                                });
                                settle(&mut rt);
                                assert_eq!(
                                    blocks
                                        .iter()
                                        .filter(|b| has(&rt, &format!("pane-header:{b}")))
                                        .count(),
                                    headers.len(),
                                    "fading never reveals a hidden header"
                                );
                                for (block, before) in blocks.iter().zip(&grids) {
                                    let after = subtree(&rt, &format!("grid:{block}"));
                                    assert_eq!(
                                        before.len(),
                                        after.len(),
                                        "{density:?} {header:?} {tab} {focused} {window_focused} {pane} {window} {block}"
                                    );
                                    assert_same(before, &after);
                                }
                                for (block, before) in &headers {
                                    let after = subtree(&rt, &format!("pane-header:{block}"));
                                    assert_eq!(before.len(), after.len());
                                    let fade = if (pane
                                        && rt.state.focused_block().as_deref() != Some(block))
                                        || (window && !window_focused)
                                    {
                                        pane_header::FADE
                                    } else {
                                        1.
                                    };
                                    for (a, b) in before.iter().zip(&after) {
                                        assert_eq!(
                                            (a.rect, a.fill, a.opacity),
                                            (b.rect, b.fill, b.opacity)
                                        );
                                        let name =
                                            a.key.as_deref() == Some(&format!("pane-name:{block}"));
                                        let icon = [
                                            "pane-close:",
                                            "pane-split-right:",
                                            "pane-split-down:",
                                            "pane-zoom:",
                                        ]
                                        .iter()
                                        .any(|key| a.path.contains(key))
                                            && a.text_color.is_some()
                                            && a.opacity == 1.;
                                        if name || icon {
                                            let color = a.text_color.unwrap();
                                            assert_eq!(
                                                b.text_color,
                                                Some(color.alpha(color.a * fade))
                                            );
                                        } else {
                                            assert_eq!(
                                                a.text_color, b.text_color,
                                                "cue/disabled icon: {}",
                                                a.path
                                            );
                                        }
                                    }
                                }
                                assert!(
                                    sent.is_empty() && host.is_empty(),
                                    "no resize, focus, ownership or input"
                                );
                                assert_eq!(rt.state.claimed, claimed);
                            }
                        }
                    }
                }
                if tab == "split" && header == PaneHeader::Always {
                    assert!(has(&rt, "pane-word:block:c"));
                    assert!(has(&rt, "pane-progress:block:c"));
                    assert!(has(&rt, "strip-action:block:c"));
                }
                std::fs::remove_file(&rt.state.settings_path).ok();
            }
        }
    }
}

#[test]
fn header_fading_native_focus_callback_keeps_existing_claim_policy() {
    let _families = rune::text::with_families("Inter", "Inter");
    for on in [false, true] {
        let (mut ws, host) = tabbed_workspace("split");
        ws.settings.interface.fade_pane_headers_in_inactive_window = on;
        let (sender, sent) = flume::unbounded();
        framed_terminal(&mut ws, "block:b", &sender, b"\x1b[?25lunchanged");
        let mut rt = runtime(ws);
        settle(&mut rt);
        rt.window_focus(true);
        settle(&mut rt);
        host.drain();
        let before = subtree(&rt, "grid:block:b");
        rt.window_focus(false);
        settle(&mut rt);
        assert!(!rt.state.window_focused);
        assert_eq!(
            element(&rt, "pane-name:block:b").text_color.unwrap().a,
            if on { pane_header::FADE } else { 1. }
        );
        assert_same(&before, &subtree(&rt, "grid:block:b"));
        assert!(sent.is_empty() && host.is_empty());
        rt.window_focus(true);
        settle(&mut rt);
        assert!(rt.state.window_focused);
        assert_eq!(element(&rt, "pane-name:block:b").text_color.unwrap().a, 1.);
        assert_same(&before, &subtree(&rt, "grid:block:b"));
        let commands: Vec<_> = host.try_iter().collect();
        assert_eq!(
            commands.len(),
            1,
            "focus-in retains the existing visible-pane claim"
        );
        assert!(
            matches!(&commands[0], HostCommand::Claim { blocks, .. } if blocks == &vec!["block:b".to_owned(), "block:c".to_owned()])
        );
        assert!(sent.is_empty(), "no terminal resize, focus or input");
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn header_fading_switches_search_save_and_fit_at_minimum_size() {
    let _families = rune::text::with_families("Inter", "Inter");
    for size in [(1024., 700.), (640., 480.)] {
        let (mut ws, host) = tabbed_workspace("split");
        ws.perform(Intent::Settings, &mut rune::Tx::default());
        let mut rt = runtime_at(ws, size);
        for (key, label) in [
            ("fade-inactive-headers", "Fade inactive pane headers"),
            (
                "fade-headers-inactive-window",
                "Fade headers in inactive windows",
            ),
        ] {
            rt.update(|s, _| {
                s.settings_nav.search.text = label.into();
                s.settings_search_changed();
            });
            let hits = rt.state.settings_nav.hits();
            assert_eq!(hits[0].label, label);
            assert_eq!(hits[0].page, SettingsSection::Interface);
            rt.update(|s, tx| {
                s.settings_nav.search.text.clear();
                s.settings_search_changed();
                tx.scroll_center(key);
            });
            settle(&mut rt);
            let e = element(&rt, key);
            let [x, y, w, h] = e.rect;
            assert!(x >= 0. && y >= 0. && x + w <= size.0 && y + h <= size.1);
            assert_eq!(e.rect, e.visible);
            for on in [true, false, true] {
                click(&mut rt, key);
                settle(&mut rt);
                let saved = Settings::load(&rt.state.settings_path).unwrap();
                assert_eq!(
                    if key == "fade-inactive-headers" {
                        saved.interface.fade_inactive_pane_headers
                    } else {
                        saved.interface.fade_pane_headers_in_inactive_window
                    },
                    on
                );
            }
        }
        let again = relaunch(&rt.state);
        assert!(again.settings.interface.fade_inactive_pane_headers);
        assert!(
            again
                .settings
                .interface
                .fade_pane_headers_in_inactive_window
        );
        assert!(host.is_empty() && rt.state.claimed.is_empty());
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn header_fading_preserves_hover_disabled_buttons_and_keyboard_destination() {
    let _families = rune::text::with_families("Inter", "Inter");
    let (mut ws, host) = tabbed_workspace("single");
    ws.settings.pane_header = PaneHeader::Always;
    ws.settings.interface.fade_pane_headers_in_inactive_window = true;
    ws.window_focused = false;
    let (sender, sent) = flume::unbounded();
    framed_terminal(&mut ws, "block:a", &sender, b"$ ");
    let mut rt = runtime(ws);
    settle(&mut rt);
    let disabled = subtree(&rt, "pane-zoom:block:a");
    click(&mut rt, "pane-zoom:block:a");
    settle(&mut rt);
    assert_same(&disabled, &subtree(&rt, "pane-zoom:block:a"));
    assert!(host.is_empty() && sent.is_empty());
    let [x, y, w, h] = rect(&rt, "pane-close:block:a");
    rt.pointer_move(x + w / 2., y + h / 2.);
    settle(&mut rt);
    let icon = subtree(&rt, "pane-close:block:a")
        .into_iter()
        .find(|e| e.text_color.is_some())
        .unwrap();
    assert_eq!(icon.text_color, Some(rgb(ink())));
    assert_eq!(icon.opacity, 1.);
    click(&mut rt, "pane-split-right:block:a");
    assert_eq!(requests(&host)[0].0, "session.new_split");
    assert_eq!(
        focused(&rt),
        None,
        "header buttons never steal the keyboard"
    );
    assert!(sent.is_empty());
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn header_name_hover_restores_colour_without_focus_or_input() {
    let _families = rune::text::with_families("Inter", "Inter");
    for (tab, block) in [("single", "block:a"), ("split", "block:c")] {
        let (mut ws, host) = tabbed_workspace(tab);
        ws.settings.pane_header = PaneHeader::Always;
        ws.window_focused = false;
        let (sender, sent) = flume::unbounded();
        framed_terminal(&mut ws, block, &sender, b"unchanged");
        let mut rt = runtime(ws);
        settle(&mut rt);
        let key = format!("pane-name:{block}");
        let original = element(&rt, &key).text_color.unwrap();
        let grid = subtree(&rt, &format!("grid:{block}"));
        rt.update(|s, _| s.settings.interface.fade_pane_headers_in_inactive_window = true);
        settle(&mut rt);
        let faded = Some(original.alpha(original.a * pane_header::FADE));
        assert_eq!(element(&rt, &key).text_color, faded);
        let [x, y, w, h] = rect(&rt, &key);
        rt.pointer_move(x + w / 2., y + h / 2.);
        settle(&mut rt);
        assert_eq!(element(&rt, &key).text_color, Some(original));
        rt.pointer_move(0., 0.);
        settle(&mut rt);
        assert_eq!(element(&rt, &key).text_color, faded);
        assert_same(&grid, &subtree(&rt, &format!("grid:{block}")));
        assert!(host.is_empty() && sent.is_empty() && rt.state.claimed.is_empty());
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn header_fading_keeps_interface_zoom_and_floating_header_geometry() {
    let _families = rune::text::with_families("Inter", "Inter");
    for scale in [0.7, 1., 1.5] {
        let (mut ws, host) = tabbed_workspace("split");
        ws.settings.ui_scale = scale;
        ws.settings.pane_header = PaneHeader::OnHover;
        ws.header_reveal.pinned = true;
        ws.window_focused = false;
        let (sender, sent) = flume::unbounded();
        framed_terminal(&mut ws, "block:b", &sender, b"unchanged");
        let mut rt = runtime_at(ws, (1440., 1000.));
        settle(&mut rt);
        let grid = subtree(&rt, "grid:block:b");
        let header = rect(&rt, "pane-header:block:b");
        rt.update(|s, _| s.settings.interface.fade_pane_headers_in_inactive_window = true);
        settle(&mut rt);
        assert_same(&grid, &subtree(&rt, "grid:block:b"));
        assert_eq!(header, rect(&rt, "pane-header:block:b"));
        assert_eq!(
            element(&rt, "pane-name:block:b").text_color.unwrap().a,
            pane_header::FADE
        );
        assert!(sent.is_empty() && host.is_empty() && rt.state.claimed.is_empty());
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

/// Real-font evidence only when requested; run individually, never with parallel font loaders.
#[test]
fn header_fading_draw() {
    let Some(mut camera) = Camera::new() else {
        return;
    };
    let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
    let _families = rune::text::with_families("Inter", "IBM Plex Mono");
    let (mut ws, _host) = tabbed_workspace("split");
    let (sender, _sent) = flume::unbounded();
    for block in ["block:b", "block:c"] {
        framed_terminal(
            &mut ws,
            block,
            &sender,
            b"\x1b[31mred output\x1b[0m unchanged\r\n$ cargo test",
        );
    }
    cues(&mut ws);
    let size = if std::env::var_os("REX_M27_MINIMUM").is_some() {
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
    camera.snap(&mut rt, "headers-default");
    rt.update(|s, _| s.settings.interface.fade_inactive_pane_headers = true);
    settle(&mut rt);
    camera.snap(&mut rt, "headers-inactive-pane");
    let [x, y, w, h] = rect(&rt, "pane-name:block:c");
    rt.pointer_move(x + w / 2., y + h / 2.);
    settle(&mut rt);
    camera.snap(&mut rt, "headers-inactive-name-hover");
    rt.pointer_move(0., 0.);
    settle(&mut rt);
    rt.update(|s, _| {
        s.settings.interface.fade_pane_headers_in_inactive_window = true;
        s.window_focus(false);
    });
    settle(&mut rt);
    camera.snap(&mut rt, "headers-inactive-window");
    rt.update(|s, _| {
        s.settings.pane_header = PaneHeader::OnHover;
        s.header_reveal.pinned = true;
    });
    settle(&mut rt);
    camera.snap(&mut rt, "headers-floating");
    rt.update(|s, tx| {
        s.perform(Intent::Settings, tx);
        tx.scroll_center("fade-inactive-headers");
    });
    settle(&mut rt);
    camera.snap(&mut rt, "header-fading-settings");
    std::fs::remove_file(&rt.state.settings_path).ok();
}
