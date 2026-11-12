//! Run with scripts/prompt-acceptance.py, never against a configured server.
use anyhow::{Context, Result, ensure};
use ghostty_vt::{
    Terminal,
    screen::{CellSemanticContent, RowSemanticPrompt},
    terminal::{Point, PointCoordinate},
};
use rex_rune::{
    connection::{Command, HostConnection},
    terminal::TerminalStream,
    transport, wire,
};
use serde_json::json;
use std::{path::Path, time::Duration};

fn marks(terminal: &Terminal) -> Result<Vec<usize>> {
    let mut rows = Vec::new();
    for y in 0..terminal.total_rows()? {
        let grid = terminal.grid_ref(Point::Screen(PointCoordinate {
            x: 0,
            y: y.try_into()?,
        }))?;
        if grid.row()?.semantic_prompt()? == RowSemanticPrompt::Prompt {
            ensure!(grid.cell()?.semantic_content()? == CellSemanticContent::Prompt);
            rows.push(y);
        }
    }
    Ok(rows)
}

async fn open(
    control: &mut transport::Client,
    endpoint: &str,
    session: &str,
    block: &str,
) -> Result<(transport::Socket, TerminalStream)> {
    let ticket = control
        .call_raw(
            "data.connect",
            json!({"session_id":session,"block_id":block,
        "data_ref_id":"terminal","subprotocol":wire::PROTOCOL}),
        )
        .await?;
    let mut socket = transport::socket(endpoint, "/api/data", wire::PROTOCOL).await?;
    let welcome =
        wire::handshake(&mut socket, ticket["ticket"].as_str().context("ticket")?).await?;
    let mut stream = TerminalStream::new(welcome.nonce);
    while stream.terminal.is_none() {
        output(&mut socket, &mut stream).await?;
    }
    Ok((socket, stream))
}

async fn output(socket: &mut transport::Socket, stream: &mut TerminalStream) -> Result<()> {
    let bytes = tokio::time::timeout(Duration::from_secs(10), wire::next_binary(socket)).await??;
    for chunk in bytes.chunks(17) {
        stream.feed(chunk)?;
    }
    Ok(())
}

async fn wait_file(path: &Path) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(10), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await?;
    Ok(())
}

#[test]
#[ignore = "starts sessions: use scripts/prompt-acceptance.py's private server"]
fn osc133_survives_rex_snapshot_and_live_stream() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(run())
}

