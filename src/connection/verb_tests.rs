//! `Command::Reset` and `Command::Clear`: one `terminal.reset` or `terminal.clear` to the block
//! named, with no claim, and never a second one after a refusal or while disconnected.
use super::*;

const RESET: &str = "com.superlogical.terminal.reset";
const CLEAR: &str = "com.superlogical.terminal.clear";

fn verb(host: &HostConnection, block: &str, command: Command) {
    host.commands
        .send(HostCommand::Terminal {
            session: SESSION.into(),
            block: block.into(),
            command,
        })
        .unwrap();
}

/// Every request up to the sentinel sent now, as (method, payload), without the worker's
/// own housekeeping.
async fn requests_until_sentinel(
    server: &mut ThemeServer,
    host: &HostConnection,
) -> Vec<(String, Value)> {
    host.commands
        .send(HostCommand::Request {
            method: "test.sentinel".into(),
            payload: json!({}),
        })
        .unwrap();
    let mut seen = Vec::new();
    loop {
        let (method, payload) =
            tokio::time::timeout(Duration::from_secs(8), server.requests.recv())
                .await
                .expect("the sentinel")
                .unwrap();
        if method == "test.sentinel" {
            return seen;
        }
        if method == RESET || method == CLEAR || method == "session.own_block" {
            seen.push((method, payload));
        }
    }
}

async fn attached(host: &HostConnection) {
    connected(host).await;
    until(host, |m| {
        m.terminals.contains_key("block:a") && m.terminals.contains_key("block:b")
    })
    .await;
}

#[tokio::test]
async fn reset_and_clear_go_once_to_the_block_named_without_claiming_it() {
    let mut server = ThemeServer::start(
        &[("block:a", None), ("block:b", Some("client:other"))],
        None,
    )
    .await;
    let host = HostConnection::spawn(server.endpoint.clone(), None, None, None, true, None);
    attached(&host).await;
    requests_until_sentinel(&mut server, &host).await;

    // Another client's block: reset all the same, and not taken from it.
    verb(&host, "block:b", Command::Reset);
    verb(&host, "block:a", Command::Clear);
    let block = |id: &str| json!({"session_id":SESSION,"block_id":id,"args":{}});
    assert_eq!(
        requests_until_sentinel(&mut server, &host).await,
        [
            (RESET.to_owned(), block("block:b")),
            (CLEAR.to_owned(), block("block:a")),
        ]
    );
    assert_eq!(host.model.lock().unwrap().notice, None);

    // A block this host does not show gets nothing.
    verb(&host, "block:gone", Command::Reset);
    assert_eq!(requests_until_sentinel(&mut server, &host).await, []);
    drop(host);
}

#[tokio::test]
async fn a_refused_reset_is_reported_and_not_sent_again() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (record, mut resets) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        let mut seq = 1;
        loop {
            let mut ws = accept(&listener, "rex.control.v1").await;
            // Data sockets say no hello a control server understands; drop them.
            let Some(Ok(Message::Text(hello))) = ws.next().await else {
                continue;
            };
            if !hello.contains("api_revision") {
                continue;
            }
            send(
                &mut ws,
                json!({"type":"welcome","protocol":"rex.control.v1","api_revision":11,
                "limits":{"max_message_bytes":65536},
                "client":{"id":"client:test","reconnect_token":"test-token"}}),
            )
            .await;
            while let Some(Ok(message)) = ws.next().await {
                let Message::Text(text) = message else {
                    continue;
                };
                let request: Value = serde_json::from_str(&text).unwrap();
                seq += 1;
                let payload = match request["method"].as_str().unwrap_or_default() {
                    "session.bootstrap" => {
                        let mut boot = serde_json::to_value(boot()).unwrap();
                        boot["selected_session"]["view"] = json!({"session_id":SESSION,
                            "label":"snapshot","revision":1,"windows":[{"window_id":"w",
                            "layers":[{"kind":"tiled","layout":{"block_id":"block:a"},
                            "blocks":[{"block_id":"block:a","creator_name":activity::TERMINAL}]}]}]});
                        boot
                    }
                    "data.connect" => json!({"ticket":"AAAAAAAAAAAAAAAAAAAAAA"}),
                    RESET => {
                        let _ = record.send(request["payload"].clone());
                        send(
                            &mut ws,
                            json!({"type":"response","id":request["id"],
                            "error":{"code":"method_not_found","message":"no reset here","status":404}}),
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
    let host = HostConnection::spawn(endpoint, None, None, None, true, None);
    connected(&host).await;
    until(&host, |m| m.terminals.contains_key("block:a")).await;
    verb(&host, "block:a", Command::Reset);
    until(&host, |m| {
        m.notice
            .as_deref()
            .is_some_and(|n| n.contains("no reset here"))
    })
    .await;
    assert!(
        host.model.lock().unwrap().connected,
        "a refusal is not a lost connection"
    );
    // Later requests go through; the reset is not among them.
    host.commands
        .send(HostCommand::Request {
            method: "test.after".into(),
            payload: json!({}),
        })
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(resets.recv().await.is_some());
    assert!(resets.try_recv().is_err(), "sent once");
    drop(host);
}

#[tokio::test]
async fn a_reset_asked_while_disconnected_is_refused_not_kept() {
    // Nothing listens here, so the host stays between attempts.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let host = HostConnection::spawn(endpoint, None, None, None, true, None);
    tokio::time::sleep(Duration::from_millis(200)).await;
    for command in [Command::Reset, Command::Clear] {
        host.model.lock().unwrap().notice = None;
        verb(&host, "block:a", command);
        tokio::time::sleep(Duration::from_millis(200)).await;
        let notice = host.model.lock().unwrap().notice.clone();
        assert!(
            notice
                .as_deref()
                .is_some_and(|n| n.contains("not sent while disconnected")),
            "{notice:?}"
        );
    }
    drop(host);
}
