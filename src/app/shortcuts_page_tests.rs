//! Settings → Shortcuts: a row for every action, alternates on one row, what a shared key
//! says, and the filter by name and by key.
use super::*;
use crate::app::{
    actions,
    panel::SettingsSection,
    settings_view::SHORTCUT_CATEGORIES,
    shortcuts_page::{Row, listing, rows},
};
use rex_linux::keymap::{Overrides, Resolved};
use rexrs::api::client::{Binding, InspectResponseAction};
use serde_json::{Value, json};

/// The keymap of this client alone, with `overrides` as its shortcuts.json.
fn local(overrides: Value) -> Resolved {
    actions::resolve(&Overrides::parse(&overrides))
}

fn default_rows() -> Vec<Row> {
    rows(&local(json!([])), &[], &[])
}

fn titled<'a>(rows: &'a [Row], title: &str) -> &'a Row {
    let found: Vec<_> = rows.iter().filter(|row| row.title == title).collect();
    assert_eq!(found.len(), 1, "one row titled {title:?}");
    found[0]
}

fn titles(rows: &[Row]) -> Vec<&str> {
    rows.iter().map(|row| row.title.as_str()).collect()
}

fn caps(keys: &[&[&str]]) -> Vec<Vec<String>> {
    keys.iter()
        .map(|chord| chord.iter().map(|cap| (*cap).to_owned()).collect())
        .collect()
}

#[test]
fn every_registered_action_has_a_row_bound_or_not() {
    let rows = default_rows();
    for spec in actions::SPECS {
        if crate::app::compatibility::alias(spec.name).is_some()
            || crate::app::compatibility::unsupported(spec.name).is_some()
        {
            continue;
        }
        let own: Vec<_> = rows.iter().filter(|row| row.action == spec.name).collect();
        assert!(!own.is_empty(), "{}: no row", spec.name);
        if spec.schema().is_none() {
            assert_eq!(own.len(), 1, "{}: one row", spec.name);
            assert_eq!(own[0].title, spec.title());
        }
        for row in own {
            assert_eq!(row.category, spec.category(), "{}", spec.name);
        }
    }
    // Nothing else is listed without a server, and no two rows are the same.
    assert!(rows.iter().all(|row| actions::find(&row.action).is_some()));
    let ids: std::collections::HashSet<String> = rows.iter().map(Row::id).collect();
    assert_eq!(ids.len(), rows.len());
    // An action with no default key is there, with no keys and nothing to explain.
    let close = titled(&rows, "Close tab");
    assert!(close.keys.is_empty() && close.notes.is_empty());
    assert!(
        rows.iter().filter(|row| row.keys.is_empty()).count() > 20,
        "the unbound actions are listed"
    );
}

#[test]
fn an_action_with_a_direction_has_a_row_a_direction() {
    let rows = default_rows();
    let split: Vec<_> = rows
        .iter()
        .filter(|row| row.action == "pane.split")
        .map(|row| (row.title.as_str(), row.keys.clone()))
        .collect();
    assert_eq!(
        split,
        [
            ("Split right", vec![caps(&[&["Ctrl", "Shift", "D"]])]),
            ("Split left", vec![]),
            ("Split down", vec![caps(&[&["Ctrl", "Alt", "D"]])]),
            ("Split up", vec![]),
            ("Split best fit", vec![]),
        ]
    );
    assert!(
        titled(&rows, "Resize pane · direction: left")
            .keys
            .is_empty()
    );
    assert!(
        titled(&rows, "Move pane · direction: right")
            .keys
            .is_empty()
    );
    // The four pane-focus defaults are four rows, each found by its own key.
    for direction in ["left", "right", "up", "down"] {
        let row = titled(&rows, &format!("Focus pane {direction}"));
        let mut key = direction.to_owned();
        key[..1].make_ascii_uppercase();
        assert_eq!(row.keys, [caps(&[&["Ctrl", "Alt", &key]])]);
        assert_eq!(row.id(), format!("pane.focus:{direction}"));
    }
    assert_eq!(
        titles(&rows)
            .iter()
            .filter(|title| title.starts_with("Move tab "))
            .count(),
        4
    );
}

