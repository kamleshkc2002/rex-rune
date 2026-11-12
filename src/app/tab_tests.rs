//! Tab management: the tab menu on toolbar pills and sidebar rows, moving a tab within its
//! session's order, muting it on this client, closing the others and its default title.
use super::*;
use crate::app::{actions, tabs::TabAction};
use rex_rune::connection::activity::{DesktopNotice, ProgramState::*};
use serde_json::{Value, json};

fn tab(id: &str, label: &str) -> Value {
    json!({"window_id": id, "label": label, "focused_block_id": format!("block:{id}"),
        "layers": [{"kind": "tiled", "layout": {"block_id": format!("block:{id}")},
            "blocks": [{"block_id": format!("block:{id}")}]}]})
}

/// Host 0 connected on session "s" with one-pane tabs "a", "b" and "c" (labels "one", "two"
/// and "three"), showing `active`.
fn three_tabs(active: &str) -> (Workspace, flume::Receiver<HostCommand>) {
    let mut ws = workspace();
    let rx = capture_host(&mut ws, 0);
    let model = &mut ws.hosts[0].model;
    model.connected = true;
    model.sessions = vec![session("s")];
    model.selected = Some("s".into());
    model.views.insert(
        "s".into(),
        serde_json::from_value(json!({
            "revision": 1, "session_id": "s", "active_window_id": active,
            "windows": [tab("a", "one"), tab("b", "two"), tab("c", "three")]
        }))
        .expect("a view"),
    );
    (ws, rx)
}

fn order(ws: &Workspace, session: &str) -> Vec<String> {
    ws.hosts[0].model.views[session]
        .windows
        .iter()
        .filter_map(|w| w.window_id.clone())
        .collect()
}

/// What the server does with `session.move_window`: place the window just before another,
/// or at the end.
fn server_moves(ws: &mut Workspace, payload: &Value) {
    let windows = &mut ws.hosts[0]
        .model
        .views
        .get_mut(payload["session_id"].as_str().unwrap())
        .unwrap()
        .windows;
    let id = |w: &rexrs::api::session::Window| w.window_id.clone().unwrap();
    let from = windows
        .iter()
        .position(|w| id(w) == payload["window_id"])
        .expect("the window to move");
    let moved = windows.remove(from);
    let to = match payload["before_window_id"].as_str() {
        Some(before) => windows
            .iter()
            .position(|w| id(w) == before)
            .expect("the window to go before"),
        None => windows.len(),
    };
    windows.insert(to, moved);
}

fn labels(ws: &Workspace) -> Vec<(String, bool)> {
    ws.entries()
        .into_iter()
        .map(|e| (e.label, e.enabled))
        .collect()
}

fn choose(rt: &mut Runtime<Workspace>, label: &str) {
    let entry = rt
        .state
        .entries()
        .into_iter()
        .find(|e| e.label == label)
        .unwrap_or_else(|| panic!("no {label}"));
    assert!(entry.enabled, "{label} is disabled");
    rt.update(|s, tx| s.perform(entry.action.clone(), tx));
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
}

