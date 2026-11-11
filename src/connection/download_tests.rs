//! Downloads against a mock Rex: a control server that answers `download_begin`, and a data
//! server that plays one stream. The real server is exercised by `rex-download-probe`.
use super::*;
use crate::download::{
    self, Config, Confirmation, Failure, Limits, Reason, State,
    tests::{Temp, names},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, frame::coding::CloseCode};

const TICKET: &str = "AAAAAAAAAAAAAAAAAAAAAA";

/// What the data server plays after the WELCOME.
#[derive(Clone)]
enum Step {
    Data(Vec<u8>),
    /// The empty message that ends a stream.
    End,
    Close(u16, &'static str),
    Text,
    /// Hang up without a Close frame.
    Hangup,
    /// Wait for the client to end the stream, within the test's patience.
    Hold,
}

#[derive(Debug, PartialEq, Eq)]
enum Ended {
    /// The client sent a Close frame with this code.
    ClientClose(Option<u16>),
    /// The client dropped the connection.
    ClientGone,
    /// The server hung up itself.
    ServerHangup,
    /// Nobody ended it.
    Nobody,
}

async fn serve_data(listener: &TcpListener, steps: Vec<Step>) -> (Vec<u8>, Ended) {
    let mut ws = accept(listener, crate::upload::PROTOCOL).await;
    let hello = ws.next().await.unwrap().unwrap().into_data().to_vec();
    let mut welcome = vec![0; 32];
    welcome[0] = 0x81;
    welcome[24] = 1;
    ws.send(Message::Binary(welcome.into())).await.unwrap();
    for step in steps {
        match step {
            Step::Data(bytes) => ws.send(Message::Binary(bytes.into())).await.unwrap(),
            Step::End => ws.send(Message::Binary(Vec::new().into())).await.unwrap(),
            Step::Close(code, reason) => ws
                .send(Message::Close(Some(CloseFrame {
                    code: CloseCode::from(code),
                    reason: reason.into(),
                })))
                .await
                .unwrap(),
            Step::Text => ws.send(Message::Text("hello".into())).await.unwrap(),
            Step::Hangup => return (hello, Ended::ServerHangup),
            Step::Hold => break,
        }
    }
    let ended = tokio::time::timeout(Duration::from_secs(6), async {
        loop {
            match ws.next().await {
                Some(Ok(Message::Close(frame))) => {
                    return Ended::ClientClose(frame.map(|f| u16::from(f.code)));
                }
                Some(Ok(_)) => {}
                Some(Err(_)) | None => return Ended::ClientGone,
            }
        }
    })
    .await
    .unwrap_or(Ended::Nobody);
    (hello, ended)
}

/// Accepts a control connection, completes its welcome and bootstrap.
async fn control(listener: &TcpListener, revision: u32) -> WebSocketStream<TcpStream> {
    let mut ws = accept(listener, "rex.control.v1").await;
    welcome(&mut ws, revision).await;
    let request = recv(&mut ws).await;
    assert_eq!(request["method"], "session.bootstrap");
    send(
        &mut ws,
        json!({"type":"response","id":request["id"],"seq":1,"payload":boot()}),
    )
    .await;
    ws
}

fn reply(request: &Value, payload: Value) -> Value {
    json!({"type":"response","id":request["id"],"seq":2,"payload":payload})
}

fn api_error(request: &Value, code: &str, status: u16, message: &str) -> Value {
    json!({"type":"response","id":request["id"],"error":{"code":code,"message":message,"status":status}})
}

fn ticket_reply(size: u64) -> Value {
    json!({"ticket":TICKET,"expires_in_ms":60000,"path":"/srv/data/file.bin","name":"file.bin",
        "size":size,"modified_at":"2026-11-11T00:00:00Z"})
}

fn download_begin(request: &Value) -> Value {
    assert_eq!(
        request["method"],
        "com.superlogical.terminal.download_begin"
    );
    request["payload"].clone()
}

struct Fixture {
    listener: Arc<TcpListener>,
    endpoint: String,
    temp: Temp,
    dir: PathBuf,
}

impl Fixture {
    async fn new() -> Self {
        let listener = Arc::new(TcpListener::bind("127.0.0.1:0").await.unwrap());
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let temp = Temp::new("conn");
        let dir = temp.dir("out");
        Self {
            listener,
            endpoint,
            temp,
            dir,
        }
    }
    fn config(&self) -> Config {
        Config {
            records: self.temp.records(),
            ..Config::default()
        }
    }
    fn request(&self, config: Config) -> (u64, HostCommand) {
        let id = download::next_id();
        let mut request =
            downloads::Request::new(id, SESSION, "block:1", "some/file.bin", &self.dir);
        request.config = config;
        (id, HostCommand::Download(Box::new(request)))
    }
}

async fn wait_for<T>(
    host: &HostConnection,
    what: &str,
    mut get: impl FnMut(&HostModel) -> Option<T>,
) -> T {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if let Some(found) = get(&host.model.lock().unwrap()) {
            return found;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

async fn finished(host: &HostConnection, id: u64) -> State {
    wait_for(host, "a final state", |m| {
        m.downloads
            .iter()
            .find(|t| t.id == id)
            .filter(|t| t.state.is_final())
            .map(|t| t.state.clone())
    })
    .await
}

async fn state_of(
    host: &HostConnection,
    id: u64,
    what: &str,
    test: impl Fn(&State) -> bool,
) -> State {
    wait_for(host, what, |m| {
        m.downloads
            .iter()
            .find(|t| t.id == id)
            .filter(|t| test(&t.state))
            .map(|t| t.state.clone())
    })
    .await
}

/// Whether the client opened the ticket's stream, got its WELCOME and closed it with 1000 at
/// once, which is how Rex is told to free a file and slot it was holding for the ticket.
async fn released(listener: &TcpListener) -> bool {
    let accepted = tokio::time::timeout(Duration::from_secs(3), async {
        serve_data(listener, vec![Step::Hold]).await
    })
    .await;
    let Ok((hello, ended)) = accepted else {
        return false;
    };
    hello.first() == Some(&1)
        && hello[1..] == URL_SAFE_NO_PAD.decode(TICKET).unwrap()
        && ended == Ended::ClientClose(Some(1000))
}

/// A partial file appears in `dir`.
async fn partial_appears(dir: &Path) -> String {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if let Some(name) = names(dir).into_iter().find(|n| n.ends_with(".rex-part")) {
            return name;
        }
        assert!(Instant::now() < deadline, "no partial file appeared");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

fn chunks(bytes: &[u8]) -> Vec<Step> {
    bytes
        .chunks(32 * 1024)
        .map(|c| Step::Data(c.to_vec()))
        .collect()
}

fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i * 7 + i / 251) as u8).collect()
}

/// A download that plays `steps` on a hint of `hint` bytes. Returns the final state, the server's
/// view of the request and stream, and leaves nothing running.
async fn run(
    hint: u64,
    steps: Vec<Step>,
    configure: impl FnOnce(&mut Config),
) -> (State, Value, (Vec<u8>, Ended), Fixture) {
    let f = Fixture::new().await;
    let mut config = f.config();
    configure(&mut config);
    let (id, command) = f.request(config);
    let listener = f.listener.clone();
    let server = tokio::spawn(async move {
        let mut ws = control(&listener, 13).await;
        let request = recv(&mut ws).await;
        let payload = download_begin(&request);
        send(&mut ws, reply(&request, ticket_reply(hint))).await;
        let data = serve_data(&listener, steps).await;
        (payload, data, ws)
    });
    let host = HostConnection::spawn(f.endpoint.clone(), None, None, None, false, None);
    connected(&host).await;
    host.commands.send(command).unwrap();
    let state = finished(&host, id).await;
    let (payload, data, mut ws) = server.await.unwrap();
    drop(host);
    assert!(matches!(
        ws.next().await,
        Some(Ok(Message::Close(_))) | None
    ));
    (state, payload, data, f)
}

fn assert_clean(f: &Fixture, saved: &[&str]) {
    assert_eq!(names(&f.dir), saved, "nothing but the saved files is left");
    assert!(
        f.temp.records().list().is_empty(),
        "no partial file is still recorded"
    );
}

#[tokio::test]
async fn a_download_sends_the_documented_request_and_saves_the_exact_bytes() {
    let bytes = pattern(100_000);
    let mut steps = chunks(&bytes);
    steps.extend([Step::End, Step::Close(1000, "")]);
    let (state, payload, (hello, ended), f) = run(bytes.len() as u64, steps, |_| {}).await;
    // The request: the pane's block, a fresh lower-case ref, the path as typed, nothing else.
    assert_eq!(payload["session_id"], SESSION);
    assert_eq!(payload["block_id"], "block:1");
    let args = payload["args"].as_object().unwrap();
    assert_eq!(args.keys().collect::<Vec<_>>(), ["path", "ref"]);
    assert_eq!(args["path"], "some/file.bin");
    let reference = args["ref"].as_str().unwrap();
    let uuid = reference.strip_prefix("download:").unwrap();
    assert_eq!(uuid.len(), 36);
    assert_eq!(uuid, uuid.to_lowercase());
    assert!(!payload.to_string().contains("if_none_match"));
    // The stream: HELLO with the ticket's 16 bytes.
    assert_eq!(hello[0], 1);
    assert_eq!(&hello[1..], URL_SAFE_NO_PAD.decode(TICKET).unwrap());
    // The client answered the server's Close, which ends the stream on the server's side.
    assert!(
        matches!(ended, Ended::ClientGone | Ended::ClientClose(Some(1000))),
        "{ended:?}"
    );
    let State::Saved { path, bytes: n, .. } = state else {
        panic!("{state:?}")
    };
    assert_eq!(n, 100_000);
    assert_eq!(path, f.dir.join("file.bin"));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_clean(&f, &["file.bin"]);
}

#[tokio::test]
async fn an_empty_file_and_a_hostile_name_save_safely() {
    let f = Fixture::new().await;
    let (id, command) = f.request(f.config());
    let listener = f.listener.clone();
    let server = tokio::spawn(async move {
        let mut ws = control(&listener, 13).await;
        let request = recv(&mut ws).await;
        let mut payload = ticket_reply(0);
        payload["name"] = json!("..\\..\\.bash\u{202e}rc\n. ");
        send(&mut ws, reply(&request, payload)).await;
        let data = serve_data(&listener, vec![Step::End, Step::Close(1000, "")]).await;
        (data, ws)
    });
    let host = HostConnection::spawn(f.endpoint.clone(), None, None, None, false, None);
    connected(&host).await;
    host.commands.send(command).unwrap();
    let state = finished(&host, id).await;
    let State::Saved { path, bytes, .. } = state else {
        panic!("{state:?}")
    };
    assert_eq!(bytes, 0);
    assert_eq!(path, f.dir.join("_bashrc"));
    assert_eq!(std::fs::read(&path).unwrap(), b"");
    let transfer = host.model.lock().unwrap().downloads[0].clone();
    assert_eq!(transfer.name.as_deref(), Some("_bashrc"));
    assert_eq!(transfer.resolved.as_deref(), Some("/srv/data/file.bin"));
    drop(host);
    server.await.unwrap();
    assert_clean(&f, &["_bashrc"]);
}

#[tokio::test]
async fn a_stream_shorter_than_the_hint_is_kept_and_says_so() {
    // Rex ends a stream at the file's end whatever its size said (sysfs reports 4096 for a
    // 6-byte file), so a normal end is a complete file that is shorter than the hint.
    let bytes = pattern(5_000);
    let mut steps = chunks(&bytes);
    steps.extend([Step::End, Step::Close(1000, "")]);
    let (state, _, _, f) = run(9_000, steps, |_| {}).await;
    let State::Saved {
        path,
        bytes: n,
        short_of,
    } = state
    else {
        panic!("{state:?}")
    };
    assert_eq!((n, short_of), (5_000, Some(9_000)));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_clean(&f, &["file.bin"]);
}

#[tokio::test]
async fn a_stream_past_the_growth_cap_fails_and_one_within_it_succeeds() {
    let limits = Limits {
        growth: 1_000,
        ..Limits::default()
    };
    // Hint 1000: the cap is 2000.
    let mut steps = chunks(&pattern(2_500));
    steps.push(Step::Hold);
    let (state, _, (_, ended), f) = run(1_000, steps, |c| c.limits = limits).await;
    assert_eq!(state, State::Failed(Failure::TooMuchData { limit: 2_000 }));
    assert!(
        matches!(ended, Ended::ClientGone | Ended::ClientClose(_)),
        "{ended:?}"
    );
    assert_clean(&f, &[]);

    // A file that grew a little is saved as it arrived.
    let mut steps = chunks(&pattern(1_800));
    steps.extend([Step::End, Step::Close(1000, "")]);
    let (state, _, _, f) = run(1_000, steps, |c| c.limits = limits).await;
    let State::Saved { path, bytes, .. } = state else {
        panic!("{state:?}")
    };
    assert_eq!(bytes, 1_800);
    assert_eq!(std::fs::read(path).unwrap(), pattern(1_800));
    assert_clean(&f, &["file.bin"]);
}

#[tokio::test]
async fn an_ending_that_is_not_the_normal_one_fails() {
    let data = || chunks(&pattern(1_000));
    let cases: Vec<(Vec<Step>, Failure)> = vec![
        (
            [data(), vec![Step::Close(1011, "data provider failed")]].concat(),
            Failure::Closed {
                code: Some(1011),
                reason: "data provider failed".into(),
            },
        ),
        (
            [data(), vec![Step::Close(1001, "data provider canceled")]].concat(),
            Failure::Closed {
                code: Some(1001),
                reason: "data provider canceled".into(),
            },
        ),
        (
            [data(), vec![Step::Close(4000, "block closed")]].concat(),
            Failure::Closed {
                code: Some(4000),
                reason: "block closed".into(),
            },
        ),
        // The reason is the server's text: cleaned before anyone shows it.
        (
            [
                data(),
                vec![Step::Close(1011, "bad\n\u{1b}[0m\u{202e}reason")],
            ]
            .concat(),
            Failure::Closed {
                code: Some(1011),
                reason: "bad  [0mreason".into(),
            },
        ),
        // The end marker and then something other than 1000.
        (
            [data(), vec![Step::End, Step::Close(1011, "")]].concat(),
            Failure::Closed {
                code: Some(1011),
                reason: String::new(),
            },
        ),
        // 1000 with no end marker is not success.
        (
            [data(), vec![Step::Close(1000, "")]].concat(),
            Failure::Protocol("the stream closed without its end marker".into()),
        ),
        // Data after the end marker.
        (
            [data(), vec![Step::End], data()].concat(),
            Failure::Protocol("data after the end of the file".into()),
        ),
        (
            [data(), vec![Step::Text]].concat(),
            Failure::Protocol("a text message".into()),
        ),
    ];
    for (steps, expected) in cases {
        let mut steps = steps;
        steps.push(Step::Hold);
        let (state, _, _, f) = run(1_000, steps, |_| {}).await;
        assert_eq!(state, State::Failed(expected.clone()), "{expected:?}");
        assert_clean(&f, &[]);
    }
    // The server hangs up in the middle without a Close.
    let (state, _, _, f) = run(1_000, [data(), vec![Step::Hangup]].concat(), |_| {}).await;
    assert!(
        matches!(
            state,
            State::Failed(Failure::Closed { code: None, .. } | Failure::Connection(_))
        ),
        "{state:?}"
    );
    assert_clean(&f, &[]);
}

#[tokio::test]
async fn a_stalled_stream_fails_after_the_timeout_and_is_dropped() {
    let mut steps = chunks(&pattern(3_000));
    steps.push(Step::Hold);
    let started = Instant::now();
    let (state, _, (_, ended), f) = run(10_000, steps, |c| {
        c.limits.stall = Duration::from_millis(300)
    })
    .await;
    assert_eq!(state, State::Failed(Failure::Stalled));
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(
        matches!(ended, Ended::ClientGone | Ended::ClientClose(_)),
        "the socket is closed: {ended:?}"
    );
    assert_clean(&f, &[]);
}

#[tokio::test]
async fn cancel_closes_the_socket_and_removes_the_partial_file() {
    let f = Fixture::new().await;
    let (id, command) = f.request(f.config());
    let listener = f.listener.clone();
    let server = tokio::spawn(async move {
        let mut ws = control(&listener, 13).await;
        let request = recv(&mut ws).await;
        send(&mut ws, reply(&request, ticket_reply(1_000_000))).await;
        let mut steps = chunks(&pattern(100_000));
        steps.push(Step::Hold);
        (serve_data(&listener, steps).await, ws)
    });
    let host = HostConnection::spawn(f.endpoint.clone(), None, None, None, false, None);
    connected(&host).await;
    host.commands.send(command).unwrap();
    partial_appears(&f.dir).await;
    state_of(&host, id, "bytes", |s| matches!(s, State::Receiving { .. })).await;
    host.commands.send(HostCommand::DownloadCancel(id)).unwrap();
    assert_eq!(finished(&host, id).await, State::Cancelled);
    let ((_, ended), _ws) = server.await.unwrap();
    assert_eq!(
        ended,
        Ended::ClientClose(Some(1000)),
        "the data socket was closed"
    );
    assert_clean(&f, &[]);
    // Cancelling again, or an unknown transfer, does nothing.
    host.commands.send(HostCommand::DownloadCancel(id)).unwrap();
    host.commands
        .send(HostCommand::DownloadCancel(u64::MAX))
        .unwrap();
    host.commands
        .send(HostCommand::DownloadConfirm(u64::MAX))
        .unwrap();
    drop(host);
}

#[tokio::test]
async fn a_reconnect_mid_transfer_fails_it_and_sends_nothing_again() {
    let f = Fixture::new().await;
    let (id, command) = f.request(f.config());
    let (late_id, late_command) = f.request(f.config());
    let listener = f.listener.clone();
    let server = tokio::spawn(async move {
        let mut first = control(&listener, 13).await;
        let request = recv(&mut first).await;
        send(&mut first, reply(&request, ticket_reply(1_000_000))).await;
        let mut steps = chunks(&pattern(100_000));
        steps.push(Step::Hold);
        let (_, ended) = serve_data(&listener, steps).await;
        // The reconnected control connection sees a bootstrap and nothing else.
        let mut second = control(&listener, 13).await;
        let extra = tokio::time::timeout(Duration::from_millis(600), second.next()).await;
        (ended, extra.is_err(), first, second)
    });
    let host = HostConnection::spawn(f.endpoint.clone(), None, None, None, false, None);
    connected(&host).await;
    host.commands.send(command).unwrap();
    partial_appears(&f.dir).await;
    host.commands.send(HostCommand::Reconnect).unwrap();
    assert_eq!(
        finished(&host, id).await,
        State::Failed(Failure::ConnectionLost)
    );
    // While disconnected a new request is refused, not queued for later.
    host.commands.send(late_command).unwrap();
    assert_eq!(
        finished(&host, late_id).await,
        State::Failed(Failure::Disconnected)
    );
    host.commands.send(HostCommand::Reconnect).unwrap();
    let (ended, quiet, _first, _second) = server.await.unwrap();
    assert!(
        matches!(ended, Ended::ClientGone | Ended::ClientClose(_)),
        "{ended:?}"
    );
    assert!(quiet, "nothing was sent on the new connection");
    assert_clean(&f, &[]);
    drop(host);
}

#[tokio::test]
async fn a_reconnect_while_a_question_is_open_fails_it_without_touching_the_ticket() {
    // No host-side sweep fails it: its own task sees the control connection go away. Rex has
    // revoked the ticket with that connection, so nothing is opened on the data endpoint.
    let f = Fixture::new().await;
    let mut config = f.config();
    config.limits.confirm_at = 1_000;
    let (id, command) = f.request(config);
    let listener = f.listener.clone();
    let server = tokio::spawn(async move {
        let mut first = control(&listener, 13).await;
        let request = recv(&mut first).await;
        send(&mut first, reply(&request, ticket_reply(5_000))).await;
        let second = control(&listener, 13).await;
        let extra = tokio::time::timeout(Duration::from_millis(600), listener.accept()).await;
        (extra.is_err(), first, second)
    });
    let host = HostConnection::spawn(f.endpoint.clone(), None, None, None, false, None);
    connected(&host).await;
    host.commands.send(command).unwrap();
    state_of(&host, id, "the question", |s| {
        matches!(s, State::AwaitingConfirmation(_))
    })
    .await;
    host.commands.send(HostCommand::Reconnect).unwrap();
    assert_eq!(
        finished(&host, id).await,
        State::Failed(Failure::ConnectionLost)
    );
    assert!(server.await.unwrap().0, "no data connection was opened");
    drop(host);
    assert_clean(&f, &[]);
}

#[tokio::test]
async fn an_old_revision_never_sends_the_request() {
    for revision in [10, 11, 12] {
        let f = Fixture::new().await;
        let (id, command) = f.request(f.config());
        let listener = f.listener.clone();
        let server = tokio::spawn(async move {
            let mut ws = control(&listener, revision).await;
            let extra = tokio::time::timeout(Duration::from_millis(400), ws.next()).await;
            (extra.is_err(), ws)
        });
        let host = HostConnection::spawn(f.endpoint.clone(), None, None, None, false, None);
        connected(&host).await;
        host.commands.send(command).unwrap();
        assert_eq!(
            finished(&host, id).await,
            State::Failed(Failure::ServerTooOld),
            "revision {revision}"
        );
        let (quiet, _ws) = server.await.unwrap();
        assert!(quiet, "revision {revision}: nothing was sent");
        drop(host);
    }
}

#[tokio::test]
async fn rex_refusing_the_request_is_shown_in_its_words_and_nothing_is_streamed() {
    let cases = [
        (
            "not_found",
            404,
            "not found: /x does not exist",
            Failure::Server("not found: /x does not exist".into()),
        ),
        (
            "bad_request",
            400,
            "not a regular file: /x is a directory",
            Failure::Server("not a regular file: /x is a directory".into()),
        ),
        (
            "conflict",
            409,
            "conflict: 16 downloads are already reserved, the most a terminal allows",
            Failure::Server(
                "conflict: 16 downloads are already reserved, the most a terminal allows".into(),
            ),
        ),
        (
            "revision_too_low",
            400,
            "needs API revision 13",
            Failure::ServerTooOld,
        ),
        (
            "method_not_found",
            404,
            "method not found",
            Failure::ServerTooOld,
        ),
    ];
    for (code, status, message, expected) in cases {
        let f = Fixture::new().await;
        let (id, command) = f.request(f.config());
        let listener = f.listener.clone();
        let server = tokio::spawn(async move {
            let mut ws = control(&listener, 13).await;
            let request = recv(&mut ws).await;
            send(&mut ws, api_error(&request, code, status, message)).await;
            let extra = tokio::time::timeout(Duration::from_millis(300), listener.accept()).await;
            (extra.is_err(), ws)
        });
        let host = HostConnection::spawn(f.endpoint.clone(), None, None, None, false, None);
        connected(&host).await;
        host.commands.send(command).unwrap();
        assert_eq!(finished(&host, id).await, State::Failed(expected), "{code}");
        let (no_stream, _ws) = server.await.unwrap();
        assert!(no_stream, "{code}: no data connection was opened");
        // The host stays connected: a refusal is not a broken connection.
        assert!(host.model.lock().unwrap().connected);
        drop(host);
        assert_clean(&f, &[]);
    }
}

#[tokio::test]
async fn an_undecodable_reply_or_an_oversized_request_fails_only_that_transfer() {
    for case in ["no size", "huge path"] {
        let f = Fixture::new().await;
        let (id, mut command) = f.request(f.config());
        if case == "huge path" {
            let HostCommand::Download(request) = &mut command else {
                unreachable!()
            };
            // The mock server's message limit is 64 KiB.
            request.path = "x".repeat(200_000);
        }
        let listener = f.listener.clone();
        let server = tokio::spawn(async move {
            let mut ws = control(&listener, 13).await;
            if case == "no size" {
                let request = recv(&mut ws).await;
                let mut payload = ticket_reply(1);
                payload.as_object_mut().unwrap().remove("size");
                send(&mut ws, reply(&request, payload)).await;
            }
            // The control connection must still be the one in use.
            let extra = tokio::time::timeout(Duration::from_millis(500), listener.accept()).await;
            (extra.is_err(), ws)
        });
        let host = HostConnection::spawn(f.endpoint.clone(), None, None, None, false, None);
        connected(&host).await;
        let epoch = host.model.lock().unwrap().connection_epoch;
        host.commands.send(command).unwrap();
        let state = finished(&host, id).await;
        let State::Failed(failure) = &state else {
            panic!("{case}: {state:?}")
        };
        assert!(
            !matches!(failure, Failure::ConnectionLost),
            "{case}: {failure:?}"
        );
        let (kept, _ws) = server.await.unwrap();
        assert!(kept, "{case}: the host did not reconnect");
        let model = host.model.lock().unwrap();
        assert!(model.connected, "{case}");
        assert_eq!(model.connection_epoch, epoch, "{case}");
        drop(model);
        drop(host);
        assert_clean(&f, &[]);
    }
}

#[tokio::test]
async fn long_server_messages_are_cut_and_cleaned() {
    let f = Fixture::new().await;
    let (id, command) = f.request(f.config());
    let listener = f.listener.clone();
    let server = tokio::spawn(async move {
        let mut ws = control(&listener, 13).await;
        let request = recv(&mut ws).await;
        let message = format!(
            "not found: {}\n\u{1b}[31m does not exist",
            "p".repeat(5_000)
        );
        send(&mut ws, api_error(&request, "not_found", 404, &message)).await;
        ws
    });
    let host = HostConnection::spawn(f.endpoint.clone(), None, None, None, false, None);
    connected(&host).await;
    host.commands.send(command).unwrap();
    let State::Failed(Failure::Server(message)) = finished(&host, id).await else {
        panic!()
    };
    assert_eq!(message.chars().count(), 301);
    assert!(!message.contains(['\n', '\u{1b}']));
    let _ws = server.await.unwrap();
    drop(host);
}

#[tokio::test]
async fn a_reply_without_a_ticket_or_with_not_modified_fails_without_streaming() {
    let replies = [
        json!({"path":"/p","name":"f","size":1,"modified_at":"x"}),
        json!({"not_modified":true,"path":"/p","name":"f","size":1,"modified_at":"x","version":"v1"}),
        json!({"ticket":TICKET,"not_modified":true,"path":"/p","name":"f","size":1,"modified_at":"x"}),
    ];
    for payload in replies {
        let f = Fixture::new().await;
        let (id, command) = f.request(f.config());
        let listener = f.listener.clone();
        let server = tokio::spawn(async move {
            let mut ws = control(&listener, 13).await;
            let request = recv(&mut ws).await;
            download_begin(&request);
            send(&mut ws, reply(&request, payload)).await;
            let extra = tokio::time::timeout(Duration::from_millis(300), listener.accept()).await;
            (extra.is_err(), ws)
        });
        let host = HostConnection::spawn(f.endpoint.clone(), None, None, None, false, None);
        connected(&host).await;
        host.commands.send(command).unwrap();
        let state = finished(&host, id).await;
        assert!(
            matches!(state, State::Failed(Failure::UnexpectedReply(_))),
            "{state:?}"
        );
        let (no_stream, _ws) = server.await.unwrap();
        assert!(no_stream);
        drop(host);
        assert_clean(&f, &[]);
    }
}

#[tokio::test]
async fn a_symlinked_destination_is_refused_before_anything_is_sent() {
    let f = Fixture::new().await;
    let link = f.temp.0.join("link");
    std::os::unix::fs::symlink(&f.dir, &link).unwrap();
    let id = download::next_id();
    let mut request = downloads::Request::new(id, SESSION, "block:1", "file", &link);
    request.config = f.config();
    let listener = f.listener.clone();
    let server = tokio::spawn(async move {
        let mut ws = control(&listener, 13).await;
        let extra = tokio::time::timeout(Duration::from_millis(400), ws.next()).await;
        (extra.is_err(), ws)
    });
    let host = HostConnection::spawn(f.endpoint.clone(), None, None, None, false, None);
    connected(&host).await;
    host.commands
        .send(HostCommand::Download(Box::new(request)))
        .unwrap();
    let state = finished(&host, id).await;
    let State::Failed(Failure::Destination(message)) = state else {
        panic!("{state:?}")
    };
    assert!(message.contains("symbolic link"), "{message}");
    assert!(server.await.unwrap().0, "no request was sent");
    drop(host);
}

#[tokio::test]
async fn a_large_file_waits_for_a_yes_and_streams_only_after_it() {
    let f = Fixture::new().await;
    let mut config = f.config();
    config.limits.confirm_at = 1_000;
    let (id, command) = f.request(config);
    let listener = f.listener.clone();
    let (confirmed_tx, confirmed) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        let mut ws = control(&listener, 13).await;
        let request = recv(&mut ws).await;
        send(&mut ws, reply(&request, ticket_reply(5_000))).await;
        // No stream until the person says yes.
        let early = tokio::time::timeout(Duration::from_millis(400), listener.accept()).await;
        assert!(
            early.is_err(),
            "a data connection was opened before the confirmation"
        );
        confirmed_tx.send(()).ok();
        let mut steps = chunks(&pattern(5_000));
        steps.extend([Step::End, Step::Close(1000, "")]);
        (serve_data(&listener, steps).await, ws)
    });
    let host = HostConnection::spawn(f.endpoint.clone(), None, None, None, false, None);
    connected(&host).await;
    host.commands.send(command).unwrap();
    let state = state_of(&host, id, "the question", |s| {
        matches!(s, State::AwaitingConfirmation(_))
    })
    .await;
    let State::AwaitingConfirmation(Confirmation { reason, hint, .. }) = state else {
        unreachable!()
    };
    assert_eq!((reason, hint), (Reason::Large, 5_000));
    assert!(names(&f.dir).is_empty(), "no partial file before the yes");
    tokio::time::sleep(Duration::from_millis(450)).await;
    host.commands
        .send(HostCommand::DownloadConfirm(id))
        .unwrap();
    confirmed.await.unwrap();
    let State::Saved { path, bytes, .. } = finished(&host, id).await else {
        panic!()
    };
    assert_eq!(bytes, 5_000);
    assert_eq!(std::fs::read(path).unwrap(), pattern(5_000));
    let _ = server.await.unwrap();
    drop(host);
}

