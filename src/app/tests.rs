//! Headless focus and key-routing checks on Rune's runtime (no server: the one host is a
//! socket that does not exist).
#[path = "bindings_integration_tests.rs"]
mod bindings_integration_tests;
#[path = "downloads_tests.rs"]
mod downloads_tests;
#[path = "navigation_tests.rs"]
mod navigation_tests;
#[path = "remote_session_tests.rs"]
mod remote_session_tests;
#[path = "remote_tests.rs"]
mod remote_tests;
use super::{
    panel::{Form, Intent, PanelKind, SessionAction},
    placeholder, view,
    workspace::Workspace,
};
use clap::Parser;
use rex_linux::{
    connection::{HostCommand, HostConnection, Options},
    settings::{Host, Settings, TerminalThemeChoice},
    terminal_theme::ThemeDirs,
};
use rune::{AppSpec, Env, Key, KeyEvent, Mods, Runtime};

fn workspace() -> Workspace {
    workspace_hosts(1)
}

fn workspace_hosts(count: usize) -> Workspace {
    let endpoint = "unix:///nonexistent/rex-test.sock";
    let options = Options::parse_from(["rex-linux", "--endpoint", endpoint]);
    let settings = Settings {
        hosts: (0..count)
            .map(|i| Host {
                label: format!("test-{i}"),
                endpoint: if i == 0 {
                    endpoint.into()
                } else {
                    format!("unix:///nonexistent/rex-test-{i}.sock")
                },
                remote_control: false,
                collapsed: false,
                shell: None,
            })
            .collect(),
        ..Settings::default()
    };
    let handle = rune::App::new(AppSpec::new("Rex", (1024., 700.), placeholder, view)).handle();
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "rex-linux-test-{}-{}.json",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    Workspace::new(options, settings, path, handle, None)
}

/// A second launch on the settings `ws` saved.
fn relaunch(ws: &Workspace) -> Workspace {
    let options = Options::parse_from([
        "rex-linux",
        "--endpoint",
        "unix:///nonexistent/rex-test.sock",
    ]);
    let settings = Settings::load(&ws.settings_path).unwrap();
    let handle = rune::App::new(AppSpec::new("Rex", (1024., 700.), placeholder, view)).handle();
    Workspace::new(options, settings, ws.settings_path.clone(), handle, None)
}

#[test]
fn the_sidebar_opens_at_launch_as_it_was_left() {
    let mut ws = workspace();
    assert!(!ws.sidebar.is_open(), "closed on a first launch");
    ws.toggle_sidebar();
    assert!(relaunch(&ws).sidebar.is_open(), "left open, opens again");
    ws.toggle_sidebar();
    assert!(
        !relaunch(&ws).sidebar.is_open(),
        "left closed, stays closed"
    );
    std::fs::remove_file(&ws.settings_path).ok();
}

fn click(rt: &mut Runtime<Workspace>, key: &str) {
    let r = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("missing {key}"))
        .rect;
    let (x, y) = (r[0] + r[2] / 2., r[1] + r[3] / 2.);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
}

/// Every session a host was asked to create, as (label, shell).
fn creates(rx: &flume::Receiver<HostCommand>) -> Vec<(Option<String>, Option<String>)> {
    rx.try_iter()
        .filter_map(|c| match c {
            HostCommand::CreateSession { label, shell } => Some((label, shell)),
            _ => None,
        })
        .collect()
}

fn has(rt: &Runtime<Workspace>, key: &str) -> bool {
    rt.elements()
        .iter()
        .any(|e| !e.exiting && e.key.as_deref() == Some(key))
}

