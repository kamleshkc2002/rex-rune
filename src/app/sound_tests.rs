//! Never reaches a real audio executable: Workspace's sound seam is fake in all tests.
use super::*;
use crate::app::{
    panel::SettingsSection,
    sounds::{Health, Request, Sound},
};
use rex_linux::connection::activity::ProgramState::*;
use std::time::Duration;

fn flush(ws: &mut Workspace) {
    ws.sounds.fake.advance(Duration::from_millis(100));
}

#[test]
fn sound_defaults_old_files_and_round_trip() {
    for json in ["{}", r#"{"notifications":{"done":false}}"#] {
        let old: Settings = serde_json::from_str(json).unwrap();
        assert!(!old.notifications.play_sounds);
        assert!(
            serde_json::to_value(&old).unwrap()["notifications"]
                .get("play_sounds")
                .is_none()
        );
    }
    let mut settings = Settings::default();
    settings.notifications.play_sounds = true;
    let copy: Settings = serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
    assert_eq!(settings.notifications, copy.notifications);
}

#[test]
fn sound_off_never_looks_up_or_spawns_for_any_activity() {
    let (mut ws, _) = tabbed_workspace("single");
    for state in [Working, Blocked, Error, Done, Idle] {
        set_activity(&mut ws, "block:b", &[("", record(state, true))]);
        flush(&mut ws);
    }
    assert!(ws.sounds.fake.player.lookups.is_empty());
    assert!(ws.sounds.fake.player.invocations.is_empty());
}

#[test]
fn sounds_use_fixed_ids_independent_of_desktop_and_hostile_text() {
    for desktop in [false, true] {
        for (state, expected) in [
            (Blocked, Sound::NeedsYou),
            (Error, Sound::Error),
            (Done, Sound::Done),
        ] {
            let (mut ws, host) = tabbed_workspace("single");
            ws.settings.notifications.play_sounds = true;
            ws.settings.notifications.system_notifications = desktop;
            ws.notification_outbox.failure = Some(crate::app::notify::Service::NotFound);
            let mut r = record(state, true);
            let hostile = "--help; $(touch /never) `bad` | app";
            r.title = Some(hostile.into());
            r.msg = Some(hostile.into());
            r.app = Some(hostile.into());
            set_activity(&mut ws, "block:b", &[(hostile, r)]);
            flush(&mut ws);
            assert_eq!(ws.sounds.fake.player.lookups, [expected]);
            let command = &ws.sounds.fake.player.invocations[0];
            assert_eq!(
                command.program,
                std::path::PathBuf::from("/private/canberra-gtk-play")
            );
            let id = match expected {
                Sound::NeedsYou => "dialog-warning",
                Sound::Error => "dialog-error",
                Sound::Done => "complete",
            };
            assert_eq!(
                command.args,
                vec![std::ffi::OsString::from("-i"), id.into()]
            );
            assert!(host.is_empty());
            assert!(ws.notices.is_empty());
        }
    }
}

#[test]
fn sound_uses_existing_visibility_mute_and_kind_rules_but_never_readback_or_working() {
    for case in [
        "working",
        "readback",
        "muted",
        "seen",
        "blocked-off",
        "error-off",
        "done-off",
    ] {
        let (mut ws, _) = tabbed_workspace("single");
        ws.settings.notifications.play_sounds = true;
        ws.settings.notifications.working = true;
        let mut block = "block:b";
        let mut state = Blocked;
        let mut live = true;
        match case {
            "working" => state = Working,
            "readback" => live = false,
            "muted" => ws.settings.muted_tabs.push(rex_linux::settings::MutedTab {
                endpoint: ws.settings.hosts[0].endpoint.clone(),
                session: "s".into(),
                window: "split".into(),
            }),
            "seen" => {
                ws.window_focused = true;
                block = "block:a";
            }
            "blocked-off" => ws.settings.notifications.blocked = false,
            "error-off" => {
                state = Error;
                ws.settings.notifications.error = false;
            }
            "done-off" => {
                state = Done;
                ws.settings.notifications.done = false;
            }
            _ => unreachable!(),
        }
        set_activity(&mut ws, block, &[("", record(state, live))]);
        flush(&mut ws);
        assert!(ws.sounds.fake.player.lookups.is_empty(), "{case}");
        assert!(ws.sounds.fake.player.invocations.is_empty(), "{case}");
    }
}

