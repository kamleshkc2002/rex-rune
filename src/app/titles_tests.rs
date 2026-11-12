//! Generated names are display-only and agree across every chrome surface.
use super::*;
use crate::app::panel::SettingsSection;
use rex_linux::connection::{activity::TERMINAL, titles::Place};

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
