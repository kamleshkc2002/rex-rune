//! Activity is an index, not a read receipt or a subscription.
use super::pane_verb_tests::{Camera, framed_terminal};
use super::*;
use crate::app::{
    actions,
    activity_list::{ACTION, COVERAGE},
    notify::NoteTarget,
};
use rex_linux::connection::activity::{BlockedKind, ProgramState, StatusRecord};
use serde_json::{Value, json};

fn fixture() -> (Workspace, Vec<flume::Receiver<HostCommand>>) {
    let mut ws = workspace_hosts(2);
    ws.window_focused = true;
    ws.settings.notifications.system_notifications = false;
    let receivers = (0..2).map(|h| capture_host(&mut ws, h)).collect();
    for h in 0..2 {
        let m = &mut ws.hosts[h].model;
        m.connected = true;
        m.sessions = vec![session("s"), session("other"), session("unknown")];
        m.selected = Some("s".into());
        for (sid, ids) in [
            ("s", vec!["front", "blocked", "error", "done", "idle"]),
            ("other", vec!["build"]),
        ] {
            m.views.insert(
                sid.into(),
                serde_json::from_value(json!({
                    "session_id":sid, "revision":1, "active_window_id":ids[0],
                    "windows":ids.iter().map(|id| json!({"window_id":id, "label":id,
                        "focused_block_id":id, "layers":[{"kind":"tiled", "layout":{"block_id":id},
                        "blocks":[{"block_id":id}]}]})).collect::<Vec<_>>()
                }))
                .unwrap(),
            );
        }
        for (id, state) in [
            ("front", ProgramState::Working),
            ("blocked", ProgramState::Blocked),
            ("error", ProgramState::Error),
            ("done", ProgramState::Done),
            ("idle", ProgramState::Idle),
            ("build", ProgramState::Working),
        ] {
            m.activity.insert(
                id.into(),
                [(
                    String::new(),
                    StatusRecord {
                        state,
                        kind: (state == ProgramState::Blocked).then_some(BlockedKind::Permission),
                        app: Some("cargo".into()),
                        live: true,
                        ..Default::default()
                    },
                )]
                .into(),
            );
        }
        ws.unseen_done.insert((ws.hosts[h].id, "done".into()));
        ws.unseen_bells.insert((ws.hosts[h].id, "done".into()));
    }
    (ws, receivers)
}
fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
}
fn edit(ws: &mut Workspace, query: &str) {
    ws.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new(query);
    ws.panel.as_mut().unwrap().inputs[0].focused = true;
    ws.query_changed();
}
fn target(ws: &Workspace, host: usize, sid: &str, tab: &str) -> NoteTarget {
    NoteTarget {
        host: ws.hosts[host].id,
        session: sid.into(),
        window: Some(tab.into()),
    }
}

#[test]
fn states_order_mute_hosts_sessions_and_filter() {
    let (mut ws, _rx) = fixture();
    ws.show_activity();
    let rows = ws.entries();
    assert_eq!(
        rows.iter()
            .map(|e| (e.group.as_str(), e.label.as_str()))
            .collect::<Vec<_>>(),
        [
            ("Needs you", "blocked"),
            ("Needs you", "blocked"),
            ("Error", "error"),
            ("Error", "error"),
            ("Done", "done"),
            ("Done", "done"),
            ("Working", "front"),
            ("Working", "build"),
            ("Working", "front"),
            ("Working", "build")
        ]
    );
    assert_eq!(rows[0].detail, "Needs approval");
    assert_eq!(rows[0].activity.as_ref().unwrap().place, "s-label · test-0");
    assert_eq!(rows[1].activity.as_ref().unwrap().place, "s-label · test-1");
    assert_ne!(
        format!("{:?}", rows[0].action),
        format!("{:?}", rows[1].action)
    );
    for (kind, word) in [
        (None, "Needs input"),
        (Some(BlockedKind::Question), "Question"),
        (Some(BlockedKind::Auth), "Needs sign-in"),
    ] {
        ws.hosts[0]
            .model
            .activity
            .get_mut("blocked")
            .unwrap()
            .get_mut("")
            .unwrap()
            .kind = kind;
        assert_eq!(ws.entries()[0].detail, word);
    }
    ws.settings.muted_tabs.push(rex_linux::settings::MutedTab {
        endpoint: ws.settings.hosts[0].endpoint.clone(),
        session: "s".into(),
        window: "blocked".into(),
    });
    assert_eq!(ws.entries().len(), 9);
    for (query, count) in [("TEST-1", 5), ("other", 2), ("build", 2), ("absent", 0)] {
        edit(&mut ws, query);
        assert_eq!(ws.entries().len(), count, "{query}");
    }
    edit(&mut ws, "");
    ws.unseen_done.clear();
    assert!(!ws.entries().iter().any(|e| e.group == "Done"));
    ws.hosts[1].model.connected = false;
    assert_eq!(ws.entries().len(), 3);
}

