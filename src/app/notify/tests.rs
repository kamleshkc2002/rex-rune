use super::{worker::*, *};
use rex_rune::connection::activity::ProgramState::*;

fn note(window: &str, kind: Kind) -> Note {
    Note {
        summary: window.into(),
        body: String::new(),
        kind,
        target: Some(NoteTarget {
            host: HostId(1),
            session: "session".into(),
            window: Some(window.into()),
        }),
    }
}

#[test]
fn dbus_missing_service_errors_are_not_found() {
    for error in [
        zbus::fdo::Error::ServiceUnknown("not installed".into()),
        zbus::fdo::Error::NameHasNoOwner("not running".into()),
    ] {
        assert_eq!(failure(error.into()), Service::NotFound);
    }
}

#[test]
fn only_an_error_reply_from_the_service_counts_as_its_answer() {
    // The service replied with an error: it is there, and its reason is not shown.
    let refused: zbus::Error = zbus::fdo::Error::Failed("private daemon details".into()).into();
    assert!(answered(&refused));
    assert_eq!(Service::Rejected.reason(), "Service rejected the request");
    // Nothing answered, or the bus answered for a service that is not there.
    for error in [
        zbus::fdo::Error::NoReply("slow".into()).into(),
        zbus::fdo::Error::TimedOut("slow".into()).into(),
        zbus::fdo::Error::ServiceUnknown("gone".into()).into(),
        zbus::fdo::Error::Disconnected("bus".into()).into(),
        zbus::fdo::Error::SpawnChildExited("daemon failed to start".into()).into(),
        zbus::fdo::Error::SpawnExecFailed("daemon missing".into()).into(),
        zbus::Error::InputOutput(std::sync::Arc::new(std::io::Error::other("closed"))),
    ] {
        assert!(!answered(&error), "{error:?}");
    }
    // Finding or starting the service never reads as its refusal: a daemon that failed
    // to start is unreachable, and its diagnostics stay out of Settings.
    for error in [
        zbus::fdo::Error::SpawnChildExited("private daemon details".into()),
        zbus::fdo::Error::Failed("private daemon details".into()),
    ] {
        assert_eq!(
            failure(error.into()),
            Service::Unreachable("Could not reach the service".into())
        );
    }
    assert_eq!(
        failure(zbus::fdo::Error::NoReply("slow".into()).into()),
        Service::Unreachable("Service did not respond".into())
    );
}

#[test]
fn a_refused_close_is_not_trouble_but_an_unanswered_one_is() {
    let n = note("one", Kind::Activity(Blocked));
    // The daemon no longer has the notification: closed is what was wanted.
    let mut out = Outbox::default();
    out.execute(Command::Send(n.clone()));
    out.close_failure = Some(Service::Rejected);
    out.execute(Command::Seen(n.target.clone().unwrap()));
    assert_eq!(out.ledger.health.service, Service::Available);
    // No answer is trouble, and the close is still not tried again.
    let silent = Service::Unreachable("Service did not respond".into());
    out.execute(Command::Send(n.clone()));
    out.close_failure = Some(silent.clone());
    out.execute(Command::Seen(n.target.clone().unwrap()));
    assert_eq!(out.ledger.health.service, silent);
    out.execute(Command::Seen(n.target.clone().unwrap()));
    assert_eq!(out.closed.len(), 2, "one attempt per notification");
    // A daemon that does not answer is asked once, not once per remaining notification.
    for _ in 0..3 {
        out.execute(Command::Send(note("one", Kind::Program)));
    }
    out.close_failure = Some(silent);
    out.execute(Command::Seen(n.target.clone().unwrap()));
    assert_eq!(out.closed.len(), 3);
    out.execute(Command::Seen(n.target.unwrap()));
    assert_eq!(out.closed.len(), 3, "and the rest are not tried later");
}

#[test]
fn a_click_leads_once_and_a_replacement_with_its_id_is_a_new_notification() {
    let mut out = Outbox::default();
    let n = note("one", Kind::Activity(Blocked));
    out.execute(Command::Send(n.clone()));
    assert_eq!(out.ledger.click(1), n.target);
    assert_eq!(
        out.ledger.click(1),
        None,
        "a repeated action does not navigate again"
    );
    assert_eq!(out.ledger.click(99), None, "not ours");
    // The close after the click never reached the daemon; the next note reuses the id.
    out.execute(Command::Send(n.clone()));
    assert_eq!(out.sent[1], (n.clone(), 1, 1));
    assert_eq!(
        out.ledger.click(1),
        n.target,
        "the new notification can be clicked"
    );
    out.ledger.closed(1);
    assert_eq!(out.ledger.click(1), None);
}

#[test]
fn only_the_newest_notifications_are_remembered() {
    let mut out = Outbox::default();
    let total = REMEMBERED + 6;
    for i in 0..total {
        out.execute(Command::Send(note(&format!("tab-{i}"), Kind::Program)));
    }
    assert_eq!(out.ledger.remembered(), REMEMBERED);
    // The oldest are forgotten, not closed: seeing their tab asks the daemon nothing.
    assert_eq!(out.ledger.target(1), None);
    out.execute(Command::Seen(note("tab-0", Kind::Program).target.unwrap()));
    assert!(out.closed.is_empty());
    let newest = note(&format!("tab-{}", total - 1), Kind::Program);
    out.execute(Command::Seen(newest.target.unwrap()));
    assert_eq!(out.closed, [total as u32]);
    assert_eq!(out.ledger.remembered(), REMEMBERED - 1);
}

