//! Notification Settings use only the local worker, even over a live terminal.
use super::pane_verb_tests::{Camera, framed_terminal, rect};
use super::*;
use crate::app::{
    notify::{Command, Health, Service, TestResult},
    panel::SettingsSection,
};

fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
}

#[test]
fn notification_settings_search_test_and_health_are_local_and_fit() {
    let _families = rune::text::with_families("Inter", "Inter");
    for size in [(1024., 700.), (640., 480.)] {
        let (mut ws, host) = tabbed_workspace("single");
        let (sender, terminal) = flume::unbounded();
        framed_terminal(&mut ws, "block:a", &sender, b"unchanged terminal");
        ws.settings.notifications.system_notifications = false;
        assert_eq!(ws.notification_health, Health::default());
        let mut rt = runtime_at(ws, size);
        settle(&mut rt);
        host.drain().for_each(drop);
        terminal.drain().for_each(drop);
        let claimed = rt.state.claimed.clone();
        rt.update(|s, tx| s.perform(Intent::Settings, tx));
        settle(&mut rt);
        click(&mut rt, "settings-section:notifications");
        settle(&mut rt);
        assert_eq!(rt.state.notification_health.service, Service::Available);
        assert!(
            rt.state.notification_outbox.sent.is_empty(),
            "opening only checks"
        );
        for label in ["Notification service", "Send test notification"] {
            rt.update(|s, _| {
                s.settings_nav.search = rune::TextEdit::new(label);
                s.settings_search_changed();
            });
            assert_eq!(rt.state.settings_nav.hits()[0].label, label);
            assert_eq!(
                rt.state.settings_nav.hits()[0].page,
                SettingsSection::Notifications
            );
        }
        rt.update(|s, tx| {
            s.clear_settings_search();
            tx.scroll_center("notification-test");
        });
        settle(&mut rt);
        click(&mut rt, "notification-test");
        settle(&mut rt);
        assert_eq!(rt.state.notification_health.test, TestResult::Sent);
        let outbox = &rt.state.notification_outbox;
        assert_eq!(outbox.sent.len(), 1);
        let (note, replaces, _) = &outbox.sent[0];
        assert_eq!(note.summary, "Rex test notification");
        assert!(note.target.is_none());
        assert_eq!((*replaces, note.kind.urgency()), (0, 1));
        rt.update(|s, _| s.notification_outbox.failure = Some(Service::NotFound));
        click(&mut rt, "notification-test");
        settle(&mut rt);
        assert_eq!(rt.state.notification_health.service, Service::NotFound);
        assert_eq!(
            rt.state.notification_health.test,
            TestResult::Failed("No notification service".into())
        );
        let scroll = rect(&rt, "settings-scroll:notifications");
        for e in rt.elements().iter().filter(|e| {
            !e.exiting
                && e.path.contains("settings-scroll:notifications")
                && matches!(e.role, "text" | "button" | "status")
        }) {
            assert!(
                e.rect[0] >= scroll[0] - 0.5
                    && e.rect[0] + e.rect[2] <= scroll[0] + scroll[2] + 0.5,
                "{} {:?} outside {scroll:?}",
                e.path,
                e.rect
            );
        }
        assert!(
            host.is_empty() && terminal.is_empty(),
            "no host requests, PTY input or resizes"
        );
        assert_eq!(rt.state.claimed, claimed);
        assert!(
            rt.state
                .notification_outbox
                .commands
                .iter()
                .any(|c| matches!(c, Command::Test))
        );
    }
}

