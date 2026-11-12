//! Focused M2.7 toolbar recovery tests, adapted from preserved W2 cc8bdad.
use super::pane_verb_tests::{framed_terminal, rect, sent_verbs};
use super::*;
use crate::app::{
    actions,
    toolbar_reveal::{self, Placement},
};
use rex_rune::settings::{
    PaneDensity,
    interface::{InterfaceConfig, ToolbarBehavior},
};
use serde_json::Value;

const TOOLBAR: f32 = 52.;
const MARGIN: f32 = 10.;

fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
}

fn tap(rt: &mut Runtime<Workspace>, key: &str) {
    click(rt, key);
    settle(rt);
}

/// The tab `active` of `tabbed_workspace` under `toolbar`, its panes live terminals that
/// send to the second channel.
fn drawn(
    active: &str,
    toolbar: ToolbarBehavior,
) -> (
    Runtime<Workspace>,
    flume::Receiver<HostCommand>,
    flume::Receiver<HostCommand>,
) {
    let (mut ws, host) = tabbed_workspace(active);
    ws.settings.interface.toolbar = toolbar;
    let (sender, sent) = flume::unbounded();
    for block in ["block:a", "block:b", "block:c"] {
        framed_terminal(&mut ws, block, &sender, b"$ make\r\nok");
    }
    let mut rt = runtime(ws);
    settle(&mut rt);
    (rt, host, sent)
}

/// Rest the pointer at (x, y) long enough for a delayed hover to be heard.
fn rest_at(rt: &mut Runtime<Workspace>, x: f32, y: f32) {
    rt.pointer_move(x, y);
    for _ in 0..30 {
        rt.frame(1. / 60.);
    }
}

/// Ctrl Alt Shift T, the default key of Reveal toolbar.
fn reveal_key(rt: &mut Runtime<Workspace>) {
    rt.key(KeyEvent {
        key: Key::Text("T".into()),
        mods: Mods {
            ctrl: true,
            shift: true,
            alt: true,
            ..Mods::default()
        },
    });
    settle(rt);
}

/// What the app's Commands row for the toolbar reads, and whether it can run.
fn toolbar_command(rt: &mut Runtime<Workspace>) -> (String, bool) {
    rt.update(|s, _| s.picker(false));
    rt.frame(1. / 60.);
    let row = rt
        .state
        .entries()
        .into_iter()
        .find(|e| e.label.ends_with(" toolbar"))
        .map(|e| (e.label, e.enabled))
        .expect("a Commands row");
    press(rt, Key::Escape);
    row
}

/// Presses the row named `label` of the list a popup button opened.
fn choose(rt: &mut Runtime<Workspace>, control: &str, label: &str) {
    tap(rt, control);
    let r = rt
        .elements()
        .into_iter()
        .find(|e| !e.exiting && e.role == "menuitem" && e.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("no {label} row"))
        .rect;
    rt.pointer_down(r[0] + r[2] / 2., r[1] + r[3] / 2.);
    rt.pointer_up(r[0] + r[2] / 2., r[1] + r[3] / 2.);
    settle(rt);
}

/// Settings open on Interface in a window tall enough for the dialog's full height.
fn interface_settings() -> Runtime<Workspace> {
    let (mut ws, _host) = tabbed_workspace("split");
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    let mut rt = runtime_at(ws, (1440., 1000.));
    settle(&mut rt);
    rt
}

fn saved(rt: &Runtime<Workspace>) -> InterfaceConfig {
    relaunch(&rt.state).settings.interface
}

/// Synthetic transfer: no disk or server traffic.
fn add_download(s: &mut Workspace) {
    let mut transfer = rex_rune::download::Transfer::new(
        1,
        "s".into(),
        "block:a".into(),
        "~/build.log".into(),
        std::env::temp_dir(),
        1,
    );
    transfer.name = Some("build.log".into());
    transfer.state = rex_rune::download::State::Receiving {
        received: 42,
        hint: 100,
    };
    s.hosts[0].model.downloads.push(transfer);
}

