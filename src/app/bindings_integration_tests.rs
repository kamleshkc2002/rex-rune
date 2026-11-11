use super::*;
use crate::app::{actions, bindings::Input};
use rex_linux::{connection::Command, keymap::Overrides};
use rune::{PhysicalInput, PhysicalKeyCode as Code};
use serde_json::{Value, json};

fn overrides(ws: &mut Workspace, value: Value) {
    ws.settings.shortcut_file.overrides = Overrides::parse(&value);
    ws.local_keymap = actions::resolve(&ws.settings.shortcut_file.overrides);
    ws.keymap_generation += 1;
}
fn key(rt: &mut Runtime<Workspace>, code: Code, event: KeyEvent, pressed: bool, repeat: bool) {
    rt.physical_key_input(
        PhysicalInput::Key {
            key: code,
            mods: event.mods,
            pressed,
            repeat,
        },
        event,
        None,
    );
    rt.frame(1. / 60.);
}
fn ctrl(key: Key) -> KeyEvent {
    KeyEvent {
        key,
        mods: Mods {
            ctrl: true,
            ..Mods::default()
        },
    }
}

fn server_map(
    ws: &mut Workspace,
    bindings: Vec<rexrs::api::client::Binding>,
    actions: Vec<rexrs::api::client::InspectResponseAction>,
) {
    let reply = rexrs::api::client::KeymapResponse {
        bindings,
        actions,
        ..Default::default()
    };
    ws.hosts[0].model.connected = true;
    ws.hosts[0].model.keymap = Some(std::sync::Arc::new(
        rex_linux::connection::keymap::Snapshot {
            generation: ws.keymap_generation,
            resolved: rex_linux::keymap::Resolved::from_server(
                &reply,
                &ws.settings.shortcut_file.overrides,
            ),
            reply,
        },
    ));
    ws.hosts[0].model.keymap_revision += 1;
}

#[test]
fn forward_wins_over_local_and_sequence_echoes_original_prefix_with_release() {
    use rexrs::api::client::Binding;
    let (mut ws, rx) = tabbed_workspace("single");
    server_map(
        &mut ws,
        vec![
            Binding::new(["ctrl+shift+t"], "client.key.forward"),
            Binding::new(["ctrl+shift+t"], "client.tab.new"),
            Binding::new(["Control+X", "ctrl+s"], "client.key.forward"),
        ],
        vec![],
    );
    let (sender, sent) = flume::unbounded();
    live_terminal(&mut ws, "block:a", &sender);
    let mut rt = runtime(ws);
    let ev = KeyEvent {
        key: Key::Command('t'),
        mods: Mods {
            ctrl: true,
            shift: true,
            ..Mods::default()
        },
    };
    key(&mut rt, Code::KeyT, ev.clone(), true, false);
    key(&mut rt, Code::KeyT, ev, false, false);
    assert_eq!(sent_keys(&sent).len(), 2);
    assert!(requests(&rx).is_empty());
    key(&mut rt, Code::KeyX, ctrl(Key::Command('x')), true, false);
    key(&mut rt, Code::KeyX, ctrl(Key::Command('x')), false, false);
    key(&mut rt, Code::KeyS, ctrl(Key::Command('s')), true, false);
    key(&mut rt, Code::KeyS, ctrl(Key::Command('s')), true, true);
    let calls: Vec<_> = sent.try_iter().collect();
    assert_eq!(calls.len(), 1);
    assert!(
        matches!(&calls[0],HostCommand::Terminal{command:Command::KeyWithPrefix{prefix,key},..} if prefix==&["Control+X"] && key["action"]=="press")
    );
    key(&mut rt, Code::KeyS, ctrl(Key::Command('s')), false, false);
    assert_eq!(sent_keys(&sent)[0]["action"], "release");
}

