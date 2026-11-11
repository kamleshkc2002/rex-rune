use super::*;
#[path = "cwd_tests.rs"]
mod cwd_tests;
#[path = "download_tests.rs"]
mod download_tests;
#[path = "keymap_tests.rs"]
mod keymap_tests;
#[path = "navigation_tests.rs"]
mod navigation_tests;
#[path = "perform_tests.rs"]
mod perform_tests;
#[path = "server_info_tests.rs"]
mod server_info_tests;
#[path = "verb_tests.rs"]
mod verb_tests;
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

const SESSION: &str = "session:00000000-0000-0000-0000-000000000001";

fn ticket(session: &str, block: &str, expires_in_ms: u64) -> session::DataTicket {
    session::DataTicket {
        session_id: session.into(),
        block_id: block.into(),
        data_ref_id: "terminal".into(),
        subprotocol: Some(wire::PROTOCOL.into()),
        ticket: "single-use".into(),
        expires_in_ms,
    }
}

#[test]
fn tickets_are_scoped_expiring_and_single_use() {
    let mut state = ControlState::default();
    let mut wrong = ticket(SESSION, "wrong", 60_000);
    wrong.subprotocol = Some("other".into());
    state.add_tickets(
        vec![
            ticket(SESSION, "valid", 60_000),
            ticket(SESSION, "expired", 0),
            wrong,
        ],
        Instant::now(),
    );
    assert!(state.take_ticket("other-session", "valid").is_none());
    assert!(state.take_ticket(SESSION, "wrong").is_none());
    assert!(state.take_ticket(SESSION, "expired").is_none());
    assert!(state.take_ticket(SESSION, "valid").unwrap().bundled);
    assert!(state.take_ticket(SESSION, "valid").is_none());
    state.add_tickets(
        vec![ticket(SESSION, "slow", 100)],
        Instant::now() - Duration::from_secs(1),
    );
    assert!(state.take_ticket(SESSION, "slow").is_none());
    state.add_tickets(vec![ticket(SESSION, "removed", 60_000)], Instant::now());
    state.prune(&HostModel::default());
    assert!(state.tickets.is_empty());
}

#[test]
fn selection_and_bootstrap_preserve_explicit_intent() {
    let sessions = vec![session::Session {
        session_id: SESSION.into(),
        label: "work".into(),
    }];
    assert_eq!(select_session(&sessions, None).as_deref(), Some(SESSION));
    assert_eq!(
        select_session(&sessions, Some("work")).as_deref(),
        Some(SESSION)
    );
    assert_eq!(
        select_session(&sessions, Some(SESSION)).as_deref(),
        Some(SESSION)
    );
    assert!(select_session(&sessions, Some("missing")).is_none());
    let mut duplicate = sessions.clone();
    duplicate.push(session::Session {
        session_id: "other".into(),
        label: "work".into(),
    });
    assert!(select_session(&duplicate, Some("work")).is_none());
    for (wanted, creating, expected) in [
        (None, false, None),
        (Some("work"), false, Some(json!([]))),
        (Some("session:bad-id"), false, Some(json!([]))),
        (None, true, Some(json!([]))),
        (Some(SESSION), false, Some(json!([SESSION]))),
    ] {
        let request =
            serde_json::to_value(bootstrap_request(wanted, creating, std::iter::empty())).unwrap();
        assert_eq!(request.get("attach_session_ids").cloned(), expected);
        assert_eq!(request["data_tickets"][0]["subprotocol"], wire::PROTOCOL);
    }
    let retained = bootstrap_request(
        Some(SESSION),
        false,
        [SESSION.to_owned(), SESSION.to_owned()].into_iter(),
    );
    assert_eq!(retained.attach_session_ids, Some(vec![SESSION.into()]));
}

#[allow(clippy::result_large_err)] // tungstenite's callback fixes the error type.
async fn accept(listener: &TcpListener, protocol: &str) -> WebSocketStream<TcpStream> {
    let (tcp, _) = listener.accept().await.unwrap();
    tokio_tungstenite::accept_hdr_async(
        tcp,
        |_: &_, mut response: tokio_tungstenite::tungstenite::handshake::server::Response| {
            response
                .headers_mut()
                .insert("Sec-WebSocket-Protocol", protocol.parse().unwrap());
            Ok(response)
        },
    )
    .await
    .unwrap()
}

async fn recv(ws: &mut WebSocketStream<TcpStream>) -> Value {
    serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap()).unwrap()
}

async fn send(ws: &mut WebSocketStream<TcpStream>, value: Value) {
    ws.send(Message::Text(value.to_string().into()))
        .await
        .unwrap();
}

async fn welcome(ws: &mut WebSocketStream<TcpStream>, revision: u32) -> Value {
    let hello = recv(ws).await;
    assert_eq!(hello["api_revision"], 13);
    assert_eq!(hello["api_min_revision"], 10);
    send(ws, json!({"type":"welcome", "protocol":"rex.control.v1", "api_revision":revision,
        "limits":{"max_message_bytes":65536}, "client":{"id":"client:test", "reconnect_token":"test-token"}})).await;
    hello
}

fn boot() -> session::BootstrapResponse {
    session::BootstrapResponse {
        client_id: "client:test".into(),
        attach_all: session::AttachAllResponse {
            sessions: vec![session::Session {
                session_id: SESSION.into(),
                label: "snapshot".into(),
            }],
        },
        selected_session: Some(session::SelectedSession {
            session: session::Session {
                session_id: SESSION.into(),
                label: "snapshot".into(),
            },
            view: session::ViewResponse {
                session_id: Some(SESSION.into()),
                label: Some("snapshot".into()),
                revision: 1,
                ..Default::default()
            },
        }),
        ..Default::default()
    }
}

async fn connected(host: &HostConnection) {
    tokio::time::timeout(Duration::from_secs(8), async {
        while !host.model.lock().unwrap().connected {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn bootstrap_is_first_and_newer_pre_response_events_survive() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 10).await;
        let request = recv(&mut ws).await;
        assert_eq!(request["method"], "session.bootstrap");
        // An older destruction must not remove a session present in the snapshot.
        send(&mut ws, json!({"type":"event","seq":9,"payload":{"type":"session_destroyed","payload":{"session_id":SESSION}}})).await;
        // A newer event can arrive BEFORE the bootstrap response.
        send(&mut ws, json!({"type":"event","seq":11,"payload":{"type":"session_event","payload":{
            "session_id":SESSION,"event":{"type":"session_label_changed","payload":{"label":"newer"}}}}})).await;
        send(
            &mut ws,
            json!({"type":"response","id":request["id"],"seq":10,"payload":boot()}),
        )
        .await;
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    connected(&host).await;
    {
        let model = host.model.lock().unwrap();
        assert_eq!(model.sessions[0].label, "newer");
        assert_eq!(model.selected.as_deref(), Some(SESSION));
        assert_eq!(model.protocol.bootstrap_requests, 1);
        assert_eq!(model.protocol.attach_requests, 0);
    }
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn unsupported_negotiated_revision_is_rejected() {
    for revision in [9, 14] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let mut ws = accept(&listener, "rex.control.v1").await;
            welcome(&mut ws, revision).await;
        });
        let error = transport::control(&endpoint, None).await.err().unwrap();
        assert_eq!(
            error.to_string(),
            format!("Rex negotiated API revision {revision}; this client supports 10–13")
        );
        server.await.unwrap();
    }
}

