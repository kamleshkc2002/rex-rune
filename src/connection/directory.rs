//! One read-only directory request. Results are correlated and never replayed.
//!
//! A listing waits on the terminal machine's filesystem, which can be slow (a network
//! mount), so it is sent without waiting for it, as the server's version and status reads
//! are: keys, events and every other request on the host keep flowing meanwhile.
use super::{Client, Shared};
use rexrs::api::{
    BlockMethod,
    terminal::{ListDir, ListDirResponse},
};
use std::{
    sync::{Arc, Weak},
    time::Instant,
};
use tokio::sync::oneshot;

pub struct Request {
    pub id: u64,
    pub epoch: u64,
    pub session: String,
    pub block: String,
    pub args: ListDir,
    pub deadline: Instant,
    /// Closing the browser cancels requests that have not been sent yet.
    pub open: Weak<()>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Failure {
    Disconnected,
    Unsupported,
    Timeout,
    Request,
}
impl Failure {
    pub fn message(&self) -> &'static str {
        match self {
            Self::Disconnected => "Host is disconnected. Try again after reconnecting.",
            Self::Unsupported => "This Rex server can't list paths. Update Rex on the host.",
            Self::Timeout => "Rex didn't answer in time. Edit the path to try again.",
            Self::Request => "Couldn't list this folder. Edit the path to try again.",
        }
    }
}

#[derive(Clone)]
pub struct Reply {
    pub id: u64,
    pub epoch: u64,
    pub result: Result<ListDirResponse, Failure>,
}

pub(super) fn finish(shared: &Shared, request: &Request, result: Result<ListDirResponse, Failure>) {
    if request.open.strong_count() == 0 {
        return;
    }
    shared.update(|m| {
        m.directory = Some(Arc::new(Reply {
            id: request.id,
            epoch: request.epoch,
            result,
        }));
    });
}

/// Room for a request's type, id and method around its payload.
const ENVELOPE: usize = 256;

/// A host's listing in flight, at most one: a browser asks for one at a time.
#[derive(Default)]
pub(super) struct Reader {
    read: Option<(Request, oneshot::Receiver<String>)>,
}

impl Reader {
    /// Send the request without waiting for its answer. Only a failed send is an error.
    pub async fn start(
        &mut self,
        control: &mut Client,
        shared: &Shared,
        request: Request,
    ) -> rexrs::Result<()> {
        if request.open.strong_count() == 0 {
            return Ok(());
        }
        let epoch = shared.model.lock().expect("host model").connection_epoch;
        if epoch != request.epoch {
            finish(shared, &request, Err(Failure::Disconnected));
            return Ok(());
        }
        if request.deadline <= Instant::now() {
            finish(shared, &request, Err(Failure::Timeout));
            return Ok(());
        }
        // A new browser's first read can arrive while a closed one's is still out.
        self.abandon(shared, Failure::Request);
        let payload = serde_json::json!({"session_id":request.session, "block_id":request.block, "args":request.args});
        // A detached request skips the client's own size check, and a server closes the
        // connection on a message over its limit: that must fail this listing only.
        let size = serde_json::to_vec(&payload).map_or(usize::MAX, |bytes| bytes.len());
        if size.saturating_add(ENVELOPE) > control.welcome().limits.max_message_bytes {
            finish(shared, &request, Err(Failure::Request));
            return Ok(());
        }
        match control.transport().detach(ListDir::NAME, payload).await {
            Ok(reply) => {
                self.read = Some((request, reply));
                Ok(())
            }
            Err(error) => {
                finish(shared, &request, Err(Failure::Disconnected));
                Err(error)
            }
        }
    }

    pub fn in_flight(&self) -> bool {
        self.read.is_some()
    }

    /// Wait for the listing in flight to answer or run out of time, and record it.
    /// Cancel-safe: dropping this leaves the request in flight.
    pub async fn finished(&mut self, shared: &Shared) {
        let Some((request, reply)) = &mut self.read else {
            return std::future::pending().await;
        };
        let deadline = tokio::time::Instant::from_std(request.deadline);
        let result = match tokio::time::timeout_at(deadline, reply).await {
            Err(_) => Err(Failure::Timeout),
            // The socket went away with the answer still owed.
            Ok(Err(_)) => Err(Failure::Disconnected),
            Ok(Ok(text)) => parse(&text),
        };
        if let Some((request, _)) = self.read.take() {
            finish(shared, &request, result);
        }
    }

    /// The connection ended, or another listing takes this one's place: its answer, if
    /// one still comes, is dropped.
    pub fn abandon(&mut self, shared: &Shared, failure: Failure) {
        if let Some((request, _)) = self.read.take() {
            finish(shared, &request, Err(failure));
        }
    }
}

/// A raw response: its listing, or why there is none.
fn parse(text: &str) -> Result<ListDirResponse, Failure> {
    let mut response: serde_json::Value =
        serde_json::from_str(text).map_err(|_| Failure::Request)?;
    if !response["error"].is_null() {
        return Err(match response["error"]["code"].as_str() {
            Some("method_not_found" | "revision_too_low" | "not_supported") => Failure::Unsupported,
            _ => Failure::Request,
        });
    }
    serde_json::from_value(response["payload"].take()).map_err(|_| Failure::Request)
}