async fn run() -> Result<()> {
    let work = std::env::var("REX_PROMPT_WORK").context("use scripts/prompt-acceptance.py")?;
    let work = Path::new(&work).canonicalize()?;
    let endpoint = format!("unix://{}", work.join("rex.sock").display());
    ensure!(
        work.join("private-prompt-fixture").is_file(),
        "private fixture required"
    );
    let quoted = |path: &Path| format!("'{}'", path.display().to_string().replace('\'', "'\\''"));
    let fixture = work.join("emit.sh");
    std::fs::write(
        &fixture,
        format!(
            "printf '\\033[2J\\033[3J\\033[H'\n\
         emit() {{ i=$1; end=$2; while [ $i -le $end ]; do\n\
           printf '\\033]133;A\\007PROMPT_%s> \\033]133;B\\007command\\033]133;C\\007\\n' \"$i\"\n\
           j=0; while [ $j -lt 8 ]; do printf 'output %s %s\\n' \"$i\" \"$j\"; j=$((j+1)); done\n\
           printf '\\033]133;D;0\\007'; i=$((i+1)); done; }}\n\
         emit 1 3\ntouch {}\nwhile [ ! -f {} ]; do sleep .02; done\n\
         emit 4 6\nprintf 'LIVE_DONE\\n'\ntouch {}\n",
            quoted(&work.join("ready")),
            quoted(&work.join("go")),
            quoted(&work.join("done"))
        ),
    )?;
    let mut control = transport::control(&endpoint, None).await?;
    println!("negotiated API {}", control.welcome().api_revision);
    let created = control
        .call_raw(
            "session.create",
            json!({"label":"prompt-contract",
        "initial_windows":[{"layout":{"block":{"flavor":"com.superlogical.terminal.shell",
            "options":{"shell":"/bin/sh"}}}}]}),
        )
        .await?;
    let session = created["session_id"].as_str().context("session")?;
    let block = created["initial_windows"][0]["block_ids"][0]
        .as_str()
        .context("block")?;
    let result = async {
        control
            .call_raw("session.attach", json!({"session_id":session}))
            .await?;
        control
            .call_raw(
                "com.superlogical.terminal.resize",
                json!({"session_id":session,
            "block_id":block,"args":{"columns":80,"rows":8}}),
            )
            .await?;
        control
            .call_raw(
                "com.superlogical.terminal.write",
                json!({"session_id":session,
            "block_id":block,"args":{"data":format!("stty -echo; . {}\n", quoted(&fixture))}}),
            )
            .await?;
        wait_file(&work.join("ready")).await?;
        let (mut socket, mut stream) = open(&mut control, &endpoint, session, block).await?;
        let snapshot_marks = marks(stream.terminal.as_ref().context("terminal")?)?;
        println!(
            "snapshot: bytes={} marks={snapshot_marks:?} scrollback={}",
            stream.received,
            stream.terminal.as_ref().unwrap().scrollback_rows()?
        );
        ensure!(snapshot_marks == [0, 9, 18], "snapshot lost prompt marks");
        std::fs::write(work.join("go"), "go")?;
        while !stream
            .text()?
            .lines()
            .any(|line| line.trim() == "LIVE_DONE")
        {
            output(&mut socket, &mut stream).await?;
        }
        let live_marks = marks(stream.terminal.as_ref().unwrap())?;
        println!(
            "live: bytes={} marks={live_marks:?} scrollback={}",
            stream.received,
            stream.terminal.as_ref().unwrap().scrollback_rows()?
        );
        ensure!(
            live_marks == [0, 9, 18, 27, 36, 45],
            "live stream lost prompt marks"
        );
        ensure!(stream.snapshot_count == 1);
        wait_file(&work.join("done")).await?;
        drop(socket);
        let (_, fresh) = open(&mut control, &endpoint, session, block).await?;
        let fresh_marks = marks(fresh.terminal.as_ref().unwrap())?;
        println!(
            "reattach snapshot: bytes={} marks={fresh_marks:?}",
            fresh.received
        );
        ensure!(fresh_marks == live_marks, "reattach lost prompt marks");
        // Exercise the actual host/data-worker dispatch used by the app, not just the
        // wrapper. This client attaches passively and never writes keys or resizes.
        let host = HostConnection::spawn(
            endpoint.clone(),
            Some(session.into()),
            None,
            None,
            true,
            None,
        );
        let connection = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let connection = host.model.lock().unwrap().terminals.get(block).cloned();
                if let Some(connection) = connection
                    && connection.model.lock().unwrap().frame.is_some()
                {
                    break connection;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await?;
        let before = control
            .call_raw("session.view", json!({"session_id":session}))
            .await?;
        let mut visited = Vec::new();
        for (older, expected) in [
            (true, 6),
            (true, 5),
            (true, 4),
            (true, 3),
            (true, 2),
            (true, 1),
            (true, 1),
            (false, 2),
            (false, 3),
            (false, 4),
            (false, 5),
            (false, 6),
            (false, 6),
        ] {
            let revision = connection.model.lock().unwrap().revision;
            connection.commands.try_send(Command::PromptStep(older))?;
            tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    let rendered = {
                        let model = connection.model.lock().unwrap();
                        (model.revision > revision)
                            .then(|| model.frame.clone())
                            .flatten()
                    };
                    if let Some(frame) = rendered {
                        let line: String = frame.cells[0].iter().map(|c| c.text.as_str()).collect();
                        ensure!(
                            line.starts_with(&format!("PROMPT_{expected}>")),
                            "wrong viewport: {line}"
                        );
                        ensure!(frame.display_row_start(3) == 0, "small pane hid prompt");
                        break Ok::<_, anyhow::Error>(());
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await??;
            visited.push(expected);
        }
        // Ordinary scrolling and explicit bottom/search navigation must not inherit the
        // prompt crop. A subsequent successful prompt jump can anchor the top again.
        for (command, anchored) in [
            (Command::Scroll(-1), false),
            (Command::PromptStep(true), true),
            (Command::Search(String::new()), false),
            (Command::PromptStep(true), true),
            (Command::SearchStep(true), false),
            (Command::PromptStep(true), true),
            (Command::Scroll(10_000), false),
        ] {
            let revision = connection.model.lock().unwrap().revision;
            connection.commands.try_send(command)?;
            tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    let frame = {
                        let model = connection.model.lock().unwrap();
                        (model.revision > revision)
                            .then(|| model.frame.clone())
                            .flatten()
                    };
                    if let Some(frame) = frame {
                        ensure!(frame.prompt_anchor == anchored, "wrong prompt anchor");
                        if anchored {
                            ensure!(frame.display_row_start(3) == 0, "prompt crop lost");
                        } else {
                            ensure!(
                                frame.display_row_start(3) == usize::from(frame.rows) - 3,
                                "ordinary scroll crop changed"
                            );
                        }
                        break Ok::<_, anyhow::Error>(());
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await??;
        }
        ensure!(
            connection.model.lock().unwrap().input_events == 0,
            "navigation sent input"
        );
        let after = control
            .call_raw("session.view", json!({"session_id":session}))
            .await?;
        ensure!(
            before["windows"] == after["windows"],
            "navigation mutated server layout/ownership"
        );
        drop(host);
        ensure!(
            rex_rune::connection::wait_for_hosts(Duration::from_secs(5)),
            "worker did not stop"
        );
        println!("worker viewport prompts={visited:?}; input_events=0; server windows unchanged");
        Ok::<_, anyhow::Error>(())
    }
    .await;
    let cleanup = control
        .call_raw("session.destroy", json!({"session_id":session}))
        .await;
    control.close().await?;
    cleanup?;
    result
}
