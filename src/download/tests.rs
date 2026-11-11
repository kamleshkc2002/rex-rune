//! Unit tests for the library side of downloads: limits, the state machine, and the files.
use super::store::{self, Record, is_partial_name};
use super::*;
use std::{
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
};

/// A temporary folder, removed with its contents when dropped.
pub(crate) struct Temp(pub PathBuf);

impl Temp {
    pub fn new(label: &str) -> Self {
        static COUNT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rex-download-test-{}-{}-{label}",
            std::process::id(),
            COUNT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }
    pub fn dir(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::create_dir_all(&path).unwrap();
        path
    }
    pub fn records(&self) -> Records {
        Records::at(self.0.join("state/downloads.json"))
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The names in a folder, sorted.
pub(crate) fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

// -- Limits --------------------------------------------------------------------------------

#[test]
fn defaults_are_the_roadmaps_limits() {
    let limits = Limits::default();
    assert_eq!(limits.confirm_at, GIB);
    assert_eq!(limits.reserve, 256 * MIB);
    assert_eq!(limits.low_floor, 16 * MIB);
    assert_eq!(limits.growth, 64 * MIB);
    assert_eq!(limits.stall, Duration::from_secs(30));
    assert_eq!(limits.per_host, 3);
}

#[test]
fn confirmation_is_for_large_files_and_for_files_that_would_eat_the_reserve() {
    let limits = Limits::default();
    let free = 100 * GIB;
    assert_eq!(limits.confirmation(0, free), None);
    assert_eq!(limits.confirmation(GIB - 1, free), None);
    let large = limits.confirmation(GIB, free).unwrap();
    assert_eq!(
        (large.reason, large.hint, large.free),
        (Reason::Large, GIB, Some(free))
    );
    assert_eq!(
        limits.confirmation(50 * GIB, free).unwrap().reason,
        Reason::Large
    );

    // Free space less the reserve is the largest file that needs no question.
    assert_eq!(limits.confirmation(144 * MIB, 400 * MIB), None);
    assert_eq!(
        limits.confirmation(145 * MIB, 400 * MIB).unwrap().reason,
        Reason::LowSpace
    );
    // Large and too big for the space is a space problem first.
    assert_eq!(
        limits.confirmation(2 * GIB, GIB).unwrap().reason,
        Reason::LowSpace
    );
    // Less free space than the reserve: every file, even an empty one, is a question.
    assert_eq!(
        limits.confirmation(1, 100 * MIB).unwrap().reason,
        Reason::LowSpace
    );
    assert_eq!(
        limits.confirmation(0, 100 * MIB).unwrap().reason,
        Reason::LowSpace
    );

    // Space that can't be read still allows the question about a large file, and no other.
    assert_eq!(limits.confirmation_without_space(GIB - 1), None);
    let large = limits.confirmation_without_space(5 * GIB).unwrap();
    assert_eq!(
        (large.reason, large.hint, large.free),
        (Reason::Large, 5 * GIB, None)
    );
}

#[test]
fn admission_refuses_what_cannot_fit_asks_what_would_eat_the_reserve_and_starts_the_rest() {
    let limits = Limits::default();
    let ask = |reason, hint, free| Admission::Ask(Confirmation { reason, hint, free });
    let refuse = |free| Admission::Refuse(Failure::LowSpace { free });
    // Plenty of room: a small file starts, a large one is asked about.
    assert_eq!(limits.admission(0, Some(100 * GIB)), Admission::Start);
    assert_eq!(limits.admission(GIB - 1, Some(100 * GIB)), Admission::Start);
    assert_eq!(
        limits.admission(GIB, Some(100 * GIB)),
        ask(Reason::Large, GIB, Some(100 * GIB))
    );
    // 300 MiB free, a 100 MiB file: it fits but would leave 200 MiB, under the 256 MiB reserve.
    // That is a question (a yes lets it use the reserve), not a certain failure.
    assert_eq!(
        limits.admission(100 * MIB, Some(300 * MIB)),
        ask(Reason::LowSpace, 100 * MIB, Some(300 * MIB))
    );
    // The edges: free less the reserve needs no question; a byte more does; free less the floor
    // is still a question; a byte more cannot fit while keeping the floor.
    assert_eq!(
        limits.admission(144 * MIB, Some(400 * MIB)),
        Admission::Start
    );
    assert_eq!(
        limits.admission(145 * MIB, Some(400 * MIB)),
        ask(Reason::LowSpace, 145 * MIB, Some(400 * MIB))
    );
    assert_eq!(
        limits.admission(384 * MIB, Some(400 * MIB)),
        ask(Reason::LowSpace, 384 * MIB, Some(400 * MIB))
    );
    assert_eq!(
        limits.admission(384 * MIB + 1, Some(400 * MIB)),
        refuse(400 * MIB)
    );
    // Exactly the free space used to be a question; it leaves nothing, so it is refused now.
    assert_eq!(
        limits.admission(400 * MIB, Some(400 * MIB)),
        refuse(400 * MIB)
    );
    // A file larger than the free space is refused without a question, large or not.
    assert_eq!(limits.admission(2 * GIB, Some(GIB)), refuse(GIB));
    // Less free space than the reserve is a question for a file that leaves the floor, whatever
    // its size, and a refusal for one that does not (or when the floor itself is gone).
    assert_eq!(
        limits.admission(1, Some(100 * MIB)),
        ask(Reason::LowSpace, 1, Some(100 * MIB))
    );
    assert_eq!(
        limits.admission(0, Some(100 * MIB)),
        ask(Reason::LowSpace, 0, Some(100 * MIB))
    );
    assert_eq!(
        limits.admission(85 * MIB, Some(100 * MIB)),
        refuse(100 * MIB)
    );
    assert_eq!(
        limits.admission(0, Some(16 * MIB - 1)),
        refuse(16 * MIB - 1)
    );
    assert_eq!(limits.admission(0, Some(0)), refuse(0));
    // Space that can't be read: only a large file is asked about, and nothing is refused.
    assert_eq!(limits.admission(GIB - 1, None), Admission::Start);
    assert_eq!(
        limits.admission(5 * GIB, None),
        ask(Reason::Large, 5 * GIB, None)
    );
}

#[test]
fn a_yes_is_looked_at_again_against_the_space_there_is_now() {
    let limits = Limits::default();
    let low = |free| Err(Failure::LowSpace { free });
    // Large, answered yes: still room, and a disk that cannot say, goes on.
    assert_eq!(
        limits.recheck(Reason::Large, 2 * GIB, Some(100 * GIB)),
        Ok(())
    );
    assert_eq!(limits.recheck(Reason::Large, 2 * GIB, None), Ok(()));
    // Large: the file no longer fits with the floor left over: refused.
    assert_eq!(
        limits.recheck(Reason::Large, 2 * GIB, Some(2 * GIB)),
        low(2 * GIB)
    );
    // Large: space is now low enough for the other question: failed, not silently let through.
    assert_eq!(
        limits.recheck(Reason::Large, 2 * GIB, Some(2 * GIB + 100 * MIB)),
        low(2 * GIB + 100 * MIB)
    );
    // Low space, answered yes: the same band is what was agreed to; the floor is not.
    assert_eq!(
        limits.recheck(Reason::LowSpace, 300 * MIB, Some(400 * MIB)),
        Ok(())
    );
    assert_eq!(
        limits.recheck(Reason::LowSpace, 300 * MIB, Some(310 * MIB)),
        low(310 * MIB)
    );
    // Low space, answered yes, and the disk has since been cleared: nothing to refuse.
    assert_eq!(
        limits.recheck(Reason::LowSpace, 300 * MIB, Some(10 * GIB)),
        Ok(())
    );
}

#[test]
fn growth_past_the_hint_is_capped_at_the_larger_of_double_and_plus_64_mib() {
    let limits = Limits::default();
    assert_eq!(limits.growth_cap(0), 64 * MIB);
    assert_eq!(limits.growth_cap(10 * MIB), 74 * MIB);
    assert_eq!(limits.growth_cap(64 * MIB), 128 * MIB);
    assert_eq!(limits.growth_cap(100 * MIB), 200 * MIB);
    assert_eq!(limits.growth_cap(5 * GIB), 10 * GIB);
    assert_eq!(limits.growth_cap(u64::MAX), u64::MAX);
}

#[test]
fn the_meter_aborts_past_the_growth_cap() {
    let limits = Limits {
        check_every: u64::MAX,
        ..Limits::default()
    };
    let now = Instant::now();
    let space = || -> std::io::Result<u64> { Ok(u64::MAX) };
    let mut meter = Meter::new(limits, 10 * MIB, now);
    let cap = limits.growth_cap(10 * MIB);
    meter.wrote(cap - 1, now, &space).unwrap();
    meter.wrote(1, now, &space).unwrap();
    assert_eq!(meter.received(), cap);
    assert_eq!(
        meter.wrote(1, now, &space),
        Err(Failure::TooMuchData { limit: cap })
    );
}

#[test]
fn the_meter_refuses_bytes_that_would_cross_the_cap_before_they_are_counted() {
    let limits = Limits {
        growth: 1_000,
        ..Limits::default()
    };
    let now = Instant::now();
    let space = || -> std::io::Result<u64> { Ok(u64::MAX) };
    // Hint 0: the cap is the growth allowance alone, 1000.
    let mut meter = Meter::new(limits, 0, now);
    assert_eq!(meter.admit(1_000), Ok(()));
    assert_eq!(
        meter.admit(1_001),
        Err(Failure::TooMuchData { limit: 1_000 })
    );
    // Asking changes nothing: the count is only moved by what was written.
    assert_eq!(meter.received(), 0);
    meter.wrote(600, now, &space).unwrap();
    assert_eq!(meter.admit(400), Ok(()));
    assert_eq!(meter.admit(401), Err(Failure::TooMuchData { limit: 1_000 }));
    // Huge messages cannot wrap the sum.
    assert!(meter.admit(u64::MAX).is_err());
}

#[tokio::test]
async fn a_chunk_that_would_cross_the_cap_is_not_written() {
    use super::stream::write_chunk;
    let temp = Temp::new("chunk");
    let dir = temp.dir("out");
    let records = temp.records();
    let destination = Destination::open(&dir).unwrap();
    let mut partial = destination.create_partial("x.bin", &records).unwrap();
    let limits = Limits {
        growth: 1_000,
        check_every: u64::MAX,
        ..Limits::default()
    };
    let space = || -> std::io::Result<u64> { Ok(u64::MAX) };
    // Hint 0: the cap is 1000.
    let mut meter = Meter::new(limits, 0, Instant::now());
    write_chunk(&mut partial, &mut meter, &[1; 800], &space)
        .await
        .unwrap();
    assert_eq!(partial.written(), 800);
    // 900 KiB more would be far over: refused, and not a byte of it is written.
    let failure = write_chunk(&mut partial, &mut meter, &vec![2; 900 * 1024], &space)
        .await
        .unwrap_err();
    assert_eq!(failure, Failure::TooMuchData { limit: 1_000 });
    assert_eq!((partial.written(), meter.received()), (800, 800));
    // The same for a chunk that is only 201 bytes too many; 200 more is exactly the cap.
    assert!(
        write_chunk(&mut partial, &mut meter, &[3; 201], &space)
            .await
            .is_err()
    );
    assert_eq!(partial.written(), 800);
    write_chunk(&mut partial, &mut meter, &[3; 200], &space)
        .await
        .unwrap();
    assert_eq!((partial.written(), meter.received()), (1_000, 1_000));
    // What was written is what is on disk once finished.
    let saved = partial.finish("x.bin").await.unwrap();
    assert_eq!(std::fs::metadata(saved).unwrap().len(), 1_000);
}

#[test]
fn the_meter_aborts_when_free_space_falls_below_the_reserve() {
    let limits = Limits {
        check_every: 4 * MIB,
        ..Limits::default()
    };
    let now = Instant::now();
    let calls = AtomicU64::new(0);
    let free = AtomicU64::new(300 * MIB);
    let space = || -> std::io::Result<u64> {
        calls.fetch_add(1, Ordering::Relaxed);
        Ok(free.load(Ordering::Relaxed))
    };
    let mut meter = Meter::new(limits, 100 * MIB, now);
    // Not looked at before the first 4 MiB.
    meter.wrote(3 * MIB, now, &space).unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    meter.wrote(MIB, now, &space).unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    // Exactly the reserve is enough; one byte less is not.
    free.store(256 * MIB, Ordering::Relaxed);
    meter.wrote(4 * MIB, now, &space).unwrap();
    free.store(256 * MIB - 1, Ordering::Relaxed);
    assert_eq!(
        meter.wrote(4 * MIB, now, &space),
        Err(Failure::LowSpace {
            free: 256 * MIB - 1
        })
    );
    // A filesystem that can't say is not a reason to stop.
    let unknown = || -> std::io::Result<u64> { Err(std::io::ErrorKind::Other.into()) };
    let mut meter = Meter::new(limits, 100 * MIB, now);
    meter.wrote(8 * MIB, now, &unknown).unwrap();
}

#[test]
fn the_meter_stalls_after_the_timeout_without_bytes() {
    let limits = Limits {
        check_every: u64::MAX,
        ..Limits::default()
    };
    let start = Instant::now();
    let space = || -> std::io::Result<u64> { Ok(u64::MAX) };
    let mut meter = Meter::new(limits, 10, start);
    assert_eq!(meter.deadline(), start + Duration::from_secs(30));
    assert!(!meter.stalled(start + Duration::from_millis(29_999)));
    assert!(meter.stalled(start + Duration::from_secs(30)));
    // Bytes restart the clock.
    let later = start + Duration::from_secs(25);
    meter.wrote(1, later, &space).unwrap();
    assert!(!meter.stalled(start + Duration::from_secs(54)));
    assert!(meter.stalled(later + Duration::from_secs(30)));
}

#[test]
fn at_most_three_transfers_run_per_host() {
    let transfer = |id, state: State| {
        let mut t = Transfer::new(id, "s".into(), "b".into(), "f".into(), "/d".into(), 1);
        t.state = state;
        t
    };
    let mut list = vec![
        transfer(1, State::Requested),
        transfer(2, State::Cancelled),
        transfer(
            3,
            State::Receiving {
                received: 1,
                hint: 2,
            },
        ),
        transfer(4, State::Failed(Failure::Stalled)),
    ];
    assert_eq!(active(&list), 2);
    assert!(active(&list) < Limits::default().per_host);
    list.push(transfer(
        5,
        State::AwaitingConfirmation(Confirmation {
            reason: Reason::Large,
            hint: 1,
            free: Some(1),
        }),
    ));
    assert_eq!(active(&list), 3);
    assert!(active(&list) >= Limits::default().per_host);
}

// -- The state machine -----------------------------------------------------------------------

fn fresh() -> Transfer {
    Transfer::new(
        1,
        "session".into(),
        "block".into(),
        "~/a/b.txt".into(),
        "/d".into(),
        1,
    )
}

fn begun(hint: u64, confirmation: Option<Confirmation>) -> Event {
    Event::Begun {
        resolved: "/home/u/a/b.txt".into(),
        name: "b.txt".into(),
        hint,
        confirmation,
    }
}

#[test]
fn a_transfer_goes_requested_receiving_saved() {
    let mut t = fresh();
    assert_eq!(t.state, State::Requested);
    assert_eq!(t.display_name(), "b.txt");
    assert!(t.apply(begun(100, None)));
    assert_eq!(
        t.state,
        State::Receiving {
            received: 0,
            hint: 100
        }
    );
    assert_eq!(t.resolved.as_deref(), Some("/home/u/a/b.txt"));
    assert!(t.apply(Event::Progress { received: 40 }));
    assert_eq!(t.fraction(), Some(0.4));
    // Progress never goes backwards.
    assert!(!t.apply(Event::Progress { received: 30 }));
    assert_eq!(
        t.state,
        State::Receiving {
            received: 40,
            hint: 100
        }
    );
    assert!(t.apply(Event::Saved {
        path: "/d/b.txt".into(),
        bytes: 100,
        short_of: None
    }));
    assert_eq!(
        t.state,
        State::Saved {
            path: "/d/b.txt".into(),
            bytes: 100,
            short_of: None
        }
    );
    assert!(t.state.is_final());
    assert_eq!(t.fraction(), None);
}

#[test]
fn a_large_file_waits_for_confirmation_before_receiving() {
    let confirmation = Confirmation {
        reason: Reason::Large,
        hint: 2 * GIB,
        free: Some(10 * GIB),
    };
    let mut t = fresh();
    assert!(t.apply(begun(2 * GIB, Some(confirmation))));
    assert_eq!(t.state, State::AwaitingConfirmation(confirmation));
    assert!(!t.state.is_final());
    // Progress before the yes is out of order.
    assert!(!t.apply(Event::Progress { received: 1 }));
    assert!(!t.apply(Event::Saved {
        path: "/x".into(),
        bytes: 0,
        short_of: None
    }));
    assert!(t.apply(Event::Confirmed));
    assert_eq!(
        t.state,
        State::Receiving {
            received: 0,
            hint: 2 * GIB
        }
    );
    assert!(!t.apply(Event::Confirmed));
}

#[test]
fn cancel_and_failure_end_it_from_any_live_state_and_final_states_stay() {
    let confirmation = Confirmation {
        reason: Reason::LowSpace,
        hint: 5,
        free: Some(1),
    };
    let live = |setup: &dyn Fn(&mut Transfer)| {
        let mut t = fresh();
        setup(&mut t);
        t
    };
    let states: [Transfer; 3] = [
        live(&|_| {}),
        live(&|t| {
            t.apply(begun(5, Some(confirmation)));
        }),
        live(&|t| {
            t.apply(begun(5, None));
        }),
    ];
    for t in states {
        let mut cancelled = t.clone();
        assert!(cancelled.apply(Event::Cancelled));
        assert_eq!(cancelled.state, State::Cancelled);
        let mut failed = t.clone();
        assert!(failed.apply(Event::Failed(Failure::Stalled)));
        assert_eq!(failed.state, State::Failed(Failure::Stalled));
        for done in [cancelled, failed] {
            let before = done.clone();
            let mut done = done;
            for event in [
                begun(1, None),
                Event::Confirmed,
                Event::Progress { received: 9 },
                Event::Saved {
                    path: "/x".into(),
                    bytes: 1,
                    short_of: None,
                },
                Event::Cancelled,
                Event::Failed(Failure::ConnectionLost),
            ] {
                assert!(!done.apply(event));
                assert_eq!(done, before);
            }
        }
    }
    // A saved file can't later fail.
    let mut saved = fresh();
    saved.apply(begun(1, None));
    saved.apply(Event::Saved {
        path: "/d/b.txt".into(),
        bytes: 1,
        short_of: None,
    });
    assert!(!saved.apply(Event::Failed(Failure::ConnectionLost)));
    assert!(!saved.apply(Event::Cancelled));
}

#[test]
fn the_transfer_list_keeps_the_latest_finished_ones() {
    let mut list = Vec::new();
    for id in 0..(KEEP_FINISHED as u64 + 10) {
        let mut t = Transfer::new(id, "s".into(), "b".into(), "f".into(), "/d".into(), 1);
        t.apply(Event::Cancelled);
        push(&mut list, t);
    }
    let running = Transfer::new(999, "s".into(), "b".into(), "f".into(), "/d".into(), 1);
    push(&mut list, running);
    assert_eq!(
        list.iter().filter(|t| t.state.is_final()).count(),
        KEEP_FINISHED
    );
    assert_eq!(list.first().unwrap().id, 10);
    assert_eq!(active(&list), 1);
    assert_eq!(list.last().unwrap().id, 999);
}

#[test]
fn server_text_is_cleaned_and_cut_for_display() {
    assert_eq!(
        display_text("not found: /a\nb\u{1b}[0m"),
        "not found: /a b [0m"
    );
    let long = display_text(&"x".repeat(10_000));
    assert_eq!(long.chars().count(), 300);
    assert!(long.contains('…'));
    assert_eq!(display_text(&"界".repeat(300)).chars().count(), 300);
    assert_eq!(display_text(&"界".repeat(301)).chars().count(), 300);
}

#[test]
fn a_long_path_is_shortened_in_the_middle_so_its_end_still_shows() {
    // The end of a path is the part that tells files apart; cutting from the front hid it.
    let path = format!("/home/user/{}final-name.tar.gz", "deep/".repeat(200));
    let shown = display_text(&path);
    assert_eq!(shown.chars().count(), 300);
    assert!(shown.starts_with("/home/user/deep/"), "{shown}");
    assert!(shown.ends_with("/deep/final-name.tar.gz"), "{shown}");
    assert_eq!(shown.matches('…').count(), 1);
    assert_eq!(display_text(&shown), shown, "idempotent");
    // A sentence that repeats the path keeps its words at both ends.
    let sentence = format!("not found: {path} does not exist");
    let shown = display_text(&sentence);
    assert!(shown.starts_with("not found: /home/user/"), "{shown}");
    assert!(
        shown.ends_with("final-name.tar.gz does not exist"),
        "{shown}"
    );
    // Exactly the limit is untouched; a text one over is cut by one character, in the middle.
    let exact = "a".repeat(150) + &"b".repeat(150);
    assert_eq!(display_text(&exact), exact);
    let over = format!("{exact}c");
    let shown = display_text(&over);
    assert_eq!(shown.chars().count(), 300);
    assert!(shown.starts_with(&"a".repeat(100)) && shown.ends_with('c'));
}

#[test]
fn display_text_drops_direction_controls_and_invisible_fillers() {
    let shown = display_text("/home/u/\u{202e}gpj.exe\u{2066}\u{3164}");
    assert_eq!(shown, "/home/u/gpj.exe");
    assert_eq!(display_text(&shown), shown);
    // Cleaning is idempotent, cut text included.
    let long = display_text(&"y".repeat(10_000));
    assert_eq!(display_text(&long), long);
    // Every string a failure shows goes through it.
    let dirty = "a\nb\u{202e}c";
    for failure in [
        Failure::Server(dirty.into()),
        Failure::Connection(dirty.into()),
        Failure::Disk(dirty.into()),
        Failure::Destination(dirty.into()),
        Failure::Protocol(dirty.into()),
        Failure::UnexpectedReply(dirty.into()),
        Failure::Closed {
            code: Some(1011),
            reason: dirty.into(),
        },
    ] {
        let text = failure.to_string();
        assert!(text.contains("a bc"), "{failure:?}: {text}");
        assert!(!text.contains(['\n', '\u{202e}']), "{failure:?}: {text}");
    }
    assert_eq!(
        Failure::Closed {
            code: Some(1011),
            reason: "\u{202e}\n".into()
        }
        .to_string(),
        "Rex ended the transfer (1011)"
    );
}

#[test]
fn the_method_and_revision_match_the_sdk() {
    use rexrs::api::BlockMethod;
    assert_eq!(METHOD, rexrs::api::terminal::DownloadBegin::NAME);
    assert_eq!(MIN_API_REVISION, 13);
    assert_eq!(PROTOCOL, "rex.data.bytes.v1");
}

#[test]
fn refs_are_download_and_a_lower_case_uuid() {
    let a = new_ref().unwrap();
    assert!(a.starts_with("download:"));
    let uuid = &a["download:".len()..];
    assert_eq!(uuid.len(), 36);
    assert!(
        uuid.chars()
            .all(|c| c == '-' || c.is_ascii_digit() || ('a'..='f').contains(&c))
    );
    assert_ne!(a, new_ref().unwrap());
    assert_ne!(next_id(), next_id());
}

// -- The destination -------------------------------------------------------------------------

#[tokio::test]
async fn a_partial_file_is_private_until_finished_then_world_readable_never_executable() {
    let temp = Temp::new("partial");
    let dir = temp.dir("out");
    let records = temp.records();
    let destination = Destination::open(&dir).unwrap();
    let mut partial = destination.create_partial("report.txt", &records).unwrap();
    let partial_name = partial.file_name().to_owned();
    assert!(is_partial_name(&partial_name), "{partial_name}");
    assert!(partial_name.starts_with(".report.txt."));
    assert_eq!(records.list().len(), 1);
    partial.write(b"hello ").await.unwrap();
    partial.write(b"world").await.unwrap();
    assert_eq!(partial.written(), 11);
    assert_eq!(names(&dir), std::slice::from_ref(&partial_name));
    let meta = std::fs::metadata(dir.join(&partial_name)).unwrap();
    assert_eq!(meta.permissions().mode() & 0o777, 0o600 & !umask());

    let saved = partial.finish("report.txt").await.unwrap();
    assert_eq!(saved, dir.join("report.txt"));
    assert_eq!(std::fs::read(&saved).unwrap(), b"hello world");
    assert_eq!(names(&dir), ["report.txt"]);
    let mode = std::fs::metadata(&saved).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o644 & !umask());
    assert_eq!(mode & 0o111, 0, "never executable");
    assert!(records.list().is_empty());
}

fn umask() -> u32 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let line = status
        .lines()
        .find_map(|l| l.strip_prefix("Umask:"))
        .unwrap();
    u32::from_str_radix(line.trim(), 8).unwrap()
}