#[test]
fn hidden_controls_leave_no_hits_and_downloads_stay_usable() {
    let (mut rt, _host, sent) = drawn("single", ToolbarBehavior::Hidden);
    // With no pane header either, the terminal has the toolbar's row.
    rt.update(|s, _| s.settings.pane_header = rex_rune::settings::PaneHeader::Hidden);
    settle(&mut rt);
    for key in [
        "close-app",
        "settings",
        "commands",
        "title-drag",
        "tabs",
        "toggle-sidebar",
    ] {
        assert!(!has(&rt, key), "hidden {key} is absent, not transparent");
    }
    // Where the sidebar button sat; now the terminal owns this point.
    let (x, y) = (25., 26.);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1. / 60.);
    assert!(rt.shell_request.is_none());
    let verbs = sent_verbs(&sent);
    assert!(
        verbs.iter().any(|(_, v)| v.starts_with("selection Down")),
        "{verbs:?}, grid {:?}",
        rect(&rt, "grid:block:a")
    );
    rt.update(|s, _| add_download(s));
    settle(&mut rt);
    assert!(has(&rt, "download-chip"));
    click(&mut rt, "download-chip");
    settle(&mut rt);
    assert!(has(&rt, "downloads-popover"));
    press(&mut rt, Key::Escape);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

