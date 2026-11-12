//! Local age of a status, not a server uptime or a clock-skew estimate.
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// How far ahead of this client's clock a report may claim to be and still count as now.
const AHEAD: Duration = Duration::from_secs(60);
/// The oldest report that is shown with an age.
const OLDEST: Duration = Duration::from_secs(365 * 86400);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Age {
    pub observed: Instant,
    pub initial: Duration,
    /// The initial age is only since the server's last report, not a state transition.
    pub reported: bool,
}

impl Age {
    pub fn live(now: Instant) -> Self {
        Self {
            observed: now,
            initial: Duration::ZERO,
            reported: false,
        }
    }

    pub fn read_back(value: &str, now: Instant, wall: SystemTime) -> Option<Self> {
        let at = timestamp(value)?;
        let initial = match wall.duration_since(at) {
            Ok(age) => age,
            // Clocks differ a little: a report from the near future is from just now. One
            // from further ahead means the clocks disagree, and then no age can be trusted:
            // an old request must not be made to look fresh.
            Err(ahead) if ahead.duration() <= AHEAD => Duration::ZERO,
            Err(_) => return None,
        };
        // Nothing waits this long; it is a wrong clock or a made-up time.
        (initial <= OLDEST).then_some(Self {
            observed: now,
            initial,
            reported: true,
        })
    }

    pub fn elapsed(self, now: Instant) -> Duration {
        self.initial
            .saturating_add(now.saturating_duration_since(self.observed))
    }
}