#[test]
fn result_actions_are_classified_and_absent_args_are_omitted() {
    use rexrs::api::client::{Binding, InspectResponseAction};
    let (mut ws, rx) = tabbed_workspace("single");
    server_map(
        &mut ws,
        vec![
            Binding::new(["ctrl+j"], "client.mac.only"),
            Binding::new(["ctrl+j"], "clear_screen"),
        ],
        vec![InspectResponseAction::new("clear_screen").kind("lua")],
    );
    let mut rt = runtime(ws);
    key(&mut rt, Code::KeyJ, ctrl(Key::Command('j')), true, false);
    let request = rx
        .try_iter()
        .find_map(|c| match c {
            HostCommand::Action { request, .. } => Some(request),
            _ => None,
        })
        .unwrap();
    let value = serde_json::to_value(request).unwrap();
    assert_eq!(value["action"], "clear_screen");
    assert_eq!(value["origin"], "key");
    assert_eq!(value["session_id"], "s");
    assert_eq!(value["block_id"], "block:a");
    assert!(
        value.get("args").is_none()
            && value.get("servers").is_none()
            && value.get("server").is_none()
    );
}

#[test]
fn returned_client_actions_are_taken_once_in_order_unknown_names_ignored() {
    let mut ws = workspace();
    capture_host(&mut ws, 0);
    {
        let mut model = ws.hosts[0].connection.model.lock().unwrap();
        for action in [
            "client.zoom.in",
            "client.zoom.reset",
            "client.not_known",
            "client.sidebar.toggle",
        ] {
            model
                .client_actions
                .push(rex_linux::connection::actions::Queued {
                    generation: 0,
                    session: None,
                    block: None,
                    action: rexrs::api::client::ClientAction {
                        action: action.into(),
                        args: None,
                    },
                });
        }
    }
    let before = ws.sidebar.is_open();
    ws.sync(0, &mut rune::Tx::default());
    assert_eq!(ws.settings.ui_scale, 1.);
    assert_ne!(ws.sidebar.is_open(), before);
    assert!(
        ws.hosts[0]
            .connection
            .model
            .lock()
            .unwrap()
            .client_actions
            .is_empty()
    );
    assert!(ws.hosts[0].model.client_actions.is_empty());
    ws.sync(0, &mut rune::Tx::default());
    assert_ne!(ws.sidebar.is_open(), before);
}

#[test]
fn returned_actions_allow_changed_pane_focus_but_guard_workspace_and_panels() {
    for changed in ["pane", "session", "generation", "panel", "popup", "host"] {
        let (mut ws, _rx) = tabbed_workspace("split");
        let mut model = ws.hosts[0].model.clone();
        assert_eq!(ws.focused_block().as_deref(), Some("block:b"));
        model.views.get_mut("s").unwrap().windows[1].focused_block_id = Some("block:c".into());
        model
            .client_actions
            .push(rex_linux::connection::actions::Queued {
                generation: ws.keymap_generation,
                session: Some("s".into()),
                block: Some("block:b".into()),
                action: rexrs::api::client::ClientAction {
                    action: "client.sidebar.toggle".into(),
                    args: None,
                },
            });
        match changed {
            "session" => model.selected = Some("different".into()),
            "generation" => ws.keymap_generation += 1,
            "panel" => ws.perform(Intent::Settings, &mut rune::Tx::default()),
            "popup" => ws.picker(false),
            "host" => {
                ws.settings.hosts.push(ws.settings.hosts[0].clone());
                ws.connect_host(1, false);
                ws.active = 1;
            }
            _ => {}
        }
        *ws.hosts[0].connection.model.lock().unwrap() = model;
        let before = ws.sidebar.is_open();
        ws.sync(0, &mut rune::Tx::default());
        assert_eq!(
            ws.sidebar.is_open() != before,
            changed == "pane",
            "{changed}"
        );
        assert!(
            ws.hosts[0]
                .connection
                .model
                .lock()
                .unwrap()
                .client_actions
                .is_empty()
        );
        std::fs::remove_file(&ws.settings_path).ok();
    }
}

