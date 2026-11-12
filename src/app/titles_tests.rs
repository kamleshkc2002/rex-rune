//! Generated names are display-only and agree across every chrome surface.
use super::*;
use crate::app::panel::SettingsSection;
use rex_rune::connection::{activity::TERMINAL, titles::Place};

#[test]
fn saving_an_untouched_tab_rename_keeps_the_server_label_not_the_generated_name() {
    for label in [Some("Shell"), Some("work"), None] {
        let (mut ws, rx) = tabbed_workspace("split");
        ws.settings.generated_titles = true;
        let model = &mut ws.hosts[0].model;
        model.views.get_mut("s").unwrap().windows[1].label = label.map(str::to_owned);
        model.programs.insert("block:b".into(), "vim".into());
        model.places.insert(
            "block:b".into(),
            Place {
                cwd: Some("/home/u/rex".into()),
                home: Some("/home/u".into()),
                shell: Some(1),
            },
        );
        let display = ws.active_tab().unwrap().name;
        assert_eq!(
            display,
            if label == Some("work") {
                "work"
            } else {
                "vim - ~/rex"
            }
        );
        let mut rt = runtime(ws);
        rt.update(|s, tx| s.perform(Intent::RenameWindow("split".into()), tx));
        for _ in 0..4 {
            rt.frame(1. / 60.);
        }
        assert_eq!(
            rt.state.panel.as_ref().unwrap().inputs[0].text,
            label.unwrap_or_default()
        );
        assert!(requests(&rx).is_empty());
        click(&mut rt, "save-form");
        let calls = requests(&rx);
        if label.is_none() {
            assert!(calls.is_empty(), "an empty rename is not submitted");
            continue;
        }
        assert_eq!(calls.len(), 1, "{label:?}");
        assert_eq!(calls[0].0, "session.set_window_label");
        assert_eq!(calls[0].2["label"], label.unwrap_or_default());
        assert_ne!(calls[0].2["label"], "vim - ~/rex");
    }
}

#[test]
fn generated_names_agree_and_the_setting_saves_without_server_requests() {
    let (mut ws, rx) = tabbed_workspace("split");
    let model = &mut ws.hosts[0].model;
    let tab = &mut model.views.get_mut("s").unwrap().windows[1];
    tab.label = Some("Shell".into());
    for block in &mut tab.layers[0].blocks {
        block.label = Some("shell".into());
        block.creator_name = Some(TERMINAL.into());
        block.flavor = Some(format!("{TERMINAL}.shell"));
    }
    model.programs.insert("block:b".into(), "vim".into());
    model.places.insert(
        "block:b".into(),
        Place {
            cwd: Some("/home/u/rex".into()),
            home: Some("/home/u".into()),
            shell: Some(1),
        },
    );
    model
        .titles
        .insert("block:b".into(), ("shell title".into(), Some(1)));
    model.foreground.insert("block:b".into(), vec![7, 1]);
    let original = model.views.clone();
    let mut rt = runtime_at(ws, (1440., 1000.));
    rt.update(|s, tx| s.prefer_generated_titles(true, tx));
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
    let name = "vim - ~/rex";
    assert!(element_text(&rt, "tab:split").iter().any(|t| t == name));
    assert!(
        element_text(&rt, "pane-header:block:b")
            .iter()
            .any(|t| t == name)
    );
    assert!(rt.state.title.starts_with(name));
    assert_eq!(rt.state.active_tab().unwrap().name, name);
    assert_eq!(rt.state.pane_name("block:b"), name);
    rt.update(|s, _| {
        s.settings.vertical_tabs = true;
        s.sidebar.toggle();
    });
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
    assert!(
        element_text(&rt, "sidebar-tab:0:s:split")
            .iter()
            .any(|t| t == name)
    );
    rt.update(|s, _| s.pane_menu("block:b".into(), None));
    assert_eq!(
        rt.state
            .panel
            .as_ref()
            .unwrap()
            .target
            .as_ref()
            .unwrap()
            .name,
        name
    );
    assert_eq!(
        rt.state
            .panel
            .as_ref()
            .unwrap()
            .target
            .as_ref()
            .unwrap()
            .tab,
        name
    );
    assert_eq!(
        rt.state.panel.as_ref().unwrap().title,
        format!("{name} · {name}")
    );
    assert!(relaunch(&rt.state).generated_titles());

    rt.update(|s, tx| {
        s.close_panel(tx);
        s.perform(Intent::Settings, tx);
        s.open_section(SettingsSection::Terminal);
        tx.scroll_to_instant("settings-scroll:terminal", 0., 1000.);
    });
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
    assert!(has(&rt, "generated-titles"));
    click(&mut rt, "generated-titles");
    assert!(!rt.state.generated_titles());
    assert!(!relaunch(&rt.state).generated_titles());
    assert!(rt.state.title.starts_with("shell title"));
    assert_eq!(rt.state.pane_name("block:b"), "shell title");
    assert_eq!(
        rt.state.hosts[0].model.views, original,
        "no label/layout mutations"
    );
    assert!(
        requests(&rx).is_empty(),
        "a preference sends no server requests"
    );
    std::fs::remove_file(&rt.state.settings_path).unwrap();
}
