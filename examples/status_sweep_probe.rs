//! Read-only production-worker measurement. Use only with scripts/status-sweep-private.py.
use rex_rune::connection::{HostCommand, HostConnection};
use serde_json::json;
use std::time::{Duration, Instant};

fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(
        args.len() == 3,
        "private Unix endpoint and session required"
    );
    anyhow::ensure!(
        args[1].starts_with("unix://") && args[1].contains("/rex-m32-sweep/"),
        "only the sweep's private scratch socket is accepted"
    );
    let host = HostConnection::spawn(
        args[1].clone(),
        Some(args[2].clone()),
        None,
        None,
        false,
        None,
    );
    // How often to send the comparison read. It must be well above the link's round trip:
    // each read is awaited on the host loop with its follow-up, and a loop kept
    // continuously busy gives background reads no turn at all.
    let period = Duration::from_millis(
        std::env::var("REX_SWEEP_PROBE_PERIOD_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100),
    );
    let start = Instant::now();
    let mut connected = None;
    let mut finished = None;
    let mut last = Instant::now();
    loop {
        anyhow::ensure!(start.elapsed() < Duration::from_secs(280), "sweep timeout");
        {
            let m = host.model.lock().unwrap();
            if m.connected {
                connected.get_or_insert_with(Instant::now);
                if !m.activity_reading && finished.is_none() {
                    anyhow::ensure!(!m.activity_incomplete, "incomplete sweep");
                    finished = Some(Instant::now());
                    println!(
                        "{}",
                        json!({"connected_to_finished_ms":connected.unwrap().elapsed().as_secs_f64()*1000.,
                        "sessions":m.sessions.len(), "views":m.views.len()})
                    );
                }
            }
        }
        if finished.is_some_and(|at| at.elapsed() >= Duration::from_secs(5)) {
            break;
        }
        if connected.is_some() && last.elapsed() >= period {
            // A selected-session read on the SAME host command loop during and after the
            // sweep. The wire proxy measures both this call and its normal view refresh.
            host.commands.send(HostCommand::Request {
                method: "session.view".into(),
                payload: json!({"session_id":args[2]}),
            })?;
            last = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    drop(host);
    Ok(())
}
