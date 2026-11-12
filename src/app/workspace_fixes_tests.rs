//! The window title, the sidebar's tab counts, the identity chip's size, splits on every side,
//! focus through a tab's panes, terminal font size actions and Rex help.
use super::*;
use crate::app::{actions, font_size, help, splits};
use serde_json::{Value, json};

fn mods(ctrl: bool, alt: bool, shift: bool) -> Mods {
    Mods {
        ctrl,
        alt,
        shift,
        ..Mods::default()
    }
}

fn key(rt: &mut Runtime<Workspace>, key: Key, mods: Mods) {
    rt.key(KeyEvent { key, mods });
    for _ in 0..3 {
        rt.frame(1. / 60.);
    }
}

fn rect(rt: &Runtime<Workspace>, key: &str) -> [f32; 4] {
    rt.elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("missing {key}"))
        .rect
}

fn notices(ws: &Workspace) -> Vec<&str> {
    ws.notices.iter().map(|n| n.text.as_str()).collect()
}

fn run(ws: &mut Workspace, action: &str, args: Value) {
    assert!(
        actions::run(ws, action, &args, &mut rune::Tx::default()),
        "{action} {args}"
    );
}

/// The one `session.new_split` in `rx`, as (anchor, direction, side).
fn split(rx: &flume::Receiver<HostCommand>) -> (String, String, String) {
    let sent = requests(rx);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(sent[0].0, "session.new_split");
    let field = |name: &str| sent[0].2[name].as_str().unwrap().to_owned();
    (field("anchor_id"), field("direction"), field("side"))
}