#[test]
fn browsing_is_local_and_is_not_seeing_a_tab() {
    let _families = rune::text::with_families("Inter", "Inter");
    let (mut ws, rx) = fixture();
    let (sender, terminal) = flume::unbounded();
    framed_terminal(&mut ws, "front", &sender, b"$ ");
    let note = target(&ws, 1, "s", "done");
    ws.notified_tabs.insert(note.clone());
    let mut rt = runtime(ws);
    settle(&mut rt);
    for r in &rx {
        r.drain();
    }
    terminal.drain();
    let done = rt.state.unseen_done.clone();
    let bells = rt.state.unseen_bells.clone();
    rt.update(|s, tx| {
        assert!(actions::run(s, ACTION, &Value::Null, tx));
    });
    settle(&mut rt);
    assert!(
        rt.elements()
            .iter()
            .any(|e| e.text.as_deref() == Some(COVERAGE))
    );
    rt.update(|s, _| edit(s, "build"));
    settle(&mut rt);
    rt.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    settle(&mut rt);
    assert!(rt.state.panel.is_none());
    assert!(focused(&rt).is_none());
    assert_eq!(rt.state.unseen_done, done);
    assert_eq!(rt.state.unseen_bells, bells);
    assert!(rt.state.notified_tabs.contains(&note));
    for r in &rx {
        assert!(r.is_empty(), "no host command");
    }
    assert!(terminal.is_empty(), "no PTY, resize or claim");
}

#[test]
fn live_model_updates_keep_the_same_selected_tab() {
    let (mut ws, _rx) = fixture();
    ws.show_activity();
    ws.panel.as_mut().unwrap().selected = 9;
    let chosen = ws.activity_selection().unwrap();
    assert_eq!(chosen, target(&ws, 1, "other", "build"));
    // Drive the production host wake path rather than a popup-specific refresh.
    ws.track_activity(1);
    let mut model = ws.hosts[1].model.clone();
    model
        .activity
        .get_mut("build")
        .unwrap()
        .get_mut("")
        .unwrap()
        .state = ProgramState::Blocked;
    *ws.hosts[1].connection.model.lock().unwrap() = model.clone();
    ws.sync(1, &mut rune::Tx::default());
    assert_eq!(ws.activity_selection(), Some(chosen.clone()));
    assert_eq!(ws.panel.as_ref().unwrap().selected, 2);
    model
        .activity
        .get_mut("build")
        .unwrap()
        .get_mut("")
        .unwrap()
        .state = ProgramState::Done;
    *ws.hosts[1].connection.model.lock().unwrap() = model.clone();
    ws.sync(1, &mut rune::Tx::default());
    assert_eq!(ws.activity_selection(), Some(chosen));
    assert_eq!(
        ws.entries()[ws.panel.as_ref().unwrap().selected].group,
        "Done"
    );
    model.views.get_mut("other").unwrap().windows.clear();
    *ws.hosts[1].connection.model.lock().unwrap() = model.clone();
    ws.sync(1, &mut rune::Tx::default());
    assert!(!ws.entries().iter().any(|e| matches!(&e.action, Intent::ActivityTab(t) if t.host == ws.hosts[1].id && t.session == "other")));
    assert!(ws.panel.as_ref().unwrap().selected < ws.entries().len());
    let selected = ws.activity_selection();
    let count = ws.entries().len();
    model
        .activity
        .get_mut("idle")
        .unwrap()
        .get_mut("")
        .unwrap()
        .state = ProgramState::Working;
    *ws.hosts[1].connection.model.lock().unwrap() = model;
    ws.sync(1, &mut rune::Tx::default());
    assert_eq!(
        ws.entries().len(),
        count + 1,
        "new activity appears while open"
    );
    assert_eq!(ws.activity_selection(), selected);
}

#[test]
fn regaining_window_focus_keeps_the_same_selected_tab() {
    let (mut ws, _rx) = fixture();
    // The tab on screen finished while the window was away, so it is latched Done and
    // listed above the rows that follow it.
    ws.window_focused = false;
    ws.hosts[0]
        .model
        .activity
        .get_mut("front")
        .unwrap()
        .get_mut("")
        .unwrap()
        .state = ProgramState::Idle;
    let front = (ws.hosts[0].id, "front".to_owned());
    ws.unseen_done.insert(front.clone());
    ws.show_activity();
    let chosen = target(&ws, 1, "other", "build");
    let before = ws
        .entries()
        .iter()
        .position(|e| matches!(&e.action, Intent::ActivityTab(t) if *t == chosen))
        .unwrap();
    ws.panel.as_mut().unwrap().selected = before;
    // Coming back sees that tab: its row leaves, and the rows below move up.
    ws.window_focus(true);
    assert!(!ws.unseen_done.contains(&front));
    assert_eq!(ws.activity_selection(), Some(chosen));
    assert_eq!(ws.panel.as_ref().unwrap().selected, before - 1);
}

