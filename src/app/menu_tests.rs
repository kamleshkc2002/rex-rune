//! Menu structure: groups and separators, destructive rows, the selection, and shortcut hints
//! and tooltips that follow the effective keymap.
use super::*;
use crate::app::{
    actions,
    keycaps::spell,
    theme::{MIDNIGHT, danger, ink, muted, rgb, selection, set_palette},
};
use rex_linux::keymap::Overrides;
use rune::ElementInfo;
use serde_json::json;

fn rebind(rt: &mut Runtime<Workspace>, value: serde_json::Value) {
    rt.update(|s, _| {
        s.settings.shortcut_file.overrides = Overrides::parse(&value);
        s.local_keymap = actions::resolve(&s.settings.shortcut_file.overrides);
        s.keymap_generation += 1;
    });
    settle(rt);
}

/// Long enough for colours to ease and for removed elements to leave.
fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
}

fn element(rt: &Runtime<Workspace>, key: &str) -> ElementInfo {
    rt.elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("missing {key}"))
}

fn help(rt: &Runtime<Workspace>, key: &str) -> String {
    element(rt, key).help.unwrap_or_default()
}

/// The separators drawn in the open menu.
fn separators(rt: &Runtime<Workspace>) -> usize {
    rt.elements()
        .iter()
        .filter(|e| !e.exiting && e.role == "separator")
        .count()
}

/// Each row of the open menu as (label, group, destructive).
fn rows(rt: &Runtime<Workspace>) -> Vec<(String, u8, bool)> {
    rt.state
        .entries()
        .into_iter()
        .map(|e| (e.label, e.section, e.danger))
        .collect()
}

/// A menu's groups never interleave, and its destructive rows are its last group, ending with
/// `close`: nothing else is in that group and nothing destructive is in another.
fn assert_structured(rt: &Runtime<Workspace>, menu: &str, close: &str) {
    let rows = rows(rt);
    assert!(
        rows.windows(2).all(|w| w[0].1 <= w[1].1),
        "{menu}: groups in order: {rows:?}"
    );
    let groups = rows
        .iter()
        .map(|r| r.1)
        .collect::<std::collections::BTreeSet<_>>();
    assert!(groups.len() > 1, "{menu}: more than one group");
    assert_eq!(
        separators(rt),
        groups.len() - 1,
        "{menu}: a line a boundary"
    );
    let (last, others) = rows.split_last().unwrap();
    assert_eq!((last.0.as_str(), last.2), (close, true), "{menu}: last row");
    assert!(
        others.iter().all(|r| r.2 == (r.1 == last.1)),
        "{menu}: the last group is the destructive rows and only them: {rows:?}"
    );
}

/// A label's colour once it has settled (colours ease between states).
fn label_color(rt: &mut Runtime<Workspace>, label: &str) -> rune::Color {
    settle(rt);
    rt.elements()
        .into_iter()
        .find(|e| !e.exiting && e.text.as_deref() == Some(label))
        .and_then(|e| e.text_color)
        .unwrap_or_else(|| panic!("no text {label}"))
}

