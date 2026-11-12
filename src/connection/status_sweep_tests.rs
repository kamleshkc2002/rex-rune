use super::*;

fn terminal_view(session: &str, blocks: &[&str]) -> Value {
    json!({"session_id":session,"revision":1,"windows":[{"window_id":format!("{session}/tab"),
        "layers":[{"kind":"tiled","blocks":blocks.iter().map(|b| json!({
            "block_id":b,"creator_name":activity::TERMINAL
        })).collect::<Vec<_>>()}]}]})
}
async fn status(ws: &mut WebSocketStream<TcpStream>, session: &str, block: &str) -> Value {
    let request = next(ws).await;
    assert_eq!(
        request["method"],
        format!("{}.program_status", activity::TERMINAL)
    );
    assert_eq!(
        request["payload"],
        json!({"session_id":session,"block_id":block,"args":{}})
    );
    request
}
async fn old_status(ws: &mut WebSocketStream<TcpStream>, session: &str, block: &str) {
    let request = status(ws, session, block).await;
    reply(
        ws,
        &request,
        2,
        json!({"records":[{"id":"","state":"blocked"}]}),
    )
    .await;
}
async fn metadata(ws: &mut WebSocketStream<TcpStream>, block: &str) {
    for method in ["title", "process"] {
        let request = next(ws).await;
        assert_eq!(
            request["method"],
            format!("{}.{method}", activity::TERMINAL)
        );
        assert_eq!(request["payload"]["block_id"], block);
        reply(ws, &request, 2, json!({})).await;
    }
}

#[tokio::test]
async fn sweep_selected_first_server_order_once_and_selection_only_adds_metadata() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (sent, mut ready) = tokio::sync::mpsc::unbounded_channel();
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 13).await;
        let request = recv(&mut ws).await;
        assert_eq!(request["method"], "session.bootstrap");
        let mut boot = boot_with(&[OTHER, THIRD]);
        boot["selected_session"]["view"] = terminal_view(SESSION, &["front"]);
        reply(&mut ws, &request, 1, boot).await;
        old_status(&mut ws, SESSION, "front").await;
        metadata(&mut ws, "front").await;
        for (session, blocks) in [(OTHER, vec!["b", "c"]), (THIRD, vec!["d"])] {
            let request = prefetched(&mut ws, session).await;
            reply(&mut ws, &request, 2, terminal_view(session, &blocks)).await;
            for block in blocks {
                old_status(&mut ws, session, block).await;
            }
        }
        sent.send(()).unwrap();
        // Selection still attaches, but must not query status twice.
        let request = next(&mut ws).await;
        assert_eq!(request["method"], "session.attach");
        reply(
            &mut ws,
            &request,
            3,
            json!({"client_id":"client:test", "view":terminal_view(OTHER, &["b", "c"])}),
        )
        .await;
        metadata(&mut ws, "b").await;
        metadata(&mut ws, "c").await;
        sent.send(()).unwrap();
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    prefetch(&host, &[OTHER, THIRD]);
    ready.recv().await.unwrap();
    until(&host, |m| !m.activity_reading && m.activity.len() == 4).await;
    {
        let m = host.model.lock().unwrap();
        assert!(!m.activity_incomplete);
        assert_eq!(m.selected.as_deref(), Some(SESSION));
        assert!(
            m.activity
                .values()
                .flat_map(|r| r.values())
                .all(|r| !r.live)
        );
        assert!(m.terminals.is_empty(), "sweep does not attach data streams");
    }
    host.commands
        .send(HostCommand::Select(OTHER.into()))
        .unwrap();
    ready.recv().await.unwrap();
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn sweep_refusals_are_not_retried_and_slow_status_does_not_block_commands() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (sent, mut ready) = tokio::sync::mpsc::unbounded_channel();
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 10).await;
        let request = recv(&mut ws).await;
        reply(&mut ws, &request, 1, boot_with(&[OTHER, THIRD])).await;
        let request = prefetched(&mut ws, OTHER).await;
        reply(
            &mut ws,
            &request,
            2,
            terminal_view(OTHER, &["slow", "refused"]),
        )
        .await;
        let slow = status(&mut ws, OTHER, "slow").await;
        sent.send(()).unwrap();
        answer_request(&mut ws).await;
        reply(&mut ws, &slow, 4, json!({"records":[]})).await;
        let refused = status(&mut ws, OTHER, "refused").await;
        send(&mut ws, json!({"type":"response","id":refused["id"],"error":{"code":"not_supported","message":"no","status":400}})).await;
        let refused = prefetched(&mut ws, THIRD).await;
        send(&mut ws, json!({"type":"response","id":refused["id"],"error":{"code":"not_supported","message":"no","status":400}})).await;
        sent.send(()).unwrap();
        // A later ordinary command does not restart either failed sweep read.
        answer_request(&mut ws).await;
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    ready.recv().await.unwrap();
    focus_window(&host);
    ready.recv().await.unwrap();
    until(&host, |m| !m.activity_reading && m.activity_incomplete).await;
    focus_window(&host);
    until(&host, |m| m.views[SESSION].revision == 2).await;
    // Synchronize command completion before closing, rather than waiting for no requests.
    tokio::time::sleep(Duration::from_millis(100)).await;
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn sweep_new_panes_and_sessions_join_once_and_reconnect_restarts_one_pass() {
    const NEW: &str = "session:00000000-0000-0000-0000-000000000004";
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        for connection in 0..2 {
            let mut ws = accept(&listener, "rex.control.v1").await;
            welcome(&mut ws, 13).await;
            let request = recv(&mut ws).await;
            reply(&mut ws, &request, 1, boot_with(&[OTHER])).await;
            let request = prefetched(&mut ws, OTHER).await;
            reply(&mut ws, &request, 2, terminal_view(OTHER, &["b"])).await;
            old_status(&mut ws, OTHER, "b").await;
            if connection == 0 {
                send(&mut ws, json!({"type":"event","seq":5,"payload":{"type":"session_event","payload":{
                    "session_id":OTHER,"event":{"type":"session_view_changed","payload":{"revision":2}}}}})).await;
                let request = next(&mut ws).await;
                assert_eq!(request["method"], "session.view");
                let mut view = terminal_view(OTHER, &["b", "new-pane"]);
                view["revision"] = json!(2);
                reply(&mut ws, &request, 6, view).await;
                old_status(&mut ws, OTHER, "new-pane").await;
                send(&mut ws, json!({"type":"event","seq":7,"payload":{"type":"session_created","payload":{"session_id":NEW}}})).await;
                let request = next(&mut ws).await;
                assert_eq!(request["method"], "session.attach_all");
                reply(
                    &mut ws,
                    &request,
                    8,
                    boot_with(&[OTHER, NEW])["attach_all"].clone(),
                )
                .await;
                let request = prefetched(&mut ws, NEW).await;
                reply(&mut ws, &request, 9, terminal_view(NEW, &["new-session"])).await;
                old_status(&mut ws, NEW, "new-session").await;
                assert!(!matches!(ws.next().await, Some(Ok(Message::Text(_)))));
            } else {
                assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
            }
        }
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    until(&host, |m| {
        !m.activity_reading && m.activity.contains_key("new-session")
    })
    .await;
    host.commands.send(HostCommand::Reconnect).unwrap();
    until(&host, |m| !m.connected).await;
    host.commands.send(HostCommand::Reconnect).unwrap();
    until(&host, |m| {
        m.connection_epoch == 2 && !m.activity_reading && m.activity.contains_key("b")
    })
    .await;
    assert_eq!(host.model.lock().unwrap().activity.len(), 1);
    drop(host);
    server.await.unwrap();
}