/// RFC 3339 without a new dependency. Server settings intentionally keep timestamps as
/// strings, so there is no parser there to share. Validate before civil-date conversion;
/// reject leap seconds (unknown age), and truncate sub-nanosecond precision.
fn timestamp(raw: &str) -> Option<SystemTime> {
    let b = raw.as_bytes();
    if b.len() < 20
        || !raw.is_ascii()
        || b[4] != b'-'
        || b[7] != b'-'
        || !matches!(b[10], b'T' | b't')
        || b[13] != b':'
        || b[16] != b':'
    {
        return None;
    }
    let number = |s: &str| -> Option<i64> {
        s.bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| s.parse().ok())
            .flatten()
    };
    let year = number(&raw[..4])?;
    let month = number(&raw[5..7])?;
    let day = number(&raw[8..10])?;
    let hour = number(&raw[11..13])?;
    let minute = number(&raw[14..16])?;
    let second = number(&raw[17..19])?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        1..=12 => 31,
        _ => return None,
    };
    if year == 0 || !(1..=days).contains(&day) || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let mut end = 19;
    let mut nanos = 0;
    if b.get(end) == Some(&b'.') {
        end += 1;
        let start = end;
        while b.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        if start == end {
            return None;
        }
        let digits = &raw[start..end.min(start + 9)];
        nanos = number(digits)? as u32 * 10u32.pow(9 - digits.len() as u32);
    }
    let zone = &raw[end..];
    let offset = if matches!(zone, "Z" | "z") {
        0
    } else {
        let z = zone.as_bytes();
        if z.len() != 6 || !matches!(z[0], b'+' | b'-') || z[3] != b':' {
            return None;
        }
        let h = number(&zone[1..3])?;
        let m = number(&zone[4..6])?;
        // `-00:00` is UTC with the local offset unknown: the instant is still known.
        if h > 23 || m > 59 {
            return None;
        }
        (h * 3600 + m * 60) * if z[0] == b'-' { -1 } else { 1 }
    };
    // Gregorian civil date to days since 1970-01-01 (400-year eras).
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yo = y - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let days = era * 146097 + yo * 365 + yo / 4 - yo / 100 + doy - 719468;
    let seconds = days * 86400 + hour * 3600 + minute * 60 + second - offset;
    if seconds >= 0 {
        UNIX_EPOCH.checked_add(Duration::new(seconds as u64, nanos))
    } else {
        UNIX_EPOCH
            .checked_sub(Duration::from_secs(seconds.unsigned_abs()))?
            .checked_add(Duration::from_nanos(nanos.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_repeats_keep_start_and_transitions_restart() {
        use crate::connection::activity::StatusRecord;
        use serde_json::json;
        let start = Instant::now();
        // The client held an idle record and watched it turn to Working.
        let idle = StatusRecord::parse(&json!({"state":"idle"}));
        let mut first = StatusRecord {
            live: true,
            ..StatusRecord::parse(&json!({"state":"working"}))
        };
        first.observe_at(Some(&idle), false, start);
        assert!(!first.age.unwrap().reported);
        let later = start + Duration::from_secs(240);
        let mut repeat = StatusRecord {
            live: true,
            ..StatusRecord::parse(&json!({"state":"working", "msg":"more", "progress":90}))
        };
        repeat.observe_at(Some(&first), false, later);
        assert_eq!(repeat.age.unwrap().elapsed(later).as_secs(), 240);
        let mut changed = StatusRecord {
            live: true,
            ..StatusRecord::parse(&json!({"state":"blocked"}))
        };
        changed.observe_at(Some(&repeat), false, later);
        assert_eq!(changed.age.unwrap().elapsed(later), Duration::ZERO);
    }

    #[test]
    fn read_back_to_live_keeps_lower_bound_only_for_same_state() {
        use crate::connection::activity::StatusRecord;
        use serde_json::json;
        let now = Instant::now();
        let wall = UNIX_EPOCH + Duration::from_secs(3600);
        let old = StatusRecord::parse_at(
            &json!({"state":"working", "updated_at":"1970-01-01T00:00:00Z"}),
            now,
            wall,
        );
        assert!(!old.live);
        assert_eq!(old.age.unwrap().elapsed(now).as_secs(), 3600);
        for state in ["working", "blocked"] {
            let mut live = StatusRecord {
                live: true,
                ..StatusRecord::parse_at(
                    &json!({"state":state, "updated_at":"1970-01-01T01:00:00Z"}),
                    now,
                    wall,
                )
            };
            live.observe_at(Some(&old), false, now);
            let age = live.age.unwrap();
            assert_eq!(
                age.elapsed(now).as_secs(),
                if state == "working" { 3600 } else { 0 }
            );
            assert_eq!(age.reported, state == "working");
        }
        for value in [
            json!({"state":"working"}),
            json!({"updated_at":"bad"}),
            json!({"updated_at":"2099-01-01T00:00:00Z"}),
        ] {
            assert!(StatusRecord::parse_at(&value, now, wall).age.is_none());
        }
    }

    #[test]
    fn a_state_the_client_did_not_watch_begin_never_gets_an_observed_clock() {
        use crate::connection::activity::StatusRecord;
        use serde_json::json;
        let now = Instant::now();
        let wall = UNIX_EPOCH + Duration::from_secs(7200);
        let live = |value: serde_json::Value| StatusRecord {
            live: true,
            ..StatusRecord::parse_at(&value, now, wall)
        };
        // A first report for a pane: the program may have been working for hours before
        // this client connected, or before the status pass reached the pane.
        let mut first = live(json!({"state":"working", "updated_at":"1970-01-01T01:59:00Z"}));
        first.observe_at(None, false, now);
        let age = first.age.expect("the report's own time");
        assert!(age.reported, "qualified as a report, not an observed start");
        assert_eq!(age.elapsed(now).as_secs(), 60);
        // With no time on the report there is nothing honest to show.
        let mut bare = live(json!({"state":"blocked"}));
        bare.observe_at(None, false, now);
        assert_eq!(bare.age, None);
        // Once the pane's status has been read on this connection and this record was
        // not there, its first report is a state the client watched begin.
        let mut watched = live(json!({"state":"working", "updated_at":"1970-01-01T01:59:00Z"}));
        watched.observe_at(None, true, now);
        let age = watched.age.unwrap();
        assert!(!age.reported);
        assert_eq!(age.elapsed(now), Duration::ZERO);
        // A same-state report over a read-back record that had no time is still only a
        // report: it must not start an observed clock either.
        let ageless = StatusRecord::parse_at(&json!({"state":"blocked"}), now, wall);
        assert_eq!(ageless.age, None);
        let mut repeat = live(json!({"state":"blocked", "updated_at":"1970-01-01T02:00:00Z"}));
        repeat.observe_at(Some(&ageless), false, now);
        assert!(repeat.age.unwrap().reported);
        let mut bare_repeat = live(json!({"state":"blocked"}));
        bare_repeat.observe_at(Some(&ageless), false, now);
        assert_eq!(bare_repeat.age, None);
    }

    #[test]
    fn host_event_path_preserves_query_age() {
        use crate::connection::{
            HostModel,
            activity::{StatusRecord, parse_block_event},
        };
        use serde_json::json;
        let now = Instant::now();
        let wall = UNIX_EPOCH + Duration::from_secs(3600);
        let old = StatusRecord::parse_at(
            &json!({"state":"working", "updated_at":"1970-01-01T00:00:00Z"}),
            now,
            wall,
        );
        let mut model = HostModel::default();
        model
            .activity
            .insert("pane".into(), [("".into(), old.clone())].into());
        let (block, event) = parse_block_event(&json!({"creator_name":"com.superlogical.terminal", "block_id":"pane", "name":"program_status_changed", "payload":{"id":"", "record":{"state":"working", "progress":42, "updated_at":"1970-01-01T01:00:00Z"}}})).unwrap();
        model.apply_block_event("session", block, event);
        assert_eq!(model.activity["pane"][""].age, old.age);
        assert!(model.activity["pane"][""].live);
        assert_eq!(model.activity["pane"][""].progress, Some(42));
    }

    #[test]
    fn host_event_path_watches_a_first_report_only_for_a_pane_already_read() {
        use crate::connection::{HostModel, activity::parse_block_event};
        use serde_json::json;
        let first_report = || {
            parse_block_event(&json!({"creator_name":"com.superlogical.terminal",
                "block_id":"pane", "name":"program_status_changed",
                "payload":{"id":"", "record":{"state":"blocked"}}}))
            .unwrap()
        };
        // Not yet read on this connection: the program may have been waiting for hours.
        let mut model = HostModel::default();
        let (block, event) = first_report();
        model.apply_block_event("session", block, event);
        assert!(model.activity["pane"][""].live);
        assert_eq!(model.activity["pane"][""].age, None);
        // Read and found without a record: the report is the state beginning.
        let mut model = HostModel::default();
        model.status_read.insert("pane".into());
        let (block, event) = first_report();
        model.apply_block_event("session", block, event);
        let age = model.activity["pane"][""].age.expect("an observed start");
        assert!(!age.reported);
    }

    #[test]
    fn read_back_clock_and_rfc3339() {
        let now = Instant::now();
        let wall = UNIX_EPOCH + Duration::from_secs(86400);
        for raw in [
            "1970-01-01T00:00:00Z",
            "1969-12-31T16:00:00-08:00",
            "1970-01-01t01:00:00+01:00",
        ] {
            assert_eq!(
                Age::read_back(raw, now, wall)
                    .unwrap()
                    .elapsed(now)
                    .as_secs(),
                86400
            );
        }
        // UTC with the local offset unknown is still a known instant.
        assert_eq!(
            Age::read_back("1970-01-01T00:00:00-00:00", now, wall)
                .unwrap()
                .elapsed(now)
                .as_secs(),
            86400
        );
        // A little ahead of this clock is now; further ahead the clocks disagree and an
        // old request must not look fresh. Absurdly old is not an age either.
        assert_eq!(
            Age::read_back("1970-01-02T00:01:00Z", now, wall)
                .unwrap()
                .elapsed(now),
            Duration::ZERO
        );
        for unusable in [
            "1970-01-02T00:01:01Z",
            "2099-01-01T00:00:00Z",
            "9999-12-31T23:59:59Z",
        ] {
            assert_eq!(Age::read_back(unusable, now, wall), None, "{unusable}");
        }
        let much_later = UNIX_EPOCH + Duration::from_secs(366 * 86400);
        assert_eq!(
            Age::read_back("1970-01-01T00:00:00Z", now, much_later),
            None
        );
        assert_eq!(
            Age::read_back("0001-01-01T00:00:00Z", now, much_later),
            None
        );
        assert!(Age::read_back("1970-01-02T00:00:00Z", now, much_later).is_some());
        assert_eq!(
            timestamp("1970-01-01T00:00:00.123456789123Z"),
            Some(UNIX_EPOCH + Duration::from_nanos(123456789))
        );
        for bad in [
            "",
            "oops",
            "2026-02-29T00:00:00Z",
            "2024-02-30T00:00:00Z",
            "2024-01-01T24:00:00Z",
            "2024-01-01T00:00:00",
            "2024-01-01T00:00:00.Z",
            "2024-01-01T00:00:00+24:00",
            "2024-01-01T00:00:00Zextra",
            "💥",
        ] {
            assert!(Age::read_back(bad, now, wall).is_none(), "{bad}");
        }
        assert!(timestamp("2024-02-29T23:59:59Z").is_some());
    }
}
