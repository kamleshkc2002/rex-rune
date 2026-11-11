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
        (Reason::Large, GIB, free)
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
    // Less free space than the reserve: any bytes at all are too many.
    assert_eq!(
        limits.confirmation(1, 100 * MIB).unwrap().reason,
        Reason::LowSpace
    );
    assert_eq!(limits.confirmation(0, 100 * MIB), None);
    assert_eq!(limits.confirmation(0, 0), None);
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
            free: 1,
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
        bytes: 100
    }));
    assert_eq!(
        t.state,
        State::Saved {
            path: "/d/b.txt".into(),
            bytes: 100
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
        free: 10 * GIB,
    };
    let mut t = fresh();
    assert!(t.apply(begun(2 * GIB, Some(confirmation))));
    assert_eq!(t.state, State::AwaitingConfirmation(confirmation));
    assert!(!t.state.is_final());
    // Progress before the yes is out of order.
    assert!(!t.apply(Event::Progress { received: 1 }));
    assert!(!t.apply(Event::Saved {
        path: "/x".into(),
        bytes: 0
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
        free: 1,
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
    assert_eq!(long.chars().count(), 301);
    assert!(long.ends_with('…'));
    assert_eq!(display_text(&"界".repeat(300)).chars().count(), 300);
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
