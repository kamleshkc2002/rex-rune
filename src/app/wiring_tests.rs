//! What every action owes the rest of the app, whichever part of it added the action: a row
//! in Commands (or a stated reason for none), a place and a name on Settings → Shortcuts once
//! it is bound, a decision about remote callers, and its keys shown from the effective keymap.
use super::*;
use crate::app::{
    actions::{self, Remote},
    commands, keycaps,
    panel::SettingsSection,
    settings_view::SHORTCUT_CATEGORIES,
};
use rex_linux::keymap::Overrides;
use serde_json::{Value, json};

/// Arguments for the actions that cannot run without them. An action that starts taking
/// arguments needs a line here.
fn sample_args(name: &str) -> Value {
    match name {
        "session.select" => json!({"session_id": "s"}),
        "pane.split" | "client.tab.move" => json!({"direction": "right"}),
        "pane.move" | "pane.resize" => json!({"direction": "right"}),
        "client.mode.enter" => json!({"name":"resize"}),
        "pane.focus" => json!({"direction": "up"}),
        "pane.scroll" => json!({"lines": 3}),
        "client.tab.goto" => json!({"index": 2}),
        "pane.send_key" => json!({"key": "ctrl+c"}),
        _ => Value::Null,
    }
}

/// A two-chord sequence no default uses, different for each `index`.
fn chord(index: usize) -> String {
    const SECOND: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    format!(
        "ctrl+alt+shift+f{}>{}",
        9 + index / SECOND.len(),
        SECOND[index % SECOND.len()] as char
    )
}

/// The split tab of `tabbed_workspace` with a live pane focused, and every registered action
/// bound in shortcuts.json to its own sequence.
fn all_bound() -> Workspace {
    let (mut ws, _host) = tabbed_workspace("split");
    let (sender, _sent) = flume::unbounded();
    super::pane_verb_tests::framed_terminal(&mut ws, "block:b", &sender, b"$ ");
    let bindings: Vec<Value> = actions::SPECS
        .iter()
        .enumerate()
        .map(|(index, spec)| {
            let args = sample_args(spec.name);
            assert!(
                spec.accepts(&args),
                "{}: give it arguments in sample_args",
                spec.name
            );
            let mut binding = json!({"keys": chord(index), "action": spec.name});
            if !args.is_null() {
                binding["args"] = args;
            }
            binding
        })
        .collect();
    ws.settings.shortcut_file.overrides = Overrides::parse(&Value::Array(bindings));
    ws.local_keymap = actions::resolve(&ws.settings.shortcut_file.overrides);
    ws.keymap_generation += 1;
    assert!(
        ws.effective_keymap().diagnostics.iter().all(|d| d
            .action
            .as_deref()
            .is_some_and(|a| crate::app::compatibility::unsupported(a).is_some())),
        "{:?}",
        ws.effective_keymap().diagnostics
    );
    ws
}

#[test]
fn every_action_has_a_title_a_shortcuts_heading_and_a_reason_when_it_is_refused_remotely() {
    for spec in actions::SPECS {
        assert!(!spec.title().is_empty(), "{}", spec.name);
        assert!(
            SHORTCUT_CATEGORIES.contains(&spec.category()),
            "{}: Settings → Shortcuts has no {} heading",
            spec.name,
            spec.category()
        );
        if let Remote::Never(reason) = spec.remote() {
            assert!(!reason.is_empty(), "{}", spec.name);
        }
    }
}

