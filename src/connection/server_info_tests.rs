use super::*;
use server_info::Answer;

async fn settled(host: &HostConnection, epoch: u64) -> HostModel {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            {
                let model = host.model.lock().unwrap();
                if model.connected
                    && model.connection_epoch == epoch
                    && model.server_info.build.settled()
                {
                    return model.clone();
                }
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap()
}

/// The one read a connection sends once it is ready.
async fn version_read(ws: &mut WebSocketStream<TcpStream>) -> Value {
    let version = recv(ws).await;
    assert_eq!(version["method"], "server.version");
    assert_eq!(version["payload"], json!({}));
    version
}

/// Answer the bootstrap, whose reply carries the server's status.
async fn bootstrap(ws: &mut WebSocketStream<TcpStream>, status: session::ServerStatus) {
    let request = recv(ws).await;
    assert_eq!(request["method"], "session.bootstrap");
    let reply = session::BootstrapResponse {
        server_status: status,
        ..boot()
    };
    send(
        ws,
        json!({"type":"response","id":request["id"],"seq":1,"payload":reply}),
    )
    .await;
}

#[tokio::test]
async fn identity_is_read_once_per_connection_and_a_refusal_is_quiet() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (hang_up, hung_up) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        // A current server: the welcome carries the range, the bootstrap reply the status,
        // and the version read answers.
        let mut ws = accept(&listener, "rex.control.v1").await;
        recv(&mut ws).await;
        send(
            &mut ws,
            json!({"type":"welcome", "protocol":"rex.control.v1", "api_revision":13,
            "api_min_revision":10, "api_server_revision":13, "limits":{"max_message_bytes":65536},
            "client":{"id":"client:test", "reconnect_token":"test-token"}}),
        )
        .await;
        bootstrap(
            &mut ws,
            session::ServerStatus {
                pid: 4242,
                instance_id: "instance:1".into(),
                started_at: "2026-11-10T08:00:00Z".into(),
                session_count: 2,
                tailscale: Some(session::Tailscale {
                    state: "starting".into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await;
        let version = version_read(&mut ws).await;
        send(
            &mut ws,
            json!({"type":"response","id":version["id"],"payload":{
            "rex_version":"0.1.0-preview.15", "platform":"linux_amd64", "dirty":false,
            "api_revision":13, "api_min_revision":10}}),
        )
        .await;
        // The server says its Tailscale state changed: the status is read again, the
        // version is not.
        send(&mut ws, json!({"type":"event","seq":2,"payload":{"type":"tailscale_status_changed","payload":{}}})).await;
        let again = recv(&mut ws).await;
        assert_eq!(again["method"], "server.status");
        assert_eq!(again["payload"], json!({}));
        send(
            &mut ws,
            json!({"type":"response","id":again["id"],"payload":{
            "pid":4242, "instance_id":"instance:1", "session_count":2,
            "tailscale":{"state":"running","allow_tailnet_members":false}}}),
        )
        .await;
        hung_up.await.unwrap();
        drop(ws);

        // The next connection reads again; this server doesn't know the method.
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 10).await;
        bootstrap(&mut ws, session::ServerStatus::default()).await;
        let version = version_read(&mut ws).await;
        send(
            &mut ws,
            json!({"type":"response","id":version["id"],
            "error":{"code":"method_not_found","message":"no such method","status":404}}),
        )
        .await;
        // Nothing follows a refusal: no retry, no other request.
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    host.commands.send(HostCommand::Identify).unwrap();

    let model = settled(&host, 1).await;
    assert_eq!(model.server_info.rex_version(), Some("0.1.0-preview.15"));
    assert_eq!(model.server_info.api_revisions, Some((10, 13)));
    let build = model.server_info.build.reported().unwrap();
    assert_eq!(build.platform.as_deref(), Some("linux_amd64"));
    assert_eq!(build.commit, None, "left out by the server");
    // The status came with the bootstrap reply, without a request of its own.
    let process = model.server_info.process.reported().unwrap();
    assert_eq!((process.pid, process.session_count), (Some(4242), Some(2)));
    assert_eq!(process.instance_id.as_deref(), Some("instance:1"));
    assert_eq!(process.started_at.as_deref(), Some("2026-11-10T08:00:00Z"));
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let state = {
                let model = host.model.lock().unwrap();
                let process = model.server_info.process.reported().cloned();
                process.and_then(|p| p.tailscale).map(|t| t.state)
            };
            if state.as_deref() == Some("running") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("the status is read again after tailscale_status_changed");
    assert_eq!(
        host.model.lock().unwrap().server_info.rex_version(),
        Some("0.1.0-preview.15")
    );
    // Reads never attach or create: one bootstrap, no attach, the snapshot's one session.
    assert_eq!(model.protocol.attach_requests, 0);
    assert_eq!(model.sessions.len(), 1);

    hang_up.send(()).unwrap();
    let model = settled(&host, 2).await;
    assert_eq!(model.server_info.build, Answer::Unreported);
    assert_eq!(model.server_info.rex_version(), None);
    assert_eq!(model.server_info.api_revisions, None);
    let process = model.server_info.process.reported().unwrap();
    assert_eq!(
        (&process.instance_id, &process.tailscale),
        (&None, &None),
        "the last connection's facts are gone"
    );
    assert!(model.connected, "a refusal doesn't disconnect");
    assert_eq!(model.status, "Connected");
    assert_eq!(model.notice, None, "and says nothing");
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn a_front_end_that_never_asked_sends_no_read() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 13).await;
        bootstrap(
            &mut ws,
            session::ServerStatus {
                pid: 7,
                ..Default::default()
            },
        )
        .await;
        send(&mut ws, json!({"type":"event","seq":2,"payload":{"type":"tailscale_status_changed","payload":{}}})).await;
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    connected(&host).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    {
        let model = host.model.lock().unwrap();
        assert_eq!(model.server_info.build, Answer::Pending);
        assert_eq!(model.server_info.process.reported().unwrap().pid, Some(7));
    }
    drop(host);
    server.await.unwrap();
}