#[test]
fn an_action_with_a_number_or_a_key_has_a_row_for_each_bound_value() {
    let rows = default_rows();
    // Bound by default with two values: two rows, by value.
    let scroll: Vec<_> = rows
        .iter()
        .filter(|row| row.action == "pane.scroll")
        .map(|row| row.title.as_str())
        .collect();
    assert_eq!(scroll, ["Scroll up · 20 lines", "Scroll down · 20 lines"]);
    // Bound with nothing: one row that says what it needs.
    let goto = titled(&rows, "Go to tab");
    assert!(goto.keys.is_empty());
    assert_eq!(
        goto.notes,
        ["Bound in shortcuts.json with an argument: index"]
    );
    assert_eq!(
        titled(&rows, "Send key").notes,
        ["Bound in shortcuts.json with an argument: key"]
    );

    let rows = self::rows(
        &local(json!([
            {"keys": "alt+2", "action": "client.tab.goto", "args": {"index": 2}},
            {"keys": "alt+1", "action": "client.tab.goto", "args": {"index": 1}},
            {"keys": "alt+9", "action": "client.tab.goto", "args": {"index": -1}},
        ])),
        &[],
        &[],
    );
    let goto: Vec<_> = rows
        .iter()
        .filter(|row| row.action == "client.tab.goto")
        .map(|row| (row.title.as_str(), row.keys.clone(), row.notes.clone()))
        .collect();
    let from_file = vec!["shortcuts.json".to_owned()];
    assert_eq!(
        goto,
        [
            (
                "Go to tab -1",
                vec![caps(&[&["Alt", "9"]])],
                from_file.clone()
            ),
            (
                "Go to tab 1",
                vec![caps(&[&["Alt", "1"]])],
                from_file.clone()
            ),
            ("Go to tab 2", vec![caps(&[&["Alt", "2"]])], from_file),
        ]
    );
}

#[test]
fn an_actions_bindings_share_one_row_in_the_order_keys_are_tried() {
    let rows = default_rows();
    let zoom = titled(&rows, "Increase interface zoom");
    assert_eq!(
        zoom.keys,
        [
            caps(&[&["Ctrl", "Plus"]]),
            caps(&[&["Ctrl", "Shift", "="]]),
            caps(&[&["Ctrl", "="]]),
        ],
        "the main block's keys, a literal plus spelled out"
    );
    assert_eq!(zoom.notes, ["Also on the numpad"]);
    assert_eq!(
        titled(&rows, "Decrease interface zoom").keys,
        [caps(&[&["Ctrl", "-"]])]
    );
    // The first is the one Commands and the menus show.
    let ws = workspace();
    assert_eq!(
        ws.shortcut_chords("client.zoom.in", &Value::Null),
        zoom.keys[0]
    );
    // The numpad's key still finds the row.
    let found = listing(rows, "ctrl+[NumpadAdd]");
    let (keys, on_key) = found.key.expect("a key");
    assert_eq!(keys, "Ctrl NumpadAdd");
    assert_eq!(titles(&on_key), ["Increase interface zoom"]);
    std::fs::remove_file(&ws.settings_path).ok();

    // A key from shortcuts.json joins the default's row and is named there.
    let rows = self::rows(
        &local(json!([{"keys": "ctrl+j", "action": "client.tab.new"}])),
        &[],
        &[],
    );
    let new_tab = titled(&rows, "New tab");
    assert_eq!(
        new_tab.keys,
        [caps(&[&["Ctrl", "J"]]), caps(&[&["Ctrl", "Shift", "T"]])]
    );
    assert_eq!(new_tab.notes, ["Ctrl J from shortcuts.json"]);
    // A sequence is one binding of two chords.
    let rows = self::rows(
        &local(json!([{"keys": "ctrl+b>c", "action": "client.tab.close"}])),
        &[],
        &[],
    );
    let close = titled(&rows, "Close tab");
    assert_eq!(close.keys, [caps(&[&["Ctrl", "B"], &["C"]])]);
    assert_eq!(close.notes, ["shortcuts.json"]);
}

#[test]
fn a_key_given_to_two_actions_says_so_on_both_rows() {
    let rows = self::rows(
        &local(json!([{"keys": "ctrl+k", "action": "client.tab.new"}])),
        &[],
        &[],
    );
    let new_tab = titled(&rows, "New tab");
    assert_eq!(
        new_tab.notes,
        [
            "Ctrl K from shortcuts.json",
            "Ctrl K is also bound to Switch session"
        ]
    );
    // The default it took never runs: the row has no key and says who has it.
    let switch = titled(&rows, "Switch session");
    assert!(switch.keys.is_empty());
    assert_eq!(switch.notes, ["Ctrl K is taken by New tab"]);

    // The key lists both, the one it performs first, under any spelling of it.
    for query in ["ctrl+k", " Control+K ", "ctrl + k"] {
        let found = listing(rows.clone(), query);
        let (keys, on_key) = found.key.expect("a key");
        assert_eq!(keys, "Ctrl K");
        assert_eq!(titles(&on_key), ["New tab", "Switch session"], "{query}");
        assert!(found.tie);
        assert!(found.rows.is_empty());
    }

    // A sequence takes its first chord from the key bound alone, which is not a shared key.
    let rows = self::rows(
        &local(json!([{"keys": "ctrl+k>t", "action": "client.tab.new"}])),
        &[],
        &[],
    );
    assert_eq!(
        titled(&rows, "New tab").notes,
        ["Ctrl K, T from shortcuts.json"]
    );
    assert_eq!(
        titled(&rows, "Switch session").notes,
        ["Ctrl K is taken by New tab"]
    );
    let found = listing(rows, "ctrl+k");
    assert_eq!(
        titles(&found.key.as_ref().unwrap().1),
        ["New tab", "Switch session"]
    );
    assert!(!found.tie, "one starts a sequence, one is the key alone");
}

