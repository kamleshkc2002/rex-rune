//! Commands: the pinned list of rows, its groups and search, the hints, and the actions the
//! list gained (sessions, hosts, view, app and the host's own).
use super::*;
use crate::app::{
    about, actions,
    commands::{self, Run},
    navigation::Visit,
};
use rex_linux::settings::PaneDensity;
use rexrs::api::client::{Binding, InspectResponseAction};
use serde_json::{Value, json};

/// Every row Commands lists for `fixture()`, in order. A row leaving this list is a command
/// someone can no longer reach: remove its id here only on purpose. `host.start_server` and
/// `app.update` are listed only while they apply (a host waiting for a server, a verified
/// update).
const IDS: &[&str] = &[
    "session.switch",
    "session.new",
    "session.rename",
    "session.next",
    "session.previous",
    "history.back",
    "history.forward",
    "session.refresh",
    "session.close",
    "tab.new",
    "tab.rename",
    "tab.next",
    "tab.previous",
    "tab.next_global",
    "tab.previous_global",
    "tab.mute",
    "tab.move_left",
    "tab.move_right",
    "tab.close_others",
    "tab.close",
    "pane.copy",
    "pane.paste",
    "pane.select_all",
    "pane.search",
    "pane.find_next",
    "pane.find_previous",
    "pane.split.right",
    "pane.split.left",
    "pane.split.down",
    "pane.split.up",
    "pane.split.best_fit",
    "pane.zoom",
    "pane.focus.next",
    "pane.focus.previous",
    "pane.focus.left",
    "pane.focus.right",
    "pane.focus.up",
    "pane.focus.down",
    "pane.move.left",
    "pane.move.right",
    "pane.move.up",
    "pane.move.down",
    "pane.resize.left",
    "pane.resize.right",
    "pane.resize.up",
    "pane.resize.down",
    "pane.balance",
    "pane.move_to_tab",
    "pane.prompt_previous",
    "pane.prompt_next",
    "pane.page_up",
    "pane.page_down",
    "pane.rename",
    "pane.copy_id",
    "pane.download",
    "pane.insert_path",
    "pane.clear",
    "pane.reset",
    "pane.reconnect",
    "pane.close_focused",
    "pane.close_others",
    "pane.close",
    "host.switch",
    "host.add",
    "host.edit",
    "host.shell",
    "host.remote_control",
    "host.reconnect",
    "host.collapse_others",
    "host.remove",
    "view.sidebar",
    "view.tab_position",
    "view.pane_density",
    "view.toolbar",
    "view.pane_headers",
    "view.terminal_theme",
    "view.interface_theme",
    "view.zoom.in",
    "view.zoom.out",
    "view.zoom.reset",
    "view.font.increase",
    "view.font.decrease",
    "view.font.reset",
    "app.shortcuts",
    "app.settings",
    "app.downloads",
    "app.check_updates",
    "app.about",
    "app.help",
    "app.report_issue",
    "app.copy_version",
    "app.copy_debug",
    "app.config_reload",
    "app.config_path",
    "app.mode_exit",
    "app.mode_exit_all",
    "app.close",
];

/// Two connected hosts: test-0 on session "s" (of "s" and "t") showing its split tab "dev",
/// and test-1 with session "u".
fn fixture() -> (
    Workspace,
    flume::Receiver<HostCommand>,
    flume::Receiver<HostCommand>,
) {
    let mut ws = workspace_hosts(2);
    let first = capture_host(&mut ws, 0);
    let second = capture_host(&mut ws, 1);
    let model = &mut ws.hosts[0].model;
    model.connected = true;
    model.sessions = vec![session("s"), session("t")];
    model.selected = Some("s".into());
    model.views.insert(
        "s".into(),
        serde_json::from_value(json!({
            "revision": 1, "session_id": "s", "label": "s-label", "active_window_id": "split",
            "windows": [
                {"window_id": "single", "label": "server", "focused_block_id": "block:a",
                 "layers": [{"kind": "tiled", "layout": {"block_id": "block:a"},
                    "blocks": [{"block_id": "block:a"}]}]},
                {"window_id": "split", "label": "dev", "focused_block_id": "block:b",
                 "layers": [{"kind": "tiled", "layout": {"split": {"direction": "horizontal",
                    "ratio": 0.5, "before": {"block_id": "block:b"},
                    "after": {"block_id": "block:c"}}},
                    "blocks": [{"block_id": "block:b", "label": "editor"},
                        {"block_id": "block:c"}]}]}
            ]
        }))
        .expect("a view"),
    );
    ws.hosts[1].model.connected = true;
    ws.hosts[1].model.sessions = vec![session("u")];
    (ws, first, second)
}