#[test]
fn the_tab_menu_groups_its_actions_and_ends_with_close() {
    let (ws, _rx) = three_tabs("a");
    let mut rt = runtime(ws);
    secondary_click(&mut rt, "tab:b");
    let panel = rt.state.panel.as_ref().expect("a tab menu");
    assert!(matches!(panel.kind, PanelKind::Window));
    assert_eq!(panel.title, "two");
    assert_eq!(
        labels(&rt.state),
        [
            ("Rename tab…", true),
            ("Mute tab", true),
            ("Move left", true),
            ("Move right", true),
            ("New tab", true),
            ("Close other tabs…", true),
            ("Close tab…", true),
        ]
        .map(|(l, e)| (l.to_owned(), e))
    );
    // A hairline starts each group, and the rows that end tabs are the last one, in DANGER.
    let entries = rt.state.entries();
    let sections: Vec<u8> = entries.iter().map(|e| e.section).collect();
    assert_eq!(sections, [0, 0, 1, 1, 2, 3, 3]);
    let danger: Vec<bool> = entries.iter().map(|e| e.danger).collect();
    assert_eq!(danger, [false, false, false, false, false, true, true]);
    let lines = rt
        .elements()
        .iter()
        .filter(|e| !e.exiting && e.role == "separator")
        .count();
    assert_eq!(lines, 3, "a hairline between each of the four groups");
    assert!(
        has(&rt, "tab-target:b"),
        "the tab the menu acts on is outlined"
    );
    let details: Vec<String> = rt.state.entries().into_iter().map(|e| e.detail).collect();
    assert_eq!(details[2], "Ctrl Shift PageUp", "{details:?}");
    assert_eq!(details[3], "Ctrl Shift PageDown");
    assert_eq!(details[4], "Ctrl Shift T");
    press(&mut rt, Key::Escape);
    assert!(rt.state.panel.is_none());
    assert!(!has(&rt, "tab-target:b"));

    // The ends of the order can't move past them.
    secondary_click(&mut rt, "tab:a");
    let rows = labels(&rt.state);
    assert_eq!(rows[2], ("Move left".to_owned(), false));
    assert_eq!(rows[3], ("Move right".to_owned(), true));
    press(&mut rt, Key::Escape);
    secondary_click(&mut rt, "tab:c");
    let rows = labels(&rt.state);
    assert_eq!(rows[2], ("Move left".to_owned(), true));
    assert_eq!(rows[3], ("Move right".to_owned(), false));
    press(&mut rt, Key::Escape);

    // A lone tab has nowhere to go and no others to close.
    rt.update(|s, _| {
        s.hosts[0]
            .model
            .views
            .get_mut("s")
            .unwrap()
            .windows
            .truncate(1)
    });
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
    secondary_click(&mut rt, "tab:a");
    assert_eq!(
        labels(&rt.state)
            .into_iter()
            .filter(|(_, enabled)| !enabled)
            .map(|(label, _)| label)
            .collect::<Vec<_>>(),
        ["Move left", "Move right", "Close other tabs…"]
    );
    press(&mut rt, Key::Escape);

    // Offline, only Mute is left: it is this client's alone.
    rt.update(|s, _| s.hosts[0].model.connected = false);
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
    secondary_click(&mut rt, "tab:a");
    assert_eq!(rt.state.panel.as_ref().unwrap().title, "one · Offline");
    assert_eq!(
        labels(&rt.state)
            .into_iter()
            .filter(|(_, enabled)| *enabled)
            .map(|(label, _)| label)
            .collect::<Vec<_>>(),
        ["Mute tab"]
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn moving_a_tab_asks_the_server_for_its_new_place_in_the_order() {
    let (mut ws, rx) = three_tabs("b");
    let mut tx = rune::Tx::default();
    let mut step = |ws: &mut Workspace, id: &str, action: TabAction| {
        let target = ws.current_tab(id).expect("a tab");
        ws.perform(Intent::Window(target, action), &mut tx);
        let sent = requests(&rx);
        for (method, session, payload) in &sent {
            assert_eq!(
                (method.as_str(), session.as_str()),
                ("session.move_window", "s")
            );
            server_moves(ws, payload);
        }
        sent.into_iter().map(|r| r.2).collect::<Vec<_>>()
    };
    // Left goes before the tab on its left.
    let sent = step(&mut ws, "b", TabAction::MoveLeft);
    assert_eq!(
        sent,
        [json!({"session_id":"s","window_id":"b","before_window_id":"a"})]
    );
    assert_eq!(order(&ws, "s"), ["b", "a", "c"]);
    // Right goes before the tab after its right neighbour.
    let sent = step(&mut ws, "b", TabAction::MoveRight);
    assert_eq!(
        sent,
        [json!({"session_id":"s","window_id":"b","before_window_id":"c"})]
    );
    assert_eq!(order(&ws, "s"), ["a", "b", "c"]);
    // Right from the next-to-last place names no tab: to the end.
    let sent = step(&mut ws, "b", TabAction::MoveRight);
    assert_eq!(sent, [json!({"session_id":"s","window_id":"b"})]);
    assert_eq!(order(&ws, "s"), ["a", "c", "b"]);
    // At the ends nothing is asked.
    assert!(step(&mut ws, "b", TabAction::MoveRight).is_empty());
    assert!(step(&mut ws, "a", TabAction::MoveLeft).is_empty());
    assert_eq!(order(&ws, "s"), ["a", "c", "b"]);
    // The order, focus and active tab stay the server's: nothing else was sent.
    assert_eq!(
        ws.model().view().unwrap().active_window_id.as_deref(),
        Some("b")
    );
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn ctrl_shift_page_keys_move_the_tab_on_screen() {
    let (ws, rx) = three_tabs("b");
    let mut rt = runtime(ws);
    for (key, expected) in [
        (
            Key::PageUp,
            json!({"session_id":"s","window_id":"b","before_window_id":"a"}),
        ),
        (Key::PageDown, json!({"session_id":"s","window_id":"b"})),
    ] {
        rt.key(KeyEvent {
            key,
            mods: Mods {
                ctrl: true,
                shift: true,
                ..Mods::default()
            },
        });
        rt.frame(1. / 60.);
        let sent = requests(&rx);
        assert_eq!(sent.len(), 1, "{sent:?}");
        assert_eq!(sent[0].0, "session.move_window");
        assert_eq!(sent[0].2, expected);
    }
    // The named action takes a direction and nothing else.
    let spec = actions::find("client.tab.move").unwrap();
    assert!(spec.accepts(&json!({"direction":"left"})));
    for bad in [
        Value::Null,
        json!({}),
        json!({"direction":"up"}),
        json!({"direction":"left","window_id":"a"}),
    ] {
        assert!(!spec.accepts(&bad), "{bad}");
    }
    for name in [
        "client.tab.move",
        "client.tab.mute",
        "client.tab.close_others",
    ] {
        assert!(
            matches!(
                actions::find(name).unwrap().remote(),
                actions::Remote::Never(_)
            ),
            "{name} is never remote"
        );
    }
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_muted_tab_shows_no_word_and_sends_no_notification() {
    let (ws, _rx) = three_tabs("a");
    let mut rt = runtime(ws);
    rt.update(|s, _| s.track_activity(0));
    secondary_click(&mut rt, "tab:b");
    choose(&mut rt, "Mute tab");
    assert!(rt.state.panel.is_none(), "muting closes the menu");
    assert!(rt.state.tab_muted(0, "b") && !rt.state.tab_muted(0, "c"));
    assert!(has(&rt, "tab-muted:b") && !has(&rt, "tab-muted:c"));

    // Trouble in the muted tab and in another one.
    rt.update(|s, _| {
        set_activity(s, "block:b", &[("", record(Blocked, true))]);
        set_activity(s, "block:c", &[("", record(Blocked, true))]);
    });
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
    assert!(!has(&rt, "tab-word:b"), "a muted tab shows no word");
    assert!(has(&rt, "tab-word:c"), "other tabs keep theirs");
    let notified: Vec<_> = rt
        .state
        .outbox
        .iter()
        .map(|n| n.target.as_ref().unwrap().window.clone().unwrap())
        .collect();
    assert_eq!(notified, ["c"], "only the other tab notified");

    // What a muted tab's program asks to say is not passed on either.
    let notice = |block: &str| DesktopNotice {
        session: "s".into(),
        block: block.into(),
        title: String::new(),
        body: "Build finished".into(),
    };
    rt.update(|s, _| s.deliver_program_notices(0, vec![notice("block:b"), notice("block:c")]));
    assert_eq!(rt.state.outbox.len(), 2);
    assert_eq!(
        rt.state.outbox[1]
            .target
            .as_ref()
            .unwrap()
            .window
            .as_deref(),
        Some("c")
    );

    // Finishing while muted latches nothing to show later; a bell neither.
    rt.update(|s, _| {
        set_activity(s, "block:b", &[]);
        s.ring_bells(0, vec![("s".into(), "block:b".into())]);
    });
    assert_eq!(rt.state.outbox.len(), 2);

    // The menu now offers to unmute, and the tab speaks again from then on.
    secondary_click(&mut rt, "tab:b");
    assert!(labels(&rt.state).contains(&("Unmute tab".to_owned(), true)));
    choose(&mut rt, "Unmute tab");
    assert!(!rt.state.tab_muted(0, "b"));
    assert!(!has(&rt, "tab-muted:b"));
    assert!(
        !has(&rt, "tab-word:b"),
        "what happened while muted is not news afterwards"
    );
    rt.update(|s, _| set_activity(s, "block:b", &[("", record(Error, true))]));
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
    assert!(has(&rt, "tab-word:b"));
    assert_eq!(rt.state.outbox.len(), 3);
    assert_eq!(
        rt.state.outbox[2]
            .target
            .as_ref()
            .unwrap()
            .window
            .as_deref(),
        Some("b")
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn muted_tabs_are_saved_until_they_close_and_old_settings_still_load() {
    let old: Settings = serde_json::from_str("{}").unwrap();
    assert!(old.muted_tabs.is_empty());
    assert!(
        !serde_json::to_string(&old).unwrap().contains("muted_tabs"),
        "nothing is written until a tab is muted"
    );

    let (mut ws, _rx) = three_tabs("a");
    let mut tx = rune::Tx::default();
    for id in ["b", "c"] {
        let target = ws.current_tab(id).unwrap();
        ws.perform(Intent::Window(target, TabAction::Mute), &mut tx);
    }
    let again = relaunch(&ws);
    assert!(again.tab_muted(0, "b") && again.tab_muted(0, "c") && !again.tab_muted(0, "a"));

    // A view that no longer lists the tab means it closed; an unloaded view says nothing.
    let view = ws.hosts[0].model.views.remove("s").unwrap();
    ws.sync_tabs(0, &mut tx);
    assert!(ws.tab_muted(0, "b") && ws.tab_muted(0, "c"));
    ws.hosts[0].model.views.insert("s".into(), view);
    ws.hosts[0]
        .model
        .views
        .get_mut("s")
        .unwrap()
        .windows
        .retain(|w| w.window_id.as_deref() != Some("b"));
    ws.sync_tabs(0, &mut tx);
    assert!(!ws.tab_muted(0, "b") && ws.tab_muted(0, "c"));
    let saved = Settings::load(&ws.settings_path).unwrap();
    assert_eq!(saved.muted_tabs.len(), 1);
    assert_eq!(saved.muted_tabs[0].window, "c");
    assert_eq!(saved.muted_tabs[0].session, "s");
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn a_muted_tab_is_forgotten_when_its_session_ends() {
    let (mut ws, _rx) = three_tabs("a");
    let mut tx = rune::Tx::default();
    let target = ws.current_tab("b").unwrap();
    ws.perform(Intent::Window(target, TabAction::Mute), &mut tx);

    // Offline, the host's list says nothing: a reconnect must not wipe what was muted.
    let (sessions, view) = {
        let model = &mut ws.hosts[0].model;
        model.connected = false;
        (
            std::mem::take(&mut model.sessions),
            model.views.remove("s").unwrap(),
        )
    };
    ws.sync_tabs(0, &mut tx);
    assert!(ws.tab_muted(0, "b"));

    // Connected with the session still listed and its view not loaded: still nothing.
    ws.hosts[0].model.connected = true;
    ws.hosts[0].model.sessions = sessions;
    ws.sync_tabs(0, &mut tx);
    assert!(ws.tab_muted(0, "b"));

    // The session ended, which drops its view with it, so only the list can say so.
    drop(view);
    ws.hosts[0].model.sessions.clear();
    ws.sync_tabs(0, &mut tx);
    assert!(!ws.tab_muted(0, "b"));
    assert!(ws.settings.muted_tabs.is_empty());
    assert!(
        Settings::load(&ws.settings_path)
            .unwrap()
            .muted_tabs
            .is_empty()
    );
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn close_other_tabs_asks_once_and_closes_only_the_tabs_it_named() {
    let (ws, rx) = three_tabs("a");
    let mut rt = runtime(ws);
    secondary_click(&mut rt, "tab:b");
    choose(&mut rt, "Close other tabs…");
    let panel = rt.state.panel.as_ref().expect("a confirmation");
    assert_eq!(panel.title, "Close 2 other tabs?");
    let PanelKind::Confirm { description, .. } = &panel.kind else {
        panic!("not a confirmation");
    };
    assert_eq!(
        description,
        "Ends every program in the 2 other tabs of s-label on test-0; two stays open. Closing the app itself leaves programs running."
    );
    assert!(has(&rt, "tab-target:b"), "the tab that stays is outlined");
    assert!(
        focused(&rt).is_some_and(|path| path.ends_with("cancel-close")),
        "Keep running is the default"
    );
    assert!(requests(&rx).is_empty(), "nothing closes before the answer");

    // Meanwhile one named tab closes and a new one opens: only the other named tab closes.
    rt.update(|s, _| {
        let windows = &mut s.hosts[0].model.views.get_mut("s").unwrap().windows;
        windows.retain(|w| w.window_id.as_deref() != Some("a"));
        windows.push(serde_json::from_value(tab("d", "four")).unwrap());
    });
    click(&mut rt, "confirm-close");
    assert!(rt.state.panel.is_none());
    let sent = requests(&rx);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(sent[0].0, "session.close_window");
    assert_eq!(sent[0].2, json!({"session_id":"s","window_id":"c"}));

    // Keep running closes nothing; the named action asks about the tab on screen.
    rt.update(|s, tx| {
        s.hosts[0]
            .model
            .views
            .get_mut("s")
            .unwrap()
            .active_window_id = Some("d".into());
        assert!(actions::run(s, "client.tab.close_others", &Value::Null, tx));
    });
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
    assert_eq!(
        rt.state.panel.as_ref().unwrap().title,
        "Close 2 other tabs?"
    );
    press(&mut rt, Key::Enter);
    assert!(rt.state.panel.is_none());
    assert!(requests(&rx).is_empty());

    // One other tab reads as one; a lone tab is not asked about at all.
    rt.update(|s, tx| {
        s.hosts[0]
            .model
            .views
            .get_mut("s")
            .unwrap()
            .windows
            .truncate(2);
        let target = s.current_tab("b").unwrap();
        s.perform(Intent::Window(target, TabAction::CloseOthers), tx);
    });
    assert_eq!(
        rt.state.panel.as_ref().unwrap().title,
        "Close the other tab?"
    );
    rt.update(|s, tx| {
        s.close_panel(tx);
        s.hosts[0]
            .model
            .views
            .get_mut("s")
            .unwrap()
            .windows
            .truncate(1);
        let target = s.current_tab("b").unwrap();
        s.perform(Intent::Window(target, TabAction::CloseOthers), tx);
    });
    assert!(rt.state.panel.is_none());
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn closing_a_tab_names_it_and_a_tab_that_closed_leaves_its_panel() {
    let (ws, rx) = three_tabs("a");
    let mut rt = runtime(ws);
    secondary_click(&mut rt, "tab:c");
    choose(&mut rt, "Close tab…");
    let panel = rt.state.panel.as_ref().expect("a confirmation");
    assert_eq!(panel.title, "Close three?");
    assert!(
        matches!(&panel.kind, PanelKind::Confirm { method, payload, description }
            if method == "session.close_window" && payload["window_id"] == "c"
                && description.contains("on test-0"))
    );
    // The tab closes elsewhere while the question is open.
    rt.update(|s, tx| {
        s.hosts[0]
            .model
            .views
            .get_mut("s")
            .unwrap()
            .windows
            .retain(|w| w.window_id.as_deref() != Some("c"));
        s.sync_tabs(0, tx);
    });
    assert!(rt.state.panel.is_none());
    assert!(
        rt.state
            .notices
            .iter()
            .any(|n| n.text == "That tab is no longer available.")
    );
    assert!(requests(&rx).is_empty());
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_named_tab_can_go_back_to_its_default_title() {
    let (ws, rx) = three_tabs("a");
    let mut rt = runtime(ws);
    secondary_click(&mut rt, "tab:b");
    choose(&mut rt, "Rename tab…");
    let panel = rt.state.panel.as_ref().expect("the rename form");
    assert!(matches!(&panel.kind, PanelKind::Form(Form::RenameWindow(id)) if id == "b"));
    assert_eq!(panel.inputs[0].text, "two");
    assert!(has(&rt, "default-title"));
    click(&mut rt, "default-title");
    assert!(rt.state.panel.is_none());
    let sent = requests(&rx);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(sent[0].0, "session.set_window_label");
    assert_eq!(
        sent[0].2,
        json!({"session_id":"s","window_id":"b","label":""}),
        "an empty label restores the server's default"
    );

    // A tab that already shows its program's title has nothing to go back to: neither the
    // label this client gives new tabs nor the server's own default.
    for label in [
        rex_rune::connection::DEFAULT_TAB_LABEL.to_owned(),
        "Window 9dcd0a4c".to_owned(),
    ] {
        rt.update(|s, tx| {
            let window = &mut s.hosts[0].model.views.get_mut("s").unwrap().windows[1];
            window.window_id = Some("window:01a1aa89-4009-757a-a7b9-1ce89dcd0a4c".into());
            window.label = Some(label.clone());
            s.perform(Intent::RenameWindow(window_id(s, 1)), tx);
        });
        for _ in 0..4 {
            rt.frame(1. / 60.);
        }
        assert!(matches!(
            rt.state.panel.as_ref().unwrap().kind,
            PanelKind::Form(Form::RenameWindow(_))
        ));
        assert!(!has(&rt, "default-title"), "{label}");
        press(&mut rt, Key::Escape);
    }

    // Saving a name still names the tab.
    rt.update(|s, tx| s.perform(Intent::RenameWindow("a".into()), tx));
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
    rt.update(|s, _| s.edit_input(0).text = " build ".into());
    click(&mut rt, "save-form");
    let sent = requests(&rx);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(
        sent[0].2,
        json!({"session_id":"s","window_id":"a","label":"build"})
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

fn window_id(ws: &Workspace, index: usize) -> String {
    ws.hosts[0].model.views["s"].windows[index]
        .window_id
        .clone()
        .unwrap()
}

#[test]
fn sidebar_tab_rows_open_the_same_menu_for_their_own_session() {
    let (ws, rx) = sidebar_tabs_workspace();
    let mut rt = runtime(ws);
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    requests(&rx);
    let row = "sidebar-tab:0:other:other-second";
    secondary_click(&mut rt, row);
    let panel = rt.state.panel.as_ref().expect("a tab menu");
    assert!(matches!(panel.kind, PanelKind::Window));
    assert_eq!(panel.title, "other-label · dev");
    assert_eq!(
        labels(&rt.state),
        [
            ("Rename tab…", true),
            ("Mute tab", true),
            ("Move up", true),
            ("Move down", false),
            ("New tab", true),
            ("Close other tabs…", true),
            ("Close tab…", true),
        ]
        .map(|(l, e)| (l.to_owned(), e)),
        "tabs run top to bottom in the sidebar"
    );
    assert!(has(&rt, "tab-target:other-second"));
    assert!(
        rt.state.entries().iter().all(|e| e.detail.is_empty()),
        "shortcuts act on the session on screen, so another session's tab shows none"
    );

    // Every action names the row's session, not the one on screen.
    choose(&mut rt, "Move up");
    let sent = requests(&rx);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(sent[0].0, "session.move_window");
    assert_eq!(
        sent[0].2,
        json!({"session_id":"other","window_id":"other-second","before_window_id":"other-first"})
    );

    secondary_click(&mut rt, row);
    choose(&mut rt, "New tab");
    let sent = requests(&rx);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(
        (sent[0].0.as_str(), sent[0].1.as_str()),
        ("session.new_window", "other")
    );
    assert_eq!(
        sent[0].2["cwd_from"], "block:b",
        "it starts where the row's tab is"
    );

    secondary_click(&mut rt, row);
    choose(&mut rt, "Rename tab…");
    rt.update(|s, _| s.edit_input(0).text = "logs".into());
    click(&mut rt, "save-form");
    let sent = requests(&rx);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(
        sent[0].2,
        json!({"session_id":"other","window_id":"other-second","label":"logs"})
    );

    secondary_click(&mut rt, row);
    choose(&mut rt, "Close tab…");
    assert_eq!(rt.state.panel.as_ref().unwrap().title, "Close dev?");
    click(&mut rt, "confirm-close");
    let sent = requests(&rx);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(sent[0].0, "session.close_window");
    assert_eq!(
        sent[0].2,
        json!({"session_id":"other","window_id":"other-second"})
    );

    secondary_click(&mut rt, row);
    choose(&mut rt, "Mute tab");
    assert!(has(&rt, "tab-muted:other-second"));
    assert!(requests(&rx).is_empty(), "mute never reaches the server");
    assert_eq!(
        rt.state.model().selected.as_deref(),
        Some("s"),
        "none of it switched sessions"
    );

    // A middle-click asks to close the row's tab; a double-click renames it.
    let r = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("sidebar-tab:0:other:other-first"))
        .unwrap()
        .rect;
    let (x, y) = (r[0] + r[2] / 2., r[1] + r[3] / 2.);
    rt.button_down(x, y, rune::PointerButton::Middle);
    rt.button_up(x, y, rune::PointerButton::Middle);
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
    let panel = rt.state.panel.as_ref().expect("a close confirmation");
    assert!(
        matches!(&panel.kind, PanelKind::Confirm { method, payload, .. }
            if method == "session.close_window" && payload["window_id"] == "other-first")
    );
    assert_eq!(
        panel.tab.as_ref().map(|t| t.session.as_str()),
        Some("other")
    );
    assert!(requests(&rx).is_empty(), "a middle-click doesn't switch");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn commands_lists_the_tab_actions_for_the_tab_on_screen() {
    let (mut ws, rx) = three_tabs("c");
    ws.picker(false);
    let rows = labels(&ws);
    for (label, enabled) in [
        ("Mute tab", true),
        ("Move tab left", true),
        ("Move tab right", false),
        ("Close other tabs…", true),
    ] {
        assert!(
            rows.contains(&(label.to_owned(), enabled)),
            "{label}: {rows:?}"
        );
    }
    let action = ws
        .entries()
        .into_iter()
        .find(|e| e.label == "Move tab left")
        .unwrap()
        .action;
    ws.perform(action, &mut rune::Tx::default());
    assert!(ws.panel.is_none());
    let sent = requests(&rx);
    assert_eq!(sent.len(), 1);
    assert_eq!(
        sent[0].2,
        json!({"session_id":"s","window_id":"c","before_window_id":"b"})
    );
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn a_host_with_no_session_on_screen_draws_no_tab_track() {
    // No session chosen: an empty track would be a sliver of chip beside the identity.
    let mut rt = runtime(workspace());
    assert!(!has(&rt, "tab-track"));
    assert!(has(&rt, "identity"));
    std::fs::remove_file(&rt.state.settings_path).ok();

    let (ws, _rx) = three_tabs("a");
    rt = runtime(ws);
    assert!(has(&rt, "tab-track") && has(&rt, "tab:a"));
    // The session's view goes (its session ended): the track goes with its tabs.
    rt.update(|s, _| {
        s.hosts[0].model.views.clear();
    });
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
    assert!(!has(&rt, "tab-track"));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_sidebar_footer_counts_one_session_in_the_singular() {
    let count = |rt: &Runtime<Workspace>| {
        rt.elements()
            .into_iter()
            .find(|e| !e.exiting && e.key.as_deref() == Some("sidebar-session-count"))
            .and_then(|e| e.text)
            .expect("the footer's count")
    };
    let (mut ws, _rx) = three_tabs("a");
    ws.sidebar.toggle();
    let mut rt = runtime(ws);
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    assert_eq!(count(&rt), "1 session");
    rt.update(|s, _| s.hosts[0].model.sessions.push(session("t")));
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
    assert_eq!(count(&rt), "2 sessions");
    rt.update(|s, _| s.hosts[0].model.sessions.clear());
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
    assert_eq!(count(&rt), "0 sessions");
    std::fs::remove_file(&rt.state.settings_path).ok();
}