#[tokio::test]
async fn supported_revisions_negotiate() {
    for revision in [10, 11, 12, 13] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let mut ws = accept(&listener, "rex.control.v1").await;
            welcome(&mut ws, revision).await;
            assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
        });
        let control = transport::control(&endpoint, None).await.unwrap();
        assert_eq!(control.welcome().api_revision, revision);
        control.close().await.unwrap();
        server.await.unwrap();
    }
}

#[test]
fn snapshot_cutoffs_are_scoped_to_the_state_they_replace() {
    let state = ControlState {
        bootstrap_seq: 10,
        list_seq: 15,
        view_seq: HashMap::from([(SESSION.into(), 20)]),
        ..Default::default()
    };
    let event = |seq, name: &str, session: Option<&str>| rexrs::Event {
        seq,
        name: name.into(),
        session_id: session.map(str::to_owned),
        payload: json!({}),
    };
    assert!(!state.includes(&event(20, "block_closed", Some(SESSION))));
    assert!(state.includes(&event(21, "block_closed", Some(SESSION))));
    assert!(state.includes(&event(19, "block_closed", Some("other-session"))));
    assert!(!state.includes(&event(15, "session_destroyed", None)));
    assert!(state.includes(&event(16, "session_destroyed", Some(SESSION))));
    assert!(!state.includes(&event(19, "session_label_changed", Some(SESSION))));
    assert!(state.includes(&event(21, "session_label_changed", Some(SESSION))));
}

#[tokio::test]
async fn failed_bootstrap_closes_and_retries_without_old_identity() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut first = accept(&listener, "rex.control.v1").await;
        welcome(&mut first, 11).await;
        let request = recv(&mut first).await;
        assert_eq!(request["method"], "session.bootstrap");
        send(&mut first, json!({"type":"response","id":request["id"],"error":{"code":"session_not_found","message":"closed during setup","status":404}})).await;
        assert!(matches!(first.next().await, Some(Ok(Message::Close(_)))));
        let mut second = accept(&listener, "rex.control.v1").await;
        let hello = welcome(&mut second, 11).await;
        assert!(hello.get("reconnect_token").is_none());
        let request = recv(&mut second).await;
        assert_eq!(request["method"], "session.bootstrap");
        send(
            &mut second,
            json!({"type":"response","id":request["id"],"seq":1,"payload":boot()}),
        )
        .await;
        assert!(matches!(second.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    connected(&host).await;
    assert_eq!(host.model.lock().unwrap().protocol.bootstrap_requests, 2);
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn rejected_bundle_gets_fresh_ticket_and_new_socket() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        for accepted in [false, true] {
            let mut ws = accept(&listener, wire::PROTOCOL).await;
            let message = ws.next().await.unwrap().unwrap().into_data();
            assert_eq!(message.len(), 17);
            assert_eq!(message[0], 1);
            let mut response = if accepted { vec![0; 32] } else { vec![0x82, 1] };
            if accepted {
                response[0] = 0x81;
                response[24] = 1;
            }
            ws.send(Message::Binary(response.into())).await.unwrap();
        }
    });
    let (sender, commands) = flume::unbounded();
    let host = tokio::spawn(async move {
        for bundled in [true, false] {
            let HostCommand::Ticket {
                session,
                block,
                reply,
            } = commands.recv_async().await.unwrap()
            else {
                panic!("expected ticket");
            };
            assert_eq!(session, SESSION);
            assert_eq!(block, "block");
            assert!(
                reply
                    .send(Ok(TerminalTicket {
                        token: "AAAAAAAAAAAAAAAAAAAAAA".into(),
                        bundled
                    }))
                    .is_ok()
            );
        }
    });
    let (_, welcome) = open_data(&endpoint, SESSION, "block", &sender)
        .await
        .unwrap();
    assert_eq!(welcome.nonce, 1);
    host.await.unwrap();
    server.await.unwrap();
}

/// A data server for one upload: the HELLO, then every byte until the empty end message,
/// closed with `code`.
async fn upload_server(listener: TcpListener, code: u16) -> Vec<u8> {
    use tokio_tungstenite::tungstenite::protocol::{CloseFrame, frame::coding::CloseCode};
    let mut ws = accept(&listener, crate::upload::PROTOCOL).await;
    let hello = ws.next().await.unwrap().unwrap().into_data();
    assert_eq!((hello.len(), hello[0]), (17, 1));
    let mut welcome = vec![0; 32];
    welcome[0] = 0x81;
    welcome[24] = 1;
    ws.send(Message::Binary(welcome.into())).await.unwrap();
    let mut received = Vec::new();
    loop {
        let bytes = ws.next().await.unwrap().unwrap().into_data();
        if bytes.is_empty() {
            break;
        }
        assert!(bytes.len() <= 64 * 1024);
        received.extend_from_slice(&bytes);
    }
    let reason = if code == 1000 { "" } else { "too large" };
    ws.close(Some(CloseFrame {
        code: CloseCode::from(code),
        reason: reason.into(),
    }))
    .await
    .unwrap();
    while ws.next().await.is_some() {}
    received
}

#[tokio::test]
async fn uploads_stream_exact_bytes_end_empty_and_succeed_only_on_a_normal_close() {
    use crate::upload::{File, send};
    let bytes: Vec<u8> = (0..200_000u32).map(|i| i as u8).collect();
    let path = std::env::temp_dir().join(format!("rex-upload-test-{}", std::process::id()));
    std::fs::write(&path, &bytes).unwrap();
    for (file, code) in [
        (File::path(path.clone()), 1000),
        (File::bytes("clipboard.png", bytes.clone()), 1000),
        (File::bytes("clipboard.png", bytes.clone()), 4009),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(upload_server(listener, code));
        let size = file.size().unwrap();
        let result = send(&endpoint, "AAAAAAAAAAAAAAAAAAAAAA", &file, size).await;
        assert_eq!(server.await.unwrap(), bytes);
        match code {
            1000 => result.unwrap(),
            _ => assert!(format!("{:#}", result.unwrap_err()).contains("too large")),
        }
    }
    std::fs::remove_file(path).unwrap();
}

const FIXTURE: &str = include_str!("fixtures/block_events.jsonl");
const FIXTURE_BLOCK: &str = "block:01a19b73-2201-7d4a-9b0b-39967ffa0cd5";

#[test]
fn captured_block_events_update_activity_programs_and_notices() {
    use activity::{BlockedKind, ProgramState};
    let mut model = HostModel::default();
    for line in FIXTURE.lines() {
        let payload: Value = serde_json::from_str(line).unwrap();
        let (block, event) = activity::parse_block_event(&payload).expect("a terminal event");
        assert_eq!(block, FIXTURE_BLOCK);
        model.apply_block_event(SESSION, block, event);
    }
    let records = &model.activity[FIXTURE_BLOCK];
    let ids: Vec<_> = records.keys().map(String::as_str).collect();
    // "kid" reported done, then was removed when the foreground process changed.
    assert_eq!(ids, ["", "build", "build/test"]);
    assert_eq!(records[""].state, ProgramState::Working);
    assert_eq!(records[""].app.as_deref(), Some("claude"));
    assert_eq!(records["build"].state, ProgramState::Blocked);
    assert_eq!(records["build"].kind, Some(BlockedKind::Permission));
    assert_eq!(records["build/test"].state, ProgramState::Error);
    // The shell's argv0 is "/usr/bin/bash"; only its basename shows.
    assert_eq!(model.programs[FIXTURE_BLOCK], "bash");
    let notices: Vec<_> = model
        .notices
        .iter()
        .map(|n| (n.session.as_str(), n.title.as_str(), n.body.as_str()))
        .collect();
    assert_eq!(
        notices,
        [
            (SESSION, "", "hello there"),
            (SESSION, "Title", "Body text")
        ]
    );
}

