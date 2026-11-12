//! Actual control-wire checks: no terminal input, claim, layout request or replay.
use super::*;
use crate::connection::directory::{Failure, Request};
use rexrs::api::terminal::ListDir;

fn listing(id: u64, epoch: u64, open: &Arc<()>) -> Request {
    Request {
        id,
        epoch,
        session: SESSION.into(),
        block: "block:test".into(),
        args: ListDir::new().path("~/tree/").prefix("a").files(true),
        deadline: Instant::now() + Duration::from_secs(8),
        open: Arc::downgrade(open),
    }
}
async fn answer(host: &HostConnection, id: u64) -> Arc<directory::Reply> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(reply) = host
                .model
                .lock()
                .unwrap()
                .directory
                .clone()
                .filter(|r| r.id == id)
            {
                return reply;
            }
            host.wake.recv_async().await.unwrap();
        }
    })
    .await
    .unwrap()
}
async fn start(listener: &TcpListener) -> WebSocketStream<TcpStream> {
    let mut ws = accept(listener, "rex.control.v1").await;
    welcome(&mut ws, 13).await;
    let bootstrap = recv(&mut ws).await;
    assert_eq!(bootstrap["method"], "session.bootstrap");
    send(
        &mut ws,
        json!({"type":"response","id":bootstrap["id"],"seq":1,"payload":boot()}),
    )
    .await;
    ws
}

#[tokio::test]
async fn directory_reads_only_list_and_cancels_closed_old_epoch_and_expired_requests() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut ws = start(&listener).await;
        for code in [None, Some("method_not_found"), Some("internal")] {
            let request = recv(&mut ws).await;
            assert_eq!(request["method"], "com.superlogical.terminal.list_dir");
            assert_eq!(
                request["payload"],
                json!({"session_id":SESSION,"block_id":"block:test",
                "args":{"path":"~/tree/","prefix":"a","files":true}})
            );
            let response = if let Some(code) = code {
                json!({"type":"response","id":request["id"],"error":{"code":code,"message":"fixture refusal","status":404}})
            } else {
                json!({"type":"response","id":request["id"],"payload":{"path":"/home/dev/tree","home":"/home/dev","status":"ok","entries":[{"name":"a b","file":true}],"truncated":false}})
            };
            send(&mut ws, response).await;
        }
        assert!(
            matches!(ws.next().await, Some(Ok(Message::Close(_)))),
            "nothing follows a listing except shutdown"
        );
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    connected(&host).await;
    let open = Arc::new(());
    let closed = listing(1, 1, &Arc::new(()));
    host.commands
        .send(HostCommand::ListDir(Box::new(closed)))
        .unwrap();
    host.commands
        .send(HostCommand::ListDir(Box::new(listing(2, 0, &open))))
        .unwrap();
    assert_eq!(answer(&host, 2).await.result, Err(Failure::Disconnected));
    let mut expired = listing(3, 1, &open);
    expired.deadline = Instant::now();
    host.commands
        .send(HostCommand::ListDir(Box::new(expired)))
        .unwrap();
    assert_eq!(answer(&host, 3).await.result, Err(Failure::Timeout));
    for (id, failure) in [
        (4, None),
        (5, Some(Failure::Unsupported)),
        (6, Some(Failure::Request)),
    ] {
        host.commands
            .send(HostCommand::ListDir(Box::new(listing(id, 1, &open))))
            .unwrap();
        let reply = answer(&host, id).await;
        assert_eq!(reply.epoch, 1);
        if let Some(failure) = failure {
            assert_eq!(reply.result, Err(failure));
        } else {
            assert_eq!(reply.result.as_ref().unwrap().entries[0].name, "a b");
        }
    }
    assert!(host.model.lock().unwrap().connected);
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn an_oversized_listing_request_fails_only_that_listing() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut ws = start(&listener).await;
        // The oversized request never reaches the wire; the next one does, on this
        // same connection.
        let request = recv(&mut ws).await;
        assert_eq!(request["payload"]["args"]["path"], "~/tree/");
        send(
            &mut ws,
            json!({"type":"response","id":request["id"],"payload":{"path":"/home/dev/tree","home":"/home/dev","status":"ok","entries":[],"truncated":false}}),
        )
        .await;
        let extra = tokio::time::timeout(Duration::from_millis(300), listener.accept()).await;
        assert!(extra.is_err(), "no reconnect");
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    connected(&host).await;
    let open = Arc::new(());
    // The mock server's message limit is 64 KiB.
    let mut huge = listing(1, 1, &open);
    huge.args = ListDir::new().path("x".repeat(200_000)).files(true);
    host.commands
        .send(HostCommand::ListDir(Box::new(huge)))
        .unwrap();
    assert_eq!(answer(&host, 1).await.result, Err(Failure::Request));
    host.commands
        .send(HostCommand::ListDir(Box::new(listing(2, 1, &open))))
        .unwrap();
    assert!(answer(&host, 2).await.result.is_ok());
    assert!(host.model.lock().unwrap().connected);
    assert_eq!(host.model.lock().unwrap().connection_epoch, 1);
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn a_slow_listing_does_not_hold_up_other_requests_on_the_host() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut ws = start(&listener).await;
        let slow = recv(&mut ws).await;
        assert_eq!(slow["method"], "com.superlogical.terminal.list_dir");
        // The listing stays unanswered while the host's other work is asked and answered.
        let list = recv(&mut ws).await;
        assert_eq!(list["method"], "session.attach_all");
        reply(
            &mut ws,
            &list,
            2,
            json!({"sessions":[{"session_id":SESSION,"label":"renamed"},
                {"session_id":OTHER,"label":"elsewhere"}]}),
        )
        .await;
        let view = recv(&mut ws).await;
        assert_eq!(view["method"], "session.view");
        reply(
            &mut ws,
            &view,
            3,
            json!({"session_id":SESSION,"label":"renamed","revision":2}),
        )
        .await;
        send(
            &mut ws,
            json!({"type":"response","id":slow["id"],"payload":{"path":"/mnt/slow","home":"/home/dev","status":"ok","entries":[{"name":"late","file":true}],"truncated":false}}),
        )
        .await;
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    connected(&host).await;
    let open = Arc::new(());
    host.commands
        .send(HostCommand::ListDir(Box::new(listing(1, 1, &open))))
        .unwrap();
    host.commands.send(HostCommand::RefreshSessions).unwrap();
    until(&host, |m| {
        m.sessions.len() == 2 && m.views.get(SESSION).is_some_and(|v| v.revision == 2)
    })
    .await;
    let reply = answer(&host, 1).await;
    assert_eq!(reply.result.as_ref().unwrap().entries[0].name, "late");
    assert!(host.model.lock().unwrap().connected);
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn a_silent_directory_request_times_out_once_without_input_or_replay() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut ws = start(&listener).await;
        assert_eq!(
            recv(&mut ws).await["method"],
            "com.superlogical.terminal.list_dir"
        );
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    connected(&host).await;
    let open = Arc::new(());
    let mut request = listing(1, 1, &open);
    request.deadline = Instant::now() + Duration::from_millis(100);
    host.commands
        .send(HostCommand::ListDir(Box::new(request)))
        .unwrap();
    assert_eq!(answer(&host, 1).await.result, Err(Failure::Timeout));
    drop(host);
    server.await.unwrap();
}