#[test]
fn every_menu_separates_its_groups_and_ends_with_its_destructive_row() {
    set_palette(&MIDNIGHT);
    let (mut ws, _rx) = tabbed_workspace("split");
    ws.sidebar.toggle();
    ws.settings.hosts.push(Host {
        label: "other".into(),
        endpoint: "unix:///nonexistent/rex-test-other.sock".into(),
        remote_control: false,
        collapsed: false,
        shell: None,
    });
    ws.hosts[0].model.sessions = vec![session("s"), session("next")];
    let mut rt = runtime(ws);

    rt.update(|s, _| s.pane_menu("block:b".into(), Some((300., 200.))));
    rt.frame(1. / 60.);
    assert_structured(&rt, "pane", "Close pane…");
    assert_eq!(
        rows(&rt).iter().map(|r| r.1).collect::<Vec<_>>(),
        [0, 0, 0, 1, 1, 1, 1, 1, 2, 2, 2, 3, 3, 3, 4],
        "clipboard, layout (four sides and Zoom), the pane itself, its terminal, Close"
    );
    assert_eq!(label_color(&mut rt, "Close pane…"), rgb(danger()));
    assert_eq!(label_color(&mut rt, "Split right"), rgb(ink()));
    press(&mut rt, Key::Escape);

    secondary_click(&mut rt, "tab:split");
    assert_structured(&rt, "tab", "Close tab…");
    assert_eq!(label_color(&mut rt, "Close tab…"), rgb(danger()));
    assert_eq!(label_color(&mut rt, "Close other tabs…"), rgb(danger()));
    press(&mut rt, Key::Escape);

    secondary_click(&mut rt, "session:0:next");
    assert_structured(&rt, "session", "Close session…");
    assert_eq!(label_color(&mut rt, "Close session…"), rgb(danger()));
    press(&mut rt, Key::Escape);

    secondary_click(&mut rt, "host:0");
    assert_structured(&rt, "host", "Remove host…");
    assert_eq!(label_color(&mut rt, "Remove host…"), rgb(danger()));
    press(&mut rt, Key::Escape);
    assert_eq!(separators(&rt), 0, "a closed menu leaves no line behind");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_destructive_row_that_cannot_run_is_muted_not_red() {
    set_palette(&MIDNIGHT);
    let mut ws = workspace();
    ws.sidebar.toggle();
    ws.hosts[0].model.connected = true;
    let mut rt = runtime(ws);
    secondary_click(&mut rt, "host:0");
    let remove = rt.state.entries().pop().unwrap();
    assert_eq!(
        (remove.label.as_str(), remove.danger, remove.enabled),
        ("Remove host…", true, false),
        "the only host cannot be removed"
    );
    assert_eq!(label_color(&mut rt, "Remove host…"), rgb(muted()));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn commands_keep_destructive_rows_last_and_a_filtered_list_is_flat() {
    let (ws, _rx) = tabbed_workspace("split");
    let mut rt = runtime(ws);
    rt.update(|s, _| s.picker(false));
    rt.frame(1. / 60.);
    let entries = rt.state.entries();
    let destructive: Vec<&str> = entries
        .iter()
        .filter(|e| e.danger)
        .map(|e| e.label.as_str())
        .collect();
    assert_eq!(
        destructive,
        [
            "Close session…",
            "Close other tabs…",
            "Close tab…",
            "Close pane or tab…",
            "Close pane…",
            "Remove host test-0…"
        ]
    );
    // Under each heading the rows that end something come last, after a hairline, and no
    // group interleaves its sections.
    let mut headings: Vec<&str> = Vec::new();
    for entry in &entries {
        if headings.last() != Some(&entry.group.as_str()) {
            headings.push(&entry.group);
        }
    }
    let mut lines = 0;
    for heading in headings {
        let group: Vec<_> = entries.iter().filter(|e| e.group == heading).collect();
        assert!(
            group.windows(2).all(|w| w[0].section <= w[1].section),
            "{heading}: sections in order"
        );
        let ends = group.last().unwrap().section;
        assert!(
            group
                .iter()
                .all(|e| e.danger == (e.danger && e.section == ends)),
            "{heading}: destructive rows are the last section"
        );
        if group.iter().any(|e| e.danger) {
            assert!(
                group.iter().all(|e| e.danger == (e.section == ends)),
                "{heading}: and nothing else is in it"
            );
        }
        lines += group
            .windows(2)
            .filter(|w| w[0].section != w[1].section)
            .count();
    }
    // A line a boundary inside a group; none across a group header.
    assert_eq!(separators(&rt), lines);
    assert!(lines >= 9, "session, tab, host and the pane's six: {lines}");

    rt.update(|s, _| {
        s.panel.as_mut().unwrap().inputs[0].text = "pane".into();
        s.query_changed();
    });
    rt.frame(1. / 60.);
    let labels: Vec<String> = rt.state.entries().into_iter().map(|e| e.label).collect();
    assert!(
        labels.contains(&"Zoom pane".to_owned()) && labels.contains(&"Close pane…".to_owned()),
        "matches from several groups: {labels:?}"
    );
    assert_eq!(separators(&rt), 0, "matches are a flat list");
    assert_eq!(
        label_color(&mut rt, "Close pane…"),
        rgb(danger()),
        "still destructive"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_selected_row_is_tinted_and_outlined_and_a_destructive_one_in_danger() {
    set_palette(&MIDNIGHT);
    let (ws, _rx) = tabbed_workspace("split");
    let mut rt = runtime(ws);
    rt.update(|s, _| s.pane_menu("block:b".into(), Some((300., 200.))));
    for _ in 0..30 {
        rt.frame(1. / 60.);
    }
    let highlights = |rt: &Runtime<Workspace>, destructive: bool| {
        let row = element(
            rt,
            &format!("entry:{}", rt.state.panel.as_ref().unwrap().selected),
        );
        rt.elements()
            .into_iter()
            .filter(|e| !e.exiting && e.fill == selection(destructive).0)
            .inspect(|e| assert_eq!(e.rect, row.rect, "the highlight covers its row"))
            .count()
    };
    let (fill, outline) = selection(false);
    assert!(
        fill.a > 0. && outline.a > fill.a,
        "a tint under a stronger outline"
    );
    assert_ne!(fill, selection(true).0);
    assert_eq!(highlights(&rt, false), 1, "one selected row");
    assert_eq!(highlights(&rt, true), 0);

    let close = rt
        .state
        .entries()
        .iter()
        .position(|e| e.label == "Close pane…")
        .unwrap();
    rt.update(|s, _| s.panel.as_mut().unwrap().selected = close);
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    assert_eq!(
        highlights(&rt, true),
        1,
        "the destructive row selects in danger"
    );
    assert_eq!(highlights(&rt, false), 0);
    assert_eq!(
        label_color(&mut rt, "Close pane…"),
        rgb(ink()),
        "on its tint the label is ink"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn hints_end_on_one_line_with_or_without_a_chevron() {
    let (ws, _rx) = tabbed_workspace("split");
    let mut rt = runtime(ws);
    rt.update(|s, _| s.pane_menu("block:b".into(), Some((300., 200.))));
    rt.frame(1. / 60.);
    let entries = rt.state.entries();
    let right = |key: String| {
        let r = element(&rt, &key).rect;
        r[0] + r[2]
    };
    let edges: Vec<f32> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| !e.keys.is_empty())
        .map(|(i, _)| right(format!("entry:{i}:keys")))
        .collect();
    assert!(
        edges.len() >= 4,
        "split, split, zoom and reconnect are bound"
    );
    assert!(
        edges.iter().all(|e| (e - edges[0]).abs() < 0.01),
        "{edges:?}"
    );
    // The chevron of a row that opens a dialog sits in the column the others keep empty.
    let rename = entries.iter().position(|e| e.followup).unwrap();
    let row = element(&rt, &format!("entry:{rename}")).rect;
    assert!(edges[0] < row[0] + row[2] - crate::app::widgets::CHEVRON_COLUMN);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn tooltips_and_hints_follow_a_rebound_or_removed_shortcut() {
    let (mut ws, _rx) = tabbed_workspace("single");
    ws.sidebar.toggle();
    let mut rt = runtime(ws);
    assert_eq!(
        help(&rt, "identity"),
        "Choose session · test-0 · Switch session · Ctrl K"
    );
    assert_eq!(help(&rt, "toggle-sidebar"), "Toggle sidebar · Ctrl Shift B");
    assert_eq!(
        help(&rt, "commands"),
        "Commands, sessions and tabs · Ctrl Shift P"
    );
    assert_eq!(help(&rt, "new-tab"), "New tab · Ctrl Shift T");
    assert_eq!(help(&rt, "settings"), "Settings · Ctrl ,");
    assert_eq!(element_text(&rt, "session-search-keys"), ["Ctrl", "K"]);

    rebind(
        &mut rt,
        json!([
            {"keys":"ctrl+k","action":null},
            {"keys":"ctrl+j","action":"session.switch"},
            {"keys":"ctrl+shift+t","action":null},
            {"keys":"ctrl+b>c","action":"client.tab.new"},
            {"keys":"ctrl+shift+p","action":null}
        ]),
    );
    assert_eq!(
        help(&rt, "identity"),
        "Choose session · test-0 · Switch session · Ctrl J"
    );
    assert_eq!(element_text(&rt, "session-search-keys"), ["Ctrl", "J"]);
    assert_eq!(help(&rt, "new-tab"), "New tab · Ctrl B, C");
    assert_eq!(
        help(&rt, "commands"),
        "Commands, sessions and tabs",
        "an unbound action names no key"
    );
    assert_eq!(
        element(&rt, "commands").label.as_deref(),
        Some("Commands"),
        "the accessible name never carried the key"
    );

    // The same keys in the switcher's rows and Commands, as keycaps.
    rt.update(|s, _| s.picker(false));
    rt.frame(1. / 60.);
    let entries = rt.state.entries();
    let switch = entries
        .iter()
        .position(|e| e.label == "Switch session…")
        .unwrap();
    assert_eq!(entries[switch].keys, [["Ctrl", "J"]]);
    assert_eq!(entries[switch].detail, spell(&entries[switch].keys));
    assert_eq!(
        element_text(&rt, &format!("entry:{switch}:keys")),
        ["Ctrl", "J"]
    );
    let tab = entries.iter().position(|e| e.label == "New tab").unwrap();
    assert_eq!(
        element_text(&rt, &format!("entry:{tab}:keys")),
        ["Ctrl", "B", "then", "C"],
        "a sequence reads in order"
    );
    press(&mut rt, Key::Escape);

    rebind(&mut rt, json!([{"keys":"ctrl+k","action":null}]));
    assert_eq!(
        help(&rt, "identity"),
        "Choose session · test-0 · Switch session"
    );
    assert!(
        element_text(&rt, "session-search-keys").is_empty(),
        "no hint for an unbound switcher"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_empty_workspace_names_the_switchers_key_only_while_it_has_one() {
    let mut ws = workspace();
    ws.hosts[0].model.connected = true;
    ws.hosts[0].model.sessions = vec![session("a")];
    let mut rt = runtime(ws);
    assert!(shown(
        &rt,
        "Select a session in the sidebar or press Ctrl K."
    ));
    rebind(&mut rt, json!([{"keys":"ctrl+k","action":null}]));
    assert!(shown(&rt, "Select a session in the sidebar."));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_hint_spells_chords_and_sequences_one_way() {
    let caps = |keys: &[&[&str]]| -> Vec<Vec<String>> {
        keys.iter()
            .map(|chord| chord.iter().map(|k| (*k).to_owned()).collect())
            .collect()
    };
    assert_eq!(spell(&[]), "");
    assert_eq!(spell(&caps(&[&["Ctrl", "Shift", "D"]])), "Ctrl Shift D");
    assert_eq!(spell(&caps(&[&["Ctrl", "B"], &["C"]])), "Ctrl B, C");

    let (mut ws, _rx) = tabbed_workspace("single");
    let null = serde_json::Value::Null;
    assert_eq!(ws.shortcut_chords("session.switch", &null), [["Ctrl", "K"]]);
    assert_eq!(
        ws.shortcut_chords("pane.split", &json!({"direction":"down"})),
        [["Ctrl", "Alt", "D"]]
    );
    assert!(
        ws.shortcut_chords("pane.split", &null).is_empty(),
        "arguments must match"
    );
    assert!(ws.shortcut_chords("no.such.action", &null).is_empty());
    assert_eq!(ws.shortcut_help("Zoom", "no.such.action"), "Zoom");
    ws.settings.shortcut_file.overrides =
        Overrides::parse(&json!([{"keys":"ctrl+b>c","action":"session.switch"}]));
    ws.local_keymap = actions::resolve(&ws.settings.shortcut_file.overrides);
    ws.keymap_generation += 1;
    assert_eq!(
        ws.shortcut_chords("session.switch", &null),
        [vec!["Ctrl", "B"], vec!["C"]],
        "an override is tried before the default"
    );
    std::fs::remove_file(&ws.settings_path).ok();
}