#[test]
fn a_title_lasts_while_the_program_that_set_it_runs() {
    let mut model = HostModel::default();
    let replay = |model: &mut HostModel, lines: &str| {
        for line in lines.lines() {
            let payload: Value = serde_json::from_str(line).unwrap();
            let (block, event) = activity::parse_block_event(&payload).unwrap();
            model.apply_block_event(SESSION, block, event);
        }
    };
    let block = "block:01a19e0b-91d8-7213-8fda-04f8b42016e6";
    let fixture: Vec<_> = include_str!("fixtures/title_events.jsonl")
        .lines()
        .collect();
    // A program titles the terminal just before the server reports it in the foreground.
    replay(&mut model, fixture[0]);
    replay(&mut model, fixture[1]);
    assert_eq!(model.title(block), Some("OC | USB device exploration"));
    // Back at the prompt, the shell titles it again.
    replay(&mut model, &fixture[2..].join("\n"));
    assert_eq!(model.title(block), Some("shuv@shuvdev:~"));

    // A program that exits without resetting the title leaves a stale one behind.
    let title =
        activity::parse_title(&json!({"title":"nvim notes.md","owner":{"pid":7,"name":"nvim"}}));
    model.apply_block_event(SESSION, "b".into(), title);
    let foreground = |pids: &[u64]| {
        activity::parse_process(&json!({
            "foreground": {"pid": pids[0], "name": "x"},
            "ancestors": pids[1..].iter().map(|p| json!({"pid": p})).collect::<Vec<_>>(),
        }))
    };
    model.apply_block_event(SESSION, "b".into(), foreground(&[7, 1]));
    assert_eq!(model.title("b"), Some("nvim notes.md"));
    // A child of the titling program keeps its title; the shell coming back drops it.
    model.apply_block_event(SESSION, "b".into(), foreground(&[9, 7, 1]));
    assert_eq!(model.title("b"), Some("nvim notes.md"));
    model.apply_block_event(SESSION, "b".into(), foreground(&[1]));
    assert_eq!(model.title("b"), None);
    // A cleared title is gone; an unknown owner can't go stale.
    let clear = activity::parse_title(&json!({"title":""}));
    model.apply_block_event(SESSION, "b".into(), clear);
    assert!(!model.titles.contains_key("b"));
    model.apply_block_event(
        SESSION,
        "b".into(),
        activity::parse_title(&json!({"title":"t"})),
    );
    assert_eq!(model.title("b"), Some("t"));
}

#[test]
fn unknown_states_kinds_and_events_stay_quiet() {
    use activity::{BlockEvent, ProgramState, StatusRecord};
    let record = StatusRecord::parse(&json!({"state":"sleeping","kind":"coffee","app":"x"}));
    assert_eq!(record.state, ProgramState::Idle);
    assert_eq!(record.kind, None);
    assert_eq!(record.app.as_deref(), Some("x"));
    let event = |creator: &str, name: &str| json!({"block_id":"b","creator_name":creator,"name":name,"payload":{}});
    assert!(activity::parse_block_event(&event("other.creator", "process_changed")).is_none());
    assert_eq!(
        activity::parse_block_event(&event(activity::TERMINAL, "bell")),
        Some(("b".into(), BlockEvent::Bell))
    );
    assert_eq!(
        activity::parse_block_event(&event(activity::TERMINAL, "process_changed")),
        Some((
            "b".into(),
            BlockEvent::Process {
                name: None,
                pids: Vec::new()
            }
        ))
    );
    assert!(ProgramState::Blocked > ProgramState::Error);
    assert!(ProgramState::Error > ProgramState::Working);
    assert!(ProgramState::Working > ProgramState::Done);
    assert!(ProgramState::Done > ProgramState::Idle);
}

#[test]
fn status_records_keep_their_progress() {
    use activity::StatusRecord;
    let parse = |v: Value| StatusRecord::parse(&v).progress;
    assert_eq!(parse(json!({"state":"working","progress":42})), Some(42));
    assert_eq!(parse(json!({"state":"working","progress":100})), Some(100));
    assert_eq!(parse(json!({"state":"working","progress":101})), None);
    assert_eq!(parse(json!({"state":"working","progress":null})), None);
    assert_eq!(parse(json!({"state":"working"})), None);
    // The live event path carries it too.
    let (_, event) = activity::parse_block_event(&json!({"block_id":"b",
        "creator_name":activity::TERMINAL,"name":"program_status_changed",
        "payload":{"id":"","record":{"state":"working","progress":7}}}))
    .unwrap();
    assert!(
        matches!(event, activity::BlockEvent::StatusChanged { record, .. } if record.progress == Some(7))
    );
}

#[test]
fn progress_reports_set_and_clear_a_blocks_progress() {
    use activity::TerminalProgress::*;
    let report = |payload: Value| {
        activity::parse_block_event(&json!({"block_id":"b","creator_name":activity::TERMINAL,
            "name":"progress_report","payload":payload}))
    };
    let mut model = HostModel::default();
    for (payload, progress) in [
        (json!({"state":"set","progress":42}), Some(Set(Some(42)))),
        (json!({"state":"set"}), Some(Set(None))),
        (json!({"state":"indeterminate"}), Some(Indeterminate)),
        (json!({"state":"pause","progress":7}), Some(Paused(Some(7)))),
        (json!({"state":"error","progress":101}), Some(Error(None))),
        (json!({"state":"remove"}), None),
    ] {
        let (block, event) = report(payload).expect("a progress report");
        model.apply_block_event("s", block, event);
        assert_eq!(model.progress.get("b").copied(), progress);
    }
    assert!(report(json!({"state":"sparkle"})).is_none());
    model.apply_block_event(
        "s",
        "b".into(),
        activity::BlockEvent::Progress(Some(Set(None))),
    );
    model.forget_block("b");
    assert!(model.progress.is_empty(), "a closed block's progress goes");

    // A report belongs to the program in the foreground: when another takes its place, or
    // the shell comes back, the report goes with it; the same program keeps it.
    let process = |name: Option<&str>| activity::BlockEvent::Process {
        name: name.map(str::to_owned),
        pids: name.map(|_| vec![7]).unwrap_or_default(),
    };
    model.apply_block_event("s", "b".into(), process(Some("cargo")));
    model.apply_block_event(
        "s",
        "b".into(),
        activity::BlockEvent::Progress(Some(Set(Some(5)))),
    );
    model.apply_block_event("s", "b".into(), process(Some("cargo")));
    assert_eq!(
        model.progress.get("b").copied(),
        Some(Set(Some(5))),
        "same program"
    );
    model.apply_block_event("s", "b".into(), process(None));
    assert!(
        model.progress.is_empty(),
        "the program ended without clearing it"
    );
    model.apply_block_event(
        "s",
        "b".into(),
        activity::BlockEvent::Progress(Some(Set(Some(5)))),
    );
    model.apply_block_event("s", "b".into(), process(Some("vim")));
    assert!(
        model.progress.is_empty(),
        "another program took the foreground"
    );
}

#[test]
fn progress_goes_when_another_process_of_the_same_name_takes_the_foreground() {
    use activity::TerminalProgress::Set;
    let mut model = HostModel::default();
    let python = |pids: Vec<u64>| activity::BlockEvent::Process {
        name: Some("python".into()),
        pids,
    };
    let progress = activity::BlockEvent::Progress(Some(Set(Some(42))));
    model.apply_block_event("s", "b".into(), python(vec![11, 1]));
    model.apply_block_event("s", "b".into(), progress);
    model.apply_block_event("s", "b".into(), python(vec![11, 1]));
    assert_eq!(
        model.progress.get("b").copied(),
        Some(Set(Some(42))),
        "the same process keeps its report"
    );
    model.apply_block_event("s", "b".into(), python(vec![12, 1]));
    assert!(
        model.progress.is_empty(),
        "a new python must not inherit the last one's report"
    );
}

