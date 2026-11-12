use super::remote_tests::{code, pending, reply};
use super::*;
use crate::app::actions;
use rex_rune::connection::perform::Outcome;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

fn two_hosts() -> (Workspace, flume::Receiver<HostCommand>) {
    let mut ws = workspace_hosts(2);
    let (source, _) = tabbed_workspace("single");
    ws.hosts[0].model = source.model().clone();
    ws.hosts[1].model = source.model().clone();
    ws.settings.hosts[1].remote_control = true;
    let rx = capture_host(&mut ws, 1);
    (ws, rx)
}
fn navigation_id(rx: &flume::Receiver<HostCommand>) -> u64 {
    let commands: Vec<_> = rx.try_iter().collect();
    assert!(
        !commands
            .iter()
            .any(|c| matches!(c, HostCommand::PerformReply { .. }))
    );
    commands
        .into_iter()
        .find_map(|c| match c {
            HostCommand::Navigate { id, deadline, .. } => {
                assert!(deadline <= Instant::now() + Duration::from_secs(5));
                Some(id)
            }
            _ => None,
        })
        .expect("one navigation command")
}

#[test]
fn session_select_switches_only_to_requesting_host_and_waits_for_acknowledgement() {
    let (mut ws, rx) = two_hosts();
    ws.remote_perform(
        1,
        pending(
            "session.select",
            json!({"session_id":"s", "window_id":"split"}),
        ),
        &mut rune::Tx::default(),
    );
    assert_eq!(ws.active, 1);
    assert!(
        ws.notices.is_empty(),
        "do not announce before acknowledgement"
    );
    let id = navigation_id(&rx);
    assert!(!ws.poll_remote_navigation(Instant::now()));
    assert!(rx.is_empty());
    ws.hosts[1]
        .model
        .views
        .get_mut("s")
        .unwrap()
        .active_window_id = Some("split".into());
    ws.hosts[1].model.navigation_result = Some((id, true));
    assert!(ws.poll_remote_navigation(Instant::now()));
    assert_eq!(reply(&rx), Outcome::Performed);
    assert_eq!(ws.notices.len(), 1);
    assert!(
        ws.notices[0]
            .text
            .contains("test-1: rex (uid 1000) performed session.select")
    );
    ws.observe_navigation();
    assert!(!ws.poll_remote_navigation(Instant::now()));
    assert!(rx.is_empty());
    ws.navigate_history(-1, &mut rune::Tx::default());
    assert_eq!(
        ws.active, 0,
        "remote navigation is in the same history as local navigation"
    );
}

#[test]
fn session_select_rejects_unknown_targets_and_mismatched_envelope_without_navigation() {
    let (mut ws, rx) = two_hosts();
    for args in [
        json!({"session_id":"missing"}),
        json!({"session_id":"s","window_id":"missing"}),
    ] {
        ws.remote_perform(1, pending("session.select", args), &mut rune::Tx::default());
        code(reply(&rx), "not_found");
        assert_eq!(ws.active, 0);
    }
    for args in [
        Value::Null,
        json!({}),
        json!({"session_id":""}),
        json!({"session_id":4}),
        json!({"session_id":"s","window_id":null}),
        json!({"session_id":"s","window_id":""}),
        json!({"session_id":"s","extra":true}),
    ] {
        ws.remote_perform(1, pending("session.select", args), &mut rune::Tx::default());
        code(reply(&rx), "bad_request");
        assert_eq!(ws.active, 0);
    }
    let mut p = pending("session.select", json!({"session_id":"s"}));
    p.request.session_id = Some("different".into());
    ws.remote_perform(1, p, &mut rune::Tx::default());
    code(reply(&rx), "refused");
    assert_eq!(ws.active, 0);
}

#[test]
fn session_select_failure_expiry_interrupt_and_disconnect_never_report_success() {
    for failure in [
        "failed",
        "expired",
        "interrupt",
        "disconnect",
        "reconnect",
        "missing-tab",
    ] {
        let (mut ws, rx) = two_hosts();
        if failure == "missing-tab" {
            ws.hosts[1].model.views.clear();
        }
        ws.remote_perform(
            1,
            pending(
                "session.select",
                json!({"session_id":"s","window_id":"split"}),
            ),
            &mut rune::Tx::default(),
        );
        let id = navigation_id(&rx);
        let mut now = Instant::now();
        match failure {
            "failed" => ws.hosts[1].model.navigation_result = Some((id, false)),
            "expired" => now += Duration::from_secs(6),
            "interrupt" => ws.navigation.cancel(),
            "disconnect" => ws.hosts[1].model.connected = false,
            "reconnect" => ws.hosts[1].model.connection_epoch += 1,
            "missing-tab" => {
                ws.hosts[1].model.views.insert(
                    "s".into(),
                    serde_json::from_value(json!({"revision":2,"windows":[]})).unwrap(),
                );
                ws.hosts[1].model.navigation_result = Some((id, false));
            }
            _ => unreachable!(),
        }
        assert!(ws.poll_remote_navigation(now));
        if matches!(failure, "disconnect" | "reconnect") {
            assert!(rx.is_empty());
        } else {
            code(
                reply(&rx),
                if failure == "missing-tab" {
                    "not_found"
                } else {
                    "internal"
                },
            );
        }
        assert!(ws.notices.is_empty());
        assert!(!ws.poll_remote_navigation(now));
        assert!(rx.is_empty());
    }
}

#[test]
fn session_select_refuses_while_typing_and_with_panel_open_even_on_another_host() {
    let (mut ws, rx) = two_hosts();
    for panel in [false, true] {
        if panel {
            ws.perform(Intent::Settings, &mut rune::Tx::default());
        } else {
            ws.last_keypress = Some(Instant::now());
        }
        rx.drain();
        ws.remote_perform(
            1,
            pending("session.select", json!({"session_id":"s"})),
            &mut rune::Tx::default(),
        );
        code(reply(&rx), "refused");
        assert_eq!(ws.active, 0);
        ws.last_keypress = None;
    }
}

#[test]
fn local_session_select_ignores_consent_and_goto_minus_one_means_last_tab() {
    let (mut ws, rx) = tabbed_workspace("single");
    assert!(!ws.settings.hosts[0].remote_control);
    assert!(actions::run(
        &mut ws,
        "session.select",
        &json!({"session_id":"s"}),
        &mut rune::Tx::default()
    ));
    assert!(
        rx.try_iter()
            .any(|c| matches!(c, HostCommand::Navigate { session, .. } if session == "s"))
    );
    ws.navigation.cancel();
    assert!(actions::run(
        &mut ws,
        "client.tab.goto",
        &json!({"index":-1}),
        &mut rune::Tx::default()
    ));
    assert!(rx.try_iter().any(
        |c| matches!(c, HostCommand::Navigate { window:Some(window), .. } if window == "split")
    ));
    assert!(
        !actions::find("client.tab.goto")
            .unwrap()
            .accepts(&json!({"index":0}))
    );
    assert!(
        !actions::find("client.tab.goto")
            .unwrap()
            .accepts(&json!({"index":-2}))
    );
    let action = actions::declarations()
        .into_iter()
        .find(|a| a.name == "session.select")
        .unwrap();
    let schema = action.args.unwrap();
    assert_eq!(schema["required"], json!(["session_id"]));
    assert_eq!(schema["additionalProperties"], false);
    assert!(schema["properties"].get("window_id").is_some());
}
