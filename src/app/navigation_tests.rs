//! joegoldin's navigation checks and the evaluator's three scratch regressions,
//! with explicit correlated acknowledgements and a deterministic debounce clock.
use super::*;
#[path = "navigation_review_tests.rs"]
mod review;
use crate::app::actions::{self, HISTORY_BACK, HISTORY_FORWARD, NEXT_GLOBAL, PREVIOUS_GLOBAL};
use crate::app::navigation::Visit;
use std::time::{Duration, Instant};

fn cycle(rt: &mut Runtime<Workspace>, shift: bool) {
    rt.key(KeyEvent {
        key: Key::Tab,
        mods: Mods {
            ctrl: true,
            shift,
            ..Default::default()
        },
    });
    rt.frame(1. / 60.);
}

fn acknowledge(ws: &mut Workspace) {
    let p = ws.navigation.pending.as_ref().expect("pending navigation");
    let (id, visit) = (p.id, p.visit.clone());
    let host = ws.host_index(visit.host).unwrap();
    ws.hosts[host].model.selected = Some(visit.session.clone());
    if let Some(window) = visit.window {
        ws.hosts[host]
            .model
            .views
            .get_mut(&visit.session)
            .unwrap()
            .active_window_id = Some(window);
    }
    ws.hosts[host].model.navigation_result = Some((id, true));
    ws.observe_navigation();
}

fn recorded_history() -> (Runtime<Workspace>, flume::Receiver<HostCommand>) {
    let (mut ws, rx) = tabbed_workspace("single");
    ws.observe_navigation();
    ws.hosts[0]
        .model
        .views
        .get_mut("s")
        .unwrap()
        .active_window_id = Some("split".into());
    ws.observe_navigation();
    (runtime(ws), rx)
}

