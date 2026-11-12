//! Issue #44: declarations, exact argument validation, reuse of existing dispatch and local-only policy.
use super::*;
use crate::app::{actions, compatibility, panel::SettingsSection};
use rex_rune::{
    connection::Command,
    keymap::{Overrides, Resolved},
};
use rexrs::api::client::{Binding, KeymapResponse, Mode};
use serde_json::{Value, json};
use std::sync::Arc;

fn run(ws: &mut Workspace, name: &str, args: Value) {
    assert!(
        actions::run(ws, name, &args, &mut rune::Tx::default()),
        "{name}: {args}"
    );
}

#[test]
fn issue_44_schemas_accept_examples_and_reject_invalid_args_before_dispatch() {
    for (name, args) in [
        ("client.config.reload", Value::Null),
        ("pane.resize", json!({"direction":"left","amount":1})),
        ("pane.resize", json!({"direction":"down","amount":15})),
        ("pane.resize", json!({"direction":"up"})),
        ("pane.move", json!({"direction":"right"})),
        ("pane.move_to_tab", json!({"window_id":"single"})),
        ("client.tab.move", json!({"direction":"backward"})),
        ("client.tab.move", json!({"direction":"forward"})),
        ("client.mode.enter", json!({"name":"resize","once":true})),
        ("client.find.next", json!({"when":"searching"})),
        ("client.find.previous", json!({"when":"searching"})),
    ] {
        let spec = actions::find(name).unwrap();
        assert!(spec.accepts(&args), "{name}: {args}");
        assert!(matches!(spec.remote(), actions::Remote::Never(_)));
    }
    for (name, args) in [
        ("pane.resize", Value::Null),
        ("pane.resize", json!({"direction":"left","amount":0})),
        ("pane.resize", json!({"direction":"left","amount":-1})),
        ("pane.resize", json!({"direction":"left","amount":"1"})),
        ("pane.resize", json!({"direction":"diagonal"})),
        ("pane.resize", json!({"direction":"left","typo":1})),
        ("pane.focus.left", json!({"direction":"right"})),
        ("pane.move_to_tab", json!({"window_id":""})),
        ("client.mode.enter", json!({"name":"Resize"})),
        ("client.mode.enter", json!({"name":""})),
        ("client.mode.enter", json!({"name":"resize","once":1})),
        ("client.find.next", json!({"when":"switching"})),
        ("client.config.reload", json!({"unexpected":true})),
    ] {
        let (mut ws, rx) = tabbed_workspace("split");
        assert!(
            !actions::run(&mut ws, name, &args, &mut rune::Tx::default()),
            "{name}: {args}"
        );
        assert!(requests(&rx).is_empty());
    }
    let resize = actions::declarations()
        .into_iter()
        .find(|a| a.name == "pane.resize")
        .unwrap()
        .args
        .unwrap();
    assert_eq!(resize["required"], json!(["direction"]));
    assert_eq!(resize["properties"]["amount"]["type"], "number");
    assert_eq!(resize["additionalProperties"], false);
}

#[test]
fn every_added_name_is_remote_denied_including_aliases_of_allowed_actions() {
    let names = [
        "client.config.reload",
        "client.config.copy_path",
        "client.prompt.previous",
        "client.prompt.next",
        "pane.prompt_previous",
        "pane.prompt_next",
        "pane.resize",
        "pane.balance",
        "pane.rename",
        "pane.close_others",
        "pane.move",
        "pane.move_to_tab",
        "pane.move_to_new_tab",
        "pane.focus.left",
        "pane.focus.right",
        "pane.focus.up",
        "pane.focus.down",
        "pane.split.left",
        "pane.split.right",
        "pane.split.up",
        "pane.split.down",
        "pane.split.auto",
        "pane.move.left",
        "pane.move.right",
        "pane.move.up",
        "pane.move.down",
        "client.find.open",
        "client.find.next",
        "client.find.previous",
        "client.tab.move.backward",
        "client.tab.move.forward",
        "client.host.switch",
        "client.host.add",
        "client.host.remove",
        "client.theme.change",
        "client.mode.enter",
        "client.mode.exit",
        "client.mode.exit_all",
        "client.scroll.page_up",
        "client.scroll.page_down",
    ];
    for name in names {
        assert!(
            matches!(
                actions::find(name).unwrap().remote(),
                actions::Remote::Never(_)
            ),
            "{name}"
        );
        let (mut ws, rx) = tabbed_workspace("split");
        ws.settings.hosts[0].remote_control = true;
        ws.hosts[0].model.connected = true;
        ws.remote_perform(
            0,
            super::remote_tests::pending(name, Value::Null),
            &mut rune::Tx::default(),
        );
        super::remote_tests::code(super::remote_tests::reply(&rx), "not_supported");
        assert!(ws.panel.is_none());
    }
}