#[test]
fn a_bound_action_is_listed_on_settings_shortcuts_by_its_title() {
    let mut ws = all_bound();
    let overrides = ws.settings.shortcut_file.overrides.clone();
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    ws.open_section(SettingsSection::Shortcuts);
    // Opening Settings reloads the absent file. Restore this test's in-memory map.
    ws.settings.shortcut_file.overrides = overrides;
    ws.local_keymap = actions::resolve(&ws.settings.shortcut_file.overrides);
    ws.keymap_generation += 1;
    let mut rt = runtime_at(ws, (1440., 1000.));
    for _ in 0..30 {
        rt.frame(1. / 60.);
    }
    let words: std::collections::HashSet<String> =
        rt.elements().into_iter().filter_map(|e| e.text).collect();
    for heading in SHORTCUT_CATEGORIES {
        // "Server" lists the host's own actions; this workspace has no keymap from a server.
        assert_eq!(
            words.contains(heading),
            heading != "Server",
            "the {heading} heading"
        );
    }
    assert_eq!(
        rt.state
            .settings
            .shortcut_file
            .overrides
            .layer
            .bindings
            .len(),
        actions::SPECS.len()
    );
    for binding in &rt.state.settings.shortcut_file.overrides.layer.bindings {
        let row = rt
            .state
            .shortcut_rows()
            .into_iter()
            .find(|r| r.action == binding.action && r.args == binding.args)
            .expect("bound action row");
        assert!(has(&rt, &format!("shortcut:{}", row.id())));
        assert!(
            !row.keys.is_empty(),
            "{}: its keys are visible",
            binding.action
        );
        let title = row.title;
        assert!(
            words.contains(&title),
            "{}: no row titled {title:?}",
            binding.action
        );
    }
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn every_commands_row_of_a_bound_action_shows_its_keys() {
    let mut ws = all_bound();
    ws.picker(false);
    let mut rows = 0;
    for (index, spec) in actions::SPECS.iter().enumerate() {
        let exempt = commands::NOT_LISTED
            .iter()
            .any(|(name, _)| *name == spec.name);
        let args = sample_args(spec.name);
        let keys = ws.shortcut_chords(spec.name, &args);
        assert_eq!(
            keycaps::spell(&keys).to_lowercase().replace(' ', "+"),
            chord(index).replace('>', ",+"),
            "{}: the effective keymap has its binding",
            spec.name
        );
        let row = ws.entries().into_iter().find(|entry| {
            entry
                .action
                .shortcut()
                .is_some_and(|(name, row_args)| name == spec.name && row_args == args)
        });
        match row {
            Some(row) => {
                assert_eq!(row.keys, keys, "{}: {}", spec.name, row.label);
                assert_eq!(row.detail, keycaps::spell(&keys));
                rows += 1;
            }
            None => assert!(exempt, "{}: no Commands row carries it", spec.name),
        }
    }
    assert_eq!(
        rows,
        actions::SPECS.len() - commands::NOT_LISTED.len(),
        "every action Commands lists carried its keys"
    );
    std::fs::remove_file(&ws.settings_path).ok();
}

/// The surfaces that only exist once every part is in: the tab menu with its destructive
/// group, the sidebar's tab menu, Commands at its Pane and App groups and as a search, the
/// Pane headers list over Settings, Shortcuts with every action bound, and About. Drawn for
/// a look (see `Camera`); nothing is asserted.
#[test]
fn the_merged_surfaces_draw() {
    use super::pane_verb_tests::{Camera, framed_terminal};
    let Some(mut camera) = Camera::new() else {
        return;
    };
    let settle = |rt: &mut Runtime<Workspace>| {
        for _ in 0..90 {
            rt.frame(1. / 60.);
        }
    };
    let scene = |prepare: &dyn Fn(&mut Workspace)| {
        let (mut ws, _host) = tabbed_workspace("split");
        let (sender, _sent) = flume::unbounded();
        framed_terminal(
            &mut ws,
            "block:b",
            &sender,
            b"$ cargo test\r\nrunning 12 tests\r\ntest result: ok",
        );
        framed_terminal(&mut ws, "block:c", &sender, b"$ tail -f server.log");
        prepare(&mut ws);
        let mut rt = Runtime::new(ws, view, (1024., 700.), Env::new(true, 2.));
        rt.on_key = Some(crate::app::input::on_key);
        for _ in 0..90 {
            rt.frame(1. / 60.);
        }
        rt
    };
    let select = |rt: &mut Runtime<Workspace>, label: &str| {
        let index = rt
            .state
            .entries()
            .iter()
            .position(|e| e.label == label)
            .unwrap_or_else(|| panic!("no {label}"));
        rt.update(|s, tx| {
            s.panel.as_mut().unwrap().selected = index;
            tx.scroll_into_view(format!("entry:{index}"));
        });
    };
    let done = |rt: Runtime<Workspace>| {
        std::fs::remove_file(&rt.state.settings_path).ok();
    };

    let mut rt = scene(&|_| {});
    secondary_click(&mut rt, "tab:split");
    settle(&mut rt);
    camera.snap(&mut rt, "merged-tab-menu");
    select(&mut rt, "Close tab…");
    settle(&mut rt);
    camera.snap(&mut rt, "merged-tab-menu-close");
    done(rt);

    let mut rt = scene(&|ws| {
        ws.sidebar.toggle();
        ws.settings.vertical_tabs = true;
    });
    secondary_click(&mut rt, "sidebar-tab:0:s:single");
    settle(&mut rt);
    camera.snap(&mut rt, "merged-sidebar-tab-menu");
    done(rt);

    let mut rt = scene(&|_| {});
    rt.update(|s, _| s.picker(false));
    settle(&mut rt);
    select(&mut rt, "Close pane…");
    settle(&mut rt);
    camera.snap(&mut rt, "merged-commands-pane");
    select(&mut rt, "Close app");
    settle(&mut rt);
    camera.snap(&mut rt, "merged-commands-app");
    for c in "close".chars() {
        press(&mut rt, Key::Text(c.to_string()));
    }
    settle(&mut rt);
    camera.snap(&mut rt, "merged-commands-search");
    done(rt);

    let mut rt = scene(&|_| {});
    rt.update(|s, tx| s.perform(Intent::Settings, tx));
    settle(&mut rt);
    click(&mut rt, "pane-header");
    settle(&mut rt);
    camera.snap(&mut rt, "merged-settings-pane-headers");
    done(rt);

    let mut rt = scene(&|_| {});
    rt.update(|s, tx| {
        s.perform(Intent::Settings, tx);
        s.open_section(SettingsSection::Shortcuts);
    });
    settle(&mut rt);
    camera.snap(&mut rt, "merged-settings-shortcuts");
    done(rt);

    let mut rt = scene(&|_| {});
    rt.update(|s, tx| {
        actions::run(s, crate::app::about::ABOUT, &Value::Null, tx);
    });
    settle(&mut rt);
    camera.snap(&mut rt, "merged-settings-about");
    done(rt);
}