#[tokio::test]
async fn cancelling_or_ignoring_the_question_frees_the_ticket_without_taking_data() {
    for answer in ["cancel", "expire"] {
        let f = Fixture::new().await;
        let mut config = f.config();
        config.limits.confirm_at = 1_000;
        let (id, command) = f.request(config);
        let listener = f.listener.clone();
        let server = tokio::spawn(async move {
            let mut ws = control(&listener, 13).await;
            let request = recv(&mut ws).await;
            let mut payload = ticket_reply(5_000);
            if answer == "expire" {
                // Five seconds of margin are kept: this leaves 150 ms.
                payload["expires_in_ms"] = json!(5_150);
            }
            send(&mut ws, reply(&request, payload)).await;
            (released(&listener).await, ws)
        });
        let host = HostConnection::spawn(f.endpoint.clone(), None, None, None, false, None);
        connected(&host).await;
        host.commands.send(command).unwrap();
        state_of(&host, id, "the question", |s| {
            matches!(s, State::AwaitingConfirmation(_))
        })
        .await;
        if answer == "cancel" {
            host.commands.send(HostCommand::DownloadCancel(id)).unwrap();
        }
        let state = finished(&host, id).await;
        match answer {
            "cancel" => assert_eq!(state, State::Cancelled),
            _ => assert_eq!(state, State::Failed(Failure::ConfirmationExpired)),
        }
        assert!(server.await.unwrap().0, "{answer}: the ticket was released");
        drop(host);
        assert_clean(&f, &[]);
    }
}