#[test]
fn reconnect_keeps_history() {
    // Adopted from scratch_reconnect_keeps_history.
    let (mut rt, rx) = recorded_history();
    rt.update(|s, _| {
        let saved = s.hosts[0].model.views.clone();
        s.hosts[0].model.connected = false;
        s.hosts[0].model.views.clear();
        s.observe_navigation();
        s.hosts[0].model.connected = true;
        s.observe_navigation(); // Connected, but the cache is still empty.
        s.hosts[0].model.views = saved;
        s.observe_navigation();
    });
    rt.update(|s, tx| s.navigate_history(-1, tx));
    let sent = requests(&rx);
    assert_eq!(
        sent.len(),
        1,
        "Back after reconnect must still reach single"
    );
    assert_eq!(sent[0].2["window_id"], "single");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn notice_keeps_forward_history() {
    // Adopted from scratch_notice_keeps_forward_history. Check both an old sticky
    // notice and a new unrelated failure while the navigation request is in flight.
    let (mut rt, rx) = recorded_history();
    rt.update(|s, _| s.hosts[0].model.notice = Some("Upload failed".into()));
    rt.update(|s, tx| s.navigate_history(-1, tx));
    assert_eq!(requests(&rx)[0].2["window_id"], "single");
    rt.update(|s, _| {
        s.observe_navigation();
        s.hosts[0].model.notice = Some("Search failed".into());
        s.observe_navigation();
        acknowledge(s);
    });
    rt.update(|s, tx| s.navigate_history(1, tx));
    let sent = requests(&rx);
    assert_eq!(sent.len(), 1, "Forward must still reach split");
    assert_eq!(sent[0].2["window_id"], "split");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn rapid_ctrl_tab_with_notice() {
    // Adopted from scratch_rapid_ctrl_tab_with_notice.
    let (ws, rx) = tabbed_workspace("single");
    let mut rt = runtime(ws);
    rt.update(|s, _| s.hosts[0].model.notice = Some("Upload failed".into()));
    cycle(&mut rt, false);
    rt.update(|s, _| s.observe_navigation());
    cycle(&mut rt, false);
    let sent = requests(&rx);
    let ids: Vec<_> = sent
        .iter()
        .map(|r| r.2["window_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["split", "single"], "two presses must move twice");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn ctrl_tab_wraps_but_page_keys_stay_server_side_without_a_view() {
    let (ws, rx) = tabbed_workspace("single");
    let mut rt = runtime(ws);
    cycle(&mut rt, false);
    cycle(&mut rt, false);
    cycle(&mut rt, true);
    let sent = requests(&rx);
    assert_eq!(
        sent.iter()
            .map(|r| r.2["window_id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["split", "single", "split"]
    );
    rt.update(|s, _| s.hosts[0].model.views.clear());
    for key in [Key::PageUp, Key::PageDown] {
        rt.key(KeyEvent {
            key,
            mods: Mods {
                ctrl: true,
                ..Default::default()
            },
        });
    }
    rt.frame(1. / 60.);
    let sent = requests(&rx);
    assert_eq!(
        sent.iter().map(|r| r.0.as_str()).collect::<Vec<_>>(),
        ["session.focus_previous_window", "session.focus_next_window"]
    );
    assert!(
        sent.iter()
            .all(|r| r.2.get("wrap").is_none() && r.2.get("window_id").is_none())
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn mouse_back_and_forward_revisit_tabs_without_branching_history() {
    let (mut rt, rx) = recorded_history();
    rt.button_down(500., 300., rune::PointerButton::Back);
    rt.button_up(500., 300., rune::PointerButton::Back);
    rt.frame(1. / 60.);
    assert_eq!(requests(&rx)[0].2["window_id"], "single");
    rt.update(|s, _| acknowledge(s));
    rt.button_down(500., 300., rune::PointerButton::Forward);
    rt.button_up(500., 300., rune::PointerButton::Forward);
    rt.frame(1. / 60.);
    assert_eq!(requests(&rx)[0].2["window_id"], "split");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn ctrl_tab_crosses_sidebar_order_but_only_attaches_the_settled_session() {
    let (seed, _) = tabbed_workspace("split");
    let mut ws = workspace_hosts(3);
    ws.hosts[0].model = seed.hosts[0].model.clone();
    let mut view = seed.hosts[0].model.views["s"].clone();
    view.session_id = Some("t".into());
    view.active_window_id = Some("t1".into());
    for (w, id) in view.windows.iter_mut().zip(["t1", "t2"]) {
        w.window_id = Some(id.into());
    }
    ws.hosts[0].model.sessions.push(session("t"));
    ws.hosts[0].model.views.insert("t".into(), view.clone());
    ws.hosts[1].model.connected = true;
    ws.hosts[1].model.sessions = vec![session("u")];
    // Host 1 has not selected/attached u yet.
    view.session_id = Some("u".into());
    view.active_window_id = Some("u1".into());
    for (w, id) in view.windows.iter_mut().zip(["u1", "u2"]) {
        w.window_id = Some(id.into());
    }
    ws.hosts[1].model.views.insert("u".into(), view.clone());
    ws.hosts[2].model.sessions = vec![session("u")];
    ws.hosts[2].model.views.insert("u".into(), view);
    let rx0 = capture_host(&mut ws, 0);
    let rx1 = capture_host(&mut ws, 1);
    let rx2 = capture_host(&mut ws, 2);
    let mut rt = runtime(ws);
    for (shift, host, session, window) in [
        (false, 0, "t", "t1"),
        (false, 0, "t", "t2"),
        (false, 1, "u", "u1"),
        (false, 1, "u", "u2"),
        (false, 0, "s", "single"),
        (false, 0, "s", "split"),
        (true, 0, "s", "single"),
        (true, 1, "u", "u2"),
    ] {
        cycle(&mut rt, shift);
        let p = &rt.state.navigation.pending.as_ref().unwrap().visit;
        assert_eq!(
            p,
            &Visit {
                host: rt.state.hosts[host].id,
                session: session.into(),
                window: Some(window.into())
            }
        );
    }
    assert!(requests(&rx0).is_empty());
    assert!(requests(&rx1).is_empty());
    rt.update(|s, tx| {
        let at = s.navigation.pending.as_ref().unwrap().dispatch_at.unwrap();
        assert!(s.poll_navigation(at, tx));
    });
    assert_eq!(requests(&rx1)[0].2["window_id"], "u2");
    assert!(requests(&rx0).is_empty());
    assert!(requests(&rx2).is_empty());
    assert_eq!(rt.state.active, 1);
    std::fs::remove_file(&rt.state.settings_path).ok();
    std::fs::remove_file(&seed.settings_path).ok();
}

#[test]
fn unknown_sessions_are_one_stop_then_their_loaded_windows_are_traversed() {
    let (mut ws, rx) = tabbed_workspace("split");
    let saved = ws.hosts[0].model.views["s"].clone();
    ws.hosts[0].model.sessions.push(session("unknown"));
    let mut rt = runtime(ws);
    cycle(&mut rt, false);
    let p = &rt.state.navigation.pending.as_ref().unwrap().visit;
    assert_eq!(p.session, "unknown");
    assert_eq!(p.window, None);
    assert!(requests(&rx).is_empty());
    rt.update(|s, tx| {
        s.poll_navigation(Instant::now() + Duration::from_secs(1), tx);
    });
    assert!(rx.try_iter().any(
        |c| matches!(c, HostCommand::Navigate { session, window: None, .. } if session == "unknown")
    ));
    rt.update(|s, _| {
        let mut view = saved;
        view.session_id = Some("unknown".into());
        view.active_window_id = Some("single".into());
        s.hosts[0].model.views.insert("unknown".into(), view);
        acknowledge(s);
    });
    cycle(&mut rt, false);
    assert_eq!(requests(&rx)[0].2["window_id"], "split");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_session_read_only_for_activity_is_one_stop_until_it_is_opened_or_listed() {
    // The background status pass reads every session's layout. Knowing it is not having
    // opened it: Ctrl Tab must not step into its tabs, which would change that session's
    // active tab on the server.
    let (mut ws, rx) = tabbed_workspace("split");
    let mut view = ws.hosts[0].model.views["s"].clone();
    view.session_id = Some("background".into());
    view.active_window_id = Some("single".into());
    let model = &mut ws.hosts[0].model;
    model.sessions.push(session("background"));
    model.views.insert("background".into(), view);
    model.background_views.insert("background".into());
    let mut rt = runtime(ws);
    cycle(&mut rt, false);
    let p = rt.state.navigation.pending.as_ref().unwrap().visit.clone();
    assert_eq!((p.session.as_str(), p.window), ("background", None));
    rt.update(|s, tx| {
        s.poll_navigation(Instant::now() + Duration::from_secs(1), tx);
    });
    // One correlated navigation to the session, naming no tab: nothing asks the server
    // to change which of its tabs is active.
    let sent: Vec<_> = rx.try_iter().collect();
    assert!(sent.iter().any(
        |c| matches!(c, HostCommand::Navigate { session, window: None, .. } if session == "background")
    ));
    assert!(!sent.iter().any(|c| matches!(
        c,
        HostCommand::Navigate {
            window: Some(_),
            ..
        }
    )));
    // Opened (the connection drops it from the set when it is selected), its tabs are stops.
    rt.update(|s, _| {
        s.hosts[0].model.background_views.remove("background");
        acknowledge(s);
    });
    cycle(&mut rt, false);
    let p = rt.state.navigation.pending.as_ref().unwrap().visit.clone();
    assert_eq!(p.session, "background");
    assert!(p.window.is_some(), "the opened session's next tab");
    std::fs::remove_file(&rt.state.settings_path).ok();

    // With tabs listed in the sidebar its tabs are listed, so they are stops as before.
    let (mut ws, _rx) = tabbed_workspace("split");
    let mut view = ws.hosts[0].model.views["s"].clone();
    view.session_id = Some("background".into());
    let model = &mut ws.hosts[0].model;
    model.sessions.push(session("background"));
    model.views.insert("background".into(), view);
    model.background_views.insert("background".into());
    ws.settings.vertical_tabs = true;
    if !ws.sidebar.is_open() {
        ws.sidebar.toggle();
    }
    let mut rt = runtime(ws);
    cycle(&mut rt, false);
    let p = rt.state.navigation.pending.as_ref().unwrap().visit.clone();
    assert_eq!(p.session, "background");
    assert!(p.window.is_some(), "listed tabs are stops");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_missing_current_tab_starts_at_the_correct_end_in_both_directions() {
    for (shift, expected) in [(false, "single"), (true, "split")] {
        let (mut ws, rx) = tabbed_workspace("single");
        ws.hosts[0].model.selected = None;
        let mut rt = runtime(ws);
        cycle(&mut rt, shift);
        assert_eq!(
            rt.state
                .navigation
                .pending
                .as_ref()
                .unwrap()
                .visit
                .window
                .as_deref(),
            Some(expected)
        );
        rt.update(|s, tx| {
            s.poll_navigation(Instant::now() + Duration::from_secs(1), tx);
        });
        assert_eq!(requests(&rx)[0].2["window_id"], expected);
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn only_the_matching_failure_or_deadline_cancels_a_visit_and_keeps_forward() {
    for timeout in [false, true] {
        let (mut rt, rx) = recorded_history();
        rt.update(|s, tx| s.navigate_history(-1, tx));
        rt.update(|s, _| acknowledge(s));
        requests(&rx);
        rt.update(|s, tx| s.navigate_history(1, tx));
        rt.update(|s, tx| {
            let p = s.navigation.pending.as_ref().unwrap();
            let (id, deadline) = (p.id, p.deadline);
            s.hosts[0].model.navigation_result = Some((id - 1, false));
            s.observe_navigation();
            assert!(s.navigation.pending.is_some(), "an old failure is not ours");
            if timeout {
                s.poll_navigation(deadline, tx);
            } else {
                s.hosts[0].model.navigation_result = Some((id, false));
                s.observe_navigation();
            }
            assert!(s.navigation.pending.is_none());
        });
        requests(&rx);
        rt.update(|s, tx| s.navigate_history(1, tx));
        assert_eq!(
            requests(&rx)[0].2["window_id"],
            "split",
            "a failed Forward can be retried"
        );
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn closed_windows_destroyed_sessions_and_removed_hosts_are_skipped() {
    let (mut ws, rx) = tabbed_workspace("single");
    ws.observe_navigation();
    let mut other = ws.hosts[0].model.views["s"].clone();
    other.session_id = Some("other".into());
    ws.hosts[0].model.sessions.push(session("other"));
    ws.hosts[0].model.views.insert("other".into(), other);
    ws.hosts[0].model.selected = Some("other".into());
    ws.observe_navigation();
    ws.hosts[0].model.selected = Some("s".into());
    ws.hosts[0]
        .model
        .views
        .get_mut("s")
        .unwrap()
        .active_window_id = Some("split".into());
    ws.observe_navigation();
    ws.hosts[0]
        .model
        .sessions
        .retain(|s| s.session_id != "other");
    let mut rt = runtime(ws);
    rt.update(|s, tx| s.navigate_history(-1, tx));
    assert_eq!(requests(&rx)[0].2["window_id"], "single");
    rt.update(|s, _| acknowledge(s));
    rt.update(|s, tx| {
        s.hosts[0]
            .model
            .views
            .get_mut("s")
            .unwrap()
            .windows
            .retain(|w| w.window_id.as_deref() != Some("split"));
        s.navigate_history(1, tx);
    });
    assert!(requests(&rx).is_empty(), "a known closed tab is skipped");
    rt.update(|s, tx| {
        // Reusing an array position with a new stable id cannot redirect old visits.
        s.hosts[0].id = crate::app::workspace::HostId(99);
        s.navigate_history(-1, tx);
    });
    assert!(requests(&rx).is_empty());
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn history_crosses_hosts_and_a_failed_forward_keeps_its_origin() {
    let (seed, _) = tabbed_workspace("single");
    let mut ws = workspace_hosts(2);
    for host in &mut ws.hosts {
        host.model = seed.hosts[0].model.clone();
    }
    ws.observe_navigation();
    ws.active = 1;
    ws.hosts[1]
        .model
        .views
        .get_mut("s")
        .unwrap()
        .active_window_id = Some("split".into());
    ws.observe_navigation();
    let rx0 = capture_host(&mut ws, 0);
    let rx1 = capture_host(&mut ws, 1);
    let mut rt = runtime(ws);
    rt.update(|s, tx| s.navigate_history(-1, tx));
    assert_eq!(rt.state.active, 0);
    assert_eq!(requests(&rx0)[0].2["window_id"], "single");
    rt.update(|s, _| acknowledge(s));
    rt.update(|s, tx| s.navigate_history(1, tx));
    requests(&rx1);
    rt.update(|s, _| {
        let id = s.navigation.pending.as_ref().unwrap().id;
        // The destination now shows a different tab, but failure must not record it.
        s.hosts[1]
            .model
            .views
            .get_mut("s")
            .unwrap()
            .active_window_id = Some("single".into());
        s.hosts[1].model.navigation_result = Some((id, false));
        s.observe_navigation();
        assert_eq!(s.active, 0);
        s.observe_navigation();
    });
    rt.update(|s, tx| s.navigate_history(1, tx));
    assert_eq!(rt.state.active, 1);
    assert_eq!(requests(&rx1)[0].2["window_id"], "split");
    std::fs::remove_file(&rt.state.settings_path).ok();
    std::fs::remove_file(&seed.settings_path).ok();
}

#[test]
fn named_actions_are_in_commands_and_keep_modal_mouse_navigation_out() {
    let (mut rt, rx) = recorded_history();
    rt.update(|s, tx| {
        for (name, expected) in [(NEXT_GLOBAL, "single"), (PREVIOUS_GLOBAL, "split")] {
            assert!(actions::run(s, name, &serde_json::Value::Null, tx));
            assert_eq!(
                s.navigation
                    .pending
                    .as_ref()
                    .unwrap()
                    .visit
                    .window
                    .as_deref(),
                Some(expected)
            );
        }
        s.navigation.cancel();
        assert!(actions::run(s, HISTORY_BACK, &serde_json::Value::Null, tx));
        acknowledge(s);
        assert!(actions::run(
            s,
            HISTORY_FORWARD,
            &serde_json::Value::Null,
            tx
        ));
        acknowledge(s);
        s.picker(false);
        for name in [
            NEXT_GLOBAL,
            PREVIOUS_GLOBAL,
            HISTORY_BACK,
            HISTORY_FORWARD,
            actions::SEARCH,
        ] {
            assert!(
                s.entries()
                    .iter()
                    .any(|e| matches!(e.action, Intent::ClientAction(n, _) if n == name))
            );
        }
    });
    requests(&rx);
    rt.button_down(500., 300., rune::PointerButton::Back);
    rt.button_up(500., 300., rune::PointerButton::Back);
    rt.frame(1. / 60.);
    assert!(
        requests(&rx).is_empty(),
        "history cannot navigate behind a panel"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn navigation_keys_and_mouse_buttons_do_not_leak_to_a_tracking_terminal() {
    use rex_linux::connection::{Command, Connection, TerminalSender};
    let (mut ws, _) = tabbed_workspace("single");
    let (sender, sent) = flume::unbounded();
    let connection = Connection {
        model: Default::default(),
        commands: TerminalSender::new(sender, "s".into(), "block:a".into()),
        wake: flume::unbounded().1,
    };
    let mut terminal = ghostty_vt::Terminal::new(ghostty_vt::TerminalOptions {
        cols: 80,
        rows: 24,
        ..Default::default()
    })
    .unwrap();
    terminal.vt_write(b"\x1b[?1000h\x1b[?1006h");
    {
        let mut m = connection.model.lock().unwrap();
        m.connected = true;
        m.control_connected = true;
        m.frame = Some(std::sync::Arc::new(
            rex_linux::terminal::Renderer::new()
                .unwrap()
                .frame(&terminal)
                .unwrap(),
        ));
    }
    let mut term = crate::app::term::Term::new(connection);
    term.refresh();
    ws.terminals.insert(ws.term_key("block:a"), term);
    let mut rt = runtime(ws);
    cycle(&mut rt, false);
    cycle(&mut rt, true);
    for button in [rune::PointerButton::Back, rune::PointerButton::Forward] {
        rt.button_down(500., 300., button);
        rt.button_up(500., 300., button);
    }
    rt.frame(1. / 60.);
    assert!(
        !sent.try_iter().any(|c| match c {
            HostCommand::Terminal {
                command: Command::Key(_),
                ..
            } => true,
            HostCommand::Terminal {
                command: Command::Mouse(m),
                ..
            } => m["action"] != "motion",
            _ => false,
        }),
        "navigation sends neither keys nor mouse presses/releases"
    );
    rt.key(KeyEvent {
        key: Key::Tab,
        mods: Mods {
            ctrl: true,
            shift: true,
            alt: true,
            ..Default::default()
        },
    });
    rt.frame(1. / 60.);
    assert!(
        sent.try_iter().any(|c| matches!(
            c,
            HostCommand::Terminal {
                command: Command::Key(_),
                ..
            }
        )),
        "Alt Ctrl Shift Tab is not a workspace shortcut"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}