#[test]
fn direction_aliases_send_the_same_payload_as_the_original_handler() {
    for name in [
        "pane.focus.left",
        "pane.focus.right",
        "pane.focus.up",
        "pane.focus.down",
        "pane.split.left",
        "pane.split.right",
        "pane.split.up",
        "pane.split.down",
        "pane.split.auto",
        "pane.move.left",
        "pane.move.right",
        "pane.move.up",
        "pane.move.down",
        "client.tab.move.backward",
        "client.tab.move.forward",
    ] {
        let (action, args) = compatibility::alias(name).unwrap();
        let (mut original, rx1) = tabbed_workspace("split");
        let (mut alias, rx2) = tabbed_workspace("split");
        run(&mut original, action, args);
        run(&mut alias, name, Value::Null);
        assert_eq!(requests(&rx1), requests(&rx2), "{name}");
    }
}

#[test]
fn resize_uses_server_ratio_and_amount_and_balance_does_not_invent_a_layout() {
    let (mut ws, rx) = tabbed_workspace("split");
    run(
        &mut ws,
        "pane.resize",
        json!({"direction":"right","amount":1}),
    );
    let calls = requests(&rx);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "session.resize_pane");
    assert_eq!(
        calls[0].2,
        json!({"session_id":"s","direction":"horizontal","before_block_id":"block:b","after_block_id":"block:c","ratio":0.51})
    );
    run(
        &mut ws,
        "pane.resize",
        json!({"direction":"right","amount":15}),
    );
    assert_eq!(
        requests(&rx)[0].2["ratio"],
        0.65,
        "from server 0.5, not locally accumulated 0.51"
    );
    run(&mut ws, "pane.resize", json!({"direction":"up"}));
    assert!(
        requests(&rx).is_empty(),
        "no border above or below this pane"
    );
    run(&mut ws, "pane.balance", Value::Null);
    assert_eq!(requests(&rx)[0].2["ratio"], 0.5);
    run(&mut ws, "pane.move.right", Value::Null);
    assert_eq!(
        requests(&rx)[0].2,
        json!({"session_id":"s","block_id":"block:b","other_block_id":"block:c"})
    );
}

#[test]
fn balance_sends_one_third_for_a_three_column_chain_and_keeps_the_server_view() {
    let (mut ws, rx) = tabbed_workspace("split");
    let layer = &mut ws.hosts[0].model.views.get_mut("s").unwrap().windows[1].layers[0];
    layer.layout = Some(
        serde_json::from_value(json!({"split":{
            "direction":"horizontal", "ratio":0.5,
            "before":{"block_id":"block:b"},
            "after":{"split":{"direction":"horizontal", "ratio":0.5,
                "before":{"block_id":"block:c"}, "after":{"block_id":"block:d"}}}
        }}))
        .unwrap(),
    );
    let original = serde_json::to_value(ws.model().window()).unwrap();
    run(&mut ws, "pane.balance", Value::Null);
    let calls = requests(&rx);
    assert_eq!(calls.len(), 2);
    assert!(calls.iter().all(|c| c.0 == "session.resize_pane"));
    assert_eq!(
        calls[0].2,
        json!({"session_id":"s", "direction":"horizontal",
        "before_block_id":"block:b", "after_block_id":"block:c", "ratio":1. / 3.})
    );
    assert_eq!(calls[1].2["ratio"], 0.5);
    assert_eq!(serde_json::to_value(ws.model().window()).unwrap(), original);
}

#[test]
fn move_to_tab_rejects_current_foreign_and_missing_windows() {
    let (mut ws, rx) = tabbed_workspace("split");
    let mut foreign = ws.hosts[0].model.views["s"].clone();
    foreign.windows[0].window_id = Some("foreign".into());
    ws.hosts[0]
        .model
        .views
        .insert("other-session".into(), foreign);
    for id in ["split", "foreign", "missing"] {
        run(&mut ws, "pane.move_to_tab", json!({"window_id":id}));
        assert!(requests(&rx).is_empty(), "{id}");
    }
    run(&mut ws, "pane.move_to_tab", json!({"window_id":"single"}));
    let calls = requests(&rx);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "session.move_block");
    assert_eq!(calls[0].2["session_id"], "s");
}

