//! Views of sessions whose tabs the sidebar lists but this client hasn't attached. They are
//! read with `session.view` (no attach, no data tickets), one request in flight at a time,
//! alongside control traffic rather than in its way.
use crate::{connection::HostModel, transport::Client};
use rexrs::api::{Method, session};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};
use tokio::sync::oneshot;

/// The gap between fetches, so a sweep of a large server doesn't crowd out other requests.
pub const PACE: Duration = Duration::from_millis(25);
const TIMEOUT: Duration = Duration::from_secs(8);
const BACKOFF: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(5),
    Duration::from_secs(30),
];

#[derive(Default)]
pub struct Prefetch {
    /// The sessions the UI lists tabs for. Kept across reconnects; the rest is per connection.
    eligible: HashSet<String>,
    /// Failures so far and when to try again.
    failures: HashMap<String, (usize, Instant)>,
    in_flight: Option<InFlight>,
}

struct InFlight {
    session: String,
    reply: oneshot::Receiver<String>,
    deadline: Instant,
}

#[derive(Deserialize)]
struct Response {
    #[serde(default)]
    seq: u64,
    payload: Option<session::ViewResponse>,
    error: Option<serde_json::Value>,
}

impl Prefetch {
    /// Replace sidebar eligibility. An in-flight read also serves the activity sweep,
    /// so collapsing the sidebar does not discard it.
    pub fn set_eligible(&mut self, sessions: HashSet<String>) {
        self.failures.retain(|id, _| sessions.contains(id));
        self.eligible = sessions;
    }

    /// A new connection starts over.
    pub fn reset(&mut self) {
        self.failures.clear();
        self.in_flight = None;
    }

    pub fn in_flight(&self) -> bool {
        self.in_flight.is_some()
    }

    fn missing<'a>(&'a self, model: &'a HostModel) -> impl Iterator<Item = &'a str> {
        model
            .sessions
            .iter()
            .map(|s| s.session_id.as_str())
            .filter(|id| self.eligible.contains(*id) && !model.views.contains_key(*id))
    }

    fn ready_at(&self, id: &str, now: Instant) -> Instant {
        self.failures.get(id).map_or(now, |&(_, at)| at)
    }

    /// When the next fetch can start: never while one is in flight or nothing is missing.
    pub fn due(&self, model: &HostModel, now: Instant) -> Option<Instant> {
        if self.in_flight.is_some() {
            return None;
        }
        self.missing(model).map(|id| self.ready_at(id, now)).min()
    }

    /// The next session to fetch, if one is due.
    pub fn next(&self, model: &HostModel, now: Instant) -> Option<String> {
        if self.in_flight.is_some() {
            return None;
        }
        self.missing(model)
            .find(|id| self.ready_at(id, now) <= now)
            .map(str::to_owned)
    }

    /// Send `session`'s fetch without waiting for it. Only a failed send is an error.
    pub async fn start(&mut self, control: &mut Client, session: String) -> rexrs::Result<()> {
        let reply = control
            .transport()
            .detach(session::View::NAME, session::View::new(&session))
            .await?;
        self.in_flight = Some(InFlight {
            session,
            reply,
            deadline: Instant::now() + TIMEOUT,
        });
        Ok(())
    }

    /// Wait for the request in flight to answer or time out. Cancel-safe: dropping this
    /// leaves the request in flight.
    pub async fn finished(&mut self) -> (String, Option<String>) {
        let Some(f) = &mut self.in_flight else {
            return std::future::pending().await;
        };
        let deadline = tokio::time::Instant::from_std(f.deadline);
        let text = tokio::time::timeout_at(deadline, &mut f.reply)
            .await
            .ok()
            .and_then(Result::ok);
        let f = self.in_flight.take().expect("a request in flight");
        (f.session, text)
    }

    /// The view and its `seq` from a finished request; a failure schedules a retry instead.
    pub fn result(
        &mut self,
        session: &str,
        text: Option<String>,
        now: Instant,
    ) -> Option<(u64, session::ViewResponse)> {
        let view = text
            .and_then(|t| serde_json::from_str::<Response>(&t).ok())
            .filter(|r| r.error.is_none())
            .and_then(|r| Some((r.seq, r.payload?)));
        if view.is_some() {
            self.failures.remove(session);
        } else {
            // A sweep can fail before the UI's eligibility command arrives. Keep its
            // backoff too, so a later sidebar request does not immediately repeat it.
            let count = self.failures.get(session).map_or(0, |&(n, _)| n);
            let delay = BACKOFF[count.min(BACKOFF.len() - 1)];
            self.failures
                .insert(session.into(), (count + 1, now + delay));
        }
        view
    }
}

/// Resolves when a fetch due at `at` may start, keeping the pace across calls.
pub async fn wait(pace: &mut tokio::time::Interval, at: Instant) {
    tokio::time::sleep_until(tokio::time::Instant::from_std(at)).await;
    pace.tick().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(sessions: &[&str]) -> HostModel {
        HostModel {
            sessions: sessions
                .iter()
                .map(|id| session::Session {
                    session_id: (*id).into(),
                    label: (*id).into(),
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn failures_back_off_without_holding_up_other_sessions() {
        let model = model(&["a", "b"]);
        let mut prefetch = Prefetch::default();
        let now = Instant::now();
        assert_eq!(prefetch.next(&model, now), None, "nothing is eligible");
        prefetch.set_eligible(HashSet::from(["a".into(), "b".into()]));
        assert_eq!(prefetch.next(&model, now).as_deref(), Some("a"));
        let mut at = now;
        for delay in [1, 5, 30, 30] {
            assert!(prefetch.result("a", None, at).is_none());
            assert_eq!(prefetch.next(&model, at).as_deref(), Some("b"));
            assert_eq!(prefetch.due(&model, at), Some(at), "b is due now");
            let retry = at + Duration::from_secs(delay);
            assert_eq!(prefetch.failures["a"].1, retry);
            at = retry;
        }
        let error = r#"{"type":"response","id":"x","error":{"code":"conflict"}}"#;
        assert!(prefetch.result("b", Some(error.into()), now).is_none());
        let view = r#"{"type":"response","id":"x","seq":4,"payload":{"revision":2}}"#;
        let (seq, view) = prefetch.result("a", Some(view.into()), at).unwrap();
        assert_eq!((seq, view.revision), (4, 2));
        assert!(!prefetch.failures.contains_key("a"));
        prefetch.set_eligible(HashSet::from(["a".into()]));
        assert!(
            prefetch.failures.is_empty(),
            "b left, and its retry with it"
        );
        prefetch.reset();
        assert_eq!(prefetch.due(&model, at), Some(at));
    }
}
