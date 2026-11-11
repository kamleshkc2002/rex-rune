use super::*;
use crate::app::actions::{self, Remote};
use rex_linux::connection::perform::{Outcome, Pending, Request};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

pub(super) fn pending(action: &str, args: Value) -> Pending {
    Pending { epoch: 0, received: Instant::now(), request: serde_json::from_value::<Request>(json!({
        "call_id":"1", "action":action, "args":args,
        "requester":{"client_id":"cli", "info":{"name":"rex"}, "principal":{"display":"uid 1000"}}
    })).unwrap() }
}
pub(super) fn reply(rx: &flume::Receiver<HostCommand>) -> Outcome {
    let replies: Vec<_> = rx
        .try_iter()
        .filter_map(|c| match c {
            HostCommand::PerformReply { outcome, .. } => Some(outcome),
            _ => None,
        })
        .collect();
    assert_eq!(replies.len(), 1);
    replies.into_iter().next().unwrap()
}
pub(super) fn code(outcome: Outcome, expected: &str) {
    assert!(
        matches!(outcome, Outcome::Error { code, .. } if code == expected),
        "{outcome:?}"
    );
}

#[test]
fn remote_set_is_exact_and_every_denial_is_described() {
    let mut allowed: Vec<_> = actions::SPECS
        .iter()
        .filter(|s| matches!(s.remote(), Remote::View { .. }))
        .map(|s| s.name)
        .collect();
    allowed.sort();
    assert_eq!(
        allowed,
        [
            "client.sidebar.toggle",
            "client.tab.goto",
            "client.tab.next",
            "client.tab.previous",
            "pane.focus",
            "pane.scroll",
            "pane.zoom",
            "session.select"
        ]
    );
    for action in actions::declarations() {
        if matches!(
            actions::find(&action.name).unwrap().remote(),
            Remote::Never(_)
        ) {
            assert!(
                action
                    .description
                    .unwrap()
                    .contains("Not available remotely")
            );
        }
    }
}

#[test]
fn remote_default_off_and_all_hazards_are_denied_without_side_effects() {
    let (mut ws, rx) = tabbed_workspace("single");
    let sidebar = ws.sidebar.is_open();
    for enabled in [false, true] {
        ws.settings.hosts[0].remote_control = enabled;
        for spec in actions::SPECS {
            if enabled && matches!(spec.remote(), Remote::View { .. }) {
                continue;
            }
            ws.remote_perform(0, pending(spec.name, Value::Null), &mut rune::Tx::default());
            let commands: Vec<_> = rx.try_iter().collect();
            assert_eq!(commands.len(), 1, "{} must send only a reply", spec.name);
            let HostCommand::PerformReply { outcome, .. } = commands.into_iter().next().unwrap()
            else {
                panic!()
            };
            code(
                outcome,
                if matches!(spec.remote(), Remote::Never(_)) {
                    "not_supported"
                } else {
                    "refused"
                },
            );
            assert!(ws.panel.is_none());
            assert_eq!(ws.sidebar.is_open(), sidebar);
        }
    }
    assert_eq!(ws.notices.len(), 1, "off notice once per host per launch");
    assert!(ws.notices[0].text.contains("rex (uid 1000)"));
}

#[test]
fn unknown_and_download_remote_actions_never_dispatch() {
    let (mut ws, rx) = tabbed_workspace("single");
    let sidebar = ws.sidebar.is_open();
    for enabled in [false, true] {
        ws.settings.hosts[0].remote_control = enabled;
        // API 13 adds a server method, not permission to download through client.perform.
        for action in ["com.superlogical.terminal.download_begin", "future.action"] {
            ws.remote_perform(
                0,
                pending(action, json!({"path":"/private/file"})),
                &mut rune::Tx::default(),
            );
            let commands: Vec<_> = rx.try_iter().collect();
            assert_eq!(commands.len(), 1, "must send only a refusal");
            let HostCommand::PerformReply { outcome, .. } = commands.into_iter().next().unwrap()
            else {
                panic!()
            };
            code(outcome, "not_supported");
            assert!(ws.panel.is_none());
            assert_eq!(ws.sidebar.is_open(), sidebar);
        }
    }
}