#[test]
fn progress_goes_with_the_process_not_just_its_name() {
    use activity::{BlockEvent, TerminalProgress::Set};
    let mut model = HostModel::default();
    let python = |pids: &[u64]| BlockEvent::Process {
        name: Some("python".into()),
        pids: pids.to_vec(),
    };
    let report = BlockEvent::Progress(Some(Set(Some(42))));
    model.apply_block_event("s", "b".into(), python(&[11, 1]));
    model.apply_block_event("s", "b".into(), report.clone());

    // Rex repeats `process_changed` when the process only moves directory, and to each client
    // that starts receiving the terminal.
    model.apply_block_event("s", "b".into(), python(&[11, 1]));
    assert_eq!(
        model.progress.get("b"),
        Some(&Set(Some(42))),
        "same process"
    );

    // A child in the foreground leaves the program that reported running beneath it.
    model.apply_block_event("s", "b".into(), python(&[12, 11, 1]));
    assert_eq!(model.progress.get("b"), Some(&Set(Some(42))), "same parent");

    // Back to the parent: the foreground process that may have reported is gone.
    model.apply_block_event("s", "b".into(), python(&[11, 1]));
    assert!(model.progress.is_empty(), "the child ended");

    // Another run of the same program, seen without the shell between them.
    model.apply_block_event("s", "b".into(), report);
    model.apply_block_event("s", "b".into(), python(&[13, 1]));
    assert!(model.progress.is_empty(), "the job was replaced");
}

#[test]
fn program_names_prefer_the_script_and_drop_login_dashes() {
    let name = |v: Value| activity::program_name(&v);
    assert_eq!(
        name(json!({"name":"node","script":"/home/u/.bun/bin/pi","argv0":"node"})).as_deref(),
        Some("pi")
    );
    assert_eq!(
        name(json!({"name":"zsh","argv0":"-zsh"})).as_deref(),
        Some("zsh")
    );
    assert_eq!(name(json!({"name":"nvim"})).as_deref(), Some("nvim"));
    assert_eq!(name(Value::Null), None);
}

const OTHER: &str = "session:00000000-0000-0000-0000-000000000002";

async fn reply(ws: &mut WebSocketStream<TcpStream>, request: &Value, seq: u64, payload: Value) {
    send(
        ws,
        json!({"type":"response","id":request["id"],"seq":seq,"payload":payload}),
    )
    .await;
}

#[tokio::test]
async fn activity_follows_every_session_and_queries_the_selected_one() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 11).await;
        let request = recv(&mut ws).await;
        assert_eq!(request["method"], "session.bootstrap");
        assert_eq!(request["payload"]["event_scope"], "all");
        let mut boot = serde_json::to_value(boot()).unwrap();
        boot["attach_all"]["sessions"]
            .as_array_mut()
            .unwrap()
            .push(json!({"session_id":OTHER,"label":"other"}));
        boot["selected_session"]["view"]["windows"] = json!([{"window_id":"w",
            "layers":[{"kind":"tiled","layout":{"block_id":"block:a"},
                "blocks":[{"block_id":"block:a","creator_name":activity::TERMINAL}]}]}]);
        reply(&mut ws, &request, 1, boot).await;
        // Hidden hosts stream nothing, so both questions are asked.
        let status = recv(&mut ws).await;
        assert_eq!(status["method"], "com.superlogical.terminal.program_status");
        assert_eq!(status["payload"]["block_id"], "block:a");
        reply(
            &mut ws,
            &status,
            2,
            json!({"records":[{"state":"blocked","kind":"question","app":"codex"}]}),
        )
        .await;
        let title = recv(&mut ws).await;
        assert_eq!(title["method"], "com.superlogical.terminal.title");
        reply(
            &mut ws,
            &title,
            3,
            json!({"title":"Codex · fix tests","owner":{"pid":41,"name":"node"}}),
        )
        .await;
        let process = recv(&mut ws).await;
        assert_eq!(process["method"], "com.superlogical.terminal.process");
        reply(
            &mut ws,
            &process,
            4,
            json!({"foreground":{"name":"node","argv0":"codex"}}),
        )
        .await;
        // attach_all replaces the scope, so after session_created it must still ask for all.
        send(&mut ws, json!({"type":"event","seq":5,"payload":{"type":"session_created","payload":{"session_id":OTHER}}})).await;
        let list = recv(&mut ws).await;
        assert_eq!(list["method"], "session.attach_all");
        assert_eq!(list["payload"]["event_scope"], "all");
        reply(
            &mut ws,
            &list,
            6,
            json!({"sessions":[{"session_id":SESSION,"label":"snapshot"},
            {"session_id":OTHER,"label":"other"}]}),
        )
        .await;
        // A report from an unattached session: its view is fetched once to place it.
        let report = |seq: u64| {
            json!({"type":"event","seq":seq,"payload":{"type":"session_event","payload":{
            "session_id":OTHER,"event":{"type":"block_event","payload":{"block_id":"block:z",
                "creator_name":activity::TERMINAL,"name":"program_status_changed",
                "payload":{"id":"","record":{"state":"working"}}}}}}})
        };
        send(&mut ws, report(7)).await;
        let view = recv(&mut ws).await;
        assert_eq!(view["method"], "session.view");
        assert_eq!(view["payload"]["session_id"], OTHER);
        send(&mut ws, json!({"type":"response","id":view["id"],"error":{"code":"conflict","message":"no","status":409}})).await;
        send(&mut ws, report(8)).await;
        // No second session.view: the next message is the client closing.
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            {
                let m = host.model.lock().unwrap();
                if m.activity.contains_key("block:z") && m.programs.contains_key("block:a") {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    {
        let m = host.model.lock().unwrap();
        let a = &m.activity["block:a"][""];
        assert_eq!(a.state, activity::ProgramState::Blocked);
        assert_eq!(m.title("block:a"), Some("Codex · fix tests"));
        assert_eq!(a.app.as_deref(), Some("codex"));
        assert_eq!(m.programs["block:a"], "codex");
        assert_eq!(m.sessions.len(), 2);
    }
    // Let the second report land before closing.
    tokio::time::sleep(Duration::from_millis(100)).await;
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn destroying_a_session_forgets_it_without_an_error() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 11).await;
        let request = recv(&mut ws).await;
        reply(&mut ws, &request, 1, serde_json::to_value(boot()).unwrap()).await;
        let destroy = recv(&mut ws).await;
        assert_eq!(destroy["method"], "session.destroy");
        reply(&mut ws, &destroy, 2, json!({})).await;
        // The client re-reads the session it acted on, which is gone now.
        let view = recv(&mut ws).await;
        assert_eq!(view["method"], "session.view");
        send(
            &mut ws,
            json!({"type":"response","id":view["id"],"error":{"code":"session_not_found",
            "message":"session not found","status":404}}),
        )
        .await;
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    connected(&host).await;
    host.commands
        .send(HostCommand::Request {
            method: "session.destroy".into(),
            payload: json!({"session_id": SESSION}),
        })
        .unwrap();
    tokio::time::timeout(Duration::from_secs(8), async {
        while !host.model.lock().unwrap().sessions.is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    {
        let m = host.model.lock().unwrap();
        assert_eq!(m.notice, None);
        assert_eq!(m.selected, None);
        assert!(m.connected);
    }
    drop(host);
    server.await.unwrap();
}

#[test]
fn clipboard_events_are_not_replaced_by_a_view_snapshot() {
    let state = ControlState {
        bootstrap_seq: 10,
        view_seq: HashMap::from([(SESSION.into(), 20)]),
        ..Default::default()
    };
    for name in ["clipboard_written", "selection_copied"] {
        let event = rexrs::Event {
            seq: 19,
            name: "block_event".into(),
            session_id: Some(SESSION.into()),
            payload: json!({"creator_name": activity::TERMINAL, "name": name,
                "block_id": "block:copy", "payload": {"id": 1, "bytes": 4, "text": "copy"}}),
        };
        assert!(
            state.includes(&event),
            "a view contains no clipboard text: {name}"
        );
        assert!(!state.includes(&rexrs::Event { seq: 9, ..event }));
    }
}

#[tokio::test]
async fn clipboard_reads_pin_ids_and_count_utf8_bytes_for_both_copy_events() {
    for (event, method) in [
        (
            "clipboard_written",
            "com.superlogical.terminal.clipboard_read",
        ),
        ("selection_copied", "com.superlogical.terminal.blob_read"),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let mut ws = accept(&listener, "rex.control.v1").await;
            welcome(&mut ws, 11).await;
            let mut offset = 0;
            for (index, data) in ["世", "界🦀\n"].into_iter().enumerate() {
                let request = recv(&mut ws).await;
                assert_eq!(request["method"], method);
                assert_eq!(request["payload"]["session_id"], SESSION);
                assert_eq!(request["payload"]["block_id"], "block:copy");
                assert_eq!(
                    request["payload"]["args"],
                    json!({"id": 42, "offset": offset, "max_bytes": 49152})
                );
                offset += data.len();
                send(
                    &mut ws,
                    json!({"type":"response", "id":request["id"], "payload":{
                    "id":42,"data":data,"total":11,"eof": index == 1}}),
                )
                .await;
            }
            assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
        });
        let mut client = transport::control(&endpoint, None).await.unwrap();
        let payload =
            json!({"name": event, "block_id": "block:copy", "payload": {"id": 42, "bytes": 11}});
        let copy = clipboard::read(&mut client, SESSION, &payload)
            .await
            .unwrap();
        assert_eq!(copy.text, "世界🦀\n");
        assert_eq!(copy.session, SESSION);
        assert_eq!(copy.block, "block:copy");
        client.close().await.unwrap();
        server.await.unwrap();
    }
}