/// The helper text's promise, for every key of a keymap with shared keys in it.
#[test]
fn the_first_result_of_a_key_is_the_action_the_key_performs() {
    let map = local(json!([
        {"keys": "ctrl+k", "action": "client.tab.new"},
        {"keys": "ctrl+shift+d", "action": "pane.zoom"},
        {"keys": "ctrl+shift+t", "action": "pane.split", "args": {"direction": "up"}},
        {"keys": "ctrl+b>c", "action": "client.tab.close"},
        {"keys": "ctrl+b>n", "action": "client.tab.next"},
    ]));
    let rows = rows(&map, &[], &[]);
    let effective = actions::effective(&map, &[]);
    assert!(effective.len() > 30);
    for binding in effective {
        let query = binding.binding.keys.join(">");
        let found = listing(rows.clone(), &query);
        let (_, on_key) = found.key.unwrap_or_else(|| panic!("{query} is a key"));
        let first = on_key.first().unwrap_or_else(|| panic!("{query}: no row"));
        assert_eq!(first.action, binding.binding.action, "{query}");
        assert_eq!(
            first.args.clone().unwrap_or(Value::Null),
            binding.binding.args.clone().unwrap_or(Value::Null),
            "{query}"
        );
    }
    // Every key that runs something is on a row, and no row shows one that does not run.
    let shown: usize = rows.iter().map(|row| row.keys.len()).sum();
    let folded = rows
        .iter()
        .filter(|row| row.notes.iter().any(|note| note == "Also on the numpad"))
        .count();
    assert_eq!(shown + folded, actions::effective(&map, &[]).len());
}

#[test]
fn the_filter_takes_a_key_or_a_name() {
    let rows = default_rows();
    // The roadmap's check: the key lists Split right.
    let found = listing(rows.clone(), "ctrl+shift+d");
    let (keys, on_key) = found.key.clone().expect("a key");
    assert_eq!(keys, "Ctrl Shift D");
    assert_eq!(titles(&on_key), ["Split right"]);
    assert!(!found.tie && found.rows.is_empty());

    // A key nothing is bound to is still a key, and says so.
    let found = listing(rows.clone(), "ctrl+shift+j");
    assert_eq!(found.key, Some(("Ctrl Shift J".into(), vec![])));
    assert!(found.is_empty());

    // A prefix of a chord is not a key: it is looked for in the keycaps.
    let found = listing(rows.clone(), "ctrl+shift");
    assert!(found.key.is_none());
    assert!(found.rows.len() > 10);
    assert!(
        found.rows.iter().all(|row| row
            .keys
            .iter()
            .flatten()
            .any(|chord| chord.starts_with(&["Ctrl".to_owned(), "Shift".to_owned()]))),
        "{:?}",
        titles(&found.rows)
    );

    // A name, in any case; rows keep the page's order.
    let found = listing(rows.clone(), "SPLIT");
    assert!(found.key.is_none());
    assert_eq!(
        titles(&found.rows),
        [
            "Split right",
            "Split left",
            "Split down",
            "Split up",
            "Split best fit",
            "Balance splits"
        ]
    );
    // `tab` is a key's name too, but nothing is bound to Tab alone: it finds names.
    let found = listing(rows.clone(), "tab");
    assert!(found.key.is_none());
    assert!(titles(&found.rows).contains(&"New tab"));
    assert!(titles(&found.rows).contains(&"Next tab across hosts"));
    // An action's name as shortcuts.json writes it.
    assert_eq!(
        titles(&listing(rows.clone(), "client.tab.close").rows),
        ["Close tab", "Close other tabs"]
    );
    // What is not set.
    let unset = listing(rows.clone(), "not set");
    assert!(unset.rows.len() > 20);
    assert!(unset.rows.iter().all(|row| row.keys.is_empty()));
    // Nothing.
    assert!(listing(rows.clone(), "zzzz").is_empty());
    // No query is every row.
    assert_eq!(listing(rows.clone(), "  ").rows, rows);
}

#[test]
fn the_hosts_actions_are_listed_under_server_bound_or_not() {
    let mut map = local(json!([]));
    // As a host's reply has them: its own bindings carry no source.
    map.bindings.insert(
        0,
        Binding::new(["ctrl+shift+l"], "layout_tall")
            .args(json!({}))
            .into(),
    );
    map.bindings
        .insert(1, Binding::new(["ctrl+shift+y"], "untitled").into());
    let action = |name: &str, title: Option<&str>| {
        let mut action = InspectResponseAction::new(name).kind("lua");
        action.title = title.map(str::to_owned);
        action
    };
    let server = [
        action("layout_tall", Some("Tall layout")),
        action("layout_wide", Some("Wide layout")),
        action("untitled", None),
    ];
    let offered = [
        ("layout_tall".to_owned(), "Tall layout".to_owned()),
        ("layout_wide".to_owned(), "Wide layout".to_owned()),
    ];
    let rows = rows(&map, &server, &offered);
    let listed: Vec<_> = rows
        .iter()
        .filter(|row| row.category == "Server")
        .map(|row| (row.title.as_str(), row.keys.clone(), row.notes.clone()))
        .collect();
    assert_eq!(
        listed,
        [
            (
                "Tall layout",
                vec![caps(&[&["Ctrl", "Shift", "L"]])],
                vec!["Server".to_owned()]
            ),
            ("Wide layout", vec![], vec![]),
            (
                "untitled",
                vec![caps(&[&["Ctrl", "Shift", "Y"]])],
                vec!["Server".to_owned()]
            ),
        ]
    );
    // An action the server does not offer cannot run: its binding is not a row.
    let mut map = local(json!([]));
    map.bindings
        .insert(0, Binding::new(["ctrl+shift+l"], "gone").into());
    assert!(
        self::rows(&map, &[], &[])
            .iter()
            .all(|row| row.category != "Server")
    );
}

