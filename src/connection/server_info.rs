//! What a Rex server says about itself: its version and build (`server.version`) and its
//! process, sessions, listeners and Tailscale state (`server.status`).
//!
//! The status needs no request of its own: every `session.bootstrap` reply carries it, as
//! `server.status` returns it, so each connection has it once it is ready. The version is
//! read for a front end that asks with `HostCommand::Identify`: from then on each control
//! connection sends `server.version` once, as soon as it is ready, and reads
//! `server.status` again when a `tailscale_status_changed` event says it changed.
//!
//! Both methods only read: neither attaches to a session, creates one or changes the server.
//! The requests are sent without waiting, so a slow or silent server holds nothing up, and a
//! refusal is quiet: the facts stay unreported and the connection carries on.
use crate::transport::Client;
use rexrs::api::{
    Method,
    server::{Listener, Status, Tailscale, Version},
};
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

const TIMEOUT: Duration = Duration::from_secs(8);

/// The answer to one of the two reads.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Answer<T> {
    /// Not asked, or not answered yet.
    #[default]
    Pending,
    /// The server refused, answered something unreadable or did not answer in time.
    Unreported,
    Reported(T),
}

impl<T> Answer<T> {
    pub fn reported(&self) -> Option<&T> {
        match self {
            Self::Reported(value) => Some(value),
            _ => None,
        }
    }

    /// Nothing more will arrive on this connection.
    pub fn settled(&self) -> bool {
        !matches!(self, Self::Pending)
    }
}

/// `server.version`, each member as the server gave it. An older server leaves members out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Build {
    /// As `rex version` prints it, such as `0.1.0-preview.15`.
    pub rex_version: Option<String>,
    /// The system and CPU the server was built for, such as `linux_amd64`.
    pub platform: Option<String>,
    pub commit: Option<String>,
    /// An RFC 3339 timestamp.
    pub built_at: Option<String>,
    /// Built from source with uncommitted changes.
    pub dirty: Option<bool>,
}

/// `server.status`, each member as the server gave it. An older server leaves members out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Process {
    pub pid: Option<i64>,
    /// A random id for this run of the server.
    pub instance_id: Option<String>,
    /// The machine and account the server runs as.
    pub host_id: Option<String>,
    /// An RFC 3339 timestamp.
    pub started_at: Option<String>,
    pub session_count: Option<u64>,
    /// Primary first; empty from servers older than API revision 12.
    pub listeners: Vec<Listener>,
    /// Present only when the server can run built-in Tailscale.
    pub tailscale: Option<Tailscale>,
}

/// A host's server identity on its current control connection; kept, as last read, while the
/// host is disconnected.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ServerInfo {
    /// The oldest and newest API revisions the server speaks: from the welcome, else from
    /// either read. None when the server said neither.
    pub api_revisions: Option<(u32, u32)>,
    pub build: Answer<Build>,
    pub process: Answer<Process>,
}

impl ServerInfo {
    /// A new connection's identity: what its welcome said, the rest still to come.
    pub fn welcomed(welcome: &rexrs::Welcome) -> Self {
        Self {
            api_revisions: range(
                u64::from(welcome.api_min_revision),
                u64::from(welcome.api_server_revision),
            ),
            ..Self::default()
        }
    }

    /// The status in a `session.bootstrap` reply.
    pub(super) fn bootstrapped(&mut self, status: &rexrs::api::session::ServerStatus) {
        // Read member by member, as a `server.status` reply is, so both give the same facts.
        if let Ok(payload) = serde_json::to_value(status) {
            self.status(&payload);
        }
    }

    fn revisions(&mut self, payload: &Value) {
        if self.api_revisions.is_none() {
            self.api_revisions = range(
                payload["api_min_revision"].as_u64().unwrap_or(0),
                payload["api_revision"].as_u64().unwrap_or(0),
            );
        }
    }

    fn status(&mut self, payload: &Value) {
        self.revisions(payload);
        self.process = Answer::Reported(process(payload));
    }

    /// The server's version, when it reported one.
    pub fn rex_version(&self) -> Option<&str> {
        self.build.reported()?.rex_version.as_deref()
    }

    fn apply(&mut self, reply: Reply) {
        match reply {
            Reply::Version(text) => match payload(text) {
                Some(payload) => {
                    self.revisions(&payload);
                    self.build = Answer::Reported(build(&payload));
                }
                None => self.build = Answer::Unreported,
            },
            Reply::Status(text) => match payload(text) {
                Some(payload) => self.status(&payload),
                // A status read again and refused leaves the earlier answer standing.
                None if self.process.reported().is_some() => {}
                None => self.process = Answer::Unreported,
            },
        }
    }
}

/// A revision range, when the server gave both ends. A missing end is 0 on the wire.
fn range(oldest: u64, newest: u64) -> Option<(u32, u32)> {
    let (oldest, newest) = (u32::try_from(oldest).ok()?, u32::try_from(newest).ok()?);
    (oldest != 0 && newest >= oldest).then_some((oldest, newest))
}