#[tokio::test]
async fn an_existing_name_is_never_replaced() {
    for how in [Placement::Auto, Placement::Link] {
        let temp = Temp::new("unique");
        let dir = temp.dir("out");
        let records = temp.records();
        let destination = Destination::open(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "mine").unwrap();
        // A dangling symlink and a directory hold their names too.
        symlink("/nonexistent/target", dir.join("a (2).txt")).unwrap();
        std::fs::create_dir(dir.join("a (3).txt")).unwrap();
        let mut saved = Vec::new();
        for text in ["one", "two", "three"] {
            let mut partial = destination.create_partial("a.txt", &records).unwrap();
            partial.write(text.as_bytes()).await.unwrap();
            saved.push(partial.finish_with("a.txt", how).await.unwrap());
        }
        let expected: Vec<PathBuf> = ["a (1).txt", "a (4).txt", "a (5).txt"]
            .iter()
            .map(|n| dir.join(n))
            .collect();
        assert_eq!(saved, expected, "{how:?}");
        assert_eq!(std::fs::read(dir.join("a.txt")).unwrap(), b"mine");
        assert_eq!(std::fs::read(&saved[0]).unwrap(), b"one");
        assert_eq!(std::fs::read(&saved[2]).unwrap(), b"three");
        assert_eq!(
            std::fs::read_link(dir.join("a (2).txt")).unwrap(),
            Path::new("/nonexistent/target")
        );
        assert!(dir.join("a (3).txt").is_dir());
        assert_eq!(
            names(&dir).len(),
            6,
            "no partial files left: {:?}",
            names(&dir)
        );
    }
}