/// `overrides` as the workspace's shortcuts.json, without a file. Settings reads the file
/// again when it opens, so this comes after.
fn set_overrides(ws: &mut Workspace, overrides: Value) {
    ws.settings.shortcut_file.overrides = Overrides::parse(&overrides);
    ws.local_keymap = actions::resolve(&ws.settings.shortcut_file.overrides);
    ws.keymap_generation += 1;
}

#[test]
fn a_default_key_taken_away_in_shortcuts_json_is_named_on_its_row() {
    let mut ws = workspace();
    set_overrides(
        &mut ws,
        json!([
            {"keys": "ctrl+k", "action": null},
            {"keys": "ctrl+shift+d", "action": null},
            {"keys": "ctrl+shift+t", "action": null},
            {"keys": "ctrl+shift+t", "action": "client.tab.new"},
        ]),
    );
    let rows = ws.shortcut_rows();
    let switch = titled(&rows, "Switch session");
    assert!(switch.keys.is_empty());
    assert_eq!(switch.notes, ["Ctrl K removed in shortcuts.json"]);
    assert_eq!(
        titled(&rows, "Split right").notes,
        ["Ctrl Shift D removed in shortcuts.json"]
    );
    // Taken away and given back: the row has the key, so nothing was lost.
    let new_tab = titled(&rows, "New tab");
    assert_eq!(new_tab.keys, [caps(&[&["Ctrl", "Shift", "T"]])]);
    assert_eq!(new_tab.notes, ["shortcuts.json"]);
    std::fs::remove_file(&ws.settings_path).ok();
}

/// Settings open on Shortcuts in a window large enough for the whole dialog, at rest.
fn shortcuts_open(ws: Workspace) -> Runtime<Workspace> {
    let mut ws = ws;
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    ws.open_section(SettingsSection::Shortcuts);
    let mut rt = runtime_at(ws, (1440., 1000.));
    settle(&mut rt);
    rt
}

fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..90 {
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

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1.
}

/// The drawn text `words`, and where.
fn text_rects(rt: &Runtime<Workspace>, words: &str) -> Vec<[f32; 4]> {
    rt.elements()
        .into_iter()
        .filter(|e| !e.exiting && e.text.as_deref() == Some(words))
        .map(|e| e.rect)
        .collect()
}

/// The keys of the action rows on the page, in order.
fn row_keys(rt: &Runtime<Workspace>) -> Vec<String> {
    rt.elements()
        .into_iter()
        .filter(|e| !e.exiting)
        .filter_map(|e| e.key)
        .filter(|key| key.starts_with("shortcut:"))
        .collect()
}

fn type_in(rt: &mut Runtime<Workspace>, words: &str) {
    for c in words.chars() {
        press(rt, Key::Text(c.to_string()));
    }
    settle(rt);
}

