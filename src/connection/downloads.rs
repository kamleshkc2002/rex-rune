//! Downloads on a host connection (API revision 13). The control connection reserves the file
//! with `download_begin`; the bytes then arrive on a separate data stream, on this same host's
//! data endpoint, filled in by a task that lives only as long as this connection.
//!
//! Nothing is retried or re-sent. A transfer ends as saved, cancelled or failed, and a failed or
//! cancelled one has its partial file deleted. When the control connection ends or is replaced,
//! every transfer in flight fails: Rex ends their streams when the connection that issued the
//! tickets goes away, so a reconnect can't carry one over.
use super::{Client, Shared};
use crate::download::{
    self, Config, Event, Failure, FreeSpace, MIN_API_REVISION, Transfer, name,
    store::Destination,
    stream::{self, Job, Outcome, Stop},
};
use anyhow::Result;
use rexrs::api::{BlockMethod, terminal::DownloadBegin};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

/// A download to start: the pane's captured session and block, and what to fetch.
pub struct Request {
    /// Chosen by the caller with [`download::next_id`]; names the transfer in the model and in
    /// `HostCommand::DownloadConfirm` and `HostCommand::DownloadCancel`.
    pub id: u64,
    pub session: String,
    pub block: String,
    /// The file on the host: absolute, `~/…`, or relative to the foreground program's directory.
    pub path: String,
    /// The folder to save into; it must be a real folder, not a symbolic link.
    pub directory: PathBuf,
    pub config: Config,
}

impl Request {
    pub fn new(
        id: u64,
        session: impl Into<String>,
        block: impl Into<String>,
        path: impl Into<String>,
        directory: impl Into<PathBuf>,
    ) -> Self {
        Self {
            id,
            session: session.into(),
            block: block.into(),
            path: path.into(),
            directory: directory.into(),
            config: Config::default(),
        }
    }
}

/// What the UI can tell a running transfer.
pub(super) enum Signal {
    Confirm,
    Cancel,
}

/// The transfers of one control connection. Dropping it ends them all: each task finds its
/// signal channel closed and fails with [`Failure::ConnectionLost`].
#[derive(Default)]
pub(super) struct Downloads {
    tasks: HashMap<u64, flume::Sender<Signal>>,
}

fn update(shared: &Shared, id: u64, event: Event) {
    shared.update(|m| {
        if let Some(transfer) = m.downloads.iter_mut().find(|t| t.id == id) {
            transfer.apply(event);
        }
    });
}

/// Adds a transfer that failed before it started.
pub(super) fn refuse(shared: &Shared, request: Request, failure: Failure) {
    shared.update(|m| {
        let mut transfer = new_transfer(&request, m.connection_epoch);
        transfer.apply(Event::Failed(failure));
        download::push(&mut m.downloads, transfer);
    });
}

fn new_transfer(request: &Request, epoch: u64) -> Transfer {
    Transfer::new(
        request.id,
        request.session.clone(),
        request.block.clone(),
        request.path.clone(),
        request.directory.clone(),
        epoch,
    )
}

impl Downloads {
    pub(super) fn signal(&mut self, id: u64, signal: Signal) {
        self.tasks.retain(|_, tx| !tx.is_disconnected());
        if let Some(tx) = self.tasks.get(&id) {
            let _ = tx.send(signal);
        }
    }