#[test]
fn choosing_uses_notification_navigation_and_only_acknowledged_view_clears_latches() {
    let (mut ws, rx) = fixture();
    let chosen = target(&ws, 1, "s", "done");
    ws.show_activity();
    ws.panel.as_mut().unwrap().selected = 5;
    ws.submit(&mut rune::Tx::default());
    assert!(ws.panel.is_none());
    assert_eq!(ws.active, 1);
    assert!(rx[1].try_iter().any(|c| matches!(c, HostCommand::Navigate { session, window, .. } if session == "s" && window.as_deref() == Some("done"))));
    assert!(ws.unseen_on(1, "done") && ws.bell_on(1, "done"));
    ws.hosts[1]
        .model
        .views
        .get_mut("s")
        .unwrap()
        .active_window_id = chosen.window;
    ws.track_activity(1);
    assert!(!ws.unseen_on(1, "done") && !ws.bell_on(1, "done"));
    ws.show_activity();
    let other = target(&ws, 0, "other", "build");
    ws.perform(Intent::ActivityTab(other), &mut rune::Tx::default());
    assert_eq!(ws.active, 0);
    assert!(rx[0].try_iter().any(|c| matches!(c, HostCommand::Navigate { session, window, .. } if session == "other" && window.as_deref() == Some("build"))));
}

#[test]
fn stale_click_reports_gone_instead_of_navigating() {
    let (mut ws, rx) = fixture();
    ws.show_activity();
    let row = ws.entries().remove(0).action;
    ws.hosts[0]
        .model
        .views
        .get_mut("s")
        .unwrap()
        .windows
        .retain(|w| w.window_id.as_deref() != Some("blocked"));
    ws.perform(row, &mut rune::Tx::default());
    assert!(ws.panel.is_none());
    assert!(
        ws.notices
            .iter()
            .any(|n| n.text.contains("That tab is no longer available"))
    );
    for r in rx {
        assert!(r.is_empty());
    }
}

#[test]
fn activity_wiring_is_unbound_local_and_rebindable() {
    let (mut ws, _rx) = fixture();
    let spec = actions::find(ACTION).unwrap();
    assert!(matches!(spec.remote(), actions::Remote::Never(_)));
    assert!(ws.shortcut_chords(ACTION, &Value::Null).is_empty());
    assert!(ws.shortcut_rows().iter().any(|r| r.action == ACTION));
    ws.picker(false);
    let row = ws
        .entries()
        .into_iter()
        .find(|e| e.label == "Show activity…")
        .unwrap();
    ws.perform(row.action, &mut rune::Tx::default());
    assert!(matches!(
        ws.panel.as_ref().unwrap().kind,
        PanelKind::Activity
    ));
}

#[test]
fn empty_filter_and_pointer_selection_use_the_shared_popup() {
    let _families = rune::text::with_families("Inter", "Inter");
    let (mut ws, rx) = fixture();
    ws.show_activity();
    let mut rt = runtime(ws);
    settle(&mut rt);
    rt.update(|s, _| edit(s, "missing"));
    settle(&mut rt);
    assert!(
        rt.elements()
            .iter()
            .any(|e| e.text.as_deref() == Some("No matching activity."))
    );
    rt.update(|s, _| edit(s, ""));
    settle(&mut rt);
    click(&mut rt, "entry:1");
    settle(&mut rt);
    assert!(rt.state.panel.is_none());
    assert_eq!(rt.state.active, 1);
    assert!(rx[1].try_iter().any(|c| matches!(c, HostCommand::Navigate { session, window, .. } if session == "s" && window.as_deref() == Some("blocked"))));
    rt.update(|s, _| {
        for h in &mut s.hosts {
            h.model.activity.clear();
        }
        s.unseen_done.clear();
        s.show_activity();
    });
    settle(&mut rt);
    assert!(
        rt.elements()
            .iter()
            .any(|e| e.text.as_deref() == Some("Nothing needs you."))
    );
    assert!(rt.state.entries().is_empty());
}

fn draw(case: &str) {
    let Some(mut camera) = Camera::new() else {
        return;
    };
    let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
    let _families = rune::text::with_families("Inter", "IBM Plex Mono");
    let (mut ws, _rx) = fixture();
    // Four groups fit together in the minimum viewport with one representative each.
    ws.hosts[1].model.connected = false;
    ws.hosts[0].model.activity.remove("build");
    ws.show_activity();
    if case == "filtered" {
        edit(&mut ws, "blocked");
    }
    if case == "empty" {
        ws.hosts[0].model.activity.clear();
        ws.unseen_done.clear();
    }
    let size = if std::env::var_os("REX_M32_MINIMUM").is_some() {
        (640., 480.)
    } else {
        (1024., 700.)
    };
    ws.viewport = [size.0, size.1];
    let mut rt = Runtime::with_text(
        ws,
        view,
        size,
        Env::new(true, 1.),
        rune::text::TextSystem::new(fonts.files, fonts.families),
    );
    settle(&mut rt);
    camera.snap(&mut rt, &format!("activity-{case}"));
}
#[test]
fn activity_populated_draw() {
    draw("populated");
}
#[test]
fn activity_filtered_draw() {
    draw("filtered");
}
#[test]
fn activity_empty_draw() {
    draw("empty");
}