#[test]
fn the_page_draws_a_row_for_every_action_with_its_keys_or_not_set() {
    let rt = shortcuts_open(workspace_hosts(2));
    let expected: Vec<String> = rt
        .state
        .shortcut_rows()
        .iter()
        .map(|row| format!("shortcut:{}", row.id()))
        .collect();
    assert!(expected.len() > 60);
    let mut drawn = row_keys(&rt);
    let mut sorted = expected.clone();
    drawn.sort();
    sorted.sort();
    assert_eq!(drawn, sorted, "one row an action, each drawn once");

    for row in rt.state.shortcut_rows() {
        let frame = rect(&rt, &format!("shortcut:{}", row.id()));
        let keys = rect(&rt, &format!("shortcut-keys:{}", row.id()));
        assert!(
            close(keys[0] + keys[2], frame[0] + frame[2] - 12.),
            "{}: its keys end at the card's inset: {keys:?} in {frame:?}",
            row.title
        );
        assert!(
            keys[0] > frame[0] + frame[2] / 2.,
            "{}: keys on the right",
            row.title
        );
        assert!(
            keys[1] >= frame[1] && keys[1] + keys[3] <= frame[1] + frame[3],
            "{}: keys inside the row: {keys:?} in {frame:?}",
            row.title
        );
    }
    // A name starts at the inset and ends before its keys.
    let frame = rect(&rt, "shortcut:client.tab.next");
    let title = text_rects(&rt, "Next tab")[0];
    let keys = rect(&rt, "shortcut-keys:client.tab.next");
    assert!(close(title[0], frame[0] + 12.) && title[0] + title[2] < keys[0]);
    // The keys are the keycaps menus and Commands draw: 17 pt chips on a 7% white fill.
    let chip = crate::app::theme::white(crate::app::theme::KEYCAP_FILL);
    let caps = rt
        .elements()
        .iter()
        .filter(|e| !e.exiting && e.fill == chip && close(e.rect[3], 17.))
        .count();
    let bound: usize = rt
        .state
        .shortcut_rows()
        .iter()
        .flat_map(|row| row.keys.iter().flatten())
        .map(Vec::len)
        .sum();
    assert_eq!(caps, bound, "a chip a key");
    // Valid unbound actions offer recording; references stay read-only. Pointer hints remain words.
    let unbound = rt
        .state
        .shortcut_rows()
        .iter()
        .filter(|row| row.keys.is_empty() && !row.recordable)
        .count();
    assert_eq!(text_rects(&rt, "Not set").len(), unbound);
    assert_eq!(
        text_rects(&rt, "Record shortcut").len(),
        rt.state
            .shortcut_rows()
            .iter()
            .filter(|row| row.keys.is_empty() && row.recordable)
            .count()
    );
    assert_eq!(text_rects(&rt, "Mouse Back").len(), 1);
    assert_eq!(text_rects(&rt, "Mouse Forward").len(), 1);
    // Named as Commands and the menus name them, each once.
    for title in [
        "Split right",
        "Split down",
        "Move tab left",
        "Move tab right",
        "Close pane or tab",
        "Focus pane left",
    ] {
        assert_eq!(text_rects(&rt, title).len(), 1, "{title}");
    }
    assert_eq!(text_rects(&rt, "Also on the numpad").len(), 3);

    // Alternates are stacked on one row, which grows for them; every line ends on the inset.
    let zoom = rect(&rt, "shortcut:client.zoom.in");
    let plain = rect(&rt, "shortcut:client.settings");
    assert!(zoom[3] > plain[3] + 30., "{zoom:?} against {plain:?}");
    assert!(close(plain[3], 40.), "a plain row: {plain:?}");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_filter_stays_above_the_rows_and_narrows_them_by_key() {
    let mut rt = shortcuts_open(workspace_hosts(2));
    let field = rect(&rt, "shortcut-filter");
    let scroll = rect(&rt, "settings-scroll:shortcuts");
    assert!(
        field[1] + field[3] <= scroll[1]
            && close(field[0] + field[2] + 8. + 32., scroll[0] + scroll[2] - 20.),
        "the filter is above the scroll region, inset as the page is: {field:?} over {scroll:?}"
    );
    // Scrolled, the rows move and the filter does not.
    rt.wheel(
        scroll[0] + scroll[2] / 2.,
        scroll[1] + scroll[3] / 2.,
        -900.,
    );
    settle(&mut rt);
    assert_eq!(rect(&rt, "shortcut-filter"), field);
    assert!(
        rect(&rt, "shortcut:client.tab.next")[1] < scroll[1],
        "the first row went under it"
    );

    click(&mut rt, "shortcut-filter");
    type_in(&mut rt, "ctrl+shift+d");
    assert_eq!(rt.state.shortcut_filter.text, "ctrl+shift+d");
    assert_eq!(row_keys(&rt), ["shortcut:pane.split:right"]);
    assert_eq!(text_rects(&rt, "Bound to Ctrl Shift D").len(), 1);
    // The results are at the top of the page again, under the filter.
    let row = rect(&rt, "shortcut:pane.split:right");
    assert!(
        row[1] > scroll[1] && row[1] < scroll[1] + 80.,
        "{row:?} in {scroll:?}"
    );
    // The page's own explanations make way for results.
    assert!(!has(&rt, "shortcuts-help") && !has(&rt, "shortcuts-path"));

    // Escape empties the filter and keeps Settings; a second Escape closes Settings.
    press(&mut rt, Key::Escape);
    settle(&mut rt);
    assert_eq!(rt.state.shortcut_filter.text, "");
    assert!(rt.state.panel.is_some());
    assert!(row_keys(&rt).len() > 60);
    assert!(has(&rt, "shortcuts-help") && has(&rt, "shortcuts-path"));
    press(&mut rt, Key::Escape);
    settle(&mut rt);
    assert!(rt.state.panel.is_none());
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_filter_says_when_nothing_matches_and_starts_empty_each_time_settings_opens() {
    let mut rt = shortcuts_open(workspace_hosts(2));
    click(&mut rt, "shortcut-filter");
    type_in(&mut rt, "zzzz");
    assert!(row_keys(&rt).is_empty());
    assert_eq!(text_rects(&rt, "No action or key matches “zzzz”.").len(), 1);
    // The clear button empties it.
    let clear = rt
        .elements()
        .into_iter()
        .find(|e| !e.exiting && e.role == "button" && e.label.as_deref() == Some("Clear"))
        .expect("a clear button while there is a query")
        .rect;
    rt.pointer_down(clear[0] + clear[2] / 2., clear[1] + clear[3] / 2.);
    rt.pointer_up(clear[0] + clear[2] / 2., clear[1] + clear[3] / 2.);
    settle(&mut rt);
    assert_eq!(rt.state.shortcut_filter.text, "");
    assert!(row_keys(&rt).len() > 60);

    // A key nothing has.
    click(&mut rt, "shortcut-filter");
    type_in(&mut rt, "ctrl+shift+j");
    assert!(row_keys(&rt).is_empty());
    assert_eq!(
        text_rects(&rt, "Nothing is bound to Ctrl Shift J.").len(),
        1
    );
    // Closed and opened again, Settings does not keep the query.
    rt.update(|s, tx| {
        s.close_panel(tx);
        s.perform(Intent::Settings, tx);
    });
    settle(&mut rt);
    assert_eq!(rt.state.settings_section, SettingsSection::Shortcuts);
    assert_eq!(rt.state.shortcut_filter.text, "");
    assert!(row_keys(&rt).len() > 60);
    std::fs::remove_file(&rt.state.settings_path).ok();
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
            resolved: Resolved::from_server(&reply, &ws.settings.shortcut_file.overrides),
            reply,
        },
    ));
    ws.hosts[0].model.keymap_revision += 1;
}