#[tokio::test]
async fn clipboard_reads_reject_stalled_truncated_changed_and_oversized_results() {
    for part in [
        json!({"id":42,"data":"","total":5,"eof":false}),
        json!({"id":42,"data":"copy","total":5,"eof":true}),
        json!({"id":43,"data":"copy","total":4,"eof":true}),
        json!({"id":42,"data":"copy","total":3,"eof":true}),
        json!({"id":42,"data":"","total":-1,"eof":true}),
        json!({"id":42,"data":"","total":8 * 1024 * 1024 + 1,"eof":false}),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let mut ws = accept(&listener, "rex.control.v1").await;
            welcome(&mut ws, 11).await;
            let request = recv(&mut ws).await;
            send(
                &mut ws,
                json!({"type":"response","id":request["id"],"payload":part}),
            )
            .await;
            assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
        });
        let mut client = transport::control(&endpoint, None).await.unwrap();
        assert!(
            clipboard::read(
                &mut client,
                SESSION,
                &json!({"name":"clipboard_written",
            "block_id":"block:copy","payload":{"id":42}})
            )
            .await
            .is_err()
        );
        client.close().await.unwrap();
        server.await.unwrap();
    }
}

#[test]
fn font_preferences_use_saved_values_unless_explicitly_overridden() {
    use clap::Parser;
    let settings = crate::settings::Settings {
        font_family: "IBM Plex Mono".into(),
        font_fallbacks: vec!["Inter".into()],
        font_size: 22.,
        ..Default::default()
    };
    let before = serde_json::to_value(&settings).unwrap();
    let defaults = Options::parse_from(["rex-linux"]);
    assert_eq!(
        defaults.fonts(&settings),
        ("IBM Plex Mono", settings.font_fallbacks.as_slice())
    );
    assert_eq!(defaults.font_size, None);
    let explicit = Options::parse_from([
        "rex-linux",
        "--font",
        "TX02 Nerd Font Mono",
        "--font-size",
        "15",
        "--font-fallback",
        "Noto Sans CJK JP",
        "--font-fallback",
        "Inter",
    ]);
    let (family, fallbacks) = explicit.fonts(&settings);
    assert_eq!(family, "TX02 Nerd Font Mono");
    assert_eq!(fallbacks, ["Noto Sans CJK JP", "Inter"]);
    assert_eq!(
        explicit.font_size,
        Some(15.),
        "an explicit 15 is not the default sentinel"
    );
    assert_eq!(
        serde_json::to_value(&settings).unwrap(),
        before,
        "CLI family choices do not overwrite preferences"
    );
    let primary_only = Options::parse_from(["rex-linux", "--font", "Inter"]);
    assert_eq!(
        primary_only.fonts(&settings),
        ("Inter", settings.font_fallbacks.as_slice())
    );
    let fallback_only = Options::parse_from(["rex-linux", "--font-fallback", "Noto Sans CJK JP"]);
    assert_eq!(fallback_only.fonts(&settings).0, "IBM Plex Mono");
    for args in [
        ["rex-linux", "--font", " "],
        ["rex-linux", "--font-fallback", "a,b"],
        ["rex-linux", "--font-size", "NaN"],
        ["rex-linux", "--font-size", "inf"],
    ] {
        assert!(Options::try_parse_from(args).is_err());
    }
}

const THIRD: &str = "session:00000000-0000-0000-0000-000000000003";

/// A bootstrap that lists `others` after the selected session.
fn boot_with(others: &[&str]) -> Value {
    let mut boot = serde_json::to_value(boot()).unwrap();
    for id in others {
        boot["attach_all"]["sessions"]
            .as_array_mut()
            .unwrap()
            .push(json!({"session_id":id,"label":id}));
    }
    boot
}

fn view_of(id: &str) -> Value {
    json!({"session_id":id,"revision":1,"windows":[{"window_id":format!("{id}/tab")}]})
}

/// The next message, which must come well inside the 8 s request timeout.
async fn next(ws: &mut WebSocketStream<TcpStream>) -> Value {
    tokio::time::timeout(Duration::from_secs(2), recv(ws))
        .await
        .expect("a message in time")
}

async fn prefetched(ws: &mut WebSocketStream<TcpStream>, id: &str) -> Value {
    let view = next(ws).await;
    assert_eq!(view["method"], "session.view");
    assert_eq!(view["payload"]["session_id"], id);
    assert!(view["id"].as_str().unwrap().starts_with("detached:"));
    view
}

