//! M2.7 first piece: independent padding and chrome-only active-pane outlines.
use super::pane_verb_tests::{Camera, framed_terminal, rect};
use super::*;
use crate::app::{find, panel::SettingsSection, panes};
use rex_linux::{
    connection::Command,
    settings::{PaneDensity, PaneHeader, typography::SpacingUnit},
};

fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
}

fn tap(rt: &mut Runtime<Workspace>, key: &str) {
    rt.update(|_, tx| tx.scroll_center(key));
    settle(rt);
    click(rt, key);
    settle(rt);
}

#[test]
fn independent_padding_controls_save_and_search_uses_the_actual_labels() {
    let mut ws = workspace();
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    ws.open_section(SettingsSection::Terminal);
    let mut rt = runtime(ws);
    tap(&mut rt, "pane-padding-larger");
    assert_eq!(rt.state.settings.padding(), (9., 8.));
    tap(&mut rt, "pane-padding-y-smaller");
    assert_eq!(
        Settings::load(&rt.state.settings_path).unwrap().padding(),
        (9., 7.)
    );
    for (label, section) in [
        ("Horizontal padding", SettingsSection::Terminal),
        ("Vertical padding", SettingsSection::Terminal),
        ("Active pane outline", SettingsSection::Interface),
    ] {
        rt.update(|s, _| {
            s.settings_nav.search.text = label.into();
            s.settings_search_changed();
        });
        let hits = rt.state.settings_nav.hits();
        assert_eq!(hits[0].label, label);
        assert_eq!(hits[0].page, section);
    }
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn outline_control_saves_on_off_and_returns_to_auto_at_both_sizes() {
    for size in [(1024., 700.), (640., 480.)] {
        let (mut ws, host) = tabbed_workspace("split");
        ws.perform(Intent::Settings, &mut rune::Tx::default());
        let mut rt = runtime_at(ws, size);
        for (label, value) in [("Off", Some(false)), ("On", Some(true)), ("Auto", None)] {
            tap(&mut rt, "active-pane-outline");
            let row = rt
                .elements()
                .into_iter()
                .find(|e| !e.exiting && e.role == "menuitem" && e.label.as_deref() == Some(label))
                .unwrap();
            let [x, y, w, h] = row.rect;
            assert!(x >= 0. && y >= 0. && x + w <= size.0 && y + h <= size.1);
            rt.pointer_down(x + w / 2., y + h / 2.);
            rt.pointer_up(x + w / 2., y + h / 2.);
            settle(&mut rt);
            assert_eq!(
                Settings::load(&rt.state.settings_path)
                    .unwrap()
                    .interface
                    .highlight_active_pane,
                value
            );
            assert!(matches!(
                rt.state.panel.as_ref().map(|p| &p.kind),
                Some(PanelKind::Settings)
            ));
            assert!(host.is_empty() && rt.state.claimed.is_empty());
        }
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn asymmetric_padding_keeps_layout_and_sizes_only_owned_panes() {
    let _families = rune::text::with_families("Inter", "Inter");
    for density in [PaneDensity::Comfortable, PaneDensity::Compact] {
        for foreign_background in [false, true] {
            let (mut ws, host) = tabbed_workspace("split");
            ws.settings.pane_density = density;
            ws.settings.pane_header = PaneHeader::Always;
            if foreign_background {
                ws.term_theme = Some(rex_linux::terminal_theme::TerminalTheme {
                    background: Some(0x331122),
                    ..Default::default()
                });
            }
            ws.settings.terminal.font.line_height = 20;
            ws.settings.terminal.font.spacing = 20;
            ws.settings.terminal.font.spacing_unit = SpacingUnit::Percent;
            let (sender, sent) = flume::unbounded();
            for block in ["block:b", "block:c"] {
                framed_terminal(
                    &mut ws,
                    block,
                    &sender,
                    if foreign_background {
                        b"\x1b]11;#331122\x07$ hello"
                    } else {
                        b"$ hello"
                    },
                );
                ws.terminals[&ws.term_key(block)]
                    .connection
                    .model
                    .lock()
                    .unwrap()
                    .owner = block == "block:b";
            }
            if foreign_background {
                assert_eq!(
                    ws.terminals[&ws.term_key("block:b")].body(&ws.term_colors()),
                    0x331122
                );
            }
            let mut rt = runtime(ws);
            settle(&mut rt);
            let panes_before = [rect(&rt, "pane:block:b"), rect(&rt, "pane:block:c")];
            let grids_before = [rect(&rt, "grid:block:b"), rect(&rt, "grid:block:c")];
            sent.drain();
            host.drain();
            rt.update(|s, _| {
                s.settings.pane_padding = 24.;
                s.settings.interface.pane_padding_vertical = Some(2.);
            });
            settle(&mut rt);
            let cell = rt.state.metrics().cell;
            for (i, block) in ["block:b", "block:c"].iter().enumerate() {
                let grid = rect(&rt, &format!("grid:{block}"));
                let old = grids_before[i];
                assert_eq!(rect(&rt, &format!("pane:{block}")), panes_before[i]);
                assert_eq!(
                    grid,
                    [old[0] + 16., old[1] - 6., old[2] - 32., old[3] + 12.]
                );
                assert_eq!(rt.state.terminals[&rt.state.term_key(block)].rect, grid);
            }
            let grid = rect(&rt, "grid:block:b");
            let commands: Vec<_> = sent.try_iter().collect();
            assert_eq!(commands.len(), 1, "only the owner's new size");
            assert!(
                matches!(&commands[0], HostCommand::Terminal { block, command: Command::Resize(cols, rows, w, h), .. }
                if block == "block:b" && (*cols, *rows, *w, *h) == ((grid[2]/cell.0).floor() as u16, (grid[3]/cell.1).floor() as u16, cell.0.ceil() as u16, cell.1.ceil() as u16))
            );
            assert!(
                host.is_empty() && rt.state.claimed.is_empty(),
                "no input, ownership or shared-layout change"
            );
            std::fs::remove_file(&rt.state.settings_path).ok();
        }
    }
}

#[test]
fn vertical_steps_resize_a_full_screen_terminal() {
    let _families = rune::text::with_families("Inter", "Inter");
    let (mut ws, _host) = tabbed_workspace("single");
    ws.settings.pane_padding = 28.;
    ws.settings.interface.pane_padding_vertical = Some(2.);
    ws.settings.terminal.font.line_height = 20;
    let (sender, _sent) = flume::unbounded();
    let mut terminal = framed_terminal(&mut ws, "block:a", &sender, b"hi");
    terminal.resize(89, 24, 11, 26).unwrap();
    let key = ws.term_key("block:a");
    let t = ws.terminals.get_mut(&key).unwrap();
    {
        let mut model = t.connection.model.lock().unwrap();
        model.owner = true;
        model.frame = Some(std::sync::Arc::new(
            rex_linux::terminal::Renderer::new()
                .unwrap()
                .frame(&terminal)
                .unwrap(),
        ));
    }
    t.refresh();
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    ws.open_section(SettingsSection::Terminal);
    let mut rt = runtime(ws);
    rt.update(|_, tx| tx.scroll_center("pane-padding-y-larger"));
    settle(&mut rt);
    for y in 3..=26 {
        click(&mut rt, "pane-padding-y-larger");
        settle(&mut rt);
        assert_eq!(rt.state.settings.padding(), (28., y as f32));
        assert_eq!(
            rect(&rt, "grid:block:a"),
            [28., 52. + y as f32, 968., 648. - 2. * y as f32]
        );
        assert_eq!(rt.state.terminals[&key].rect, rect(&rt, "grid:block:a"));
    }
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn asymmetric_grid_mouse_wheel_cursor_links_selection_and_find_agree() {
    let _families = rune::text::with_families("Inter", "Inter");
    let (mut ws, host) = tabbed_workspace("single");
    ws.settings.pane_padding = 24.;
    ws.settings.interface.pane_padding_vertical = Some(2.);
    ws.settings.terminal.font.line_height = 20;
    ws.settings.terminal.font.spacing = 20;
    ws.settings.terminal.font.spacing_unit = SpacingUnit::Percent;
    let (sender, sent) = flume::unbounded();
    framed_terminal(&mut ws, "block:a", &sender,
        b"\x1b]8;;https://example.org\x1b\\link\x1b]8;;\x1b\\\r\nsecond\r\nthird\x1b[?1000h\x1b[?1006h");
    let mut rt = runtime(ws);
    settle(&mut rt);
    let grid = rect(&rt, "grid:block:a");
    assert_eq!(grid, [24., 54., 976., 644.]);
    let cell = rt.state.metrics().cell;
    assert_eq!(cell.1, 26.);
    let key = rt.state.term_key("block:a");
    let t = &rt.state.terminals[&key];
    assert_eq!(t.index(2.5 * cell.0, 2.5 * cell.1, cell), 82);
    assert_eq!(
        t.link_at(2.5 * cell.0, 0.5 * cell.1, cell).unwrap().1.uri,
        "https://example.org/"
    );
    let cursor = rect(&rt, "gblock:a/cursor");
    // Rune quantizes drawn text/cursor geometry to 1/64 pt; input uses the exact cell.
    for (actual, expected) in
        cursor
            .into_iter()
            .zip([grid[0] + 5. * cell.0, grid[1] + 2. * cell.1, cell.0, cell.1])
    {
        assert!((actual - expected).abs() < 1. / 64.);
    }
    let (x, y) = (grid[0] + 20.5 * cell.0, grid[1] + 2.5 * cell.1);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    settle(&mut rt);
    let events: Vec<_> = sent.try_iter().collect();
    assert_eq!(events.len(), 2);
    for (event, action) in events.iter().zip(["press", "release"]) {
        assert!(
            matches!(event, HostCommand::Terminal { command: Command::Mouse(p), .. }
            if p["action"] == action && p["x"] == 20 && p["y"] == 2)
        );
    }
    rt.wheel_event(rune::WheelEvent::new(0., -cell.1).at(x, y));
    settle(&mut rt);
    let events: Vec<_> = sent.try_iter().collect();
    assert_eq!(events.len(), 1);
    assert!(
        matches!(&events[0], HostCommand::Terminal { command: Command::Wheel(p), .. }
        if p["x"] == 20 && p["y"] == 2)
    );
    rt.update(|s, _| {
        let t = s.terminals.get_mut(&key).unwrap();
        std::sync::Arc::make_mut(t.frame.as_mut().unwrap()).mouse_tracking = false;
    });
    settle(&mut rt);
    let start = (grid[0] + 2.1 * cell.0, grid[1] + 1.5 * cell.1);
    let end = (grid[0] + 6.8 * cell.0, start.1);
    rt.pointer_down(start.0, start.1);
    rt.pointer_move(end.0 - 2., end.1);
    rt.pointer_move(end.0, end.1);
    rt.pointer_up(end.0, end.1);
    settle(&mut rt);
    let events: Vec<_> = sent
        .try_iter()
        .map(|c| match c {
            HostCommand::Terminal {
                command: Command::Selection(input),
                ..
            } => input.at,
            _ => panic!("selection must send no program input or ownership request"),
        })
        .collect();
    assert_eq!(events.first(), Some(&(2, 1)));
    assert_eq!(events.last(), Some(&(6, 1)));
    // Every corner, especially the bottom, must use the vertical inset, not horizontal.
    rt.update(|s, _| {
        s.terminals.get_mut(&key).unwrap().search = Some(rune::TextEdit::default());
    });
    for corner in [
        find::Corner::TopRight,
        find::Corner::BottomRight,
        find::Corner::BottomLeft,
        find::Corner::TopLeft,
    ] {
        rt.update(|s, _| s.terminals.get_mut(&key).unwrap().find.corner = corner);
        settle(&mut rt);
        let pane = find::Pane {
            size: (grid[2], grid[3]),
            cell,
            lowered: false,
        };
        let expected = pane.apron(corner);
        let apron = rect(&rt, &format!("find-apron:block:a:{corner:?}"));
        for (actual, expected) in apron.into_iter().zip([
            grid[0] + expected[0],
            grid[1] + expected[1],
            expected[2],
            expected[3],
        ]) {
            assert!((actual - expected).abs() < 1. / 64.);
        }
        let expected = pane.bar(corner);
        let bar = rect(&rt, "search:block:a");
        for (actual, expected) in bar.into_iter().zip([
            grid[0] + expected[0],
            grid[1] + expected[1],
            expected[2],
            expected[3],
        ]) {
            assert!((actual - expected).abs() < 0.01, "{corner:?}: {bar:?}");
        }
    }
    assert!(host.is_empty() && rt.state.claimed.is_empty());
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn outline_auto_and_explicit_choices_preserve_terminal_content_and_send_nothing() {
    for (tab, density) in [
        ("single", PaneDensity::Comfortable),
        ("split", PaneDensity::Comfortable),
        ("split", PaneDensity::Compact),
    ] {
        let (mut ws, host) = tabbed_workspace(tab);
        ws.settings.pane_density = density;
        let flat = tab == "single" || density == PaneDensity::Compact;
        let block = if tab == "single" {
            "block:a"
        } else {
            "block:b"
        };
        let (sender, sent) = flume::unbounded();
        framed_terminal(&mut ws, block, &sender, b"\x1b[31mred\x1b[0m unchanged");
        let mut rt = runtime(ws);
        settle(&mut rt);
        let content = |rt: &Runtime<Workspace>| {
            rt.elements()
                .into_iter()
                .filter(|e| e.path.contains(&format!("grid:{block}/")))
                .map(|e| (e.key, e.text, e.rect, e.fill, e.text_color, e.opacity))
                .collect::<Vec<_>>()
        };
        let before = content(&rt);
        assert!(!before.is_empty(), "terminal descendants were inspected");
        for choice in [None, Some(false), Some(true)] {
            rt.update(|s, _| s.settings.interface.highlight_active_pane = choice);
            settle(&mut rt);
            assert_eq!(
                panes::outlines(&rt.state, rt.state.model().window().unwrap(), flat),
                choice.unwrap_or(density == PaneDensity::Comfortable) && tab != "single"
            );
            assert_eq!(content(&rt), before, "outline changes only chrome");
            assert!(sent.is_empty() && host.is_empty() && rt.state.claimed.is_empty());
        }
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn compact_outline_overlays_foreign_background_without_changing_content() {
    let _families = rune::text::with_families("Inter", "Inter");
    let (mut ws, host) = tabbed_workspace("split");
    ws.settings.pane_density = PaneDensity::Compact;
    ws.settings.pane_header = PaneHeader::Hidden;
    ws.term_theme = Some(rex_linux::terminal_theme::TerminalTheme {
        background: Some(0x331122),
        ..Default::default()
    });
    let (sender, sent) = flume::unbounded();
    framed_terminal(&mut ws, "block:b", &sender, b"\x1b]11;#331122\x07unchanged");
    let mut rt = runtime(ws);
    settle(&mut rt);
    let content = |rt: &Runtime<Workspace>| {
        rt.elements()
            .into_iter()
            .filter(|e| e.path.contains("grid:block:b/"))
            .map(|e| (e.key, e.text, e.rect, e.fill, e.text_color, e.opacity))
            .collect::<Vec<_>>()
    };
    let before = content(&rt);
    assert!(!before.is_empty());
    assert_eq!(
        rt.state.terminals[&rt.state.term_key("block:b")].body(&rt.state.term_colors()),
        0x331122
    );
    let grid_before = rect(&rt, "grid:block:b");
    for (outline, drop_hover, visible) in [
        (Some(true), false, true),
        (Some(false), false, false),
        (None, false, false),
        (Some(false), true, true),
    ] {
        rt.update(|s, _| {
            s.settings.interface.highlight_active_pane = outline;
            s.drop_hover = drop_hover.then(|| "block:b".into());
        });
        settle(&mut rt);
        let elements = rt.elements();
        let ring = elements
            .iter()
            .position(|e| !e.exiting && e.key.as_deref() == Some("pane-outline:block:b"));
        assert_eq!(ring.is_some(), visible);
        if let Some(index) = ring {
            assert_eq!(elements[index].rect, rect(&rt, "pane:block:b"));
            assert!(
                elements
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.path.contains("grid:block:b/"))
                    .all(|(i, _)| i < index),
                "outline paints after terminal descendants"
            );
        }
        assert_eq!(rect(&rt, "grid:block:b"), grid_before);
        assert_eq!(content(&rt), before);
        assert!(sent.is_empty() && host.is_empty() && rt.state.claimed.is_empty());
    }
    std::fs::remove_file(&rt.state.settings_path).ok();
}

/// Run alone with REX_TEST_SNAP_DIR; REX_M27_MINIMUM=1 uses 640×480.
#[test]
fn pane_ergonomics_draw() {
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
            "┌──────────┐ ██\r\n│ 世界 é  │\r\n└──────────┘\r\n$ cargo test".as_bytes(),
        );
    }
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
    camera.snap(&mut rt, "panes-default");
    rt.update(|s, _| {
        s.settings.pane_padding = 24.;
        s.settings.interface.pane_padding_vertical = Some(2.);
        s.settings.terminal.font.line_height = 20;
        s.settings.terminal.font.spacing = 20;
        s.settings.terminal.font.spacing_unit = SpacingUnit::Percent;
    });
    settle(&mut rt);
    camera.snap(&mut rt, "panes-padding-24x2-metrics20");
    rt.update(|s, _| s.settings.interface.highlight_active_pane = Some(false));
    settle(&mut rt);
    camera.snap(&mut rt, "panes-outline-off");
    rt.update(|s, _| {
        s.settings.pane_density = PaneDensity::Compact;
        s.settings.interface.highlight_active_pane = Some(true);
    });
    settle(&mut rt);
    camera.snap(&mut rt, "panes-outline-compact");
    rt.update(|s, _| {
        s.term_theme = Some(rex_linux::terminal_theme::TerminalTheme {
            background: Some(0x331122),
            ..Default::default()
        });
        for block in ["block:b", "block:c"] {
            framed_terminal(
                s,
                block,
                &sender,
                b"\x1b]11;#331122\x07$ custom background\r\nunchanged content",
            );
        }
    });
    settle(&mut rt);
    camera.snap(&mut rt, "panes-outline-compact-foreign");
    rt.update(|s, _| s.settings.interface.highlight_active_pane = Some(false));
    settle(&mut rt);
    camera.snap(&mut rt, "panes-outline-compact-foreign-off");
    rt.update(|s, tx| {
        s.settings.interface.highlight_active_pane = Some(true);
        s.perform(Intent::Settings, tx);
    });
    tap(&mut rt, "active-pane-outline");
    camera.snap(&mut rt, "panes-outline-settings");
    press(&mut rt, Key::Escape);
    tap(&mut rt, "settings-section:terminal");
    rt.update(|_, tx| tx.scroll_center("pane-padding-y-larger"));
    settle(&mut rt);
    camera.snap(&mut rt, "panes-padding-settings");
    std::fs::remove_file(&rt.state.settings_path).ok();
}