/// Real fonts and actual viewport-sized images. Run this test alone with REX_TEST_SNAP_DIR.
#[test]
fn toolbar_draws_at_normal_minimum_and_zoomed_sizes() {
    let Some(dir) = std::env::var_os("REX_TEST_SNAP_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let gpu = rune::render::Gpu::headless();
    for (name, scale, size) in [
        ("normal", 1., (1024., 700.)),
        ("minimum", 1., (640., 480.)),
        ("zoomed", 1.3, (1024., 700.)),
    ] {
        let fonts = crate::app::fonts::load("JetBrainsMono Nerd Font", &[]);
        let text = rune::text::TextSystem::new(fonts.files, fonts.families);
        let (mut ws, _host) = tabbed_workspace("split");
        ws.settings.ui_scale = scale;
        let (sender, _sent) = flume::unbounded();
        framed_terminal(
            &mut ws,
            "block:b",
            &sender,
            b"$ cargo test\r\nrunning 12 tests\r\ntest result: ok",
        );
        framed_terminal(
            &mut ws,
            "block:c",
            &sender,
            b"$ tail -f server.log\r\nlistening on :8080",
        );
        let mut rt = Runtime::with_text(ws, view, size, Env::new(true, 2.), text);
        rt.on_key = Some(crate::app::input::on_key);
        let mut painter = rune::render::Renderer::new(&gpu.device, rune::render::OFFSCREEN_FORMAT);
        settle(&mut rt);
        for capture in [
            "always",
            "hidden",
            "revealed",
            "downloads",
            "settings",
            "choices",
            "shortcuts",
        ] {
            match capture {
                "hidden" => rt.update(|s, tx| s.set_toolbar(ToolbarBehavior::Hidden, tx)),
                "revealed" => rt.update(|s, _| s.toggle_toolbar()),
                "downloads" => rt.update(|s, _| {
                    s.toggle_toolbar();
                    add_download(s);
                }),
                "settings" => rt.update(|s, tx| s.perform(Intent::Settings, tx)),
                "choices" => click(&mut rt, "toolbar-behavior"),
                "shortcuts" => {
                    press(&mut rt, Key::Escape);
                    click(&mut rt, "settings-section:shortcuts");
                    rt.update(|s, _| s.shortcut_filter = rune::TextEdit::new("Reveal toolbar"));
                }
                _ => {}
            }
            settle(&mut rt);
            if has(&rt, "toolbar") {
                let close = rect(&rt, "close-app");
                let settings = rect(&rt, "settings");
                let drag = rect(&rt, "title-drag");
                assert!(close[0] + close[2] <= size.0 + 0.5, "{name}: {close:?}");
                assert!(settings[0] + settings[2] <= close[0]);
                assert!(drag[2] >= 48. * rt.state.ui_scale() - 0.5);
            }
            if capture == "settings" || capture == "choices" {
                let dialog = rect(&rt, "dialog");
                assert!(dialog[1] >= 0. && dialog[1] + dialog[3] <= size.1 + 0.5);
                assert!(has(&rt, "close-settings"));
            }
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
                &dir.join(format!("m27-{name}-{capture}.png")),
                w,
                h,
                &target.read(&gpu.device),
            )
            .unwrap();
        }
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn always_keeps_the_toolbar_in_its_row_above_the_panes() {
    let (rt, _host, _sent) = drawn("single", ToolbarBehavior::Always);
    assert_eq!(toolbar_reveal::placement(&rt.state), Placement::Row);
    assert_eq!(rect(&rt, "toolbar"), [0., 0., 1024., TOOLBAR]);
    assert_eq!(rt.state.area, [0., TOOLBAR, 1024., 700. - TOOLBAR]);
    assert!(!has(&rt, "toolbar-band") && !has(&rt, "toolbar-float"));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_lone_panes_header_stays_clear_of_the_top_edge_when_the_toolbar_has_no_row() {
    const BUTTONS: [&str; 4] = [
        "pane-split-right",
        "pane-split-down",
        "pane-zoom",
        "pane-close",
    ];
    for toolbar in [ToolbarBehavior::OnHover, ToolbarBehavior::Hidden] {
        let (mut rt, _host, sent) = drawn("single", toolbar);
        // The default: a lone pane has its header, and here it is the top of the window.
        assert_eq!(rect(&rt, "pane:block:a"), [0., 0., 1024., 700.]);
        assert_eq!(
            rect(&rt, "pane-header:block:a"),
            [0., toolbar_reveal::EDGE, 1024., 30.],
            "{toolbar:?}: under the edge that brings the toolbar out"
        );
        for name in BUTTONS {
            let r = rect(&rt, &format!("{name}:block:a"));
            assert!(r[1] >= toolbar_reveal::EDGE, "{toolbar:?} {name}: {r:?}");
        }
        // Resting on the pane's Close brings no toolbar out over it, and a press there
        // asks about the pane: it never reaches Close app, which a floating toolbar
        // would put at this very point.
        let close = rect(&rt, "pane-close:block:a");
        let (x, y) = (close[0] + 12., close[1] + 12.);
        rest_at(&mut rt, x, y);
        assert!(
            !has(&rt, "toolbar") && !has(&rt, "toolbar-float"),
            "{toolbar:?}"
        );
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        settle(&mut rt);
        assert!(rt.shell_request.is_none(), "{toolbar:?}: the window stays");
        assert!(
            matches!(
                rt.state.panel.as_ref().map(|p| &p.kind),
                Some(PanelKind::Confirm { method, .. }) if method == "block.close"
            ),
            "{toolbar:?}: Close pane's confirmation"
        );
        press(&mut rt, Key::Escape);
        settle(&mut rt);
        // Where the sidebar button sat is the header now: a press there is the header's
        // (it focuses the pane), not a selection in the terminal and not a window action.
        let _ = sent_verbs(&sent);
        rt.pointer_down(25., 26.);
        rt.pointer_up(25., 26.);
        rt.frame(1. / 60.);
        assert!(rt.shell_request.is_none());
        assert!(
            !sent_verbs(&sent)
                .iter()
                .any(|(_, v)| v.starts_with("selection")),
            "{toolbar:?}"
        );
        if toolbar == ToolbarBehavior::OnHover {
            // The edge itself still works, above the header.
            rest_at(&mut rt, x, 3.);
            assert!(has(&rt, "toolbar"), "the toolbar comes out from its edge");
        }
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
    // With the toolbar in its own row there is nothing to keep clear of.
    let (rt, _host, _sent) = drawn("single", ToolbarBehavior::Always);
    assert_eq!(rect(&rt, "pane-header:block:a")[1], rt.state.area[1]);
    assert!(!has(&rt, "pane-header-clear"));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn hidden_gives_the_toolbars_row_to_the_panes() {
    let (mut rt, _host, _sent) = drawn("single", ToolbarBehavior::Hidden);
    assert_eq!(toolbar_reveal::placement(&rt.state), Placement::None);
    assert!(!has(&rt, "toolbar") && !has(&rt, "toolbar-band"));
    assert_eq!(rt.state.area, [0., 0., 1024., 700.], "all 52 pt of it");
    assert_eq!(rect(&rt, "pane:block:a"), [0., 0., 1024., 700.]);
    assert_eq!(
        rect(&rt, "grid:block:a")[1],
        toolbar_reveal::EDGE + crate::app::theme::CARD_HEADER + 8.,
        "under the edge kept clear, the pane's header and its own padding"
    );
    // Resting on the top edge brings nothing out.
    rest_at(&mut rt, 300., 3.);
    assert!(!has(&rt, "toolbar"));
    std::fs::remove_file(&rt.state.settings_path).ok();

    // Cards and the sidebar keep the outer margin from the window's top edge.
    let (mut ws, _host) = tabbed_workspace("split");
    ws.settings.interface.toolbar = ToolbarBehavior::Hidden;
    ws.sidebar.toggle();
    let mut rt = runtime(ws);
    settle(&mut rt);
    let (left, right) = (rect(&rt, "pane:block:b"), rect(&rt, "pane:block:c"));
    assert_eq!((left[1], right[1]), (MARGIN, MARGIN));
    assert_eq!(left[1] + left[3], 700. - MARGIN, "and from its bottom edge");
    assert_eq!(rect(&rt, "sidebar")[1], 0.);
    let first = rt
        .elements()
        .into_iter()
        .filter(|e| !e.exiting && e.text.is_some())
        .map(|e| e.rect[1])
        .fold(f32::MAX, f32::min);
    assert!(
        first >= MARGIN,
        "nothing is drawn against the edge: {first}"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();

    // Compact's flush panes go edge to edge.
    let (mut ws, _host) = tabbed_workspace("split");
    ws.settings.interface.toolbar = ToolbarBehavior::Hidden;
    ws.settings.pane_density = PaneDensity::Compact;
    let mut rt = runtime(ws);
    settle(&mut rt);
    assert_eq!(rect(&rt, "pane:block:b")[1], 0.);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn on_hover_floats_the_toolbar_over_the_top_edge_without_moving_a_terminal() {
    let (mut rt, host, sent) = drawn("single", ToolbarBehavior::OnHover);
    assert!(!has(&rt, "toolbar"));
    assert_eq!(rt.state.area, [0., 0., 1024., 700.]);
    assert_eq!(rect(&rt, "toolbar-band"), [0., 0., 1024., 8.]);
    let (pane, grid) = (rect(&rt, "pane:block:a"), rect(&rt, "grid:block:a"));
    let x = 300.;

    // Below the edge: nothing.
    rest_at(&mut rt, x, 40.);
    assert!(!has(&rt, "toolbar"));
    // A pass over the edge on the way elsewhere: nothing either.
    rt.pointer_move(x, 4.);
    rt.frame(1. / 60.);
    rt.pointer_move(x, 90.);
    settle(&mut rt);
    assert!(!has(&rt, "toolbar"), "too quick to mean it");

    // Resting on the top 8 pt brings it out, within 150 ms.
    rt.pointer_move(x, 4.);
    for _ in 0..9 {
        rt.frame(1. / 60.);
    }
    assert!(has(&rt, "toolbar"), "revealed");
    assert_eq!(toolbar_reveal::placement(&rt.state), Placement::Floating);
    settle(&mut rt);
    assert_eq!(rect(&rt, "toolbar-float"), [0., 0., 1024., TOOLBAR]);
    assert_eq!(rect(&rt, "toolbar"), [0., 0., 1024., TOOLBAR]);
    assert_eq!(rect(&rt, "pane:block:a"), pane, "the pane keeps its place");
    assert_eq!(rect(&rt, "grid:block:a"), grid, "and the terminal its rows");
    assert_eq!(rt.state.area, [0., 0., 1024., 700.]);
    assert!(sent_verbs(&sent).is_empty() && requests(&host).is_empty());

    // On the toolbar itself it stays, and its controls work.
    assert_eq!(rect(&rt, "toolbar-band")[3], TOOLBAR);
    let settings = rect(&rt, "settings");
    rest_at(&mut rt, settings[0] + 10., settings[1] + 26.);
    assert!(has(&rt, "toolbar"));
    click(&mut rt, "settings");
    assert!(matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::Settings)
    ));
    settle(&mut rt);
    let dialog = rect(&rt, "dialog");
    rest_at(&mut rt, 500., 400.);
    assert!(!rt.state.toolbar_reveal.hover);
    assert_eq!(
        rect(&rt, "dialog"),
        dialog,
        "leaving the toolbar never moves Settings"
    );
    click(&mut rt, "close-settings");
    assert!(rt.state.panel.is_none());

    // Leaving it puts it away.
    rt.pointer_move(x, 200.);
    settle(&mut rt);
    assert!(!has(&rt, "toolbar"), "gone again");
    assert!(!rt.state.toolbar_reveal.hover);
    assert_eq!(rect(&rt, "toolbar-band")[3], 8.);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_hover_edge_leaves_presses_to_the_terminal() {
    let (mut ws, _host) = tabbed_workspace("single");
    ws.settings.interface.toolbar = ToolbarBehavior::OnHover;
    // No padding and no pane header: the terminal's first row is under the edge.
    ws.settings.pane_padding = 0.;
    ws.settings.pane_header = rex_rune::settings::PaneHeader::Hidden;
    let (sender, sent) = flume::unbounded();
    framed_terminal(&mut ws, "block:a", &sender, b"first line\r\nsecond");
    let mut rt = runtime(ws);
    settle(&mut rt);
    assert_eq!(rect(&rt, "grid:block:a")[1], 0.);
    // Inside the edge, under the 6 pt that resize the window.
    rt.pointer_move(40., 7.);
    rt.pointer_down(40., 7.);
    rt.pointer_up(40., 7.);
    rt.frame(1. / 60.);
    assert!(
        sent_verbs(&sent)
            .iter()
            .any(|(block, verb)| block == "block:a" && verb.starts_with("selection Down")),
        "the terminal took the press"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn hover_revealed_toolbar_keeps_tab_session_and_window_context_menus() {
    let (mut rt, host, sent) = drawn("split", ToolbarBehavior::OnHover);
    for (key, kind) in [
        ("tab:single", PanelKind::Window),
        ("identity", PanelKind::Session),
    ] {
        rest_at(&mut rt, 300., 200.);
        rest_at(&mut rt, 300., 4.);
        settle(&mut rt);
        assert!(has(&rt, "toolbar"));
        let r = rect(&rt, key);
        let (x, y) = (r[0] + 20., r[1] + r[3] / 2.);
        rest_at(&mut rt, x, y);
        rt.secondary_down(x, y);
        rt.frame(1. / 60.);
        rt.secondary_up(x, y);
        settle(&mut rt);
        let panel = rt.state.panel.as_ref().expect("context menu opens");
        assert_eq!(
            std::mem::discriminant(&panel.kind),
            std::mem::discriminant(&kind),
            "context menu for {key}"
        );
        press(&mut rt, Key::Escape);
        settle(&mut rt);
    }

    rest_at(&mut rt, 300., 200.);
    rest_at(&mut rt, 300., 4.);
    settle(&mut rt);
    let r = rect(&rt, "title-drag");
    let (x, y) = (r[0] + r[2] / 2., r[1] + r[3] / 2.);
    rest_at(&mut rt, x, y);
    rt.events.clear();
    rt.secondary_down(x, y);
    rt.frame(1. / 60.);
    rt.secondary_up(x, y);
    settle(&mut rt);
    // Rune keeps the native menu request private. Its input trace confirms dispatch
    // to the toolbar handler that calls Tx::show_window_menu, not the hover band.
    assert!(
        rt.events
            .iter()
            .any(|event| event.starts_with("secondary press ") && event.ends_with("/toolbar")),
        "window-menu handler reached: {:?}",
        rt.events
    );
    assert!(sent_verbs(&sent).is_empty() && requests(&host).is_empty());
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_hover_edge_keeps_the_terminals_context_menu_before_reveal() {
    let (mut ws, _host) = tabbed_workspace("single");
    ws.settings.interface.toolbar = ToolbarBehavior::OnHover;
    ws.settings.pane_padding = 0.;
    let (sender, _sent) = flume::unbounded();
    framed_terminal(&mut ws, "block:a", &sender, b"first line\r\nsecond");
    let mut rt = runtime(ws);
    settle(&mut rt);
    // Below the native 6 pt resize band, within the unrevealed 8 pt hover band.
    assert!(!has(&rt, "toolbar"));
    rt.secondary_down(300., 7.);
    rt.frame(1. / 60.);
    rt.secondary_up(300., 7.);
    settle(&mut rt);
    assert!(matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::Pane)
    ));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn reveal_toolbar_floats_it_from_the_keyboard_to_move_or_close_the_window() {
    let spec = actions::find("client.toolbar.toggle").expect("registered");
    assert_eq!(spec.title(), "Reveal toolbar");
    assert_eq!(spec.category(), "Interface");
    assert!(matches!(spec.remote(), actions::Remote::Never(_)));
    assert!(spec.accepts(&Value::Null) && !spec.accepts(&serde_json::json!({"x": 1})));
    assert!(!spec.repeats() && !spec.global());
    let keys: Vec<_> = rex_rune::keymap::defaults()
        .bindings
        .into_iter()
        .filter(|b| b.action == "client.toolbar.toggle")
        .map(|b| b.keys)
        .collect();
    assert_eq!(keys, [["ctrl+alt+shift+t"]], "one default key");

    for toolbar in [ToolbarBehavior::Hidden, ToolbarBehavior::OnHover] {
        let (mut rt, host, sent) = drawn("single", toolbar);
        assert_eq!(
            rt.state
                .shortcut_detail("client.toolbar.toggle", &Value::Null),
            "Ctrl Alt Shift T"
        );
        let (area, grid) = (rt.state.area, rect(&rt, "grid:block:a"));
        let before = std::fs::read(&rt.state.settings_path).ok();
        assert_eq!(
            toolbar_command(&mut rt),
            ("Reveal toolbar".into(), true),
            "{toolbar:?}"
        );

        reveal_key(&mut rt);
        assert!(rt.state.toolbar_reveal.pinned, "{toolbar:?}");
        assert_eq!(rect(&rt, "toolbar-float"), [0., 0., 1024., TOOLBAR]);
        assert_eq!(rt.state.area, area, "{toolbar:?}: the panes keep the row");
        assert_eq!(rect(&rt, "grid:block:a"), grid, "{toolbar:?}: same rows");
        assert_eq!(toolbar_command(&mut rt), ("Hide toolbar".into(), true));
        // It stays with the pointer anywhere, until it is run again.
        rest_at(&mut rt, 300., 400.);
        assert!(has(&rt, "toolbar"));
        // The window's drag space and Close are in it.
        let drag = rect(&rt, "title-drag");
        assert!(drag[2] >= 48. && drag[3] == TOOLBAR, "{drag:?}");
        assert!(rt.shell_request.is_none());
        click(&mut rt, "close-app");
        assert!(rt.shell_request.take().is_some(), "Close closes the window");

        reveal_key(&mut rt);
        assert!(!has(&rt, "toolbar"), "{toolbar:?}: put back");
        assert!(!rt.state.toolbar_reveal.pinned);
        assert!(sent_verbs(&sent).is_empty() && requests(&host).is_empty());
        assert_eq!(
            std::fs::read(&rt.state.settings_path).ok(),
            before,
            "nothing was saved"
        );
        std::fs::remove_file(&rt.state.settings_path).ok();
    }

    // Always has nothing to bring out; its key is still the app's, so it reaches no program.
    let (mut rt, _host, sent) = drawn("single", ToolbarBehavior::Always);
    reveal_key(&mut rt);
    assert!(!rt.state.toolbar_reveal.pinned);
    assert_eq!(toolbar_reveal::placement(&rt.state), Placement::Row);
    assert_eq!(toolbar_command(&mut rt), ("Reveal toolbar".into(), false));
    assert!(sent.try_iter().next().is_none());
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_waiting_key_sequence_still_shows_with_no_toolbar_to_hold_it() {
    use rune::{PhysicalInput, PhysicalKeyCode as Code};
    let (mut ws, _host) = tabbed_workspace("single");
    ws.settings.interface.toolbar = ToolbarBehavior::Hidden;
    ws.settings.shortcut_file.overrides = rex_rune::keymap::Overrides::parse(
        &serde_json::json!([{"keys":"ctrl+b>c","action":"client.tab.new"}]),
    );
    ws.local_keymap = actions::resolve(&ws.settings.shortcut_file.overrides);
    ws.keymap_generation += 1;
    let mut rt = runtime(ws);
    assert!(!has(&rt, "pending-sequence"));
    let mods = Mods {
        ctrl: true,
        ..Mods::default()
    };
    rt.physical_key_input(
        PhysicalInput::Key {
            key: Code::KeyB,
            mods,
            pressed: true,
            repeat: false,
        },
        KeyEvent {
            key: Key::Command('b'),
            mods,
        },
        None,
    );
    settle(&mut rt);
    assert_eq!(rt.state.bindings.sequence.depth, 1);
    assert!(!has(&rt, "toolbar"));
    let chip = rect(&rt, "pending-sequence-float");
    assert_eq!((chip[0], chip[1]), (MARGIN, 8.));
    assert!(has(&rt, "pending-sequence"));
    press(&mut rt, Key::Escape);
    settle(&mut rt);
    assert!(!has(&rt, "pending-sequence"));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_toolbar_setting_saves_each_choice_and_a_relaunch_keeps_it() {
    let mut rt = interface_settings();
    let label = |rt: &Runtime<Workspace>| {
        rt.elements()
            .into_iter()
            .find(|e| !e.exiting && e.key.as_deref() == Some("toolbar-behavior"))
            .and_then(|e| e.label)
    };
    assert_eq!(label(&rt).as_deref(), Some("Always show"));
    let note = element_text(&rt, "toolbar-reveal-note").join(" ");
    assert!(
        note.contains("Reveal toolbar (Ctrl Alt Shift T, and in Commands)"),
        "the way back is named from the keymap: {note}"
    );
    for (choice, toolbar) in [
        ("Hidden", ToolbarBehavior::Hidden),
        ("Show on hover", ToolbarBehavior::OnHover),
        ("Always show", ToolbarBehavior::Always),
    ] {
        choose(&mut rt, "toolbar-behavior", choice);
        assert_eq!(rt.state.settings.interface.toolbar, toolbar);
        assert_eq!(saved(&rt).toolbar, toolbar);
        assert_eq!(label(&rt).as_deref(), Some(choice));
        assert!(
            matches!(
                rt.state.panel.as_ref().map(|p| &p.kind),
                Some(PanelKind::Settings)
            ),
            "Settings stays open"
        );
        assert_eq!(has(&rt, "toolbar"), toolbar == ToolbarBehavior::Always);
        let dialog = rect(&rt, "dialog");
        assert!(
            dialog[1] >= toolbar_reveal::row_height(&rt.state) && dialog[1] + dialog[3] <= 1000.,
            "{choice}: the dialog is whole, clear of the toolbar's row: {dialog:?}"
        );
    }
    // A choice puts away a toolbar that was revealed by hand.
    choose(&mut rt, "toolbar-behavior", "Hidden");
    rt.update(|s, _| s.toggle_toolbar());
    assert!(rt.state.toolbar_reveal.pinned);
    choose(&mut rt, "toolbar-behavior", "Show on hover");
    assert!(!rt.state.toolbar_reveal.pinned);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn notices_keep_to_the_corner_whether_or_not_the_toolbar_is_there() {
    for (toolbar, top) in [
        (ToolbarBehavior::Always, TOOLBAR + 4.),
        (ToolbarBehavior::Hidden, MARGIN),
    ] {
        let (mut rt, _host, _sent) = drawn("single", toolbar);
        rt.update(|s, _| s.inform("Copied"));
        settle(&mut rt);
        assert_eq!(rect(&rt, "toasts")[1], top, "{toolbar:?}");
        if toolbar == ToolbarBehavior::Hidden {
            rt.update(|s, _| s.toggle_toolbar());
            settle(&mut rt);
            assert_eq!(rect(&rt, "toasts")[1], TOOLBAR + 4., "under a floating one");
        }
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}