#[tokio::test]
async fn a_partial_is_deleted_when_dropped_discarded_or_failed() {
    let temp = Temp::new("drop");
    let dir = temp.dir("out");
    let records = temp.records();
    let destination = Destination::open(&dir).unwrap();
    {
        let mut partial = destination.create_partial("x.bin", &records).unwrap();
        partial.write(b"abc").await.unwrap();
        assert_eq!(names(&dir).len(), 1);
    }
    assert!(names(&dir).is_empty());
    assert!(records.list().is_empty());
    let partial = destination.create_partial("x.bin", &records).unwrap();
    partial.discard();
    assert!(names(&dir).is_empty());
    // A finish that fails leaves nothing behind either: the name is not a file name.
    let partial = destination.create_partial("x.bin", &records).unwrap();
    assert!(partial.finish("a/b").await.is_err());
    assert!(names(&dir).is_empty());
    assert!(records.list().is_empty());
}

#[tokio::test]
async fn the_longest_names_still_fit_as_partial_and_final_files() {
    let temp = Temp::new("long");
    let dir = temp.dir("out");
    let records = temp.records();
    let destination = Destination::open(&dir).unwrap();
    let long = name::sanitize(&format!("{}.tar.gz", "n".repeat(500)));
    assert_eq!(long.len(), 255);
    for _ in 0..2 {
        let mut partial = destination.create_partial(&long, &records).unwrap();
        assert!(partial.file_name().len() <= 255);
        partial.write(b"x").await.unwrap();
        let saved = partial.finish(&long).await.unwrap();
        assert!(saved.file_name().unwrap().len() <= 255);
    }
    assert_eq!(names(&dir).len(), 2);
}