#[test]
fn activity_outbox_replaces_and_seen_closes_only_that_tab() {
    use rex_rune::connection::activity::ProgramState::*;
    let (mut ws, _host) = tabbed_workspace("single");
    ws.track_activity(0);
    set_activity(&mut ws, "block:b", &[("", record(Blocked, true))]);
    set_activity(&mut ws, "block:b", &[("", record(Working, true))]);
    set_activity(&mut ws, "block:b", &[("", record(Blocked, true))]);
    assert_eq!(ws.notification_outbox.sent.len(), 2);
    assert_eq!(ws.notification_outbox.sent[1].1, 1);
    ws.window_focused = false;
    set_activity(&mut ws, "block:a", &[("", record(Blocked, true))]);
    assert_eq!(ws.notification_outbox.sent[2].1, 0);
    ws.window_focused = true;
    ws.track_activity(0);
    assert_eq!(ws.notification_outbox.closed, [2]);
    ws.hosts[0]
        .model
        .views
        .get_mut("s")
        .unwrap()
        .active_window_id = Some("split".into());
    ws.track_activity(0);
    assert_eq!(ws.notification_outbox.closed, [2, 1]);
}

fn seen_commands(ws: &Workspace) -> usize {
    ws.notification_outbox
        .commands
        .iter()
        .filter(|c| matches!(c, Command::Seen(_)))
        .count()
}

#[test]
fn a_seen_tab_asks_once_per_notification_and_never_on_every_update() {
    use rex_rune::connection::activity::ProgramState::*;
    let (mut ws, _host) = tabbed_workspace("single");
    // A tab on screen with nothing sent about it asks for nothing, however often it updates.
    for _ in 0..5 {
        ws.track_activity(0);
    }
    assert_eq!(seen_commands(&ws), 0);

    // A note goes out while the window is away; the daemon then stops answering.
    ws.window_focused = false;
    set_activity(&mut ws, "block:a", &[("", record(Blocked, true))]);
    assert_eq!(ws.notification_outbox.sent.len(), 1);
    ws.window_focused = true;
    let silent = Service::Unreachable("Service did not respond".into());
    ws.notification_outbox.failure = Some(silent.clone());
    ws.track_activity(0);
    assert_eq!(seen_commands(&ws), 1);
    assert_eq!(ws.notification_health.service, silent);
    assert!(ws.notification_outbox.closed.is_empty());

    // Later updates of the tab in front queue nothing behind the silent daemon.
    for _ in 0..20 {
        ws.track_activity(0);
    }
    assert_eq!(seen_commands(&ws), 1, "no retry on every host update");

    // The next note for that tab is what earns the next attempt.
    ws.window_focused = false;
    set_activity(&mut ws, "block:a", &[("", record(Working, true))]);
    set_activity(&mut ws, "block:a", &[("", record(Error, true))]);
    ws.window_focused = true;
    ws.track_activity(0);
    assert_eq!(seen_commands(&ws), 2);
    assert!(!ws.notification_outbox.closed.is_empty());
}

#[test]
fn a_test_on_its_way_stays_sending_until_its_own_answer() {
    let (mut ws, _host) = tabbed_workspace("single");
    ws.notification_health.test = TestResult::Sending;
    let checked = Health {
        service: Service::Available,
        test: TestResult::None,
    };
    ws.notification_health_posted(checked.clone(), false);
    assert_eq!(ws.notification_health.service, Service::Available);
    assert_eq!(ws.notification_health.test, TestResult::Sending);
    let answered = Health {
        test: TestResult::Sent,
        ..checked
    };
    ws.notification_health_posted(answered.clone(), true);
    assert_eq!(ws.notification_health, answered);
}

