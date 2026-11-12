//! One pass of old program status, after selected-session restoration. Layout reads use
//! Prefetch's detached slot; the host loop shares its pace and never runs both slots at once.
use super::{Client, ControlState, Shared, activity, view_blocks};
use serde_json::{Value, json};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

pub(super) enum Work {
    View(String),
    Status(String, String),
}

/// Scan the current inventory, not a frozen startup queue: new sessions and panes join in
/// server order. Attempt sets live for the whole connection, even after a failure/removal.
pub(super) fn next(shared: &Shared, state: &ControlState) -> Option<Work> {
    let model = shared.model.lock().expect("host model");
    // A session a live report arrived for goes first: still one read, through the same slot.
    if let Some(id) = state.urgent_views.iter().find(|id| {
        !state.viewed.contains(*id)
            && !model.views.contains_key(*id)
            && model.sessions.iter().any(|s| &s.session_id == *id)
    }) {
        return Some(Work::View(id.clone()));
    }
    for session in &model.sessions {
        let id = &session.session_id;
        if let Some(view) = model.views.get(id) {
            if let Some(block) = view_blocks(view).find(|b| !state.status_queried.contains(b)) {
                return Some(Work::Status(id.clone(), block));
            }
        } else if !state.viewed.contains(id) {
            return Some(Work::View(id.clone()));
        }
    }
    None
}

pub(super) fn coverage(shared: &Shared, state: &ControlState, reading: bool) {
    let incomplete = {
        let m = shared.model.lock().expect("host model");
        m.sessions.iter().any(|s| match m.views.get(&s.session_id) {
            Some(view) => view_blocks(view).any(|b| state.status_failed.contains(&b)),
            None => state.viewed.contains(&s.session_id),
        })
    };
    let changed = {
        let m = shared.model.lock().expect("host model");
        m.activity_reading != reading || m.activity_incomplete != incomplete
    };
    if changed {
        shared.update(|m| {
            m.activity_reading = reading;
            m.activity_incomplete = incomplete;
        });
    }
}

pub(super) fn record(
    shared: &Shared,
    state: &mut ControlState,
    session: &str,
    block: &str,
    reply: Option<Value>,
    removed: &std::collections::HashSet<String>,
) {
    let Some(reply) = reply else {
        state.status_failed.insert(block.into());
        return;
    };
    let mut records = activity::parse_status(&reply);
    // Removed live while this read was out: the reply is older than the removal.
    records.retain(|(id, _)| !removed.contains(id));
    // Most panes report nothing: don't wake the app to tell it so.
    {
        let mut model = shared.model.lock().expect("host model");
        if records.is_empty() && !model.activity.contains_key(block) {
            // The pane had reported nothing, and that is now known: its first report
            // will be a state this client watched begin.
            model.status_read.insert(block.into());
            return;
        }
    }
    shared.update(|m| {
        if !m
            .views
            .get(session)
            .is_some_and(|v| view_blocks(v).any(|b| b == block))
        {
            return;
        }
        // Only now, with the reply about to be kept: a reply that is thrown away must
        // not count as having read the pane.
        m.status_read.insert(block.into());
        // Same snapshot semantics as the selected-session query. These are old reports,
        // not live news: no notification or unseen-Done receipt is invented here.
        let records = merge(m.activity.remove(block), records);
        if !records.is_empty() {
            m.activity.insert(block.into(), records);
        }
    });
}

/// A block's records after a status read. The read replaces what was known, except a
/// record that arrived live on this connection: that report is at least as new as the
/// read, and it is news. A background session's layout and status are read moments
/// apart, so without this a report made just after connecting could be overwritten as
/// old before the app had seen it, and its notification lost.
fn merge(
    known: Option<std::collections::BTreeMap<String, activity::StatusRecord>>,
    read: Vec<(String, activity::StatusRecord)>,
) -> std::collections::BTreeMap<String, activity::StatusRecord> {
    let mut records: std::collections::BTreeMap<_, _> = read.into_iter().collect();
    records.extend(
        known
            .into_iter()
            .flatten()
            .filter(|(_, record)| record.live),
    );
    records
}

/// Background reads in flight at once, layout reads included (at most one of those).
/// More than one shortens the pass on a slow link, where each read waits a round trip;
/// starts stay paced, and the host loop's other work still comes first.
pub(super) const IN_FLIGHT: usize = 3;

#[derive(Default)]
pub(super) struct Reader {
    reads: Vec<Read>,
}

struct Read {
    session: String,
    block: String,
    reply: oneshot::Receiver<String>,
    deadline: std::pin::Pin<Box<tokio::time::Sleep>>,
}
impl Reader {
    pub fn in_flight(&self) -> usize {
        self.reads.len()
    }

    pub async fn start(
        &mut self,
        control: &mut Client,
        session: String,
        block: String,
    ) -> rexrs::Result<()> {
        let reply = control
            .transport()
            .detach(
                &format!("{}.program_status", activity::TERMINAL),
                json!({"session_id":session,"block_id":block,"args":{}}),
            )
            .await?;
        self.reads.push(Read {
            session,
            block,
            reply,
            deadline: Box::pin(tokio::time::sleep_until(tokio::time::Instant::from_std(
                Instant::now() + Duration::from_secs(8),
            ))),
        });
        Ok(())
    }