/// A response's payload: none for an error, an unanswered request or anything unreadable.
fn payload(text: Option<String>) -> Option<Value> {
    let mut response: Value = serde_json::from_str(&text?).ok()?;
    if !response["error"].is_null() {
        return None;
    }
    let payload = response["payload"].take();
    payload.is_object().then_some(payload)
}

fn string(value: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned)
}

fn build(payload: &Value) -> Build {
    Build {
        rex_version: string(&payload["rex_version"]),
        platform: string(&payload["platform"]),
        commit: string(&payload["commit"]),
        built_at: string(&payload["built_at"]),
        dirty: payload["dirty"].as_bool(),
    }
}

fn process(payload: &Value) -> Process {
    Process {
        pid: payload["pid"].as_i64(),
        instance_id: string(&payload["instance_id"]),
        host_id: string(&payload["host_id"]),
        started_at: string(&payload["started_at"]),
        session_count: payload["session_count"].as_u64(),
        // One listener this client can't read doesn't hide the others.
        listeners: payload["listeners"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|l| serde_json::from_value(l.clone()).ok())
            .collect(),
        tailscale: serde_json::from_value(payload["tailscale"].clone()).ok(),
    }
}

/// A finished read: the raw response, or none when it timed out or the socket went away.
pub enum Reply {
    Version(Option<String>),
    Status(Option<String>),
}

struct InFlight {
    reply: oneshot::Receiver<String>,
    deadline: Instant,
}

impl InFlight {
    fn new(reply: oneshot::Receiver<String>) -> Self {
        Self {
            reply,
            deadline: Instant::now() + TIMEOUT,
        }
    }
}

/// One host's reads, across its connections.
#[derive(Default)]
pub struct Reader {
    /// The front end asked. Kept across reconnects; the rest is per connection.
    wanted: bool,
    asked: bool,
    version: Option<InFlight>,
    status: Option<InFlight>,
}

impl Reader {
    /// Read on every connection from now on.
    pub fn want(&mut self) {
        self.wanted = true;
    }

    /// A new connection starts over; answers to the last one's requests are ignored.
    pub fn reset(&mut self) {
        *self = Self {
            wanted: self.wanted,
            ..Self::default()
        };
    }

    /// Send this connection's version read, once, without waiting for it. Only a failed
    /// send is an error.
    pub async fn start(&mut self, control: &mut Client) -> rexrs::Result<()> {
        if !self.wanted || self.asked {
            return Ok(());
        }
        self.asked = true;
        let reply = control
            .transport()
            .detach(Version::NAME, Version::new())
            .await?;
        self.version = Some(InFlight::new(reply));
        Ok(())
    }

    /// Read the status again after the server said it changed. An answer still on its way
    /// is dropped for the newer one.
    pub async fn reread_status(&mut self, control: &mut Client) -> rexrs::Result<()> {
        if !self.asked {
            return Ok(());
        }
        let reply = control
            .transport()
            .detach(Status::NAME, Status::new())
            .await?;
        self.status = Some(InFlight::new(reply));
        Ok(())
    }

    pub fn in_flight(&self) -> bool {
        self.version.is_some() || self.status.is_some()
    }

    /// Wait for a read in flight to answer or time out. Cancel-safe: dropping this leaves
    /// the requests in flight.
    pub async fn finished(&mut self) -> Reply {
        tokio::select! {
            text = answer(&mut self.version) => Reply::Version(text),
            text = answer(&mut self.status) => Reply::Status(text),
        }
    }
}

async fn answer(slot: &mut Option<InFlight>) -> Option<String> {
    let Some(read) = slot else {
        return std::future::pending().await;
    };
    let deadline = tokio::time::Instant::from_std(read.deadline);
    let text = tokio::time::timeout_at(deadline, &mut read.reply)
        .await
        .ok()
        .and_then(Result::ok);
    *slot = None;
    text
}