#[test]
fn close_others_does_not_close_panes_added_after_confirmation_opened() {
    let (mut ws, rx) = tabbed_workspace("split");
    run(&mut ws, "pane.close_others", Value::Null);
    let layer = &mut ws.hosts[0].model.views.get_mut("s").unwrap().windows[1].layers[0];
    layer
        .blocks
        .push(serde_json::from_value(json!({"block_id":"block:d"})).unwrap());
    ws.submit(&mut rune::Tx::default());
    let calls = requests(&rx);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].2["block_id"], "block:c");
}

#[test]
fn rename_close_others_and_move_to_tab_use_existing_pane_and_confirmation_paths() {
    let (mut ws, rx) = tabbed_workspace("split");
    run(&mut ws, "pane.rename", Value::Null);
    assert!(matches!(
        ws.panel.as_ref().unwrap().kind,
        PanelKind::Form(Form::RenamePane(_))
    ));
    assert!(requests(&rx).is_empty());
    ws.close_panel(&mut rune::Tx::default());
    run(&mut ws, "pane.close_others", Value::Null);
    assert!(matches!(
        ws.panel.as_ref().unwrap().kind,
        PanelKind::Confirm { .. }
    ));
    assert!(requests(&rx).is_empty(), "asks before closing");
    ws.submit(&mut rune::Tx::default());
    let calls = requests(&rx);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "block.close");
    assert_eq!(calls[0].2["block_id"], "block:c");
    run(&mut ws, "pane.move_to_tab", Value::Null);
    assert!(matches!(
        ws.panel.as_ref().unwrap().kind,
        PanelKind::MovePane
    ));
    assert_eq!(ws.entries().len(), 1);
    ws.submit(&mut rune::Tx::default());
    let calls = requests(&rx);
    assert_eq!(calls[0].0, "session.move_block");
    assert_eq!(calls[0].2["block_id"], "block:b");
    assert_eq!(calls[0].2["anchor_block_id"], "block:a");
    assert!(ws.panel.is_none());
}

#[test]
fn reload_reads_local_overrides_and_requests_the_active_hosts_lua_reload_without_a_session() {
    let mut ws = workspace();
    let rx = capture_host(&mut ws, 0);
    let path = rex_rune::settings::shortcuts::Shortcuts::path(&ws.settings_path);
    std::fs::write(
        &path,
        r#"[{"keys":"ctrl+alt+q","action":"client.config.reload"}]"#,
    )
    .unwrap();
    let generation = ws.keymap_generation;
    run(&mut ws, "client.config.reload", Value::Null);
    assert!(ws.keymap_generation > generation);
    assert!(
        ws.effective_bindings()
            .iter()
            .any(|b| b.binding.action == "client.config.reload")
    );
    let calls: Vec<_> = rx
        .try_iter()
        .filter_map(|c| match c {
            HostCommand::Request { method, payload } => Some((method, payload)),
            _ => None,
        })
        .collect();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "server.config_reload");
    assert_eq!(
        calls[0].1,
        json!({}),
        "server method must not acquire a session_id"
    );
    std::fs::remove_file(path).ok();
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn unsupported_actions_are_explicit_in_problems_and_dispatch_without_side_effects() {
    let name = "pane.move_to_new_tab";
    let overrides = Overrides::parse(&json!([{"keys":"ctrl+alt+q","action":name}]));
    let map = actions::resolve(&overrides);
    assert_eq!(map.diagnostics.len(), 1);
    assert!(
        map.diagnostics[0]
            .message
            .contains("recognized but not supported")
    );
    assert!(!map.diagnostics[0].message.contains("unknown action"));
    let (mut ws, rx) = tabbed_workspace("split");
    run(&mut ws, name, Value::Null);
    assert!(requests(&rx).is_empty());
    assert!(ws.panel.is_none());
}

