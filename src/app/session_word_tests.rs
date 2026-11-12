//! A sidebar session row says the most pressing thing one of its out-of-sight tabs would.
use super::pane_verb_tests::Camera;
use super::*;
use rex_rune::connection::activity::{BlockedKind, ProgramState, StatusRecord};
use serde_json::json;

/// Host 0 shows session `s`; session `other` has tabs `build`, `agent` and `quiet`.
fn fixture() -> (Workspace, flume::Receiver<HostCommand>) {
    let (mut ws, rx) = tabbed_workspace("single");
    ws.window_focused = true;
    ws.settings.notifications.system_notifications = false;
    ws.sidebar.toggle();
    let model = &mut ws.hosts[0].model;
    model.sessions = vec![session("s"), session("other")];
    let windows: Vec<_> = ["build", "agent", "quiet"]
        .iter()
        .map(|id| {
            json!({
                "window_id": id, "label": id, "focused_block_id": id,
                "layers": [{"kind": "tiled", "blocks": [{"block_id": id}]}]
            })
        })
        .collect();
    model.views.insert(
        "other".into(),
        serde_json::from_value(json!({
            "revision": 1, "session_id": "other", "active_window_id": "build",
            "windows": windows
        }))
        .expect("a view"),
    );
    (ws, rx)
}

fn report(ws: &mut Workspace, block: &str, state: ProgramState, kind: Option<BlockedKind>) {
    ws.hosts[0].model.activity.insert(
        block.into(),
        [(
            String::new(),
            StatusRecord {
                state,
                kind,
                app: Some("cargo".into()),
                live: true,
                ..Default::default()
            },
        )]
        .into(),
    );
}

fn word(ws: &Workspace) -> Option<(String, u32, String)> {
    ws.session_word(0, "other")
        .map(|(word, tone, tab)| (word.into_owned(), tone, tab))
}

#[test]
fn the_most_pressing_tab_speaks_for_its_session() {
    use crate::app::theme::{accent, danger, warning};
    use rex_rune::connection::activity::TerminalProgress;
    let (mut ws, _rx) = fixture();
    let host = ws.hosts[0].id;
    assert_eq!(word(&ws), None, "quiet when healthy");
    // Working shows no word on a tab, so none here.
    report(&mut ws, "build", ProgramState::Working, None);
    assert_eq!(word(&ws), None);
    // Each step puts the more pressing word on a LATER tab than the one it outranks, so
    // tab order cannot stand in for the ranking. Tabs are build, agent, quiet.
    // An unseen Done, on the first tab, is the least pressing word.
    ws.hosts[0].model.activity.remove("build");
    ws.unseen_done.insert((host, "build".into()));
    assert_eq!(word(&ws), Some(("Done".into(), accent(), "build".into())));
    // A paused report on the second tab outranks it.
    ws.hosts[0]
        .model
        .progress
        .insert("agent".into(), TerminalProgress::Paused(Some(40)));
    assert_eq!(
        word(&ws),
        Some(("Paused 40%".into(), warning(), "agent".into()))
    );
    // So does a bell, and of two warnings the earlier tab speaks.
    ws.unseen_bells.insert((host, "quiet".into()));
    assert_eq!(word(&ws).unwrap().2, "agent");
    ws.hosts[0].model.progress.remove("agent");
    assert_eq!(word(&ws), Some(("Bell".into(), warning(), "quiet".into())));
    // A failed progress report on the last tab outranks a bell on an earlier one.
    ws.unseen_bells.clear();
    ws.unseen_bells.insert((host, "agent".into()));
    ws.hosts[0]
        .model
        .progress
        .insert("quiet".into(), TerminalProgress::Error(None));
    assert_eq!(word(&ws), Some(("Error".into(), danger(), "quiet".into())));
    // A program error does too.
    ws.hosts[0].model.progress.remove("quiet");
    report(&mut ws, "quiet", ProgramState::Error, None);
    assert_eq!(word(&ws), Some(("Error".into(), danger(), "quiet".into())));
    // Needing you, on the last tab, outranks an error on the first.
    report(&mut ws, "build", ProgramState::Error, None);
    report(
        &mut ws,
        "quiet",
        ProgramState::Blocked,
        Some(BlockedKind::Permission),
    );
    assert_eq!(
        word(&ws),
        Some(("Needs approval".into(), warning(), "quiet".into()))
    );
    // Among equals the first tab in the server's order speaks.
    report(
        &mut ws,
        "agent",
        ProgramState::Blocked,
        Some(BlockedKind::Question),
    );
    assert_eq!(
        word(&ws),
        Some(("Question".into(), warning(), "agent".into()))
    );
}