#[test]
fn host_plus_creates_on_its_own_host_at_once_with_its_shell() {
    let mut ws = workspace_hosts(3);
    ws.sidebar.toggle();
    let active = capture_host(&mut ws, 0);
    let other = capture_host(&mut ws, 1);
    let offline = capture_host(&mut ws, 2);
    ws.hosts[0].model.connected = true;
    ws.hosts[0].model.sessions = vec![session("cur")];
    ws.hosts[0].model.selected = Some("cur".into());
    ws.hosts[1].model.connected = true;
    ws.hosts[1].model.sessions = vec![session("b")];
    ws.hosts[2].model.sessions = vec![session("away")];
    ws.settings.hosts[1].shell = Some("zsh".into());
    ws.settings.hosts[1].collapsed = true;
    let mut rt = runtime(ws);
    assert!(has(&rt, "host-new-session:0") && has(&rt, "host-new-session:1"));
    assert!(!has(&rt, "host-new-session:2"), "offline hosts have no +");
    assert!(has(&rt, "host-new-session-slot:2"), "but keep its slot");

    click(&mut rt, "host-new-session:1");
    assert_eq!(creates(&other), [(None, Some("zsh".to_owned()))]);
    assert!(
        creates(&active).is_empty(),
        "nothing goes to the active host"
    );
    assert_eq!(rt.state.active, 1);
    assert!(rt.state.panel.is_none());
    assert!(
        !rt.state.settings.hosts[1].collapsed,
        "a collapsed host expands"
    );
    assert_eq!(focused(&rt), None, "the next key goes to the terminal");

    let mut tx = rune::Tx::default();
    rt.state.create_session(2, None, &mut tx);
    assert!(creates(&offline).is_empty(), "offline hosts create nothing");
    assert_eq!(rt.state.active, 1);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn host_menu_rows_online_and_offline() {
    let mut ws = workspace_hosts(2);
    ws.sidebar.toggle();
    let other = capture_host(&mut ws, 0);
    ws.hosts[0].model.connected = true;
    ws.hosts[0].model.sessions = vec![session("cur")];
    ws.settings.hosts[0].shell = Some("zsh".into());
    let mut rt = runtime(ws);
    let rows = |rt: &Runtime<Workspace>| -> Vec<(String, String, bool)> {
        rt.state
            .entries()
            .into_iter()
            .map(|e| (e.label, e.detail, e.enabled))
            .collect()
    };

    // A right-click on the + opens the menu; it never creates.
    secondary_click(&mut rt, "host-new-session:0");
    let panel = rt.state.panel.as_ref().expect("host menu");
    assert!(matches!(panel.kind, PanelKind::Host(0)));
    assert_eq!(panel.title, "test-0");
    assert!(creates(&other).is_empty());
    assert_eq!(
        rows(&rt),
        [
            ("New session".to_owned(), String::new(), true),
            ("Default shell…".to_owned(), "zsh".to_owned(), true),
            ("Edit host…".to_owned(), String::new(), true),
            ("Remove host…".to_owned(), String::new(), true),
        ]
    );
    press(&mut rt, Key::Escape);

    secondary_click(&mut rt, "host:1");
    assert_eq!(rt.state.panel.as_ref().unwrap().title, "test-1 · Offline");
    assert_eq!(
        rows(&rt),
        [
            ("New session".to_owned(), String::new(), false),
            ("Default shell…".to_owned(), String::new(), true),
            ("Retry connection".to_owned(), String::new(), true),
            ("Edit host…".to_owned(), String::new(), true),
            ("Remove host…".to_owned(), String::new(), true),
        ]
    );
    assert_eq!(rt.state.panel.as_ref().unwrap().selected, 1);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_host_menu_edits_a_host_in_place() {
    let mut ws = workspace_hosts(2);
    ws.sidebar.toggle();
    let mut rt = runtime(ws);
    secondary_click(&mut rt, "host:1");
    let index = rt
        .state
        .entries()
        .iter()
        .position(|e| matches!(e.action, Intent::EditHost(1)))
        .expect("Edit host…");
    rt.state.panel.as_mut().unwrap().selected = index;
    press(&mut rt, Key::Enter);
    let panel = rt.state.panel.as_ref().expect("the edit form");
    assert!(matches!(panel.kind, PanelKind::Form(Form::EditHost(1))));
    assert_eq!(panel.inputs[0].text, "test-1");
    assert_eq!(panel.inputs[1].text, rt.state.settings.hosts[1].endpoint);

    let endpoint = "https://test-1.example.ts.net:8443".to_owned();
    rt.state.panel.as_mut().unwrap().inputs[1] = rune::TextEdit::new(&endpoint);
    click(&mut rt, "save-form");
    assert!(rt.state.panel.is_none(), "saving closes the form");
    assert_eq!(rt.state.settings.hosts[1].endpoint, endpoint);
    assert_eq!(rt.state.settings.hosts[1].label, "test-1");

    // The only host can be edited but not removed.
    rt.state.settings.hosts.truncate(1);
    rt.state.hosts.truncate(1);
    secondary_click(&mut rt, "host:0");
    let remove = rt
        .state
        .entries()
        .into_iter()
        .find(|e| matches!(e.action, Intent::RemoveHost(0)))
        .expect("Remove host…");
    assert!(!remove.enabled);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_empty_host_row_collapses_with_its_host() {
    let mut ws = workspace_hosts(2);
    ws.sidebar.toggle();
    capture_host(&mut ws, 1);
    ws.hosts[1].model.connected = true;
    let mut rt = runtime(ws);
    assert!(has(&rt, "new-session-on:1"));
    click(&mut rt, "host-disclosure:1");
    // Let the leaving row finish its exit.
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
    assert!(rt.state.settings.hosts[1].collapsed);
    assert!(!has(&rt, "new-session-on:1"), "collapsing hides the row");
    assert!(has(&rt, "host-new-session:1"), "the header + stays");
    click(&mut rt, "host-disclosure:1");
    assert!(has(&rt, "new-session-on:1"), "expanding brings it back");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn empty_host_row_creates_in_one_step() {
    let mut ws = workspace_hosts(2);
    ws.sidebar.toggle();
    let other = capture_host(&mut ws, 1);
    ws.hosts[1].model.connected = true;
    let mut rt = runtime(ws);
    assert!(!has(&rt, "new-session-on:0"), "offline hosts keep retry");
    click(&mut rt, "new-session-on:1");
    assert_eq!(creates(&other), [(None, None)]);
    assert_eq!(rt.state.active, 1);
    assert!(rt.state.panel.is_none());
    assert_eq!(focused(&rt), None);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn only_a_switcher_query_names_the_new_session() {
    let mut ws = workspace_hosts(2);
    let active = capture_host(&mut ws, 0);
    let other = capture_host(&mut ws, 1);
    ws.hosts[0].model.connected = true;
    ws.hosts[1].model.connected = true;
    let mut tx = rune::Tx::default();

    ws.picker(true);
    ws.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("api");
    let footer: Vec<_> = ws.entries().into_iter().filter(|e| e.footer).collect();
    assert_eq!(footer[0].label, "New session \"api\" on test-0");
    assert_eq!(footer[0].detail, "", "no shortcut hint for a named create");
    assert_eq!(footer[1].label, "Create \"api\" on another host…");
    ws.perform(footer[0].action.clone(), &mut tx);
    assert_eq!(creates(&active), [(Some("api".to_owned()), None)]);
    assert!(ws.panel.is_none());

    // Another host via the chooser keeps the name and says so.
    ws.picker(true);
    ws.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("api");
    ws.perform(Intent::ChooseHost("api".into()), &mut tx);
    assert_eq!(ws.panel.as_ref().unwrap().title, "Create \"api\" on…");
    let host = ws.entries().remove(1);
    ws.perform(host.action, &mut tx);
    assert_eq!(creates(&other), [(Some("api".to_owned()), None)]);

    // Commands' query is a search, never a name.
    ws.picker(false);
    ws.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("new ses");
    let entry = ws
        .entries()
        .into_iter()
        .find(|e| e.label.starts_with("New session on"))
        .expect("Commands create");
    assert_eq!(entry.label, "New session on test-1");
    ws.perform(entry.action, &mut tx);
    assert_eq!(creates(&other), [(None, None)]);
    assert!(creates(&active).is_empty());
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn every_connected_host_offers_a_new_session_in_the_switcher() {
    let mut ws = workspace_hosts(2);
    capture_host(&mut ws, 0);
    let other = capture_host(&mut ws, 1);
    ws.hosts[0].model.connected = true;
    ws.hosts[1].model.connected = true;
    let mut tx = rune::Tx::default();

    ws.picker(true);
    let rows: Vec<_> = ws.entries().into_iter().filter(|e| !e.footer).collect();
    for host in 0..2 {
        let row = rows
            .iter()
            .find(|e| matches!(e.action, Intent::CreateOn(h, None) if h == host))
            .expect("an empty host has a New session row");
        assert_eq!(row.label, "New session");
        assert!(row.enabled);
        assert!(
            matches!(&row.group_create, Some((Intent::CreateOn(h, None), help))
                if *h == host && help == &format!("New session on test-{host}")),
            "the host header carries a create button"
        );
    }
    let row = rows
        .into_iter()
        .find(|e| matches!(e.action, Intent::CreateOn(1, _)))
        .unwrap();
    ws.perform(row.action, &mut tx);
    assert_eq!(creates(&other), [(None, None)]);

    // A typed name carries into both the row and the header button.
    ws.picker(true);
    ws.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("test-1");
    let row = ws
        .entries()
        .into_iter()
        .find(|e| matches!(e.action, Intent::CreateOn(1, _)) && !e.footer)
        .unwrap();
    assert_eq!(row.label, "New session \"test-1\"");
    assert!(matches!(
        &row.group_create,
        Some((Intent::CreateOn(1, Some(name)), _)) if name == "test-1"
    ));
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn default_shell_form_trims_clears_and_rejects_arguments() {
    let mut ws = workspace_hosts(2);
    let mut tx = rune::Tx::default();
    let set = |ws: &mut Workspace, text: &str| {
        ws.perform(Intent::HostShell(1), &mut rune::Tx::default());
        let panel = ws.panel.as_mut().unwrap();
        assert!(matches!(panel.kind, PanelKind::Form(Form::HostShell(1))));
        panel.inputs[0] = rune::TextEdit::new(text);
        ws.submit(&mut rune::Tx::default());
    };
    set(&mut ws, "  zsh ");
    assert_eq!(ws.settings.hosts[1].shell.as_deref(), Some("zsh"));
    assert!(ws.settings.hosts[0].shell.is_none(), "only that host");
    for bad in ["zsh -l", "zsh\u{7}"] {
        set(&mut ws, bad);
        assert_eq!(
            ws.panel.as_ref().unwrap().error.as_deref(),
            Some("Enter one shell name or path, without arguments.")
        );
        ws.cancel_panel(&mut tx);
        assert_eq!(ws.settings.hosts[1].shell.as_deref(), Some("zsh"));
    }
    ws.perform(Intent::HostShell(1), &mut tx);
    assert_eq!(
        ws.panel.as_ref().unwrap().inputs[0].text,
        "zsh",
        "prefilled"
    );
    ws.cancel_panel(&mut tx);
    set(&mut ws, "");
    assert!(ws.settings.hosts[1].shell.is_none(), "empty clears it");
    let saved = std::fs::read_to_string(&ws.settings_path).unwrap();
    assert!(!saved.contains("\"shell\""), "no shell key once cleared");
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn tabs_and_splits_start_the_right_hosts_shell() {
    let mut ws = workspace_hosts(2);
    let active = capture_host(&mut ws, 0);
    let other = capture_host(&mut ws, 1);
    ws.hosts[0].model.connected = true;
    ws.hosts[0].model.selected = Some("cur".into());
    ws.hosts[0].model.views.insert(
        "cur".into(),
        serde_json::from_value(serde_json::json!({
            "revision": 1, "session_id": "cur", "active_window_id": "w",
            "windows": [{"window_id": "w", "focused_block_id": "block:f",
                "layers": [{"kind": "tiled", "blocks": [{"block_id": "block:f"}]}]}]
        }))
        .unwrap(),
    );
    ws.settings.hosts[0].shell = Some("zsh".into());
    ws.settings.hosts[1].shell = Some("fish".into());
    let mut tx = rune::Tx::default();
    ws.perform(Intent::NewWindow, &mut tx);
    let sent = requests(&active);
    assert_eq!(sent[0].0, "session.new_window");
    assert_eq!(sent[0].1, "cur");
    assert_eq!(
        sent[0].2["cwd_from"], "block:f",
        "new tab uses the focused pane"
    );
    assert_eq!(
        sent[0].2["layout"]["block"]["options"],
        serde_json::json!({"shell":"zsh"}),
        "Follow server gives new blocks no theme"
    );
    ws.term_theme = Some(rex_linux::terminal_theme::TerminalTheme::midnight());
    let target = super::panel::PaneTarget {
        host: 1,
        session: "b".into(),
        block: "block:1".into(),
        label: "shell".into(),
        name: "shell".into(),
        tab: "Shell".into(),
    };
    ws.hosts[1].model.views.insert(
        "b".into(),
        serde_json::from_value(serde_json::json!({
            "revision": 1,
            "session_id": "b",
            "windows": [{"window_id": "w", "layers": [{"kind": "tiled", "blocks": [{"block_id": "block:1"}]}]}]
        }))
        .expect("a view"),
    );
    ws.perform(
        Intent::Pane(target, super::panel::PaneAction::Split("horizontal")),
        &mut tx,
    );
    let sent = requests(&other);
    assert_eq!(sent[0].0, "session.new_split");
    assert_eq!(sent[0].1, "b", "pane menu keeps its captured session");
    assert_eq!(
        sent[0].2["cwd_from"], "block:1",
        "pane menu uses its target, not focus"
    );
    assert_eq!(
        sent[0].2["layout"]["block"]["options"],
        serde_json::json!({"shell":"fish","theme":{"background":"#001528","scheme":"dark"}}),
        "a declared theme goes into new blocks' layouts"
    );
    std::fs::remove_file(&ws.settings_path).ok();
}

/// Host 0 connected on session "s" with a one-pane tab "single" and a split tab "split".
fn tabbed_workspace(active: &str) -> (Workspace, flume::Receiver<HostCommand>) {
    let mut ws = workspace();
    let rx = capture_host(&mut ws, 0);
    let model = &mut ws.hosts[0].model;
    model.connected = true;
    model.sessions = vec![session("s")];
    model.selected = Some("s".into());
    model.views.insert(
        "s".into(),
        serde_json::from_value(serde_json::json!({
            "revision": 1,
            "session_id": "s",
            "active_window_id": active,
            "windows": [
                {"window_id": "single", "label": "server", "focused_block_id": "block:a",
                 "layers": [{"kind": "tiled", "layout": {"block_id": "block:a"},
                    "blocks": [{"block_id": "block:a"}]}]},
                {"window_id": "split", "label": "dev", "focused_block_id": "block:b",
                 "layers": [{"kind": "tiled", "layout": {"split": {"direction": "horizontal", "ratio": 0.5,
                    "before": {"block_id": "block:b"}, "after": {"block_id": "block:c"}}},
                    "blocks": [{"block_id": "block:b"}, {"block_id": "block:c"}]}]}
            ]
        }))
        .expect("a view"),
    );
    (ws, rx)
}

fn ctrl_shift(rt: &mut Runtime<Workspace>, c: char) {
    rt.key(KeyEvent {
        key: Key::Text(c.to_ascii_uppercase().to_string()),
        mods: Mods {
            ctrl: true,
            shift: true,
            ..Mods::default()
        },
    });
    for _ in 0..3 {
        rt.frame(1. / 60.);
    }
}

#[test]
fn a_single_pane_is_full_bleed_and_a_split_keeps_its_cards() {
    let (ws, _rx) = tabbed_workspace("single");
    let mut rt = runtime(ws);
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
    assert!(has(&rt, "pane:block:a"));
    assert!(!has(&rt, "pane-header:block:a"), "no header on a lone pane");
    let r = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("pane:block:a"))
        .unwrap()
        .rect;
    assert_eq!((r[0], r[2]), (0., 1024.), "edge to edge: {r:?}");
    assert!(!has(&rt, "close-tab:single"), "tabs have no close button");
    std::fs::remove_file(&rt.state.settings_path).ok();

    let (ws, _rx) = tabbed_workspace("split");
    let rt = runtime(ws);
    assert!(has(&rt, "pane-header:block:b") && has(&rt, "pane-header:block:c"));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn compact_separators_are_straight_full_length_lines_in_both_directions() {
    for vertical in [false, true] {
        for dragging in [false, true] {
            let mut rt = Runtime::new(
                (vertical, dragging),
                |&(vertical, dragging), _| {
                    super::panes::divider_handle(
                        rune::el().size(100., 100.),
                        vertical,
                        dragging,
                        true,
                    )
                },
                (100., 100.),
                Env::new(true, 1.),
            );
            rt.frame(0.);
            let line = rt
                .elements()
                .into_iter()
                .find(|e| e.key.as_deref() == Some("divider-line"))
                .unwrap();
            assert_eq!(
                line.rect,
                if vertical {
                    [5., 0., 1., 100.]
                } else {
                    [0., 5., 100., 1.]
                }
            );
        }
    }
}

#[test]
fn pane_density_switch_saves_and_keeps_split_geometry_and_reconnect_controls() {
    use rex_linux::settings::PaneDensity;
    let (mut ws, _rx) = tabbed_workspace("split");
    let (sender, _terminal_rx) = flume::unbounded();
    live_terminal(&mut ws, "block:b", &sender);
    live_terminal(&mut ws, "block:c", &sender);
    ws.terminals[&ws.term_key("block:c")]
        .connection
        .model
        .lock()
        .unwrap()
        .connected = false;
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    let mut rt = runtime(ws);
    assert!(has(&rt, "pane-density"));
    for (label, density) in [
        ("Compact", PaneDensity::Compact),
        ("Comfortable", PaneDensity::Comfortable),
        ("Compact", PaneDensity::Compact),
    ] {
        let r = rt
            .elements()
            .into_iter()
            .find(|e| e.text.as_deref() == Some(label))
            .unwrap()
            .rect;
        let (x, y) = (r[0] + r[2] / 2., r[1] + r[3] / 2.);
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        for _ in 0..4 {
            rt.frame(1. / 60.);
        }
        assert_eq!(rt.state.settings.pane_density, density);
        assert_eq!(
            Settings::load(&rt.state.settings_path)
                .unwrap()
                .pane_density,
            density
        );
    }
    press(&mut rt, Key::Escape);
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
    assert!(!has(&rt, "pane-header:block:b") && !has(&rt, "pane-header:block:c"));
    assert!(has(&rt, "strip-action:block:c"), "Retry remains available");
    let rect = |key: &str| {
        rt.elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some(key))
            .unwrap()
            .rect
    };
    let left = rect("pane:block:b");
    let right = rect("pane:block:c");
    assert_eq!(left[0], 0.);
    assert_eq!(left[0] + left[2], right[0], "no gap or overlap");
    assert_eq!(right[0] + right[2], rt.state.area[2]);
    assert_eq!(left[3], rt.state.area[3]);
    assert_eq!(rect("divider-line"), [right[0], right[1], 1., right[3]]);
    let divider = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref().is_some_and(|k| k.starts_with("divider:")))
        .unwrap()
        .rect;
    let (x, y) = (divider[0] + divider[2] / 2., divider[1] + divider[3] / 2.);
    rt.pointer_down(x, y);
    rt.pointer_move(x + 100., y);
    rt.frame(1. / 60.);
    let expected = 0.5 + 100. / rt.state.area[2] as f64;
    assert!((rt.state.drag.as_ref().unwrap().ratio - expected).abs() < 0.001);
    rt.pointer_up(x + 100., y);
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
    rt.update(|s, _| s.pane_menu("block:c".into(), None));
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
    let outline = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("target-outline"))
        .unwrap()
        .rect;
    assert_eq!(outline, right);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn split_pane_headers_show_their_own_status_word() {
    use rex_linux::connection::activity::{BlockedKind, ProgramState::*};
    let (mut ws, _rx) = tabbed_workspace("split");
    let mut blocked = record(Blocked, true);
    blocked.kind = Some(BlockedKind::Question);
    ws.hosts[0]
        .model
        .activity
        .insert("block:c".into(), [("".into(), blocked)].into());
    let mut rt = runtime(ws);
    assert!(has(&rt, "pane-word:block:c"));
    assert!(
        !has(&rt, "pane-word:block:b"),
        "only the pane that reported"
    );
    // A direct field write doesn't invalidate the view; go through update.
    rt.update(|s, _| s.settings.notifications.show_in_tabs = false);
    // Let the word's exit transition finish.
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
    assert!(
        !has(&rt, "pane-word:block:c"),
        "the activity switch hides it"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_tabs_menu_and_middle_click_act_on_that_tab() {
    let (ws, rx) = tabbed_workspace("single");
    let mut rt = runtime(ws);
    secondary_click(&mut rt, "tab:split");
    let panel = rt.state.panel.as_ref().expect("a tab menu");
    assert!(matches!(&panel.kind, PanelKind::Window(id) if id == "split"));
    assert_eq!(panel.title, "dev");
    let labels: Vec<String> = rt.state.entries().into_iter().map(|e| e.label).collect();
    assert_eq!(labels, ["Rename tab…", "Close tab…", "New tab"]);
    assert!(has(&rt, "menu-surface"));
    press(&mut rt, Key::Escape);
    assert!(rt.state.panel.is_none());

    let r = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("tab:split"))
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
            if method == "session.close_window" && payload["window_id"] == "split")
    );
    assert!(
        requests(&rx).iter().all(|r| r.0 != "session.focus_window"),
        "a middle-click doesn't switch tabs"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn ctrl_shift_w_closes_a_lone_panes_tab_or_a_splits_focused_pane() {
    let (ws, _rx) = tabbed_workspace("single");
    let mut rt = runtime(ws);
    ctrl_shift(&mut rt, 'w');
    let panel = rt.state.panel.take().expect("a confirmation");
    assert!(
        matches!(&panel.kind, PanelKind::Confirm { method, payload, .. }
            if method == "session.close_window" && payload["window_id"] == "single")
    );
    std::fs::remove_file(&rt.state.settings_path).ok();

    let (ws, _rx) = tabbed_workspace("split");
    let mut rt = runtime(ws);
    ctrl_shift(&mut rt, 'w');
    let panel = rt.state.panel.take().expect("a confirmation");
    assert!(
        matches!(&panel.kind, PanelKind::Confirm { method, payload, .. }
            if method == "block.close" && payload["block_id"] == "block:b")
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

fn session(id: &str) -> rexrs::api::session::Session {
    rexrs::api::session::Session {
        session_id: id.into(),
        label: format!("{id}-label"),
    }
}

/// Swap host `index`'s worker for a channel the test reads.
fn capture_host(ws: &mut Workspace, index: usize) -> flume::Receiver<HostCommand> {
    let (commands, receiver) = flume::unbounded();
    let (_, wake) = flume::unbounded();
    ws.hosts[index].connection = HostConnection {
        model: Default::default(),
        commands,
        wake,
    };
    receiver
}

#[test]
fn split_uses_the_focused_pane_as_its_directory_source() {
    let (mut ws, rx) = tabbed_workspace("split");
    ws.perform(Intent::Split("vertical"), &mut rune::Tx::default());
    let sent = requests(&rx);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, "session.new_split");
    assert_eq!(sent[0].1, "s");
    assert_eq!(sent[0].2["cwd_from"], sent[0].2["anchor_id"]);
    assert_eq!(sent[0].2["cwd_from"], ws.model().focused().unwrap());
}

#[test]
fn new_tabs_without_a_source_use_defaults_and_without_a_session_do_nothing() {
    let mut ws = workspace();
    let rx = capture_host(&mut ws, 0);
    ws.term_theme = Some(rex_linux::terminal_theme::TerminalTheme::midnight());
    ws.perform(Intent::NewWindow, &mut rune::Tx::default());
    assert!(rx.is_empty(), "no selected session, no creation");
    assert!(
        !ws.settings.terminal_theme_declared,
        "no phantom theme declaration"
    );

    ws.hosts[0].model.selected = Some("cur".into());
    ws.perform(Intent::NewWindow, &mut rune::Tx::default());
    let sent: Vec<_> = rx.try_iter().collect();
    assert!(
        matches!(&sent[..], [HostCommand::Request { method, payload }]
        if method == "session.new_window" && payload["session_id"] == "cur"),
        "no focused block means no process query"
    );
    std::fs::remove_file(&ws.settings_path).ok();
}

/// Every request as (method, session_id, payload), with a synthetic cwd_from for creation.
fn requests(rx: &flume::Receiver<HostCommand>) -> Vec<(String, String, serde_json::Value)> {
    rx.try_iter()
        .filter_map(|c| match c {
            HostCommand::Request { method, payload } => {
                Some((method, payload["session_id"].as_str()?.to_owned(), payload))
            }
            HostCommand::NewTerminal {
                method,
                mut payload,
                cwd_from,
            } => {
                let session = payload["session_id"].as_str()?.to_owned();
                payload["cwd_from"] = cwd_from.into();
                Some((method, session, payload))
            }
            HostCommand::Navigate {
                session,
                window: Some(window),
                ..
            } => Some((
                "session.focus_window".into(),
                session.clone(),
                serde_json::json!({"session_id":session,"window_id":window}),
            )),
            _ => None,
        })
        .collect()
}

fn secondary_click(rt: &mut Runtime<Workspace>, key: &str) {
    let r = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("missing {key}"))
        .rect;
    let (x, y) = (r[0] + r[2] / 2., r[1] + r[3] / 2.);
    rt.secondary_down(x, y);
    rt.secondary_up(x, y);
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
}

#[test]
fn switcher_rows_rename_in_place_and_the_session_chip_opens_the_session_menu() {
    let mut ws = workspace_hosts(2);
    let _current = capture_host(&mut ws, 0);
    let other = capture_host(&mut ws, 1);
    ws.hosts[0].model.connected = true;
    ws.hosts[0].model.sessions = vec![session("cur")];
    ws.hosts[0].model.selected = Some("cur".into());
    ws.hosts[1].model.connected = true;
    ws.hosts[1].model.sessions = vec![session("b")];
    let mut rt = runtime(ws);

    click(&mut rt, "identity");
    let b_row = |rt: &Runtime<Workspace>| {
        rt.elements()
            .into_iter()
            .find(|e| {
                e.label.as_deref() == Some("b-label")
                    && e.key.as_deref().is_some_and(|k| k.starts_with("entry:"))
            })
            .and_then(|e| e.key)
            .expect("b's switcher row")
    };
    let row = b_row(&rt);
    let index: usize = row["entry:".len()..].parse().unwrap();

    // Right-click renames the row in place; Esc goes back to browsing, not out.
    secondary_click(&mut rt, &row);
    let panel = rt.state.panel.as_ref().expect("switcher");
    assert!(matches!(panel.kind, PanelKind::Picker(true)));
    assert_eq!(panel.rename.as_ref().unwrap().1.text, "b-label");
    assert!(
        rt.elements()
            .iter()
            .any(|e| e.key.as_deref() == Some("session-rename"))
    );
    press(&mut rt, Key::Escape);
    let panel = rt.state.panel.as_ref().expect("switcher stays open");
    assert!(panel.rename.is_none());

    // F2 on the highlighted row, then typing over the selected name and Enter saves it.
    rt.state.panel.as_mut().unwrap().selected = index;
    press(&mut rt, Key::Function(2));
    assert!(rt.state.panel.as_ref().unwrap().rename.is_some());
    for c in "renamed".chars() {
        rt.key(KeyEvent {
            key: Key::Text(c.to_string()),
            mods: Mods::default(),
        });
    }
    press(&mut rt, Key::Enter);
    assert!(rt.state.panel.as_ref().unwrap().rename.is_none());
    assert_eq!(
        requests(&other)
            .iter()
            .map(|(m, id, p)| (m.as_str(), id.as_str(), &p["label"]))
            .collect::<Vec<_>>(),
        vec![("session.set_label", "b", &serde_json::json!("renamed"))]
    );

    // The highlighted row's × asks first, and cancelling comes back to the switcher.
    rt.state.panel.as_mut().unwrap().selected = index;
    rt.frame(1. / 60.);
    click(&mut rt, &format!("close:{index}"));
    let panel = rt.state.panel.as_ref().expect("close confirmation");
    assert!(matches!(panel.kind, PanelKind::Confirm { .. }));
    press(&mut rt, Key::Escape);
    let panel = rt.state.panel.as_ref().expect("back to the switcher");
    assert!(matches!(panel.kind, PanelKind::Picker(true)));
    press(&mut rt, Key::Escape);

    secondary_click(&mut rt, "identity");
    let panel = rt.state.panel.as_ref().expect("session menu");
    assert_eq!(panel.title, "test-0 · cur-label");
    let rename = rt
        .state
        .entries()
        .iter()
        .position(|e| e.label == "Rename session…")
        .unwrap();
    click(&mut rt, &format!("entry:{rename}"));
    let panel = rt.state.panel.as_ref().expect("rename form");
    assert_eq!(panel.inputs[0].text, "cur-label");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn session_menu_acts_on_its_row_not_the_current_session() {
    let mut ws = workspace_hosts(2);
    ws.sidebar.toggle();
    let current = capture_host(&mut ws, 0);
    let other = capture_host(&mut ws, 1);
    ws.hosts[0].model.connected = true;
    ws.hosts[0].model.sessions = vec![session("cur")];
    ws.hosts[0].model.selected = Some("cur".into());
    ws.hosts[1].model.connected = true;
    ws.hosts[1].model.sessions = vec![session("a"), session("b"), session("c")];
    ws.hosts[1].model.selected = Some("a".into());
    let mut rt = runtime(ws);

    secondary_click(&mut rt, "session:1:b");
    let panel = rt.state.panel.as_ref().expect("session menu");
    assert!(matches!(panel.kind, PanelKind::Session));
    assert_eq!(panel.title, "test-1 · b-label");
    assert!(
        rt.elements()
            .iter()
            .any(|e| e.key.as_deref() == Some("session-target:1:b")),
        "the targeted row is outlined"
    );
    let labels: Vec<_> = rt
        .state
        .entries()
        .into_iter()
        .map(|e| (e.label, e.enabled))
        .collect();
    assert_eq!(
        labels,
        [
            ("Switch to session".to_owned(), true),
            ("Rename session…".to_owned(), true),
            ("Move up".to_owned(), true),
            ("Move down".to_owned(), true),
            ("Close session…".to_owned(), true),
        ]
    );

    // Close → confirm → submit destroys b on host 1, and nothing on the current host.
    let target = rt.state.panel.as_ref().unwrap().session.clone().unwrap();
    let mut tx = rune::Tx::default();
    rt.state.perform(
        Intent::Session(target.clone(), SessionAction::Close),
        &mut tx,
    );
    let panel = rt.state.panel.as_ref().expect("confirm");
    assert_eq!(panel.title, "Close b-label?");
    rt.state.submit(&mut tx);
    let sent = requests(&other);
    assert_eq!(sent.len(), 1);
    assert_eq!(
        (sent[0].0.as_str(), sent[0].1.as_str()),
        ("session.destroy", "b")
    );
    assert!(
        requests(&current).is_empty(),
        "the current session is untouched"
    );

    // Rename names b on host 1 too.
    rt.state.perform(
        Intent::Session(target.clone(), SessionAction::Rename),
        &mut tx,
    );
    rt.state.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("renamed");
    rt.state.submit(&mut tx);
    let sent = requests(&other);
    assert_eq!(
        (sent[0].0.as_str(), sent[0].1.as_str(), &sent[0].2["label"]),
        ("session.set_label", "b", &serde_json::json!("renamed"))
    );

    // Up goes before a; down from the next-to-last row goes to the end.
    rt.state.perform(
        Intent::Session(target.clone(), SessionAction::MoveUp),
        &mut tx,
    );
    rt.state
        .perform(Intent::Session(target, SessionAction::MoveDown), &mut tx);
    let sent = requests(&other);
    assert_eq!(sent[0].2["before_session_id"], "a");
    assert!(sent[1].2.get("before_session_id").is_none());
    assert!(requests(&current).is_empty());
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn session_menu_on_an_edge_row_and_an_offline_host() {
    let mut ws = workspace_hosts(2);
    ws.sidebar.toggle();
    ws.hosts[0].model.connected = true;
    ws.hosts[0].model.sessions = vec![session("cur"), session("next")];
    ws.hosts[0].model.selected = Some("cur".into());
    ws.hosts[1].model.sessions = vec![session("away")];
    let mut rt = runtime(ws);

    secondary_click(&mut rt, "session:0:cur");
    let entries: Vec<_> = rt
        .state
        .entries()
        .into_iter()
        .map(|e| (e.label, e.enabled))
        .collect();
    assert_eq!(
        entries[0],
        ("Rename session…".to_owned(), true),
        "no Switch on the current row"
    );
    assert_eq!(entries[1], ("Move up".to_owned(), false));
    assert_eq!(entries[2], ("Move down".to_owned(), true));
    press(&mut rt, Key::Escape);
    assert!(rt.state.panel.is_none());

    secondary_click(&mut rt, "session:1:away");
    let panel = rt.state.panel.as_ref().expect("offline session menu");
    assert_eq!(panel.title, "test-1 · away-label · Offline");
    let entries: Vec<_> = rt
        .state
        .entries()
        .into_iter()
        .map(|e| (e.label, e.enabled))
        .collect();
    assert_eq!(
        entries,
        [
            ("Switch to session".to_owned(), true),
            ("Rename session…".to_owned(), false),
            ("Move up".to_owned(), false),
            ("Move down".to_owned(), false),
            ("Close session…".to_owned(), false),
            ("Retry connection".to_owned(), true),
            ("Edit host…".to_owned(), true),
        ]
    );
    assert_eq!(panel.selected, 0);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

/// Shift+right-click goes to a mouse-tracking program instead of opening the pane menu, so
/// the grid's secondary handler must see Shift.
#[test]
fn a_secondary_press_handler_sees_held_shift() {
    struct S {
        shift: Option<bool>,
    }
    fn v(_: &S, _: &mut rune::Cx<S>) -> rune::El {
        rune::el().w_full().h_full().child(
            rune::el()
                .key("target")
                .size(100., 100.)
                .on_secondary_press(|s: &mut S, _, tx: &mut rune::Tx| {
                    s.shift = Some(tx.mods().shift)
                }),
        )
    }
    let mut rt = Runtime::new(S { shift: None }, v, (200., 200.), Env::new(true, 1.));
    rt.frame(1. / 60.);
    rt.physical_input(rune::PhysicalInput::Modifiers(Mods {
        shift: true,
        ..Mods::default()
    }));
    rt.secondary_down(50., 50.);
    rt.secondary_up(50., 50.);
    rt.frame(1. / 60.);
    assert_eq!(rt.state.shift, Some(true));
}

#[test]
fn ui_zoom_reflows_the_root_and_preserves_a_focused_forms_text() {
    let mut ws = workspace();
    ws.open_panel(
        PanelKind::Form(Form::RenameSession(None)),
        "Rename session",
        &[("Session name", "", "keep-me")],
    );
    let mut rt = runtime_at(ws, (1200., 800.));
    let focus = focused(&rt);
    let mods = Mods {
        ctrl: true,
        cmd: true,
        ..Mods::default()
    };
    for _ in 0..8 {
        rt.key(KeyEvent {
            key: Key::Command('+'),
            mods,
        });
        rt.frame(1. / 60.);
    }
    assert_eq!(rt.state.settings.ui_scale, 1.5);
    assert_eq!(focused(&rt), focus);
    assert_eq!(rt.state.panel.as_ref().unwrap().inputs[0].text, "keep-me");
    assert!((rt.state.area[2] - 1200. / 1.5).abs() < 1.);
    for _ in 0..10 {
        rt.key(KeyEvent {
            key: Key::Command('-'),
            mods,
        });
        rt.frame(1. / 60.);
    }
    assert_eq!(rt.state.settings.ui_scale, 0.7);
    rt.key(KeyEvent {
        key: Key::Command('0'),
        mods,
    });
    rt.frame(1. / 60.);
    assert_eq!(rt.state.settings.ui_scale, 1.);
    assert_eq!(focused(&rt), focus);
    assert_eq!(rt.state.panel.as_ref().unwrap().inputs[0].text, "keep-me");
    let saved = Settings::load(&rt.state.settings_path).unwrap();
    assert_eq!(saved.ui_scale, 1.);
    std::fs::remove_file(&rt.state.settings_path).unwrap();
}

fn runtime(ws: Workspace) -> Runtime<Workspace> {
    runtime_at(ws, (1024., 700.))
}

#[test]
fn scaled_pointer_hits_and_sidebar_drag_use_window_deltas_once() {
    let mut ws = workspace_hosts(2);
    ws.sidebar.toggle();
    ws.settings.ui_scale = 1.5;
    ws.hosts[1].model.connected = true;
    let mut rt = runtime_at(ws, (1200., 800.));
    click(&mut rt, "new-session-on:1");
    assert_eq!(rt.state.active, 1, "the scaled press hit the row");
    let edge = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("sidebar-edge"))
        .unwrap()
        .rect;
    let x = edge[0] + edge[2] / 2.;
    let y = edge[1] + 120.;
    let before = rt.state.sidebar.width();
    rt.pointer_down(x, y);
    rt.pointer_move(x + 30., y);
    rt.pointer_up(x + 30., y);
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
    assert!(
        (rt.state.sidebar.width() - before - 20.).abs() < 1.,
        "30 window points = 20 layout points at 150%"
    );
    std::fs::remove_file(&rt.state.settings_path).unwrap();
}

#[test]
fn zoom_in_a_query_keeps_text_and_small_windows_keep_their_layout_floor() {
    let mut ws = workspace();
    ws.picker(true);
    ws.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("keep-query");
    let mut rt = runtime_at(ws, (640., 480.));
    let focus = focused(&rt);
    rt.key(KeyEvent {
        key: Key::Command('='),
        mods: Mods {
            ctrl: true,
            cmd: true,
            ..Mods::default()
        },
    });
    rt.frame(1. / 60.);
    assert_eq!(rt.state.settings.ui_scale, 1.1);
    assert_eq!(rt.state.ui_scale(), 1.);
    assert_eq!(
        rt.state.panel.as_ref().unwrap().inputs[0].text,
        "keep-query"
    );
    assert_eq!(focused(&rt), focus);
    assert!((rt.state.area[2] - 640.).abs() < 1.);
    std::fs::remove_file(&rt.state.settings_path).unwrap();
}

fn runtime_at(ws: Workspace, size: (f32, f32)) -> Runtime<Workspace> {
    let mut rt = Runtime::new(ws, view, size, Env::new(true, 1.));
    rt.on_key = Some(super::input::on_key);
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
    rt
}

fn focused(rt: &Runtime<Workspace>) -> Option<String> {
    rt.elements()
        .into_iter()
        .find(|e| e.focused)
        .map(|e| e.path)
}

fn press(rt: &mut Runtime<Workspace>, key: Key) {
    rt.key(KeyEvent {
        key,
        mods: Mods::default(),
    });
    for _ in 0..3 {
        rt.frame(1. / 60.);
    }
}

#[test]
fn close_confirmation_opens_on_keep_running_and_return_keeps_it_running() {
    let mut ws = workspace();
    ws.panel = Some(super::panel::Panel::new(
        PanelKind::Confirm {
            method: "session.close_window".into(),
            payload: serde_json::json!({}),
            description: "Ends programs.".into(),
        },
        "Close this tab?",
        &[],
    ));
    let mut rt = runtime(ws);
    let path = focused(&rt).expect("a focused control");
    assert!(path.ends_with("cancel-close"), "focus starts on {path}");
    press(&mut rt, Key::Enter);
    assert!(
        rt.state.panel.is_none(),
        "Return pressed Keep running and closed the dialog"
    );
    assert!(rt.state.notices.is_empty(), "no request failed");
}

#[test]
fn about_is_keyboard_reachable_and_opens_links_and_copies_details() {
    let mut ws = workspace();
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    let mut rt = runtime(ws);
    // The existing Settings section list exposes About without another shortcut.
    for _ in 0..7 {
        press(&mut rt, Key::Down);
    }
    press(&mut rt, Key::Enter);
    assert_eq!(
        rt.state.settings_section,
        super::panel::SettingsSection::About
    );
    assert!(has(&rt, "about-repository"));
    assert!(
        rt.elements()
            .iter()
            .any(|e| e.text.as_deref() == Some("Rex Linux"))
    );
    assert!(
        rt.elements()
            .iter()
            .any(|e| e.text.as_deref() == Some(env!("CARGO_PKG_VERSION")))
    );
    assert!(!has(&rt, "about-server"));
    press(&mut rt, Key::Tab); // Close
    press(&mut rt, Key::Tab); // Repository
    assert!(focused(&rt).unwrap().ends_with("about-repository"));
    press(&mut rt, Key::Enter);
    press(&mut rt, Key::Tab);
    press(&mut rt, Key::Text(" ".into()));
    press(&mut rt, Key::Tab);
    press(&mut rt, Key::Enter);
    assert_eq!(
        rt.state.opened_links,
        vec![
            super::about::REPOSITORY.to_owned(),
            super::about::release_url(),
            format!(
                "{}/blob/v{}/LICENSE",
                super::about::REPOSITORY,
                env!("CARGO_PKG_VERSION")
            ),
        ]
    );
    let clipboard = rune::clipboard::memory();
    rt.use_clipboard(&clipboard);
    press(&mut rt, Key::Tab);
    assert!(focused(&rt).unwrap().ends_with("about-copy"));
    press(&mut rt, Key::Enter);
    assert_eq!(
        clipboard.text().as_deref().map(String::as_str),
        Some(super::about::version_info(&rt.state).as_str())
    );
}

#[test]
fn about_only_reports_the_connected_active_hosts_api() {
    let mut ws = workspace_hosts(2);
    ws.hosts[0].model.connected = true;
    ws.hosts[0].model.protocol.api_revision = 12;
    ws.hosts[1].model.connected = true;
    ws.hosts[1].model.protocol.api_revision = 11;
    assert!(super::about::version_info(&ws).contains("Server API revision: 12"));
    ws.active = 1;
    assert!(super::about::version_info(&ws).contains("Server API revision: 11"));
    ws.hosts[1].model.connected = false;
    assert!(!super::about::version_info(&ws).contains("Server API"));
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    ws.open_section(super::panel::SettingsSection::About);
    ws.hosts[1].model.connected = true;
    let rt = runtime(ws);
    assert!(has(&rt, "about-server"));
}

#[test]
fn the_toolbar_settings_button_opens_settings() {
    let mut rt = runtime(workspace());
    // The root eases from the default viewport to the window's before the right end is on screen.
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    click(&mut rt, "settings");
    assert!(matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::Settings)
    ));
}

#[test]
fn forms_focus_their_first_field_and_settings_start_on_its_section_list() {
    let mut ws = workspace();
    ws.open_panel(
        PanelKind::Form(Form::RenameSession(None)),
        "Rename session",
        &[("Session name", "", "work")],
    );
    let rt = runtime(ws);
    let path = focused(&rt).expect("a focused field");
    assert!(
        path.contains("field:0"),
        "rename focuses its field, not {path}"
    );

    let mut rt = runtime(workspace());
    rt.key(KeyEvent {
        key: Key::Text(",".into()),
        mods: Mods {
            ctrl: true,
            ..Mods::default()
        },
    });
    for _ in 0..3 {
        rt.frame(1. / 60.);
    }
    assert!(matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::Settings)
    ));
    let path = focused(&rt).expect("a focused control");
    assert!(
        path.ends_with("settings-section:interface"),
        "settings focus {path}"
    );
    // The keyboard path scripts/x11-smoke.py takes to the font stepper.
    press(&mut rt, Key::Down);
    press(&mut rt, Key::Enter);
    assert_eq!(
        rt.state.settings_section,
        super::panel::SettingsSection::Terminal
    );
    let font = rt.state.settings.font_size;
    for _ in 0..4 {
        press(&mut rt, Key::Tab);
    }
    let path = focused(&rt).expect("a focused control");
    assert!(path.ends_with("font-larger"), "Tab reached {path}");
    press(&mut rt, Key::Enter);
    assert_eq!(rt.state.settings.font_size, font + 1.);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn invalid_shortcuts_start_with_one_notice_and_a_persistent_settings_problem() {
    let mut ws = workspace();
    let directory = ws.settings_path.with_extension("shortcuts-test");
    std::fs::create_dir_all(&directory).unwrap();
    ws.settings_path = directory.join("workspace.json");
    ws.settings.save(&ws.settings_path).unwrap();
    let shortcuts = rex_linux::settings::shortcuts::Shortcuts::path(&ws.settings_path);
    std::fs::write(&shortcuts, "[broken").unwrap();
    let mut ws = relaunch(&ws);
    assert_eq!(
        ws.notices
            .iter()
            .filter(|n| n.text.contains("using default shortcuts"))
            .count(),
        1
    );
    assert!(ws.settings.shortcut_file.notice.is_none());
    ws.notices.clear();
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    let mut rt = runtime(ws);
    click(&mut rt, "settings-section:shortcuts");
    assert!(has(&rt, "shortcut-problem-0"));
    let elements = rt.elements();
    let problem = elements
        .iter()
        .find(|e| e.key.as_deref() == Some("shortcut-problem-0"))
        .unwrap();
    assert!(problem.text.as_deref().unwrap().contains("Invalid JSON"));
    let path = elements
        .iter()
        .find(|e| e.key.as_deref() == Some("shortcuts-path"))
        .unwrap();
    assert_eq!(path.text.as_deref(), Some(shortcuts.to_str().unwrap()));
    rt.state.settings.save(&rt.state.settings_path).unwrap();
    assert_eq!(std::fs::read_to_string(shortcuts).unwrap(), "[broken");
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn a_notification_switch_flips_and_settings_stay_open() {
    let mut ws = workspace();
    let mut tx = rune::Tx::default();
    ws.perform(Intent::Settings, &mut tx);
    let mut rt = runtime(ws);
    click(&mut rt, "settings-section:notifications");
    assert!(rt.state.settings.notifications.show_in_tabs);
    click(&mut rt, "notify-tabs");
    assert!(
        matches!(
            rt.state.panel.as_ref().map(|p| &p.kind),
            Some(PanelKind::Settings)
        ),
        "Settings stays open"
    );
    assert!(
        !rt.state.settings.notifications.show_in_tabs,
        "the switch flipped"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_disabled_notification_switch_leaves_settings_open() {
    let mut ws = workspace();
    ws.settings.notifications.system_notifications = false;
    let blocked = ws.settings.notifications.blocked;
    let mut tx = rune::Tx::default();
    ws.perform(Intent::Settings, &mut tx);
    let mut rt = runtime(ws);
    click(&mut rt, "settings-section:notifications");
    click(&mut rt, "notify-blocked");
    assert!(
        matches!(
            rt.state.panel.as_ref().map(|p| &p.kind),
            Some(PanelKind::Settings)
        ),
        "a press on a disabled switch must not reach the dismiss layer"
    );
    assert_eq!(rt.state.settings.notifications.blocked, blocked, "disabled");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_press_on_a_dialogs_padding_leaves_it_open() {
    let mut ws = workspace();
    let mut tx = rune::Tx::default();
    ws.perform(Intent::Settings, &mut tx);
    let mut rt = runtime(ws);
    let r = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("dialog"))
        .expect("the Settings dialog")
        .rect;
    rt.pointer_down(r[0] + 6., r[1] + 6.);
    rt.pointer_up(r[0] + 6., r[1] + 6.);
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
    assert!(
        matches!(
            rt.state.panel.as_ref().map(|p| &p.kind),
            Some(PanelKind::Settings)
        ),
        "only a press outside the dialog closes it"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn dialog_padding_preserves_field_focus_but_the_scrim_dismisses() {
    let mut ws = workspace();
    ws.open_panel(
        PanelKind::Form(Form::AddHost),
        "Add host",
        &[
            ("Name", "", "work"),
            ("Endpoint", "", "unix:///tmp/rex.sock"),
        ],
    );
    let mut rt = runtime(ws);
    press(&mut rt, Key::Tab);
    let before = focused(&rt).expect("the second field is focused");
    assert!(before.contains("/field:1/"), "Tab reached {before}");
    let r = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("dialog"))
        .unwrap()
        .rect;
    rt.pointer_down(r[0] + 6., r[1] + 6.);
    rt.pointer_up(r[0] + 6., r[1] + 6.);
    rt.frame(1. / 60.);
    assert!(rt.state.panel.is_some(), "padding is not the scrim");
    assert_eq!(focused(&rt), Some(before), "typing stays in the field");

    // Stay off the window's resize edges, which own their presses even with a panel open.
    rt.pointer_down(40., 200.);
    rt.pointer_up(40., 200.);
    rt.frame(1. / 60.);
    assert!(rt.state.panel.is_none(), "outside presses still dismiss");
    assert_eq!(focused(&rt), None, "focus returns to the terminal");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn settings_sections_and_their_forms_return_to_settings() {
    let mut ws = workspace_hosts(2);
    let mut tx = rune::Tx::default();
    ws.perform(Intent::Settings, &mut tx);
    let mut rt = runtime(ws);
    click(&mut rt, "settings-section:hosts");
    assert!(has(&rt, "host-row:0") && has(&rt, "host-row:1"));
    click(&mut rt, "host-edit:1");
    let panel = rt.state.panel.as_ref().expect("the edit form");
    assert!(matches!(panel.kind, PanelKind::Form(Form::EditHost(1))));
    assert_eq!(panel.inputs[0].text, "test-1");
    press(&mut rt, Key::Escape);
    assert!(
        matches!(
            rt.state.panel.as_ref().map(|p| &p.kind),
            Some(PanelKind::Settings)
        ),
        "Escape goes back to Settings"
    );
    click(&mut rt, "host-remove:1");
    assert!(matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::RemoveHost(1))
    ));
    click(&mut rt, "confirm-close");
    assert_eq!(rt.state.settings.hosts.len(), 1);
    assert!(
        matches!(
            rt.state.panel.as_ref().map(|p| &p.kind),
            Some(PanelKind::Settings)
        ),
        "back in Settings after removing"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_theme_picker_filters_declares_to_every_host_and_returns_to_settings() {
    let root = std::env::temp_dir().join(format!("rex-linux-picker-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("Rex Test Mocha"),
        "background = #1e1e2e\nforeground = #cdd6f4\npalette = 1=#f38ba8\n",
    )
    .unwrap();
    std::fs::write(root.join("Paper"), "background = #f2eede\n").unwrap();
    let mut ws = workspace_hosts(2);
    ws.theme_dirs = ThemeDirs {
        user: Some(root.clone()),
        system: vec![],
    };
    let hosts = [capture_host(&mut ws, 0), capture_host(&mut ws, 1)];
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    let mut rt = runtime(ws);
    click(&mut rt, "settings-section:terminal");
    click(&mut rt, "terminal-theme");
    assert!(matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::Themes)
    ));
    let rows = |ws: &Workspace| {
        ws.entries()
            .into_iter()
            .map(|e| (e.label, e.group, e.current))
            .collect::<Vec<_>>()
    };
    let row = |label: &str, group: &str, current| (label.to_owned(), group.to_owned(), current);
    assert_eq!(
        rows(&rt.state),
        [
            row("Follow server", "Server", true),
            row("Rex default colours", "Server", false),
            row("Follow chrome", "Built in", false),
            row("Midnight", "Built in", false),
            row("Rosé Pine", "Built in", false),
            row("Rosé Pine Moon", "Built in", false),
            row("Gruvbox Dark", "Built in", false),
            row("Rex Test Mocha", "Dark", false),
            row("Paper", "Light", false),
        ]
    );
    press(&mut rt, Key::Escape);
    assert!(
        matches!(
            rt.state.panel.as_ref().map(|p| &p.kind),
            Some(PanelKind::Settings)
        ),
        "Escape goes back to Settings"
    );
    assert!(hosts.iter().all(|h| h.is_empty()), "nothing was declared");

    click(&mut rt, "terminal-theme");
    rt.update(|s, _| {
        s.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("mocha");
        s.query_changed();
    });
    assert_eq!(rows(&rt.state), [row("Rex Test Mocha", "Dark", false)]);
    press(&mut rt, Key::Enter);
    assert!(
        matches!(
            rt.state.panel.as_ref().map(|p| &p.kind),
            Some(PanelKind::Settings)
        ),
        "applying goes back to Settings"
    );
    let theme = rt.state.term_theme.clone().expect("a declared theme");
    assert_eq!(theme.background, Some(0x1e1e2e));
    let declared = |host: &flume::Receiver<HostCommand>| {
        host.try_iter()
            .filter_map(|c| match c {
                HostCommand::Theme(args) => Some(args),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    for host in &hosts {
        assert_eq!(
            declared(host),
            [Some(theme.args())],
            "every host is told once"
        );
    }
    let saved = Settings::load(&rt.state.settings_path).unwrap();
    assert_eq!(
        saved.terminal_theme,
        TerminalThemeChoice::Named("Rex Test Mocha".into())
    );
    assert_eq!(
        relaunch(&rt.state).term_theme,
        None,
        "a theme missing at launch declares nothing but stays chosen"
    );

    rt.update(|s, tx| s.perform(Intent::Theme(TerminalThemeChoice::FollowServer), tx));
    for host in &hosts {
        assert_eq!(declared(host), [None], "Follow server stops declaring");
    }
    assert_eq!(rt.state.term_theme, None);
    std::fs::remove_file(&rt.state.settings_path).ok();
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn follow_server_resets_only_after_a_theme_was_ever_declared() {
    let mut ws = workspace_hosts(2);
    let hosts = [capture_host(&mut ws, 0), capture_host(&mut ws, 1)];
    let mut tx = rune::Tx::default();
    let declared = |host: &flume::Receiver<HostCommand>| {
        host.try_iter()
            .filter_map(|c| match c {
                HostCommand::Theme(args) => Some(args),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(ws.declared_theme(), None, "a fresh client declares nothing");
    ws.apply_theme(TerminalThemeChoice::FollowServer).unwrap();
    for host in &hosts {
        assert_eq!(declared(host), [None]);
    }
    ws.sync(0, &mut tx);
    assert!(!ws.settings.terminal_theme_declared);
    assert!(hosts.iter().all(|h| h.is_empty()));

    ws.hosts[0].connection.model.lock().unwrap().theme_declared = true;
    ws.sync(0, &mut tx);
    assert!(
        ws.settings.terminal_theme_declared,
        "set by the first declaration"
    );
    assert!(
        Settings::load(&ws.settings_path)
            .unwrap()
            .terminal_theme_declared
    );
    let reset = || Some(rexrs::api::terminal::SetTheme::new());
    for host in &hosts {
        assert_eq!(declared(host), [reset()], "Follow server now resets");
    }
    ws.sync(0, &mut tx);
    assert!(hosts.iter().all(|h| h.is_empty()), "told once");
    assert_eq!(
        relaunch(&ws).declared_theme(),
        reset(),
        "and on every later launch"
    );
    ws.apply_theme(TerminalThemeChoice::FollowServer).unwrap();
    assert_eq!(declared(&hosts[1]), [reset()]);
    std::fs::remove_file(&ws.settings_path).ok();

    let mut ws = workspace();
    let host = capture_host(&mut ws, 0);
    ws.hosts[0].model.selected = Some("cur".into());
    ws.term_theme = Some(rex_linux::terminal_theme::TerminalTheme::midnight());
    ws.perform(Intent::NewWindow, &mut tx);
    assert_eq!(requests(&host)[0].0, "session.new_window");
    assert!(
        ws.settings.terminal_theme_declared,
        "a theme in a new tab's layout is a declaration"
    );
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn removing_a_host_keeps_the_others_and_ignores_its_late_wakes() {
    let mut ws = workspace_hosts(3);
    let ids: Vec<_> = ws.hosts.iter().map(|h| h.id).collect();
    ws.active = 2;
    for (id, block) in ids.iter().zip(["a", "b", "c"]) {
        ws.claimed.insert((*id, block.to_owned()));
    }
    let mut tx = rune::Tx::default();
    ws.remove_host(1, &mut tx);
    let labels: Vec<_> = ws.settings.hosts.iter().map(|h| h.label.as_str()).collect();
    assert_eq!(labels, ["test-0", "test-2"]);
    assert_eq!(
        ws.hosts.iter().map(|h| h.id).collect::<Vec<_>>(),
        [ids[0], ids[2]]
    );
    assert_eq!(ws.active, 1, "the active host keeps being active");
    assert!(ws.claimed.contains(&(ids[0], "a".into())));
    assert!(ws.claimed.contains(&(ids[2], "c".into())));
    assert!(!ws.claimed.iter().any(|(id, _)| *id == ids[1]));
    // A wake the removed host's worker sent before it stopped lands nowhere.
    ws.sync_host(ids[1], &mut tx);
    assert_eq!(ws.hosts.len(), 2);

    ws.remove_host(0, &mut tx);
    ws.remove_host(0, &mut tx);
    assert_eq!(ws.settings.hosts.len(), 1, "the last host stays");
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn a_new_endpoint_reconnects_under_a_new_id_and_a_rename_does_not() {
    let mut ws = workspace_hosts(2);
    let id = ws.hosts[1].id;
    ws.claimed.insert((id, "b".into()));
    ws.edit_host(
        1,
        "renamed".into(),
        ws.settings.hosts[1].endpoint.clone(),
        Some("zsh".into()),
    );
    assert_eq!(ws.hosts[1].id, id);
    assert_eq!(ws.settings.hosts[1].label, "renamed");
    assert_eq!(ws.settings.hosts[1].shell.as_deref(), Some("zsh"));

    ws.edit_host(
        1,
        "renamed".into(),
        "unix:///nonexistent/elsewhere.sock".into(),
        None,
    );
    assert_ne!(ws.hosts[1].id, id);
    assert_eq!(
        ws.settings.hosts[1].endpoint,
        "unix:///nonexistent/elsewhere.sock"
    );
    assert!(ws.claimed.is_empty(), "the old server's claims are dropped");
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn terminal_keys_are_not_taken_by_focus_traversal_or_activation() {
    let mut rt = runtime(workspace());
    for key in [
        Key::Tab,
        Key::Text(" ".into()),
        Key::Enter,
        Key::Escape,
        Key::ContextMenu,
    ] {
        press(&mut rt, key.clone());
        assert_eq!(focused(&rt), None, "{key:?} moved focus off the terminal");
        assert!(rt.state.panel.is_none(), "{key:?} opened a panel");
    }
}

#[test]
fn escape_walks_back_from_the_host_step_to_the_switcher() {
    let mut ws = workspace();
    ws.picker(true);
    ws.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("proj");
    let mut tx = rune::Tx::default();
    ws.perform(Intent::ChooseHost("proj".into()), &mut tx);
    let mut rt = runtime(ws);
    press(&mut rt, Key::Escape);
    let panel = rt.state.panel.as_ref().expect("the switcher");
    assert!(
        matches!(panel.kind, PanelKind::Picker(true)),
        "Escape on the host step returns to the switcher"
    );
    assert_eq!(panel.inputs[0].text, "proj", "with its query intact");
    press(&mut rt, Key::Escape);
    assert!(rt.state.panel.is_none());
}

fn record(
    state: rex_linux::connection::activity::ProgramState,
    live: bool,
) -> rex_linux::connection::activity::StatusRecord {
    rex_linux::connection::activity::StatusRecord {
        state,
        app: Some("claude".into()),
        live,
        ..Default::default()
    }
}

fn set_activity(
    ws: &mut Workspace,
    block: &str,
    records: &[(&str, rex_linux::connection::activity::StatusRecord)],
) {
    let model = &mut ws.hosts[0].model;
    model.activity.remove(block);
    for (id, r) in records {
        model
            .activity
            .entry(block.into())
            .or_default()
            .insert((*id).into(), r.clone());
    }
    ws.track_activity(0);
}

#[test]
fn a_tab_folds_its_records_by_priority_and_shows_its_program() {
    use super::activity::{program_icon, tab_activity, tab_word};
    use rex_linux::connection::activity::{BlockedKind, ProgramState::*};
    let (mut ws, _rx) = tabbed_workspace("single");
    let model = &mut ws.hosts[0].model;
    model.programs.insert("block:a".into(), "nvim".into());
    model.programs.insert("block:b".into(), "bash".into());
    let mut blocked = record(Blocked, true);
    blocked.kind = Some(BlockedKind::Permission);
    blocked.msg = Some("Run cargo test?".into());
    model.activity.insert(
        "block:c".into(),
        [("".into(), record(Working, true)), ("x".into(), blocked)].into(),
    );
    model
        .activity
        .insert("block:b".into(), [("".into(), record(Error, true))].into());
    let view = model.view().unwrap().clone();
    let single = tab_activity(model, &view.windows[0]);
    assert_eq!(single.state, Idle);
    assert_eq!(single.program.as_deref(), Some("nvim"));
    assert!(std::ptr::eq(program_icon(Some("nvim")), &super::theme::VIM));
    assert_eq!(tab_word(&single, false, false), None);
    assert_eq!(
        tab_word(&single, false, true).map(|word| word.0),
        Some("Bell".into())
    );
    let split = tab_activity(model, &view.windows[1]);
    // Blocked beats Error beats Working, across both panes of the split.
    assert_eq!(split.state, Blocked);
    assert_eq!(split.msg.as_deref(), Some("Run cargo test?"));
    // The focused pane (block:b) reports through claude, which beats its foreground shell.
    assert_eq!(split.program.as_deref(), Some("claude"));
    assert_eq!(
        tab_word(&split, false, false).map(|w| w.0).as_deref(),
        Some("Needs approval")
    );
    assert!(std::ptr::eq(
        program_icon(Some("bash")),
        &super::theme::TERMINAL
    ));
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn background_bells_latch_until_their_tab_is_seen() {
    let (mut ws, _rx) = tabbed_workspace("single");
    ws.window_focused = false;
    ws.ring_bells(0, vec![("s".into(), "block:b".into())]);
    assert!(ws.bell_on(0, "split"));
    ws.window_focused = true;
    ws.track_activity(0);
    assert!(ws.bell_on(0, "split"), "a background tab keeps its bell");
    ws.hosts[0]
        .model
        .views
        .get_mut("s")
        .unwrap()
        .active_window_id = Some("split".into());
    ws.track_activity(0);
    assert!(!ws.bell_on(0, "split"), "seeing the tab clears its bell");
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn a_tabs_progress_report_is_a_bar_while_working_and_a_word_when_paused_or_failed() {
    use super::activity::{tab_activity, tab_progress, tab_word};
    use rex_linux::connection::activity::{ProgramState::Blocked, TerminalProgress::*};
    let (mut ws, _rx) = tabbed_workspace("single");
    let model = &mut ws.hosts[0].model;
    let view = model.view().unwrap().clone();
    let word = |model: &rex_linux::connection::HostModel, tab: usize, done: bool| {
        tab_word(&tab_activity(model, &view.windows[tab]), done, false).map(|(w, _)| w.into_owned())
    };
    let bar = |model: &rex_linux::connection::HostModel, tab: usize| {
        tab_progress(&tab_activity(model, &view.windows[tab])).map(|(n, _)| n)
    };
    // A working program's report is a bar and no word; Done still shows while it works.
    for (progress, fraction) in [
        (Set(Some(42)), Some(42)),
        (Set(None), None),
        (Indeterminate, None),
    ] {
        model.progress.insert("block:a".into(), progress);
        assert_eq!(word(model, 0, false), None, "{progress:?}");
        assert_eq!(word(model, 0, true).as_deref(), Some("Done"));
        assert_eq!(bar(model, 0), Some(fraction), "{progress:?}");
    }
    // Paused and failed reports are trouble, so they carry a word, and keep the bar.
    for (progress, expected, fraction) in [
        (Paused(Some(7)), "Paused 7%", Some(7)),
        (Paused(None), "Paused", None),
        (Error(Some(3)), "Error", Some(3)),
    ] {
        model.progress.insert("block:a".into(), progress);
        assert_eq!(word(model, 0, true).as_deref(), Some(expected));
        assert_eq!(bar(model, 0), Some(fraction));
    }
    // Across a split, an error beats a pause beats progress.
    model.progress.insert("block:b".into(), Set(Some(10)));
    model.progress.insert("block:c".into(), Paused(Some(20)));
    assert_eq!(word(model, 1, false).as_deref(), Some("Paused 20%"));
    assert_eq!(bar(model, 1), Some(Some(20)));
    model.progress.insert("block:b".into(), Error(None));
    assert_eq!(word(model, 1, false).as_deref(), Some("Error"));
    // OSC 7501 trouble still comes first, and takes the bar with it.
    model.activity.insert(
        "block:a".into(),
        [("".into(), record(Blocked, true))].into(),
    );
    assert_eq!(word(model, 0, false).as_deref(), Some("Needs input"));
    assert_eq!(bar(model, 0), None);
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn a_working_progress_report_draws_a_bar_on_its_tab_and_pane() {
    use rex_linux::connection::activity::TerminalProgress::Set;
    let (mut ws, _rx) = tabbed_workspace("split");
    ws.hosts[0]
        .model
        .progress
        .insert("block:c".into(), Set(Some(60)));
    let mut rt = runtime(ws);
    assert!(has(&rt, "tab-progress:split"));
    assert!(has(&rt, "pane-progress:block:c"));
    assert!(
        !has(&rt, "pane-progress:block:b"),
        "only the pane that reported"
    );
    assert!(!has(&rt, "tab-word:split"), "working is a bar, not a word");
    rt.update(|s, _| s.settings.notifications.show_in_tabs = false);
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
    assert!(
        !has(&rt, "tab-progress:split"),
        "the activity switch hides it"
    );
    assert!(!has(&rt, "pane-progress:block:c"));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_titled_pane_shows_its_title_but_renames_from_its_label() {
    let (mut ws, _rx) = tabbed_workspace("split");
    let model = &mut ws.hosts[0].model;
    let block = &mut model.views.get_mut("s").unwrap().windows[1].layers[0].blocks[0];
    block.label = Some("shell".into());
    block.creator_name = Some("com.superlogical.terminal".into());
    block.flavor = Some("com.superlogical.terminal.shell".into());
    model
        .titles
        .insert("block:b".into(), ("Claude Code".into(), None));
    let target = ws.target("block:b").unwrap();
    assert_eq!(target.label, "shell", "Rename prefills the server label");
    assert_eq!(target.name, "Claude Code");
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn each_pane_reports_its_own_activity_and_osc_7501_progress() {
    use super::activity::{pane_activity, program_icon, tab_activity, tab_progress, tab_word};
    use rex_linux::connection::activity::{BlockedKind, ProgramState::*, TerminalProgress::Set};
    let (mut ws, _rx) = tabbed_workspace("split");
    let model = &mut ws.hosts[0].model;
    let mut blocked = record(Blocked, true);
    blocked.kind = Some(BlockedKind::Permission);
    blocked.progress = Some(50);
    let mut working = record(Working, true);
    working.progress = Some(40);
    model
        .activity
        .insert("block:b".into(), [("".into(), blocked)].into());
    model
        .activity
        .insert("block:c".into(), [("".into(), working)].into());
    let word = |a| tab_word(&a, false, false).map(|(w, _)| w.into_owned());
    let bar = |a| tab_progress(&a).map(|(n, _)| n);
    assert_eq!(
        word(pane_activity(model, "block:b")).as_deref(),
        Some("Needs approval")
    );
    // A working record's percentage is a bar on its pane, not a word.
    assert_eq!(word(pane_activity(model, "block:c")), None);
    assert_eq!(bar(pane_activity(model, "block:c")), Some(Some(40)));
    let view = model.view().unwrap().clone();
    assert_eq!(
        word(tab_activity(model, &view.windows[1])).as_deref(),
        Some("Needs approval")
    );
    assert_eq!(
        bar(tab_activity(model, &view.windows[1])),
        None,
        "trouble has the tab's attention"
    );
    // An OSC 9;4 report beats the record's own progress.
    model.progress.insert("block:c".into(), Set(Some(10)));
    assert_eq!(bar(pane_activity(model, "block:c")), Some(Some(10)));
    // A pane with nothing to report shows nothing.
    assert_eq!(word(pane_activity(model, "block:a")), None);
    assert_eq!(bar(pane_activity(model, "block:a")), None);
    // Equal Working records: the one with progress decides, whatever its id order.
    model.progress.clear();
    for ids in [("a", "z"), ("z", "a")] {
        let mut with = record(Working, true);
        with.progress = Some(25);
        model.activity.insert(
            "block:c".into(),
            [(ids.0.into(), with), (ids.1.into(), record(Working, true))].into(),
        );
        assert_eq!(bar(pane_activity(model, "block:c")), Some(Some(25)));
    }
    // Across panes too: block:b Working without progress, block:c Working with 25%.
    model.activity.insert(
        "block:b".into(),
        [("".into(), record(Working, true))].into(),
    );
    let view = model.view().unwrap().clone();
    assert_eq!(bar(tab_activity(model, &view.windows[1])), Some(Some(25)));
    assert_eq!(word(tab_activity(model, &view.windows[1])), None);
    assert!(std::ptr::eq(
        program_icon(Some("claude-code")),
        &super::theme::CLAUDE
    ));
    assert!(std::ptr::eq(program_icon(Some("omp")), &super::theme::PI));
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn background_tabs_latch_done_and_notify_only_about_live_changes() {
    use rex_linux::connection::activity::ProgramState::*;
    let (mut ws, _rx) = tabbed_workspace("single");
    ws.track_activity(0);
    // A query restoring a blocked state isn't news.
    set_activity(&mut ws, "block:b", &[("", record(Blocked, false))]);
    assert!(ws.outbox.is_empty());
    // A live report that it's working again isn't worth a notification by default.
    set_activity(&mut ws, "block:b", &[("", record(Working, true))]);
    assert!(ws.outbox.is_empty());
    // Finishing in a tab the user can't see latches Done and notifies.
    set_activity(&mut ws, "block:b", &[]);
    assert!(ws.unseen("split"));
    assert_eq!(ws.outbox.len(), 1);
    assert_eq!(ws.outbox[0].summary, "dev · Done");
    assert_eq!(ws.outbox[0].target.window.as_deref(), Some("split"));
    // A live error is news too.
    set_activity(&mut ws, "block:c", &[("", record(Error, true))]);
    assert_eq!(ws.outbox.len(), 2);
    assert_eq!(ws.outbox[1].summary, "claude · Error");
    // The same changes in the tab in front are quiet, and seeing a tab clears its Done.
    set_activity(&mut ws, "block:a", &[("", record(Working, true))]);
    set_activity(&mut ws, "block:a", &[("", record(Done, true))]);
    assert_eq!(ws.outbox.len(), 2);
    assert!(!ws.unseen("single"));
    ws.hosts[0]
        .model
        .views
        .get_mut("s")
        .unwrap()
        .active_window_id = Some("split".into());
    ws.track_activity(0);
    assert!(!ws.unseen("split"));
    // A reconnect drops activity wholesale; that gap is not programs finishing.
    set_activity(&mut ws, "block:a", &[("", record(Working, true))]);
    ws.hosts[0].model.activity.clear();
    ws.hosts[0].model.activity_epoch += 1;
    ws.window_focused = false;
    ws.track_activity(0);
    assert_eq!(ws.outbox.len(), 2);
    assert!(!ws.unseen("single"));
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn notification_settings_gate_what_is_sent() {
    use rex_linux::connection::activity::{DesktopNotice, ProgramState::*};
    let (mut ws, _rx) = tabbed_workspace("single");
    ws.track_activity(0);
    ws.settings.notifications.done = false;
    set_activity(&mut ws, "block:b", &[("", record(Working, true))]);
    set_activity(&mut ws, "block:b", &[]);
    assert!(ws.outbox.is_empty(), "Done notifications are off");
    assert!(ws.unseen("split"), "the tab still shows Done");
    let notice = |block: &str| DesktopNotice {
        session: "s".into(),
        block: block.into(),
        title: String::new(),
        body: "Build finished".into(),
    };
    ws.deliver_program_notices(0, vec![notice("block:a"), notice("block:c")]);
    assert_eq!(ws.outbox.len(), 1, "the tab in front stays quiet");
    assert_eq!(ws.outbox[0].summary, "dev");
    assert_eq!(ws.outbox[0].body, "Build finished\ns-label · test-0");
    ws.settings.notifications.system_notifications = false;
    set_activity(&mut ws, "block:c", &[("", record(Error, true))]);
    ws.deliver_program_notices(0, vec![notice("block:c")]);
    assert_eq!(ws.outbox.len(), 1);
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn tab_pills_carry_the_activity_word_and_hide_it_when_turned_off() {
    use rex_linux::connection::activity::{BlockedKind, ProgramState::*};
    let pill = |show: bool| {
        let (mut ws, _rx) = tabbed_workspace("single");
        ws.settings.notifications.show_in_tabs = show;
        let mut blocked = record(Blocked, true);
        blocked.kind = Some(BlockedKind::Question);
        ws.hosts[0]
            .model
            .activity
            .insert("block:c".into(), [("".into(), blocked)].into());
        let rt = runtime(ws);
        let elements = rt.elements();
        let word = elements
            .iter()
            .find(|e| e.key.as_deref() == Some("tab-word:split"))
            .and_then(|e| e.text.clone());
        let label = elements
            .iter()
            .find(|e| e.key.as_deref() == Some("tab:split"))
            .and_then(|e| e.label.clone());
        std::fs::remove_file(&rt.state.settings_path).ok();
        (word, label)
    };
    assert_eq!(
        pill(true),
        (Some("Question".into()), Some("dev · Question".into()))
    );
    assert_eq!(pill(false), (None, Some("dev".into())));
}

#[test]
fn a_default_tab_takes_its_programs_title_and_a_named_one_keeps_its_name() {
    let (mut ws, _rx) = tabbed_workspace("single");
    let model = &mut ws.hosts[0].model;
    let view = model.views.get_mut("s").unwrap();
    view.windows[1].label = Some(rex_linux::connection::DEFAULT_TAB_LABEL.into());
    for (block, title) in [
        ("block:a", "vim notes"),
        ("block:b", "OC | USB device exploration"),
    ] {
        model.titles.insert(block.into(), (title.into(), None));
    }
    let rt = runtime(ws);
    let label = |key: &str| {
        rt.elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some(key))
            .and_then(|e| e.label)
    };
    assert_eq!(label("tab:single").as_deref(), Some("server"));
    assert_eq!(
        label("tab:split").as_deref(),
        Some("OC | USB device exploration")
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

const SKILL_STATUS: &str = include_str!("fixtures/skill_status.json");
const SKILL_SETUP_CODEX: &str = include_str!("fixtures/skill_setup_codex.json");

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| a.to_string()).collect()
}

#[test]
fn agent_commands_never_autostart_and_always_name_the_naming_choice() {
    use super::integrations::{AgentChange, status_args};
    assert_eq!(
        status_args(),
        strings(&["--autostart=false", "skill", "status", "--json"])
    );
    let setup = AgentChange::Setup {
        agents: vec!["codex".into()],
        force: false,
    };
    assert_eq!(
        setup.args(false, true),
        strings(&[
            "--autostart=false",
            "skill",
            "setup",
            "--agent=codex",
            "--session-naming=false",
            "--dry-run",
            "--json"
        ])
    );
    let all = AgentChange::Setup {
        agents: vec![],
        force: true,
    };
    assert_eq!(
        all.args(true, false),
        strings(&[
            "--autostart=false",
            "skill",
            "setup",
            "--session-naming=true",
            "--force",
            "--json"
        ])
    );
    let remove = AgentChange::Remove {
        agents: vec!["pi".into()],
    };
    assert_eq!(
        remove.args(true, false),
        strings(&[
            "--autostart=false",
            "skill",
            "remove",
            "--agent=pi",
            "--json"
        ])
    );
    let omp = AgentChange::Setup {
        agents: vec!["omp".into(), "pi".into()],
        force: false,
    };
    assert_eq!(
        omp.args(false, true),
        strings(&[
            "--autostart=false",
            "skill",
            "setup",
            "--agent=pi",
            "--session-naming=false",
            "--dry-run",
            "--json"
        ])
    );
    let omp_alone = AgentChange::Remove {
        agents: vec!["omp".into()],
    };
    assert_eq!(
        omp_alone.args(true, false),
        strings(&[
            "--autostart=false",
            "skill",
            "remove",
            "--agent=pi",
            "--json"
        ])
    );
}

/// A home directory with OMP's agent directory in it.
fn omp_home(name: &str) -> std::path::PathBuf {
    let home = std::env::temp_dir().join(format!("rex-linux-omp-{}-{name}", std::process::id()));
    std::fs::create_dir_all(home.join(".omp/agent")).unwrap();
    home
}

fn omp_at(home: &std::path::Path) -> super::integrations::OmpEnv {
    super::integrations::OmpEnv {
        home: Some(home.to_owned()),
        ..Default::default()
    }
}

#[test]
fn omp_is_detected_from_its_agent_directory_or_path() {
    use super::integrations::OmpEnv;
    let home = omp_home("detect");
    let bin = home.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let empty = home.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let path = |dirs: &[&std::path::Path]| Some(std::env::join_paths(dirs).unwrap());

    assert!(!OmpEnv::default().detected());
    assert!(omp_at(&home).detected());
    assert!(
        !OmpEnv {
            home: Some(empty.clone()),
            path: path(&[&empty, &bin]),
            pi_coding_agent_dir: Some(home.join("missing")),
        }
        .detected()
    );
    assert!(
        OmpEnv {
            pi_coding_agent_dir: Some(home.join(".omp")),
            ..Default::default()
        }
        .detected()
    );
    std::fs::write(bin.join("omp"), "").unwrap();
    assert!(
        OmpEnv {
            home: Some(empty.clone()),
            path: path(&[&empty, &bin]),
            ..Default::default()
        }
        .detected()
    );
    std::fs::remove_dir_all(&home).ok();
}

fn integrations_open(
    status: Result<String, String>,
    omp: super::integrations::OmpEnv,
) -> Runtime<Workspace> {
    let mut ws = workspace();
    ws.integrations.omp = omp;
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    ws.open_section(super::panel::SettingsSection::Integrations);
    assert_eq!(
        ws.integrations.ran,
        vec![super::integrations::status_args()],
        "opening the section reads the status once"
    );
    ws.open_section(super::panel::SettingsSection::Integrations);
    assert_eq!(ws.integrations.ran.len(), 1);
    ws.apply_agent_status(status);
    runtime_at(ws, (1280., 900.))
}

fn element_text(rt: &Runtime<Workspace>, key: &str) -> Vec<String> {
    let elements = rt.elements();
    let Some(index) = elements.iter().position(|e| e.key.as_deref() == Some(key)) else {
        return Vec::new();
    };
    let mut inside = vec![index];
    let mut texts = Vec::new();
    for (i, e) in elements.iter().enumerate().skip(index) {
        if i != index && !e.parent.is_some_and(|p| inside.contains(&p)) {
            continue;
        }
        inside.push(i);
        if let Some(t) = &e.text {
            texts.push(t.clone());
        }
    }
    texts
}

#[test]
fn integrations_list_agents_and_preview_changes_before_making_them() {
    use super::integrations::AgentChange;
    let home = omp_home("list");
    let mut rt = integrations_open(Ok(SKILL_STATUS.into()), omp_at(&home));
    let codex = element_text(&rt, "agent:codex").join(" | ");
    assert!(codex.contains("Not installed"), "{codex}");
    assert!(codex.contains("with OpenCode and Pi and OMP"), "{codex}");
    let omp = element_text(&rt, "agent:omp").join(" | ");
    assert!(omp.contains("OMP"), "{omp}");
    assert!(omp.contains("Not installed"), "{omp}");
    assert!(omp.contains("with Codex and OpenCode and Pi"), "{omp}");
    assert!(omp.contains("Shares Pi's skill directory"), "{omp}");
    assert!(omp.contains("Install…"), "{omp}");
    assert!(
        element_text(&rt, "agent:claude")
            .join(" ")
            .contains("Install…")
    );
    click(&mut rt, "agent-setup:codex");
    let preview = rt.state.integrations.ran.last().unwrap().clone();
    assert!(preview.contains(&"--dry-run".to_owned()));
    assert!(preview.contains(&"--session-naming=false".to_owned()));
    let change = AgentChange::Setup {
        agents: vec!["codex".into()],
        force: false,
    };
    rt.state
        .show_agent_preview(change.clone(), Ok(SKILL_SETUP_CODEX.into()));
    let ws = rt.state;
    let panel = ws.panel.as_ref().unwrap();
    assert_eq!(panel.title, "Set up Rex for Codex?");
    assert!(matches!(panel.kind, PanelKind::AgentChange { .. }));
    let mut rt = runtime_at(ws, (1280., 900.));
    let lines = rt
        .elements()
        .into_iter()
        .filter_map(|e| e.text)
        .collect::<Vec<_>>()
        .join("\n");
    // The shared skill names every agent it affects, not only the one asked for.
    assert!(
        lines.contains("Create skill /home/user/.agents/skills/rex · Codex, OpenCode, Pi, OMP"),
        "{lines}"
    );
    assert!(
        lines.contains("shared skill changes for every agent"),
        "{lines}"
    );
    click(&mut rt, "agents-apply");
    let applied = rt.state.integrations.ran.last().unwrap().clone();
    assert!(!applied.contains(&"--dry-run".to_owned()));
    assert_eq!(applied, change.args(false, false));
    assert!(matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::Settings)
    ));
    std::fs::remove_file(&rt.state.settings_path).ok();
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn omp_mirrors_pis_skill_and_is_only_offered_when_found() {
    use super::integrations::{AgentStatus, Item};
    let pi_skill = |state: &str| {
        let mut status: serde_json::Value = serde_json::from_str(SKILL_STATUS).unwrap();
        for harness in status["harnesses"].as_array_mut().unwrap() {
            if harness["id"] == "pi" {
                harness["items"][0]["state"] = state.into();
            }
        }
        status.to_string()
    };
    let home = omp_home("mirror");
    for (state, words) in [
        ("current", "Installed"),
        ("missing", "Not installed"),
        ("unsupported", "Not installed"),
    ] {
        let rt = integrations_open(Ok(pi_skill(state)), omp_at(&home));
        let AgentStatus::Ready(harnesses) = &rt.state.integrations.status else {
            panic!("status not read");
        };
        let omp = harnesses.iter().find(|h| h.id == "omp").unwrap();
        assert!(omp.detected);
        assert_eq!(
            omp.items,
            vec![Item {
                kind: "skill".into(),
                path: Some("/home/user/.agents/skills/rex".into()),
                state: state.into(),
            }]
        );
        let row = element_text(&rt, "agent:omp").join(" | ");
        assert!(row.contains(words), "{state}: {row}");
        assert!(
            element_text(&rt, "agent:pi").join(" | ").contains(words),
            "{state}"
        );
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
    std::fs::remove_dir_all(&home).ok();

    let rt = integrations_open(Ok(SKILL_STATUS.into()), Default::default());
    let row = element_text(&rt, "agent:omp").join(" | ");
    assert!(row.contains("Not found on this machine"), "{row}");
    assert!(!row.contains("Install…"), "{row}");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn removing_omp_previews_that_pi_loses_the_skill_too() {
    use super::integrations::AgentChange;
    let home = omp_home("remove");
    let installed = SKILL_STATUS.replace("\"missing\"", "\"current\"");
    let mut rt = integrations_open(Ok(installed), omp_at(&home));
    click(&mut rt, "agent-remove:omp");
    let change = AgentChange::Remove {
        agents: vec!["omp".into()],
    };
    let preview = rt.state.integrations.ran.last().unwrap().clone();
    assert_eq!(preview, change.args(false, true));
    rt.state.show_agent_preview(
        change.clone(),
        Ok(
            r#"{"steps": [{"harnesses": ["codex", "opencode", "pi"], "kind": "skill",
            "path": "/home/user/.agents/skills/rex", "change": "removed"}]}"#
                .into(),
        ),
    );
    let ws = rt.state;
    assert_eq!(ws.panel.as_ref().unwrap().title, "Remove Rex from OMP?");
    let mut rt = runtime_at(ws, (1280., 900.));
    let lines = rt
        .elements()
        .into_iter()
        .filter_map(|e| e.text)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        lines.contains("Remove skill /home/user/.agents/skills/rex · Codex, OpenCode, Pi, OMP"),
        "{lines}"
    );
    assert!(
        lines.contains("OMP and Pi share one skill, so both lose it."),
        "{lines}"
    );
    click(&mut rt, "agents-apply");
    assert_eq!(
        rt.state.integrations.ran.last().unwrap(),
        &change.args(false, false)
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
    std::fs::remove_dir_all(&home).ok();
}

/// `SKILL_STATUS` with each listed agent's detection and Rex files set; unlisted agents are
/// fully set up.
fn skill_status(agents: &[(&str, bool, &str)]) -> String {
    let mut status: serde_json::Value = serde_json::from_str(SKILL_STATUS).unwrap();
    for harness in status["harnesses"].as_array_mut().unwrap() {
        let (detected, state) = agents
            .iter()
            .find(|(id, ..)| harness["id"] == *id)
            .map_or((true, "current"), |&(_, detected, state)| (detected, state));
        harness["detected"] = detected.into();
        for item in harness["items"].as_array_mut().unwrap() {
            if item["state"] != "unsupported" {
                item["state"] = state.into();
            }
        }
    }
    status.to_string()
}

#[test]
fn set_up_all_names_agents_only_while_omp_is_pending() {
    use super::integrations::{AgentStatus, setup_all};
    let home = omp_home("all");
    let not_pi = [("codex", false, "missing"), ("opencode", false, "missing")];
    type Case<'a> = (&'a str, Vec<(&'a str, bool, &'a str)>, bool, &'a [&'a str]);
    let cases: [Case; 5] = [
        (
            "OMP alone",
            [&not_pi[..], &[("pi", false, "missing")]].concat(),
            true,
            &["--agent=pi"],
        ),
        (
            "OMP and Codex without Pi",
            vec![
                ("codex", true, "missing"),
                ("opencode", false, "missing"),
                ("pi", false, "missing"),
            ],
            true,
            &["--agent=codex", "--agent=pi"],
        ),
        (
            "OMP and Pi",
            [&not_pi[..], &[("pi", true, "missing")]].concat(),
            true,
            &["--agent=pi"],
        ),
        (
            "no OMP",
            vec![
                ("codex", true, "missing"),
                ("opencode", true, "missing"),
                ("pi", true, "missing"),
            ],
            false,
            &[],
        ),
        (
            "OMP set up",
            [&not_pi[..], &[("claude", true, "missing")]].concat(),
            true,
            &[],
        ),
    ];
    for (case, agents, omp_found, expected) in cases {
        let omp = if omp_found {
            omp_at(&home)
        } else {
            Default::default()
        };
        let mut rt = integrations_open(Ok(skill_status(&agents)), omp);
        // Settings scrolls, and longer agent rows push the button past its bottom edge.
        rt.wheel(800., 500., -400.);
        for _ in 0..30 {
            rt.frame(1. / 60.);
        }
        click(&mut rt, "agents-setup-all");
        assert_eq!(rt.state.integrations.ran.len(), 2, "{case}");
        let preview = rt.state.integrations.ran.last().unwrap().clone();
        let named: Vec<_> = preview
            .iter()
            .filter(|a| a.starts_with("--agent="))
            .collect();
        assert_eq!(named, expected.to_vec(), "{case}");
        let AgentStatus::Ready(harnesses) = &rt.state.integrations.status else {
            panic!("{case}: status not read");
        };
        let change = setup_all(harnesses).unwrap();
        rt.state.show_agent_preview(
            change,
            Ok(r#"{"steps": [{"harnesses": ["pi"], "kind": "skill",
                "path": "/home/user/.agents/skills/rex", "change": "created"}]}"#
                .into()),
        );
        let mut rt = runtime_at(rt.state, (1280., 900.));
        click(&mut rt, "agents-apply");
        let applied = rt.state.integrations.ran.last().unwrap().clone();
        let previewed: Vec<_> = preview.into_iter().filter(|a| a != "--dry-run").collect();
        assert_eq!(applied, previewed, "{case}");
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn a_missing_rex_explains_itself_and_naming_reapplies_to_installed_agents() {
    let rt = integrations_open(
        Err("The rex command isn't installed on this machine.".into()),
        Default::default(),
    );
    let texts = rt
        .elements()
        .into_iter()
        .filter_map(|e| e.text)
        .collect::<Vec<_>>();
    assert!(texts.iter().any(|t| t.contains("isn't installed")));
    std::fs::remove_file(&rt.state.settings_path).ok();

    let installed = SKILL_STATUS.replacen("\"missing\"", "\"current\"", 2);
    let mut ws = workspace();
    ws.apply_agent_status(Ok(installed));
    ws.toggle_session_naming();
    assert!(ws.settings.agent_session_naming);
    let preview = ws.integrations.ran.last().unwrap();
    assert!(
        preview.contains(&"--agent=claude".to_owned()),
        "{preview:?}"
    );
    assert!(!preview.contains(&"--agent=codex".to_owned()));
    assert!(preview.contains(&"--session-naming=true".to_owned()));
    assert!(preview.contains(&"--dry-run".to_owned()));
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn terminal_copies_reach_the_clipboard_once_and_only_in_the_foreground() {
    use rex_linux::connection::clipboard::ClipboardCopy;
    for (block, session, focused, modal, expected) in [
        ("block:a", "s", true, false, "terminal copy 世界"),
        ("block:b", "s", true, false, "sentinel"),
        ("block:a", "other", true, false, "sentinel"),
        ("block:a", "s", false, false, "sentinel"),
        ("block:a", "s", true, true, "sentinel"),
    ] {
        let (mut ws, _rx) = tabbed_workspace("single");
        ws.window_focused = focused;
        ws.hosts[0].model.visible = true;
        *ws.hosts[0].connection.model.lock().unwrap() = ws.hosts[0].model.clone();
        let mut rt = runtime(ws);
        let clipboard = rune::clipboard::memory();
        clipboard.set_text("sentinel");
        rt.use_clipboard(&clipboard);
        rt.update(|s, tx| {
            if modal {
                s.perform(Intent::Settings, tx);
            }
            s.hosts[0].connection.model.lock().unwrap().clipboard = Some(ClipboardCopy {
                session: session.into(),
                block: block.into(),
                text: "terminal copy 世界".into(),
            });
            s.sync(0, tx);
        });
        rt.frame(1. / 60.);
        assert_eq!(
            clipboard.text().as_deref().map(String::as_str),
            Some(expected)
        );
        assert!(
            rt.state.hosts[0]
                .connection
                .model
                .lock()
                .unwrap()
                .clipboard
                .is_none()
        );
        // A later wake must never replay a copy over something the user copied elsewhere.
        clipboard.set_text("later copy");
        rt.update(|s, tx| s.sync(0, tx));
        rt.frame(1. / 60.);
        assert_eq!(
            clipboard.text().as_deref().map(String::as_str),
            Some("later copy")
        );
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn alt_arrows_belong_to_terminal_programs_and_ctrl_alt_arrows_move_pane_focus() {
    let mut ws = workspace();
    let rx = capture_host(&mut ws, 0);
    ws.hosts[0].model.selected = Some("s".into());
    for key in [Key::Up, Key::Down, Key::Left, Key::Right] {
        super::input::on_key(
            &mut ws,
            &KeyEvent {
                key,
                mods: Mods {
                    alt: true,
                    ..Mods::default()
                },
            },
            &mut rune::Tx::default(),
        );
    }
    assert!(
        requests(&rx).is_empty(),
        "Alt+Arrow was intercepted before reaching the terminal"
    );
    for key in [Key::Up, Key::Down, Key::Left, Key::Right] {
        super::input::on_key(
            &mut ws,
            &KeyEvent {
                key,
                mods: Mods {
                    ctrl: true,
                    alt: true,
                    ..Mods::default()
                },
            },
            &mut rune::Tx::default(),
        );
    }
    let calls = requests(&rx);
    assert_eq!(calls.len(), 4);
    for (call, direction) in calls.iter().zip(["up", "down", "left", "right"]) {
        assert_eq!(call.0, "session.focus_direction");
        assert_eq!(call.2["direction"], direction);
    }
}

/// Make `block` a connected terminal whose input lands on `sender`.
fn live_terminal(ws: &mut Workspace, block: &str, sender: &flume::Sender<HostCommand>) {
    use rex_linux::connection::{Connection, TerminalSender};
    let connection = Connection {
        model: Default::default(),
        commands: TerminalSender::new(sender.clone(), "s".into(), block.into()),
        wake: flume::unbounded().1,
    };
    {
        let mut m = connection.model.lock().unwrap();
        m.connected = true;
        m.control_connected = true;
    }
    let key = ws.term_key(block);
    ws.terminals.insert(key, super::term::Term::new(connection));
}

/// The `client.key` payloads terminals were sent, drained from `rx`.
fn sent_keys(rx: &flume::Receiver<HostCommand>) -> Vec<serde_json::Value> {
    rx.try_iter()
        .filter_map(|c| match c {
            HostCommand::Terminal {
                command: rex_linux::connection::Command::Key(key),
                ..
            } => Some(key),
            _ => None,
        })
        .collect()
}

#[test]
fn modified_terminal_keys_reach_the_pty_and_reserved_combinations_stay_local() {
    let (mut ws, _rx) = tabbed_workspace("split");
    let (sender, sent) = flume::unbounded();
    for block in ["block:b", "block:c"] {
        live_terminal(&mut ws, block, &sender);
    }
    let mut rt = runtime(ws);
    let m = |shift, ctrl, alt| Mods {
        shift,
        ctrl,
        cmd: ctrl,
        alt,
        ..Mods::default()
    };
    let (shift, ctrl, alt) = (
        m(true, false, false),
        m(false, true, false),
        m(false, false, true),
    );
    let send = |rt: &mut Runtime<Workspace>, key: Key, mods: Mods, repeat: bool| {
        let ev = KeyEvent { key, mods };
        if repeat {
            rt.repeat_key(ev);
        } else {
            rt.key(ev);
        }
        for _ in 0..3 {
            rt.frame(1. / 60.);
        }
        sent_keys(&sent)
    };
    for (key, mods, expected) in [
        (Key::Tab, shift, ("tab", "shift")),
        (Key::Enter, shift, ("enter", "shift")),
        (Key::Enter, ctrl, ("enter", "ctrl")),
        (Key::Enter, alt, ("enter", "alt")),
        (Key::Text(" ".into()), shift, ("space", "shift")),
        (Key::Up, shift, ("arrow_up", "shift")),
        (Key::Left, ctrl, ("arrow_left", "ctrl")),
        (Key::Right, alt, ("arrow_right", "alt")),
        (Key::Home, shift, ("home", "shift")),
        (Key::End, ctrl, ("end", "ctrl")),
        (Key::Backspace, shift, ("backspace", "shift")),
        (Key::Delete, ctrl, ("delete", "ctrl")),
        (Key::PageUp, alt, ("page_up", "alt")),
    ] {
        let label = format!("{key:?} {mods:?}");
        let keys = send(&mut rt, key, mods, false);
        assert_eq!(keys.len(), 1, "{label} sent {keys:?}");
        let k = &keys[0];
        assert_eq!(
            (
                k["key"].as_str(),
                k["mods"].as_str(),
                k["consumed_mods"].as_str()
            ),
            (Some(expected.0), Some(expected.1), Some("")),
            "{label}"
        );
        assert_eq!(k["action"], "press", "{label}");
        assert_eq!(focused(&rt), None, "{label} moved focus");
        assert!(rt.state.panel.is_none(), "{label} opened a panel");
    }
    let keys = send(&mut rt, Key::Tab, shift, true);
    assert_eq!(keys.len(), 1, "Shift+Tab repeat sent {keys:?}");
    assert_eq!(
        (
            keys[0]["key"].as_str(),
            keys[0]["mods"].as_str(),
            keys[0]["action"].as_str()
        ),
        (Some("tab"), Some("shift"), Some("repeat"))
    );
    // Workspace shortcuts: zoom, scrollback, next tab, focus left.
    for (key, mods) in [
        (Key::Enter, m(true, true, false)),
        (Key::PageUp, shift),
        (Key::PageDown, ctrl),
        (Key::Left, m(false, true, true)),
    ] {
        let label = format!("{key:?} {mods:?}");
        let keys = send(&mut rt, key, mods, false);
        assert!(keys.is_empty(), "{label} reached the PTY: {keys:?}");
    }
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_clipboard_paste_reaches_only_the_pane_that_asked_while_it_stays_focused() {
    use rex_linux::connection::Command;
    for (move_focus, expected) in [(false, Some("pasted")), (true, None)] {
        let (mut ws, _rx) = tabbed_workspace("split");
        let (sender, sent) = flume::unbounded();
        for block in ["block:b", "block:c"] {
            live_terminal(&mut ws, block, &sender);
        }
        let mut rt = runtime(ws);
        let clipboard = rune::clipboard::memory();
        clipboard.set_text("pasted");
        rt.use_clipboard(&clipboard);
        let paste = KeyEvent {
            key: Key::Text("V".into()),
            mods: Mods {
                ctrl: true,
                shift: true,
                ..Mods::default()
            },
        };
        rt.update(move |s, tx| {
            super::input::on_key(s, &paste, tx);
        });
        if move_focus {
            rt.update(|s, _| {
                let view = s.hosts[0].model.views.get_mut("s").unwrap();
                view.windows[1].focused_block_id = Some("block:c".into());
            });
        }
        for _ in 0..3 {
            rt.frame(1. / 60.);
        }
        let pastes: Vec<(String, String)> = sent
            .try_iter()
            .filter_map(|c| match c {
                HostCommand::Terminal {
                    block,
                    command: Command::Paste(text),
                    ..
                } => Some((block, text)),
                _ => None,
            })
            .collect();
        assert_eq!(
            pastes,
            expected
                .map(|text| ("block:b".to_owned(), text.to_owned()))
                .into_iter()
                .collect::<Vec<_>>(),
            "focus moved: {move_focus}"
        );
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn pane_padding_controls_reach_zero_and_save() {
    let mut ws = workspace();
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    let mut rt = runtime(ws);
    click(&mut rt, "settings-section:terminal");
    for _ in 0..9 {
        click(&mut rt, "pane-padding-smaller");
    }
    assert_eq!(rt.state.settings.pane_padding, 0.);
    assert_eq!(
        Settings::load(&rt.state.settings_path)
            .unwrap()
            .pane_padding,
        0.
    );
    click(&mut rt, "pane-padding-larger");
    assert_eq!(rt.state.settings.pane_padding, 1.);
    assert_eq!(
        Settings::load(&rt.state.settings_path)
            .unwrap()
            .pane_padding,
        1.
    );
    std::fs::remove_file(&rt.state.settings_path).unwrap();
}

#[test]
fn pane_padding_changes_every_grid_without_changing_pane_placements() {
    use rex_linux::settings::PaneDensity;
    for tab in ["single", "split"] {
        for density in [PaneDensity::Compact, PaneDensity::Comfortable] {
            let (mut ws, _rx) = tabbed_workspace(tab);
            ws.settings.pane_density = density;
            ws.settings.pane_padding = 0.;
            let blocks = if tab == "single" {
                vec!["block:a"]
            } else {
                vec!["block:b", "block:c"]
            };
            let (sender, _terminal_rx) = flume::unbounded();
            for block in &blocks {
                live_terminal(&mut ws, block, &sender);
            }
            let mut rt = runtime(ws);
            for _ in 0..90 {
                rt.frame(1. / 60.);
            }
            let rect = |rt: &Runtime<Workspace>, key: &str| {
                rt.elements()
                    .into_iter()
                    .find(|e| e.key.as_deref() == Some(key))
                    .unwrap()
                    .rect
            };
            let before: Vec<_> = blocks
                .iter()
                .map(|block| {
                    (
                        rect(&rt, &format!("pane:{block}")),
                        rect(&rt, &format!("grid:{block}")),
                    )
                })
                .collect();
            rt.update(|s, _| s.settings.pane_padding = 8.);
            for _ in 0..4 {
                rt.frame(1. / 60.);
            }
            for (block, (pane, grid)) in blocks.iter().zip(before) {
                assert_eq!(rect(&rt, &format!("pane:{block}")), pane);
                let padded = rect(&rt, &format!("grid:{block}"));
                assert_eq!(
                    padded,
                    [grid[0] + 8., grid[1] + 8., grid[2] - 16., grid[3] - 16.]
                );
                assert_eq!(rt.state.terminals[&rt.state.term_key(block)].rect, padded);
            }
            std::fs::remove_file(&rt.state.settings_path).ok();
        }
    }
}

#[test]
fn font_preferences_save_from_settings_without_server_requests() {
    let mut ws = workspace();
    let requests = capture_host(&mut ws, 0);
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    let mut rt = runtime(ws);
    click(&mut rt, "settings-section:terminal");
    click(&mut rt, "font-change");
    rt.state.panel.as_mut().unwrap().inputs[0].text = "TX02 Nerd Font Mono".into();
    rt.state.panel.as_mut().unwrap().inputs[1].text = "Noto Sans CJK JP, Noto Color Emoji".into();
    click(&mut rt, "save-form");
    assert!(matches!(
        rt.state.panel.as_ref().unwrap().kind,
        PanelKind::Settings
    ));
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&rt.state.settings_path).unwrap()).unwrap();
    assert_eq!(saved["font_family"], "TX02 Nerd Font Mono");
    assert_eq!(
        saved["font_fallbacks"],
        serde_json::json!(["Noto Sans CJK JP", "Noto Color Emoji"])
    );
    assert!(
        requests.try_iter().next().is_none(),
        "font preferences are entirely local"
    );
    std::fs::remove_file(&rt.state.settings_path).unwrap();
}

/// Rendered text containing `needle`.
fn shown(rt: &Runtime<Workspace>, needle: &str) -> bool {
    rt.elements()
        .iter()
        .any(|e| e.text.as_deref().is_some_and(|t| t.contains(needle)))
}

#[test]
fn font_family_row_says_a_saved_font_waits_for_the_next_launch() {
    let mut ws = workspace();
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    let mut rt = runtime(ws);
    click(&mut rt, "settings-section:terminal");
    assert!(shown(
        &rt,
        "JetBrainsMono Nerd Font isn't installed, so terminals use IBM Plex Mono (bundled)."
    ));

    click(&mut rt, "font-change");
    rt.state.panel.as_mut().unwrap().inputs[0].text = "MonoLisaCode Variable".into();
    click(&mut rt, "save-form");
    assert!(shown(
        &rt,
        "MonoLisaCode Variable is saved and applies when you reopen the client."
    ));
    let notices: Vec<_> = rt.state.notices.iter().map(|n| n.text.as_str()).collect();
    assert_eq!(notices, ["Fonts saved. Reopen the client to use them."]);

    // Saving without a change doesn't repeat the notice.
    click(&mut rt, "font-change");
    click(&mut rt, "save-form");
    assert_eq!(rt.state.notices.len(), 1);
    std::fs::remove_file(&rt.state.settings_path).unwrap();
}

#[test]
fn font_preferences_validate_cancel_and_clear_optional_fallbacks() {
    let mut ws = workspace();
    ws.settings.font_fallbacks = vec!["Inter".into()];
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    let mut rt = runtime(ws);
    click(&mut rt, "settings-section:terminal");
    click(&mut rt, "font-change");
    rt.state.panel.as_mut().unwrap().inputs[0].text.clear();
    click(&mut rt, "save-form");
    assert!(rt.state.panel.as_ref().unwrap().error.is_some());
    assert!(!rt.state.settings_path.exists());
    press(&mut rt, Key::Escape);
    assert_eq!(rt.state.settings.font_fallbacks, ["Inter"]);
    click(&mut rt, "font-change");
    rt.state.panel.as_mut().unwrap().inputs[1].text.clear();
    click(&mut rt, "save-form");
    assert!(rt.state.settings.font_fallbacks.is_empty());
    let restored = Settings::load(&rt.state.settings_path).unwrap();
    assert!(restored.font_fallbacks.is_empty());
    assert_eq!(restored.font_family, "JetBrainsMono Nerd Font");
    std::fs::remove_file(&rt.state.settings_path).unwrap();
}

/// A workspace with sessions "s" (selected) and "other", each with two tabs, listing tabs in
/// the sidebar.
fn sidebar_tabs_workspace() -> (Workspace, flume::Receiver<HostCommand>) {
    let (mut ws, rx) = tabbed_workspace("single");
    let mut other = ws.hosts[0].model.views["s"].clone();
    other.session_id = Some("other".into());
    other.active_window_id = Some("other-first".into());
    for (window, id) in other
        .windows
        .iter_mut()
        .zip(["other-first", "other-second"])
    {
        window.window_id = Some(id.into());
    }
    ws.hosts[0].model.sessions.push(session("other"));
    ws.hosts[0].model.views.insert("other".into(), other);
    ws.set_vertical_tabs(true);
    (ws, rx)
}

#[test]
fn sidebar_tabs_replace_the_tab_strip_only_while_the_sidebar_is_open() {
    let (ws, _rx) = sidebar_tabs_workspace();
    assert!(ws.sidebar.is_open(), "choosing Sidebar opens the sidebar");
    let saved = Settings::load(&ws.settings_path).unwrap();
    assert!(saved.vertical_tabs && saved.sidebar_open);
    let mut rt = runtime(ws);
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    assert!(!has(&rt, "tab-track"), "tabs are not shown twice");
    for tab in [
        "s:single",
        "s:split",
        "other:other-first",
        "other:other-second",
    ] {
        assert!(has(&rt, &format!("sidebar-tab:0:{tab}")), "{tab}");
    }
    ctrl_shift(&mut rt, 'b');
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    assert!(!rt.state.sidebar.is_open());
    assert!(
        has(&rt, "tab-track"),
        "closing the sidebar brings tabs back"
    );
    assert!(rt.state.settings.vertical_tabs, "the setting stays");
    ctrl_shift(&mut rt, 'b');
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    assert!(!has(&rt, "tab-track"));
    rt.update(|s, _| s.set_vertical_tabs(false));
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    assert!(has(&rt, "tab-track"));
    assert!(!has(&rt, "sidebar-tab:0:s:single"));
    assert!(
        !Settings::load(&rt.state.settings_path)
            .unwrap()
            .vertical_tabs
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_sidebar_tab_selects_its_session_and_focuses_its_window() {
    let (ws, rx) = sidebar_tabs_workspace();
    let mut rt = runtime(ws);
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    let sent = |rx: &flume::Receiver<HostCommand>| -> Vec<String> {
        rx.try_iter()
            .filter_map(|c| match c {
                HostCommand::Navigate {
                    session, window, ..
                } => Some(format!("navigate {session} {}", window.unwrap_or_default())),
                HostCommand::Request { method, payload } => Some(format!(
                    "{method} {} {}",
                    payload["session_id"].as_str()?,
                    payload["window_id"].as_str()?
                )),
                _ => None,
            })
            .collect()
    };
    sent(&rx);
    click(&mut rt, "sidebar-tab:0:other:other-second");
    assert_eq!(sent(&rx), ["navigate other other-second"]);
    click(&mut rt, "sidebar-tab:0:s:split");
    assert_eq!(sent(&rx), ["navigate s split"]);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn views_are_prefetched_only_for_tabs_the_sidebar_lists() {
    fn latest(rx: &flume::Receiver<HostCommand>) -> Option<Vec<String>> {
        rx.try_iter()
            .filter_map(|c| match c {
                HostCommand::PrefetchViews(ids) => {
                    let mut ids: Vec<_> = ids.into_iter().collect();
                    ids.sort();
                    Some(ids)
                }
                _ => None,
            })
            .last()
    }
    let all = Some(vec!["other".to_owned(), "s".to_owned()]);
    let (mut ws, rx) = tabbed_workspace("single");
    ws.hosts[0].model.sessions.push(session("other"));
    ws.toggle_sidebar();
    let mut rt = runtime(ws);
    assert_eq!(latest(&rx), None, "Top tab position loads nothing");
    rt.update(|s, _| s.set_vertical_tabs(true));
    assert_eq!(latest(&rx), all);
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    click(&mut rt, "host-disclosure:0");
    assert_eq!(latest(&rx), Some(vec![]), "a collapsed host loads nothing");
    click(&mut rt, "host-disclosure:0");
    assert_eq!(latest(&rx), all);
    ctrl_shift(&mut rt, 'b');
    assert_eq!(latest(&rx), Some(vec![]), "nor does a closed sidebar");
    ctrl_shift(&mut rt, 'b');
    assert_eq!(latest(&rx), all);
    rt.update(|s, tx| {
        let mut model = s.hosts[0].model.clone();
        model.sessions.push(session("new"));
        *s.hosts[0].connection.model.lock().unwrap() = model;
        s.sync(0, tx);
    });
    assert_eq!(
        latest(&rx),
        Some(vec!["new".into(), "other".into(), "s".into()]),
        "a new session is listed, so it is loaded too"
    );
    rt.update(|s, _| s.set_vertical_tabs(false));
    assert_eq!(latest(&rx), Some(vec![]));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

/// The themes a host was told to declare, in order.
fn declared(host: &flume::Receiver<HostCommand>) -> Vec<Option<rexrs::api::terminal::SetTheme>> {
    host.try_iter()
        .filter_map(|c| match c {
            HostCommand::Theme(args) => Some(args),
            _ => None,
        })
        .collect()
}

#[test]
fn a_chrome_palette_applies_at_once_and_brings_its_terminal_theme_until_one_is_chosen() {
    use rex_linux::{settings::ChromeTheme, terminal_theme::TerminalTheme};
    let mut ws = workspace_hosts(2);
    let hosts = [capture_host(&mut ws, 0), capture_host(&mut ws, 1)];
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    let mut rt = runtime(ws);
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    click(&mut rt, "chrome-theme");
    assert!(matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::Chromes)
    ));
    let rows: Vec<_> = rt
        .state
        .entries()
        .into_iter()
        .map(|e| (e.label, e.current))
        .collect();
    assert_eq!(
        rows,
        [
            ("Midnight".to_owned(), true),
            ("Rosé Pine".to_owned(), false),
            ("Rosé Pine Moon".to_owned(), false),
            ("Gruvbox Dark".to_owned(), false),
        ]
    );
    let rose = rt
        .elements()
        .into_iter()
        .find(|e| e.label.as_deref() == Some("Rosé Pine") && e.role == "menuitem")
        .expect("a Rosé Pine row")
        .rect;
    let (x, y) = (rose[0] + rose[2] / 2., rose[1] + rose[3] / 2.);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1. / 60.);
    assert!(
        matches!(
            rt.state.panel.as_ref().map(|p| &p.kind),
            Some(PanelKind::Settings)
        ),
        "choosing goes back to Settings"
    );
    let window = |rt: &Runtime<Workspace>| {
        rt.elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some("window"))
            .unwrap()
            .fill
    };
    assert_eq!(
        window(&rt),
        super::theme::rgb(0x191724),
        "the ground changes in one frame, without a spring"
    );
    assert_eq!(super::theme::kit().c.page, super::theme::rgb(0x191724));
    let s = &rt.state.settings;
    assert_eq!(s.chrome_theme, ChromeTheme::RosePine);
    assert_eq!(
        s.terminal_theme,
        TerminalThemeChoice::FollowChrome,
        "never chosen: the terminals follow the chrome"
    );
    assert_eq!(rt.state.term_theme, Some(TerminalTheme::rose_pine()));
    for host in &hosts {
        assert_eq!(declared(host), [Some(TerminalTheme::rose_pine().args())]);
    }
    let saved = Settings::load(&rt.state.settings_path).unwrap();
    assert_eq!(
        (saved.chrome_theme, saved.terminal_theme_chosen),
        (ChromeTheme::RosePine, false)
    );

    rt.update(|s, tx| s.perform(Intent::Chrome(ChromeTheme::GruvboxDark), tx));
    for host in &hosts {
        assert_eq!(
            declared(host),
            [Some(TerminalTheme::gruvbox_dark().args())],
            "Follow chrome follows the next palette"
        );
    }

    rt.update(|s, tx| s.perform(Intent::Theme(TerminalThemeChoice::FollowServer), tx));
    assert!(rt.state.settings.terminal_theme_chosen);
    for host in &hosts {
        declared(host);
    }
    rt.update(|s, tx| s.perform(Intent::Chrome(ChromeTheme::RosePineMoon), tx));
    assert_eq!(
        rt.state.settings.terminal_theme,
        TerminalThemeChoice::FollowServer,
        "a chosen terminal theme stays"
    );
    assert_eq!(rt.state.term_theme, None);
    assert!(hosts.iter().all(|h| declared(h).is_empty()));
    let again = relaunch(&rt.state);
    assert_eq!(again.settings.chrome_theme, ChromeTheme::RosePineMoon);
    assert_eq!(again.term_colors().background, 0x232136);
    rt.update(|s, tx| s.perform(Intent::Chrome(ChromeTheme::Midnight), tx));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_palette_round_trip_leaves_terminal_output_and_its_selection_alone() {
    use rex_linux::{
        connection::{Connection, TerminalSender},
        settings::ChromeTheme,
    };
    let (mut ws, _rx) = tabbed_workspace("single");
    let (sender, _sent) = flume::unbounded();
    let connection = Connection {
        model: Default::default(),
        commands: TerminalSender::new(sender, "s".into(), "block:a".into()),
        wake: flume::unbounded().1,
    };
    let mut terminal = ghostty_vt::Terminal::new(ghostty_vt::TerminalOptions {
        cols: 40,
        rows: 4,
        ..Default::default()
    })
    .unwrap();
    terminal.vt_write(b"plain \x1b[31mred\x1b[0m");
    let frame = std::sync::Arc::new(
        rex_linux::terminal::Renderer::new()
            .unwrap()
            .frame(&terminal)
            .unwrap(),
    );
    {
        let mut m = connection.model.lock().unwrap();
        m.connected = true;
        m.control_connected = true;
        m.frame = Some(frame.clone());
    }
    let key = ws.term_key("block:a");
    let mut term = super::term::Term::new(connection);
    term.selection = Some((0, 4));
    ws.terminals.insert(key.clone(), term);
    let mut rt = runtime(ws);
    let output = |rt: &Runtime<Workspace>| {
        rt.elements()
            .into_iter()
            .filter(|e| {
                e.text
                    .as_deref()
                    .is_some_and(|t| t.contains("plain") || t.contains("red"))
            })
            .map(|e| (e.text, format!("{:?}", e.text_color), e.rect))
            .collect::<Vec<_>>()
    };
    let before = output(&rt);
    assert!(!before.is_empty(), "the pane draws its text");
    rt.update(|s, tx| s.set_chrome(ChromeTheme::RosePine, tx));
    rt.frame(1. / 60.);
    assert_eq!(output(&rt), before, "the server's text and colours stay");
    assert_eq!(
        rt.state.term_colors().selection_bg,
        0x403d52,
        "the selection is drawn in the new palette's colour"
    );
    rt.update(|s, tx| s.set_chrome(ChromeTheme::Midnight, tx));
    rt.frame(1. / 60.);
    assert_eq!(output(&rt), before);
    let term = &rt.state.terminals[&key];
    assert_eq!(term.selection, Some((0, 4)), "the selection is intact");
    assert!(std::sync::Arc::ptr_eq(term.frame.as_ref().unwrap(), &frame));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn files_dropped_on_a_pane_upload_there_and_focus_it() {
    use rex_linux::connection::{Connection, TerminalSender};
    let (mut ws, rx) = tabbed_workspace("split");
    let (sender, sent) = flume::unbounded();
    for block in ["block:b", "block:c"] {
        let connection = Connection {
            model: Default::default(),
            commands: TerminalSender::new(sender.clone(), "s".into(), block.into()),
            wake: flume::unbounded().1,
        };
        {
            let mut m = connection.model.lock().unwrap();
            m.connected = true;
            m.control_connected = true;
        }
        let key = ws.term_key(block);
        ws.terminals.insert(key, super::term::Term::new(connection));
    }
    let mut rt = runtime(ws);
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    let r = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("pane:block:c"))
        .expect("the right pane")
        .rect;
    let at = (r[0] + r[2] / 2., r[1] + r[3] / 2.);
    let path = std::path::PathBuf::from("/tmp/a b.png");
    rt.drag_files(vec![path.clone()], Some(at));
    rt.frame(1. / 60.);
    assert_eq!(rt.state.drop_hover.as_deref(), Some("block:c"));
    rt.drop_files(vec![path], Some(at));
    rt.frame(1. / 60.);
    assert_eq!(rt.state.drop_hover, None);
    let uploads: Vec<_> = rx
        .try_iter()
        .filter_map(|c| match c {
            HostCommand::Upload {
                session,
                block,
                files,
            } => Some((session, block, files[0].name.clone(), files.len())),
            _ => None,
        })
        .collect();
    assert_eq!(
        uploads,
        [("s".into(), "block:c".into(), "a b.png".into(), 1)]
    );
    assert!(
        sent.try_iter()
            .any(|c| matches!(c, HostCommand::Request { method, payload }
            if method == "session.focus_block" && payload["block_id"] == "block:c")),
        "the pane a file is dropped on takes focus"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

/// A tailnet of four machines: a Rex server with built-in Tailscale, a Linux box with
/// Tailscale SSH, an Android phone, and one that is offline, plus an iPhone that is never
/// listed.
fn tailnet() -> Vec<rex_linux::tailnet::Machine> {
    let machine = |name: &str, os: &str, online: bool, ssh: bool| rex_linux::tailnet::Machine {
        name: name.into(),
        address: format!("{name}.tail1234.ts.net"),
        host_name: None,
        os: os.into(),
        online,
        ssh,
        this_machine: false,
    };
    vec![
        rex_linux::tailnet::Machine {
            this_machine: true,
            ..machine("here", "linux", true, false)
        },
        machine("studio-rex", "macOS", true, false),
        machine("box", "linux", true, true),
        machine("away", "linux", false, false),
        machine("phone", "iOS", true, false),
        machine("pixel", "android", true, true),
    ]
}

fn rex_at(address: &str) -> rex_linux::tailnet::Probe {
    rex_linux::tailnet::Probe::Rex {
        endpoint: format!("https://{address}"),
        listeners: vec![],
        reason: None,
    }
}

fn open_add_host(rt: &mut Runtime<Workspace>) {
    let ran = rt.state.discovery.ran;
    click(rt, "add-host");
    assert!(
        rt.state.panel.is_some(),
        "the sidebar's Add host opens the list"
    );
    let generation = rt.state.discovery.generation;
    assert_eq!(
        rt.state.discovery.ran,
        ran + 1,
        "the list looks at the tailnet once"
    );
    // As the scan thread would: through the handle, so the list redraws.
    rt.update(|s, _| {
        s.apply_scan(generation, Ok(tailnet()));
        s.apply_probe(
            generation,
            "studio-rex.tail1234.ts.net",
            rex_at("studio-rex.tail1234.ts.net"),
        );
        s.apply_probe(
            generation,
            "box.tail1234.ts.net",
            rex_linux::tailnet::Probe::Nothing,
        );
        s.apply_probe(
            generation,
            "pixel.tail1234.ts.net",
            rex_linux::tailnet::Probe::Nothing,
        );
    });
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
}

#[test]
fn ssh_start_is_explicit_one_shot_and_defaults_to_not_now() {
    let mut ws = workspace();
    ws.settings.hosts[0].endpoint = "ssh://work-alias".into();
    ws.hosts[0].model.ssh_server_missing = true;
    let commands = capture_host(&mut ws, 0);
    let mut rt = runtime(ws);
    assert!(has(&rt, "offer-ssh-start"));
    assert!(rt.state.ssh_setup.ran.is_empty());
    click(&mut rt, "offer-ssh-start");
    assert!(matches!(
        rt.state.panel.as_ref().unwrap().kind,
        PanelKind::StartSsh { .. }
    ));
    assert!(
        rt.state.ssh_setup.ran.is_empty(),
        "opening a preview cannot execute"
    );
    press(&mut rt, Key::Enter);
    assert!(rt.state.panel.is_none(), "Enter defaults to Not now");
    assert!(rt.state.ssh_setup.ran.is_empty());
    click(&mut rt, "offer-ssh-start");
    click(&mut rt, "confirm-ssh-start");
    assert_eq!(rt.state.ssh_setup.ran, ["ssh://work-alias"]);
    rt.state.submit(&mut rune::Tx::default());
    assert_eq!(rt.state.ssh_setup.ran.len(), 1, "no duplicate while busy");
    rt.update(|s, tx| {
        s.ssh_start_finished(
            "ssh://work-alias",
            Err(anyhow::anyhow!("remote Rex is too old").into()),
            tx,
        )
    });
    assert!(
        rt.state
            .panel
            .as_ref()
            .unwrap()
            .error
            .as_ref()
            .unwrap()
            .contains("too old")
    );
    assert!(rt.state.ssh_setup.running.is_none());
    assert_eq!(rt.state.ssh_setup.ran.len(), 1, "errors never retry");
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
    click(&mut rt, "confirm-ssh-start");
    assert_eq!(rt.state.ssh_setup.ran.len(), 2);
    rt.update(|s, tx| {
        s.ssh_start_finished(
            "ssh://work-alias",
            Ok(rex_linux::ssh::setup::Attachment::Automatic),
            tx,
        )
    });
    assert!(rt.state.panel.is_none());
    let sent: Vec<_> = commands.try_iter().collect();
    assert!(sent.iter().any(|c| matches!(c, HostCommand::Reconnect)));
    assert!(
        !sent
            .iter()
            .any(|c| matches!(c, HostCommand::CreateSession { .. }))
    );
}

#[test]
fn ssh_setup_offers_explicit_socket_without_reconnecting_or_starting_again() {
    let mut ws = workspace();
    ws.settings.hosts[0].endpoint = "ssh://box".into();
    ws.hosts[0].model.ssh_server_missing = true;
    ws.preview_ssh_start(0);
    ws.ssh_setup.running = Some("ssh://box".into());
    let commands = capture_host(&mut ws, 0);
    ws.ssh_start_finished(
        "ssh://box",
        Ok(rex_linux::ssh::setup::Attachment::Review {
            reason: "Automatic lookup cannot find this socket".into(),
            endpoint: Some("ssh://box/custom/socket".into()),
        }),
        &mut rune::Tx::default(),
    );
    assert!(
        !commands
            .try_iter()
            .any(|c| matches!(c, HostCommand::Reconnect))
    );
    assert_eq!(ws.settings.hosts[0].endpoint, "ssh://box");
    ws.start_ssh("ssh://box".into());
    assert!(ws.ssh_setup.ran.is_empty());
    ws.save_ssh_socket("ssh://box", &mut rune::Tx::default());
    assert_eq!(ws.settings.hosts[0].endpoint, "ssh://box/custom/socket");
    assert!(ws.panel.is_none());
}

#[test]
fn uncertain_ssh_setup_reconnects_once_without_retrying_setup() {
    let mut ws = workspace();
    ws.settings.hosts[0].endpoint = "ssh://box".into();
    ws.ssh_setup.running = Some("ssh://box".into());
    let commands = capture_host(&mut ws, 0);
    for _ in 0..2 {
        ws.ssh_start_finished(
            "ssh://box",
            Err(rex_linux::ssh::setup::Failure {
                message: "Server may have started: timeout".into(),
                may_have_started: true,
            }),
            &mut rune::Tx::default(),
        );
    }
    assert_eq!(
        commands
            .try_iter()
            .filter(|c| matches!(c, HostCommand::Reconnect))
            .count(),
        1
    );
    assert!(ws.ssh_setup.ran.is_empty());
}

#[test]
fn ssh_setup_completion_does_not_reconnect_removed_or_connected_hosts() {
    for connected in [false, true] {
        let mut ws = workspace();
        ws.settings.hosts[0].endpoint = "ssh://box".into();
        ws.ssh_setup.running = Some("ssh://box".into());
        let commands = capture_host(&mut ws, 0);
        if connected {
            ws.hosts[0].model.connected = true;
        } else {
            ws.settings.hosts[0].endpoint = "ssh://different".into();
        }
        ws.ssh_start_finished(
            "ssh://box",
            Ok(rex_linux::ssh::setup::Attachment::Automatic),
            &mut rune::Tx::default(),
        );
        assert!(
            !commands
                .try_iter()
                .any(|c| matches!(c, HostCommand::Reconnect))
        );
    }
}

#[test]
fn ssh_setup_does_not_offer_for_auth_errors_custom_sockets_or_stale_hosts() {
    let mut ws = workspace();
    ws.settings.hosts[0].endpoint = "ssh://box".into();
    assert!(
        !ws.can_start_ssh(0),
        "a generic disconnected host is not a missing server"
    );
    ws.hosts[0].model.ssh_server_missing = true;
    assert!(ws.can_start_ssh(0));
    ws.preview_ssh_start(0);
    ws.settings.hosts[0].endpoint = "ssh://different".into();
    ws.submit(&mut rune::Tx::default());
    assert!(ws.ssh_setup.ran.is_empty());
    assert!(ws.panel.as_ref().unwrap().error.is_some());
    ws.settings.hosts[0].endpoint = "ssh://box/custom.sock".into();
    assert!(!ws.can_start_ssh(0));
    ws.settings.hosts[0].endpoint = "ssh://box".into();
    ws.hosts[0].model.connected = true;
    assert!(!ws.can_start_ssh(0));
}

#[test]
fn discovered_listeners_are_reviewed_before_connecting() {
    let mut ws = workspace();
    ws.apply_scan(0, Ok(tailnet()));
    ws.apply_probe(
        0,
        "box.tail1234.ts.net",
        rex_linux::tailnet::Probe::Rex {
            endpoint: "https://box.tail1234.ts.net".into(),
            reason: None,
            listeners: vec![rex_linux::tailnet::Listener {
                endpoint: "0.0.0.0:7297".into(),
                addresses: vec!["[::]:7297".into()],
                transport: "tcp".into(),
                admission: Some("anonymous".into()),
                primary: Some(true),
            }],
        },
    );
    assert!(
        ws.machine_form_note(2)
            .contains("not reachability or permission checks")
    );
    let before = ws.settings.hosts.len();
    ws.add_machine(2, &mut rune::Tx::default());
    assert_eq!(ws.settings.hosts.len(), before);
    let panel = ws.panel.as_ref().unwrap();
    let PanelKind::Form(Form::Discovered(note)) = &panel.kind else {
        panic!("listener review")
    };
    assert!(note.contains("Reachable from this device: https://box"));
    assert!(note.contains("[::]:7297"));
    assert!(note.contains("anonymous"));
    assert!(note.contains("not reachability or permission checks"));
    ws.submit(&mut rune::Tx::default());
    assert_eq!(
        ws.settings.hosts.last().unwrap().endpoint,
        "https://box.tail1234.ts.net"
    );
}

#[test]
fn add_host_lists_the_tailnet_by_kind_and_hides_iphones_and_this_machine() {
    let mut ws = workspace();
    ws.sidebar.toggle();
    // Tall enough for the sidebar's Add host button to be on screen.
    let mut rt = runtime_at(ws, (1024., 900.));
    click(&mut rt, "add-host");
    let panel = rt.state.panel.as_ref().expect("the Add host list");
    assert!(matches!(panel.kind, PanelKind::Discover));
    assert!(panel.is_list());
    assert_eq!(
        rt.state.discovery_empty_message(),
        "Looking at your tailnet…"
    );
    let generation = rt.state.discovery.generation;
    rt.state.apply_scan(generation, Ok(tailnet()));
    let before: Vec<String> = rt
        .state
        .entries()
        .iter()
        .map(|e| e.detail.clone())
        .collect();
    assert!(
        before.iter().any(|d| d.starts_with("Checking for Rex…")),
        "online machines are checked: {before:?}"
    );
    rt.state.apply_probe(
        generation,
        "studio-rex.tail1234.ts.net",
        rex_at("studio-rex.tail1234.ts.net"),
    );
    rt.state.apply_probe(
        generation,
        "box.tail1234.ts.net",
        rex_linux::tailnet::Probe::Nothing,
    );
    rt.state.apply_probe(
        generation,
        "pixel.tail1234.ts.net",
        rex_linux::tailnet::Probe::Unknown("https 443: timed out".into()),
    );
    rt.state.apply_probe(
        generation - 1,
        "away.tail1234.ts.net",
        rex_at("away.tail1234.ts.net"),
    );
    let entries = rt.state.entries();
    let rows: Vec<(String, String, String)> = entries
        .iter()
        .map(|e| (e.group.clone(), e.label.clone(), e.detail.clone()))
        .collect();
    assert_eq!(
        rows,
        [
            (
                "Rex servers".into(),
                "studio-rex".into(),
                "https://studio-rex.tail1234.ts.net · listener details unavailable · macOS".into()
            ),
            (
                "Tailnet".into(),
                "box".into(),
                "Tailscale SSH · Linux".into()
            ),
            (
                "Tailnet".into(),
                "pixel".into(),
                "Rex check failed · Tailscale SSH · Android".into()
            ),
            ("Offline".into(), "away".into(), "Offline · Linux".into()),
            ("".into(), "Enter an endpoint…".into(), "".into()),
        ],
        "an old scan's probe changes nothing; a failed check is said, not taken as absence"
    );
    assert!(matches!(entries[0].action, Intent::AddMachine(1)));
    assert!(matches!(entries[1].action, Intent::MachineForm(2)));
    assert!(matches!(entries[2].action, Intent::MachineForm(5)));
    assert!(matches!(entries[3].action, Intent::MachineForm(3)));
    assert!(entries[4].footer);
    assert!(matches!(entries[4].action, Intent::AddHostManually));
    assert!(
        rt.state
            .machine_form_note(5)
            .contains("https 443: timed out"),
        "the form says what the check found"
    );
    assert!(
        rt.state
            .machine_form_note(2)
            .contains("Nothing answered on ports 443 or 7297"),
        "{}",
        rt.state.machine_form_note(2)
    );

    // A Rex server on port 7297 is one too, and says so.
    rt.state.apply_probe(
        generation,
        "box.tail1234.ts.net",
        rex_linux::tailnet::Probe::Rex {
            endpoint: "https://box.tail1234.ts.net:7297".into(),
            listeners: vec![],
            reason: None,
        },
    );
    let entries = rt.state.entries();
    assert_eq!(entries[1].label, "box");
    assert_eq!(entries[1].group, "Rex servers");
    assert!(
        entries[1]
            .detail
            .starts_with("https://box.tail1234.ts.net:7297")
    );
    assert!(matches!(entries[1].action, Intent::AddMachine(2)));

    rt.state.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("bo");
    rt.state.query_changed();
    let names: Vec<String> = rt.state.entries().iter().map(|e| e.label.clone()).collect();
    assert_eq!(names, ["box", "Enter an endpoint…"]);

    rt.state.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("nothing");
    rt.state.query_changed();
    assert_eq!(
        rt.state.discovery_empty_message(),
        "No machine by that name on your tailnet."
    );
    rt.state.apply_scan(
        generation,
        Err("Tailscale isn't installed on this machine".into()),
    );
    assert_eq!(rt.state.entries().len(), 1, "only the footer remains");
    assert_eq!(
        rt.state.discovery_empty_message(),
        "Tailscale isn't installed on this machine"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_rex_server_on_the_tailnet_is_added_with_one_press() {
    let mut ws = workspace();
    ws.sidebar.toggle();
    // Tall enough for the sidebar's Add host button to be on screen.
    let mut rt = runtime_at(ws, (1024., 900.));
    open_add_host(&mut rt);
    assert_eq!(rt.state.panel.as_ref().unwrap().selected, 0);
    press(&mut rt, Key::Enter);
    assert!(rt.state.panel.is_none(), "added and closed");
    assert_eq!(rt.state.settings.hosts.len(), 2);
    let host = &rt.state.settings.hosts[1];
    assert_eq!(host.label, "studio-rex");
    assert_eq!(host.endpoint, "https://studio-rex.tail1234.ts.net");
    assert_eq!(rt.state.active, 1, "the new host is the active one");
    assert_eq!(rt.state.hosts.len(), 2, "and it is connecting");
    let saved = Settings::load(&rt.state.settings_path).unwrap();
    assert_eq!(
        saved.hosts[1].endpoint,
        "https://studio-rex.tail1234.ts.net"
    );

    // Listed again, it is marked as added; pressing it offers another login over SSH
    // rather than adding the same endpoint twice.
    open_add_host(&mut rt);
    let entries = rt.state.entries();
    assert_eq!(entries[0].label, "studio-rex");
    assert!(entries[0].current && entries[0].enabled);
    assert!(
        entries[0]
            .detail
            .starts_with("Added as studio-rex · https://studio-rex.tail1234.ts.net")
    );
    assert!(matches!(entries[0].action, Intent::MachineForm(1)));
    press(&mut rt, Key::Enter);
    let panel = rt.state.panel.as_ref().expect("the SSH form");
    assert!(matches!(panel.kind, PanelKind::Form(Form::AddSsh(1))));
    assert!(
        rt.state
            .machine_form_note(1)
            .contains("Rex also answers at https://studio-rex.tail1234.ts.net"),
        "{}",
        rt.state.machine_form_note(1)
    );
    rt.state.panel.as_mut().unwrap().inputs[1] = rune::TextEdit::new("root");
    click(&mut rt, "save-form");
    assert_eq!(
        rt.state.settings.hosts.len(),
        3,
        "a second login to the machine"
    );
    assert_eq!(
        rt.state.settings.hosts[2].endpoint,
        "ssh://root@studio-rex.tail1234.ts.net"
    );
    open_add_host(&mut rt);
    let entries = rt.state.entries();
    assert!(
        entries[0]
            .detail
            .starts_with("Added as studio-rex, studio-rex · "),
        "{}",
        entries[0].detail
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn another_machine_takes_a_user_and_connects_over_ssh() {
    let mut ws = workspace();
    ws.sidebar.toggle();
    // Tall enough for the sidebar's Add host button to be on screen.
    let mut rt = runtime_at(ws, (1024., 900.));
    open_add_host(&mut rt);
    rt.state.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("box");
    rt.state.query_changed();
    press(&mut rt, Key::Enter);
    let panel = rt.state.panel.as_ref().expect("the SSH form");
    assert!(matches!(panel.kind, PanelKind::Form(Form::AddSsh(2))));
    assert_eq!(panel.title, "Add box over SSH");
    assert_eq!(panel.inputs[0].text, "box");
    assert_eq!(
        panel.inputs[1].text,
        super::discover::local_user(),
        "the local user name is offered"
    );
    assert_eq!(
        panel.inputs[2].text, "",
        "the socket is looked up unless named"
    );
    assert!(
        rt.state
            .machine_form_note(2)
            .contains("Tailscale SSH is on"),
        "{}",
        rt.state.machine_form_note(2)
    );
    assert!(has(&rt, "save-form"));

    rt.state.panel.as_mut().unwrap().inputs[1] = rune::TextEdit::new("me@box");
    click(&mut rt, "save-form");
    let panel = rt.state.panel.as_ref().expect("still the form");
    assert_eq!(
        panel.error.as_deref(),
        Some("Enter one user name, without the host")
    );
    rt.state.panel.as_mut().unwrap().inputs[1] = rune::TextEdit::new("");
    click(&mut rt, "save-form");
    assert_eq!(
        rt.state.panel.as_ref().unwrap().error.as_deref(),
        Some("Enter the user to log in as")
    );

    rt.state.panel.as_mut().unwrap().inputs[1] = rune::TextEdit::new("admin");
    rt.state.panel.as_mut().unwrap().inputs[2] = rune::TextEdit::new("run/rex/server.sock");
    click(&mut rt, "save-form");
    assert_eq!(
        rt.state.panel.as_ref().unwrap().error.as_deref(),
        Some("Enter an absolute socket path, such as /run/rex/server.sock")
    );

    rt.state.panel.as_mut().unwrap().inputs[2] = rune::TextEdit::new("");
    click(&mut rt, "save-form");
    assert!(rt.state.panel.is_none());
    let host = &rt.state.settings.hosts[1];
    assert_eq!(host.label, "box");
    assert_eq!(host.endpoint, "ssh://admin@box.tail1234.ts.net");
    assert_eq!(rt.state.active, 1);

    // The same machine, as another user at a named socket, is another host.
    open_add_host(&mut rt);
    rt.state.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("box");
    rt.state.query_changed();
    let entries = rt.state.entries();
    assert!(entries[0].current, "marked as added");
    assert!(entries[0].detail.starts_with("Added as box · "));
    press(&mut rt, Key::Enter);
    rt.state.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("box (shared)");
    rt.state.panel.as_mut().unwrap().inputs[1] = rune::TextEdit::new("ops");
    rt.state.panel.as_mut().unwrap().inputs[2] = rune::TextEdit::new("/run/rex/server.sock");
    click(&mut rt, "save-form");
    assert!(
        rt.state.panel.is_none(),
        "{:?}",
        rt.state.panel.as_ref().unwrap().error
    );
    assert_eq!(
        rt.state.settings.hosts[2].endpoint,
        "ssh://ops@box.tail1234.ts.net/run/rex/server.sock"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn escape_walks_back_from_a_machine_form_to_the_list_and_settings() {
    let mut ws = workspace();
    let mut tx = rune::Tx::default();
    ws.perform(Intent::Settings, &mut tx);
    ws.perform(Intent::AddHost, &mut tx);
    let generation = ws.discovery.generation;
    ws.apply_scan(generation, Ok(tailnet()));
    ws.apply_probe(
        generation,
        "studio-rex.tail1234.ts.net",
        rex_at("studio-rex.tail1234.ts.net"),
    );
    ws.perform(Intent::MachineForm(3), &mut tx);
    let mut rt = runtime(ws);
    assert!(matches!(
        rt.state.panel.as_ref().unwrap().kind,
        PanelKind::Form(Form::AddSsh(3))
    ));
    press(&mut rt, Key::Escape);
    assert!(
        matches!(rt.state.panel.as_ref().unwrap().kind, PanelKind::Discover),
        "Escape on the form returns to the list"
    );
    press(&mut rt, Key::Escape);
    assert!(
        matches!(rt.state.panel.as_ref().unwrap().kind, PanelKind::Settings),
        "and from the list to Settings"
    );

    // Saving from the form returns to Settings through the list.
    rt.update(|s, tx| {
        s.perform(Intent::AddHost, tx);
        let generation = s.discovery.generation;
        s.apply_scan(generation, Ok(tailnet()));
        s.perform(Intent::MachineForm(3), tx);
        s.panel.as_mut().unwrap().inputs[1] = rune::TextEdit::new("me");
    });
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }
    click(&mut rt, "save-form");
    let panel = rt.state.panel.as_ref().expect("back in Settings");
    assert!(
        matches!(panel.kind, PanelKind::Settings),
        "{:?} {:?}",
        panel.kind,
        panel.error
    );
    assert_eq!(
        rt.state.settings.hosts[1].endpoint,
        "ssh://me@away.tail1234.ts.net"
    );

    // Adding a Rex server from the list goes back to Settings too.
    rt.update(|s, tx| {
        s.perform(Intent::AddHost, tx);
        let generation = s.discovery.generation;
        s.apply_scan(generation, Ok(tailnet()));
        s.apply_probe(
            generation,
            "studio-rex.tail1234.ts.net",
            rex_at("studio-rex.tail1234.ts.net"),
        );
        s.perform(Intent::AddMachine(1), tx);
    });
    assert!(matches!(
        rt.state.panel.as_ref().unwrap().kind,
        PanelKind::Settings
    ));
    assert_eq!(rt.state.settings.hosts.len(), 3);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn an_endpoint_by_hand_takes_ssh_and_rejects_duplicates() {
    let mut ws = workspace();
    ws.sidebar.toggle();
    // Tall enough for the sidebar's Add host button to be on screen.
    let mut rt = runtime_at(ws, (1024., 900.));
    open_add_host(&mut rt);
    let footer = rt.state.entries().len() - 1;
    click(&mut rt, &format!("entry:{footer}"));
    let panel = rt.state.panel.as_ref().expect("the endpoint form");
    assert!(matches!(panel.kind, PanelKind::Form(Form::AddHost)));
    assert!(panel.back.is_some(), "it goes back to the list");
    rt.state.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("lab");
    rt.state.panel.as_mut().unwrap().inputs[1] = rune::TextEdit::new("ssh://lab/");
    click(&mut rt, "save-form");
    assert_eq!(
        rt.state.panel.as_ref().unwrap().error.as_deref(),
        Some("Use an absolute socket path after the host, such as /run/rex/server.sock")
    );
    rt.state.panel.as_mut().unwrap().inputs[1] =
        rune::TextEdit::new("unix:///nonexistent/rex-test.sock");
    click(&mut rt, "save-form");
    assert_eq!(
        rt.state.panel.as_ref().unwrap().error.as_deref(),
        Some("This endpoint is already in the workspace")
    );
    rt.state.panel.as_mut().unwrap().inputs[1] = rune::TextEdit::new("ssh://ops@lab:2222");
    click(&mut rt, "save-form");
    assert!(rt.state.panel.is_none());
    assert_eq!(rt.state.settings.hosts[1].endpoint, "ssh://ops@lab:2222");
    assert_eq!(rt.state.settings.hosts[1].label, "lab");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_search_field_keeps_clicks_from_a_program_that_tracks_the_mouse() {
    use rex_linux::connection::{Command, Connection, TerminalSender};
    let (mut ws, _rx) = tabbed_workspace("single");
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
    let key = ws.term_key("block:a");
    let mut term = super::term::Term::new(connection);
    term.refresh();
    ws.terminals.insert(key.clone(), term);
    let mut rt = runtime(ws);
    rt.frame(1. / 60.);
    // Presses and releases; a program that asks for every move still hears the pointer pass.
    let clicks = move || {
        sent.try_iter()
            .filter(|c| {
                matches!(
                    c,
                    HostCommand::Terminal {
                        command: Command::Mouse(m),
                        ..
                    } if m["action"] != "motion"
                )
            })
            .count()
    };
    ctrl_shift(&mut rt, 'f');
    assert!(has(&rt, "search:block:a"));
    click(&mut rt, "search-field:block:a");
    assert_eq!(clicks(), 0, "a click on the field isn't the program's");
    assert!(
        focused(&rt).is_some_and(|path| path.contains("search-field:block:a")),
        "and the field keeps the keyboard"
    );

    // A click on the terminal is, and takes the keyboard back there.
    click(&mut rt, "grid:block:a");
    assert_eq!(
        clicks(),
        2,
        "a click on the terminal is: press, then release"
    );
    assert_eq!(focused(&rt), None, "the terminal has the keyboard");
    assert!(has(&rt, "search:block:a"), "the search stays open");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn ctrl_shift_f_searches_the_focused_terminal_without_typing_into_it() {
    use rex_linux::connection::{Command, Connection, TerminalSender};
    let (mut ws, _rx) = tabbed_workspace("single");
    let (sender, sent) = flume::unbounded();
    let connection = Connection {
        model: Default::default(),
        commands: TerminalSender::new(sender, "s".into(), "block:a".into()),
        wake: flume::unbounded().1,
    };
    {
        let mut m = connection.model.lock().unwrap();
        m.connected = true;
        m.control_connected = true;
    }
    let key = ws.term_key("block:a");
    ws.terminals
        .insert(key.clone(), super::term::Term::new(connection));
    let mut rt = runtime(ws);
    rt.frame(1. / 60.);
    let sent = move || -> Vec<String> {
        sent.try_iter()
            .filter_map(|c| match c {
                HostCommand::Terminal { command, .. } => Some(match command {
                    Command::Search(needle) => format!("search {needle}"),
                    Command::SearchStep(older) => format!("step older={older}"),
                    Command::Key(_) => "key".into(),
                    _ => "other".into(),
                }),
                _ => None,
            })
            .collect()
    };

    ctrl_shift(&mut rt, 'f');
    assert!(
        focused(&rt).is_some_and(|path| path.contains("search-field:block:a")),
        "the search field takes the keyboard: {:?}",
        focused(&rt)
    );
    for c in ["n", "e"] {
        press(&mut rt, Key::Text(c.into()));
    }
    press(&mut rt, Key::Enter);
    rt.key(KeyEvent {
        key: Key::Enter,
        mods: Mods {
            shift: true,
            ..Mods::default()
        },
    });
    rt.frame(1. / 60.);
    assert_eq!(
        sent(),
        [
            "search n",
            "search ne",
            "step older=true",
            "step older=false"
        ],
        "typing searches, Enter steps, and nothing reaches the program"
    );
    assert!(has(&rt, "search:block:a"));

    press(&mut rt, Key::Escape);
    assert_eq!(sent(), ["search "], "Escape clears the search");
    assert!(rt.state.terminals[&key].search.is_none());
    assert!(!has(&rt, "search:block:a"));
    assert_eq!(focused(&rt), None, "the terminal has the keyboard again");
    press(&mut rt, Key::Text("x".into()));
    assert_eq!(sent(), ["key"]);

    // The shortcut runs a named action, which bindings can run by its name.
    assert_eq!(super::actions::SEARCH, "client.search");
    rt.update(|s, tx| {
        assert!(!super::actions::run(
            s,
            "client.nothing",
            &serde_json::Value::Null,
            tx
        ));
        assert!(super::actions::run(
            s,
            super::actions::SEARCH,
            &serde_json::Value::Null,
            tx
        ));
    });
    rt.frame(1. / 60.);
    assert!(
        focused(&rt).is_some_and(|path| path.contains("search-field:block:a")),
        "client.search opens the search field: {:?}",
        focused(&rt)
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn update_notice_toast_opens_whats_new_dismisses_for_version_and_resurfaces_on_newer() {
    use rex_linux::update::{CheckOutcome, InstallChannel, UpdateState};
    let mut ws = workspace();
    let state_path = ws.settings_path.with_extension("update.json");
    ws.updates.config.state_path = state_path.clone();
    ws.updates.config.current_version = "0.5.3".into();
    ws.updates.channel = InstallChannel::Pacman {
        omarchy: false,
        custom_hint: None,
    };

    let state = UpdateState {
        last_checked: Some(1_762_000_000),
        highest_verified: Some("0.6.0".into()),
        latest_version: Some("0.6.0".into()),
        notes_url: Some("https://github.com/shuv1337/rex-rune/releases/tag/v0.6.0".into()),
        published: Some("2026-11-10T00:00:00Z".into()),
        unsupported_floor: None,
        dismissed_version: None,
        announced_version: None,
    };
    ws.finish_update_check(
        false,
        Ok((
            CheckOutcome::UpdateAvailable {
                version: "0.6.0".into(),
                notes_url: "https://github.com/shuv1337/rex-rune/releases/tag/v0.6.0".into(),
                published: "2026-11-10T00:00:00Z".into(),
            },
            state,
        )),
    );

    let mut rt = runtime(ws);
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    assert!(
        has(&rt, "toast-update"),
        "update toast appears when newer version exists"
    );
    let toast_texts: Vec<_> = rt.elements().into_iter().filter_map(|e| e.text).collect();
    assert!(
        toast_texts
            .iter()
            .any(|t| t.contains("sudo pacman -Syu rex-linux-bin")),
        "toast carries pacman channel command: {toast_texts:?}"
    );

    click(&mut rt, "toast-update-whats-new");
    assert_eq!(
        std::mem::take(&mut rt.state.opened_links),
        ["https://github.com/shuv1337/rex-rune/releases/tag/v0.6.0"]
    );
    assert!(
        has(&rt, "toast-update"),
        "What's new leaves the toast visible until dismissed"
    );

    click(&mut rt, "toast-update-dismiss");
    assert!(
        !has(&rt, "toast-update"),
        "Dismiss hides the toast for 0.6.0"
    );
    let saved = UpdateState::load(&state_path);
    assert_eq!(saved.dismissed_version.as_deref(), Some("0.6.0"));

    // A later background check returning the same 0.6.0 stays dismissed.
    rt.update(|s, _| {
        let same = saved.clone();
        s.finish_update_check(
            false,
            Ok((
                CheckOutcome::UpdateAvailable {
                    version: "0.6.0".into(),
                    notes_url: "https://github.com/shuv1337/rex-rune/releases/tag/v0.6.0".into(),
                    published: "2026-11-10T00:00:00Z".into(),
                },
                same,
            )),
        );
    });
    rt.frame(1. / 60.);
    assert!(
        !has(&rt, "toast-update"),
        "0.6.0 remains dismissed on background check"
    );

    // When 0.6.1 is released, the toast resurfaces automatically.
    rt.update(|s, _| {
        let mut newer = UpdateState::load(&state_path);
        newer.highest_verified = Some("0.6.1".into());
        newer.latest_version = Some("0.6.1".into());
        newer.notes_url = Some("https://github.com/shuv1337/rex-rune/releases/tag/v0.6.1".into());
        s.finish_update_check(
            false,
            Ok((
                CheckOutcome::UpdateAvailable {
                    version: "0.6.1".into(),
                    notes_url: "https://github.com/shuv1337/rex-rune/releases/tag/v0.6.1".into(),
                    published: "2026-11-11T00:00:00Z".into(),
                },
                newer,
            )),
        );
    });
    rt.frame(1. / 60.);
    assert!(
        has(&rt, "toast-update"),
        "0.6.1 resurfaces the update toast"
    );
    let _ = std::fs::remove_file(&state_path);
    let _ = std::fs::remove_file(state_path.with_extension("lock"));
    let _ = std::fs::remove_file(&rt.state.settings_path);
}

#[test]
fn settings_updates_shows_channel_guidance_opt_out_and_silent_background_failures() {
    use rex_linux::update::{CHECK_INTERVAL_SECS, InstallChannel, UpdateState};
    use std::time::{Duration, Instant};
    let mut ws = workspace();
    let state_path = ws.settings_path.with_extension("update-settings.json");
    ws.updates.config.state_path = state_path.clone();
    ws.updates.config.current_version = "0.5.3".into();
    ws.updates.env_opt_out = false;
    ws.updates.channel = InstallChannel::Pacman {
        omarchy: true,
        custom_hint: None,
    };

    // Even when never checked (`last_checked: None`), a failed background check must back off for ~24h, not 30s.
    ws.updates.state = UpdateState::default();
    ws.init_updates();
    ws.finish_update_check(false, Err(anyhow::anyhow!("network unreachable")));
    assert!(
        ws.notices.is_empty(),
        "background failure must not push a toast notice"
    );
    assert!(ws.updates.last_error.is_none());
    let next_wait = ws
        .updates
        .next_check
        .expect("next check scheduled after failure")
        .saturating_duration_since(Instant::now());
    assert!(
        next_wait >= Duration::from_secs(CHECK_INTERVAL_SECS - 60),
        "failed background check must wait ~24h, got {next_wait:?}"
    );

    ws.updates.state = UpdateState {
        last_checked: Some(1_762_000_000),
        highest_verified: Some("0.6.0".into()),
        latest_version: Some("0.6.0".into()),
        notes_url: Some("https://github.com/shuv1337/rex-rune/releases/tag/v0.6.0".into()),
        published: Some("2026-11-10T00:00:00Z".into()),
        unsupported_floor: None,
        dismissed_version: None,
        announced_version: Some("0.6.0".into()),
    };

    // Manual failure records last_error and informs the user, while still backing off background checks by ~24h.
    ws.finish_update_check(true, Err(anyhow::anyhow!("offline")));
    assert_eq!(ws.updates.last_error.as_deref(), Some("offline"));
    assert!(
        ws.notices
            .iter()
            .any(|n| n.text.contains("Update check failed: offline"))
    );
    ws.notices.clear();
    ws.updates.last_error = None;

    // Clicking "Updates…" on the update toast opens Settings -> Updates directly.
    let mut rt = runtime(ws);
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    let clipboard = rune::clipboard::memory();
    rt.use_clipboard(&clipboard);
    click(&mut rt, "toast-update-open");
    assert!(matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::Settings)
    ));
    assert_eq!(
        rt.state.settings_section,
        super::panel::SettingsSection::Updates
    );

    // Omarchy channel shows `omarchy update` and copies the command.
    let texts: Vec<_> = rt.elements().into_iter().filter_map(|e| e.text).collect();
    assert!(
        texts.iter().any(|t| t.contains("omarchy update")),
        "Omarchy guidance shown: {texts:?}"
    );
    click(&mut rt, "updates-copy-command");
    assert_eq!(
        clipboard.text().as_deref().map(String::as_str),
        Some("omarchy update")
    );

    // Flipping the Automatic update checks switch opts out, saves to workspace.json, clears the timer, and hides the cached toast.
    assert!(rt.state.settings.updates.check);
    assert!(has(&rt, "toast-update"));
    click(&mut rt, "updates-auto-switch");
    assert!(!rt.state.settings.updates.check);
    assert!(rt.state.updates.next_check.is_none());
    assert!(
        !has(&rt, "toast-update"),
        "opting out hides cached update toast"
    );
    let reloaded = Settings::load(&rt.state.settings_path).unwrap();
    assert!(!reloaded.updates.check);

    // Check other channels render their specific guidance in Settings -> Updates.
    for (channel, expected_snippet) in [
        (
            InstallChannel::Tarball,
            "https://github.com/shuv1337/rex-rune/releases/latest/download/install.sh | sh",
        ),
        (
            InstallChannel::Deb { custom_hint: None },
            "sudo apt install ./rex-linux_0.6.0_",
        ),
        (InstallChannel::Nix, "nix flake update rex-rune"),
    ] {
        rt.update(|s, _| s.updates.channel = channel);
        rt.frame(1. / 60.);
        let texts: Vec<_> = rt.elements().into_iter().filter_map(|e| e.text).collect();
        assert!(
            texts.iter().any(|t| t.contains(expected_snippet)),
            "expected {expected_snippet:?} in {texts:?}"
        );
    }

    let _ = std::fs::remove_file(&state_path);
    let _ = std::fs::remove_file(state_path.with_extension("lock"));
    let _ = std::fs::remove_file(&rt.state.settings_path);
}