async fn until(host: &HostConnection, done: impl Fn(&HostModel) -> bool) {
    tokio::time::timeout(Duration::from_secs(8), async {
        while !done(&host.model.lock().unwrap()) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}

fn prefetch(host: &HostConnection, ids: &[&str]) {
    let ids = ids.iter().map(|id| (*id).to_owned()).collect();
    host.commands.send(HostCommand::PrefetchViews(ids)).unwrap();
}

fn focus_window(host: &HostConnection) {
    host.commands
        .send(HostCommand::Request {
            method: "session.focus_window".into(),
            payload: json!({"session_id":SESSION,"window_id":"w"}),
        })
        .unwrap();
}

/// Answer a control request and the view refresh that follows it.
async fn answer_request(ws: &mut WebSocketStream<TcpStream>) {
    let request = next(ws).await;
    assert_eq!(request["method"], "session.focus_window");
    reply(ws, &request, 3, json!({})).await;
    let refresh = next(ws).await;
    assert_eq!(refresh["method"], "session.view");
    assert_eq!(refresh["payload"]["session_id"], SESSION);
    reply(ws, &refresh, 3, json!({"session_id":SESSION,"revision":2})).await;
}

#[tokio::test]
async fn a_slow_prefetch_neither_delays_control_requests_nor_overlaps_another() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (sent, mut started) = tokio::sync::mpsc::unbounded_channel();
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 11).await;
        let boot = recv(&mut ws).await;
        reply(&mut ws, &boot, 1, boot_with(&[OTHER, THIRD])).await;
        let slow = prefetched(&mut ws, OTHER).await;
        sent.send(()).unwrap();
        // Unanswered, it holds back the next fetch but not the request.
        answer_request(&mut ws).await;
        reply(&mut ws, &slow, 4, view_of(OTHER)).await;
        let view = prefetched(&mut ws, THIRD).await;
        reply(&mut ws, &view, 5, view_of(THIRD)).await;
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    connected(&host).await;
    prefetch(&host, &[OTHER, THIRD]);
    started.recv().await.unwrap();
    focus_window(&host);
    until(&host, |m| {
        m.views.contains_key(OTHER) && m.views.contains_key(THIRD)
    })
    .await;
    assert!(host.model.lock().unwrap().views[SESSION].revision == 2);
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn leaving_eligibility_cancels_queued_and_in_flight_fetches() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (sent, mut started) = tokio::sync::mpsc::unbounded_channel();
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 11).await;
        let boot = recv(&mut ws).await;
        reply(&mut ws, &boot, 1, boot_with(&[OTHER, THIRD])).await;
        let late = prefetched(&mut ws, OTHER).await;
        sent.send(()).unwrap();
        // The request is handled after the empty set, so this response is stale.
        answer_request(&mut ws).await;
        reply(&mut ws, &late, 4, view_of(OTHER)).await;
        sent.send(()).unwrap();
        // No fetch for THIRD: the next message is the client closing.
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    connected(&host).await;
    prefetch(&host, &[OTHER, THIRD]);
    started.recv().await.unwrap();
    prefetch(&host, &[]);
    focus_window(&host);
    started.recv().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(!host.model.lock().unwrap().views.contains_key(OTHER));
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn prefetch_picks_up_new_sessions_and_restarts_after_reconnect() {
    const NEW: &str = "session:00000000-0000-0000-0000-000000000004";
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 11).await;
        let boot = recv(&mut ws).await;
        reply(&mut ws, &boot, 1, boot_with(&[OTHER])).await;
        let view = prefetched(&mut ws, OTHER).await;
        reply(&mut ws, &view, 2, view_of(OTHER)).await;
        send(&mut ws, json!({"type":"event","seq":3,"payload":{"type":"session_created","payload":{"session_id":NEW}}})).await;
        let list = next(&mut ws).await;
        assert_eq!(list["method"], "session.attach_all");
        let sessions = &boot_with(&[OTHER, NEW])["attach_all"];
        reply(&mut ws, &list, 4, sessions.clone()).await;
        let view = prefetched(&mut ws, NEW).await;
        reply(&mut ws, &view, 5, view_of(NEW)).await;
        // The client drops this connection; a new one fetches every view again.
        assert!(!matches!(ws.next().await, Some(Ok(Message::Text(_)))));
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 11).await;
        let boot = recv(&mut ws).await;
        reply(&mut ws, &boot, 1, boot_with(&[OTHER, NEW])).await;
        for id in [OTHER, NEW] {
            let view = prefetched(&mut ws, id).await;
            reply(&mut ws, &view, 2, view_of(id)).await;
        }
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    connected(&host).await;
    prefetch(&host, &[OTHER, NEW]);
    until(&host, |m| m.views.contains_key(NEW)).await;
    host.commands.send(HostCommand::Reconnect).unwrap();
    // Only a Reconnect that arrives during the delay ends it; any later one would drop the
    // new connection too.
    until(&host, |m| !m.connected).await;
    host.commands.send(HostCommand::Reconnect).unwrap();
    until(&host, |m| {
        m.protocol.bootstrap_requests == 2
            && m.views.contains_key(OTHER)
            && m.views.contains_key(NEW)
    })
    .await;
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn a_failed_prefetch_is_retried_after_a_backoff() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 11).await;
        let boot = recv(&mut ws).await;
        reply(&mut ws, &boot, 1, boot_with(&[OTHER, THIRD])).await;
        let view = prefetched(&mut ws, OTHER).await;
        send(&mut ws, json!({"type":"response","id":view["id"],"error":{"code":"conflict","message":"busy","status":409}})).await;
        let failed = Instant::now();
        let view = prefetched(&mut ws, THIRD).await;
        assert!(failed.elapsed() < Duration::from_millis(500));
        reply(&mut ws, &view, 2, view_of(THIRD)).await;
        let view = prefetched(&mut ws, OTHER).await;
        assert!(failed.elapsed() >= Duration::from_millis(950));
        reply(&mut ws, &view, 3, view_of(OTHER)).await;
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    connected(&host).await;
    prefetch(&host, &[OTHER, THIRD]);
    until(&host, |m| m.views.contains_key(OTHER)).await;
    drop(host);
    server.await.unwrap();
}

/// A scripted control server for theme declarations: it answers every request, records each
/// one as (method, payload), sends the events the test pushes, and takes the client back
/// after a reconnect. Data sockets are accepted and dropped.
struct ThemeServer {
    endpoint: String,
    events: tokio::sync::mpsc::UnboundedSender<Value>,
    requests: tokio::sync::mpsc::UnboundedReceiver<(String, Value)>,
}

