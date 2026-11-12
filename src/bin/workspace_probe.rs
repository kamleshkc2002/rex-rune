//! Exercises the real host controller, using only sessions created by this probe.
use anyhow::{Context, Result, ensure};
use rex_rune::{
    connection::{Command, Connection, HostCommand, HostConnection, HostModel},
    transport,
};
use rexrs::api::{Layout, session};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

fn model(host: &HostConnection) -> HostModel {
    host.model.lock().expect("host model").clone()
}
fn text(connection: &Connection) -> String {
    connection
        .model
        .lock()
        .expect("terminal model")
        .frame
        .as_ref()
        .map(|f| {
            f.cells
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|c| if c.text.is_empty() { " " } else { &c.text })
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}
async fn wait(
    host: &HostConnection,
    label: &str,
    predicate: impl Fn(&HostModel) -> bool,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let current = model(host);
        if predicate(&current) {
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "{label} timed out: {} / {:?}",
            current.status,
            current.notice
        );
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
}
async fn ready(host: &HostConnection) -> Result<Connection> {
    wait(host, "focused terminal", |m| {
        m.focused()
            .and_then(|id| m.terminals.get(id))
            .is_some_and(|c| {
                let m = c.model.lock().unwrap();
                m.connected && m.control_connected && m.frame.is_some()
            })
    })
    .await?;
    let m = model(host);
    Ok(m.terminals
        .get(m.focused().context("focus")?)
        .context("terminal")?
        .clone())
}
fn request(host: &HostConnection, session: &str, method: &str, mut payload: Value) -> Result<()> {
    payload["session_id"] = json!(session);
    host.commands.send(HostCommand::Request {
        method: method.into(),
        payload,
    })?;
    Ok(())
}
async fn run() -> Result<()> {
    let endpoint = std::env::args()
        .nth(1)
        .unwrap_or_else(transport::local_endpoint);
    // An ssh:// endpoint is reached through a forward of the server's socket.
    let _tunnel = rex_rune::ssh::Tunnel::for_endpoint(&endpoint).await?;
    let endpoint = _tunnel
        .as_ref()
        .map_or(endpoint, |t| t.endpoint().to_owned());
    let mut observer = transport::control(&endpoint, None).await?;
    let revision = observer.welcome().api_revision;
    if let Ok(expected) = std::env::var("REX_EXPECT_REVISION") {
        ensure!(
            revision == expected.parse::<u32>()?,
            "wrong negotiated API revision"
        );
    }
    let mut created = Vec::new();
    let result:Result<()>=async {
        if observer.call(session::AttachAll::new("session")).await?.sessions.is_empty() {
            let empty = HostConnection::spawn(endpoint.clone(), None, None, None, true, None);
            wait(&empty, "empty host connected", |m| m.connected).await?;
            ensure!(model(&empty).sessions.is_empty() && model(&empty).selected.is_none() && model(&empty).terminals.is_empty(), "empty server opened a session");
            println!("PASS empty server connects without creating a session");
        }
        for suffix in ["a","b"] {
            let session=observer.call(session::Create::new().label(format!("rex-workspace-probe-{}-{suffix}",std::process::id())).initial_windows([session::CreateInitialWindow::new().layout(Layout::shell())])).await?;
            created.push(session.session_id);
        }
        let a=&created[0];let b=&created[1];
        observer.call(session::Attach::new(a)).await?;
        observer.call(session::Attach::new(b)).await?;
        let started = Instant::now();
        let host=HostConnection::spawn(endpoint.clone(),Some(a.clone()),None,None,true,None);
        let original=ready(&host).await?;
        println!("STARTUP first terminal frame in {} ms (controller probe polling)", started.elapsed().as_millis());
        let stats = model(&host).protocol;
        ensure!(stats.api_revision == revision, "host negotiated the wrong revision");
        ensure!(stats.bootstrap_requests == 1 && stats.attach_requests == 0 && stats.bundled_tickets_used == 1 && stats.data_connect_requests == 0, "cold startup did not use the bundled ticket");
        println!("PASS API {revision} startup: {} bootstrap, {} attach, {} bundled tickets, {} data.connect", stats.bootstrap_requests, stats.attach_requests, stats.bundled_tickets_used, stats.data_connect_requests);
        let block=original.model.lock().unwrap().block.clone();
        ensure!(!original.model.lock().unwrap().owner,"navigation took ownership");
        original.commands.try_send(Command::Paste("printf 'WORKSPACE_INPUT_OK\\n'".into()))?;
        original.commands.try_send(Command::Key(json!({"key":"enter","mods":"","text":"","action":"press","composing":false})))?;
        wait(&host,"input output",|_|text(&original).lines().any(|l|l.trim()=="WORKSPACE_INPUT_OK")).await?;
        ensure!(original.model.lock().unwrap().owner,"typing did not take ownership");
        println!("PASS navigation does not own; intentional input takes ownership and reaches the terminal");

        let generation=original.model.lock().unwrap().restore_generation;
        host.commands.send(HostCommand::Select(b.clone()))?;
        wait(&host,"session B",|m|m.selected.as_ref()==Some(b)).await?;
        let second=ready(&host).await?;
        ensure!(!second.model.lock().unwrap().owner,"switching session took ownership");
        let frame=original.model.lock().unwrap().frame.clone().unwrap();
        observer.call_raw("com.superlogical.terminal.write",json!({"session_id":a,"block_id":block,"args":{"data":"printf 'OUTPUT_WHILE_HIDDEN\\n'\n"}})).await?;
        tokio::time::sleep(Duration::from_millis(250)).await;
        ensure!(std::sync::Arc::ptr_eq(&frame,original.model.lock().unwrap().frame.as_ref().unwrap()),"hidden terminal repainted");
        host.commands.send(HostCommand::Select(a.clone()))?;
        wait(&host,"hidden output",|m|m.selected.as_ref()==Some(a)&&text(&original).lines().any(|l|l.trim()=="OUTPUT_WHILE_HIDDEN")).await?;
        ensure!(original.model.lock().unwrap().restore_generation==generation,"switching recreated the emulator");
        println!("PASS session switching retains the terminal stream; hidden output parses without repainting");

        request(&host,a,"session.new_window",json!({"window_label":"Probe window","layout":Layout::shell(),"focus":true}))?;
        wait(&host,"new window",|m|m.view().is_some_and(|v|v.windows.len()==2)).await?;
        let new=ready(&host).await?;
        let anchor=new.model.lock().unwrap().block.clone();
        request(&host,a,"session.new_split",json!({"anchor_id":anchor,"direction":"horizontal","side":"after","layout":Layout::shell(),"focus":true}))?;
        wait(&host,"split",|m|m.window().is_some_and(|w|w.layers.iter().map(|l|l.blocks.len()).sum::<usize>()==2)).await?;
        let split=ready(&host).await?;
        let split_id=split.model.lock().unwrap().block.clone();
        ensure!(split_id!=anchor,"split did not move focus");
        let view=observer.call(session::View::new(a)).await?;
        ensure!(view.windows.len()==2,"second client does not see the windows");
        request(&host,a,"session.resize_pane",json!({"before_block_id":anchor,"after_block_id":split_id,"direction":"horizontal","ratio":0.65}))?;
        wait(&host,"divider ratio",|m|m.window().and_then(|w|w.layers.first()).and_then(|l|l.layout.as_ref()).and_then(|n|n.split.as_ref()).is_some_and(|s|(s.ratio-0.65).abs()<1e-6)).await?;
        request(&host,a,"session.zoom_block",json!({"block_id":split_id,"zoomed":true}))?;
        wait(&host,"zoom",|m|m.window().is_some_and(|w|w.zoomed_block_id.as_ref()==Some(&split_id))).await?;
        request(&host,a,"session.zoom_block",json!({"block_id":split_id,"zoomed":false}))?;
        wait(&host,"restore",|m|m.window().is_some_and(|w|w.zoomed_block_id.is_none())).await?;
        println!("PASS authoritative windows, splits, resize and zoom; a second API client sees the layout");

        observer.call_raw("session.set_window_label",json!({"session_id":a,"window_id":model(&host).window().unwrap().window_id,"label":"Renamed externally"})).await?;
        wait(&host,"external rename",|m|m.window().is_some_and(|w|w.label.as_deref()==Some("Renamed externally"))).await?;
        let identity=model(&host).client_id;
        host.commands.send(HostCommand::Reconnect)?;
        wait(&host,"control disconnect",|m|!m.connected).await?;
        wait(&host,"control reconnect",|m|m.connected).await?;
        ensure!(model(&host).client_id==identity,"control reconnect changed identity");
        ready(&host).await?;
        ensure!(original.model.lock().unwrap().restore_generation == generation, "control reconnect restarted a healthy terminal");
        println!("PASS external changes reconcile and host reconnect restores the control identity");

        observer.call_raw("block.close",json!({"session_id":a,"block_id":split_id})).await?;
        wait(&host,"closed pane",|m|!m.terminals.contains_key(&split_id)&&m.window().is_some_and(|w|w.layers.iter().map(|l|l.blocks.len()).sum::<usize>()==1)).await?;
        ensure!(model(&host).connected,"closing a pane disconnected the host");
        println!("PASS external pane closure removes its state without losing the host");
        drop(host);

        // Selection contracts use fresh connections, so bootstrap really is first.
        // On a populated server, default selection may precede our owned fixtures.
        let default_session = observer.call(session::AttachAll::new("session")).await?.sessions.first().map(|s|s.session_id.clone());
        for (wanted, expected) in [(None, default_session), (Some(format!("rex-workspace-probe-{}-a", std::process::id())), Some(a.clone())),
            (Some("session:00000000-0000-0000-0000-000000000000".into()), None), (Some("missing-label".into()), None)] {
            let host = HostConnection::spawn(endpoint.clone(), wanted, None, None, true, None);
            wait(&host, "selection connected", |m| m.connected).await?;
            ensure!(model(&host).selected == expected, "startup changed exact selection semantics");
            if expected.is_some() { ready(&host).await?; }
            else {
                ensure!(model(&host).terminals.is_empty(), "missing selection opened a terminal");
                host.commands.send(HostCommand::Reconnect)?;
                wait(&host, "missing selection disconnect", |m| !m.connected).await?;
                wait(&host, "missing selection reconnect", |m| m.connected).await?;
                ensure!(model(&host).selected.is_none(), "reconnect fell back from an explicit missing selection");
            }
        }
        println!("PASS default, exact-label and missing selection, including reconnect");
        let duplicate = format!("probe-duplicate-{}", std::process::id());
        observer.call_raw("session.set_label", json!({"session_id":a,"label":duplicate})).await?;
        observer.call_raw("session.set_label", json!({"session_id":b,"label":duplicate})).await?;
        let ambiguous = HostConnection::spawn(endpoint.clone(), Some(duplicate), None, None, true, None);
        wait(&ambiguous, "ambiguous selection", |m| m.connected).await?;
        ensure!(model(&ambiguous).selected.is_none() && model(&ambiguous).terminals.is_empty(), "ambiguous label selected an arbitrary session");
        drop(ambiguous);
        let created_host = HostConnection::spawn(endpoint.clone(), None, Some((format!("probe-created-{}", std::process::id()), None)), None, true, None);
        ready(&created_host).await?;
        created.push(model(&created_host).selected.context("created selection")?);
        ensure!(model(&created_host).protocol.data_connect_requests == 1, "new terminal did not fall back to data.connect");
        drop(created_host);
        println!("PASS ambiguous label selects none; explicit create uses fresh ticket");

        let multi = observer.call(session::Create::new().label("probe-bundled-panes").initial_windows([
            session::CreateInitialWindow::new().layout(Layout::shell()),
            session::CreateInitialWindow::new().layout(Layout::shell()),
        ])).await?;
        created.push(multi.session_id.clone());
        observer.call(session::Attach::new(&multi.session_id)).await?;
        let windows = observer.call(session::View::new(&multi.session_id)).await?.windows;
        let first_window = windows[0].window_id.as_ref().context("first window")?;
        let second_window = windows[1].window_id.as_ref().context("second window")?;
        observer.call_raw("session.new_split", json!({"session_id":multi.session_id,"anchor_id":windows[0].focused_block_id,
            "direction":"horizontal","side":"after","layout":Layout::shell(),"focus":true})).await?;
        observer.call_raw("session.focus_window", json!({"session_id":multi.session_id,"window_id":first_window})).await?;
        let multi_host = HostConnection::spawn(endpoint.clone(), Some(multi.session_id.clone()), None, None, true, None);
        wait(&multi_host, "two bundled panes", |m| m.terminals.len() == 2 && m.terminals.values().all(|c| c.model.lock().unwrap().connected)).await?;
        request(&multi_host, &multi.session_id, "session.focus_window", json!({"window_id":second_window}))?;
        wait(&multi_host, "inactive window ticket", |m| m.terminals.len() == 3 && m.terminals.values().all(|c| c.model.lock().unwrap().connected)).await?;
        let stats = model(&multi_host).protocol;
        ensure!(stats.bundled_tickets_used == 3 && stats.data_connect_requests == 0, "split/inactive-window tickets were not reused");
        drop(multi_host);
        println!("PASS three panes across split and inactive windows: {} bundled, {} data.connect", stats.bundled_tickets_used, stats.data_connect_requests);
        Ok(())
    }.await;
    for id in created {
        observer.call(session::Attach::new(&id)).await?;
        observer.call(session::Destroy::new(id)).await?;
    }
    observer.close().await?;
    result
}
fn main() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(run())
}