#[test]
fn the_ranking_colours_are_distinct_in_every_palette() {
    // The rank after needs-you is read from the word's colour.
    for chrome in rex_rune::settings::ChromeTheme::ALL {
        let p = crate::app::theme::palette_of(chrome);
        assert!(
            p.accent != p.warning && p.warning != p.danger && p.accent != p.danger,
            "{}",
            chrome.name()
        );
    }
}

#[test]
fn muted_tabs_and_the_activity_switch_keep_the_row_quiet() {
    let (mut ws, _rx) = fixture();
    report(&mut ws, "agent", ProgramState::Blocked, None);
    report(&mut ws, "build", ProgramState::Error, None);
    assert_eq!(word(&ws).unwrap().0, "Needs input");
    // A muted tab says nothing; the next most pressing one does.
    ws.settings.muted_tabs.push(rex_rune::settings::MutedTab {
        endpoint: ws.settings.hosts[0].endpoint.clone(),
        session: "other".into(),
        window: "agent".into(),
    });
    assert_eq!(word(&ws).unwrap().0, "Error");
    // With activity in tabs off, no tab shows a word and neither does the session.
    ws.settings.notifications.show_in_tabs = false;
    assert_eq!(word(&ws), None);
    // A session this client has no layout for says nothing.
    assert_eq!(ws.session_word(0, "unknown"), None);
}