/// Opt-in real worker test, without native UI libraries that can terminate the process
/// when their own session-bus connection disappears. Never falls back to a personal bus.
#[test]
#[ignore = "requires REX_M34_PRIVATE_BUS and its matching private DBUS_SESSION_BUS_ADDRESS"]
fn notification_worker_reconnects_after_a_private_bus_restart() {
    use crate::app::notify::{Kind, Note, NoteTarget, Notifier};
    use rex_rune::connection::activity::ProgramState::Blocked;
    use std::{
        path::PathBuf,
        process::{Child, Command as Process, Stdio},
        time::{Duration, Instant},
    };
    let root =
        PathBuf::from(std::env::var_os("REX_M34_PRIVATE_BUS").expect("private fixture directory"));
    assert!(root.is_absolute());
    assert_eq!(
        std::env::var("DBUS_SESSION_BUS_ADDRESS").unwrap(),
        format!("unix:path={}/bus", root.display())
    );
    assert!(!root.exists(), "use a fresh private fixture directory");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("bus.conf"), format!(
        "<busconfig><type>session</type><listen>unix:path={}/bus</listen><auth>EXTERNAL</auth><policy context=\"default\"><allow own=\"*\"/><allow send_destination=\"*\"/><allow receive_sender=\"*\"/></policy></busconfig>", root.display()
    )).unwrap();
    struct ChildGuard(Child);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let wait = |predicate: &dyn Fn() -> bool| {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !predicate() {
            assert!(
                Instant::now() < deadline,
                "private worker fixture timed out"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    };
    let start = || {
        let bus = ChildGuard(
            Process::new("dbus-daemon")
                .args(["--nofork", "--config-file"])
                .arg(root.join("bus.conf"))
                .stdout(Stdio::null())
                .spawn()
                .unwrap(),
        );
        wait(&|| root.join("bus").exists());
        let service = ChildGuard(
            Process::new("/usr/bin/python3")
                .arg(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/scripts/notification-quality-x11.py"
                ))
                .arg("--service")
                .arg(&root)
                .env_remove("DISPLAY")
                .env_remove("WAYLAND_DISPLAY")
                .stdout(Stdio::null())
                .spawn()
                .unwrap(),
        );
        wait(&|| root.join("service-ready").exists());
        (bus, service)
    };
    let notifications = || -> Vec<serde_json::Value> {
        std::fs::read_to_string(root.join("calls.jsonl"))
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|call| call["method"] == "Notify")
            .collect()
    };
    let send = |worker: &Notifier, summary: &str| {
        assert!(worker.send(Command::Send(Note {
            summary: summary.into(),
            body: String::new(),
            kind: Kind::Activity(Blocked),
            target: Some(NoteTarget {
                host: crate::app::workspace::HostId(1),
                session: "private".into(),
                window: Some("tab".into()),
            })
        })));
    };
    let (bus, service) = start();
    let worker = Notifier::spawn(workspace().handle);
    send(&worker, "before bus loss");
    wait(&|| notifications().len() == 1);
    drop(service);
    drop(bus);
    std::fs::remove_file(root.join("bus")).ok();
    std::fs::remove_file(root.join("service-ready")).unwrap();
    send(&worker, "lost with bus");
    // Give the failed connection attempt its bounded deadline before restoring the bus.
    std::thread::sleep(Duration::from_secs(4));
    let (_bus, _service) = start();
    send(&worker, "after bus loss");
    wait(&|| notifications().len() >= 2);
    let sent = notifications();
    assert_eq!(sent.len(), 2, "failed note was not replayed");
    assert_eq!(sent[1]["summary"], "after bus loss");
    assert_eq!(
        sent[1]["replaces"], 0,
        "new bus never inherits the old daemon id"
    );
}

fn draw(case: &str) {
    let Some(mut camera) = Camera::new() else {
        return;
    };
    let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
    let _families = rune::text::with_families("Inter", "IBM Plex Mono");
    let mut ws = workspace();
    ws.panel = Some(crate::app::panel::Panel::new(
        PanelKind::Settings,
        "Settings",
        &[],
    ));
    ws.open_section(SettingsSection::Notifications);
    ws.notification_health = match case {
        "not-found" => Health {
            service: Service::NotFound,
            test: TestResult::Failed("No notification service".into()),
        },
        "after-test" => Health {
            service: Service::Available,
            test: TestResult::Sent,
        },
        _ => Health {
            service: Service::Available,
            test: TestResult::None,
        },
    };
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
    camera.snap(&mut rt, &format!("notification-{case}"));
    rt.update(|_, tx| tx.scroll_center("notification-test"));
    settle(&mut rt);
    camera.snap(&mut rt, &format!("notification-{case}-bottom"));
}

#[test]
fn notification_available_draw() {
    draw("available");
}
#[test]
fn notification_not_found_draw() {
    draw("not-found");
}
#[test]
fn notification_after_test_draw() {
    draw("after-test");
}
