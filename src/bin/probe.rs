//! Live, destructive only to its own temporary session. Exercises the same
//! snapshot/APC/offset implementation used by the native view.
use anyhow::{Context, Result, ensure};
use rex_rune::{terminal::TerminalStream, transport, wire};
use rexrs::api::{Layout, session};
use serde_json::json;
use std::time::Duration;

fn main() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(run())
}

async fn open(
    control: &mut transport::Client,
    endpoint: &str,
    session: &str,
    block: &str,
) -> Result<(transport::Socket, wire::Welcome)> {
    let ticket=control.call_raw("data.connect",json!({"session_id":session,"block_id":block,"data_ref_id":"terminal","subprotocol":wire::PROTOCOL})).await?;
    let mut socket = transport::socket(endpoint, "/api/data", wire::PROTOCOL).await?;
    let welcome =
        wire::handshake(&mut socket, ticket["ticket"].as_str().context("ticket")?).await?;
    Ok((socket, welcome))
}
async fn output(socket: &mut transport::Socket, stream: &mut TerminalStream) -> Result<()> {
    let bytes = tokio::time::timeout(Duration::from_secs(10), wire::next_binary(socket)).await??;
    // Deliberately cut records, UTF-8, and Rex APCs independently of WebSocket messages.
    for chunk in bytes.chunks(17) {
        stream.feed(chunk)?;
    }
    Ok(())
}
fn verify_sequence(stream: &TerminalStream) -> Result<()> {
    let text = stream.text()?;
    let numbers: Vec<u32> = text
        .lines()
        .filter_map(|line| line.trim().strip_prefix("REXSEQ_")?.parse().ok())
        .collect();
    ensure!(
        numbers == (1..=160).collect::<Vec<_>>(),
        "missing or duplicated output: got {} numbered lines",
        numbers.len()
    );
    ensure!(
        text.contains("SNAPSHOT_UNICODE_界_café_e\u{301}"),
        "Unicode marker missing"
    );
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
    let mut control = transport::control(&endpoint, None).await?;
    let revision = control.welcome().api_revision;
    if let Ok(expected) = std::env::var("REX_EXPECT_REVISION") {
        ensure!(
            revision == expected.parse::<u32>()?,
            "wrong negotiated API revision"
        );
    }
    if let Ok(directory) = std::env::var("REX_KEYMAP_CAPTURE") {
        ensure!(revision == 11, "keymap fixtures require API revision 11");
        ensure!(
            std::env::var("REX_SERVER").as_deref() == Ok(endpoint.as_str()),
            "keymap capture requires REX_SERVER to equal the explicit disposable endpoint"
        );
        let request: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/keymap/request.json"))?;
        let reply = control.call_raw("client.keymap", request).await?;
        std::fs::write(
            std::path::Path::new(&directory).join("response.json"),
            serde_json::to_string_pretty(&reply)? + "\n",
        )?;
        control.close().await?;
        println!("PASS captured revision-{revision} client.keymap reply in {directory}");
        return Ok(());
    }
    println!("PASS negotiated API revision {revision}");
    let created = control
        .call(
            session::Create::new()
                .label(format!("rex-snapshot-probe-{}", std::process::id()))
                .initial_windows([session::CreateInitialWindow::new().layout(Layout::shell())]),
        )
        .await?;
    let session = &created.session_id;
    let block = &created.initial_windows[0].block_ids[0];
    let result=async {
        control.call(session::Attach::new(session)).await?;
        control.call_raw("com.superlogical.terminal.resize",json!({"session_id":session,"block_id":block,"args":{"columns":120,"rows":24}})).await?;
        let command="sh -c 'printf \"\\033[2J\\033[H\"; i=1; while [ $i -le 160 ]; do printf \"REXSEQ_%04d\\n\" $i; i=$((i+1)); sleep 0.01; done; printf \"SNAPSHOT_UNICODE_界_café_é\\nREX_STREAM_DONE\\n\"'\n";
        control.call_raw("com.superlogical.terminal.write",json!({"session_id":session,"block_id":block,"args":{"data":command}})).await?;
        tokio::time::sleep(Duration::from_millis(120)).await;
        let (mut socket, welcome)=open(&mut control,&endpoint,session,block).await?;
        let mut stream=TerminalStream::new(welcome.nonce);
        while stream.terminal.is_none() {output(&mut socket,&mut stream).await?;}
        let offset=stream.received;
        drop(socket);
        tokio::time::sleep(Duration::from_millis(350)).await;
        socket=transport::socket(&endpoint,"/api/data",wire::PROTOCOL).await?;
        let resumed=wire::resume(&mut socket,&welcome.token,offset).await?;
        ensure!(resumed.nonce==welcome.nonce,"resume changed nonce");
        ensure!(resumed.token!=welcome.token,"resume did not rotate token");
        while !stream.text()?.lines().any(|line|line.trim()=="REX_STREAM_DONE") {output(&mut socket,&mut stream).await?;}
        verify_sequence(&stream)?;
        ensure!(stream.snapshot_count==1,"resume restored an extra snapshot");
        println!("PASS concurrent attach + data resume: 160 ordered lines, no duplicates/gaps, Unicode, {} stream bytes",stream.received);

        drop(socket);
        let mut rejected=transport::socket(&endpoint,"/api/data",wire::PROTOCOL).await?;
        ensure!(wire::resume(&mut rejected,&[0;16],stream.received).await.is_err(),"invalid token accepted");
        drop(rejected);
        let (mut socket,welcome)=open(&mut control,&endpoint,session,block).await?;
        let mut fresh=TerminalStream::new(welcome.nonce);
        while fresh.terminal.is_none() {output(&mut socket,&mut fresh).await?;}
        verify_sequence(&fresh)?;
        println!("PASS rejected resume → fresh snapshot: full scrollback restored");

        let client=control.welcome().client.as_ref().context("client identity")?.clone();
        tokio::io::AsyncWriteExt::shutdown(control.transport().socket.get_mut()).await?;
        control=transport::control(&endpoint,Some(&client.reconnect_token)).await?;
        let current=control.welcome().client.as_ref().context("resumed client")?;
        ensure!(current.id==client.id && current.resumed,"control identity was not resumed");
        control.call(session::Attach::new(session)).await?;
        println!("PASS control resume: stable client identity and session reattachment");
        // The remote process and terminal are still intact after reconnect.
        verify_sequence(&fresh)?;
        drop(socket);
        Ok::<_,anyhow::Error>(())
    }.await;
    let cleanup = control.call(session::Destroy::new(session)).await;
    control.close().await?;
    cleanup?;
    result
}