fn block_event(seq: u64, session: &str, block: &str, name: &str, payload: Value) -> Value {
    json!({"type":"event","seq":seq,"payload":{"type":"session_event","payload":{
        "session_id":session,"event":{"type":"block_event","payload":{"block_id":block,
            "creator_name":activity::TERMINAL,"name":name,"payload":payload}}}}})
}

#[tokio::test]
async fn a_record_removed_while_its_read_was_out_is_not_brought_back() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 13).await;
        let request = recv(&mut ws).await;
        reply(&mut ws, &request, 1, boot_with(&[OTHER])).await;
        let request = prefetched(&mut ws, OTHER).await;
        reply(&mut ws, &request, 2, terminal_view(OTHER, &["b"])).await;
        // The answer says Blocked; the program's record is removed right behind it. A
        // detached reply is handled after the event that follows it on the wire.
        let request = status(&mut ws, OTHER, "b").await;
        // Both frames in one flush, so the client reads the event right behind the reply.
        for frame in [
            json!({"type":"response","id":request["id"],"seq":3,
                "payload":{"records":[{"id":"","state":"blocked"}]}}),
            block_event(4, OTHER, "b", "program_status_removed", json!({"ids":[""]})),
        ] {
            ws.feed(Message::Text(frame.to_string().into()))
                .await
                .unwrap();
        }
        ws.flush().await.unwrap();
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    until(&host, |m| {
        m.views.contains_key(OTHER) && !m.activity_reading
    })
    .await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    {
        let m = host.model.lock().unwrap();
        assert!(
            !m.activity.contains_key("b"),
            "the removed record came back: {:?}",
            m.activity.get("b")
        );
        assert!(!m.activity_incomplete);
    }
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn a_live_report_for_an_unreached_session_reads_its_layout_next_and_stays_live() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 13).await;
        let request = recv(&mut ws).await;
        reply(&mut ws, &request, 1, boot_with(&[OTHER, THIRD])).await;
        // The pass starts on the first other session. While that read is out, a program
        // in the last session reports that it needs the person.
        let first = prefetched(&mut ws, OTHER).await;
        send(
            &mut ws,
            block_event(
                2,
                THIRD,
                "z",
                "program_status_changed",
                json!({"id":"","record":{"state":"blocked"}}),
            ),
        )
        .await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        reply(&mut ws, &first, 3, terminal_view(OTHER, &["b"])).await;
        // Its layout is read next, ahead of the first session's pane.
        let urgent = prefetched(&mut ws, THIRD).await;
        reply(&mut ws, &urgent, 4, terminal_view(THIRD, &["z"])).await;
        let request = status(&mut ws, OTHER, "b").await;
        reply(&mut ws, &request, 5, json!({"records":[]})).await;
        // The read of the reporting pane answers with something older than the report.
        let request = status(&mut ws, THIRD, "z").await;
        reply(
            &mut ws,
            &request,
            6,
            json!({"records":[{"id":"","state":"working"}]}),
        )
        .await;
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    until(&host, |m| {
        m.views.contains_key(OTHER) && m.views.contains_key(THIRD) && !m.activity_reading
    })
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    {
        let m = host.model.lock().unwrap();
        let record = &m.activity["z"][""];
        assert_eq!(record.state, activity::ProgramState::Blocked);
        assert!(record.live, "still news, so the app can notify");
        // Both layouts are held for activity only: neither session was opened.
        assert!(m.background_views.contains(OTHER) && m.background_views.contains(THIRD));
        assert!(!m.background_views.contains(SESSION));
    }
    drop(host);
    server.await.unwrap();
}