fn with_modes(ws: &mut Workspace, exclusive: bool) {
    let reply = KeymapResponse {
        bindings: vec![
            Binding::new(["ctrl+r"], "client.mode.enter").args(json!({"name":"resize"})),
            Binding::new(["right"], "pane.resize")
                .args(json!({"direction":"right","amount":1}))
                .mode("resize"),
            Binding::new(["g", "g"], "pane.balance").mode("resize"),
            Binding::new(["q"], "client.mode.exit_all").mode("resize"),
            Binding::new(["x"], "pane.close"),
        ],
        modes: vec![Mode {
            name: "resize".into(),
            exclusive,
            blocked: vec![Some(vec!["x".into()])],
        }],
        ..Default::default()
    };
    ws.hosts[0].model.keymap = Some(Arc::new(rex_rune::connection::keymap::Snapshot {
        generation: ws.keymap_generation,
        resolved: Resolved::from_server(&reply, &Overrides::default()),
        reply,
    }));
    ws.hosts[0].model.connected = true;
}

#[test]
fn mode_keys_stay_inert_outside_the_mode_and_explicit_escape_overrides_exit() {
    let (mut ws, rx) = tabbed_workspace("split");
    let (sender, sent) = flume::unbounded();
    super::pane_verb_tests::framed_terminal(&mut ws, "block:b", &sender, b"prompt");
    with_modes(&mut ws, false);
    let snapshot = Arc::make_mut(ws.hosts[0].model.keymap.as_mut().unwrap());
    snapshot.reply.bindings.push(
        Binding::new(["escape"], "pane.resize")
            .args(json!({"direction":"right","amount":1}))
            .mode("resize"),
    );
    snapshot.resolved = Resolved::from_server(&snapshot.reply, &Overrides::default());
    let mut rt = runtime(ws);
    press(&mut rt, Key::Right);
    assert!(requests(&rx).is_empty());
    assert!(
        !sent_keys(&sent).is_empty(),
        "ordinary Right reaches the terminal"
    );
    rt.update(|s, tx| {
        actions::run(s, "client.mode.enter", &json!({"name":"resize"}), tx);
    });
    press(&mut rt, Key::Escape);
    assert_eq!(requests(&rx)[0].0, "session.resize_pane");
    assert_eq!(
        rt.state.bindings.modes.len(),
        1,
        "explicit Escape keeps the mode"
    );
    assert!(sent_keys(&sent).is_empty());
}

#[test]
fn modes_route_keys_sequences_blocked_keys_exclusive_misses_and_escape() {
    for exclusive in [false, true] {
        let (mut ws, rx) = tabbed_workspace("split");
        let (sender, sent) = flume::unbounded();
        super::pane_verb_tests::framed_terminal(&mut ws, "block:b", &sender, b"prompt");
        with_modes(&mut ws, exclusive);
        let mut rt = runtime(ws);
        rt.update(|s, tx| {
            actions::run(s, "client.mode.enter", &json!({"name":"resize"}), tx);
        });
        press(&mut rt, Key::Right);
        assert_eq!(requests(&rx)[0].0, "session.resize_pane");
        press(&mut rt, Key::Text("x".into()));
        assert!(
            rt.state.panel.is_none(),
            "mode blocks the ordinary close binding"
        );
        assert!(sent_keys(&sent).is_empty());
        press(&mut rt, Key::Text("z".into()));
        assert_eq!(sent_keys(&sent).is_empty(), exclusive);
        press(&mut rt, Key::Text("g".into()));
        assert_eq!(rt.state.bindings.sequence.depth, 1);
        press(&mut rt, Key::Text("g".into()));
        assert_eq!(requests(&rx)[0].0, "session.resize_pane");
        press(&mut rt, Key::Escape);
        assert!(rt.state.bindings.modes.is_empty());
        assert!(
            sent_keys(&sent).is_empty(),
            "consumed mode keys do not leak releases"
        );
    }
}

#[test]
fn modes_nest_once_pops_only_the_mode_that_supplied_a_binding_and_reload_clears_them() {
    let (mut ws, rx) = tabbed_workspace("split");
    with_modes(&mut ws, false);
    let mut rt = runtime(ws);
    rt.update(|s, tx| {
        actions::run(s, "client.mode.enter", &json!({"name":"resize"}), tx);
        actions::run(
            s,
            "client.mode.enter",
            &json!({"name":"resize","once":true}),
            tx,
        );
    });
    assert_eq!(rt.state.bindings.modes.len(), 2);
    press(&mut rt, Key::Right);
    assert_eq!(rt.state.bindings.modes.len(), 1);
    assert_eq!(requests(&rx).len(), 1);
    press(&mut rt, Key::Text("q".into()));
    assert!(rt.state.bindings.modes.is_empty());
    rt.update(|s, tx| {
        actions::run(s, "client.mode.enter", &json!({"name":"resize"}), tx);
    });
    rt.state.keymap_generation += 1;
    press(&mut rt, Key::Right);
    assert!(rt.state.bindings.modes.is_empty());
}