#[tokio::test]
async fn a_large_file_is_confirmed_even_when_free_space_cannot_be_read() {
    let f = Fixture::new().await;
    let mut config = f.config();
    config.limits.confirm_at = 1_000;
    config.space = Some(Arc::new(|| -> std::io::Result<u64> {
        Err(std::io::Error::other("no statvfs"))
    }));
    let (id, command) = f.request(config);
    let listener = f.listener.clone();
    let server = tokio::spawn(async move {
        let mut ws = control(&listener, 13).await;
        let request = recv(&mut ws).await;
        send(&mut ws, reply(&request, ticket_reply(5_000))).await;
        (released(&listener).await, ws)
    });
    let host = HostConnection::spawn(f.endpoint.clone(), None, None, None, false, None);
    connected(&host).await;
    host.commands.send(command).unwrap();
    let state = state_of(&host, id, "the question", |s| {
        matches!(s, State::AwaitingConfirmation(_))
    })
    .await;
    let State::AwaitingConfirmation(question) = state else {
        unreachable!()
    };
    assert_eq!(
        (question.reason, question.hint, question.free),
        (Reason::Large, 5_000, None)
    );
    host.commands.send(HostCommand::DownloadCancel(id)).unwrap();
    assert_eq!(finished(&host, id).await, State::Cancelled);
    assert!(server.await.unwrap().0);
    drop(host);
}

