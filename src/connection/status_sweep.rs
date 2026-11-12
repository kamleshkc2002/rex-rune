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
    if records.is_empty()
        && !shared
            .model
            .lock()
            .expect("host model")
            .activity
            .contains_key(block)
    {
        return;
    }
    shared.update(|m| {
        if !m
            .views
            .get(session)
            .is_some_and(|v| view_blocks(v).any(|b| b == block))
        {
            return;
        }
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

#[derive(Default)]
pub(super) struct Reader {
    read: Option<Read>,
}

struct Read {
    session: String,
    block: String,
    reply: oneshot::Receiver<String>,
    deadline: Instant,
}
impl Reader {
    pub fn in_flight(&self) -> bool {
        self.read.is_some()
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
        self.read = Some(Read {
            session,
            block,
            reply,
            deadline: Instant::now() + Duration::from_secs(8),
        });
        Ok(())
    }

    /// Cancel-safe bounded wait; the control socket continues receiving in the host loop.
    pub async fn finished(&mut self) -> (String, String, Option<Value>) {
        let Some(read) = &mut self.read else {
            return std::future::pending().await;
        };
        let text = tokio::time::timeout_at(
            tokio::time::Instant::from_std(read.deadline),
            &mut read.reply,
        )
        .await
        .ok()
        .and_then(Result::ok);
        let payload = text
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .filter(|v| v["error"].is_null() && v["payload"].is_object())
            .map(|mut v| v["payload"].take());
        let read = self.read.take().expect("status in flight");
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
            read: Some(Read {
                session: "s".into(),
                block: "b".into(),
                reply,
                deadline: Instant::now(),
            }),
        };
        assert_eq!(reader.finished().await, ("s".into(), "b".into(), None));
        assert!(!reader.in_flight());
        assert!(sender.send("late".into()).is_err());
    }
}