#[test]
fn a_symlinked_destination_is_refused() {
    let temp = Temp::new("symlink");
    let real = temp.dir("real");
    let link = temp.0.join("link");
    symlink(&real, &link).unwrap();
    for shown in [
        link.clone(),
        PathBuf::from(format!("{}/", link.display())),
        link.join("."),
    ] {
        let error = Destination::open(&shown).unwrap_err().to_string();
        assert!(error.contains("symbolic link"), "{shown:?}: {error}");
        assert!(
            error.contains(real.to_str().unwrap()),
            "names the real folder: {error}"
        );
    }
    // The folder itself, however it is written, is fine.
    Destination::open(&real).unwrap();
    Destination::open(&PathBuf::from(format!("{}/", real.display()))).unwrap();
    Destination::open(&real.join(".")).unwrap();
    // A symlink above the folder is the user's own arrangement and is resolved.
    let inside = real.join("sub");
    std::fs::create_dir(&inside).unwrap();
    Destination::open(&link.join("sub")).unwrap();
    // Nothing else a folder can't be.
    let file = temp.0.join("file");
    std::fs::write(&file, "").unwrap();
    assert!(
        Destination::open(&file)
            .unwrap_err()
            .to_string()
            .contains("isn't a folder")
    );
    assert!(
        Destination::open(&temp.0.join("missing"))
            .unwrap_err()
            .to_string()
            .contains("doesn't exist")
    );
    assert!(Destination::open(Path::new("relative/dir")).is_err());
    assert!(Destination::open(&real.join("../real")).is_err());
    let dangling = temp.0.join("dangling");
    symlink("/nonexistent", &dangling).unwrap();
    assert!(
        Destination::open(&dangling)
            .unwrap_err()
            .to_string()
            .contains("symbolic link")
    );
}