#[test]
fn settings_shows_effective_sequences_sources_and_pending_chip_cancels() {
    let mut ws = workspace();
    let path = rex_linux::settings::shortcuts::Shortcuts::path(&ws.settings_path);
    std::fs::write(
        &path,
        r#"[{"keys":"ctrl+k","action":null},{"keys":"ctrl+b>c","action":"client.tab.new"}]"#,
    )
    .unwrap();
    ws.settings.shortcut_file = rex_linux::settings::shortcuts::Shortcuts::load(&ws.settings_path);
    overrides(
        &mut ws,
        json!([
            {"keys":"ctrl+k","action":null},
            {"keys":"ctrl+b>c","action":"client.tab.new"}
        ]),
    );
    let mut rt = runtime(ws);
    key(&mut rt, Code::KeyB, ctrl(Key::Command('b')), true, false);
    assert!(has(&rt, "pending-sequence"));
    press(&mut rt, Key::Escape);
    assert_eq!(rt.state.bindings.sequence.depth, 0);
    rt.frame(0.5);
    assert!(!has(&rt, "pending-sequence"));
    rt.update(|s, tx| {
        s.perform(Intent::Settings, tx);
        s.open_section(crate::app::panel::SettingsSection::Shortcuts);
    });
    rt.frame(0.);
    let words: Vec<_> = rt.elements().into_iter().filter_map(|e| e.text).collect();
    assert!(words.iter().any(|s| s == "then"));
    assert!(words.iter().any(|s| s == "shortcuts.json"));
    assert!(words.iter().any(|s| s == "Switch session"));
    assert!(words.iter().any(|s| s == "Unbound"));
    assert!(
        rt.state
            .shortcut_chords("session.switch", &Value::Null)
            .is_empty()
    );
    assert!(!has(&rt, "shortcut-problem-0"));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn commands_details_follow_rebinding_unbinding_and_argument_matches() {
    let (mut ws, _) = tabbed_workspace("single");
    overrides(
        &mut ws,
        json!([
            {"keys":"ctrl+shift+t","action":null},
            {"keys":"ctrl+b>c","action":"client.tab.new"},
            {"keys":"ctrl+shift+d","action":null},
            {"keys":"ctrl+j","action":"pane.split","args":{"direction":"right"}}
        ]),
    );
    ws.picker(false);
    let entries = ws.entries();
    assert_eq!(
        entries
            .iter()
            .find(|e| e.label == "New tab")
            .unwrap()
            .detail,
        "Ctrl B, C"
    );
    assert!(matches!(
        entries
            .iter()
            .find(|e| e.label == "New tab")
            .unwrap()
            .action,
        Intent::ClientAction("client.tab.new", _)
    ));
    assert_eq!(
        entries
            .iter()
            .find(|e| e.label == "Split right")
            .unwrap()
            .detail,
        "Ctrl J"
    );
    assert_eq!(
        entries
            .iter()
            .find(|e| e.label == "Split down")
            .unwrap()
            .detail,
        "Ctrl Alt D"
    );
    assert_eq!(
        entries
            .iter()
            .find(|e| e.label == "Back in history")
            .unwrap()
            .detail,
        "Mouse Back"
    );
    overrides(&mut ws, json!([{"keys":"ctrl+shift+t","action":null}]));
    assert_eq!(
        ws.entries()
            .iter()
            .find(|e| e.label == "New tab")
            .unwrap()
            .detail,
        ""
    );
}

#[test]
fn effective_map_swaps_by_host_and_generation_and_preserves_problems() {
    let mut ws = workspace_hosts(2);
    let overrides = Overrides::parse(&json!([{"keys":"ctrl+b>","action":"client.settings"}]));
    for (index, action) in [(0, "client.sidebar.toggle"), (1, "client.search")] {
        let reply = rexrs::api::client::KeymapResponse {
            bindings: vec![rexrs::api::client::Binding::new(["ctrl+j"], action)],
            ..Default::default()
        };
        ws.hosts[index].model.connected = true;
        ws.hosts[index].model.keymap = Some(std::sync::Arc::new(
            rex_linux::connection::keymap::Snapshot {
                generation: 0,
                resolved: rex_linux::keymap::Resolved::from_server(&reply, &overrides),
                reply,
            },
        ));
    }
    assert_eq!(
        ws.effective_keymap().bindings[0].binding.action,
        "client.sidebar.toggle"
    );
    assert_eq!(ws.effective_keymap().diagnostics.len(), 1);
    ws.active = 1;
    assert_eq!(
        ws.effective_keymap().bindings[0].binding.action,
        "client.search"
    );
    ws.keymap_generation = 1;
    assert_eq!(ws.effective_keymap(), &ws.local_keymap);
    ws.keymap_generation = 0;
    ws.hosts[1].model.connected = false;
    assert_eq!(ws.effective_keymap(), &ws.local_keymap);
}

#[test]
fn server_config_path_marks_only_missing_files() {
    for exists in [false, true] {
        let mut ws = workspace();
        server_map(&mut ws, vec![], vec![]);
        let snapshot = std::sync::Arc::make_mut(ws.hosts[0].model.keymap.as_mut().unwrap());
        snapshot.reply.config.path = "/server/init.lua".into();
        snapshot.reply.config.exists = exists;
        ws.perform(Intent::Settings, &mut rune::Tx::default());
        let mut rt = runtime(ws);
        click(&mut rt, "settings-section:shortcuts");
        let elements = rt.elements();
        let path = elements
            .iter()
            .find(|e| e.key.as_deref() == Some("server-shortcuts-path"))
            .unwrap();
        assert_eq!(
            path.text.as_deref(),
            Some(if exists {
                "Server: /server/init.lua"
            } else {
                "Server: /server/init.lua (not created)"
            })
        );
    }
}

#[test]
fn changed_shortcuts_reach_offline_hosts_but_unchanged_touches_send_nothing() {
    use rex_linux::settings::shortcuts::Shortcuts;
    let mut ws = workspace_hosts(2);
    let receivers = [capture_host(&mut ws, 0), capture_host(&mut ws, 1)];
    ws.hosts[0].model.connected = true;
    ws.hosts[1].model.connected = false;
    let path = Shortcuts::path(&ws.settings_path);
    std::fs::write(&path, "[]").unwrap();
    ws.settings.shortcut_file = Shortcuts::load(&ws.settings_path);
    let old = ws.keymap_generation;
    std::fs::write(&path, r#"[{"keys":"ctrl+j","action":"client.tab.new"}]"#).unwrap();
    ws.reload_shortcuts();
    assert_eq!(ws.keymap_generation, old + 1);
    for rx in &receivers {
        let commands: Vec<_> = rx.try_iter().collect();
        assert_eq!(commands.len(), 1);
        let HostCommand::Keymap(declaration) = &commands[0] else {
            panic!("expected keymap declaration");
        };
        assert_eq!(declaration.generation, old + 1);
        assert_eq!(declaration.overrides, ws.settings.shortcut_file.overrides);
    }
    let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(modified + std::time::Duration::from_secs(1))
        .unwrap();
    assert!(
        ws.settings.shortcut_file.changed(),
        "exercise a genuine timestamp change"
    );
    ws.reload_shortcuts();
    assert_eq!(ws.keymap_generation, old + 1);
    assert!(!ws.settings.shortcut_file.changed());
    assert!(receivers.iter().all(|rx| rx.is_empty()));
    ws.reload_shortcuts();
    assert!(receivers.iter().all(|rx| rx.is_empty()));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn shortcut_import_notice_uses_custom_config_filenames() {
    let ws = workspace();
    std::fs::write(
        &ws.settings_path,
        r#"{"keybindings":[{"key":"ctrl+j","action":"client.tab.new"}]}"#,
    )
    .unwrap();
    let loaded = relaunch(&ws);
    let from = ws.settings_path.file_name().unwrap().to_string_lossy();
    let path = &loaded.settings.shortcut_file.path;
    let to = path.file_name().unwrap().to_string_lossy();
    assert!(
        loaded
            .notices
            .iter()
            .any(|n| n.text == format!("Moved 1 custom shortcut(s) from {from} to {to}."))
    );
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(&ws.settings_path).unwrap();
}

#[test]
fn a_bad_shortcut_entry_notifies_once_across_two_connected_hosts() {
    let mut ws = workspace_hosts(2);
    let path = rex_linux::settings::shortcuts::Shortcuts::path(&ws.settings_path);
    std::fs::write(&path, r#"[{"keys":"ctrl+b>","action":"client.settings"}]"#).unwrap();
    let _receivers = [capture_host(&mut ws, 0), capture_host(&mut ws, 1)];
    ws.reload_shortcuts();
    assert_eq!(ws.settings.shortcut_file.overrides.diagnostics.len(), 1);
    assert_eq!(ws.notices.len(), 1);
    assert!(ws.notices[0].text.contains(&path.display().to_string()));
    for index in 0..2 {
        {
            let mut model = ws.hosts[index].connection.model.lock().unwrap();
            let reply = rexrs::api::client::KeymapResponse::default();
            model.connected = true;
            model.keymap = Some(std::sync::Arc::new(
                rex_linux::connection::keymap::Snapshot {
                    generation: ws.keymap_generation,
                    resolved: rex_linux::keymap::Resolved::from_server(
                        &reply,
                        &ws.settings.shortcut_file.overrides,
                    ),
                    reply,
                },
            ));
        }
        ws.sync(index, &mut rune::Tx::default());
        assert_eq!(
            ws.notices.len(),
            1,
            "host {index} duplicated the file notice"
        );
    }
    assert_eq!(ws.effective_keymap().diagnostics.len(), 1);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn settings_diagnostic_sets_toast_once_per_host_never_file_or_linux() {
    let mut ws = workspace();
    capture_host(&mut ws, 0);
    let problem = |source: &str| rexrs::api::client::Diagnostic {
        source: Some(source.into()),
        message: "test diagnostic".into(),
        severity: "warning".into(),
        ..Default::default()
    };
    for (diagnostics, expected) in [
        (vec![problem("linux"), problem("init.lua:1")], 0),
        (vec![problem("settings")], 1),
        (vec![], 1),
        (vec![problem("settings")], 1),
    ] {
        let reply = rexrs::api::client::KeymapResponse {
            diagnostics,
            ..Default::default()
        };
        {
            let mut model = ws.hosts[0].connection.model.lock().unwrap();
            model.connected = true;
            model.keymap = Some(std::sync::Arc::new(
                rex_linux::connection::keymap::Snapshot {
                    generation: 0,
                    resolved: rex_linux::keymap::Resolved::from_server(
                        &reply,
                        &Overrides::default(),
                    ),
                    reply,
                },
            ));
        }
        ws.sync(0, &mut rune::Tx::default());
        assert_eq!(ws.notices.len(), expected);
    }
}

#[test]
fn unbound_ctrl_tab_press_and_release_are_forwarded() {
    let (mut ws, _) = tabbed_workspace("single");
    overrides(&mut ws, json!([{"keys":"ctrl+tab","action":null}]));
    let (sender, sent) = flume::unbounded();
    live_terminal(&mut ws, "block:a", &sender);
    let mut rt = runtime(ws);
    key(&mut rt, Code::Tab, ctrl(Key::Tab), true, false);
    assert!(
        rt.state.forwarded.contains(
            &Input {
                event: &ctrl(Key::Tab),
                code: Some(Code::Tab),
                base: None,
                repeat: false
            }
            .identity()
        )
    );
    key(&mut rt, Code::Tab, ctrl(Key::Tab), false, false);
    let keys = sent_keys(&sent);
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0]["action"], "press");
    assert_eq!(keys[1]["action"], "release");
    assert!(rt.state.forwarded.is_empty());
}

#[test]
fn prefixes_repeats_and_failed_tails_do_not_leak_or_retry() {
    let (mut ws, requests_rx) = tabbed_workspace("single");
    overrides(
        &mut ws,
        json!([
            {"keys":"ctrl+b>c","action":"client.tab.new"},
            {"keys":"x","action":"client.settings"}
        ]),
    );
    let (sender, sent) = flume::unbounded();
    live_terminal(&mut ws, "block:a", &sender);
    let mut rt = runtime(ws);
    key(&mut rt, Code::KeyB, ctrl(Key::Command('b')), true, false);
    assert_eq!(rt.state.bindings.sequence.depth, 1);
    key(&mut rt, Code::KeyB, ctrl(Key::Command('b')), true, true);
    key(&mut rt, Code::KeyB, ctrl(Key::Command('b')), false, false);
    assert!(sent_keys(&sent).is_empty());
    key(
        &mut rt,
        Code::KeyX,
        KeyEvent {
            key: Key::Text("x".into()),
            mods: Mods::default(),
        },
        true,
        false,
    );
    assert!(
        rt.state.panel.is_none(),
        "failed tail is not retried as Settings"
    );
    assert_eq!(sent_keys(&sent).len(), 1);
    key(&mut rt, Code::KeyB, ctrl(Key::Command('b')), true, false);
    key(
        &mut rt,
        Code::KeyC,
        KeyEvent {
            key: Key::Text("c".into()),
            mods: Mods::default(),
        },
        true,
        false,
    );
    key(
        &mut rt,
        Code::KeyC,
        KeyEvent {
            key: Key::Text("c".into()),
            mods: Mods::default(),
        },
        true,
        true,
    );
    assert_eq!(
        requests(&requests_rx)
            .iter()
            .filter(|r| r.0 == "session.new_window")
            .count(),
        1
    );
    assert!(sent_keys(&sent).is_empty());
}

#[test]
fn nonrepeating_logical_keys_and_consumed_release_survive_dialog_close() {
    let (mut ws, _) = tabbed_workspace("single");
    overrides(
        &mut ws,
        json!([{"keys":"ctrl+j","action":"client.sidebar.toggle"}]),
    );
    let mut rt = runtime(ws);
    let ev = ctrl(Key::Command('j'));
    rt.key(ev.clone());
    rt.frame(0.);
    let open = rt.state.sidebar.is_open();
    rt.repeat_key(ev);
    rt.frame(0.);
    assert_eq!(
        rt.state.sidebar.is_open(),
        open,
        "logical-only repeats are suppressed too"
    );
    key(&mut rt, Code::Comma, ctrl(Key::Command(',')), true, false);
    assert!(rt.state.panel.is_some());
    rt.update(|s, tx| s.close_panel(tx));
    rt.frame(0.);
    key(&mut rt, Code::Comma, ctrl(Key::Command(',')), false, false);
    assert!(
        rt.state
            .bindings
            .consumed
            .keys()
            .all(|id| *id != crate::app::bindings::Identity::Physical(Code::Comma))
    );
}

#[test]
fn release_inside_settings_does_not_swallow_the_next_comma_hold() {
    let (mut ws, _) = tabbed_workspace("single");
    let (sender, sent) = flume::unbounded();
    live_terminal(&mut ws, "block:a", &sender);
    let mut rt = runtime(ws);
    key(&mut rt, Code::Comma, ctrl(Key::Command(',')), true, false);
    assert!(rt.state.panel.is_some());
    key(&mut rt, Code::Comma, ctrl(Key::Command(',')), false, false);
    rt.update(|s, tx| s.close_panel(tx));
    rt.frame(0.);
    for repeat in [false, true, true] {
        key(
            &mut rt,
            Code::Comma,
            KeyEvent {
                key: Key::Text(",".into()),
                mods: Mods::default(),
            },
            true,
            repeat,
        );
    }
    let keys = sent_keys(&sent);
    assert_eq!(keys.len(), 3, "{keys:?}");
    assert_eq!(keys[0]["action"], "press");
    assert_eq!(keys[1]["action"], "repeat");
    assert_eq!(keys[2]["action"], "repeat");
    assert!(keys.iter().all(|k| k["key"] == "comma"));
}

#[test]
fn holding_session_tab_navigation_sends_three_focus_requests() {
    for (code, key_event, method) in [
        (
            Code::PageUp,
            ctrl(Key::PageUp),
            "session.focus_previous_window",
        ),
        (
            Code::PageDown,
            ctrl(Key::PageDown),
            "session.focus_next_window",
        ),
    ] {
        let (ws, rx) = tabbed_workspace("single");
        let mut rt = runtime(ws);
        for repeat in [false, true, true] {
            key(&mut rt, code, key_event.clone(), true, repeat);
        }
        let sent = requests(&rx);
        assert_eq!(sent.len(), 3, "{sent:?}");
        assert!(sent.iter().all(|r| r.0 == method), "{sent:?}");
    }
}

#[test]
fn repeating_pane_focus_and_context_changes() {
    let (mut ws, rx) = tabbed_workspace("single");
    overrides(
        &mut ws,
        json!([{"keys":"ctrl+b>c","action":"client.tab.new"}]),
    );
    let mut rt = runtime(ws);
    let focus = KeyEvent {
        key: Key::Left,
        mods: Mods {
            ctrl: true,
            alt: true,
            ..Mods::default()
        },
    };
    key(&mut rt, Code::ArrowLeft, focus.clone(), true, false);
    key(&mut rt, Code::ArrowLeft, focus, true, true);
    assert_eq!(requests(&rx).len(), 2);
    key(&mut rt, Code::KeyB, ctrl(Key::Command('b')), true, false);
    rt.update(|s, tx| s.perform(Intent::AddHostManually, tx));
    rt.frame(0.);
    assert_eq!(rt.state.bindings.sequence.depth, 0);
    rt.update(|s, tx| s.close_panel(tx));
    rt.frame(0.);
    key(&mut rt, Code::KeyB, ctrl(Key::Command('b')), false, false);
    key(&mut rt, Code::KeyB, ctrl(Key::Command('b')), true, false);
    rt.state.keymap_generation += 1;
    press(&mut rt, Key::Text("c".into()));
    assert!(requests(&rx).is_empty());
    key(&mut rt, Code::KeyB, ctrl(Key::Command('b')), false, false);
    key(&mut rt, Code::KeyB, ctrl(Key::Command('b')), true, false);
    rt.window_focus(false);
    rt.frame(0.);
    assert!(!rt.state.window_focused);
    assert_eq!(rt.state.bindings.sequence.depth, 0);
}

#[test]
fn custom_zoom_reaches_focused_query_and_form_without_editing_them() {
    for form in [false, true] {
        let mut ws = workspace();
        overrides(
            &mut ws,
            json!([{"keys":"ctrl+j","action":"client.zoom.in"}]),
        );
        if form {
            ws.perform(Intent::AddHostManually, &mut rune::Tx::default());
        } else {
            ws.picker(true);
        }
        ws.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("keep");
        let mut rt = runtime(ws);
        rt.key(ctrl(Key::Command('j')));
        rt.frame(0.);
        assert_eq!(rt.state.settings.ui_scale, 1.1);
        assert_eq!(rt.state.panel.as_ref().unwrap().inputs[0].text, "keep");
    }
}

#[test]
fn send_key_uses_explicit_command_and_new_close_actions_ask() {
    let (mut ws, _) = tabbed_workspace("single");
    let (sender, sent) = flume::unbounded();
    live_terminal(&mut ws, "block:a", &sender);
    let mut rt = runtime(ws);
    rt.update(|s, tx| {
        assert!(actions::run(
            s,
            "pane.send_key",
            &json!({"key":"ctrl+b"}),
            tx
        ));
    });
    rt.frame(0.);
    let keys: Vec<_> = sent
        .try_iter()
        .filter_map(|c| match c {
            HostCommand::Terminal {
                command: Command::SendKey(k),
                ..
            } => Some(k),
            _ => None,
        })
        .collect();
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0]["action"], "press");
    assert_eq!(keys[1]["action"], "release");
    rt.update(|s, tx| {
        assert!(actions::run(s, "client.tab.close", &Value::Null, tx));
    });
    rt.frame(0.);
    assert!(rt.state.panel.as_ref().unwrap().title.contains("Close"));
}