impl ThemeServer {
    /// Serves one session whose single window holds `blocks` (`[id, owner]`, owner none for
    /// nobody); `set_theme` fails with `theme_error` when given.
    #[allow(clippy::result_large_err)] // tungstenite's callback fixes the error type.
    async fn start(blocks: &[(&str, Option<&str>)], theme_error: Option<&'static str>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let blocks: Vec<Value> = blocks
            .iter()
            .map(|(id, owner)| json!({"block_id":id,"creator_name":activity::TERMINAL,"owner":owner}))
            .collect();
        let view = json!({"session_id":SESSION,"label":"snapshot","revision":1,
            "windows":[{"window_id":"w","layers":[{"kind":"tiled",
                "layout":{"block_id":blocks[0]["block_id"]},"blocks":blocks}]}]});
        let (events, mut pushed) = tokio::sync::mpsc::unbounded_channel::<Value>();
        let (record, requests) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut seq = 1;
            loop {
                let (tcp, _) = listener.accept().await.unwrap();
                let mut path = String::new();
                let mut ws = tokio_tungstenite::accept_hdr_async(
                    tcp,
                    |request: &tokio_tungstenite::tungstenite::handshake::server::Request,
                     mut response: tokio_tungstenite::tungstenite::handshake::server::Response| {
                        path = request.uri().path().to_owned();
                        if let Some(protocol) = request.headers().get("Sec-WebSocket-Protocol") {
                            response
                                .headers_mut()
                                .insert("Sec-WebSocket-Protocol", protocol.clone());
                        }
                        Ok(response)
                    },
                )
                .await
                .unwrap();
                if path != "/api/control" {
                    continue;
                }
                welcome(&mut ws, 11).await;
                loop {
                    let request: Value = tokio::select! {
                        message = ws.next() => match message {
                            Some(Ok(Message::Text(text))) => serde_json::from_str(&text).unwrap(),
                            Some(Ok(_)) => continue,
                            _ => break,
                        },
                        Some(event) = pushed.recv() => {
                            seq += 1;
                            send(&mut ws, json!({"type":"event","seq":seq,"payload":{"type":"session_event",
                                "payload":{"session_id":SESSION,"event":event}}})).await;
                            continue;
                        }
                    };
                    seq += 1;
                    let method = request["method"].as_str().unwrap_or_default().to_owned();
                    let _ = record.send((method.clone(), request["payload"].clone()));
                    let payload = match method.as_str() {
                        "session.bootstrap" => {
                            let mut boot = serde_json::to_value(boot()).unwrap();
                            boot["selected_session"]["view"] = view.clone();
                            boot
                        }
                        "session.attach_all" => {
                            json!({"sessions":[{"session_id":SESSION,"label":"snapshot"},
                                {"session_id":OTHER,"label":"other"}]})
                        }
                        "session.create" => json!({"session_id":OTHER}),
                        "session.attach" => json!({"client_id":"client:test",
                            "view":{"session_id":OTHER,"revision":1}}),
                        "data.connect" => json!({"ticket":"AAAAAAAAAAAAAAAAAAAAAA"}),
                        "com.superlogical.terminal.set_theme" if theme_error.is_some() => {
                            send(
                                &mut ws,
                                json!({"type":"response","id":request["id"],
                                "error":{"code":theme_error,"message":"no","status":404}}),
                            )
                            .await;
                            continue;
                        }
                        _ => json!({}),
                    };
                    reply(&mut ws, &request, seq, payload).await;
                }
            }
        });
        Self {
            endpoint,
            events,
            requests,
        }
    }

    fn owner(&self, block: &str, owner: Option<&str>) {
        self.events
            .send(json!({"type":"block_owner_changed","payload":{"block_id":block,"owner":owner}}))
            .unwrap();
    }

    /// The `set_theme` calls, as (block, args), up to the next `method` request, and that
    /// request's payload.
    async fn themes_until(&mut self, method: &str) -> (Vec<(String, Value)>, Value) {
        let mut themes = Vec::new();
        loop {
            let (name, payload) =
                tokio::time::timeout(Duration::from_secs(8), self.requests.recv())
                    .await
                    .unwrap_or_else(|_| panic!("no {method}"))
                    .unwrap();
            if name == "com.superlogical.terminal.set_theme" {
                assert_eq!(payload["session_id"], SESSION);
                themes.push((
                    payload["block_id"].as_str().unwrap().to_owned(),
                    payload["args"].clone(),
                ));
            } else if name == method {
                return (themes, payload);
            }
        }
    }

    /// The `set_theme` calls an event caused: a session_created after it makes the client
    /// list sessions once it has handled the event.
    async fn themes_after_events(&mut self) -> Vec<(String, Value)> {
        self.events
            .send(json!({"type":"session_created","payload":{"session_id":OTHER}}))
            .unwrap();
        self.themes_until("session.attach_all").await.0
    }
}

/// The `set_theme` calls a command caused: a request queued after it is handled after the
/// declarations it triggers.
async fn themes_after_command(
    server: &mut ThemeServer,
    host: &HostConnection,
    command: HostCommand,
) -> Vec<(String, Value)> {
    host.commands.send(command).unwrap();
    host.commands
        .send(HostCommand::Request {
            method: "test.sentinel".into(),
            payload: json!({}),
        })
        .unwrap();
    server.themes_until("test.sentinel").await.0
}

fn sorted(mut themes: Vec<(String, Value)>) -> Vec<(String, Value)> {
    themes.sort_by(|a, b| a.0.cmp(&b.0));
    themes
}

fn declared(blocks: &[&str], args: &Value) -> Vec<(String, Value)> {
    blocks
        .iter()
        .map(|b| ((*b).to_owned(), args.clone()))
        .collect()
}

#[tokio::test]
async fn themes_go_only_to_blocks_this_client_may_theme_once_each() {
    let mut server = ThemeServer::start(
        &[
            ("block:a", None),
            ("block:b", Some("client:test")),
            ("block:c", Some("client:other")),
        ],
        None,
    )
    .await;
    let first = SetTheme::new().background("#111111");
    let host = HostConnection::spawn(
        server.endpoint.clone(),
        None,
        None,
        None,
        true,
        Some(first.clone()),
    );
    let first = serde_json::to_value(&first).unwrap();
    let (themes, _) = server
        .themes_until("com.superlogical.terminal.program_status")
        .await;
    assert_eq!(
        sorted(themes),
        declared(&["block:a", "block:b"], &first),
        "unowned and owned blocks, not another client's"
    );

    server.owner("block:c", Some("client:test"));
    assert_eq!(
        server.themes_after_events().await,
        declared(&["block:c"], &first),
        "taking a block declares to it, with no resize"
    );
    server.owner("block:a", Some("client:other"));
    server.owner("block:a", Some("client:test"));
    assert_eq!(
        server.themes_after_events().await,
        [],
        "Rex keeps this client's declaration while another owns the block"
    );

    server.owner("block:b", Some("client:other"));
    assert_eq!(server.themes_after_events().await, []);
    let second = SetTheme::new().background("#eeeeee").foreground("#222222");
    let themes =
        themes_after_command(&mut server, &host, HostCommand::Theme(Some(second.clone()))).await;
    let second_json = serde_json::to_value(&second).unwrap();
    assert_eq!(
        sorted(themes),
        declared(&["block:a", "block:c"], &second_json)
    );
    assert_eq!(
        themes_after_command(&mut server, &host, HostCommand::Theme(Some(second.clone()))).await,
        [],
        "the same theme again is not repeated"
    );

    let created = {
        host.commands
            .send(HostCommand::CreateSession {
                label: None,
                shell: None,
            })
            .unwrap();
        server.themes_until("session.create").await.1
    };
    assert_eq!(
        created["initial_windows"][0]["layout"]["block"]["options"]["theme"], second_json,
        "new blocks start with the declared theme"
    );

    let empty = json!({});
    let themes = themes_after_command(
        &mut server,
        &host,
        HostCommand::Theme(Some(SetTheme::new())),
    )
    .await;
    assert_eq!(
        sorted(themes),
        declared(&["block:a", "block:c"], &empty),
        "Rex default colours declare an empty theme"
    );
    assert_eq!(
        themes_after_command(&mut server, &host, HostCommand::Theme(None)).await,
        [],
        "Follow server after Rex default has nothing to reset"
    );
    server.owner("block:b", Some("client:test"));
    assert_eq!(
        server.themes_after_events().await,
        declared(&["block:b"], &empty),
        "under Follow server a block taken back is reset once, not left in the old theme"
    );
    drop(host);
}

#[tokio::test]
async fn follow_server_declares_nothing_and_resets_what_it_declared() {
    let mut server = ThemeServer::start(&[("block:a", None)], None).await;
    let host = HostConnection::spawn(server.endpoint.clone(), None, None, None, true, None);
    let (themes, _) = server
        .themes_until("com.superlogical.terminal.program_status")
        .await;
    assert_eq!(themes, []);
    let theme = SetTheme::new().background("#111111");
    let themes =
        themes_after_command(&mut server, &host, HostCommand::Theme(Some(theme.clone()))).await;
    assert_eq!(
        themes,
        declared(&["block:a"], &serde_json::to_value(&theme).unwrap())
    );
    assert_eq!(
        themes_after_command(&mut server, &host, HostCommand::Theme(None)).await,
        declared(&["block:a"], &json!({})),
        "one empty theme undoes the declaration"
    );
    assert_eq!(
        themes_after_command(&mut server, &host, HostCommand::Theme(None)).await,
        []
    );
    host.commands
        .send(HostCommand::CreateSession {
            label: None,
            shell: None,
        })
        .unwrap();
    let created = server.themes_until("session.create").await.1;
    assert_eq!(
        created["initial_windows"][0]["layout"]["block"].get("options"),
        None
    );
    drop(host);
}

