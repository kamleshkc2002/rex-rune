//! Correlation and disconnected delivery checks for client-side visit navigation.
use super::*;

async fn navigation_result(host: &HostConnection, id: u64) -> bool {
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            if let Some((got, ok)) = host.model.lock().unwrap().navigation_result
                && got == id
            {
                return ok;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn navigation_correlates_success_and_failure_without_extra_attaches() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 11).await;
        let request = recv(&mut ws).await;
        let mut boot = serde_json::to_value(boot()).unwrap();
        boot["attach_all"]["sessions"]
            .as_array_mut()
            .unwrap()
            .push(json!({"session_id":"other","label":"other"}));
        reply(&mut ws, &request, 1, boot).await;
        let background = prefetched(&mut ws, "other").await;
        reply(&mut ws, &background, 1, view_of("other")).await;
        // Same-session navigation focuses on the server without reattaching.
        let focus = recv(&mut ws).await;
        assert_eq!(focus["method"], "session.focus_window");
        assert_eq!(focus["payload"]["window_id"], "w");
        reply(&mut ws, &focus, 2, json!({})).await;
        let view = recv(&mut ws).await;
        assert_eq!(view["method"], "session.view");
        reply(
            &mut ws,
            &view,
            3,
            json!({"revision":3,"session_id":SESSION,
            "active_window_id":"w","windows":[{"window_id":"w","layers":[]}]}),
        )
        .await;

        // A failed focus has its own id, not a generic sticky-notice signal.
        let focus = recv(&mut ws).await;
        assert_eq!(focus["method"], "session.focus_window");
        send(
            &mut ws,
            json!({"type":"response","id":focus["id"],
            "error":{"code":"conflict","message":"tab closed","status":409}}),
        )
        .await;

        let attach = recv(&mut ws).await;
        assert_eq!(attach["method"], "session.attach");
        assert_eq!(attach["payload"]["session_id"], "other");
        reply(
            &mut ws,
            &attach,
            4,
            json!({"client_id":"client:test","view":{"revision":4,"session_id":"other",
            "active_window_id":"other-w","windows":[{"window_id":"other-w","layers":[]}]}}),
        )
        .await;
        // Selecting an unknown session (window=None) preserves its server focus.
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    until(&host, |m| m.views.contains_key("other")).await;
    host.model.lock().unwrap().notice = Some("Upload failed".into());
    for (id, session, window, expected) in [
        (1, SESSION, Some("w"), true),
        (2, SESSION, Some("closed"), false),
        (3, "other", None, true),
    ] {
        host.commands
            .send(HostCommand::Navigate {
                id,
                deadline: Instant::now() + Duration::from_secs(30),
                session: session.into(),
                window: window.map(str::to_owned),
            })
            .unwrap();
        assert_eq!(navigation_result(&host, id).await, expected);
    }
    {
        let m = host.model.lock().unwrap();
        assert_eq!(m.selected.as_deref(), Some("other"));
        assert_eq!(m.protocol.attach_requests, 1);
        assert!(m.connected);
    }
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn expired_queued_navigation_never_reaches_the_server() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (blocked, waiting) = tokio::sync::oneshot::channel();
    let (release, resume) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 11).await;
        let request = recv(&mut ws).await;
        reply(&mut ws, &request, 1, serde_json::to_value(boot()).unwrap()).await;
        // Hold one awaited operation so navigation waits in the same queue as it
        // would behind an upload. No long real-time timeout is needed.
        let claim = recv(&mut ws).await;
        assert_eq!(claim["method"], "session.own_block");
        blocked.send(()).unwrap();
        resume.await.unwrap();
        reply(&mut ws, &claim, 2, json!({"blocks":[]})).await;
        let next = ws.next().await;
        assert!(
            matches!(next, Some(Ok(Message::Close(_)))),
            "expired navigation must not attach or focus: {next:?}"
        );
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    connected(&host).await;
    host.commands
        .send(HostCommand::Claim {
            session: SESSION.into(),
            blocks: vec!["b".into()],
        })
        .unwrap();
    waiting.await.unwrap();
    let deadline = Instant::now() + Duration::from_millis(20);
    host.commands
        .send(HostCommand::Navigate {
            id: 19,
            deadline,
            session: "other".into(),
            window: Some("w".into()),
        })
        .unwrap();
    tokio::time::sleep_until(deadline.into()).await;
    release.send(()).unwrap();
    assert!(!navigation_result(&host, 19).await);
    {
        let m = host.model.lock().unwrap();
        assert!(m.connected, "expiry is not a transport failure");
        assert_eq!(m.protocol.attach_requests, 0);
        assert_eq!(m.selected.as_deref(), Some(SESSION));
        assert!(m.notice.as_ref().unwrap().contains("expired"));
    }
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn selecting_the_current_session_clears_a_sticky_notice() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 11).await;
        let request = recv(&mut ws).await;
        reply(&mut ws, &request, 1, serde_json::to_value(boot()).unwrap()).await;
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    connected(&host).await;
    host.model.lock().unwrap().notice = Some("Upload failed".into());
    host.commands
        .send(HostCommand::Navigate {
            id: 23,
            deadline: Instant::now() + Duration::from_secs(30),
            session: SESSION.into(),
            window: None,
        })
        .unwrap();
    assert!(navigation_result(&host, 23).await);
    assert_eq!(host.model.lock().unwrap().notice, None);
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn disconnected_navigation_is_failed_by_id_not_replayed() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    host.commands
        .send(HostCommand::Navigate {
            id: 17,
            deadline: Instant::now() + Duration::from_secs(30),
            session: SESSION.into(),
            window: Some("w".into()),
        })
        .unwrap();
    assert!(!navigation_result(&host, 17).await);
    let m = host.model.lock().unwrap();
    assert_eq!(m.selected, None);
    assert!(
        m.notice
            .as_ref()
            .unwrap()
            .contains("not sent while disconnected")
    );
}