#[test]
fn remote_guards_and_argument_checks_do_not_dispatch() {
    let (mut ws, rx) = tabbed_workspace("single");
    ws.settings.hosts[0].remote_control = true;
    for (action, args) in [
        ("client.sidebar.toggle", json!({"extra":1})),
        ("pane.focus", json!({"direction":"sideways"})),
        ("pane.scroll", json!({"lines":1.5})),
        ("client.tab.goto", json!({"index":0})),
        ("client.tab.goto", json!({"index":99})),
    ] {
        ws.remote_perform(0, pending(action, args), &mut rune::Tx::default());
        code(reply(&rx), "bad_request");
    }
    let action = "client.tab.next";
    for field in ["session", "block", "typing", "panel", "expired", "epoch"] {
        let mut p = pending(action, Value::Null);
        match field {
            "session" => p.request.session_id = Some("other".into()),
            "block" => p.request.block_id = Some("other".into()),
            "typing" => ws.last_keypress = Some(Instant::now()),
            "panel" => ws.perform(Intent::Settings, &mut rune::Tx::default()),
            "expired" => p.received -= Duration::from_secs(6),
            "epoch" => p.epoch = 1,
            _ => unreachable!(),
        }
        rx.drain();
        ws.remote_perform(0, p, &mut rune::Tx::default());
        if field == "epoch" {
            assert!(rx.is_empty());
        } else {
            code(
                reply(&rx),
                if field == "expired" {
                    "internal"
                } else {
                    "refused"
                },
            );
        }
        ws.last_keypress = None;
        ws.panel = None;
    }
}

#[test]
fn remote_view_actions_use_the_local_handlers_and_announce_requester() {
    for (name, args, method) in [
        ("client.tab.next", Value::Null, "session.focus_next_window"),
        (
            "client.tab.previous",
            Value::Null,
            "session.focus_previous_window",
        ),
        (
            "pane.focus",
            json!({"direction":"right"}),
            "session.focus_direction",
        ),
        ("pane.zoom", Value::Null, "session.zoom_block"),
        ("client.tab.goto", json!({"index":2}), "navigate"),
        ("client.sidebar.toggle", Value::Null, "sidebar"),
    ] {
        let (mut ws, rx) = tabbed_workspace("single");
        ws.settings.hosts[0].remote_control = true;
        ws.remote_perform(0, pending(name, args), &mut rune::Tx::default());
        let mut performed = 0;
        let mut dispatched = method == "sidebar" && ws.sidebar.is_open();
        for command in rx.try_iter() {
            match command {
                HostCommand::PerformReply { outcome, .. } => {
                    assert_eq!(outcome, Outcome::Performed);
                    performed += 1;
                }
                HostCommand::Request { method: m, .. } => dispatched |= m == method,
                HostCommand::Navigate { window, .. } => {
                    dispatched |= method == "navigate" && window.as_deref() == Some("split")
                }
                _ => {}
            }
        }
        assert!(dispatched, "{name}");
        assert_eq!(performed, 1);
        assert_eq!(ws.notices.len(), 1);
        assert!(
            ws.notices[0]
                .text
                .contains(&format!("test-0: rex (uid 1000) performed {name}"))
        );
        assert!(!ws.notices[0].sticky);
    }
}

#[test]
fn remote_scroll_is_local_and_requester_principal_is_display_only() {
    let (mut ws, rx) = tabbed_workspace("single");
    ws.settings.hosts[0].remote_control = true;
    let sender = ws.hosts[0].connection.commands.clone();
    live_terminal(&mut ws, "block:a", &sender);
    for principal in [
        json!({"kind":"anonymous"}),
        json!({"kind":"tailscale","login_name":"agent@example.test"}),
    ] {
        let mut p = pending("pane.scroll", json!({"lines":-10}));
        p.request.requester.principal = principal;
        ws.remote_perform(0, p, &mut rune::Tx::default());
        assert!(matches!(
            rx.recv().unwrap(),
            HostCommand::Terminal {
                command: rex_linux::connection::Command::Scroll(-10),
                ..
            }
        ));
        assert_eq!(reply(&rx), Outcome::Performed);
    }
}