#[test]
fn prompt_actions_and_roadmap_aliases_dispatch_only_viewport_commands() {
    let (mut ws, rx) = tabbed_workspace("split");
    let (sender, sent) = flume::unbounded();
    super::pane_verb_tests::framed_terminal(&mut ws, "block:b", &sender, b"hello");
    let before = serde_json::to_value(ws.model().window()).unwrap();
    for (name, older) in [
        ("client.prompt.previous", true),
        ("client.prompt.next", false),
        ("pane.prompt_previous", true),
        ("pane.prompt_next", false),
    ] {
        assert_eq!(compatibility::unsupported(name), None);
        let overrides = Overrides::parse(&json!([{"keys":"ctrl+alt+q","action":name}]));
        assert!(actions::resolve(&overrides).diagnostics.is_empty());
        let reply = KeymapResponse {
            bindings: vec![Binding::new(["ctrl+up"], name)],
            ..Default::default()
        };
        let mut resolved = Resolved::from_server(&reply, &Overrides::default());
        actions::diagnose_compatibility(&mut resolved);
        assert!(resolved.diagnostics.is_empty());
        assert!(actions::find(name).unwrap().repeats());
        run(&mut ws, name, Value::Null);
        assert!(matches!(sent.try_recv(), Ok(HostCommand::Terminal {
            block, command: Command::PromptStep(direction), ..
        }) if block == "block:b" && direction == older));
        assert!(sent.is_empty(), "no PTY keys or ownership request");
    }
    assert!(requests(&rx).is_empty());
    assert_eq!(serde_json::to_value(ws.model().window()).unwrap(), before);
    for (id, action, hint) in [
        (
            "pane.prompt_previous",
            "client.prompt.previous",
            "Ctrl Shift Up",
        ),
        ("pane.prompt_next", "client.prompt.next", "Ctrl Shift Down"),
    ] {
        let row = ws.commands().into_iter().find(|c| c.id == id).unwrap();
        assert!(row.entry.enabled);
        assert!(matches!(row.entry.action, Intent::ClientAction(name, _) if name == action));
        assert_eq!(
            crate::app::keycaps::spell(&ws.shortcut_chords(action, &Value::Null)),
            hint
        );
    }
    let (mut empty, _) = tabbed_workspace("split");
    run(&mut empty, "client.prompt.previous", Value::Null);
    assert!(
        !empty
            .commands()
            .into_iter()
            .find(|c| c.id == "pane.prompt_previous")
            .unwrap()
            .entry
            .enabled
    );
}

#[test]
fn scroll_pages_use_viewport_rows_find_aliases_and_host_switch_are_real() {
    let (mut ws, rx) = tabbed_workspace("split");
    let (sender, sent) = flume::unbounded();
    super::pane_verb_tests::framed_terminal(&mut ws, "block:b", &sender, b"hello");
    let cell = crate::app::term::metrics(ws.settings.font_size).cell;
    ws.active_terminal_mut().unwrap().rect = [0., 0., cell.0 * 40., cell.1 * 4.];
    run(&mut ws, "client.scroll.page_up", Value::Null);
    run(&mut ws, "client.scroll.page_down", Value::Null);
    let scrolls: Vec<_> = sent
        .try_iter()
        .filter_map(|c| match c {
            HostCommand::Terminal {
                command: Command::Scroll(n),
                ..
            } => Some(n),
            _ => None,
        })
        .collect();
    assert_eq!(scrolls, [-4, 4]);
    run(&mut ws, "client.find.next", json!({"when":"searching"}));
    assert!(ws.active_terminal().unwrap().search.is_none());
    run(&mut ws, "client.find.open", Value::Null);
    assert!(ws.active_terminal().unwrap().search.is_some());
    run(&mut ws, "client.host.switch", Value::Null);
    assert!(matches!(
        ws.panel.as_ref().unwrap().kind,
        PanelKind::SwitchHost
    ));
    ws.submit(&mut rune::Tx::default());
    assert!(
        requests(&rx).is_empty(),
        "switching host does not create a session"
    );
}

