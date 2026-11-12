//! The Activity entry point shares the list's inventory, never its read receipts.
use super::activity_list_tests::fixture;
use super::pane_verb_tests::{framed_terminal, rect};
use super::*;
use crate::app::{
    actions,
    activity_list::ACTION,
    notify::NoteTarget,
    theme::{danger, warning},
};
use rex_rune::{
    connection::activity::{BlockedKind, ProgramState, StatusRecord, TerminalProgress},
    settings::interface::ToolbarBehavior,
};
use serde_json::json;

fn state(ws: &mut Workspace, host: usize, block: &str, state: ProgramState) {
    ws.hosts[host].model.activity.insert(
        block.into(),
        [(
            String::new(),
            StatusRecord {
                state,
                live: true,
                ..Default::default()
            },
        )]
        .into(),
    );
}
fn label(ws: &Workspace) -> Option<String> {
    ws.activity_chip().map(|s| s.label())
}
fn settle(rt: &mut Runtime<Workspace>) {
    // Fixed frame budget, including the production active-tab reveal path.
    for _ in 0..6 {
        rt.update(|s, tx| {
            crate::app::poll(s, 0., tx);
        });
        for _ in 0..30 {
            rt.frame(1. / 60.);
        }
    }
}
fn wake(rt: &mut Runtime<Workspace>, f: impl FnOnce(&mut Workspace)) {
    rt.update(|s, tx| {
        f(s);
        for host in 0..s.hosts.len() {
            *s.hosts[host].connection.model.lock().unwrap() = s.hosts[host].model.clone();
            s.sync(host, tx);
        }
    });
    settle(rt);
}
fn bind(ws: &mut Workspace) {
    ws.settings.shortcut_file.overrides = rex_rune::keymap::Overrides::parse(&json!([
        {"keys":"ctrl+alt+a","action":ACTION},
        {"keys":"ctrl+b>c","action":"client.tab.new"}
    ]));
    ws.local_keymap = actions::resolve(&ws.settings.shortcut_file.overrides);
    ws.keymap_generation += 1;
}
fn prefix(rt: &mut Runtime<Workspace>) {
    let mods = Mods {
        ctrl: true,
        ..Mods::default()
    };
    rt.physical_key_input(
        rune::PhysicalInput::Key {
            key: rune::PhysicalKeyCode::KeyB,
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
    settle(rt);
}
fn download(ws: &mut Workspace) {
    let mut t = rex_rune::download::Transfer::new(
        1,
        "s".into(),
        "front".into(),
        "~/build.log".into(),
        std::env::temp_dir(),
        1,
    );
    t.name = Some("build.log".into());
    t.state = rex_rune::download::State::Receiving {
        received: 42,
        hint: 100,
    };
    ws.hosts[0].model.downloads.push(t);
}

#[test]
fn inventory_states_priority_words_and_host_session_identity() {
    let (mut ws, _rx) = fixture();
    assert_eq!(label(&ws).as_deref(), Some("2 need you"));
    assert_eq!(ws.activity_chip().unwrap().tone(), warning());
    // Same tab and session ids on the other host remain distinct.
    state(&mut ws, 0, "front", ProgramState::Blocked);
    assert_eq!(
        label(&ws).as_deref(),
        Some("2 need you"),
        "on-screen never counts"
    );
    ws.window_focused = false;
    assert_eq!(
        label(&ws).as_deref(),
        Some("2 need you"),
        "not a focus/read receipt"
    );
    state(&mut ws, 1, "front", ProgramState::Blocked);
    state(&mut ws, 1, "build", ProgramState::Blocked);
    assert_eq!(label(&ws).as_deref(), Some("4 need you"));
    assert_eq!(
        ws.activity_entries("")
            .iter()
            .filter(|e| e.group == "Needs you")
            .count(),
        5
    );
    for kind in [
        None,
        Some(BlockedKind::Auth),
        Some(BlockedKind::Question),
        Some(BlockedKind::Permission),
    ] {
        ws.hosts[1]
            .model
            .activity
            .get_mut("build")
            .unwrap()
            .get_mut("")
            .unwrap()
            .kind = kind;
        assert_eq!(label(&ws).as_deref(), Some("4 need you"));
    }
    ws.hosts[1].model.connected = false;
    assert_eq!(label(&ws).as_deref(), Some("1 needs you"));
    ws.settings.muted_tabs.push(rex_rune::settings::MutedTab {
        endpoint: ws.settings.hosts[0].endpoint.clone(),
        session: "s".into(),
        window: "blocked".into(),
    });
    assert_eq!(label(&ws).as_deref(), Some("1 error"));
    assert_eq!(ws.activity_chip().unwrap().tone(), danger());
    state(&mut ws, 0, "build", ProgramState::Error);
    assert_eq!(label(&ws).as_deref(), Some("2 errors"));
    ws.settings.notifications.show_in_tabs = false;
    assert!(ws.activity_chip().is_none());
    assert!(
        ws.activity_entries("").iter().any(|e| e.group == "Error"),
        "list policy unchanged"
    );
    ws.settings.notifications.show_in_tabs = true;
    for s in [
        ProgramState::Idle,
        ProgramState::Working,
        ProgramState::Done,
    ] {
        state(&mut ws, 0, "build", s);
        state(&mut ws, 0, "error", s);
        assert!(
            ws.activity_chip().is_none(),
            "no Working, Done or Bell chip"
        );
    }
    ws.hosts[0]
        .model
        .progress
        .insert("build".into(), TerminalProgress::Error(None));
    assert!(
        ws.activity_chip().is_none(),
        "OSC 9;4 alone is not listed activity"
    );
    // A layout absent from the session inventory is not something the list can list.
    let extra = ws.hosts[0].model.views["other"].clone();
    ws.hosts[0].model.views.insert("not-listed".into(), extra);
    state(&mut ws, 0, "build", ProgramState::Blocked);
    assert_eq!(label(&ws).as_deref(), Some("1 needs you"));
    ws.hosts[0].model.views.remove("other");
    assert!(ws.activity_chip().is_none());
}

#[test]
fn live_chip_click_is_local_preserves_latches_notes_focus_and_pane_geometry() {
    let _families = rune::text::with_families("Inter", "Inter");
    let (mut ws, hosts) = fixture();
    for h in &mut ws.hosts {
        h.model.activity.clear();
    }
    let (sender, terminal) = flume::unbounded();
    framed_terminal(&mut ws, "front", &sender, b"$ ");
    let connection = ws.terminals[&ws.term_key("front")].connection.clone();
    ws.hosts[0]
        .model
        .terminals
        .insert("front".into(), connection);
    let note = NoteTarget {
        host: ws.hosts[1].id,
        session: "s".into(),
        window: Some("done".into()),
    };
    ws.notified_tabs.insert(note.clone());
    let mut rt = runtime(ws);
    wake(&mut rt, |_| {});
    let pane = rect(&rt, "pane:front");
    let grid = rect(&rt, "grid:front");
    for h in &hosts {
        h.drain();
    }
    terminal.drain();
    let done = rt.state.unseen_done.clone();
    let bells = rt.state.unseen_bells.clone();
    assert!(!has(&rt, "activity-chip"));
    wake(&mut rt, |s| state(s, 1, "build", ProgramState::Blocked));
    assert!(has(&rt, "activity-chip"));
    assert_eq!(rect(&rt, "pane:front"), pane);
    assert_eq!(rect(&rt, "grid:front"), grid);
    let chip = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("activity-chip"))
        .unwrap();
    assert_eq!(chip.role, "button");
    assert_eq!(chip.label.as_deref(), Some("Activity: 1 needs you"));
    assert_eq!(chip.help.as_deref(), Some("1 needs you · Show activity"));
    click(&mut rt, "activity-chip");
    settle(&mut rt);
    assert!(matches!(
        rt.state.panel.as_ref().unwrap().kind,
        PanelKind::Activity
    ));
    assert_eq!(rect(&rt, "pane:front"), pane);
    assert_eq!(rect(&rt, "grid:front"), grid);
    let chip_focus = focused(&rt);
    assert!(chip_focus.is_some());
    press(&mut rt, Key::Text("b".into()));
    assert_eq!(
        rt.state.query(),
        "b",
        "typing immediately filters after a chip click"
    );
    press(&mut rt, Key::Escape);
    rt.update(|s, tx| {
        actions::run(s, ACTION, &serde_json::Value::Null, tx);
    });
    settle(&mut rt);
    assert_eq!(focused(&rt), chip_focus, "same field as Commands/action");
    press(&mut rt, Key::Escape);
    assert!(focused(&rt).is_none());
    wake(&mut rt, |s| state(s, 1, "error", ProgramState::Blocked));
    assert_eq!(label(&rt.state).as_deref(), Some("2 need you"));
    wake(&mut rt, |s| {
        state(s, 1, "build", ProgramState::Working);
        state(s, 1, "error", ProgramState::Working);
    });
    assert!(!has(&rt, "activity-chip"));
    assert_eq!(rt.state.unseen_done, done);
    assert_eq!(rt.state.unseen_bells, bells);
    assert!(rt.state.notified_tabs.contains(&note));
    assert_eq!(rect(&rt, "pane:front"), pane);
    assert_eq!(rect(&rt, "grid:front"), grid);
    for h in &hosts {
        assert!(
            h.is_empty(),
            "no host commands across appearance/click/disappearance"
        );
    }
    assert!(terminal.is_empty(), "no PTY input, resize or claim");
}

/// Close, the actions and the drag space stay on screen; with tabs in the toolbar the strip
/// keeps a floor and the active tab stays in view.
fn room(rt: &Runtime<Workspace>, width: f32, sidebar: bool, what: &str) {
    let drag = rect(rt, "title-drag");
    assert!(drag[2] >= 48., "{what}: drag {drag:?}");
    let close = rect(rt, "close-app");
    assert!(
        close[0] + close[2] <= width + 0.5,
        "{what}: close {close:?}"
    );
    let commands = rect(rt, "commands");
    assert!(drag[0] + drag[2] <= commands[0] + 0.5, "{what}");
    if !sidebar {
        let strip = rect(rt, "tabs");
        let active = rect(rt, "tab:idle");
        assert!(strip[2] >= 40., "{what}: strip {strip:?}");
        assert!(
            active[0] + 40. <= strip[0] + strip[2] && active[0] + active[2] >= strip[0] + 40.,
            "{what}: active {active:?}, strip {strip:?}"
        );
    }
}

#[test]
fn the_chip_has_one_label_and_waits_for_briefer_chips_in_a_narrow_window() {
    let _families = rune::text::with_families("Inter", "Inter");
    for width in [1024., 800.] {
        let narrow = width < 1000.;
        for sidebar in [false, true] {
            let what = format!("width {width} sidebar {sidebar}");
            let (mut ws, _rx) = fixture();
            bind(&mut ws);
            ws.hosts[0]
                .model
                .views
                .get_mut("s")
                .unwrap()
                .active_window_id = Some("idle".into());
            if sidebar {
                ws.settings.vertical_tabs = true;
                ws.sidebar.toggle();
            }
            let mut rt = runtime_at(ws, (width, 700.));
            settle(&mut rt);

            // Alone, it is the same words at every width, with the action's keys.
            let chip = rt
                .elements()
                .into_iter()
                .find(|e| e.key.as_deref() == Some("activity-chip"))
                .unwrap();
            assert_eq!(
                chip.help.as_deref(),
                Some("2 need you · Show activity · Ctrl Alt A")
            );
            assert_eq!(element_text(&rt, "activity-chip"), ["2 need you"]);
            room(&rt, width, sidebar, &format!("{what}, chip alone"));

            // A key sequence in progress: in a narrow window the chip waits for it.
            prefix(&mut rt);
            assert!(has(&rt, "pending-sequence"));
            assert_eq!(has(&rt, "activity-chip"), !narrow, "{what}");
            room(&rt, width, sidebar, &format!("{what}, sequence"));
            press(&mut rt, Key::Escape);
            settle(&mut rt);
            assert!(has(&rt, "activity-chip"), "{what}: back after the sequence");

            // Bytes arriving: in a narrow window it waits for the download chip, which
            // is then exactly the chip it is without this one.
            rt.update(|s, _| download(s));
            settle(&mut rt);
            assert!(has(&rt, "download-chip"));
            assert_eq!(has(&rt, "activity-chip"), !narrow, "{what}");
            let with_activity = element_text(&rt, "download-chip");
            room(&rt, width, sidebar, &format!("{what}, download"));
            let strip = (!sidebar).then(|| rect(&rt, "tabs"));
            rt.update(|s, _| {
                for h in &mut s.hosts {
                    h.model.activity.clear();
                }
            });
            settle(&mut rt);
            assert!(!has(&rt, "activity-chip"));
            assert_eq!(
                element_text(&rt, "download-chip"),
                with_activity,
                "{what}: the download chip never changes for this one"
            );
            if narrow {
                // Nothing in the bar had moved for the activity that was waiting.
                assert_eq!((!sidebar).then(|| rect(&rt, "tabs")), strip, "{what}");
            }

            // The transfer ends and something waits again: the chip returns, and the
            // active tab is still in view after it came and after it goes.
            rt.update(|s, _| {
                s.hosts[0].model.downloads.clear();
                state(s, 1, "build", ProgramState::Blocked);
            });
            settle(&mut rt);
            assert!(has(&rt, "activity-chip"), "{what}");
            room(&rt, width, sidebar, &format!("{what}, chip returned"));
            rt.update(|s, _| {
                for h in &mut s.hosts {
                    h.model.activity.clear();
                }
            });
            settle(&mut rt);
            assert!(!has(&rt, "activity-chip"));
            room(&rt, width, sidebar, &format!("{what}, chip gone"));
        }
    }
}

#[test]
fn at_the_minimum_width_there_is_no_chip_and_the_bar_is_as_it_was() {
    let _families = rune::text::with_families("Inter", "Inter");
    // At 640 pt the chip would leave the tab strip a sliver, so it is not drawn at all.
    let bar = |waiting: bool, transfer: bool| {
        let (mut ws, _rx) = fixture();
        ws.hosts[0]
            .model
            .views
            .get_mut("s")
            .unwrap()
            .active_window_id = Some("idle".into());
        if !waiting {
            for h in &mut ws.hosts {
                h.model.activity.clear();
            }
        }
        if transfer {
            download(&mut ws);
        }
        let mut rt = runtime_at(ws, (640., 700.));
        settle(&mut rt);
        assert!(!has(&rt, "activity-chip"));
        assert_eq!(rt.state.activity_chip().is_some(), waiting);
        assert!(rt.state.activity_chip_shown().is_none());
        [
            "identity",
            "tab-track",
            "tabs",
            "title-drag",
            "commands",
            "close-app",
        ]
        .map(|key| rect(&rt, key))
    };
    for transfer in [false, true] {
        assert_eq!(
            bar(true, transfer),
            bar(false, transfer),
            "download {transfer}"
        );
    }
    // Interface zoom lays the toolbar out narrower than the window: a 960 pt window at
    // 150% has a 640 pt toolbar, so no chip there either, and the bar is as it was.
    let zoomed = |waiting: bool| {
        let (mut ws, _rx) = fixture();
        ws.settings.ui_scale = 1.5;
        ws.hosts[0]
            .model
            .views
            .get_mut("s")
            .unwrap()
            .active_window_id = Some("idle".into());
        if !waiting {
            for h in &mut ws.hosts {
                h.model.activity.clear();
            }
        }
        let mut rt = runtime_at(ws, (960., 720.));
        settle(&mut rt);
        assert_eq!(rt.state.ui_scale(), 1.5);
        assert!(!has(&rt, "activity-chip"));
        assert_eq!(rt.state.activity_chip().is_some(), waiting);
        [
            "identity",
            "tab-track",
            "tabs",
            "title-drag",
            "commands",
            "close-app",
        ]
        .map(|key| rect(&rt, key))
    };
    assert_eq!(zoomed(true), zoomed(false));
    // Just above it the chip is drawn and the strip keeps its floor.
    let (mut ws, _rx) = fixture();
    ws.hosts[0]
        .model
        .views
        .get_mut("s")
        .unwrap()
        .active_window_id = Some("idle".into());
    let mut rt = runtime_at(ws, (720., 700.));
    settle(&mut rt);
    assert!(has(&rt, "activity-chip"));
    room(&rt, 720., false, "720");
}

#[test]
fn the_chip_arriving_alone_reveals_the_active_tab_again() {
    let _families = rune::text::with_families("Inter", "Inter");
    let (mut ws, _rx) = fixture();
    for h in &mut ws.hosts {
        h.model.activity.clear();
    }
    // The last of five tabs is on screen, so the strip overflows at this width.
    ws.hosts[0]
        .model
        .views
        .get_mut("s")
        .unwrap()
        .active_window_id = Some("idle".into());
    let mut rt = runtime_at(ws, (800., 700.));
    settle(&mut rt);
    assert!(!has(&rt, "activity-chip") && !has(&rt, "download-chip"));
    room(&rt, 800., false, "at rest");
    // Nothing else in the bar changes: only a background tab blocks.
    rt.update(|s, _| state(s, 1, "build", ProgramState::Blocked));
    settle(&mut rt);
    assert!(has(&rt, "activity-chip"));
    room(&rt, 800., false, "chip arrived alone");
    rt.update(|s, _| state(s, 1, "build", ProgramState::Working));
    settle(&mut rt);
    assert!(!has(&rt, "activity-chip"));
    room(&rt, 800., false, "chip left alone");
}

#[test]
fn the_chip_is_in_the_toolbar_only_and_never_floats_over_the_terminal() {
    let _families = rune::text::with_families("Inter", "Inter");
    for mode in [
        ToolbarBehavior::Always,
        ToolbarBehavior::OnHover,
        ToolbarBehavior::Hidden,
    ] {
        let (mut ws, _rx) = fixture();
        ws.settings.interface.toolbar = mode;
        let mut rt = runtime(ws);
        settle(&mut rt);
        let pane = rect(&rt, "pane:front");
        assert_eq!(has(&rt, "toolbar"), mode == ToolbarBehavior::Always);
        // A tab can wait for hours: with no toolbar drawn there is no chip, and nothing
        // is floated over the terminal for it.
        assert_eq!(has(&rt, "activity-chip"), mode == ToolbarBehavior::Always);
        assert!(!has(&rt, "pending-sequence-float"), "{mode:?}");
        if mode != ToolbarBehavior::Always {
            // A download does float, as before, and brings no activity chip with it.
            rt.update(|s, _| download(s));
            settle(&mut rt);
            assert!(has(&rt, "pending-sequence-float") && has(&rt, "download-chip"));
            assert!(!has(&rt, "activity-chip"));
            rt.update(|s, _| s.hosts[0].model.downloads.clear());
            settle(&mut rt);
            assert!(!has(&rt, "pending-sequence-float"));
        }
        if mode != ToolbarBehavior::Always {
            // Revealing the toolbar shows the chip in it, without moving the panes.
            rt.update(|s, _| s.toggle_toolbar());
            settle(&mut rt);
            assert!(has(&rt, "toolbar"));
            assert!(has(&rt, "activity-chip"));
            assert!(!has(&rt, "pending-sequence-float"));
            assert_eq!(rect(&rt, "pane:front"), pane);
        }
        rt.update(|s, _| {
            for h in &mut s.hosts {
                h.model.activity.clear();
            }
        });
        settle(&mut rt);
        assert!(!has(&rt, "activity-chip"));
        assert_eq!(rect(&rt, "pane:front"), pane);
    }
}