#[test]
fn remote_actions_cannot_take_focus_from_a_search_field() {
    let (mut ws, rx) = tabbed_workspace("single");
    ws.settings.hosts[0].remote_control = true;
    let (sender, terminal) = flume::unbounded();
    live_terminal(&mut ws, "block:a", &sender);
    let key = ws.term_key("block:a");
    let mut rt = runtime(ws);
    ctrl_shift(&mut rt, 'f');
    for c in ["a", "b", "c"] {
        press(&mut rt, Key::Text(c.into()));
    }
    let focus = focused(&rt);
    assert!(focus.as_ref().unwrap().contains("search-field:block:a"));
    // Even after a pause, every remote action must leave the field alone.
    rt.state.last_keypress = Some(Instant::now() - Duration::from_secs(2));
    rx.drain();
    for (action, args) in [
        ("client.tab.next", Value::Null),
        ("client.sidebar.toggle", Value::Null),
        ("pane.scroll", json!({"lines": 1})),
    ] {
        rt.update(|s, tx| s.remote_perform(0, pending(action, args), tx));
        code(reply(&rx), "refused");
        assert_eq!(focused(&rt), focus);
    }
    press(&mut rt, Key::Text("d".into()));
    assert_eq!(
        rt.state.terminals[&key].search.as_ref().unwrap().text,
        "abcd"
    );
    assert!(
        sent_keys(&terminal).is_empty(),
        "no field input reaches a shell"
    );
}

#[test]
fn search_edits_and_consumed_keys_record_activity() {
    let (mut ws, _rx) = tabbed_workspace("single");
    let (sender, _) = flume::unbounded();
    live_terminal(&mut ws, "block:a", &sender);
    let mut rt = runtime(ws);
    ctrl_shift(&mut rt, 'f');
    rt.state.last_keypress = None;
    press(&mut rt, Key::Text("a".into()));
    assert!(rt.state.last_keypress.is_some());
    rt.state.last_keypress = None;
    press(&mut rt, Key::Left);
    assert!(
        rt.state.last_keypress.is_some(),
        "cursor movement is activity too"
    );
    rt.state.last_keypress = None;
    // IME edits the TextInput directly, exercising on_change without search_key.
    rt.ime(rune::Ime::Commit("b".into()));
    rt.frame(1. / 60.);
    assert!(rt.state.last_keypress.is_some());
}

#[test]
fn remote_actions_preserve_pending_key_sequences_even_after_a_pause() {
    let (mut ws, rx) = tabbed_workspace("single");
    ws.settings.hosts[0].remote_control = true;
    ws.local_keymap = actions::resolve(&rex_linux::keymap::Overrides::parse(&json!([
        {"keys": "ctrl+b>c", "action": "client.sidebar.toggle"}
    ])));
    let mut rt = runtime(ws);
    rt.key(KeyEvent {
        key: Key::Command('b'),
        mods: Mods {
            ctrl: true,
            cmd: true,
            ..Mods::default()
        },
    });
    rt.frame(1. / 60.);
    assert_eq!(rt.state.bindings.sequence.depth, 1);
    assert!(
        rt.state.last_keypress.is_some(),
        "consumed prefix counts as activity"
    );
    rt.state.last_keypress = Some(Instant::now() - Duration::from_secs(2));
    rx.drain();
    for action in ["client.tab.next", "client.sidebar.toggle"] {
        rt.update(|s, tx| s.remote_perform(0, pending(action, Value::Null), tx));
        code(reply(&rx), "refused");
        assert_eq!(rt.state.bindings.sequence.depth, 1);
    }
    let sidebar = rt.state.sidebar.is_open();
    rt.state.last_keypress = None;
    press(&mut rt, Key::Text("c".into()));
    assert_eq!(rt.state.bindings.sequence.depth, 0);
    assert_ne!(
        rt.state.sidebar.is_open(),
        sidebar,
        "the original tail still runs"
    );
    assert!(
        rt.state.last_keypress.is_some(),
        "consumed shortcut counts as activity"
    );
}