#[tokio::test]
async fn a_folder_swapped_after_opening_cannot_redirect_the_write() {
    let temp = Temp::new("swap");
    let dir = temp.dir("out");
    let elsewhere = temp.dir("elsewhere");
    let records = temp.records();
    let destination = Destination::open(&dir).unwrap();
    let mut partial = destination.create_partial("f.txt", &records).unwrap();
    // The folder is moved away and a symlink to somewhere else takes its place.
    let moved = temp.0.join("out.moved");
    std::fs::rename(&dir, &moved).unwrap();
    symlink(&elsewhere, &dir).unwrap();
    partial.write(b"data").await.unwrap();
    partial.finish("f.txt").await.unwrap();
    assert_eq!(std::fs::read(moved.join("f.txt")).unwrap(), b"data");
    assert!(names(&elsewhere).is_empty(), "{:?}", names(&elsewhere));
}

#[test]
fn free_space_is_read_from_the_folders_filesystem() {
    let temp = Temp::new("space");
    let destination = Destination::open(&temp.dir("out")).unwrap();
    assert!(destination.free_space().unwrap() > 0);
    assert!(destination.free().unwrap() > 0);
}

#[test]
fn the_download_folder_follows_xdg_user_dirs_else_downloads() {
    let temp = Temp::new("xdg");
    let home = temp.dir("home");
    let config = temp.dir("config");
    let write = |text: &str| std::fs::write(config.join("user-dirs.dirs"), text).unwrap();
    // No file.
    assert_eq!(
        store::default_directory_in(&home, &config),
        home.join("Downloads")
    );
    write("# comment\nXDG_DESKTOP_DIR=\"$HOME/Desktop\"\nXDG_DOWNLOAD_DIR=\"$HOME/Incoming\"\n");
    assert_eq!(
        store::default_directory_in(&home, &config),
        home.join("Incoming")
    );
    write("XDG_DOWNLOAD_DIR=\"/data/dl\"\n");
    assert_eq!(
        store::default_directory_in(&home, &config),
        PathBuf::from("/data/dl")
    );
    write("XDG_DOWNLOAD_DIR=\"$HOME/\"\n");
    assert_eq!(
        store::default_directory_in(&home, &config),
        home.join("Downloads"),
        "disabled points at $HOME"
    );
    write("XDG_DOWNLOAD_DIR=\"relative\"\n");
    assert_eq!(
        store::default_directory_in(&home, &config),
        home.join("Downloads")
    );
    write("XDG_DOCUMENTS_DIR=\"$HOME/Documents\"\n");
    assert_eq!(
        store::default_directory_in(&home, &config),
        home.join("Downloads")
    );
}

