//! The Add host list's rows and groups, the Integrations page's chips and footnotes, and the
//! toolbar's Settings glyph.
use super::*;
use crate::app::{
    integrations::{self, Harness, Item},
    panel::SettingsSection,
    settings_kit::ChipTone,
    theme,
};
use rex_linux::tailnet::{Machine, Probe};

fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
}

fn machine(name: &str, os: &str, online: bool) -> Machine {
    Machine {
        name: name.into(),
        address: format!("{name}.tail1234.ts.net"),
        host_name: None,
        os: os.into(),
        online,
        ssh: false,
        this_machine: false,
    }
}

/// The index of the element keyed `key`, and the indices of everything inside it.
fn subtree(elements: &[rune::ElementInfo], key: &str) -> Vec<usize> {
    let root = elements
        .iter()
        .position(|e| !e.exiting && e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("missing {key}"));
    let mut inside = vec![root];
    for (i, e) in elements.iter().enumerate().skip(root + 1) {
        if e.parent.is_some_and(|p| inside.contains(&p)) {
            inside.push(i);
        }
    }
    inside
}

/// The text element reading `words` inside the element keyed `key`.
fn text_in<'a>(elements: &'a [rune::ElementInfo], key: &str, words: &str) -> &'a rune::ElementInfo {
    subtree(elements, key)
        .into_iter()
        .map(|i| &elements[i])
        .find(|e| e.text.as_deref() == Some(words))
        .unwrap_or_else(|| panic!("no {words:?} in {key}"))
}

const LONG_NAME: &str = "build-farm-runner-eu-central-1a-ci-00042";

/// The Add host list over a tailnet with a 40-character name whose check has much to say, a
/// machine that is already added, one whose check failed and one that is away.
fn add_host_list() -> Runtime<Workspace> {
    let mut ws = workspace_hosts(2);
    ws.sidebar.toggle();
    ws.settings.hosts[1].label = "the-box".into();
    ws.settings.hosts[1].endpoint = "ssh://me@box.tail1234.ts.net".into();
    let mut rt = runtime_at(ws, (1024., 900.));
    click(&mut rt, "add-host");
    let generation = rt.state.discovery.generation;
    rt.update(|s, _| {
        s.apply_scan(
            generation,
            Ok(vec![
                Machine {
                    host_name: Some("ip-10-42-17-203".into()),
                    ..machine(LONG_NAME, "linux", true)
                },
                machine("box", "macOS", true),
                machine("pixel", "android", true),
                machine("away", "linux", false),
            ]),
        );
        s.apply_probe(
            generation,
            &format!("{LONG_NAME}.tail1234.ts.net"),
            Probe::Rex {
                endpoint: format!("https://{LONG_NAME}.tail1234.ts.net:7297"),
                listeners: vec![],
                reason: None,
            },
        );
        s.apply_probe(generation, "box.tail1234.ts.net", Probe::Nothing);
        s.apply_probe(
            generation,
            "pixel.tail1234.ts.net",
            Probe::Unknown("https 443: timed out".into()),
        );
    });
    settle(&mut rt);
    rt
}