    /// Reserves the file and starts receiving it. A refusal or failure is recorded on the
    /// transfer and returns `Ok`; an `Err` means the control connection itself failed.
    pub(super) async fn start(
        &mut self,
        control: &mut Client,
        shared: &Shared,
        endpoint: &str,
        request: Request,
    ) -> Result<()> {
        let id = request.id;
        let running = download::active(&shared.model.lock().expect("host model").downloads);
        if running >= request.config.limits.per_host {
            let limit = request.config.limits.per_host;
            refuse(shared, request, Failure::TooManyActive { limit });
            return Ok(());
        }
        shared.update(|m| {
            let transfer = new_transfer(&request, m.connection_epoch);
            download::push(&mut m.downloads, transfer);
        });
        // The revision is the gate: below it, nothing is sent.
        if control.welcome().api_revision < MIN_API_REVISION {
            update(shared, id, Event::Failed(Failure::ServerTooOld));
            return Ok(());
        }
        let destination = match Destination::open(&request.directory) {
            Ok(destination) => destination,
            Err(error) => {
                update(
                    shared,
                    id,
                    Event::Failed(Failure::Destination(format!("{error:#}"))),
                );
                return Ok(());
            }
        };
        // No `if_none_match`: there is no cache, so Rex never has a reason to say not modified.
        let begin = DownloadBegin::new(download::new_ref()?, request.path.clone())
            .on(request.session.clone(), request.block.clone());
        let reply = match tokio::time::timeout(Duration::from_secs(8), control.call(begin)).await {
            Err(_) => {
                update(shared, id, Event::Failed(Failure::NoAnswer));
                return Ok(());
            }
            Ok(Ok(reply)) => reply,
            Ok(Err(rexrs::Error::Api(
                rexrs::ApiError::RevisionTooLow { .. } | rexrs::ApiError::MethodNotFound { .. },
            ))) => {
                update(shared, id, Event::Failed(Failure::ServerTooOld));
                return Ok(());
            }
            Ok(Err(rexrs::Error::Api(error))) => {
                let message = download::display_text(error.message());
                update(shared, id, Event::Failed(Failure::Server(message)));
                return Ok(());
            }
            Ok(Err(error)) => {
                update(shared, id, Event::Failed(Failure::ConnectionLost));
                return Err(error.into());
            }
        };
        if reply.not_modified == Some(true) {
            let failure = Failure::UnexpectedReply("the file was reported unchanged".into());
            update(shared, id, Event::Failed(failure));
            return Ok(());
        }
        let Some(ticket) = reply.ticket.clone() else {
            let failure = Failure::UnexpectedReply("no data ticket".into());
            update(shared, id, Event::Failed(failure));
            return Ok(());
        };
        let (tx, signals) = flume::unbounded();
        self.tasks.retain(|_, tx| !tx.is_disconnected());
        self.tasks.insert(id, tx);
        let task = Task {
            shared: shared.clone(),
            endpoint: endpoint.to_owned(),
            id,
            ticket,
            reply,
            destination,
            config: request.config,
            signals,
        };
        tokio::task::spawn_local(task.run());
        Ok(())
    }
}

struct Task {
    shared: Shared,
    endpoint: String,
    id: u64,
    ticket: String,
    reply: rexrs::api::terminal::DownloadBeginResponse,
    destination: Destination,
    config: Config,
    signals: flume::Receiver<Signal>,
}

/// Margin kept before a ticket's expiry when waiting for a confirmation.
const TICKET_MARGIN: Duration = Duration::from_secs(5);
const PROGRESS_EVERY: Duration = Duration::from_millis(100);

impl Task {
    async fn run(self) {
        let Task {
            shared,
            endpoint,
            id,
            ticket,
            reply,
            destination,
            config,
            signals,
        } = self;
        let hint = u64::try_from(reply.size).unwrap_or(0);
        let name = name::sanitize(&reply.name);
        let resolved = download::display_text(&reply.path);
        let space: Arc<dyn FreeSpace> = config
            .space
            .clone()
            .unwrap_or_else(|| Arc::new(destination.clone()));
        let free = space.free().ok();
        if let Some(free) = free
            && free < config.limits.reserve
        {
            update(&shared, id, Event::Failed(Failure::LowSpace { free }));
            return;
        }
        let confirmation = free.and_then(|free| config.limits.confirmation(hint, free));
        update(
            &shared,
            id,
            Event::Begun {
                resolved,
                name: name.clone(),
                hint,
                confirmation,
            },
        );
        if confirmation.is_some() {
            // Rex holds the file for the ticket's lifetime only.
            let lifetime = Duration::from_millis(reply.expires_in_ms.unwrap_or(60_000));
            let deadline = tokio::time::Instant::now() + lifetime.saturating_sub(TICKET_MARGIN);
            tokio::select! {
                signal = signals.recv_async() => match signal {
                    Ok(Signal::Confirm) => {}
                    Ok(Signal::Cancel) => return update(&shared, id, Event::Cancelled),
                    Err(_) => return update(&shared, id, Event::Failed(Failure::ConnectionLost)),
                },
                _ = tokio::time::sleep_until(deadline) => {
                    return update(&shared, id, Event::Failed(Failure::ConfirmationExpired));
                }
            }
            update(&shared, id, Event::Confirmed);
        }
        let stop = async {
            loop {
                match signals.recv_async().await {
                    Ok(Signal::Cancel) => return Stop::Cancel,
                    Ok(Signal::Confirm) => {}
                    Err(_) => return Stop::ConnectionLost,
                }
            }
        };
        let job = Job {
            endpoint: &endpoint,
            ticket: &ticket,
            hint,
            name: &name,
            destination: &destination,
            config: &config,
        };
        let mut last = Instant::now();
        let outcome = stream::receive(&job, stop, |received| {
            if last.elapsed() >= PROGRESS_EVERY {
                last = Instant::now();
                update(&shared, id, Event::Progress { received });
            }
        })
        .await;
        update(
            &shared,
            id,
            match outcome {
                Outcome::Saved { path, bytes } => Event::Saved { path, bytes },
                Outcome::Cancelled => Event::Cancelled,
                Outcome::Failed(failure) => Event::Failed(failure),
            },
        );
    }
}