// -- Records ---------------------------------------------------------------------------------

#[test]
fn partial_names_have_one_exact_shape() {
    let uuid = "0123abcd-0123-4abc-89ab-0123456789ab";
    assert!(is_partial_name(&format!(".name.txt.{uuid}.rex-part")));
    assert!(is_partial_name(&format!("._.{uuid}.rex-part")));
    for bad in [
        format!("name.{uuid}.rex-part"),
        format!(".name.{uuid}.part"),
        format!(".name.{}.rex-part", uuid.to_uppercase()),
        format!(".{uuid}.rex-part"),
        format!("..{uuid}.rex-part"),
        format!(".a/b.{uuid}.rex-part"),
        format!("./../x.{uuid}.rex-part"),
        format!(".a\0b.{uuid}.rex-part"),
        ".x.not-a-uuid.rex-part".into(),
        ".x.0123abcd-0123-4abc-89ab-0123456789a.rex-part".into(),
        ".x.0123abcd01234abc89ab0123456789abcdef.rex-part".into(),
        "".into(),
        "..".into(),
        "/etc/passwd".into(),
        format!(".{}.{uuid}.rex-part", "n".repeat(300)),
    ] {
        assert!(!is_partial_name(&bad), "{bad:?}");
    }
}

fn leftover(dir: &Path, name: &str, pid: u32, started: u64) -> Record {
    Record {
        dir: dir.into(),
        name: name.into(),
        pid,
        started,
        created: 0,
    }
}

const UUID_A: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const UUID_B: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
const DEAD: u32 = u32::MAX - 1;

fn write_records(records: &Records, list: &[Record]) {
    std::fs::create_dir_all(records.path().parent().unwrap()).unwrap();
    let json = serde_json::json!({"partials": list});
    std::fs::write(records.path(), serde_json::to_vec(&json).unwrap()).unwrap();
}

#[test]
fn leftover_partials_of_dead_processes_are_deleted_and_forgotten() {
    let temp = Temp::new("leftover");
    let dir = temp.dir("out");
    let records = temp.records();
    let partial = format!(".old.txt.{UUID_A}.rex-part");
    std::fs::write(dir.join(&partial), "half").unwrap();
    std::fs::write(dir.join("precious.txt"), "keep").unwrap();
    // Not recorded, so never touched, though it looks the same.
    let unrecorded = format!(".other.{UUID_B}.rex-part");
    std::fs::write(dir.join(&unrecorded), "x").unwrap();
    write_records(&records, &[leftover(&dir, &partial, DEAD, 5)]);
    let deleted = records.clean_up_leftovers();
    assert_eq!(deleted, [dir.join(&partial)]);
    assert_eq!(names(&dir), [unrecorded, "precious.txt".to_owned()]);
    assert!(records.list().is_empty());
    // Nothing left to do.
    assert!(records.clean_up_leftovers().is_empty());
}