#[test]
fn a_connected_hosts_actions_are_rows_under_server() {
    let (mut ws, _host) = tabbed_workspace("split");
    let mut bindings: Vec<Binding> = rex_linux::keymap::defaults()
        .bindings
        .into_iter()
        .map(|mut binding| {
            binding.source = Some("linux".into());
            binding
        })
        .collect();
    bindings.insert(0, Binding::new(["ctrl+shift+l"], "layout_tall"));
    let mut actions = actions::declarations();
    for (name, title) in [
        ("layout_tall", "Tall layout"),
        ("layout_wide", "Wide layout"),
    ] {
        actions.push(InspectResponseAction::new(name).kind("lua").title(title));
    }
    server_map(&mut ws, bindings, actions);
    let rows = ws.shortcut_rows();
    let server: Vec<_> = rows
        .iter()
        .filter(|row| row.category == "Server")
        .map(|row| (row.title.as_str(), row.keys.len()))
        .collect();
    assert_eq!(server, [("Tall layout", 1), ("Wide layout", 0)]);

    let rt = shortcuts_open(ws);
    assert_eq!(
        rt.elements()
            .iter()
            .filter(|e| !e.exiting
                && e.path.contains("settings-scroll:shortcuts")
                && e.text.as_deref() == Some("Server"))
            .count(),
        2,
        "the page heading and a source (not the Server navigation row)"
    );
    assert!(has(&rt, "shortcut:layout_tall") && has(&rt, "shortcut:layout_wide"));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn find_only_bindings_do_not_hide_the_search_opener_or_its_command_hint() {
    let map = local(json!([
        {"keys":"ctrl+shift+f", "action":"client.find.next", "args":{"when":"searching"}}
    ]));
    let all = rows(&map, &[], &[]);
    let opener = titled(&all, "Search terminal");
    assert_eq!(opener.keys, [caps(&[&["Ctrl", "Shift", "F"]])]);
    assert!(!opener.notes.iter().any(|n| n.contains("taken by")));
    let next = all.iter().find(|r| r.action == "client.find.next").unwrap();
    assert_eq!(next.keys, opener.keys);
    assert!(!next.notes.iter().any(|n| n.contains("Also bound to")));
    let effective = actions::effective(&map, &[]);
    assert!(
        effective
            .iter()
            .any(|b| b.binding.action == actions::SEARCH)
    );
    assert!(
        effective
            .iter()
            .any(|b| b.binding.action == "client.find.next")
    );
    let mut ws = workspace();
    ws.local_keymap = map;
    assert_eq!(
        ws.shortcut_chords(actions::SEARCH, &Value::Null),
        caps(&[&["Ctrl", "Shift", "F"]])
    );
}

#[test]
fn only_the_complete_not_set_phrase_matches_unbound_status() {
    let all = default_rows();
    assert!(!listing(all.clone(), "not set").rows.is_empty());
    for query in ["set", "ot", "se"] {
        let found = listing(all.clone(), query);
        assert!(
            !found.rows.iter().any(|r| r.action == "pane.balance"),
            "{query}"
        );
    }
}

#[test]
fn modes_only_conflict_within_their_own_scope_and_remain_searchable() {
    let reply = rexrs::api::client::KeymapResponse {
        bindings: vec![
            Binding::new(["ctrl+k"], "pane.balance").mode("resize"),
            Binding::new(["ctrl+k"], "pane.close").mode("resize"),
            Binding::new(["ctrl+k"], "pane.reset").mode("inspect"),
            Binding::new(["ctrl+k"], "session.switch"),
            Binding::new(["ctrl+b", "c"], "pane.close").mode("resize"),
            Binding::new(["ctrl+b"], "pane.reset"),
            Binding::new(["ctrl++"], "client.zoom.in").mode("resize"),
            Binding::new(["ctrl+[NumpadAdd]"], "client.zoom.in"),
        ],
        ..Default::default()
    };
    let map = Resolved::from_server(&reply, &Overrides::default());
    let all = rows(&map, &[], &[]);
    let balance = titled(&all, "Balance splits");
    assert!(balance.notes.iter().any(|n| n == "Ctrl K in mode resize"));
    assert!(
        balance
            .notes
            .iter()
            .any(|n| n == "Also bound to Close pane")
    );
    let close = titled(&all, "Close pane");
    assert!(
        close
            .notes
            .iter()
            .any(|n| n == "Ctrl K is taken by Balance splits")
    );
    for name in ["Reset terminal", "Switch session"] {
        let row = titled(&all, name);
        assert!(!row.keys.is_empty());
        assert!(
            !row.notes
                .iter()
                .any(|n| n.contains("taken by") || n.contains("also bound"))
        );
    }
    let zoom = titled(&all, "Increase interface zoom");
    assert_eq!(zoom.keys.len(), 2, "numpad folding must not cross modes");
    let independent = vec![
        titled(&all, "Reset terminal").clone(),
        titled(&all, "Switch session").clone(),
    ];
    assert!(
        !listing(independent, "ctrl+k").tie,
        "different modes are not a conflict"
    );
    let filtered = listing(all.clone(), "ctrl+k");
    assert!(filtered.tie, "two actions share this key in resize mode");
    assert_eq!(filtered.key.unwrap().1.len(), 4);
    let prefix = listing(all.clone(), "ctrl+b");
    assert!(!prefix.tie);
    assert_eq!(prefix.key.unwrap().1.len(), 2);
    assert!(listing(all, "mode resize").rows.len() >= 3);
}

#[test]
fn bound_aliases_and_unsupported_bindings_keep_their_identity_and_diagnostics() {
    let overrides = json!([
        {"keys":"ctrl+j","action":"pane.split.right"},
        {"keys":"ctrl+up","action":"pane.move_to_new_tab"},
        {"keys":"alt+j","action":"client.find.next","args":{"when":"searching"}}
    ]);
    let map = local(overrides.clone());
    let all = rows(&map, &[], &[]);
    let alias = all.iter().find(|r| r.action == "pane.split.right").unwrap();
    assert_eq!(alias.notes, ["shortcuts.json", "Alias: pane.split.right"]);
    assert!(!alias.keys.is_empty());
    assert_eq!(
        all.iter()
            .filter(|r| r.action == "client.find.next")
            .count(),
        1,
        "only the bound alias, not a duplicate unbound row"
    );
    assert_eq!(
        listing(all.clone(), "ctrl+j").key.unwrap().1[0].action,
        "pane.split.right"
    );
    assert!(!listing(all.clone(), "pane.split.right").rows.is_empty());
    let unsupported = all
        .iter()
        .find(|r| r.action == "pane.move_to_new_tab")
        .unwrap();
    assert!(
        unsupported
            .notes
            .iter()
            .any(|n| n.contains("recognized but not supported"))
    );
    assert_eq!(map.diagnostics.len(), 1);
    let conditional = all.iter().find(|r| r.action == "client.find.next").unwrap();
    assert!(
        conditional
            .notes
            .iter()
            .any(|n| n.contains("while the find bar is open"))
    );

    let mut rt = shortcuts_open(workspace());
    rt.update(|s, _| set_overrides(s, overrides));
    settle(&mut rt);
    assert!(has(&rt, "shortcut-problem-0"));
    assert!(has(&rt, "shortcut:pane.split.right"));
    let problem = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("shortcut-problem-0"))
        .unwrap();
    assert!(
        problem
            .text
            .unwrap()
            .contains("recognized but not supported")
    );
}