#[tokio::test]
async fn cancelling_before_the_stream_is_open_frees_the_ticket() {
    // The first connection never gets its WELCOME; the cancel lands while it is opening.
    let f = Fixture::new().await;
    let (id, command) = f.request(f.config());
    let listener = f.listener.clone();
    let server = tokio::spawn(async move {
        let mut ws = control(&listener, 13).await;
        let request = recv(&mut ws).await;
        send(&mut ws, reply(&request, ticket_reply(1_000))).await;
        let mut stuck = accept(&listener, crate::upload::PROTOCOL).await;
        let _hello = stuck.next().await;
        (released(&listener).await, ws, stuck)
    });
    let host = HostConnection::spawn(f.endpoint.clone(), None, None, None, false, None);
    connected(&host).await;
    host.commands.send(command).unwrap();
    // Receiving{0, hint} while the stream opens.
    state_of(&host, id, "receiving", |s| {
        matches!(s, State::Receiving { .. })
    })
    .await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    host.commands.send(HostCommand::DownloadCancel(id)).unwrap();
    assert_eq!(finished(&host, id).await, State::Cancelled);
    let (released, _ws, _stuck) = server.await.unwrap();
    assert!(released, "the ticket was released");
    drop(host);
    assert_clean(&f, &[]);
}