#[test]
fn repeats_replace_but_hosts_sessions_and_tabs_are_isolated() {
    let mut out = Outbox::default();
    let first = note("one", Kind::Activity(Blocked));
    out.execute(Command::Send(first.clone()));
    out.execute(Command::Send(first.clone()));
    assert_eq!(out.sent[1].1, out.sent[0].2);
    let mut other_host = first.clone();
    other_host.target.as_mut().unwrap().host = HostId(2);
    let mut other_session = first.clone();
    other_session.target.as_mut().unwrap().session = "other".into();
    for n in [
        note("two", Kind::Activity(Blocked)),
        other_host,
        other_session,
    ] {
        out.execute(Command::Send(n));
        assert_eq!(out.sent.last().unwrap().1, 0);
    }
    out.execute(Command::Seen(first.target.unwrap()));
    assert_eq!(out.closed, [out.sent[0].2]);
    for (_, _, id) in &out.sent[2..] {
        assert!(out.ledger.target(*id).is_some());
    }
}

#[test]
fn program_messages_are_unreplaced_but_close_with_their_tab() {
    let mut out = Outbox::default();
    let n = note("one", Kind::Activity(Blocked));
    out.execute(Command::Send(n.clone()));
    for _ in 0..2 {
        out.execute(Command::Send(note("one", Kind::Program)));
    }
    out.execute(Command::Send(n.clone()));
    assert_eq!(
        out.sent
            .iter()
            .map(|(_, replaces, _)| *replaces)
            .collect::<Vec<_>>(),
        [0, 0, 0, 1]
    );
    out.execute(Command::Seen(n.target.clone().unwrap()));
    out.closed.sort_unstable();
    assert_eq!(out.closed, [1, 2, 3]);
    out.execute(Command::Seen(n.target.unwrap()));
    assert_eq!(out.closed, [1, 2, 3], "no double close");
}

#[test]
fn failed_notify_drops_once_and_a_later_note_succeeds() {
    let mut out = Outbox {
        send_failure: Some(Service::Rejected),
        ..Default::default()
    };
    out.execute(Command::Send(note("failed", Kind::Activity(Blocked))));
    assert_eq!(out.ledger.health.service, Service::Rejected);
    out.execute(Command::Send(note("later", Kind::Activity(Blocked))));
    assert_eq!(
        out.attempts
            .iter()
            .map(|n| n.summary.as_str())
            .collect::<Vec<_>>(),
        ["failed", "later"]
    );
    assert_eq!(out.sent.len(), 1);
    assert_eq!(out.sent[0].0.summary, "later");
    assert_eq!(out.sent[0].1, 0);
    assert_eq!(out.ledger.health.service, Service::Available);
}

#[test]
fn missing_service_and_lost_bus_recover_only_on_a_later_command() {
    for failure in [
        Service::NotFound,
        Service::Unreachable("Session bus disconnected".into()),
    ] {
        let mut out = Outbox {
            failure: Some(failure.clone()),
            ..Default::default()
        };
        assert_eq!(out.ledger.health, Health::default());
        out.execute(Command::Test);
        assert_eq!(out.ledger.health.service, failure);
        assert_eq!(
            out.ledger.health.test,
            TestResult::Failed(failure.reason().into())
        );
        assert!(out.attempts.is_empty());
        out.execute(Command::Check);
        assert_eq!(out.ledger.health.service, Service::Available);
        assert!(out.attempts.is_empty(), "checks never send or replay");
        out.execute(Command::Test);
        assert_eq!(out.ledger.health.test, TestResult::Sent);
        assert_eq!(out.attempts, [Note::test()]);
        assert_eq!(out.ledger.target(out.sent[0].2), None);
    }
}

#[test]
fn urgency_is_critical_only_for_blocked_and_error() {
    for state in [Idle, Working, Blocked, Error, Done] {
        assert_eq!(
            Kind::Activity(state).urgency(),
            if matches!(state, Blocked | Error) {
                2
            } else {
                1
            }
        );
    }
    assert_eq!(Kind::Program.urgency(), 1);
    assert_eq!(Kind::Test.urgency(), 1);
}

#[test]
fn dismissed_and_restarted_daemon_ids_are_not_reused_or_closed() {
    let mut out = Outbox::default();
    let n = note("one", Kind::Activity(Blocked));
    out.execute(Command::Send(n.clone()));
    out.ledger.closed(1);
    out.execute(Command::Send(n.clone()));
    assert_eq!(out.sent[1].1, 0);
    out.owner = Some("new-daemon".into());
    out.execute(Command::Seen(n.target.clone().unwrap()));
    assert!(out.closed.is_empty());
    out.execute(Command::Send(n));
    assert_eq!(out.sent[2].1, 0);
}

#[test]
fn failed_replacement_keeps_the_previous_live_id() {
    let mut out = Outbox::default();
    let n = note("one", Kind::Activity(Blocked));
    out.execute(Command::Send(n.clone()));
    out.send_failure = Some(Service::Rejected);
    out.execute(Command::Send(n.clone()));
    out.execute(Command::Seen(n.target.unwrap()));
    assert_eq!(out.closed, [1]);
}