#[test]
fn the_filter_is_keyboard_reachable_without_stealing_section_navigation() {
    let mut rt = shortcuts_open(workspace());
    click(&mut rt, "settings-section:shortcuts");
    press(&mut rt, Key::Tab); // Close
    assert!(focused(&rt).unwrap().ends_with("close-settings"));
    press(&mut rt, Key::Tab); // Filter
    assert!(focused(&rt).unwrap().contains("shortcut-filter"));
    type_in(&mut rt, "ctrl+shift+d");
    assert_eq!(row_keys(&rt), ["shortcut:pane.split:right"]);
    press(&mut rt, Key::Tab); // Clear
    assert!(rt.state.panel.is_some());
    // Section navigation still works after editing. Coming back keeps the query.
    click(&mut rt, "settings-section:terminal");
    assert!(!has(&rt, "shortcut-filter"));
    click(&mut rt, "settings-section:shortcuts");
    assert_eq!(rt.state.shortcut_filter.text, "ctrl+shift+d");
    click(&mut rt, "shortcut-filter");
    press(&mut rt, Key::Escape);
    assert_eq!(rt.state.shortcut_filter.text, "");
    press(&mut rt, Key::Escape);
    assert!(rt.state.panel.is_none());
    assert_eq!(focused(&rt), None, "the next key belongs to the pane");
}