#[test]
fn the_sidebar_row_shows_the_word_except_for_the_session_on_screen() {
    let _families = rune::text::with_families("Inter", "Inter");
    let (mut ws, rx) = fixture();
    report(
        &mut ws,
        "agent",
        ProgramState::Blocked,
        Some(BlockedKind::Permission),
    );
    // The session on screen has a tab in trouble too: its pills say so, not its row.
    report(&mut ws, "block:b", ProgramState::Error, None);
    let mut rt = runtime(ws);
    rx.drain();
    assert_eq!(
        element_text(&rt, "session-word:0:other"),
        ["Needs approval"]
    );
    assert!(!has(&rt, "session-word:0:s"));
    // The word and the tab count both fit inside the row, after the name.
    let row = super::pane_verb_tests::rect(&rt, "session:0:other");
    let word = super::pane_verb_tests::rect(&rt, "session-word:0:other");
    assert!(word[0] >= row[0] && word[0] + word[2] <= row[0] + row[2] + 0.5);
    assert!(has(&rt, "session-count:0:other"));

    // Drawing the word is not seeing the tab, and asks the server for nothing: latch a
    // Done and a Bell out of sight, draw, and both are still there.
    let host = rt.state.hosts[0].id;
    rt.update(|s, _| {
        s.unseen_done.insert((host, "quiet".into()));
        s.unseen_bells.insert((host, "quiet".into()));
    });
    for _ in 0..30 {
        rt.frame(1. / 60.);
    }
    assert_eq!(
        element_text(&rt, "session-word:0:other"),
        ["Needs approval"]
    );
    assert!(rx.is_empty(), "no host command from drawing");
    assert!(rt.state.unseen_done.contains(&(host, "quiet".into())));
    assert!(rt.state.unseen_bells.contains(&(host, "quiet".into())));
    // A name cut short beside its word keeps a tooltip naming the session.
    let name = rt
        .elements()
        .into_iter()
        .find(|e| e.path.contains("session:0:other") && e.text.as_deref() == Some("other-label"))
        .expect("the session's name");
    assert_eq!(name.help.as_deref(), Some("test-0 · other-label"));

    // With tabs listed in the sidebar, each tab row carries its own word instead.
    rt.update(|s, _| s.settings.vertical_tabs = true);
    for _ in 0..30 {
        rt.frame(1. / 60.);
    }
    assert!(!has(&rt, "session-word:0:other"));

    // Back in the toolbar layout the word returns, and then clears with its causes.
    rt.update(|s, _| s.settings.vertical_tabs = false);
    for _ in 0..30 {
        rt.frame(1. / 60.);
    }
    assert!(has(&rt, "session-word:0:other"));
    rt.update(|s, _| {
        s.hosts[0].model.activity.remove("agent");
        s.unseen_done.clear();
        s.unseen_bells.clear();
    });
    for _ in 0..30 {
        rt.frame(1. / 60.);
    }
    assert!(!has(&rt, "session-word:0:other"));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

/// The sidebar with three sessions' words, at 1024x700 or (REX_M32_MINIMUM) 640x480.
#[test]
fn session_words_draw() {
    let Some(mut camera) = Camera::new() else {
        return;
    };
    let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
    let _families = rune::text::with_families("Inter", "IBM Plex Mono");
    let (mut ws, _rx) = fixture();
    let model = &mut ws.hosts[0].model;
    model.sessions.push(session("third"));
    model
        .sessions
        .push(session("a-session-with-a-long-name-to-truncate"));
    for (id, tab) in [
        ("third", "t1"),
        ("a-session-with-a-long-name-to-truncate", "t2"),
    ] {
        model.views.insert(
            id.into(),
            serde_json::from_value(json!({
                "revision": 1, "session_id": id, "active_window_id": tab,
                "windows": [{"window_id": tab, "label": tab, "focused_block_id": tab,
                    "layers": [{"kind": "tiled", "blocks": [{"block_id": tab}]}]}]
            }))
            .expect("a view"),
        );
    }
    report(
        &mut ws,
        "agent",
        ProgramState::Blocked,
        Some(BlockedKind::Permission),
    );
    report(&mut ws, "t1", ProgramState::Error, None);
    ws.unseen_done.insert((ws.hosts[0].id, "t2".into()));
    let size = if std::env::var_os("REX_M32_MINIMUM").is_some() {
        (640., 480.)
    } else {
        (1024., 700.)
    };
    ws.viewport = [size.0, size.1];
    let mut rt = Runtime::with_text(
        ws,
        view,
        size,
        Env::new(true, 1.),
        rune::text::TextSystem::new(fonts.files, fonts.families),
    );
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    camera.snap(&mut rt, "session-words");
    for id in ["other", "third", "a-session-with-a-long-name-to-truncate"] {
        let row = super::pane_verb_tests::rect(&rt, &format!("session:0:{id}"));
        let word = super::pane_verb_tests::rect(&rt, &format!("session-word:0:{id}"));
        assert!(
            word[2] > 10.,
            "the word is whole, the name gives way: {word:?}"
        );
        assert!(
            word[0] + word[2] <= row[0] + row[2] + 0.5,
            "{id}: {word:?} in {row:?}"
        );
    }
    std::fs::remove_file(&rt.state.settings_path).ok();
}

fn switcher_row(ws: &Workspace, session: &str) -> crate::app::panel::Entry {
    ws.entries()
        .into_iter()
        .find(|e| matches!(&e.action, Intent::Select(0, id) if id == session))
        .expect("the session's row")
}

#[test]
fn the_switcher_says_the_same_word_for_sessions_out_of_sight() {
    use crate::app::theme::warning;
    let (mut ws, rx) = fixture();
    ws.picker(true);
    // Quiet when healthy: a session row has no detail, as before.
    for id in ["s", "other"] {
        let row = switcher_row(&ws, id);
        assert_eq!((row.detail.as_str(), row.tone), ("", None), "{id}");
    }
    report(
        &mut ws,
        "agent",
        ProgramState::Blocked,
        Some(BlockedKind::Permission),
    );
    let row = switcher_row(&ws, "other");
    assert_eq!(
        (row.detail.as_str(), row.tone),
        ("Needs approval", Some(warning()))
    );
    // The session on screen speaks through its tabs, here as in the sidebar.
    let current = switcher_row(&ws, "s");
    assert!(current.current);
    assert_eq!((current.detail.as_str(), current.tone), ("", None));
    // The switcher never lists tabs, so the sidebar's tab layout does not silence it.
    ws.settings.vertical_tabs = true;
    assert_eq!(switcher_row(&ws, "other").detail, "Needs approval");
    ws.settings.vertical_tabs = false;
    // The filter is still the session's name and host, not the word.
    ws.panel.as_mut().unwrap().inputs[0].text = "approval".into();
    assert!(
        !ws.entries()
            .iter()
            .any(|e| matches!(e.action, Intent::Select(..)))
    );
    ws.panel.as_mut().unwrap().inputs[0].text = "other".into();
    assert_eq!(switcher_row(&ws, "other").detail, "Needs approval");
    // Commands lists sessions with their place, not their activity.
    ws.picker(false);
    ws.panel.as_mut().unwrap().inputs[0].text = "other".into();
    let row = switcher_row(&ws, "other");
    assert_eq!(row.tone, None);
    assert!(row.detail.ends_with("Session"), "{}", row.detail);
    // What silences the sidebar word silences this one.
    ws.picker(true);
    ws.settings.muted_tabs.push(rex_rune::settings::MutedTab {
        endpoint: ws.settings.hosts[0].endpoint.clone(),
        session: "other".into(),
        window: "agent".into(),
    });
    assert_eq!(switcher_row(&ws, "other").tone, None);
    ws.settings.muted_tabs.clear();
    ws.settings.notifications.show_in_tabs = false;
    assert_eq!(switcher_row(&ws, "other").tone, None);
    // Looking at the switcher sends nothing and marks nothing seen.
    assert!(rx.is_empty());
    assert!(ws.unseen_done.is_empty() && ws.unseen_bells.is_empty());
}

/// The session switcher with words, at 1024x700 or (REX_M32_MINIMUM) 640x480.
#[test]
fn switcher_session_words_draw() {
    let mut camera = Camera::new();
    let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
    let _families = rune::text::with_families("Inter", "IBM Plex Mono");
    let (mut ws, _rx) = fixture();
    let model = &mut ws.hosts[0].model;
    model.sessions.push(session("third"));
    model
        .sessions
        .push(session("a-session-with-a-long-name-to-truncate"));
    for (id, tab) in [
        ("third", "t1"),
        ("a-session-with-a-long-name-to-truncate", "t2"),
    ] {
        model.views.insert(
            id.into(),
            serde_json::from_value(json!({
                "revision": 1, "session_id": id, "active_window_id": tab,
                "windows": [{"window_id": tab, "label": tab, "focused_block_id": tab,
                    "layers": [{"kind": "tiled", "blocks": [{"block_id": tab}]}]}]
            }))
            .expect("a view"),
        );
    }
    report(
        &mut ws,
        "agent",
        ProgramState::Blocked,
        Some(BlockedKind::Permission),
    );
    report(&mut ws, "t1", ProgramState::Error, None);
    ws.unseen_done.insert((ws.hosts[0].id, "t2".into()));
    let size = if std::env::var_os("REX_M32_MINIMUM").is_some() {
        (640., 480.)
    } else {
        (1024., 700.)
    };
    ws.viewport = [size.0, size.1];
    ws.picker(true);
    let mut rt = Runtime::with_text(
        ws,
        view,
        size,
        Env::new(true, 1.),
        rune::text::TextSystem::new(fonts.files, fonts.families),
    );
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    if let Some(camera) = &mut camera {
        camera.snap(&mut rt, "switcher-session-words");
    }
    let words = |rt: &Runtime<Workspace>| -> Vec<String> {
        rt.elements()
            .into_iter()
            // The sidebar behind the switcher shows the same words on its own rows.
            .filter(|e| e.path.contains("entry:"))
            .filter_map(|e| e.text)
            .filter(|t| matches!(t.as_str(), "Needs approval" | "Error" | "Done"))
            .collect()
    };
    assert_eq!(words(&rt), ["Needs approval", "Error", "Done"]);
    let word_at = |rt: &Runtime<Workspace>| {
        rt.elements()
            .into_iter()
            .find(|e| e.path.contains("entry:") && e.text.as_deref() == Some("Needs approval"))
            .expect("the word")
            .rect
    };
    let before = word_at(&rt);
    // Selecting a row brings its Rename and Close buttons; its word stays beside them.
    rt.update(|s, _| {
        let at = s
            .entries()
            .iter()
            .position(|e| matches!(&e.action, Intent::Select(0, id) if id == "other"))
            .unwrap();
        s.panel.as_mut().unwrap().selected = at;
    });
    for _ in 0..30 {
        rt.frame(1. / 60.);
    }
    if let Some(camera) = &mut camera {
        camera.snap(&mut rt, "switcher-session-words-selected");
    }
    assert_eq!(words(&rt), ["Needs approval", "Error", "Done"]);
    let close = rt
        .elements()
        .into_iter()
        .find(|e| e.label.as_deref() == Some("Close session"))
        .expect("the highlighted row's Close button");
    // The highlight follows the pointer, so a word that moved would jump under it.
    let after = word_at(&rt);
    assert!(
        (after[0] - before[0]).abs() < 0.5 && (after[2] - before[2]).abs() < 0.5,
        "the word keeps its place: {before:?} -> {after:?}"
    );
    // The row is announced with its word, and everything stays inside it.
    let row = rt
        .elements()
        .into_iter()
        .find(|e| e.label.as_deref() == Some("other-label, Needs approval"))
        .expect("the row's label carries the word");
    assert!(
        close.rect[0] + close.rect[2] <= row.rect[0] + row.rect[2] + 0.5
            && after[0] + after[2] <= close.rect[0],
        "word {after:?}, close {:?}, row {:?}",
        close.rect,
        row.rect
    );
    let name = rt
        .elements()
        .into_iter()
        .find(|e| e.path.contains("entry:") && e.text.as_deref() == Some("other-label"))
        .expect("the name");
    assert!(name.rect[2] > 40., "the name keeps room: {:?}", name.rect);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn another_hosts_selected_session_is_out_of_sight_and_a_dropped_host_says_nothing() {
    // Two hosts, each on its session `s`, each with a tab waiting for approval there.
    let (mut ws, receivers) = super::activity_list_tests::fixture();
    let row = |ws: &Workspace, host: usize| {
        ws.entries()
            .into_iter()
            .find(|e| matches!(&e.action, Intent::Select(h, id) if *h == host && id == "s"))
            .expect("the session's row")
    };
    ws.picker(true);
    assert_eq!(ws.active, 0);
    // The session on screen speaks through its tabs.
    assert!(row(&ws, 0).current);
    assert_eq!(row(&ws, 0).tone, None);
    // The other host's selected session is not on screen, so its row speaks.
    assert!(!row(&ws, 1).current);
    assert_eq!(row(&ws, 1).detail, "Needs approval");
    assert_eq!(ws.session_word(1, "s").unwrap().0, "Needs approval");
    // What a host said before its connection dropped is not news, here or in the sidebar,
    // as the Activity list and the chip already hold.
    ws.hosts[1].model.connected = false;
    assert_eq!(row(&ws, 1).tone, None);
    assert_eq!(ws.session_word(1, "s"), None);
    assert!(receivers.iter().all(|rx| rx.is_empty()));
}