#[test]
fn once_mode_suppresses_held_repeats_and_a_persistent_top_mode_does_not_pop_an_outer_once() {
    use rune::{PhysicalInput, PhysicalKeyCode as Code};
    let (mut ws, rx) = tabbed_workspace("split");
    let (sender, sent) = flume::unbounded();
    super::pane_verb_tests::framed_terminal(&mut ws, "block:b", &sender, b"hello");
    with_modes(&mut ws, false);
    let mut rt = runtime(ws);
    rt.update(|s, tx| {
        actions::run(
            s,
            "client.mode.enter",
            &json!({"name":"resize","once":true}),
            tx,
        );
    });
    for (pressed, repeat) in [(true, false), (true, true), (false, false)] {
        rt.physical_key_input(
            PhysicalInput::Key {
                key: Code::ArrowRight,
                mods: Mods::default(),
                pressed,
                repeat,
            },
            KeyEvent {
                key: Key::Right,
                mods: Mods::default(),
            },
            None,
        );
        rt.frame(1. / 60.);
    }
    assert_eq!(requests(&rx).len(), 1);
    assert!(rt.state.bindings.modes.is_empty());
    assert!(sent_keys(&sent).is_empty());
    rt.update(|s, tx| {
        actions::run(
            s,
            "client.mode.enter",
            &json!({"name":"resize","once":true}),
            tx,
        );
        actions::run(s, "client.mode.enter", &json!({"name":"resize"}), tx);
    });
    press(&mut rt, Key::Right);
    assert_eq!(rt.state.bindings.modes.len(), 2);
}

#[test]
fn copy_config_path_uses_the_active_hosts_reported_path_and_alias_hints_follow_rebinding() {
    let (mut ws, _) = tabbed_workspace("split");
    with_modes(&mut ws, false);
    Arc::make_mut(ws.hosts[0].model.keymap.as_mut().unwrap())
        .reply
        .config
        .path = "/remote/rex/init.lua".into();
    let mut rt = runtime(ws);
    let clipboard = rune::clipboard::memory();
    clipboard.set_text("unchanged");
    rt.use_clipboard(&clipboard);
    rt.update(|s, tx| {
        actions::run(s, "client.config.copy_path", &Value::Null, tx);
    });
    assert_eq!(
        clipboard.text().as_deref().map(String::as_str),
        Some("/remote/rex/init.lua")
    );
    rt.state.hosts[0].model.keymap = None;
    rt.state.local_keymap = actions::resolve(&Overrides::parse(&json!([
        {"keys":"ctrl+alt+left","action":null},
        {"keys":"ctrl+h","action":"pane.focus.left"}
    ])));
    assert_eq!(
        crate::app::keycaps::spell(
            &rt.state
                .shortcut_chords("pane.focus", &json!({"direction":"left"}))
        ),
        "Ctrl H"
    );
}

#[test]
fn server_bindings_get_the_same_unsupported_diagnostic_exactly_once() {
    let reply = KeymapResponse {
        bindings: vec![Binding::new(["ctrl+up"], "pane.move_to_new_tab")],
        ..Default::default()
    };
    let mut map = Resolved::from_server(&reply, &Overrides::default());
    actions::diagnose_compatibility(&mut map);
    actions::diagnose_compatibility(&mut map);
    assert_eq!(map.diagnostics.len(), 1);
    assert!(
        map.diagnostics[0]
            .message
            .contains("recognized but not supported")
    );
}

#[test]
fn shortcuts_show_unbound_supported_actions_without_pretending_unsupported_ones_work() {
    let mut ws = workspace();
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    ws.open_section(SettingsSection::Shortcuts);
    let mut rt = runtime_at(ws, (1200., 900.));
    for _ in 0..30 {
        rt.frame(1. / 60.);
    }
    let words: Vec<_> = rt.elements().into_iter().filter_map(|e| e.text).collect();
    assert!(words.iter().any(|s| s == "Not set"));
    assert!(has(&rt, "shortcut:client.config.reload"));
    assert!(has(&rt, "shortcut:pane.resize:left"));
    assert!(!has(&rt, "shortcut:pane.move_to_new_tab"));
    if let Some(mut camera) = super::pane_verb_tests::Camera::new() {
        camera.snap(&mut rt, "issue44-shortcuts");
    }
    std::fs::remove_file(&rt.state.settings_path).ok();
}