/// Record a finished read in the host's model.
pub(super) fn record(shared: &super::Shared, reply: Reply) {
    shared.update(|m| m.server_info.apply(reply));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn response(payload: Value) -> Option<String> {
        Some(json!({"type":"response","id":"detached:1","payload":payload}).to_string())
    }

    #[test]
    fn both_reads_fill_every_member_the_server_gives() {
        let mut info = ServerInfo::default();
        info.apply(Reply::Version(response(json!({
            "rex_version":"0.1.0-preview.15", "platform":"linux_amd64", "commit":"9a754d",
            "built_at":"2026-11-01T10:00:00Z", "dirty":false,
            "api_revision":13, "api_min_revision":10,
        }))));
        info.apply(Reply::Status(response(json!({
            "pid":4242, "instance_id":"instance:1", "host_id":"host:1",
            "started_at":"2026-11-10T08:00:00Z", "exit_empty":false, "session_count":3,
            "config":{"generation":1,"hash":""},
            "listeners":[
                {"endpoint":"unix:///run/rex.sock","transport":"unix","admission":"peer_credentials","primary":true},
                {"endpoint":7297},
                {"endpoint":"tailnet://:7297","transport":"tailscale","addresses":null},
            ],
            "tailscale":{"state":"running","dns_name":"rex.tail1234.ts.net","allow_tailnet_members":false},
        }))));
        assert_eq!(info.rex_version(), Some("0.1.0-preview.15"));
        assert_eq!(info.api_revisions, Some((10, 13)));
        let build = info.build.reported().unwrap();
        assert_eq!(build.platform.as_deref(), Some("linux_amd64"));
        assert_eq!(build.commit.as_deref(), Some("9a754d"));
        assert_eq!(build.dirty, Some(false));
        let process = info.process.reported().unwrap();
        assert_eq!(process.pid, Some(4242));
        assert_eq!(process.session_count, Some(3));
        assert_eq!(process.instance_id.as_deref(), Some("instance:1"));
        assert_eq!(process.host_id.as_deref(), Some("host:1"));
        assert_eq!(
            process
                .listeners
                .iter()
                .map(|l| l.transport.as_str())
                .collect::<Vec<_>>(),
            ["unix", "tailscale"],
            "an unreadable listener is skipped, not the list"
        );
        assert_eq!(process.tailscale.as_ref().unwrap().state, "running");
    }

    #[test]
    fn an_older_server_leaves_members_unreported_and_a_refusal_is_not_an_answer() {
        let mut info = ServerInfo::default();
        assert!(!info.build.settled() && !info.process.settled());
        // An older API: a version string and nothing else; a status without listeners,
        // Tailscale or a host id.
        info.apply(Reply::Version(response(
            json!({"rex_version":"0.0.9", "platform":""}),
        )));
        info.apply(Reply::Status(response(json!({"pid":7, "session_count":0}))));
        let build = info.build.reported().unwrap();
        assert_eq!(build.rex_version.as_deref(), Some("0.0.9"));
        assert_eq!(
            (&build.platform, &build.commit, &build.built_at, build.dirty),
            (&None, &None, &None, None)
        );
        let process = info.process.reported().unwrap();
        assert_eq!((process.pid, process.session_count), (Some(7), Some(0)));
        assert!(process.listeners.is_empty() && process.tailscale.is_none());
        assert_eq!(info.api_revisions, None);
        // The status a bootstrap reply carries is read the same way.
        let mut booted = ServerInfo::default();
        booted.bootstrapped(&rexrs::api::session::ServerStatus {
            pid: 7,
            instance_id: "instance:1".into(),
            api_revision: Some(13),
            api_min_revision: Some(10),
            ..Default::default()
        });
        let from_boot = booted.process.reported().unwrap();
        assert_eq!(from_boot.pid, Some(7));
        assert_eq!(from_boot.instance_id.as_deref(), Some("instance:1"));
        assert_eq!(
            (&from_boot.started_at, &from_boot.host_id),
            (&None, &None),
            "an empty member is not a fact"
        );
        assert_eq!(booted.api_revisions, Some((10, 13)));
        assert!(!booted.build.settled(), "the version is its own read");
        // A later status read that fails keeps this one.
        info.apply(Reply::Status(None));
        assert_eq!(info.process.reported().unwrap().pid, Some(7));

        let refused = json!({"type":"response","id":"detached:1",
            "error":{"code":"method_not_found","message":"no such method","status":404}});
        for reply in [
            Some(refused.to_string()),
            None,
            Some("not json".into()),
            response(json!(null)),
        ] {
            let mut info = ServerInfo::default();
            info.apply(Reply::Version(reply.clone()));
            info.apply(Reply::Status(reply));
            assert_eq!(info.build, Answer::Unreported);
            assert_eq!(info.process, Answer::Unreported);
            assert_eq!(info.rex_version(), None);
        }
    }

    #[test]
    fn the_welcome_gives_the_revision_range_before_either_read() {
        let welcome = |min, newest| {
            serde_json::from_value::<rexrs::Welcome>(json!({
                "protocol":"rex.control.v1", "limits":{"max_message_bytes":65536},
                "api_revision":12, "api_min_revision":min, "api_server_revision":newest,
            }))
            .unwrap()
        };
        assert_eq!(
            ServerInfo::welcomed(&welcome(10, 13)).api_revisions,
            Some((10, 13))
        );
        // A server from before the welcome carried them says 0 for both.
        let mut info = ServerInfo::welcomed(&welcome(0, 0));
        assert_eq!(info.api_revisions, None);
        info.apply(Reply::Version(response(
            json!({"rex_version":"1", "api_revision":11, "api_min_revision":10}),
        )));
        assert_eq!(info.api_revisions, Some((10, 11)));
        // The welcome's range is not replaced by a read.
        let mut info = ServerInfo::welcomed(&welcome(10, 13));
        info.apply(Reply::Status(response(
            json!({"api_revision":12, "api_min_revision":11}),
        )));
        assert_eq!(info.api_revisions, Some((10, 13)));
    }
}
