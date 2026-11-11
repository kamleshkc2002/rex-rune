//! Receives one reserved file on its ticket: opens the data stream, writes the chunks straight
//! to a partial file, and decides how it ended.
//!
//! The stream's end is the rule observed in D0: the last data message, one empty binary message,
//! then a Close with status 1000. That whole shape is success; every other ending, including a
//! 1000 without the empty message, is a failure and the partial file is deleted.
use super::{Config, Destination, Failure, FreeSpace, Meter};
use crate::{transport, wire};
use futures_util::{SinkExt, StreamExt};
use std::{
    future::Future,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio_tungstenite::tungstenite::{
    Message,
    protocol::{CloseFrame, frame::coding::CloseCode},
};

/// Why a transfer is being stopped from outside.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// The person cancelled it.
    Cancel,
    /// The control connection that issued the ticket ended.
    ConnectionLost,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Saved { path: PathBuf, bytes: u64 },
    Cancelled,
    Failed(Failure),
}

/// One file to receive.
pub struct Job<'a> {
    /// The host's data endpoint: where uploads go too (the local end of an SSH forward).
    pub endpoint: &'a str,
    pub ticket: &'a str,
    /// The size Rex reported, a hint.
    pub hint: u64,
    /// The sanitized name to save under.
    pub name: &'a str,
    pub destination: &'a Destination,
    pub config: &'a Config,
}

/// How long to wait for the Close exchange when ending a stream on purpose. The server frees
/// its file on any close, so this only has to try.
const GOODBYE: Duration = Duration::from_secs(1);

async fn goodbye(socket: &mut transport::Socket) {
    let frame = CloseFrame {
        code: CloseCode::Normal,
        reason: "".into(),
    };
    let _ = tokio::time::timeout(GOODBYE, async {
        let _ = socket.close(Some(frame)).await;
    })
    .await;
}

/// Receives the file. `stop` ends it from outside, deleting the partial file; `progress` is
/// called with the bytes written so far after every chunk.
pub async fn receive(
    job: &Job<'_>,
    stop: impl Future<Output = Stop>,
    mut progress: impl FnMut(u64),
) -> Outcome {
    tokio::pin!(stop);
    let stopped = |stop: Stop| match stop {
        Stop::Cancel => Outcome::Cancelled,
        Stop::ConnectionLost => Outcome::Failed(Failure::ConnectionLost),
    };

    // Open the stream promptly: an unopened ticket holds the file and a slot on the server.
    let connected = async {
        let mut socket = transport::socket(job.endpoint, "/api/data", super::PROTOCOL)
            .await
            .map_err(|e| Failure::Connection(format!("{e:#}")))?;
        wire::handshake(&mut socket, job.ticket)
            .await
            .map_err(
                |error| match error.downcast_ref::<wire::HandshakeRejected>() {
                    Some(wire::HandshakeRejected(reason)) => Failure::Rejected(*reason),
                    None => Failure::Connection(format!("{error:#}")),
                },
            )?;
        Ok::<_, Failure>(socket)
    };
    let mut socket = tokio::select! {
        biased;
        s = &mut stop => return stopped(s),
        socket = connected => match socket {
            Ok(socket) => socket,
            Err(failure) => return Outcome::Failed(failure),
        },
    };

    let mut partial = match job
        .destination
        .create_partial(job.name, &job.config.records)
    {
        Ok(partial) => partial,
        Err(error) => {
            return Outcome::Failed(Failure::Disk(format!("{error:#}")));
        }
    };
    let space: Arc<dyn FreeSpace> = job
        .config
        .space
        .clone()
        .unwrap_or_else(|| Arc::new(job.destination.clone()));
    let mut meter = Meter::new(job.config.limits, job.hint, Instant::now());
    let mut ended = false;

    let failure = loop {
        let next = tokio::time::timeout_at(meter.deadline().into(), socket.next());
        let message = tokio::select! {
            biased;
            s = &mut stop => {
                goodbye(&mut socket).await;
                return stopped(s);
            }
            message = next => message,
        };
        let message = match message {
            Err(_elapsed) => break Failure::Stalled,
            Ok(None) => {
                break Failure::Closed {
                    code: None,
                    reason: String::new(),
                };
            }
            Ok(Some(Err(error))) => break Failure::Connection(error.to_string()),
            Ok(Some(Ok(message))) => message,
        };
        match message {
            Message::Binary(bytes) => {
                if ended {
                    break Failure::Protocol("data after the end of the file".into());
                }
                if bytes.is_empty() {
                    ended = true;
                    continue;
                }
                if let Err(error) = partial.write(&bytes).await {
                    break Failure::Disk(format!("Writing the file failed: {error}"));
                }
                if let Err(failure) = meter.wrote(bytes.len() as u64, Instant::now(), &*space) {
                    break failure;
                }
                progress(meter.received());
            }
            Message::Close(frame) => {
                let code = frame.as_ref().map(|f| u16::from(f.code));
                let reason = frame.map(|f| f.reason.to_string()).unwrap_or_default();
                // Answer the close, then judge it.
                goodbye(&mut socket).await;
                if code != Some(1000) {
                    break Failure::Closed { code, reason };
                }
                if !ended {
                    break Failure::Protocol("the stream closed without its end marker".into());
                }
                let received = meter.received();
                if received < job.hint {
                    break Failure::Short {
                        received,
                        expected: job.hint,
                    };
                }
                return match partial.finish_with(job.name, job.config.placement).await {
                    Ok(path) => Outcome::Saved {
                        path,
                        bytes: received,
                    },
                    Err(error) => Outcome::Failed(Failure::Disk(format!("{error:#}"))),
                };
            }
            Message::Text(_) => break Failure::Protocol("a text message".into()),
            // Pings are answered by the socket; flush so the pong goes out.
            Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {
                if let Err(error) = socket.flush().await {
                    break Failure::Connection(error.to_string());
                }
            }
        }
    };
    // The server frees its file when this socket closes.
    drop(partial);
    drop(socket);
    Outcome::Failed(failure)
}