#[test]
fn a_program_that_finishes_by_removing_its_record_is_heard_like_it_is_notified() {
    // The usual way a program finishes: it was working (or waiting), and its status
    // record goes away. The tab then has no record at all, and so nothing marked live.
    for before in [Working, Blocked] {
        let (mut ws, _) = tabbed_workspace("single");
        ws.settings.notifications.play_sounds = true;
        set_activity(&mut ws, "block:b", &[("", record(before, true))]);
        flush(&mut ws);
        ws.sounds.fake.player.lookups.clear();
        ws.notification_outbox.sent.clear();
        // Past the quiet second after the needs-you sound.
        ws.sounds.fake.advance(Duration::from_secs(2));
        set_activity(&mut ws, "block:b", &[]);
        flush(&mut ws);
        let sent = &ws.notification_outbox.sent;
        assert_eq!(sent.len(), 1, "{before:?}: notified as Done");
        assert!(sent[0].0.summary.ends_with("Done"), "{before:?}");
        assert_eq!(
            ws.sounds.fake.player.lookups,
            [Sound::Done],
            "{before:?}: and heard as Done, once"
        );
        // With the Finishes kind off, neither happens.
        let (mut ws, _) = tabbed_workspace("single");
        ws.settings.notifications.play_sounds = true;
        ws.settings.notifications.done = false;
        set_activity(&mut ws, "block:b", &[("", record(Working, true))]);
        set_activity(&mut ws, "block:b", &[]);
        flush(&mut ws);
        assert!(ws.sounds.fake.player.invocations.is_empty());
    }
}

#[test]
fn unfocused_front_tab_plays_once_but_messages_and_repeats_never_do() {
    let (mut ws, _) = tabbed_workspace("single");
    ws.settings.notifications.play_sounds = true;
    ws.window_focused = false;
    set_activity(&mut ws, "block:a", &[("", record(Blocked, true))]);
    flush(&mut ws);
    assert_eq!(ws.sounds.fake.player.lookups, [Sound::NeedsYou]);
    ws.sounds.fake.advance(Duration::from_secs(2));
    set_activity(&mut ws, "block:a", &[("", record(Blocked, true))]);
    ws.deliver_program_notices(
        0,
        vec![rex_linux::connection::activity::DesktopNotice {
            session: "s".into(),
            block: "block:b".into(),
            title: "--evil".into(),
            body: "$(bad)".into(),
        }],
    );
    flush(&mut ws);
    assert_eq!(ws.sounds.fake.player.lookups, [Sound::NeedsYou]);
    assert_eq!(
        ws.notification_outbox.sent.len(),
        2,
        "activity plus independent message"
    );
}

#[test]
fn sound_failure_does_not_affect_notifications_or_add_a_notice() {
    let (mut ws, _) = tabbed_workspace("single");
    ws.settings.notifications.play_sounds = true;
    ws.sounds.fake.player.result = Some(false);
    set_activity(&mut ws, "block:b", &[("", record(Blocked, true))]);
    flush(&mut ws);
    ws.sound_request(Request::Check);
    assert_eq!(ws.sound_health, Health::Failed);
    assert_eq!(
        ws.sound_health.text(),
        "The last sound could not be played."
    );
    assert_eq!(ws.notification_outbox.sent.len(), 1);
    assert!(ws.notices.is_empty());
}

fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
}