#[test]
fn normal_and_minimum_windows_keep_keycaps_inside_growing_rows() {
    for size in [(1024., 700.), (640., 480.)] {
        let mut ws = workspace();
        ws.perform(Intent::Settings, &mut rune::Tx::default());
        ws.open_section(SettingsSection::Shortcuts);
        let mut rt = runtime_at(ws, size);
        settle(&mut rt);
        let scroll = rect(&rt, "settings-scroll:shortcuts");
        let filter = rect(&rt, "shortcut-filter");
        assert!(filter[1] + filter[3] <= scroll[1]);
        for row in rt.state.shortcut_rows() {
            let frame = rect(&rt, &format!("shortcut:{}", row.id()));
            let keys = rect(&rt, &format!("shortcut-keys:{}", row.id()));
            assert!(keys[0] >= frame[0] && keys[0] + keys[2] <= frame[0] + frame[2]);
            assert!(keys[1] >= frame[1] && keys[1] + keys[3] <= frame[1] + frame[3]);
        }
    }
}

/// The page as drawn, for a look (see `Camera`): its top, its end, a key with two actions,
/// a name, a key nothing has, and a narrow window. Nothing is asserted.
#[test]
fn the_shortcuts_page_draws() {
    use super::pane_verb_tests::{Camera, framed_terminal};
    let Some(mut camera) = Camera::new() else {
        return;
    };
    let scene = |size: (f32, f32), overrides: Value| {
        let (mut ws, _host) = tabbed_workspace("split");
        let (sender, _sent) = flume::unbounded();
        framed_terminal(&mut ws, "block:b", &sender, b"$ cargo test");
        ws.perform(Intent::Settings, &mut rune::Tx::default());
        ws.open_section(SettingsSection::Shortcuts);
        // After Settings opened: opening it reads shortcuts.json again, and there is none.
        set_overrides(&mut ws, overrides);
        let mut rt = Runtime::new(ws, view, size, Env::new(true, 2.));
        rt.on_key = Some(crate::app::input::on_key);
        settle(&mut rt);
        rt
    };
    let shared = json!([
        {"keys": "ctrl+k", "action": "client.tab.new"},
        {"keys": "ctrl+b>c", "action": "client.tab.close"},
        {"keys": "alt+1", "action": "client.tab.goto", "args": {"index": 1}},
    ]);
    let wheel = |rt: &mut Runtime<Workspace>, by: f32| {
        let scroll = rect(rt, "settings-scroll:shortcuts");
        rt.wheel(scroll[0] + scroll[2] / 2., scroll[1] + scroll[3] / 2., by);
        settle(rt);
    };
    let done = |rt: Runtime<Workspace>| {
        std::fs::remove_file(&rt.state.settings_path).ok();
    };

    let mut rt = scene((1024., 700.), json!([]));
    camera.snap(&mut rt, "shortcuts-top");
    wheel(&mut rt, -1500.);
    camera.snap(&mut rt, "shortcuts-middle");
    wheel(&mut rt, -750.);
    camera.snap(&mut rt, "shortcuts-interface");
    wheel(&mut rt, -9000.);
    camera.snap(&mut rt, "shortcuts-end");
    done(rt);

    let mut rt = scene((1024., 700.), shared.clone());
    camera.snap(&mut rt, "shortcuts-overrides");
    click(&mut rt, "shortcut-filter");
    type_in(&mut rt, "ctrl+k");
    camera.snap(&mut rt, "shortcuts-filter-key");
    done(rt);

    let mut rt = scene((1024., 700.), shared);
    click(&mut rt, "shortcut-filter");
    type_in(&mut rt, "tab");
    camera.snap(&mut rt, "shortcuts-filter-name");
    done(rt);

    let mut rt = scene((1024., 700.), json!([]));
    click(&mut rt, "shortcut-filter");
    type_in(&mut rt, "ctrl+shift+j");
    camera.snap(&mut rt, "shortcuts-filter-free-key");
    done(rt);

    let mut rt = scene((640., 480.), json!([]));
    camera.snap(&mut rt, "shortcuts-narrow");
    click(&mut rt, "shortcut-filter");
    type_in(&mut rt, "zoom");
    camera.snap(&mut rt, "shortcuts-narrow-alternates");
    done(rt);
}

#[test]
fn the_helper_names_the_layers_and_the_filter() {
    let rt = shortcuts_open(workspace_hosts(2));
    let help = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("shortcuts-help"))
        .and_then(|e| e.text)
        .expect("the helper");
    for words in ["Record shortcut", "ctrl+k", "Filter"] {
        assert!(help.contains(words), "{words}: {help}");
    }
    // It is the first thing on the page, above the first heading.
    let top = rect(&rt, "shortcuts-help");
    let heading = text_rects(&rt, SHORTCUT_CATEGORIES[0])[0];
    assert!(top[1] + top[3] < heading[1]);
    std::fs::remove_file(&rt.state.settings_path).ok();
}