#[test]
fn remote_notices_evict_oldest_non_sticky_notice_before_warnings() {
    let (mut ws, rx) = tabbed_workspace("single");
    ws.settings.hosts[0].remote_control = true;
    ws.notify("sticky warning");
    ws.inform("old information");
    ws.notify("second warning");
    ws.remote_perform(
        0,
        pending("client.sidebar.toggle", Value::Null),
        &mut rune::Tx::default(),
    );
    assert_eq!(reply(&rx), Outcome::Performed);
    assert_eq!(ws.notices.len(), 3);
    assert_eq!(ws.notices[0].text, "sticky warning");
    assert_eq!(ws.notices[1].text, "second warning");
    assert!(
        ws.notices[2]
            .text
            .contains("performed client.sidebar.toggle")
    );
    ws.notify("third warning");
    ws.inform("new information");
    assert!(
        ws.notices.iter().all(|n| n.sticky),
        "information cannot displace three warnings"
    );
    ws.notify("fourth warning");
    assert_eq!(ws.notices.len(), 3);
    assert_eq!(
        ws.notices[0].text, "second warning",
        "all-sticky overflow evicts the oldest"
    );
}

#[test]
fn client_label_form_trims_and_rejects_empty_without_freezing_the_default() {
    let mut ws = workspace();
    assert!(ws.settings.client_label.is_none());
    let mut tx = rune::Tx::default();
    ws.perform(Intent::ClientLabel, &mut tx);
    assert_eq!(
        ws.panel.as_ref().unwrap().inputs[0].text,
        rex_linux::settings::hostname()
    );
    ws.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("  \n  ");
    ws.submit(&mut tx);
    assert!(ws.panel.as_ref().unwrap().error.is_some());
    assert!(ws.settings.client_label.is_none());
    ws.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("  My laptop  ");
    ws.submit(&mut tx);
    assert_eq!(ws.settings.client_label.as_deref(), Some("My laptop"));
    assert_eq!(
        Settings::load(&ws.settings_path)
            .unwrap()
            .client_label
            .as_deref(),
        Some("My laptop")
    );
    std::fs::remove_file(&ws.settings_path).unwrap();
}

#[test]
fn remote_control_switch_is_per_host_and_survives_save() {
    let mut ws = workspace_hosts(2);
    let rx = capture_host(&mut ws, 1);
    ws.hosts[1].model.connected = true;
    ws.settings.hosts[1].remote_control = true;
    ws.remote_perform(
        1,
        pending("client.sidebar.toggle", Value::Null),
        &mut rune::Tx::default(),
    );
    code(reply(&rx), "refused");
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    ws.settings_section = crate::app::panel::SettingsSection::Hosts;
    let mut rt = runtime(ws);
    assert!(has(&rt, "remote-control:0"));
    assert!(has(&rt, "remote-control:1"));
    // The root eases to the window's size; the switches sit at the dialog's trailing edge.
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    click(&mut rt, "remote-control:0");
    assert!(Settings::load(&rt.state.settings_path).unwrap().hosts[0].remote_control);
    let endpoint = rt.state.settings.hosts[0].endpoint.clone();
    rt.state.edit_host(0, "renamed".into(), endpoint, None);
    assert!(rt.state.settings.hosts[0].remote_control);
    rt.state.edit_host(
        0,
        "changed".into(),
        "unix:///nonexistent/changed.sock".into(),
        None,
    );
    assert!(!rt.state.settings.hosts[0].remote_control);
    assert!(rt.state.settings.hosts[1].remote_control);
}