fn search(ws: &mut Workspace, query: &str) {
    ws.picker(false);
    ws.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new(query);
    ws.query_changed();
}

fn labels(ws: &Workspace) -> Vec<String> {
    ws.entries().into_iter().map(|e| e.label).collect()
}

/// The row with this id, as Commands would show it now.
fn command(ws: &Workspace, id: &str) -> commands::Command {
    ws.commands()
        .into_iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("no {id}"))
}

fn choose(ws: &mut Workspace, id: &str) {
    ws.picker(false);
    let row = command(ws, id);
    assert!(row.entry.enabled, "{id} is disabled");
    ws.perform(row.entry.action, &mut rune::Tx::default());
}

fn run(ws: &mut Workspace, action: &str) {
    assert!(
        actions::run(ws, action, &Value::Null, &mut rune::Tx::default()),
        "{action}"
    );
}

fn notices(ws: &Workspace) -> Vec<&str> {
    ws.notices.iter().map(|n| n.text.as_str()).collect()
}

fn overrides(ws: &mut Workspace, value: Value) {
    ws.settings.shortcut_file.overrides = rex_linux::keymap::Overrides::parse(&value);
    ws.local_keymap = actions::resolve(&ws.settings.shortcut_file.overrides);
    ws.keymap_generation += 1;
}

