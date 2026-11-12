//! Activity is an index, not a read receipt or a subscription.
use super::pane_verb_tests::{Camera, framed_terminal};
use super::*;
use crate::app::{
    actions,
    activity_list::{self, ACTION, COVERAGE},
    notify::NoteTarget,
};
use rex_rune::connection::activity::{BlockedKind, ProgramState, StatusRecord};
use serde_json::{Value, json};

pub(super) fn fixture() -> (Workspace, Vec<flume::Receiver<HostCommand>>) {
    let mut ws = workspace_hosts(2);
    ws.window_focused = true;
    ws.settings.notifications.system_notifications = false;
    let receivers = (0..2).map(|h| capture_host(&mut ws, h)).collect();
    for h in 0..2 {
        let m = &mut ws.hosts[h].model;
        m.connected = true;
        m.activity_reading = true;
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
#[test]
fn swept_status_is_visible_but_is_not_news_or_a_done_latch() {
    for (state, word, chip) in [
        (
            ProgramState::Blocked,
            Some("Needs input"),
            Some("1 needs you"),
        ),
        (ProgramState::Error, Some("Error"), Some("1 error")),
        (ProgramState::Done, None, None),
    ] {
        let (mut ws, rx) = fixture();
        ws.hosts[1].model.connected = false;
        ws.settings.notifications.system_notifications = true;
        ws.unseen_done.clear();
        ws.unseen_bells.clear();
        ws.hosts[0].model.activity.clear();
        ws.track_activity(0);
        let records = rex_rune::connection::activity::parse_status(&json!({"records":[{
            "id":"", "state":match state {
                ProgramState::Blocked => "blocked", ProgramState::Error => "error", _ => "done"
            }
        }]}));
        assert!(records.iter().all(|(_, r)| !r.live));
        ws.hosts[0]
            .model
            .activity
            .insert("build".into(), records.into_iter().collect());
        ws.track_activity(0);
        assert_eq!(
            ws.session_word(0, "other")
                .map(|(w, _, _)| w.into_owned())
                .as_deref(),
            word
        );
        assert_eq!(ws.activity_chip().map(|c| c.label()).as_deref(), chip);
        ws.show_activity();
        assert_eq!(ws.entries().len(), usize::from(word.is_some()));
        if let Some(word) = word {
            assert_eq!(ws.entries()[0].label, "build");
            assert!(ws.entries()[0].detail.contains(word));
        }
        assert!(ws.outbox.is_empty(), "queried state never notifies");
        assert!(ws.unseen_done.is_empty(), "queried Done is not unseen Done");
        assert_eq!(ws.hosts[0].model.selected.as_deref(), Some("s"));
        assert!(rx.iter().all(|r| r.is_empty()));
    }
}

#[test]
fn coverage_only_names_current_gaps_and_disconnected_hosts() {
    let (mut ws, _) = fixture();
    assert_eq!(ws.activity_coverage(), COVERAGE);
    for host in &mut ws.hosts {
        host.model.activity_reading = false;
    }
    assert_eq!(ws.activity_coverage(), "");
    ws.hosts[0].model.activity_incomplete = true;
    assert_eq!(
        ws.activity_coverage(),
        crate::app::activity_list::INCOMPLETE
    );
    ws.hosts[1].model.connected = false;
    assert_eq!(
        ws.activity_coverage(),
        "1 host is not connected. Some activity could not be read."
    );
    ws.hosts[0].model.activity_reading = true;
    assert_eq!(
        ws.activity_coverage(),
        "1 host is not connected. Reading activity…"
    );
    ws.hosts[0].model.connected = false;
    assert_eq!(ws.activity_coverage(), "2 hosts are not connected.");
}

#[test]
fn rendered_coverage_has_no_empty_row_after_a_clean_pass() {
    let _families = rune::text::with_families("Inter", "Inter");
    let (mut ws, _) = fixture();
    ws.show_activity();
    let mut rt = runtime(ws);
    for (reading, incomplete, expected) in [
        (true, false, Some(COVERAGE)),
        (false, true, Some(crate::app::activity_list::INCOMPLETE)),
        (false, false, None),
    ] {
        rt.update(|s, _| {
            for host in &mut s.hosts {
                host.model.activity_reading = reading;
                host.model.activity_incomplete = incomplete;
            }
        });
        settle(&mut rt);
        let lines: Vec<_> = rt
            .elements()
            .iter()
            .filter_map(|e| e.text.as_deref())
            .filter(|t| *t == COVERAGE || *t == crate::app::activity_list::INCOMPLETE)
            .map(str::to_owned)
            .collect();
        assert_eq!(
            lines,
            expected.into_iter().map(str::to_owned).collect::<Vec<_>>()
        );
        assert!(
            !rt.state.entries().is_empty(),
            "the caveat does not replace activity"
        );
    }
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
    ws.settings.muted_tabs.push(rex_rune::settings::MutedTab {
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
fn the_filter_matches_the_state_and_the_coverage_line_names_unconnected_hosts() {
    let (mut ws, _rx) = fixture();
    ws.show_activity();
    // No tab, session or host is called any of these: only the state can match.
    for (query, group, count) in [
        ("needs approval", "Needs you", 2),
        ("needs you", "Needs you", 2),
        ("working", "Working", 4),
    ] {
        edit(&mut ws, query);
        let rows = ws.entries();
        assert_eq!(rows.len(), count, "{query}");
        assert!(rows.iter().all(|e| e.group == group), "{query}");
    }
    edit(&mut ws, "");
    assert_eq!(ws.activity_coverage(), COVERAGE);
    ws.hosts[1].model.connected = false;
    assert_eq!(
        ws.activity_coverage(),
        "1 host is not connected. Reading activity…"
    );
    ws.hosts[0].model.connected = false;
    assert_eq!(ws.activity_coverage(), "2 hosts are not connected.");
    // With every host down nothing is listed, and the line says why.
    assert!(ws.entries().is_empty());
}

#[test]
fn opening_from_the_chip_starts_on_a_tab_that_is_not_the_one_in_front() {
    let (mut ws, _rx) = fixture();
    // The tab on screen blocks too, so it heads the Needs you group.
    ws.hosts[0]
        .model
        .activity
        .get_mut("front")
        .unwrap()
        .get_mut("")
        .unwrap()
        .state = ProgramState::Blocked;
    let front = target(&ws, 0, "s", "front");
    ws.show_activity();
    assert_eq!(ws.panel.as_ref().unwrap().selected, 0);
    assert_eq!(ws.activity_selection(), Some(front.clone()));
    ws.panel = None;
    // The chip counted the others: its list opens on the first of them.
    ws.show_activity_elsewhere();
    assert_eq!(ws.panel.as_ref().unwrap().selected, 1);
    assert_eq!(
        ws.activity_selection(),
        Some(target(&ws, 0, "s", "blocked"))
    );
    ws.panel = None;
    // The same session and tab id on the other host is not the tab in front: with it
    // the only other tab that needs you, it is the row the list opens on.
    for id in ["blocked", "error", "done", "build"] {
        ws.hosts[0].model.activity.remove(id);
    }
    ws.hosts[1].model.activity.remove("blocked");
    ws.hosts[1]
        .model
        .activity
        .get_mut("front")
        .unwrap()
        .get_mut("")
        .unwrap()
        .state = ProgramState::Blocked;
    ws.unseen_done.clear();
    ws.show_activity_elsewhere();
    assert_eq!(ws.panel.as_ref().unwrap().selected, 1);
    assert_eq!(ws.activity_selection(), Some(target(&ws, 1, "s", "front")));
    ws.panel = None;
    // Only the tab in front qualifies: the list still opens, on it.
    ws.hosts[1].model.connected = false;
    ws.show_activity_elsewhere();
    assert_eq!(ws.activity_selection(), Some(front));
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
    if case == "elapsed" {
        let now = std::time::Instant::now();
        for (id, seconds, reported) in [
            ("front", 7200, false),
            ("blocked", 240, false),
            ("error", 3 * 86400, true),
            ("done", 10, false),
        ] {
            ws.hosts[0]
                .model
                .activity
                .get_mut(id)
                .unwrap()
                .get_mut("")
                .unwrap()
                .age = Some(rex_rune::connection::activity::time::Age {
                observed: now,
                initial: std::time::Duration::from_secs(seconds),
                reported,
            });
        }
        ws.hosts[0].model.views.get_mut("s").unwrap().windows[1].label =
            Some("An agent waiting for approval on a very long implementation title".into());
    }
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

#[test]
fn activity_elapsed_draw() {
    draw("elapsed");
}

#[test]
fn elapsed_boundaries_and_accessible_words_use_injected_now() {
    use rex_rune::connection::activity::time::Age;
    use std::time::{Duration, Instant};
    let start = Instant::now();
    for (seconds, text, words) in [
        (59, "now", "less than a minute"),
        (60, "1m", "1 minute"),
        (59 * 60, "59m", "59 minutes"),
        (60 * 60, "1h", "1 hour"),
        (23 * 3600, "23h", "23 hours"),
        (24 * 3600, "1d", "1 day"),
        (1234 * 86400, "1234d", "1234 days"),
    ] {
        let now = start + Duration::from_secs(seconds);
        let elapsed = activity_list::elapsed(Age::live(start), ProgramState::Blocked, now);
        assert_eq!(elapsed.text, text);
        assert_eq!(elapsed.label, format!("Observed waiting for {words}"));
        let reported = activity_list::elapsed(
            Age {
                reported: true,
                ..Age::live(start)
            },
            ProgramState::Working,
            now,
        );
        assert_eq!(reported.text, format!("reported {text}"));
        assert!(reported.label.contains("state may have begun earlier"));
    }
}

#[test]
fn folded_state_clock_survives_progress_and_restarts_after_a_higher_state() {
    use rex_rune::connection::activity::time::Age;
    use std::time::{Duration, Instant};
    let (mut ws, _rx) = fixture();
    let start = Instant::now();
    let later = start + Duration::from_secs(7200);
    ws.hosts[0]
        .model
        .activity
        .get_mut("front")
        .unwrap()
        .get_mut("")
        .unwrap()
        .age = Some(Age::live(start));
    ws.track_activity_times_at(0, start);
    let time = |ws: &Workspace, now| {
        ws.activity_entries_at("front", now)
            .remove(0)
            .activity
            .unwrap()
            .elapsed
            .unwrap()
            .text
    };
    assert_eq!(time(&ws, later), "2h");
    let blocked = StatusRecord {
        state: ProgramState::Blocked,
        live: true,
        age: Some(Age::live(later)),
        ..Default::default()
    };
    ws.hosts[0]
        .model
        .activity
        .get_mut("front")
        .unwrap()
        .insert("other".into(), blocked);
    ws.track_activity_times_at(0, later);
    assert_eq!(time(&ws, later), "now");
    let end = later + Duration::from_secs(240);
    ws.hosts[0]
        .model
        .activity
        .get_mut("front")
        .unwrap()
        .remove("other");
    ws.track_activity_times_at(0, end);
    assert_eq!(
        time(&ws, end),
        "now",
        "returning to Working is not two hours of Working group"
    );
    ws.hosts[0]
        .model
        .activity
        .get_mut("front")
        .unwrap()
        .get_mut("")
        .unwrap()
        .progress = Some(99);
    ws.track_activity_times_at(0, end + Duration::from_secs(240));
    assert_eq!(time(&ws, end + Duration::from_secs(240)), "4m");
    ws.hosts[0].model.activity_epoch += 1;
    ws.hosts[0]
        .model
        .activity
        .get_mut("front")
        .unwrap()
        .get_mut("")
        .unwrap()
        .age = None;
    ws.track_activity_times_at(0, end);
    assert!(
        ws.activity_entries_at("front", end)[0]
            .activity
            .as_ref()
            .unwrap()
            .elapsed
            .is_none()
    );
}

#[test]
fn minute_refresh_is_popup_local_and_preserves_scroll_selection_and_receipts() {
    use std::time::{Duration, Instant};
    let _families = rune::text::with_families("Inter", "Inter");
    let (mut ws, rx) = fixture();
    let (sender, terminal) = flume::unbounded();
    framed_terminal(&mut ws, "front", &sender, b"$ ");
    let note = target(&ws, 1, "s", "done");
    ws.notified_tabs.insert(note);
    let start = Instant::now();
    assert_eq!(ws.refresh_activity_at(start), (false, None));
    ws.show_activity();
    let mut rt = runtime_at(ws, (640., 480.));
    settle(&mut rt);
    rt.update(|s, _| {
        s.panel.as_mut().unwrap().activity_refresh = None;
        assert_eq!(
            s.refresh_activity_at(start),
            (false, Some(Duration::from_secs(60)))
        );
        assert_eq!(
            s.refresh_activity_at(start + Duration::from_secs(59)),
            (false, Some(Duration::from_secs(1)))
        );
    });
    // A genuine nonzero scroll, not just an assertion that zero stayed zero.
    let scroll = super::pane_verb_tests::rect(&rt, "results-scroll");
    rt.wheel(scroll[0] + 20., scroll[1] + 20., -180.);
    settle(&mut rt);
    let before = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("entry:0"))
        .unwrap()
        .scrolled;
    assert_ne!(before[1], 0.);
    let selected = rt.state.activity_selection();
    let done = rt.state.unseen_done.clone();
    let bells = rt.state.unseen_bells.clone();
    let notes = rt.state.notified_tabs.clone();
    for r in &rx {
        r.drain();
    }
    terminal.drain();
    rt.update(|s, _| {
        assert_eq!(
            s.refresh_activity_at(start + Duration::from_secs(60)),
            (true, Some(Duration::from_secs(60)))
        );
        assert!(!s.panel.as_ref().unwrap().reveal);
    });
    settle(&mut rt);
    assert_eq!(
        rt.elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some("entry:0"))
            .unwrap()
            .scrolled,
        before
    );
    assert_eq!(rt.state.activity_selection(), selected);
    assert_eq!(rt.state.unseen_done, done);
    assert_eq!(rt.state.unseen_bells, bells);
    assert_eq!(rt.state.notified_tabs, notes);
    assert!(rx.iter().all(|r| r.is_empty()) && terminal.is_empty());
    rt.state.panel = None;
    assert_eq!(
        rt.state
            .refresh_activity_at(start + Duration::from_secs(120)),
        (false, None)
    );
    rt.state
        .open_panel(PanelKind::Picker(false), "Commands", &[("", "Filter", "")]);
    assert_eq!(
        rt.state
            .refresh_activity_at(start + Duration::from_secs(180)),
        (false, None)
    );
}

#[test]
fn unknown_time_and_latch_without_done_record_have_no_time_element() {
    let _families = rune::text::with_families("Inter", "Inter");
    let (mut ws, _rx) = fixture();
    ws.hosts[0].model.activity.remove("done");
    ws.show_activity();
    assert!(
        ws.activity_entries_at("", std::time::Instant::now())
            .iter()
            .all(|e| e.activity.as_ref().unwrap().elapsed.is_none())
    );
    let mut rt = runtime(ws);
    settle(&mut rt);
    assert!(!rt.elements().iter().any(|e| {
        e.text
            .as_ref()
            .is_some_and(|t| t == "now" || t.starts_with("reported "))
    }));
}

#[test]
fn a_tab_that_left_a_state_and_returned_between_wakes_shows_the_records_younger_time() {
    use rex_rune::connection::activity::time::Age;
    use std::time::{Duration, Instant};
    let (mut ws, _rx) = fixture();
    let start = Instant::now();
    let record = |ws: &mut Workspace, age| {
        ws.hosts[0]
            .model
            .activity
            .get_mut("blocked")
            .unwrap()
            .get_mut("")
            .unwrap()
            .age = age;
    };
    let time = |ws: &Workspace, now| {
        ws.activity_entries_at("blocked", now)
            .remove(0)
            .activity
            .unwrap()
            .elapsed
            .map(|e| e.text)
    };
    record(&mut ws, Some(Age::live(start)));
    ws.track_activity_times_at(0, start);
    let later = start + Duration::from_secs(40 * 60);
    ws.track_activity_times_at(0, later);
    assert_eq!(time(&ws, later).as_deref(), Some("40m"));
    // Model wakes coalesce: the agent was answered, worked, and asked again before the
    // app looked. The tab's state is unchanged, but the record's clock restarted.
    record(&mut ws, Some(Age::live(later)));
    ws.track_activity_times_at(0, later);
    assert_eq!(
        time(&ws, later).as_deref(),
        Some("now"),
        "a question asked seconds ago is not forty minutes old"
    );
    // And a record whose age is unknown leaves nothing to show, however old the tab's
    // own clock is.
    record(&mut ws, None);
    ws.track_activity_times_at(0, later);
    assert_eq!(time(&ws, later), None);
}

#[test]
fn one_pane_with_an_unknown_age_keeps_its_tab_from_showing_a_siblings_time() {
    use rex_rune::connection::activity::time::Age;
    use std::time::{Duration, Instant};
    let (mut ws, _rx) = fixture();
    let start = Instant::now();
    let now = start + Duration::from_secs(600);
    let records = ws.hosts[0].model.activity.get_mut("front").unwrap();
    records.get_mut("").unwrap().age = Some(Age::live(start));
    ws.track_activity_times_at(0, now);
    let time = |ws: &Workspace| {
        ws.activity_entries_at("front", now)
            .remove(0)
            .activity
            .unwrap()
            .elapsed
            .map(|e| e.text)
    };
    assert_eq!(time(&ws).as_deref(), Some("10m"));
    // A second Working report in the same tab, with no usable time: it may be older.
    ws.hosts[0].model.activity.get_mut("front").unwrap().insert(
        "other".into(),
        StatusRecord {
            state: ProgramState::Working,
            live: true,
            age: None,
            ..Default::default()
        },
    );
    ws.track_activity_times_at(0, now);
    assert_eq!(time(&ws), None);
}

#[test]
fn the_app_poll_drives_the_refresh_only_while_the_list_is_open() {
    use std::time::{Duration, Instant};
    let (mut ws, rx) = fixture();
    let mut tx = rune::Tx::default();
    // The poll's own result folds in other timers, so the deadline is what is checked.
    // Closed: nothing is armed, and another panel is not this one.
    crate::app::poll(&mut ws, 0., &mut tx);
    assert!(ws.panel.is_none());
    ws.picker(false);
    crate::app::poll(&mut ws, 0., &mut tx);
    assert_eq!(ws.panel.as_ref().unwrap().activity_refresh, None);
    // Open: the poll arms the deadline about a minute out.
    ws.show_activity();
    crate::app::poll(&mut ws, 0., &mut tx);
    let armed = ws.panel.as_ref().unwrap().activity_refresh.expect("armed");
    assert!(armed > Instant::now() + Duration::from_secs(50));
    // Not due: the deadline stays where it is.
    crate::app::poll(&mut ws, 0., &mut tx);
    assert_eq!(ws.panel.as_ref().unwrap().activity_refresh, Some(armed));
    // Due: the poll takes it and arms the next one.
    let due = Instant::now() - Duration::from_secs(1);
    ws.panel.as_mut().unwrap().activity_refresh = Some(due);
    assert!(
        crate::app::poll(&mut ws, 0., &mut tx),
        "a redraw is asked for"
    );
    assert!(ws.panel.as_ref().unwrap().activity_refresh.unwrap() > Instant::now());
    assert!(rx.iter().all(|rx| rx.is_empty()), "a redraw sends nothing");
}