#[test]
fn add_host_keeps_a_long_name_whole_cuts_its_status_and_keeps_the_chevron() {
    assert_eq!(LONG_NAME.chars().count(), 40);
    let rt = add_host_list();
    let entries = rt.state.entries();
    let names: Vec<&str> = entries.iter().map(|e| e.label.as_str()).collect();
    assert_eq!(
        names,
        [LONG_NAME, "box", "pixel", "away", "Enter an endpoint…"]
    );
    assert_eq!(
        entries[0].detail,
        format!(
            "https://{LONG_NAME}.tail1234.ts.net:7297 · listener details unavailable · Linux · ip-10-42-17-203"
        ),
        "the row's words are whole, whatever the row has room to draw"
    );

    let elements = rt.elements();
    let surface = elements
        .iter()
        .find(|e| e.key.as_deref() == Some("menu-surface"))
        .unwrap()
        .rect;
    let right = |e: &rune::ElementInfo| e.rect[0] + e.rect[2];
    // The name is whole: a cut name would carry its full text as a tooltip.
    let name = text_in(&elements, "entry:0", LONG_NAME);
    assert_eq!(name.help, None, "the 40-character name is not cut");
    assert!(name.rect[2] > 200., "{:?}", name.rect);
    // What the check found gives way instead, and says so with its whole text.
    let status = elements
        .iter()
        .find(|e| e.key.as_deref() == Some("machine-status:0"))
        .expect("the long row's status");
    assert_eq!(
        status.help.as_deref(),
        Some(
            format!("https://{LONG_NAME}.tail1234.ts.net:7297 · listener details unavailable")
                .as_str()
        ),
        "the status is cut with an ellipsis"
    );
    assert!(
        status.rect[2] >= 40. && status.rect[0] >= right(name),
        "the cut status keeps room to read and starts after the name: {:?}",
        status.rect
    );
    // Every row keeps its chevron, inside the list.
    for row in 0..4 {
        let key = format!("entry:{row}");
        let inside = subtree(&elements, &key);
        let chevron = inside
            .iter()
            .map(|&i| &elements[i])
            .filter(|e| e.role == "icon")
            .max_by(|a, b| a.rect[0].total_cmp(&b.rect[0]))
            .unwrap_or_else(|| panic!("{key} has no chevron"));
        assert!(
            chevron.rect[2] > 8. && right(chevron) <= surface[0] + surface[2] - 8.,
            "{key}: {:?} in {surface:?}",
            chevron.rect
        );
        for &i in &inside {
            assert!(
                right(&elements[i]) <= surface[0] + surface[2] + 0.5,
                "{key}: {:?} {:?} leaves the list {surface:?}",
                elements[i].text,
                elements[i].rect
            );
        }
        if row == 0 {
            assert!(chevron.rect[0] >= right(status), "after the status");
        }
    }
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn add_host_rows_line_up_in_columns_with_a_chip_for_added_machines() {
    let rt = add_host_list();
    let elements = rt.elements();
    // Systems share one column, whatever the status before them says.
    let systems: Vec<f32> = [
        ("entry:0", "Linux"),
        ("entry:1", "macOS"),
        ("entry:2", "Android"),
        ("entry:3", "Linux"),
    ]
    .into_iter()
    .map(|(key, os)| text_in(&elements, key, os).rect[0])
    .collect();
    assert!(
        systems.iter().all(|x| (x - systems[0]).abs() < 0.5),
        "{systems:?}"
    );
    // Names share one too: an added machine has a chip at the end, not a check before it.
    let names: Vec<f32> = [
        ("entry:0", LONG_NAME),
        ("entry:1", "box"),
        ("entry:2", "pixel"),
        ("entry:3", "away"),
    ]
    .into_iter()
    .map(|(key, name)| text_in(&elements, key, name).rect[0])
    .collect();
    assert!(
        names.iter().all(|x| (x - names[0]).abs() < 0.5),
        "{names:?}"
    );
    let chip = elements
        .iter()
        .find(|e| e.key.as_deref() == Some("machine-added:1"))
        .expect("the added machine's chip");
    assert_eq!(chip.help.as_deref(), Some("Added as the-box"));
    assert_eq!(text_in(&elements, "machine-added:1", "Added").role, "text");
    assert!(
        chip.rect[0] > text_in(&elements, "entry:1", "macOS").rect[0],
        "the chip trails the row"
    );
    assert!(
        !elements.iter().any(|e| e
            .key
            .as_deref()
            .is_some_and(|k| k.starts_with("machine-added:") && k != "machine-added:1")),
        "only the added machine has one"
    );
    // A group that says a machine is offline doesn't repeat it on every row.
    assert!(
        !subtree(&elements, "entry:3")
            .into_iter()
            .any(|i| elements[i].text.as_deref() == Some("Offline")),
    );
    assert_eq!(rt.state.entries()[3].detail, "Offline · Linux");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn add_host_groups_have_icons_and_a_failed_check_is_trouble() {
    let rt = add_host_list();
    let entries = rt.state.entries();
    let groups: Vec<(&str, Option<&str>)> = entries
        .iter()
        .map(|e| (e.group.as_str(), e.group_icon.map(|i| i.name)))
        .collect();
    assert_eq!(
        groups,
        [
            ("Rex servers", Some("rex-server")),
            ("Tailnet", Some("rex-network")),
            ("Tailnet", Some("rex-network")),
            ("Offline", Some("rex-moon")),
            ("", None),
        ]
    );
    let words = |index: usize| {
        rt.state
            .machine_words(&rt.state.discovery.candidates[index])
    };
    assert_eq!(
        words(2).status,
        [
            ("Rex check failed".to_owned(), true),
            ("SSH".to_owned(), false)
        ],
        "said in WARNING, with the word"
    );
    assert_eq!(words(1).status, [("SSH".to_owned(), false)]);
    assert_eq!(words(1).added, ["the-box"]);
    assert!(words(0).status.iter().all(|(_, trouble)| !trouble));
    // A server that answered but refused the hello leads with the reason, so a short row
    // cuts the address and not the trouble.
    let mut refused = rt.state.discovery.candidates[0].clone();
    refused.probe = Some(Probe::Rex {
        endpoint: "https://x.tail1234.ts.net".into(),
        listeners: vec![],
        reason: Some("hello refused".into()),
    });
    assert_eq!(
        rt.state.machine_words(&refused).status,
        [
            ("hello refused".to_owned(), true),
            ("https://x.tail1234.ts.net".to_owned(), false)
        ]
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

fn harness(id: &str, detected: bool, states: &[&str]) -> Harness {
    Harness {
        id: id.into(),
        detected,
        items: states
            .iter()
            .enumerate()
            .map(|(i, state)| Item {
                kind: if i == 0 { "skill" } else { "hook" }.into(),
                path: Some(format!("/home/user/.{id}/{i}")),
                state: (*state).into(),
            })
            .collect(),
    }
}

#[test]
fn an_agents_state_is_one_chip_and_trouble_carries_its_sentence() {
    let state =
        |detected, states: &[&str]| integrations::agent_state(&harness("x", detected, states));
    assert_eq!(
        state(true, &["current", "current"]),
        ("Installed", ChipTone::Good, "")
    );
    assert_eq!(
        state(true, &["current", "outdated"]),
        ("Update available", ChipTone::Attention, "")
    );
    assert_eq!(
        state(true, &["missing", "missing"]),
        ("Not installed", ChipTone::Off, "")
    );
    assert_eq!(
        state(false, &["missing"]),
        ("Not found", ChipTone::Off, "Not found on this machine")
    );
    assert_eq!(
        state(true, &["foreign", "missing"]),
        (
            "Conflict",
            ChipTone::Attention,
            "Another skill named rex is installed"
        )
    );
    assert_eq!(
        state(true, &["unreadable"]),
        (
            "Unreadable",
            ChipTone::Attention,
            "Rex can't read this agent's settings"
        )
    );
}

#[test]
fn integrations_rows_are_one_line_with_chips_in_a_column_and_one_shared_skill_note() {
    let home = omp_home("chips");
    let mut status: serde_json::Value = serde_json::from_str(SKILL_STATUS).unwrap();
    for agent in status["harnesses"].as_array_mut().unwrap() {
        match agent["id"].as_str().unwrap() {
            "claude" => {
                agent["items"][0]["state"] = "current".into();
                agent["items"][1]["state"] = "current".into();
            }
            "codex" => agent["items"][0]["state"] = "foreign".into(),
            _ => {}
        }
    }
    let mut ws = workspace();
    ws.integrations.omp = omp_at(&home);
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    ws.open_section(SettingsSection::Integrations);
    ws.apply_agent_status(Ok(status.to_string()));
    let mut rt = runtime_at(ws, (1440., 1000.));
    settle(&mut rt);
    let elements = rt.elements();
    let keyed = |key: &str| {
        elements
            .iter()
            .find(|e| !e.exiting && e.key.as_deref() == Some(key))
            .unwrap_or_else(|| panic!("missing {key}"))
    };
    let right = |e: &rune::ElementInfo| e.rect[0] + e.rect[2];

    // One chip per agent, starting at one x, and every row a single 40 pt line.
    let agents = ["claude", "codex", "opencode", "pi", "omp"];
    let chips: Vec<&rune::ElementInfo> = agents
        .iter()
        .map(|id| keyed(&format!("agent-state:{id}")))
        .collect();
    let labels: Vec<&str> = chips.iter().map(|c| c.label.as_deref().unwrap()).collect();
    assert_eq!(
        labels,
        [
            "Installed",
            "Conflict",
            "Not installed",
            "Not installed",
            "Not installed"
        ]
    );
    assert!(
        chips
            .iter()
            .all(|c| (c.rect[0] - chips[0].rect[0]).abs() < 0.5),
        "{:?}",
        chips.iter().map(|c| c.rect[0]).collect::<Vec<_>>()
    );
    for id in agents {
        let row = keyed(&format!("agent:{id}"));
        assert!((row.rect[3] - 40.).abs() < 0.5, "{id}: {:?}", row.rect);
        assert!(
            subtree(&elements, &format!("agent:{id}"))
                .into_iter()
                .any(|i| elements[i].role == "icon"),
            "{id} leads with its mark"
        );
    }
    // Trouble is said beside its word; a healthy row says nothing more.
    assert_eq!(
        text_in(
            &elements,
            "agent:codex",
            "Another skill named rex is installed"
        )
        .role,
        "text"
    );
    assert_eq!(
        element_text(&rt, "agent:claude"),
        ["Claude Code", "Installed", "Remove…"]
    );
    assert_eq!(
        element_text(&rt, "agent:codex").last().map(String::as_str),
        Some("Replace…")
    );

    // The shared skill is explained once, under the card, with its directory.
    let texts: Vec<&str> = elements
        .iter()
        .filter(|e| !e.exiting)
        .filter_map(|e| e.text.as_deref())
        .collect();
    assert_eq!(
        texts
            .iter()
            .filter(|t| t.contains("share one skill"))
            .copied()
            .collect::<Vec<_>>(),
        [
            "Codex, OpenCode, Pi and OMP share one skill, so a change to one changes it for all of them:"
        ]
    );
    assert!(texts.contains(&"/home/user/.agents/skills/rex"));
    assert!(!texts.iter().any(|t| t.contains("Shares")), "{texts:?}");

    // The page's actions end where the rows' controls do, the one for every agent last.
    let (refresh, all) = (keyed("agents-refresh"), keyed("agents-setup-all"));
    assert!(refresh.rect[0] < all.rect[0]);
    assert!(
        (right(all) - right(keyed("agent-remove:claude"))).abs() < 0.5,
        "{:?} and {:?}",
        all.rect,
        keyed("agent-remove:claude").rect
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn agents_that_share_no_skill_get_no_note() {
    let alone = [
        harness("claude", true, &["current"]),
        harness("codex", true, &["missing"]),
    ];
    assert!(integrations::shared_skills(&alone).is_empty());
    let mut pair = alone.to_vec();
    pair[1].items[0].path = pair[0].items[0].path.clone();
    assert_eq!(
        integrations::shared_skills(&pair),
        [(
            "/home/user/.claude/0".to_owned(),
            vec!["Claude Code", "Codex"]
        )]
    );
}

#[test]
fn the_toolbars_settings_button_is_a_gear() {
    // A toothed rim around one hub, not the two sliders that read as view options.
    assert_eq!(theme::SETTINGS.name, "rex-settings");
    assert_eq!(theme::SETTINGS.body.matches("<circle").count(), 1);
    assert_eq!(theme::SETTINGS.body.matches("<path").count(), 1);
    let rt = runtime(workspace());
    let elements = rt.elements();
    let button = elements
        .iter()
        .find(|e| e.key.as_deref() == Some("settings"))
        .expect("the toolbar's Settings button");
    assert_eq!(button.label.as_deref(), Some("Settings"));
    std::fs::remove_file(&rt.state.settings_path).ok();
}