#[tokio::test]
async fn too_little_free_space_stops_it_before_and_during_the_stream() {
    // Before: under the reserve, a question can't help, so the ticket is freed and no data is taken.
    let f = Fixture::new().await;
    let mut config = f.config();
    config.space = Some(Arc::new(|| -> std::io::Result<u64> {
        Ok(100 * 1024 * 1024)
    }));
    let (id, command) = f.request(config);
    let listener = f.listener.clone();
    let server = tokio::spawn(async move {
        let mut ws = control(&listener, 13).await;
        let request = recv(&mut ws).await;
        send(&mut ws, reply(&request, ticket_reply(1_000))).await;
        (released(&listener).await, ws)
    });
    let host = HostConnection::spawn(f.endpoint.clone(), None, None, None, false, None);
    connected(&host).await;
    host.commands.send(command).unwrap();
    assert_eq!(
        finished(&host, id).await,
        State::Failed(Failure::LowSpace {
            free: 100 * 1024 * 1024
        })
    );
    assert!(server.await.unwrap().0, "the ticket was released");
    drop(host);

    // During: the disk fills while the file arrives.
    let calls = Arc::new(AtomicU64::new(0));
    let seen = calls.clone();
    let mut steps = chunks(&pattern(20_000));
    steps.push(Step::Hold);
    let (state, _, (_, ended), f) = run(1_000_000, steps, move |c| {
        c.limits.check_every = 4_000;
        c.space = Some(Arc::new(move || -> std::io::Result<u64> {
            // The first look is the start; later ones see the disk nearly full.
            Ok(if seen.fetch_add(1, Ordering::Relaxed) == 0 {
                10 * 1024 * 1024 * 1024
            } else {
                1024
            })
        }));
    })
    .await;
    assert_eq!(state, State::Failed(Failure::LowSpace { free: 1024 }));
    assert!(
        matches!(ended, Ended::ClientGone | Ended::ClientClose(_)),
        "{ended:?}"
    );
    assert_clean(&f, &[]);
}