#[test]
fn sound_settings_search_play_off_missing_state_and_local_only() {
    let _families = rune::text::with_families("Inter", "Inter");
    for size in [(1024., 700.), (640., 480.)] {
        let (mut ws, host) = tabbed_workspace("single");
        ws.settings.notifications.system_notifications = false;
        let (sender, terminal) = flume::unbounded();
        super::pane_verb_tests::framed_terminal(&mut ws, "block:a", &sender, b"unchanged");
        let mut rt = runtime_at(ws, size);
        settle(&mut rt);
        host.drain().for_each(drop);
        terminal.drain().for_each(drop);
        let claimed = rt.state.claimed.clone();
        rt.update(|s, tx| s.perform(Intent::Settings, tx));
        settle(&mut rt);
        click(&mut rt, "settings-section:notifications");
        settle(&mut rt);
        assert_eq!(rt.state.sounds.fake.player.lookups.len(), 1);
        assert!(rt.state.sounds.fake.player.invocations.is_empty());
        rt.update(|s, _| {
            s.settings_nav.search = rune::TextEdit::new("sound");
            s.settings_search_changed();
        });
        assert!(
            rt.state
                .settings_nav
                .hits()
                .iter()
                .any(|h| h.label == "Play sounds" && h.page == SettingsSection::Notifications)
        );
        rt.update(|s, tx| {
            s.clear_settings_search();
            tx.scroll_center("sound-test");
        });
        settle(&mut rt);
        click(&mut rt, "sound-test");
        assert!(!rt.state.settings.notifications.play_sounds);
        assert_eq!(rt.state.sounds.fake.player.invocations.len(), 1);
        assert_eq!(
            rt.state.sounds.fake.player.lookups.last(),
            Some(&Sound::Done)
        );
        rt.state.sounds.fake.player.missing = true;
        click(&mut rt, "sound-test");
        assert_eq!(rt.state.sound_health, Health::Missing);
        click(&mut rt, "notify-sounds");
        assert!(
            rt.state.settings.notifications.play_sounds,
            "missing keeps switch usable"
        );
        let saved = Settings::load(&rt.state.settings_path).unwrap();
        assert!(saved.notifications.play_sounds);
        settle(&mut rt);
        let scroll = super::pane_verb_tests::rect(&rt, "settings-scroll:notifications");
        for e in rt.elements().iter().filter(|e| {
            !e.exiting
                && e.path.contains("settings-scroll:notifications")
                && matches!(e.role, "text" | "button" | "status")
        }) {
            assert!(
                e.rect[0] >= scroll[0] - 0.5
                    && e.rect[0] + e.rect[2] <= scroll[0] + scroll[2] + 0.5,
                "{} {:?}",
                e.path,
                e.rect
            );
        }
        assert!(host.is_empty() && terminal.is_empty());
        assert_eq!(rt.state.claimed, claimed);
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

fn draw(missing: bool) {
    use super::pane_verb_tests::Camera;
    let Some(mut camera) = Camera::new() else {
        return;
    };
    let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
    let _families = rune::text::with_families("Inter", "IBM Plex Mono");
    let mut ws = workspace();
    ws.sounds.fake.player.missing = missing;
    ws.panel = Some(crate::app::panel::Panel::new(
        PanelKind::Settings,
        "Settings",
        &[],
    ));
    ws.open_section(SettingsSection::Notifications);
    let size = if std::env::var_os("REX_M34_MINIMUM").is_some() {
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
    settle(&mut rt);
    let name = if missing {
        "sound-missing"
    } else {
        "sound-available"
    };
    camera.snap(&mut rt, name);
    rt.update(|_, tx| tx.scroll_center("sound-test"));
    settle(&mut rt);
    camera.snap(&mut rt, &format!("{name}-row"));
}
#[test]
fn sound_available_draw() {
    draw(false);
}
#[test]
fn sound_missing_draw() {
    draw(true);
}