#[test]
fn hosts_on_different_threads_never_lose_each_others_records() {
    // Each host runs on its own thread and records its partial files in the same file.
    let temp = Temp::new("concurrent");
    let records = temp.records();
    let dir = temp.0.clone();
    let threads: Vec<_> = (0..8)
        .map(|t| {
            let (records, dir) = (records.clone(), dir.clone());
            std::thread::spawn(move || {
                for n in 0..40 {
                    let name = format!(".t{t}-{n}.{UUID_A}.rex-part");
                    records.add(&dir, &name).expect("a record is written");
                    // Half of them finish straight away.
                    if n % 2 == 0 {
                        records.remove(&dir, &name);
                    }
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    let kept = records.list();
    assert_eq!(kept.len(), 8 * 20, "no record was lost");
    for record in &kept {
        let n: u32 = record.name[1..]
            .split(['-', '.'])
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(n % 2, 1, "{}: a removed record came back", record.name);
    }
    let state = records.path().parent().unwrap();
    assert_eq!(
        names(state),
        ["downloads.json"],
        "no temporary file is left"
    );
}

#[test]
fn a_running_apps_partial_is_not_a_leftover() {
    let temp = Temp::new("running");
    let dir = temp.dir("out");
    let records = temp.records();
    let destination = Destination::open(&dir).unwrap();
    let partial = destination.create_partial("live.bin", &records).unwrap();
    assert!(records.clean_up_leftovers().is_empty());
    assert_eq!(names(&dir), [partial.file_name().to_owned()]);
    assert_eq!(records.list().len(), 1);
    // The same pid with another start time is a recycled pid: its file is a leftover.
    let mut record = records.list().remove(0);
    record.started += 1;
    write_records(&records, &[record]);
    assert_eq!(records.clean_up_leftovers().len(), 1);
    assert!(names(&dir).is_empty());
}

const DAY: u64 = 24 * 60 * 60;

#[test]
fn a_leftover_in_a_folder_that_cannot_be_opened_is_kept_until_the_folder_is_back() {
    // A drive that is not mounted at this launch: its folder is missing. The record stays, and a
    // later launch with the folder back deletes the file.
    let temp = Temp::new("unmounted");
    let drive = temp.dir("drive");
    let dir = drive.join("Downloads");
    std::fs::create_dir(&dir).unwrap();
    let records = temp.records();
    let partial = format!(".big.iso.{UUID_A}.rex-part");
    std::fs::write(dir.join(&partial), "half").unwrap();
    write_records(&records, &[leftover(&dir, &partial, DEAD, 5)]);
    let away = temp.0.join("away");
    std::fs::rename(&drive, &away).unwrap();
    let now = 1_800_000_000;
    assert!(records.clean_up_leftovers_at(now).is_empty());
    let kept = records.list();
    assert_eq!(kept.len(), 1, "the record is kept");
    assert_eq!(
        kept[0].created, now,
        "a record without a date counts as new"
    );
    // Still unreachable a week later: still kept, and its date is not moved.
    assert!(records.clean_up_leftovers_at(now + 7 * DAY).is_empty());
    assert_eq!(records.list()[0].created, now);
    // The drive is back.
    std::fs::rename(&away, &drive).unwrap();
    assert_eq!(
        records.clean_up_leftovers_at(now + 8 * DAY),
        [dir.join(&partial)]
    );
    assert!(names(&dir).is_empty());
    assert!(records.list().is_empty());
}

#[test]
fn an_unreachable_leftover_is_forgotten_after_thirty_days_without_touching_anything() {
    let temp = Temp::new("old-unreachable");
    let dir = temp.0.join("not-mounted");
    let bystander = temp.dir("out");
    std::fs::write(bystander.join("precious.txt"), "keep").unwrap();
    let records = temp.records();
    let partial = format!(".old.{UUID_A}.rex-part");
    let now = 1_800_000_000;
    let mut thirty_one = leftover(&dir, &partial, DEAD, 5);
    thirty_one.created = now - 31 * DAY;
    let mut twenty_nine = leftover(&dir, &format!(".new.{UUID_B}.rex-part"), DEAD, 5);
    twenty_nine.created = now - 29 * DAY;
    write_records(&records, &[thirty_one, twenty_nine.clone()]);
    assert!(records.clean_up_leftovers_at(now).is_empty());
    assert_eq!(records.list(), [twenty_nine]);
    assert_eq!(names(&bystander), ["precious.txt"]);
}

#[test]
fn records_from_before_the_date_was_kept_still_load_and_count_as_new() {
    let temp = Temp::new("old-format");
    let dir = temp.0.join("not-mounted");
    let records = temp.records();
    std::fs::create_dir_all(records.path().parent().unwrap()).unwrap();
    let text = format!(
        r#"{{"partials":[{{"dir":{:?},"name":".a.{UUID_A}.rex-part","pid":{DEAD},"started":5}}]}}"#,
        dir.to_str().unwrap()
    );
    std::fs::write(records.path(), text).unwrap();
    assert_eq!(records.list()[0].created, 0);
    let now = 1_800_000_000;
    // Far in the future of the epoch is not "old": a record with no date starts its 30 days now.
    assert!(records.clean_up_leftovers_at(now).is_empty());
    assert_eq!(records.list()[0].created, now);
    assert!(records.clean_up_leftovers_at(now + 31 * DAY).is_empty());
    assert!(records.list().is_empty());
}

#[test]
fn a_leftover_that_is_already_gone_is_forgotten_and_one_that_cannot_be_removed_is_kept() {
    let temp = Temp::new("unlink");
    let dir = temp.dir("out");
    let records = temp.records();
    let gone = format!(".gone.{UUID_A}.rex-part");
    let stuck = format!(".stuck.{UUID_B}.rex-part");
    std::fs::write(dir.join(&stuck), "half").unwrap();
    write_records(
        &records,
        &[
            leftover(&dir, &gone, DEAD, 5),
            leftover(&dir, &stuck, DEAD, 5),
        ],
    );
    // A read-only folder: the file is there, and unlinking it fails.
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    let now = 1_800_000_000;
    let deleted = records.clean_up_leftovers_at(now);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    if unsafe { libc::geteuid() } == 0 {
        // Root ignores the mode: nothing to prove here, only that it was deleted.
        assert_eq!(deleted, [dir.join(&stuck)]);
        return;
    }
    assert!(deleted.is_empty());
    let kept: Vec<String> = records.list().into_iter().map(|r| r.name).collect();
    assert_eq!(
        kept,
        std::slice::from_ref(&stuck),
        "the absent file's record is dropped"
    );
    assert_eq!(names(&dir), std::slice::from_ref(&stuck));
    // Writable again: the next launch deletes it.
    assert_eq!(records.clean_up_leftovers_at(now + DAY), [dir.join(&stuck)]);
    assert!(records.list().is_empty());
}

#[test]
fn a_tampered_record_can_only_delete_a_regular_partial_file_in_its_folder() {
    let temp = Temp::new("tamper");
    let dir = temp.dir("out");
    let victim_dir = temp.dir("victim");
    let records = temp.records();
    let victim = victim_dir.join("victim.txt");
    std::fs::write(&victim, "important").unwrap();
    std::fs::write(victim_dir.join("sibling"), "important").unwrap();
    let shape = |uuid: &str| format!(".x.{uuid}.rex-part");

    // A symlink named like a partial file, pointing at a victim: not followed, not even removed.
    let link_name = shape(UUID_A);
    symlink(&victim, dir.join(&link_name)).unwrap();
    // A directory named like one.
    let dir_name = shape(UUID_B);
    std::fs::create_dir(dir.join(&dir_name)).unwrap();
    std::fs::write(dir.join(&dir_name).join("inner"), "i").unwrap();
    // A folder that is a symlink to the victim's folder, with a matching partial name inside.
    let linked = temp.0.join("linked");
    symlink(&victim_dir, &linked).unwrap();
    let inside = shape("cccccccc-cccc-4ccc-8ccc-cccccccccccc");
    std::fs::write(victim_dir.join(&inside), "important").unwrap();
    // A file whose name doesn't have the form.
    std::fs::write(dir.join("plain.txt"), "important").unwrap();

    write_records(
        &records,
        &[
            leftover(&dir, &link_name, DEAD, 1),
            leftover(&dir, &dir_name, DEAD, 1),
            leftover(&linked, &inside, DEAD, 1),
            leftover(&dir, "plain.txt", DEAD, 1),
            leftover(&dir, &format!("../victim/{}", shape(UUID_A)), DEAD, 1),
            leftover(&dir, victim.to_str().unwrap(), DEAD, 1),
            leftover(&dir.join("../victim"), "victim.txt", DEAD, 1),
            leftover(&dir.join("..").join("victim"), &inside, DEAD, 1),
            leftover(Path::new("relative"), &inside, DEAD, 1),
            leftover(Path::new("/"), &inside, DEAD, 1),
            leftover(&dir, "", DEAD, 1),
            leftover(&dir, "..", DEAD, 1),
        ],
    );
    assert!(records.clean_up_leftovers().is_empty());
    assert_eq!(std::fs::read(&victim).unwrap(), b"important");
    assert!(victim_dir.join(&inside).exists());
    assert!(
        std::fs::symlink_metadata(dir.join(&link_name))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(dir.join(&dir_name).join("inner").exists());
    assert!(dir.join("plain.txt").exists());
    assert!(victim_dir.join("sibling").exists());
    // The bad records are dropped, not kept for another try.
    assert!(records.list().is_empty());
}

#[test]
fn damaged_or_missing_record_files_are_empty_not_errors() {
    let temp = Temp::new("garbage");
    let records = temp.records();
    assert!(records.clean_up_leftovers().is_empty());
    std::fs::create_dir_all(records.path().parent().unwrap()).unwrap();
    for text in [
        "",
        "not json",
        "[1,2]",
        "{\"partials\":[{\"dir\":1}]}",
        "{}",
    ] {
        std::fs::write(records.path(), text).unwrap();
        assert!(records.list().is_empty(), "{text}");
        assert!(records.clean_up_leftovers().is_empty(), "{text}");
    }
    assert_eq!(store::expand_home("/abs"), PathBuf::from("/abs"));
    assert_eq!(store::expand_home("rel"), PathBuf::from("rel"));
    assert!(store::expand_home("~/x").ends_with("x"));
    assert!(store::expand_home("~").is_absolute() || std::env::var_os("HOME").is_none());
    assert_eq!(store::expand_home("~other/x"), PathBuf::from("~other/x"));
}

// -- Leftovers at the next launch, and the record rules ---------------------------------------

/// Not a test of its own: the child process of
/// `a_killed_apps_partial_is_deleted_by_the_next_launch`. It does nothing unless that test
/// started it with the folder and records file to use, and then it dies the way a killed app
/// does: partial file written, no destructor run.
#[test]
fn killed_app_child() {
    let (Some(dir), Some(records)) = (
        std::env::var_os("REX_TEST_KILLED_DIR"),
        std::env::var_os("REX_TEST_KILLED_RECORDS"),
    ) else {
        return;
    };
    let destination = Destination::open(Path::new(&dir)).unwrap();
    let partial = destination
        .create_partial("big.iso", &Records::at(records.into()))
        .unwrap();
    std::fs::write(Path::new(&dir).join(partial.file_name()), b"half a file").unwrap();
    unsafe { libc::_exit(7) }
}

#[test]
fn a_killed_apps_partial_is_deleted_by_the_next_launch_and_a_live_ones_is_kept() {
    let temp = Temp::new("killed");
    let dir = temp.dir("out");
    let records = temp.records();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "download::tests::killed_app_child",
            "--test-threads=1",
        ])
        .env("REX_TEST_KILLED_DIR", &dir)
        .env("REX_TEST_KILLED_RECORDS", records.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(7),
        "the child died as planned: {output:?}"
    );

    // This process is a second, running app with a partial file of its own, and there is an
    // unrelated file that only looks like a partial one.
    let destination = Destination::open(&dir).unwrap();
    let live = destination.create_partial("live.bin", &records).unwrap();
    let unrecorded = format!(".other.{UUID_B}.rex-part");
    std::fs::write(dir.join(&unrecorded), "x").unwrap();
    let recorded = records.list();
    assert_eq!(recorded.len(), 2);
    let dead = recorded
        .iter()
        .find(|r| r.name.contains("big.iso"))
        .unwrap();
    assert!(dead.pid != std::process::id() && dir.join(&dead.name).exists());

    // The next launch.
    let deleted = records.clean_up_leftovers();
    assert_eq!(deleted, [dir.join(&dead.name)]);
    assert!(!dir.join(&dead.name).exists());
    assert!(
        dir.join(live.file_name()).exists(),
        "a running app's file stays"
    );
    assert!(
        dir.join(&unrecorded).exists(),
        "an unrecorded file is never touched"
    );
    let left = records.list();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].name, live.file_name());
}

#[tokio::test]
async fn a_partial_name_that_cannot_be_removed_after_linking_stays_recorded() {
    let temp = Temp::new("linked");
    let dir = temp.dir("out");
    let records = temp.records();
    let destination = Destination::open(&dir).unwrap();
    let nlink = |name: &str| {
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata(dir.join(name)).unwrap().nlink()
    };

    // The fallback that works: one name is left, and nothing is recorded.
    let mut partial = destination.create_partial("ok.bin", &records).unwrap();
    partial.write(b"data").await.unwrap();
    let saved = partial
        .finish_with("ok.bin", Placement::Link)
        .await
        .unwrap();
    assert_eq!(saved, dir.join("ok.bin"));
    assert_eq!(names(&dir), ["ok.bin"]);
    assert!(records.list().is_empty());

    // The same, but the partial name can't be removed: the file is saved, the hidden name is a
    // second hard link, and its record stays, so the next launch can remove that name.
    let mut partial = destination.create_partial("a.bin", &records).unwrap();
    partial.write(b"data").await.unwrap();
    let hidden = partial.file_name().to_owned();
    let saved = partial
        .finish_with("a.bin", Placement::LinkUnlinkFails)
        .await
        .unwrap();
    assert_eq!(saved, dir.join("a.bin"));
    assert_eq!(std::fs::read(&saved).unwrap(), b"data");
    assert_eq!(nlink("a.bin"), 2);
    assert!(dir.join(&hidden).exists());
    let recorded = records.list();
    assert_eq!(recorded.len(), 1, "the record was kept");
    assert_eq!(recorded[0].name, hidden);

    // The next launch (its owner is gone) removes the extra name, and only that.
    let mut record = recorded[0].clone();
    record.started += 1;
    write_records(&records, &[record]);
    assert_eq!(records.clean_up_leftovers(), [dir.join(&hidden)]);
    assert_eq!(names(&dir), ["a.bin", "ok.bin"]);
    assert_eq!(std::fs::read(&saved).unwrap(), b"data");
    assert_eq!(nlink("a.bin"), 1);
    assert!(records.list().is_empty());
}

#[test]
fn no_partial_file_is_made_when_its_record_cannot_be_written() {
    let temp = Temp::new("norecord");
    let dir = temp.dir("out");
    // The records file's folder would have to be inside a file.
    let blocker = temp.0.join("blocker");
    std::fs::write(&blocker, "not a folder").unwrap();
    let records = Records::at(blocker.join("downloads.json"));
    let destination = Destination::open(&dir).unwrap();
    let Err(error) = destination.create_partial("x.bin", &records) else {
        panic!("a partial file was made with nowhere to record it");
    };
    let shown = format!("{error:#}");
    assert!(
        shown.contains("downloads.json"),
        "names the records file: {shown}"
    );
    assert!(names(&dir).is_empty(), "no file without a record");
}