    /// The first read to answer or to run out of time. Cancel-safe: a read is taken out
    /// only in the poll that returns it. The control socket keeps receiving in the host
    /// loop meanwhile.
    pub async fn finished(&mut self) -> (String, String, Option<Value>) {
        use std::{future::Future, pin::Pin, task::Poll};
        if self.reads.is_empty() {
            return std::future::pending().await;
        }
        let (index, text) = std::future::poll_fn(|cx| {
            for (index, read) in self.reads.iter_mut().enumerate() {
                // The reply first: one that arrived is not lost to its deadline.
                if let Poll::Ready(reply) = Pin::new(&mut read.reply).poll(cx) {
                    return Poll::Ready((index, reply.ok()));
                }
                if read.deadline.as_mut().poll(cx).is_ready() {
                    return Poll::Ready((index, None));
                }
            }
            Poll::Pending
        })
        .await;
        let read = self.reads.remove(index);
        let payload = text
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .filter(|v| v["error"].is_null() && v["payload"].is_object())
            .map(|mut v| v["payload"].take());
        (read.session, read.block, payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use activity::{ProgramState, StatusRecord};

    fn record(state: ProgramState, live: bool) -> StatusRecord {
        StatusRecord {
            state,
            live,
            ..Default::default()
        }
    }

    #[test]
    fn a_read_replaces_old_records_but_never_a_live_report() {
        let known = [
            ("agent".to_owned(), record(ProgramState::Blocked, true)),
            ("stale".to_owned(), record(ProgramState::Working, false)),
        ]
        .into_iter()
        .collect();
        let read = vec![
            // The server's answer for the live record is no newer than the report itself.
            ("agent".to_owned(), record(ProgramState::Working, false)),
            ("other".to_owned(), record(ProgramState::Error, false)),
        ];
        let merged = merge(Some(known), read);
        assert_eq!(
            merged.len(),
            2,
            "the old record the server no longer has is gone"
        );
        assert_eq!(merged["agent"].state, ProgramState::Blocked);
        assert!(
            merged["agent"].live,
            "still news for the app to notify about"
        );
        assert_eq!(merged["other"].state, ProgramState::Error);
        assert!(!merged["other"].live);
        // Nothing known: the read is taken as it is. Nothing read: only live reports stay.
        let read = vec![("a".to_owned(), record(ProgramState::Done, false))];
        assert_eq!(merge(None, read)["a"].state, ProgramState::Done);
        let live = [("a".to_owned(), record(ProgramState::Blocked, true))]
            .into_iter()
            .collect();
        assert_eq!(merge(Some(live), vec![]).len(), 1);
        let old = [("a".to_owned(), record(ProgramState::Blocked, false))]
            .into_iter()
            .collect();
        assert!(merge(Some(old), vec![]).is_empty());
    }

    #[tokio::test]
    async fn a_silent_read_expires_and_a_late_reply_has_no_receiver() {
        let (sender, reply) = oneshot::channel();
        let mut reader = Reader {
            reads: vec![Read {
                session: "s".into(),
                block: "b".into(),
                reply,
                deadline: Box::pin(tokio::time::sleep_until(tokio::time::Instant::now())),
            }],
        };
        assert_eq!(reader.finished().await, ("s".into(), "b".into(), None));
        assert_eq!(reader.in_flight(), 0);
        assert!(sender.send("late".into()).is_err());
    }

    #[tokio::test]
    async fn reads_finish_in_the_order_they_answer_and_each_once() {
        let far = || {
            Box::pin(tokio::time::sleep_until(
                tokio::time::Instant::now() + Duration::from_secs(60),
            ))
        };
        let mut senders = Vec::new();
        let mut reader = Reader::default();
        for block in ["a", "b", "c"] {
            let (sender, reply) = oneshot::channel();
            senders.push(sender);
            reader.reads.push(Read {
                session: "s".into(),
                block: block.into(),
                reply,
                deadline: far(),
            });
        }
        // Nothing answered: waiting is cancel-safe and takes nothing out.
        assert!(
            tokio::time::timeout(Duration::from_millis(20), reader.finished())
                .await
                .is_err()
        );
        assert_eq!(reader.in_flight(), 3);
        let answer = |block: &str| {
            json!({"type":"response","id":"x","payload":{"records":[{"id":block,"state":"blocked"}]}})
                .to_string()
        };
        // The last one sent answers first, then the first; the middle one never does.
        let c = senders.pop().unwrap();
        c.send(answer("c")).unwrap();
        let (_, block, reply) = reader.finished().await;
        assert_eq!(block, "c");
        assert_eq!(reply.unwrap()["records"][0]["id"], "c");
        senders.remove(0).send(answer("a")).unwrap();
        assert_eq!(reader.finished().await.1, "a");
        assert_eq!(reader.in_flight(), 1);
        drop(senders);
        // Its sender gone (the connection's transport dropped it): a failed read.
        assert_eq!(reader.finished().await, ("s".into(), "b".into(), None));
        assert_eq!(reader.in_flight(), 0);
    }
}