#[tokio::test]
async fn a_fourth_download_for_a_host_is_refused() {
    let f = Fixture::new().await;
    let mut config = f.config();
    config.limits.confirm_at = 1;
    let listener = f.listener.clone();
    let server = tokio::spawn(async move {
        let mut ws = control(&listener, 13).await;
        let mut begins = 0;
        // Three requests are answered; a fourth that arrived would be answered too, and counted.
        while let Ok(message) = tokio::time::timeout(Duration::from_millis(700), ws.next()).await {
            let request: Value =
                serde_json::from_str(message.unwrap().unwrap().to_text().unwrap()).unwrap();
            if request["method"] == "com.superlogical.terminal.download_begin" {
                begins += 1;
                send(&mut ws, reply(&request, ticket_reply(10))).await;
            }
        }
        (begins, ws)
    });
    let host = HostConnection::spawn(f.endpoint.clone(), None, None, None, false, None);
    connected(&host).await;
    let mut ids = Vec::new();
    for _ in 0..4 {
        let (id, command) = f.request(config.clone());
        ids.push(id);
        host.commands.send(command).unwrap();
    }
    assert_eq!(
        finished(&host, ids[3]).await,
        State::Failed(Failure::TooManyActive { limit: 3 })
    );
    for id in &ids[..3] {
        state_of(&host, *id, "the question", |s| {
            matches!(s, State::AwaitingConfirmation(_))
        })
        .await;
    }
    // A finished one frees a place.
    host.commands
        .send(HostCommand::DownloadCancel(ids[0]))
        .unwrap();
    assert_eq!(finished(&host, ids[0]).await, State::Cancelled);
    let (begins, _ws) = server.await.unwrap();
    assert_eq!(begins, 3, "the fourth request was never sent");
    drop(host);
}