#[tokio::test]
async fn follow_server_declares_nothing_and_records_no_declaration() {
    let mut server = ThemeServer::start(&[("block:a", None)], None).await;
    let host = HostConnection::spawn(server.endpoint.clone(), None, None, None, true, None);
    let (themes, _) = server
        .themes_until("com.superlogical.terminal.program_status")
        .await;
    assert_eq!(themes, []);
    server.owner("block:a", Some("client:test"));
    assert_eq!(server.themes_after_events().await, []);
    host.commands.send(HostCommand::Reconnect).unwrap();
    server.themes_until("session.bootstrap").await;
    let (themes, _) = server
        .themes_until("com.superlogical.terminal.program_status")
        .await;
    assert_eq!(themes, []);
    assert!(!host.model.lock().unwrap().theme_declared);
    drop(host);
}

/// A client that once declared a theme follows the server with an empty declaration.
#[tokio::test]
async fn a_reset_goes_to_each_eligible_block_once_per_connection() {
    let mut server = ThemeServer::start(
        &[("block:a", None), ("block:b", Some("client:other"))],
        None,
    )
    .await;
    let reset = SetTheme::new();
    let empty = json!({});
    let host = HostConnection::spawn(
        server.endpoint.clone(),
        None,
        None,
        None,
        true,
        Some(reset.clone()),
    );
    let (themes, _) = server
        .themes_until("com.superlogical.terminal.program_status")
        .await;
    assert_eq!(
        themes,
        declared(&["block:a"], &empty),
        "not another client's"
    );
    assert!(host.model.lock().unwrap().theme_declared);
    assert_eq!(
        themes_after_command(&mut server, &host, HostCommand::Theme(Some(reset.clone()))).await,
        [],
        "once per block on a connection"
    );
    server.owner("block:b", Some("client:test"));
    assert_eq!(
        server.themes_after_events().await,
        declared(&["block:b"], &empty),
        "taking a block resets it"
    );
    server.owner("block:b", Some("client:other"));
    server.owner("block:b", Some("client:test"));
    assert_eq!(server.themes_after_events().await, []);

    host.commands.send(HostCommand::Reconnect).unwrap();
    server.themes_until("session.bootstrap").await;
    let (themes, _) = server
        .themes_until("com.superlogical.terminal.program_status")
        .await;
    assert_eq!(
        themes,
        declared(&["block:a"], &empty),
        "again after reconnect"
    );
    server.owner("block:b", Some("client:test"));
    assert_eq!(
        server.themes_after_events().await,
        declared(&["block:b"], &empty)
    );
    drop(host);
}

#[tokio::test]
async fn themes_are_declared_again_after_reconnect_with_the_latest_choice() {
    let mut server = ThemeServer::start(&[("block:a", None)], None).await;
    let first = SetTheme::new().background("#111111");
    let host = HostConnection::spawn(
        server.endpoint.clone(),
        None,
        None,
        None,
        true,
        Some(first.clone()),
    );
    let (themes, _) = server
        .themes_until("com.superlogical.terminal.program_status")
        .await;
    assert_eq!(
        themes,
        declared(&["block:a"], &serde_json::to_value(&first).unwrap())
    );
    host.commands.send(HostCommand::Reconnect).unwrap();
    // Changed while disconnected: the next connection declares it, and only it.
    let second = SetTheme::new().background("#222222");
    host.commands
        .send(HostCommand::Theme(Some(second.clone())))
        .unwrap();
    host.commands.send(HostCommand::Reconnect).unwrap();
    server.themes_until("session.bootstrap").await;
    let (themes, _) = server
        .themes_until("com.superlogical.terminal.program_status")
        .await;
    assert_eq!(
        themes,
        declared(&["block:a"], &serde_json::to_value(&second).unwrap())
    );
    drop(host);
}

#[tokio::test]
async fn a_server_without_set_theme_is_told_once_and_never_again() {
    let mut server = ThemeServer::start(
        &[("block:a", None), ("block:b", None)],
        Some("method_not_found"),
    )
    .await;
    let host = HostConnection::spawn(
        server.endpoint.clone(),
        None,
        None,
        None,
        true,
        Some(SetTheme::new().background("#111111")),
    );
    let (themes, _) = server
        .themes_until("com.superlogical.terminal.program_status")
        .await;
    assert_eq!(themes.len(), 1, "the first refusal stops declaring");
    {
        let m = host.model.lock().unwrap();
        assert!(m.themes_unsupported);
        assert!(
            m.notice
                .as_deref()
                .is_some_and(|n| n.contains("terminal themes"))
        );
    }
    let other = SetTheme::new().background("#333333");
    assert_eq!(
        themes_after_command(&mut server, &host, HostCommand::Theme(Some(other))).await,
        []
    );
    host.commands
        .send(HostCommand::CreateSession {
            label: None,
            shell: None,
        })
        .unwrap();
    let created = server.themes_until("session.create").await.1;
    assert_eq!(
        created["initial_windows"][0]["layout"]["block"].get("options"),
        None,
        "new blocks carry no theme either"
    );
    drop(host);
}

#[tokio::test]
async fn searching_needs_no_server_while_the_host_reconnects() {
    // Nothing listens here, so the host stays between attempts.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let host = HostConnection::spawn(endpoint, None, None, None, true, None);
    let send = |command| {
        host.commands
            .send(HostCommand::Terminal {
                session: "s".into(),
                block: "b".into(),
                command,
            })
            .unwrap();
    };
    let notice = || host.model.lock().unwrap().notice.clone();
    tokio::time::sleep(Duration::from_millis(200)).await;
    send(Command::Search("needle".into()));
    send(Command::SearchStep(true));
    send(Command::Search(String::new()));
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(notice(), None, "a search is this client's own");

    // Input still isn't taken for later.
    send(Command::Reconnect);
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        notice().is_some_and(|n| n.contains("not sent while disconnected")),
        "{:?}",
        notice()
    );
    drop(host);
}

#[tokio::test]
async fn a_worker_gets_its_search_whatever_the_control_connection_is_doing() {
    let (commands, received) = flume::unbounded();
    let workers = HashMap::from([(
        "b".to_owned(),
        PaneWorker {
            session: "s".into(),
            commands,
            task: tokio::spawn(async {}),
            size: None,
        },
    )]);
    assert!(forward_search(
        &workers,
        "b",
        &Command::Search("needle".into())
    ));
    assert!(forward_search(&workers, "b", &Command::SearchStep(false)));
    assert!(
        forward_search(&workers, "gone", &Command::SearchStep(false)),
        "a closed pane's search goes nowhere, and needs no notice"
    );
    assert!(!forward_search(&workers, "b", &Command::Reconnect));
    assert!(matches!(
        received.try_recv(),
        Ok(DataCommand::Search(needle)) if needle == "needle"
    ));
    assert!(matches!(
        received.try_recv(),
        Ok(DataCommand::SearchStep(false))
    ));
    assert!(received.try_recv().is_err());
}
