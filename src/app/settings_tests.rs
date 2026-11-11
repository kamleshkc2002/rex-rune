//! The Settings shell and the building blocks its pages share (`settings_kit`).
use super::*;
use crate::app::panel::SettingsSection;
use rex_linux::settings::NoticePosition;

/// Settings open on `section`, in a window large enough for the whole dialog, at rest.
fn settings_at(section: SettingsSection, scale: f32, size: (f32, f32)) -> Runtime<Workspace> {
    let mut ws = workspace_hosts(2);
    ws.settings.ui_scale = scale;
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    ws.open_section(section);
    let mut rt = runtime_at(ws, size);
    settle(&mut rt);
    rt
}

fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
}

/// Where the element keyed `key` is drawn, `[x, y, w, h]`.
fn rect(rt: &Runtime<Workspace>, key: &str) -> [f32; 4] {
    rt.elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("missing {key}"))
        .rect
}

fn kind(rt: &Runtime<Workspace>) -> Option<&PanelKind> {
    rt.state.panel.as_ref().map(|p| &p.kind)
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1.
}

#[test]
fn settings_is_816_wide_and_shrinks_with_a_small_window() {
    let rt = settings_at(SettingsSection::Interface, 1., (1440., 1000.));
    let dialog = rect(&rt, "dialog");
    assert!(
        close(dialog[2], 816.) && close(dialog[3], 680.),
        "{dialog:?}"
    );

    // A small window, and a larger one at a zoom that leaves as little room: the dialog
    // takes what is under the toolbar, 12 pt clear of it and of the window's bottom edge.
    for (scale, size) in [(1., (640., 480.)), (1.3, (1280., 800.))] {
        let rt = settings_at(SettingsSection::Interface, scale, size);
        let (dialog, toolbar) = (rect(&rt, "dialog"), rect(&rt, "toolbar"));
        // Rects are in window pixels; the dialog is laid out in points of the zoom.
        let (width, height) = (size.0 / scale, size.1 / scale);
        assert!(
            close(dialog[2], (width * 0.92).min(816.) * scale)
                && close(dialog[3], (height - 52. - 24.) * scale),
            "{dialog:?} at {scale}"
        );
        assert!(
            close(dialog[1], toolbar[1] + toolbar[3] + 12. * scale),
            "{dialog:?} starts under {toolbar:?} at {scale}"
        );
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

/// Every element of `role` under the element at `index`.
fn inside(elements: &[rune::ElementInfo], index: usize, role: &str) -> usize {
    let mut within = vec![index];
    let mut count = 0;
    for (i, e) in elements.iter().enumerate().skip(index + 1) {
        if e.parent.is_some_and(|p| within.contains(&p)) {
            within.push(i);
            count += usize::from(e.role == role);
        }
    }
    count
}

#[test]
fn every_page_is_cards_with_a_rule_between_each_pair_of_rows_and_one_trailing_edge() {
    for section in SettingsSection::ALL {
        for scale in [1., 1.3] {
            let mut rt = settings_at(section, scale, (1440., 1000.));
            if section == SettingsSection::Integrations {
                rt.update(|s, _| {
                    s.apply_agent_status(Ok(SKILL_STATUS.into()));
                });
                settle(&mut rt);
            }
            let elements = rt.elements();
            let name = section.key();
            let scroll = rect(&rt, &format!("settings-scroll:{name}"));
            let headings = elements.iter().filter(|e| e.role == "heading").count();
            let rules = elements.iter().filter(|e| e.role == "separator").count();
            assert!(headings + rules > 0, "{name} is built from sections");
            // A rule is always between two rows of one card: never first or last in it, and
            // inset from the card's edges by the same amount on both sides.
            for (i, rule) in elements.iter().enumerate() {
                if rule.role != "separator" {
                    continue;
                }
                let line = rule.parent.expect("a rule's row");
                let card = elements[line].parent.expect("a rule's card");
                let siblings: Vec<usize> = (0..elements.len())
                    .filter(|&j| elements[j].parent == Some(card))
                    .collect();
                let at = siblings.iter().position(|&j| j == line).unwrap();
                assert!(
                    at > 0 && at + 1 < siblings.len(),
                    "{name}: rule {i} at an end"
                );
                assert_ne!(elements[siblings[at - 1]].role, "separator", "{name}");
                assert_eq!(inside(&elements, siblings[at - 1], "separator"), 0);
                assert_eq!(siblings.len() % 2, 1, "{name}: rows and rules alternate");
                let (card, rule) = (elements[card].rect, rule.rect);
                let (left, right) = (rule[0] - card[0], card[0] + card[2] - rule[0] - rule[2]);
                assert!(
                    close(left, right) && left > 4.,
                    "{name}: {left} and {right}"
                );
            }
            // Controls end on one x: in each row, the last switch, stepper, popup button,
            // segmented control or button, and the actions under the agents' card. Remote
            // Hosts' action under its cards starts at their edge, and the copy buttons of
            // About's header are centred with it.
            let edge = |e: &rune::ElementInfo| e.rect[0] + e.rect[2];
            let mut controls: Vec<&rune::ElementInfo> = elements
                .iter()
                .filter(|e| {
                    !e.exiting
                        && e.path.contains("settings-scroll:")
                        && matches!(e.role, "switch" | "combobox" | "radiogroup" | "button")
                        && e.rect[2] < 300.
                        && !e.key.as_deref().is_some_and(|k| {
                            matches!(k, "settings-add-host" | "about-copy" | "about-copy-debug")
                        })
                })
                .collect();
            controls.sort_by(|a, b| a.rect[1].total_cmp(&b.rect[1]));
            let mut trailing: Vec<f32> = Vec::new();
            let mut row_bottom = f32::MIN;
            for control in controls {
                let middle = control.rect[1] + control.rect[3] / 2.;
                match trailing.last_mut() {
                    Some(last) if middle < row_bottom => *last = last.max(edge(control)),
                    _ => trailing.push(edge(control)),
                }
                row_bottom = row_bottom.max(control.rect[1] + control.rect[3]);
            }
            for x in &trailing {
                assert!(
                    close(*x, trailing[0]),
                    "{name} at {scale}: controls end at {trailing:?}"
                );
            }
            // Nothing a row holds is cut by the page's sides.
            for e in elements.iter().filter(|e| {
                !e.exiting
                    && e.path.contains("settings-scroll:")
                    && matches!(
                        e.role,
                        "switch" | "combobox" | "radiogroup" | "button" | "text"
                    )
            }) {
                assert!(
                    e.rect[0] >= scroll[0] - 0.5 && edge(e) <= scroll[0] + scroll[2] + 0.5,
                    "{name} at {scale}: {:?} {:?} leaves the page {scroll:?}",
                    e.key.as_deref().or(e.text.as_deref()),
                    e.rect
                );
            }
            std::fs::remove_file(&rt.state.settings_path).ok();
        }
    }
}

#[test]
fn a_picker_opens_over_settings_against_its_control_and_gives_focus_back() {
    for scale in [1., 1.3] {
        let mut rt = settings_at(SettingsSection::Terminal, scale, (1440., 1000.));
        let scroll_before = rect(&rt, "settings-scroll:terminal");
        click(&mut rt, "terminal-theme");
        settle(&mut rt);
        assert!(matches!(kind(&rt), Some(PanelKind::Themes)));
        // Settings stays where it was under the list, and takes no input.
        assert_eq!(rect(&rt, "settings-scroll:terminal"), scroll_before);
        let dialog = rt
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some("dialog"))
            .unwrap();
        assert!(dialog.inert, "the page under the list is inert");
        let (control, list) = (rect(&rt, "terminal-theme"), rect(&rt, "menu-surface"));
        let gap = 4. * scale;
        assert!(
            close(list[0] + list[2], control[0] + control[2]),
            "at {scale}: trailing edges {list:?} {control:?}"
        );
        let below = close(list[1], control[1] + control[3] + gap);
        let above = close(list[1] + list[3], control[1] - gap);
        assert!(
            below || above,
            "at {scale}: the list {list:?} abuts its control {control:?}"
        );
        assert!(
            list[1] >= 0. && list[1] + list[3] <= 1000.,
            "at {scale}: the list is on screen"
        );
        // Its search field has the keyboard.
        press(&mut rt, Key::Text("z".into()));
        assert_eq!(rt.state.panel.as_ref().unwrap().inputs[0].text, "z");
        press(&mut rt, Key::Escape);
        settle(&mut rt);
        assert!(matches!(kind(&rt), Some(PanelKind::Settings)));
        assert!(
            focused(&rt).is_some_and(|path| path.ends_with("terminal-theme")),
            "focus returns to the control, not {:?}",
            focused(&rt)
        );
        // Return opens it again from the keyboard; a press outside goes back to Settings.
        press(&mut rt, Key::Enter);
        settle(&mut rt);
        assert!(matches!(kind(&rt), Some(PanelKind::Themes)));
        let dialog = rect(&rt, "dialog");
        rt.pointer_down(dialog[0] + 40., dialog[1] + dialog[3] - 40.);
        rt.pointer_up(dialog[0] + 40., dialog[1] + dialog[3] - 40.);
        settle(&mut rt);
        assert!(
            matches!(kind(&rt), Some(PanelKind::Settings)),
            "a press outside the list returns to Settings instead of closing it"
        );
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn a_select_row_lists_its_values_and_applies_the_chosen_one() {
    let mut rt = settings_at(SettingsSection::Interface, 1., (1440., 1000.));
    click(&mut rt, "notice-position");
    settle(&mut rt);
    let Some(PanelKind::Choice(choice)) = kind(&rt) else {
        panic!("a choice list, not {:?}", kind(&rt));
    };
    assert_eq!(choice.options, ["Top right", "Bottom right"]);
    let rows: Vec<_> = rt
        .state
        .entries()
        .into_iter()
        .map(|e| (e.label, e.current))
        .collect();
    assert_eq!(
        rows,
        [
            ("Top right".to_owned(), true),
            ("Bottom right".to_owned(), false)
        ]
    );
    let (control, list) = (rect(&rt, "notice-position"), rect(&rt, "menu-surface"));
    assert!(close(list[0] + list[2], control[0] + control[2]));
    assert!(close(list[1], control[1] + control[3] + 4.), "{list:?}");
    assert!(
        !rt.elements()
            .iter()
            .any(|e| e.text.as_deref() == Some("Position") && e.path.contains("menu-surface")),
        "the row names the list; it has no title of its own"
    );

    // The keyboard walks the list although the control it came from was focused.
    press(&mut rt, Key::Down);
    press(&mut rt, Key::Enter);
    settle(&mut rt);
    assert!(matches!(kind(&rt), Some(PanelKind::Settings)));
    assert_eq!(
        rt.state.settings.notice_position,
        NoticePosition::BottomRight
    );
    assert_eq!(
        Settings::load(&rt.state.settings_path)
            .unwrap()
            .notice_position,
        NoticePosition::BottomRight
    );
    assert!(
        rt.elements()
            .iter()
            .any(|e| e.key.as_deref() == Some("notice-position")
                && e.label.as_deref() == Some("Bottom right")),
        "the control shows the new value"
    );
    assert!(focused(&rt).is_some_and(|path| path.ends_with("notice-position")));

    // Escape leaves the value alone.
    press(&mut rt, Key::Enter);
    settle(&mut rt);
    assert!(matches!(kind(&rt), Some(PanelKind::Choice(_))));
    press(&mut rt, Key::Up);
    press(&mut rt, Key::Escape);
    settle(&mut rt);
    assert!(matches!(kind(&rt), Some(PanelKind::Settings)));
    assert_eq!(
        rt.state.settings.notice_position,
        NoticePosition::BottomRight
    );

    // A pointer choice.
    click(&mut rt, "notice-position");
    settle(&mut rt);
    let top = rt
        .elements()
        .into_iter()
        .find(|e| e.role == "menuitem" && e.label.as_deref() == Some("Top right"))
        .expect("a Top right row")
        .rect;
    rt.pointer_down(top[0] + top[2] / 2., top[1] + top[3] / 2.);
    rt.pointer_up(top[0] + top[2] / 2., top[1] + top[3] / 2.);
    settle(&mut rt);
    assert_eq!(rt.state.settings.notice_position, NoticePosition::TopRight);
    assert!(matches!(kind(&rt), Some(PanelKind::Settings)));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_choice_without_settings_behind_it_closes_and_ignores_a_stale_index() {
    fn apply(s: &mut Workspace, index: usize, _: &mut rune::Tx) {
        s.settings.pane_padding = index as f32;
    }
    let mut ws = workspace();
    let choice = || crate::app::panel::Choice {
        options: vec!["a".into(), "b".into()],
        current: Some(1),
        apply,
    };
    ws.open_choice("nowhere", "Letters", choice());
    assert_eq!(
        ws.panel.as_ref().unwrap().selected,
        1,
        "starts on the current"
    );
    ws.perform(Intent::Choose(7), &mut rune::Tx::default());
    assert!(ws.panel.is_none());
    assert_eq!(
        ws.settings.pane_padding, 8.,
        "an index out of range is dropped"
    );
    ws.open_choice("nowhere", "Letters", choice());
    ws.perform(Intent::Choose(0), &mut rune::Tx::default());
    assert!(ws.panel.is_none());
    assert_eq!(ws.settings.pane_padding, 0.);
    // Choosing with no list open does nothing.
    ws.perform(Intent::Choose(1), &mut rune::Tx::default());
    assert_eq!(ws.settings.pane_padding, 0.);
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn disclosure_rows_and_segments_work_from_the_keyboard() {
    let mut rt = settings_at(SettingsSection::Terminal, 1., (1440., 1000.));
    press(&mut rt, Key::Tab); // Close
    press(&mut rt, Key::Tab); // Font family
    assert!(focused(&rt).unwrap().ends_with("font-change"));
    press(&mut rt, Key::Enter);
    assert!(matches!(kind(&rt), Some(PanelKind::Form(Form::Fonts))));
    press(&mut rt, Key::Escape);
    assert!(matches!(kind(&rt), Some(PanelKind::Settings)));

    // A host's default shell is a row that opens its form, showing what is set.
    assert!(
        element_text(&rt, "host-shell:0").contains(&"Server default".to_owned()),
        "{:?}",
        element_text(&rt, "host-shell:0")
    );
    click(&mut rt, "host-shell:1");
    assert!(matches!(
        kind(&rt),
        Some(PanelKind::Form(Form::HostShell(1)))
    ));
    press(&mut rt, Key::Escape);

    // Segments: Tab reaches the selected one, arrows move, Space picks.
    click(&mut rt, "settings-section:interface");
    settle(&mut rt);
    let density = rect(&rt, "pane-density");
    let tabs = rect(&rt, "tab-position");
    assert!(
        close(density[0], tabs[0]) && close(density[2], tabs[2]),
        "segmented controls share both edges: {density:?} {tabs:?}"
    );
    assert!(!rt.state.settings.vertical_tabs);
    let sidebar = rt
        .elements()
        .into_iter()
        .find(|e| e.role == "radio" && e.label.as_deref() == Some("Sidebar"))
        .expect("the Sidebar segment")
        .rect;
    rt.pointer_down(sidebar[0] + 4., sidebar[1] + 4.);
    rt.pointer_up(sidebar[0] + 4., sidebar[1] + 4.);
    settle(&mut rt);
    assert!(rt.state.settings.vertical_tabs, "the whole segment presses");
    press(&mut rt, Key::Left);
    press(&mut rt, Key::Text(" ".into()));
    assert!(
        !rt.state.settings.vertical_tabs,
        "Left then Space picks Top"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_long_page_scrolls_with_a_thumb_and_a_short_one_has_none() {
    let rt = settings_at(SettingsSection::Shortcuts, 1., (1440., 1000.));
    let [x, y, w, h] = rt.state.settings_fx.thumb.get();
    let scroll = rect(&rt, "settings-scroll:shortcuts");
    assert!(w > 0. && h >= 24. && h < scroll[3], "a thumb: {w} x {h}");
    assert!(x + w <= scroll[2] && y >= 0., "inside the page: {x}, {y}");

    let rt = settings_at(SettingsSection::About, 1., (1440., 1000.));
    assert_eq!(
        rt.state.settings_fx.thumb.get(),
        [0.; 4],
        "nothing to scroll"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

/// The drawn text `words`, and where.
fn text_rects(rt: &Runtime<Workspace>, words: &str) -> Vec<[f32; 4]> {
    rt.elements()
        .into_iter()
        .filter(|e| !e.exiting && e.text.as_deref() == Some(words))
        .map(|e| e.rect)
        .collect()
}

#[test]
fn a_long_row_label_is_cut_short_before_its_value() {
    let label = "unix:///home/someone/.cache/a-rather-long-directory/run/rex/server.sock";
    assert!(label.len() >= 60);
    let mut ws = workspace_hosts(2);
    ws.settings.hosts[1].label = label.into();
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    ws.open_section(SettingsSection::Terminal);
    let mut rt = runtime_at(ws, (1280., 800.));
    settle(&mut rt);

    let row = rect(&rt, "host-shell:1");
    let name = text_rects(&rt, label);
    assert_eq!(name.len(), 1, "the label is drawn once");
    let name = name[0];
    // The row's value: the "Server default" on this row, not the one on the row above.
    let value = text_rects(&rt, "Server default")
        .into_iter()
        .find(|v| v[1] >= row[1] && v[1] + v[3] <= row[1] + row[3])
        .expect("the row's value");
    assert!(
        name[0] + name[2] <= value[0] - 8.,
        "the label ends before the value: {name:?} then {value:?}"
    );
    assert!(
        name[0] >= row[0] && value[0] + value[2] <= row[0] + row[2],
        "both are inside the row: {name:?} {value:?} in {row:?}"
    );
    // A short label takes only its own width and the value keeps the same trailing edge.
    let short = rect(&rt, "host-shell:0");
    let other = text_rects(&rt, "Server default")
        .into_iter()
        .find(|v| v[1] >= short[1] && v[1] + v[3] <= short[1] + short[3])
        .expect("the first row's value");
    assert!(close(other[0] + other[2], value[0] + value[2]));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_shortcut_row_names_its_action_on_the_left_and_shows_keycaps_on_the_right() {
    let rt = settings_at(SettingsSection::Shortcuts, 1., (1440., 1000.));
    let elements = rt.elements();
    let rows: Vec<_> = elements
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            !e.exiting
                && e.key
                    .as_deref()
                    .is_some_and(|k| k.starts_with("shortcut-") && !k.contains("-keys-"))
        })
        .collect();
    assert!(rows.len() > 20, "the default bindings: {}", rows.len());
    for (_, row) in &rows {
        let key = row.key.as_deref().unwrap();
        let keys = rect(&rt, &key.replacen("shortcut-", "shortcut-keys-", 1));
        assert!(
            close(keys[0] + keys[2], row.rect[0] + row.rect[2] - 12.),
            "{key}: keys end at the card's inset: {keys:?} in {:?}",
            row.rect
        );
        assert!(
            keys[0] > row.rect[0] + row.rect[2] / 2.,
            "{key}: keys are on the right"
        );
    }
    // The first row: its title starts at the inset and ends before its keys.
    let (_, first) = rows[0];
    let title = text_rects(&rt, "Next tab")[0];
    let keys = rect(&rt, "shortcut-keys-Navigate-0");
    assert!(close(title[0], first.rect[0] + 12.) && title[0] + title[2] < keys[0]);
    // The keys are the keycaps menus and Commands draw: 17 pt chips with a hairline, on a
    // 7% white fill, never the kit's tinted Keybind.
    let chip = crate::app::theme::white(crate::app::theme::KEYCAP_FILL);
    let caps: Vec<_> = elements
        .iter()
        .filter(|e| !e.exiting && e.fill == chip && close(e.rect[3], 17.))
        .collect();
    assert!(caps.len() >= rows.len(), "a chip a key: {}", caps.len());
    // Named as Commands and the menus name them.
    for title in [
        "Split right",
        "Split down",
        "Move tab left",
        "Move tab right",
        "Close pane or tab",
        "Focus adjacent pane",
    ] {
        assert_eq!(text_rects(&rt, title).len(), 1, "{title}");
    }
    for old in [
        "Split pane · down",
        "Move tab · left",
        "Close focused pane or tab",
    ] {
        assert!(text_rects(&rt, old).is_empty(), "{old}");
    }
    // The numpad twins fold into the row's one helper line, and the pointer's row is words.
    assert_eq!(text_rects(&rt, "Also on the numpad").len(), 3);
    assert_eq!(text_rects(&rt, "Mouse Back / Forward").len(), 1);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

/// `scripts/remote-control-acceptance.py` clicks Remote Hosts in Settings by window
/// coordinates, under a real pointer in a 1440 by 900 window. Nothing else runs that script
/// here, so this holds its click to the row it means.
#[test]
fn the_acceptance_scripts_settings_click_lands_on_remote_hosts() {
    let script = include_str!("../../scripts/remote-control-acceptance.py");
    let line = script
        .lines()
        .find(|l| l.contains(r#"desk.x("mousemove", "--window", window, "#))
        .expect("the script's Settings click");
    let numbers: Vec<f32> = line
        .trim_end_matches(')')
        .rsplit(", ")
        .take(2)
        .map(|n| n.trim().parse().expect("a coordinate"))
        .collect();
    let (x, y) = (numbers[1], numbers[0]);
    let rt = settings_at(SettingsSection::Interface, 1., (1440., 900.));
    let row = rect(&rt, "settings-section:hosts");
    assert!(
        x > row[0] + 8. && x < row[0] + row[2] - 8. && y > row[1] + 4. && y < row[1] + row[3] - 4.,
        "({x}, {y}) is well inside Remote Hosts at {row:?}"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}