/// The keymap host 0's server answered with: its bindings and the actions it offers.
fn server_map(ws: &mut Workspace, bindings: Vec<Binding>, actions: Vec<InspectResponseAction>) {
    let reply = rexrs::api::client::KeymapResponse {
        bindings,
        actions,
        ..Default::default()
    };
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

/// The sessions a host was asked to show, in order.
fn navigations(rx: &flume::Receiver<HostCommand>) -> Vec<(String, Option<String>)> {
    rx.try_iter()
        .filter_map(|c| match c {
            HostCommand::Navigate {
                session, window, ..
            } => Some((session, window)),
            _ => None,
        })
        .collect()
}

#[test]
fn the_command_list_is_pinned_and_shown_whole() {
    let (mut ws, ..) = fixture();
    ws.picker(false);
    let ids: Vec<String> = ws.commands().iter().map(|c| c.id.to_string()).collect();
    assert_eq!(ids, IDS, "see IDS before changing this list");
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len(), "ids are unique");
    // Nothing is held back until a search: the list shows every row.
    assert_eq!(
        labels(&ws),
        ws.commands()
            .into_iter()
            .map(|c| c.entry.label)
            .collect::<Vec<_>>()
    );
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn every_registered_action_has_a_row_or_a_stated_reason() {
    let (mut ws, ..) = fixture();
    ws.picker(false);
    let listed: std::collections::HashSet<&str> = ws
        .commands()
        .iter()
        .filter_map(|c| c.entry.action.shortcut().map(|(name, _)| name))
        .collect();
    for spec in actions::SPECS {
        let exempt = commands::NOT_LISTED
            .iter()
            .any(|(name, _)| *name == spec.name);
        assert!(
            listed.contains(spec.name) != exempt,
            "{}: give it a row in commands.rs, or say in NOT_LISTED why it has none (not both)",
            spec.name
        );
    }
    for (name, reason) in commands::NOT_LISTED {
        assert!(actions::find(name).is_some(), "{name} is not an action");
        assert!(!reason.is_empty());
    }
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn rows_sit_under_six_headings_that_name_what_is_on_screen() {
    let (mut ws, ..) = fixture();
    ws.picker(false);
    let mut headings: Vec<String> = Vec::new();
    for entry in ws.entries() {
        if headings.last() != Some(&entry.group) {
            headings.push(entry.group);
        }
    }
    assert_eq!(
        headings,
        [
            "Session · s-label",
            "Tab · dev",
            "Pane · editor",
            "Host · test-0",
            "View",
            "App"
        ]
    );
    assert_eq!(ws.panel.as_ref().unwrap().selected, 0);
    assert_eq!(ws.entries()[0].label, "Switch session…");

    // Offline with nothing chosen: no tab or pane to act on, and the headings say no more
    // than they know.
    let mut ws = workspace();
    ws.picker(false);
    let mut headings: Vec<String> = ws.entries().into_iter().map(|e| e.group).collect();
    headings.dedup();
    assert_eq!(headings, ["Session", "Tab", "Host · test-0", "View", "App"]);
    let enabled = |id: &str| command(&ws, id).entry.enabled;
    for id in [
        "session.new",
        "session.rename",
        "session.next",
        "session.refresh",
        "session.close",
        "tab.new",
        "tab.next",
    ] {
        assert!(!enabled(id), "{id} needs a connection");
    }
    for id in [
        "session.switch",
        "tab.next_global",
        "host.reconnect",
        "app.help",
    ] {
        assert!(enabled(id), "{id}");
    }
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn a_search_is_one_list_with_the_best_match_first() {
    let (mut ws, ..) = fixture();
    search(&mut ws, "close");
    let entries = ws.entries();
    assert!(entries.iter().all(|e| e.group == "Commands"));
    assert_eq!(
        labels(&ws),
        [
            "Close session…",
            "Close other tabs…",
            "Close tab…",
            "Close pane or tab…",
            "Close other panes…",
            "Close pane…",
            "Close app"
        ]
    );
    assert_eq!(ws.panel.as_ref().unwrap().selected, 0);

    // A label that starts with the query comes before one that only contains it.
    search(&mut ws, "new");
    assert_eq!(
        labels(&ws)[..2],
        ["New session on test-0".to_owned(), "New tab".to_owned()]
    );
    // The command named exactly comes before a longer one that starts the same way.
    search(&mut ws, "close pane");
    assert_eq!(
        labels(&ws),
        ["Close pane…", "Close pane or tab…", "Close other panes…"]
    );
    // Words in any order; each must be found.
    search(&mut ws, "size font");
    assert_eq!(
        labels(&ws),
        [
            "Increase terminal font size",
            "Decrease terminal font size",
            "Reset terminal font size"
        ]
    );
    // Other names for a command, and its group's name, find it too.
    search(&mut ws, "quit");
    assert_eq!(labels(&ws), ["Close app"]);
    search(&mut ws, "view");
    assert_eq!(labels(&ws).len(), 13, "every View command");
    // A shortcut is found as typed, not by its letters.
    search(&mut ws, "ctrl shift d");
    // The exact D shortcut ranks ahead of the Down prefix.
    assert_eq!(labels(&ws), ["Split right", "Next prompt"]);
    // What is on screen is not a keyword for the commands that act on it.
    search(&mut ws, "editor");
    assert!(labels(&ws).is_empty());
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn a_search_still_finds_sessions_and_tabs_after_the_commands() {
    let (mut ws, ..) = fixture();
    search(&mut ws, "t-label");
    let entries = ws.entries();
    assert_eq!(labels(&ws), ["t-label"]);
    assert_eq!(entries[0].group, "Sessions · all hosts");
    assert!(matches!(&entries[0].action, Intent::Select(0, id) if id == "t"));

    search(&mut ws, "dev");
    let entries = ws.entries();
    let groups: Vec<&str> = entries.iter().map(|e| e.group.as_str()).collect();
    assert_eq!(
        groups,
        ["Tabs · test-0 / s-label"],
        "the tab named dev; no command says dev"
    );
    assert!(matches!(&entries[0].action, Intent::Tab(id) if id == "split"));

    // Commands come first when both match.
    search(&mut ws, "server");
    let groups: Vec<String> = ws.entries().into_iter().map(|e| e.group).collect();
    assert_eq!(groups, ["Commands", "Tabs · test-0 / s-label"]);
    assert_eq!(labels(&ws), ["Add host…", "server"]);
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn hints_are_the_effective_bindings() {
    let (mut ws, ..) = fixture();
    let hint = |ws: &Workspace, id: &str| command(ws, id).entry.detail;
    for (id, keys) in [
        ("session.switch", "Ctrl K"),
        ("session.new", "Ctrl Shift N"),
        ("tab.next", "Ctrl PageDown"),
        ("tab.move_left", "Ctrl Shift PageUp"),
        ("pane.split.right", "Ctrl Shift D"),
        ("pane.focus.left", "Ctrl Alt Left"),
        ("pane.copy", "Ctrl Shift C"),
        ("pane.prompt_previous", "Ctrl Shift Up"),
        ("pane.prompt_next", "Ctrl Shift Down"),
        ("pane.close_focused", "Ctrl Shift W"),
        ("view.sidebar", "Ctrl Shift B"),
        // The main block's keys, not the numpad's.
        ("view.zoom.in", "Ctrl Plus"),
        ("view.zoom.out", "Ctrl -"),
        ("view.zoom.reset", "Ctrl 0"),
        ("app.settings", "Ctrl ,"),
        ("history.back", "Mouse Back"),
        // Unbound by default.
        ("session.next", ""),
        ("session.refresh", ""),
        ("session.close", ""),
        ("view.terminal_theme", ""),
        ("app.copy_version", ""),
    ] {
        assert_eq!(hint(&ws, id), keys, "{id}");
    }
    overrides(
        &mut ws,
        json!([
            {"keys":"ctrl+k","action":null},
            {"keys":"ctrl+alt+r","action":"session.refresh"},
            {"keys":"ctrl+alt+]","action":"session.next"},
            {"keys":"ctrl+b>t","action":"client.theme.choose"},
            {"keys":"ctrl+alt+q","action":"session.close"},
            {"keys":"ctrl+alt+h","action":"client.sidebar.collapse_others"},
            {"keys":"ctrl+alt+v","action":"client.version.copy"},
            {"keys":"ctrl+alt+i","action":"client.issue.report"}
        ]),
    );
    assert!(
        ws.local_keymap.diagnostics.is_empty(),
        "the new actions are bindable: {:?}",
        ws.local_keymap.diagnostics
    );
    for (id, keys) in [
        ("session.switch", ""),
        ("session.refresh", "Ctrl Alt R"),
        ("session.next", "Ctrl Alt ]"),
        ("view.terminal_theme", "Ctrl B, T"),
        ("session.close", "Ctrl Alt Q"),
        ("host.collapse_others", "Ctrl Alt H"),
        ("app.copy_version", "Ctrl Alt V"),
        ("app.report_issue", "Ctrl Alt I"),
    ] {
        assert_eq!(hint(&ws, id), keys, "{id}");
    }
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn next_and_previous_session_follow_the_sidebar_across_hosts_and_wrap() {
    let (mut ws, first, second) = fixture();
    let mut tx = rune::Tx::default();
    choose(&mut ws, "session.next");
    assert!(ws.panel.is_none());
    assert_eq!(navigations(&first), [("t".to_owned(), None)]);
    // Until the server answers, the session asked for is where the next step starts.
    run(&mut ws, commands::NEXT_SESSION);
    assert_eq!(navigations(&second), [("u".to_owned(), None)]);
    assert_eq!(ws.active, 1);
    run(&mut ws, commands::NEXT_SESSION);
    assert_eq!(navigations(&first), [("s".to_owned(), None)], "wraps");
    assert_eq!(ws.active, 0);
    run(&mut ws, commands::PREVIOUS_SESSION);
    assert_eq!(navigations(&second), [("u".to_owned(), None)]);

    // An offline host's sessions are passed over.
    let (mut ws, first, second) = fixture();
    ws.hosts[1].model.connected = false;
    ws.cycle_session(-1, &mut tx);
    assert_eq!(navigations(&first), [("t".to_owned(), None)]);
    assert!(navigations(&second).is_empty());

    // With one session there is nowhere to go, and Commands says so.
    let (mut ws, rx) = tabbed_workspace("single");
    ws.picker(false);
    for id in ["session.next", "session.previous"] {
        assert!(!command(&ws, id).entry.enabled, "{id}");
    }
    run(&mut ws, commands::NEXT_SESSION);
    assert!(navigations(&rx).is_empty());
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn a_session_stop_is_the_session_not_a_remembered_tab() {
    let (mut ws, first, _second) = fixture();
    ws.cycle_session(1, &mut rune::Tx::default());
    assert_eq!(
        ws.navigation.pending.as_ref().map(|p| p.visit.clone()),
        Some(Visit {
            host: ws.hosts[0].id,
            session: "t".into(),
            window: None
        })
    );
    assert_eq!(navigations(&first), [("t".to_owned(), None)]);
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn refresh_asks_each_connected_host_to_read_again_and_says_so() {
    let (mut ws, first, second) = fixture();
    choose(&mut ws, "session.refresh");
    for rx in [&first, &second] {
        let sent: Vec<HostCommand> = rx.try_iter().collect();
        assert_eq!(sent.len(), 1);
        assert!(matches!(sent[0], HostCommand::RefreshSessions));
    }
    assert_eq!(notices(&ws), ["Reading sessions again from 2 hosts"]);

    let (mut ws, first, second) = fixture();
    ws.hosts[1].model.connected = false;
    run(&mut ws, commands::REFRESH_SESSIONS);
    assert_eq!(first.try_iter().count(), 1);
    assert_eq!(
        second.try_iter().count(),
        0,
        "nothing is sent to an offline host"
    );
    assert_eq!(notices(&ws), ["Reading sessions again from test-0"]);
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn close_session_asks_first_and_names_the_session() {
    let (mut ws, first, _second) = fixture();
    choose(&mut ws, "session.close");
    let panel = ws.panel.as_ref().expect("a confirmation");
    assert_eq!(panel.title, "Close s-label?");
    let PanelKind::Confirm {
        method, payload, ..
    } = &panel.kind
    else {
        panic!("not a confirmation");
    };
    assert_eq!(method, "session.destroy");
    assert_eq!(payload, &json!({"session_id":"s"}));
    assert_eq!(
        panel.session.as_ref().map(|s| s.session.as_str()),
        Some("s")
    );
    assert_eq!(
        first.try_iter().count(),
        0,
        "nothing is sent before the answer"
    );

    // The action does the same from a key, and nothing while the host is away.
    ws.panel = None;
    run(&mut ws, commands::CLOSE_SESSION);
    assert!(matches!(
        ws.panel.as_ref().unwrap().kind,
        PanelKind::Confirm { .. }
    ));
    ws.panel = None;
    ws.hosts[0].model.connected = false;
    run(&mut ws, commands::CLOSE_SESSION);
    assert!(ws.panel.is_none());
    assert!(!command(&ws, "session.close").entry.enabled);
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn collapse_other_hosts_folds_the_sidebar_to_the_host_on_screen() {
    let (mut ws, ..) = fixture();
    assert!(
        !command(&ws, "host.collapse_others").entry.enabled,
        "nothing to see with the sidebar closed"
    );
    ws.toggle_sidebar();
    choose(&mut ws, "host.collapse_others");
    let collapsed: Vec<bool> = ws.settings.hosts.iter().map(|h| h.collapsed).collect();
    assert_eq!(collapsed, [false, true]);
    assert!(
        relaunch(&ws).settings.hosts[1].collapsed,
        "saved with the workspace"
    );
    assert!(
        !command(&ws, "host.collapse_others").entry.enabled,
        "already folded"
    );
    // Showing one of its sessions opens a folded host again.
    ws.select(1, "u".into(), &mut rune::Tx::default());
    assert!(!ws.settings.hosts[1].collapsed);
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn the_theme_pickers_open_from_commands_and_close_without_settings() {
    let (mut ws, ..) = fixture();
    let mut tx = rune::Tx::default();
    choose(&mut ws, "view.terminal_theme");
    let panel = ws.panel.as_ref().expect("the theme list");
    assert!(matches!(panel.kind, PanelKind::Themes));
    assert!(panel.back.is_none());
    let follow = ws
        .entries()
        .into_iter()
        .find(|e| e.label == "Rex default colours")
        .unwrap();
    ws.perform(follow.action, &mut tx);
    assert!(ws.panel.is_none(), "a choice closes the list");
    assert_eq!(ws.settings.terminal_theme, TerminalThemeChoice::RexDefault);

    choose(&mut ws, "view.interface_theme");
    assert!(matches!(
        ws.panel.as_ref().unwrap().kind,
        PanelKind::Chromes
    ));
    ws.cancel_panel(&mut tx);
    assert!(ws.panel.is_none(), "Escape leaves, not back to Settings");

    // The action opens the same list from a key.
    run(&mut ws, commands::CHOOSE_THEME);
    assert!(matches!(ws.panel.as_ref().unwrap().kind, PanelKind::Themes));
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn view_commands_name_the_change_they_make() {
    let (mut ws, ..) = fixture();
    let label = |ws: &Workspace, id: &str| command(ws, id).entry.label;
    assert_eq!(label(&ws, "view.tab_position"), "Show tabs in sidebar");
    choose(&mut ws, "view.tab_position");
    assert!(ws.settings.vertical_tabs && ws.sidebar.is_open());
    assert_eq!(label(&ws, "view.tab_position"), "Show tabs in toolbar");
    assert_eq!(label(&ws, "view.sidebar"), "Hide sidebar");
    choose(&mut ws, "view.tab_position");
    assert!(!ws.settings.vertical_tabs);

    assert_eq!(label(&ws, "view.pane_density"), "Use compact panes");
    choose(&mut ws, "view.pane_density");
    assert_eq!(ws.settings.pane_density, PaneDensity::Compact);
    assert_eq!(label(&ws, "view.pane_density"), "Use comfortable panes");
    assert_eq!(relaunch(&ws).settings.pane_density, PaneDensity::Compact);
    choose(&mut ws, "view.pane_density");
    assert_eq!(ws.settings.pane_density, PaneDensity::Comfortable);

    // A zoom step that would change nothing is disabled, as the font steps are.
    let zoom = |ws: &Workspace| {
        ["view.zoom.in", "view.zoom.out", "view.zoom.reset"].map(|id| command(ws, id).entry.enabled)
    };
    assert_eq!(zoom(&ws), [true, true, false]);
    choose(&mut ws, "view.zoom.in");
    assert_eq!(ws.settings.ui_scale, 1.1);
    assert_eq!(zoom(&ws), [true, true, true]);
    ws.settings.ui_scale = 1.5;
    assert_eq!(zoom(&ws), [false, true, true]);
    ws.settings.ui_scale = 0.7;
    assert_eq!(zoom(&ws), [true, false, true]);
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn app_commands_copy_the_version_and_open_the_issue_form() {
    let (ws, ..) = fixture();
    let mut rt = runtime(ws);
    let clipboard = rune::clipboard::memory();
    rt.use_clipboard(&clipboard);
    let perform = |rt: &mut Runtime<Workspace>, id: &str| {
        rt.update(|s, tx| {
            s.picker(false);
            let row = command(s, id);
            s.perform(row.entry.action, tx);
        });
        rt.frame(1. / 60.);
    };
    perform(&mut rt, "app.copy_version");
    assert_eq!(
        clipboard.text().as_deref().map(String::as_str),
        Some(about::version_info(&rt.state).as_str())
    );
    assert_eq!(notices(&rt.state), ["Copied version info to clipboard."]);
    assert!(rt.state.panel.is_none());

    perform(&mut rt, "app.report_issue");
    assert_eq!(
        rt.state.opened_links,
        ["https://github.com/shuv1337/rex-rune/issues/new"]
    );
    assert_eq!(rt.state.opened_links, [commands::issue_url()]);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

fn offered() -> Vec<InspectResponseAction> {
    vec![
        InspectResponseAction::new("dev_layout")
            .kind("lua")
            .title("Dev layout")
            .category("Layouts")
            .keywords("editor tests"),
        // Needs an argument Commands cannot ask for.
        {
            let mut action = InspectResponseAction::new("open_project")
                .kind("lua")
                .title("Open project");
            action.args = Some(
                json!({"type":"object","properties":{"name":{"type":"string"}},
                "required":["name"]}),
            );
            action
        },
        InspectResponseAction::new("com.example.notes.pin")
            .kind("block")
            .title("Pin note"),
        // A block type's method: an API, not a command.
        InspectResponseAction::new("com.example.notes.read").kind("block"),
        // A command already offers this by the same name.
        InspectResponseAction::new("com.example.terminal.zoom")
            .kind("block")
            .title("Zoom Pane"),
        // What a stock server offers every terminal. The Pane group's own rows do the same
        // through `terminal.clear` and `terminal.reset`.
        InspectResponseAction::new("com.superlogical.terminal.clear")
            .kind("block")
            .title("Clear Terminal"),
        InspectResponseAction::new("com.superlogical.terminal.reset")
            .kind("block")
            .title("Reset Terminal"),
        // This client's own actions come back in the list too.
        InspectResponseAction::new("client.settings")
            .kind("client")
            .title("Settings"),
    ]
}

#[test]
fn the_hosts_own_actions_are_listed_after_its_commands() {
    let (mut ws, ..) = fixture();
    server_map(
        &mut ws,
        vec![Binding::new(["ctrl+alt+l"], "dev_layout")],
        offered(),
    );
    ws.picker(false);
    let server: Vec<commands::Command> = ws
        .commands()
        .into_iter()
        .filter(|c| c.group == commands::Group::Server)
        .collect();
    assert_eq!(
        server
            .iter()
            .map(|c| (
                c.id.as_ref(),
                c.entry.label.as_str(),
                c.entry.detail.as_str()
            ))
            .collect::<Vec<_>>(),
        [
            ("server:dev_layout", "Dev layout", "Ctrl Alt L"),
            ("server:com.example.notes.pin", "Pin note", ""),
        ]
    );
    // The stock server's Clear Terminal and Reset Terminal are the Pane group's rows, once.
    let all = ws.commands();
    for (id, label) in [
        ("pane.clear", "Clear terminal"),
        ("pane.reset", "Reset terminal"),
    ] {
        let rows: Vec<_> = all
            .iter()
            .filter(|c| c.entry.label.eq_ignore_ascii_case(label))
            .map(|c| (c.id.as_ref(), c.group))
            .collect();
        assert_eq!(rows, [(id, commands::Group::Pane)], "{label}");
    }
    let mut headings: Vec<String> = ws.entries().into_iter().map(|e| e.group).collect();
    headings.dedup();
    assert_eq!(
        headings[3..5],
        [
            "Host · test-0".to_owned(),
            "Server actions · test-0".to_owned()
        ]
    );
    // Found by the server's own keywords and category, and by the action's name.
    for query in ["layouts", "tests", "dev_layout"] {
        search(&mut ws, query);
        assert_eq!(labels(&ws), ["Dev layout"], "{query}");
    }

    // Another host's actions are not this one's.
    ws.active = 1;
    assert!(
        ws.commands()
            .iter()
            .all(|c| c.group != commands::Group::Server)
    );
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn a_server_action_runs_once_on_its_host_for_the_pane_on_screen() {
    let (mut ws, first, second) = fixture();
    server_map(&mut ws, Vec::new(), offered());
    choose(&mut ws, "server:dev_layout");
    assert!(ws.panel.is_none());
    let sent: Vec<HostCommand> = first.try_iter().collect();
    assert_eq!(sent.len(), 1);
    let HostCommand::Action {
        generation,
        request,
    } = &sent[0]
    else {
        panic!("not an action");
    };
    assert_eq!(*generation, ws.keymap_generation);
    assert_eq!(
        serde_json::to_value(request).unwrap(),
        json!({"action":"dev_layout","origin":"palette","session_id":"s","block_id":"block:b"})
    );
    assert_eq!(second.try_iter().count(), 0);

    // A block action needs a pane to act on; one from the configuration file does not.
    ws.hosts[0].model.selected = None;
    assert!(!command(&ws, "server:com.example.notes.pin").entry.enabled);
    assert!(command(&ws, "server:dev_layout").entry.enabled);
    ws.picker(false);
    let row = command(&ws, "server:dev_layout");
    ws.perform(row.entry.action.clone(), &mut rune::Tx::default());
    let sent: Vec<HostCommand> = first.try_iter().collect();
    let HostCommand::Action { request, .. } = &sent[0] else {
        panic!("not an action");
    };
    assert_eq!(
        serde_json::to_value(request).unwrap(),
        json!({"action":"dev_layout","origin":"palette"})
    );
    assert!(matches!(
        row.entry.action,
        Intent::Command(Run::ServerAction(_))
    ));
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn the_list_shows_about_twenty_rows_and_scrolls_to_the_rest() {
    let (ws, ..) = fixture();
    let mut rt = runtime_at(ws, (1440., 1000.));
    click(&mut rt, "commands");
    for _ in 0..30 {
        rt.frame(1. / 60.);
    }
    let rect = |rt: &Runtime<Workspace>, key: &str| {
        rt.elements()
            .into_iter()
            .find(|e| !e.exiting && e.key.as_deref() == Some(key))
            .unwrap_or_else(|| panic!("missing {key}"))
            .rect
    };
    let texts = |rt: &Runtime<Workspace>| -> Vec<String> {
        rt.elements().into_iter().filter_map(|e| e.text).collect()
    };
    let viewport = rect(&rt, "results-scroll");
    assert_eq!(
        viewport[3].round(),
        620.,
        "twenty 31 pt rows, with room to spare"
    );
    let content = rect(&rt, "results");
    assert!(
        content[3] > viewport[3] * 2.,
        "the whole list is there to scroll to: {content:?}"
    );
    let words = texts(&rt);
    // A heading's words are small capitals and the name in it keeps its case, so it is read
    // by its accessible name.
    let headings = |rt: &Runtime<Workspace>| -> Vec<String> {
        rt.elements()
            .into_iter()
            .filter(|e| !e.exiting && e.role == "heading")
            .filter_map(|e| e.label)
            .collect()
    };
    for heading in ["Session · s-label", "Tab · dev", "View", "App"] {
        assert!(headings(&rt).iter().any(|w| w == heading), "{heading}");
    }
    for (caps, name) in [("Session ·", "s-label"), ("Tab ·", "dev")] {
        assert!(
            words.iter().any(|w| w == caps) && words.iter().any(|w| w == name),
            "{caps} is drawn apart from {name}, which keeps its case"
        );
    }
    assert!(
        words
            .iter()
            .any(|w| w == "More below: Pane · Host · View · App"),
        "the hint names the groups further down: {words:?}"
    );
    let new_tab = rt
        .state
        .entries()
        .iter()
        .position(|e| e.label == "New tab")
        .expect("a New tab row");
    let row = rect(&rt, &format!("entry:{new_tab}"));
    assert!(
        row[1] + row[3] <= viewport[1] + viewport[3],
        "Tab's first row is on screen, so Tab is not named as below: {row:?}"
    );

    // Down from the last row on screen scrolls the next one into view.
    for _ in 0..25 {
        rt.key(KeyEvent {
            key: Key::Down,
            mods: Mods::default(),
        });
        rt.frame(1. / 60.);
        // The app's poll hook reveals the selection; the headless runtime has no hooks.
        rt.update(|s, tx| {
            crate::app::poll(s, 0., tx);
        });
    }
    for _ in 0..30 {
        rt.frame(1. / 60.);
    }
    let selected = rt.state.panel.as_ref().unwrap().selected;
    assert!(selected > 20, "past the first screen: {selected}");
    let row = rect(&rt, &format!("entry:{selected}"));
    let viewport = rect(&rt, "results-scroll");
    assert!(
        row[1] >= viewport[1] - 1. && row[1] + row[3] <= viewport[1] + viewport[3] + 1.,
        "the selection is in view: {row:?} in {viewport:?}"
    );

    // A search drops the headings for one list.
    rt.update(|s, _| {
        s.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("close");
        s.query_changed();
    });
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
    let words = texts(&rt);
    assert!(words.iter().any(|w| w == "Commands"));
    assert!(!headings(&rt).iter().any(|w| w == "Session · s-label"));
    let short = rect(&rt, "results-scroll");
    assert!(short[3] < 250., "six rows need no scrolling: {short:?}");
    std::fs::remove_file(&rt.state.settings_path).ok();
}