#[test]
fn the_window_title_leads_with_the_tab_on_screen() {
    let (mut ws, _rx) = tabbed_workspace("split");
    ws.hosts[0].model.views.get_mut("s").unwrap().label = Some("lunar-basin".into());
    let mut rt = runtime(ws);
    rt.update(|s, tx| s.update_title(tx));
    assert_eq!(rt.state.title, "dev - lunar-basin · test-0");

    // A tab still called Shell is known by its program's title, as on its pill.
    rt.update(|s, tx| {
        let model = &mut s.hosts[0].model;
        model.views.get_mut("s").unwrap().windows[1].label =
            Some(rex_rune::connection::DEFAULT_TAB_LABEL.into());
        model
            .titles
            .insert("block:b".into(), ("vim notes".into(), None));
        s.update_title(tx);
    });
    assert_eq!(rt.state.title, "vim notes - lunar-basin · test-0");

    rt.update(|s, tx| {
        s.hosts[0].model.selected = None;
        s.update_title(tx);
    });
    assert_eq!(rt.state.title, "Rex · test-0", "no session chosen yet");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_sidebar_counts_tabs_only_above_one() {
    let (mut ws, _rx) = tabbed_workspace("single");
    ws.sidebar.toggle();
    let model = &mut ws.hosts[0].model;
    model.sessions = vec![session("s"), session("lone")];
    model.views.insert(
        "lone".into(),
        serde_json::from_value(json!({
            "revision": 1, "session_id": "lone", "active_window_id": "w",
            "windows": [{"window_id": "w", "focused_block_id": "block:w",
                "layers": [{"kind": "tiled", "blocks": [{"block_id": "block:w"}]}]}]
        }))
        .expect("a view"),
    );
    let rt = runtime(ws);
    assert!(has(&rt, "session:0:s") && has(&rt, "session:0:lone"));
    assert_eq!(element_text(&rt, "session-count:0:s"), ["2"]);
    assert!(
        !has(&rt, "session-count:0:lone"),
        "one tab is what a session has: no badge"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_identity_chip_fits_its_words_and_names_the_host_only_when_it_matters() {
    let mut ws = workspace_hosts(2);
    let _a = capture_host(&mut ws, 0);
    let _b = capture_host(&mut ws, 1);
    let model = &mut ws.hosts[0].model;
    model.connected = true;
    model.sessions = vec![session("s")];
    model.selected = Some("s".into());
    model.views.insert(
        "s".into(),
        serde_json::from_value(json!({
            "revision": 1, "session_id": "s", "label": "dev", "active_window_id": "w",
            "windows": [{"window_id": "w", "focused_block_id": "block:w",
                "layers": [{"kind": "tiled", "blocks": [{"block_id": "block:w"}]}]}]
        }))
        .expect("a view"),
    );
    let mut rt = runtime(ws);
    // The chip's width follows its words on a spring.
    let settle = |rt: &mut Runtime<Workspace>| {
        for _ in 0..60 {
            rt.frame(1. / 60.);
        }
    };

    // One connected host: the session's name alone, at the chip's floor.
    assert!(!has(&rt, "identity-host"));
    assert_eq!(element_text(&rt, "identity"), ["dev"]);
    let short = rect(&rt, "identity");
    assert_eq!(short[2], 120., "a short name keeps the floor");
    assert_eq!(short[3], 34.);

    // A longer name widens the chip; a very long one stops at the cap and truncates.
    rt.update(|s, _| {
        s.hosts[0].model.views.get_mut("s").unwrap().label = Some("payments-staging".into())
    });
    settle(&mut rt);
    let wider = rect(&rt, "identity")[2];
    assert!(wider > 120. && wider < 210., "{wider}");
    rt.update(|s, _| {
        s.hosts[0].model.views.get_mut("s").unwrap().label =
            Some("a-session-with-a-name-far-too-long-for-any-toolbar".into())
    });
    settle(&mut rt);
    assert_eq!(rect(&rt, "identity")[2], 210.);
    let tabs = rect(&rt, "tab-track");
    assert!(
        (tabs[0] - (rect(&rt, "identity")[0] + 210. + 8.)).abs() < 0.5,
        "the tab track follows the chip: {tabs:?}"
    );

    // A second connected host: the name could be either's, so the host is named.
    rt.update(|s, _| s.hosts[1].model.connected = true);
    settle(&mut rt);
    assert_eq!(element_text(&rt, "identity-host"), ["test-0"]);

    // Trouble is always worth words.
    rt.update(|s, _| {
        s.hosts[1].model.connected = false;
        s.hosts[0].model.connected = false;
    });
    settle(&mut rt);
    assert_eq!(
        element_text(&rt, "identity-host"),
        ["test-0 · Reconnecting"]
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn splits_go_on_every_side_of_the_focused_pane() {
    let (mut ws, rx) = tabbed_workspace("split");
    for (direction, placed) in [
        ("right", ("horizontal", "after")),
        ("left", ("horizontal", "before")),
        ("down", ("vertical", "after")),
        ("up", ("vertical", "before")),
    ] {
        run(&mut ws, "pane.split", json!({"direction": direction}));
        let (anchor, way, side) = split(&rx);
        assert_eq!(anchor, "block:b", "the focused pane is the anchor");
        assert_eq!((way.as_str(), side.as_str()), placed, "{direction}");
    }
    let spec = actions::find("pane.split").unwrap();
    assert!(!spec.accepts(&json!({"direction":"sideways"})));
    assert!(!spec.accepts(&json!({"direction":"left","side":"before"})));
    assert!(!actions::run(
        &mut ws,
        "pane.split",
        &json!({"direction":"sideways"}),
        &mut rune::Tx::default()
    ));
    assert!(rx.is_empty(), "a refused split sends nothing");
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn best_fit_splits_a_pane_along_its_longer_side() {
    let (mut ws, rx) = tabbed_workspace("split");
    let (sender, _keys) = flume::unbounded();
    live_terminal(&mut ws, "block:b", &sender);
    let key = ws.term_key("block:b");
    for (size, placed) in [
        ([900., 300.], ("horizontal", "after")),
        ([300., 900.], ("vertical", "after")),
    ] {
        ws.terminals.get_mut(&key).unwrap().rect = [0., 0., size[0], size[1]];
        run(&mut ws, "pane.split", json!({"direction":"best_fit"}));
        let (anchor, way, side) = split(&rx);
        assert_eq!(anchor, "block:b");
        assert_eq!((way.as_str(), side.as_str()), placed, "{size:?}");
    }
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn the_pane_menu_offers_four_sides_in_groups_and_commands_adds_best_fit_and_focus() {
    let (ws, rx) = tabbed_workspace("split");
    let mut rt = runtime(ws);
    rt.update(|s, _| s.pane_menu("block:c".into(), None));
    let entries = rt.state.entries();
    let labels: Vec<&str> = entries.iter().map(|e| e.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Copy",
            "Paste",
            "Select all",
            "Split right",
            "Split left",
            "Split down",
            "Split up",
            "Zoom pane",
            "Rename pane…",
            "Copy pane ID",
            "Download file…",
            "Clear terminal",
            "Reset terminal",
            "Reconnect terminal",
            "Close pane…"
        ]
    );
    // Clipboard, layout, the pane, its terminal, then Close: a hairline starts each group.
    let starts: Vec<&str> = entries
        .windows(2)
        .filter(|pair| pair[0].section != pair[1].section)
        .map(|pair| pair[1].label.as_str())
        .collect();
    assert_eq!(
        starts,
        [
            "Split right",
            "Rename pane…",
            "Clear terminal",
            "Close pane…"
        ]
    );

    // The menu splits its own pane, not the focused one.
    let up = entries.iter().find(|e| e.label == "Split up").unwrap();
    let action = up.action.clone();
    rt.update(|s, tx| s.perform(action.clone(), tx));
    assert_eq!(
        split(&rx),
        ("block:c".into(), "vertical".into(), "before".into())
    );

    rt.update(|s, _| s.picker(false));
    let commands = rt.state.entries();
    let enabled = |label: &str| {
        commands
            .iter()
            .find(|e| e.label == label)
            .unwrap_or_else(|| panic!("no {label}"))
            .enabled
    };
    for label in [
        "Split left",
        "Split up",
        "Split best fit",
        "Focus next pane",
        "Focus previous pane",
    ] {
        assert!(enabled(label), "{label}");
    }
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn focus_moves_through_a_tabs_panes_and_needs_a_second_pane() {
    let (mut ws, rx) = tabbed_workspace("split");
    run(&mut ws, splits::FOCUS_NEXT, Value::Null);
    run(&mut ws, splits::FOCUS_PREVIOUS, Value::Null);
    let sent = requests(&rx);
    assert_eq!(
        sent.iter().map(|r| r.0.as_str()).collect::<Vec<_>>(),
        ["session.focus_next_pane", "session.focus_previous_pane"]
    );
    for request in &sent {
        assert_eq!(request.2, json!({"session_id":"s","wrap":true}));
    }

    // With one pane there is nowhere to go: Commands says so.
    let (mut ws, _rx) = tabbed_workspace("single");
    ws.picker(false);
    let entries = ws.entries();
    for label in ["Focus next pane", "Focus previous pane"] {
        let entry = entries.iter().find(|e| e.label == label).unwrap();
        assert!(!entry.enabled, "{label}");
    }
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn font_size_actions_step_one_point_within_range_and_say_so_once() {
    let mut ws = workspace();
    assert_eq!(ws.settings.font_size, 15.);
    run(&mut ws, font_size::INCREASE, Value::Null);
    assert_eq!(ws.settings.font_size, 16.);
    assert_eq!(notices(&ws), ["Terminal font 16 pt"]);
    run(&mut ws, font_size::DECREASE, Value::Null);
    run(&mut ws, font_size::DECREASE, Value::Null);
    assert_eq!(ws.settings.font_size, 14.);
    assert_eq!(
        notices(&ws),
        ["Terminal font 14 pt"],
        "one notice, replaced"
    );
    assert_eq!(
        Settings::load(&ws.settings_path).unwrap().font_size,
        14.,
        "saved"
    );

    for _ in 0..40 {
        run(&mut ws, font_size::INCREASE, Value::Null);
    }
    assert_eq!(ws.settings.font_size, 48.);
    assert_eq!(notices(&ws), ["Terminal font 48 pt · maximum"]);
    for _ in 0..50 {
        run(&mut ws, font_size::DECREASE, Value::Null);
    }
    assert_eq!(ws.settings.font_size, 8.);
    assert_eq!(notices(&ws), ["Terminal font 8 pt · minimum"]);

    run(&mut ws, font_size::RESET, Value::Null);
    assert_eq!(ws.settings.font_size, 15.);
    assert_eq!(notices(&ws), ["Terminal font 15 pt · default"]);
    assert_eq!(ws.settings.ui_scale, 1., "interface zoom is untouched");
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn font_size_keys_are_the_users_to_bind_and_ctrl_keys_still_zoom_the_interface() {
    let mut rt = runtime(workspace());
    // No default: Ctrl Alt = / + / - / 0 are the terminal program's keys, as they were.
    for name in ['=', '+', '-', '0'] {
        key(&mut rt, Key::Command(name), mods(true, true, false));
    }
    assert_eq!(rt.state.settings.font_size, 15.);
    assert!(rt.state.notices.is_empty());

    // Bound in shortcuts.json they size the font, a point at a time.
    rt.update(|s, _| {
        s.settings.shortcut_file.overrides = rex_rune::keymap::Overrides::parse(&json!([
            {"keys":"ctrl+alt+=","action":font_size::INCREASE},
            {"keys":"ctrl+alt+-","action":font_size::DECREASE},
            {"keys":"ctrl+alt+0","action":font_size::RESET}
        ]));
        s.local_keymap = actions::resolve(&s.settings.shortcut_file.overrides);
        s.keymap_generation += 1;
    });
    key(&mut rt, Key::Command('='), mods(true, true, false));
    key(&mut rt, Key::Command('='), mods(true, true, false));
    assert_eq!(rt.state.settings.font_size, 17.);
    key(&mut rt, Key::Command('-'), mods(true, true, false));
    assert_eq!(rt.state.settings.font_size, 16.);
    key(&mut rt, Key::Command('0'), mods(true, true, false));
    assert_eq!(rt.state.settings.font_size, 15.);
    assert_eq!(rt.state.settings.ui_scale, 1.);

    key(&mut rt, Key::Command('='), mods(true, false, false));
    assert_eq!(rt.state.settings.ui_scale, 1.1, "Ctrl = is interface zoom");
    assert_eq!(rt.state.settings.font_size, 15.);
    key(&mut rt, Key::Command('0'), mods(true, false, false));
    assert_eq!(rt.state.settings.ui_scale, 1.);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn commands_lists_font_size_rows_and_disables_a_step_that_changes_nothing() {
    let mut ws = workspace();
    ws.picker(false);
    let state = |ws: &Workspace| {
        let entries = ws.entries();
        [
            "Increase terminal font size",
            "Decrease terminal font size",
            "Reset terminal font size",
        ]
        .map(|label| {
            let entry = entries
                .iter()
                .find(|e| e.label == label)
                .unwrap_or_else(|| panic!("no {label}"));
            (entry.enabled, entry.detail.clone())
        })
    };
    assert_eq!(
        state(&ws),
        [
            (true, String::new()),
            (true, String::new()),
            (false, String::new()),
        ],
        "at the default there is nothing to reset, and none has a key until one is bound"
    );
    ws.settings.font_size = 48.;
    assert_eq!(state(&ws).map(|s| s.0), [false, true, true]);
    ws.settings.font_size = 8.;
    assert_eq!(state(&ws).map(|s| s.0), [true, false, true]);
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn rex_help_opens_this_versions_guide_from_commands() {
    let mut rt = runtime(workspace());
    rt.update(|s, _| s.picker(false));
    let entry = rt
        .state
        .entries()
        .into_iter()
        .find(|e| e.label == "Rex help")
        .expect("a Rex help row");
    assert!(entry.enabled, "help needs no connection");
    rt.update(|s, tx| s.perform(entry.action.clone(), tx));
    assert_eq!(
        rt.state.opened_links,
        [format!(
            "https://github.com/shuv1337/rex-rune/blob/v{}/README.md",
            env!("CARGO_PKG_VERSION")
        )]
    );
    assert_eq!(rt.state.opened_links, [help::url()]);
    assert!(rt.state.panel.is_none(), "Commands closes");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_new_actions_are_registered_bindable_and_never_remote() {
    for name in [
        splits::FOCUS_NEXT,
        splits::FOCUS_PREVIOUS,
        font_size::INCREASE,
        font_size::DECREASE,
        font_size::RESET,
        help::ACTION,
    ] {
        let spec = actions::find(name).unwrap_or_else(|| panic!("{name} is not registered"));
        assert!(
            matches!(spec.remote(), actions::Remote::Never(_)),
            "{name} must not be available remotely"
        );
        assert!(spec.accepts(&Value::Null) && spec.accepts(&json!({})));
        assert!(!spec.accepts(&json!({"extra":1})), "{name}");
    }
    // A user's shortcuts.json can bind them, with the split's new directions.
    let overrides = rex_rune::keymap::Overrides::parse(&json!([
        {"keys":"ctrl+alt+]","action":"pane.focus_next"},
        {"keys":"ctrl+alt+enter","action":"pane.split","args":{"direction":"best_fit"}},
        {"keys":"f1","action":"client.help"}
    ]));
    let map = actions::resolve(&overrides);
    assert!(map.diagnostics.is_empty(), "{:?}", map.diagnostics);
    let title = |action: &str| {
        map.bindings
            .iter()
            .find(|b| b.binding.action == action && b.binding.source.as_deref() == Some("settings"))
            .map(|b| actions::binding_title(&b.binding))
    };
    assert_eq!(title("pane.split").as_deref(), Some("Split best fit"));
    assert_eq!(title("pane.focus_next").as_deref(), Some("Focus next pane"));
    assert_eq!(title("client.help").as_deref(), Some("Rex help"));
}

#[test]
fn a_bindings_title_is_the_name_commands_gives_its_row() {
    use rexrs::api::client::Binding;
    let (mut ws, _rx) = tabbed_workspace("split");
    ws.picker(false);
    let rows: Vec<String> = ws
        .commands()
        .into_iter()
        .map(|c| c.entry.label.trim_end_matches('…').to_owned())
        .collect();
    let direction = |d: &str| Some(json!({"direction": d}));
    for (action, args, title) in [
        ("pane.split", direction("right"), "Split right"),
        ("pane.split", direction("down"), "Split down"),
        ("pane.split", direction("best_fit"), "Split best fit"),
        ("pane.focus", direction("left"), "Focus pane left"),
        ("pane.focus", direction("down"), "Focus pane down"),
        ("client.tab.move", direction("left"), "Move tab left"),
        ("client.tab.move", direction("right"), "Move tab right"),
        ("client.close", None, "Close pane or tab"),
        ("client.search", None, "Search terminal"),
        ("pane.focus_next", None, "Focus next pane"),
    ] {
        let mut binding = Binding::new(["f5"], action);
        binding.args = args;
        assert_eq!(actions::binding_title(&binding), title);
        assert!(
            rows.iter().any(|row| row == title),
            "Commands has no row named {title:?}"
        );
    }
    // What Commands has no row for still reads as one phrase.
    let mut goto = Binding::new(["f5"], "client.tab.goto");
    goto.args = Some(json!({"index": 3}));
    assert_eq!(actions::binding_title(&goto), "Go to tab 3");
    let mut send = Binding::new(["f5"], "pane.send_key");
    send.args = Some(json!({"key": "ctrl+b"}));
    assert_eq!(actions::binding_title(&send), "Send key · ctrl+b");
    std::fs::remove_file(&ws.settings_path).ok();
}
