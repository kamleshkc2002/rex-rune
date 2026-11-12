//! Headless checks of the download UI: the action's remote class, the dialog, the chip, the
//! popover and what a finished transfer may offer. No server: the host is a channel the test
//! reads, and its model is fed transfers the way the worker would.
use super::*;
use crate::app::{
    actions::{self, Remote},
    panel::PaneAction,
};
use rex_linux::{
    connection::downloads::Request,
    download::{Confirmation, Failure, Reason, State, Transfer},
};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

/// A new empty folder for one test's downloads.
fn scratch() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "rex-download-ui-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Two panes on a host speaking revision 13, saving into a private folder.
fn workspace_13() -> (Workspace, flume::Receiver<HostCommand>, PathBuf) {
    let (mut ws, rx) = tabbed_workspace("split");
    ws.hosts[0].model.protocol.api_revision = 13;
    let dir = scratch();
    ws.settings.downloads.directory = Some(dir.display().to_string());
    (ws, rx, dir)
}

fn pane_menu_download(ws: &mut Workspace, block: &str) {
    let target = ws.target(block).expect("a pane");
    ws.perform(
        Intent::Pane(target, PaneAction::Download),
        &mut rune::Tx::default(),
    );
}

/// What the host was asked: its downloads, and the answers to their questions.
enum Asked {
    Download(Box<Request>),
    Confirm(u64),
    Cancel(u64),
}
fn asked(rx: &flume::Receiver<HostCommand>) -> Vec<Asked> {
    rx.try_iter()
        .filter_map(|c| match c {
            HostCommand::Download(request) => Some(Asked::Download(request)),
            HostCommand::DownloadConfirm(id) => Some(Asked::Confirm(id)),
            HostCommand::DownloadCancel(id) => Some(Asked::Cancel(id)),
            _ => None,
        })
        .collect()
}

fn transfer(id: u64, name: &str, dir: &Path, state: State) -> Transfer {
    let mut t = Transfer::new(
        id,
        "s".into(),
        "block:b".into(),
        format!("~/{name}"),
        dir.to_owned(),
        1,
    );
    t.name = Some(name.into());
    t.resolved = Some(format!("/home/dev/{name}"));
    t.state = state;
    t
}

fn receiving(received: u64, hint: u64) -> State {
    State::Receiving { received, hint }
}

/// Change the host's list of transfers the way its worker does, then deliver the wake.
fn feed(rt: &mut Runtime<Workspace>, f: impl FnOnce(&mut Vec<Transfer>)) {
    rt.update(|s, tx| {
        let mut model = s.hosts[0].model.clone();
        f(&mut model.downloads);
        *s.hosts[0].connection.model.lock().unwrap() = model;
        s.sync(0, tx);
    });
    // Long enough for leaving rows and a growing popover to settle.
    for _ in 0..40 {
        rt.frame(1. / 60.);
    }
}

fn set_field(rt: &mut Runtime<Workspace>, index: usize, text: &str) {
    let mut edit = rune::TextEdit::new(text);
    edit.focused = index == 0;
    rt.state.panel.as_mut().unwrap().inputs[index] = edit;
    rt.frame(1. / 60.);
}

fn open_dialog(ws: Workspace) -> Runtime<Workspace> {
    runtime(ws)
}

fn texts(rt: &Runtime<Workspace>) -> Vec<String> {
    rt.elements()
        .into_iter()
        .filter(|e| !e.exiting)
        .filter_map(|e| e.text)
        .collect()
}

fn shows(rt: &Runtime<Workspace>, needle: &str) -> bool {
    texts(rt).iter().any(|t| t.contains(needle))
}

fn label_of(rt: &Runtime<Workspace>, key: &str) -> Option<(&'static str, String)> {
    rt.elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some(key))
        .map(|e| (e.role, e.label.unwrap_or_default()))
}

/// A frame of `cols` columns where `text` is the screen, so a selection can be copied.
fn screen_with(ws: &mut Workspace, block: &str, text: &[u8], selection: (usize, usize)) {
    use rex_linux::connection::{Connection, TerminalSender};
    let (sender, _sent) = flume::unbounded();
    let connection = Connection {
        model: Default::default(),
        commands: TerminalSender::new(sender, "s".into(), block.into()),
        wake: flume::unbounded().1,
    };
    let mut terminal = ghostty_vt::Terminal::new(ghostty_vt::TerminalOptions {
        cols: 40,
        rows: 4,
        ..Default::default()
    })
    .unwrap();
    terminal.vt_write(text);
    let frame = std::sync::Arc::new(
        rex_linux::terminal::Renderer::new()
            .unwrap()
            .frame(&terminal)
            .unwrap(),
    );
    {
        let mut m = connection.model.lock().unwrap();
        m.connected = true;
        m.control_connected = true;
        m.frame = Some(frame.clone());
    }
    let mut term = crate::app::term::Term::new(connection);
    term.frame = Some(frame);
    term.selection = Some(selection);
    // A different revision than the frame's makes the selection read from the screen.
    term.selection_revision = 1;
    let key = ws.term_key(block);
    ws.terminals.insert(key, term);
}

#[test]
fn pane_download_is_never_remote_and_the_allowed_set_stays_exactly_eight() {
    let spec = actions::find("pane.download").expect("registered");
    assert!(matches!(spec.remote(), Remote::Never(_)));
    assert_eq!(spec.title(), "Download file");
    assert!(spec.accepts(&serde_json::Value::Null));
    let mut allowed: Vec<_> = actions::SPECS
        .iter()
        .filter(|s| matches!(s.remote(), Remote::View { .. }))
        .map(|s| s.name)
        .collect();
    allowed.sort();
    assert_eq!(allowed.len(), 8, "{allowed:?}");
    assert!(!allowed.contains(&"pane.download"));
    // Declared to the server as unavailable remotely, like the other dialogs.
    let declared = actions::declarations()
        .into_iter()
        .find(|a| a.name == "pane.download")
        .unwrap();
    assert!(
        declared
            .description
            .unwrap()
            .contains("Not available remotely")
    );

    // A client.perform request is refused, with consent on, and starts nothing.
    let (mut ws, rx, _dir) = workspace_13();
    ws.settings.hosts[0].remote_control = true;
    ws.remote_perform(
        0,
        remote_tests::pending("pane.download", serde_json::json!({})),
        &mut rune::Tx::default(),
    );
    remote_tests::code(remote_tests::reply(&rx), "not_supported");
    assert!(ws.panel.is_none(), "no dialog");
    assert!(asked(&rx).is_empty());
}

#[test]
fn a_returned_client_action_cannot_open_the_download_dialog() {
    let (mut ws, rx, _dir) = workspace_13();
    let mut model = ws.hosts[0].model.clone();
    model
        .client_actions
        .push(rex_linux::connection::actions::Queued {
            generation: ws.keymap_generation,
            session: Some("s".into()),
            block: Some("block:b".into()),
            action: rexrs::api::client::ClientAction {
                action: "pane.download".into(),
                args: None,
            },
        });
    *ws.hosts[0].connection.model.lock().unwrap() = model;
    ws.sync(0, &mut rune::Tx::default());
    assert!(ws.panel.is_none());
    assert!(asked(&rx).is_empty());
    assert!(ws.notices.iter().any(|n| n.text.contains("cannot start")));
}

#[test]
fn a_key_bound_to_pane_download_opens_the_dialog_for_the_focused_pane() {
    let (ws, _rx, _dir) = workspace_13();
    let mut rt = runtime(ws);
    rt.update(|s, tx| {
        assert!(actions::run(
            s,
            "pane.download",
            &serde_json::Value::Null,
            tx
        ));
    });
    rt.frame(1. / 60.);
    let panel = rt.state.panel.as_ref().expect("the dialog");
    assert!(matches!(panel.kind, PanelKind::Download(_)));
    assert_eq!(panel.target.as_ref().unwrap().block, "block:b");
    assert!(
        rt.state
            .shortcut_detail("pane.download", &serde_json::Value::Null)
            .is_empty(),
        "no default key"
    );
}

#[test]
fn pane_menu_and_commands_list_download_file_and_commands_list_downloads() {
    let (mut ws, _rx, _dir) = workspace_13();
    ws.pane_menu("block:b".into(), None);
    let labels: Vec<_> = ws.entries().into_iter().map(|e| e.label).collect();
    assert!(labels.contains(&"Download file…".to_owned()), "{labels:?}");
    ws.picker(false);
    let labels: Vec<_> = ws.entries().into_iter().map(|e| e.label).collect();
    assert!(labels.contains(&"Download file…".to_owned()), "{labels:?}");
    assert!(labels.contains(&"Downloads".to_owned()), "{labels:?}");
    // Even a host that cannot send files keeps the action visible.
    let (mut old, _rx, _dir) = workspace_13();
    old.hosts[0].model.protocol.api_revision = 12;
    old.pane_menu("block:b".into(), None);
    assert!(
        old.entries()
            .iter()
            .any(|e| e.label == "Download file…" && e.enabled)
    );
}

#[test]
fn the_dialog_names_the_host_focuses_its_field_and_escape_closes_it_without_asking() {
    let (mut ws, rx, dir) = workspace_13();
    pane_menu_download(&mut ws, "block:b");
    let mut rt = open_dialog(ws);
    let panel = rt.state.panel.as_ref().unwrap();
    assert_eq!(panel.title, "Download from test-0");
    assert_eq!(panel.fields[0].0, "File on test-0");
    assert!(shows(&rt, "Download from test-0"));
    assert!(shows(&rt, "Save in"));
    assert!(shows(&rt, &dir.display().to_string()), "the folder shows");
    let path = focused(&rt).expect("a focused field");
    assert!(
        path.contains("field:0"),
        "the file field has focus, not {path}"
    );
    assert!(has(&rt, "target-outline"), "the captured pane is outlined");
    assert_eq!(
        label_of(&rt, "dialog"),
        Some(("dialog", "Download from test-0".to_owned()))
    );
    press(&mut rt, Key::Escape);
    assert!(rt.state.panel.is_none(), "Escape closes");
    assert!(asked(&rx).is_empty(), "closing asks nothing");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn only_a_single_line_selection_prefills_the_field() {
    for (screen, selection, expected) in [
        (
            &b"cat /var/log/build.log"[..],
            (0, 21),
            "cat /var/log/build.log",
        ),
        (b"first line\r\nsecond line", (0, 52), ""),
        (b"cat /etc/hosts", (3, 14), "/etc/hosts"),
    ] {
        let (mut ws, _rx, dir) = workspace_13();
        screen_with(&mut ws, "block:b", screen, selection);
        let copied = ws.terminals[&ws.term_key("block:b")].copy_text();
        pane_menu_download(&mut ws, "block:b");
        let got = ws.panel.as_ref().unwrap().inputs[0].text.clone();
        assert_eq!(got, expected, "selection {copied:?}");
        std::fs::remove_dir_all(dir).ok();
    }
    // Without a selection the field starts empty.
    let (mut ws, _rx, dir) = workspace_13();
    pane_menu_download(&mut ws, "block:b");
    assert_eq!(ws.panel.as_ref().unwrap().inputs[0].text, "");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn enter_sends_exactly_one_download_for_the_captured_pane_and_folder() {
    let (mut ws, rx, dir) = workspace_13();
    // The dialog is opened from the pane that is not focused: it keeps that pane.
    assert_eq!(ws.focused_block().as_deref(), Some("block:b"));
    pane_menu_download(&mut ws, "block:c");
    let mut rt = open_dialog(ws);
    set_field(&mut rt, 0, "  ~/build.log ");
    press(&mut rt, Key::Enter);
    let sent = asked(&rx);
    let [Asked::Download(request)] = &sent[..] else {
        panic!("exactly one download, got {} commands", sent.len());
    };
    assert_eq!(request.session, "s");
    assert_eq!(request.block, "block:c", "the pane captured when it opened");
    assert_eq!(request.path, "~/build.log");
    assert_eq!(request.directory, dir);
    // It waits for Rex in the dialog, and a second Enter asks nothing more.
    assert!(rt.state.panel.is_some());
    assert!(shows(&rt, "Asking test-0"));
    press(&mut rt, Key::Enter);
    assert!(
        asked(&rx).is_empty(),
        "Enter while waiting asks and cancels nothing"
    );
    assert!(rt.state.panel.is_some());
    press(&mut rt, Key::Escape);
    assert!(
        matches!(&asked(&rx)[..], [Asked::Cancel(_)]),
        "Escape cancels the request"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn an_empty_path_is_refused_in_the_dialog_before_anything_is_sent() {
    let (mut ws, rx, dir) = workspace_13();
    pane_menu_download(&mut ws, "block:b");
    let mut rt = open_dialog(ws);
    press(&mut rt, Key::Enter);
    assert!(asked(&rx).is_empty());
    assert!(shows(&rt, "Enter the path of a file on test-0."));
    assert_eq!(
        label_of(&rt, "download-error").map(|(role, _)| role),
        Some("alert")
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_large_file_keeps_the_dialog_open_with_cancel_focused_and_both_answers_work() {
    let confirmation = Confirmation {
        reason: Reason::Large,
        hint: 3_200_000_000,
        free: Some(41_000_000_000),
    };
    for answer in ["cancel", "confirm"] {
        let (mut ws, rx, dir) = workspace_13();
        pane_menu_download(&mut ws, "block:b");
        let mut rt = open_dialog(ws);
        set_field(&mut rt, 0, "/home/dev/build.img");
        press(&mut rt, Key::Enter);
        let id = match &asked(&rx)[..] {
            [Asked::Download(r)] => r.id,
            _ => panic!("one download"),
        };
        let mut t = transfer(
            id,
            "build.img",
            &dir,
            State::AwaitingConfirmation(confirmation),
        );
        t.requested = "/home/dev/build.img".into();
        feed(&mut rt, |list| list.push(t));
        assert!(rt.state.panel.is_some(), "the dialog stays open");
        let question = format!("build.img is 3.2 GB. {} has 41 GB free.", dir.display());
        assert!(shows(&rt, &question), "{:?}", texts(&rt));
        assert!(shows(&rt, "/home/dev/build.img"), "the resolved path shows");
        let path = focused(&rt).expect("a focused button");
        assert!(
            path.ends_with("cancel-download"),
            "Cancel has focus, not {path}"
        );
        assert!(asked(&rx).is_empty(), "nothing is answered for the person");
        if answer == "cancel" {
            press(&mut rt, Key::Enter);
            assert!(rt.state.panel.is_none());
            let sent = asked(&rx);
            assert!(
                matches!(&sent[..], [Asked::Cancel(c)] if *c == id),
                "{} commands",
                sent.len()
            );
        } else {
            click(&mut rt, "confirm-download");
            assert!(matches!(&asked(&rx)[..], [Asked::Confirm(c)] if *c == id));
            // Rex then starts sending: the chip takes over and the dialog goes.
            feed(&mut rt, |list| list[0].state = receiving(0, 3_200_000_000));
            assert!(rt.state.panel.is_none());
            assert!(has(&rt, "download-chip"));
        }
        std::fs::remove_dir_all(dir).ok();
    }
}

#[test]
fn escape_on_the_question_cancels_it_and_an_unseen_question_is_cancelled_too() {
    let confirmation = Confirmation {
        reason: Reason::LowSpace,
        hint: 5_000_000_000,
        free: None,
    };
    let (mut ws, rx, dir) = workspace_13();
    pane_menu_download(&mut ws, "block:b");
    let mut rt = open_dialog(ws);
    set_field(&mut rt, 0, "/x/y.img");
    press(&mut rt, Key::Enter);
    let id = match &asked(&rx)[..] {
        [Asked::Download(r)] => r.id,
        _ => panic!("one download"),
    };
    feed(&mut rt, |list| {
        list.push(transfer(
            id,
            "y.img",
            &dir,
            State::AwaitingConfirmation(confirmation),
        ))
    });
    assert!(shows(&rt, "y.img is 5 GB."), "free unknown: only the size");
    press(&mut rt, Key::Escape);
    assert!(rt.state.panel.is_none());
    assert!(matches!(&asked(&rx)[..], [Asked::Cancel(c)] if *c == id));

    // A question whose dialog is gone (here: never opened) is cancelled, once.
    feed(&mut rt, |list| {
        list[0].state = State::Cancelled;
        list.push(transfer(
            99,
            "z.img",
            &dir,
            State::AwaitingConfirmation(confirmation),
        ))
    });
    assert!(matches!(&asked(&rx)[..], [Asked::Cancel(99)]));
    feed(&mut rt, |_| {});
    assert!(asked(&rx).is_empty(), "not asked twice");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn an_old_server_says_so_and_nothing_is_sent() {
    let (mut ws, rx, dir) = workspace_13();
    ws.hosts[0].model.protocol.api_revision = 12;
    pane_menu_download(&mut ws, "block:b");
    let mut rt = open_dialog(ws);
    assert!(shows(
        &rt,
        "Rex on test-0 is too old to send files; update Rex there."
    ));
    assert!(!has(&rt, "start-download"), "there is nothing to start");
    assert!(!has(&rt, "field:0"));
    press(&mut rt, Key::Enter);
    assert!(rt.state.panel.is_none(), "Enter on Close closes");
    assert!(asked(&rx).is_empty());
    rt.update(|s, _| s.start_download());
    assert!(asked(&rx).is_empty());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn errors_before_any_bytes_stay_in_the_dialog_in_the_servers_words() {
    let failures = [
        (
            Failure::Server("No such file or directory: /nope".into()),
            "No such file or directory: /nope",
        ),
        (
            Failure::Server("/etc is a directory".into()),
            "/etc is a directory",
        ),
        (
            Failure::Server("Permission denied".into()),
            "Permission denied",
        ),
        (
            Failure::Destination("/dl is a symbolic link to /elsewhere".into()),
            "/dl is a symbolic link to /elsewhere",
        ),
        (Failure::Disconnected, "The host isn't connected."),
        (
            Failure::ServerTooOld,
            "Rex on test-0 is too old to send files; update Rex there.",
        ),
        (
            Failure::ConfirmationExpired,
            "Not confirmed in time; Rex stops holding a file after a minute.",
        ),
        (
            Failure::LowSpace { free: 0 },
            "The destination has only 0 bytes free, too little to save this file and keep some space free.",
        ),
        (
            Failure::NoAnswer,
            "Rex didn't answer in time; the download was not retried.",
        ),
    ];
    for (failure, words) in failures {
        let (mut ws, rx, dir) = workspace_13();
        pane_menu_download(&mut ws, "block:b");
        let mut rt = open_dialog(ws);
        set_field(&mut rt, 0, "~/nope");
        press(&mut rt, Key::Enter);
        let id = match &asked(&rx)[..] {
            [Asked::Download(r)] => r.id,
            _ => panic!("one download"),
        };
        feed(&mut rt, |list| {
            list.push(transfer(id, "nope", &dir, State::Failed(failure.clone())))
        });
        assert!(rt.state.panel.is_some(), "{failure:?} stays in the dialog");
        assert!(shows(&rt, words), "{failure:?}: {:?}", texts(&rt));
        assert!(has(&rt, "field:0"), "the form is back, ready to try again");
        assert_eq!(
            rt.state.panel.as_ref().unwrap().inputs[0].text,
            "~/nope",
            "what was typed is kept"
        );
        assert!(
            rt.state.notices.iter().all(|n| !n.sticky),
            "no sticky notice for {failure:?}"
        );
        // Another try is possible, and a second download is a new request.
        press(&mut rt, Key::Enter);
        assert!(matches!(&asked(&rx)[..], [Asked::Download(_)]));
        std::fs::remove_dir_all(dir).ok();
    }
}

#[test]
fn a_fourth_transfer_is_refused_in_the_dialog_and_in_a_notice() {
    let (mut ws, rx, dir) = workspace_13();
    pane_menu_download(&mut ws, "block:b");
    let mut rt = open_dialog(ws);
    set_field(&mut rt, 0, "~/d");
    press(&mut rt, Key::Enter);
    let id = match &asked(&rx)[..] {
        [Asked::Download(r)] => r.id,
        _ => panic!("one download"),
    };
    feed(&mut rt, |list| {
        list.push(transfer(
            id,
            "d",
            &dir,
            State::Failed(Failure::TooManyActive { limit: 3 }),
        ))
    });
    assert!(shows(&rt, "3 downloads from this host are already running"));
    assert!(
        rt.state
            .notices
            .iter()
            .any(|n| n.text.contains("refused") && n.text.contains("already running"))
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_failure_after_bytes_started_is_a_sticky_notice_and_a_save_is_one_information_notice() {
    let (ws, _rx, dir) = workspace_13();
    let mut rt = runtime(ws);
    std::fs::write(dir.join("build.log"), b"hello").unwrap();
    feed(&mut rt, |list| {
        list.push(transfer(
            1,
            "build.log",
            &dir,
            State::Saved {
                path: dir.join("build.log"),
                bytes: 5,
                short_of: None,
            },
        ));
        list.push(transfer(
            2,
            "big.bin",
            &dir,
            State::Failed(Failure::Stalled),
        ));
        list.push(transfer(3, "gone.bin", &dir, State::Cancelled));
    });
    let notices: Vec<_> = rt
        .state
        .notices
        .iter()
        .map(|n| (n.text.clone(), n.sticky))
        .collect();
    assert_eq!(
        notices,
        [
            (
                format!(
                    "Saved build.log from test-0 to {}",
                    crate::app::downloads::tilde(&dir)
                ),
                false
            ),
            (
                "Download of big.bin from test-0 failed: no data for 30 seconds".to_owned(),
                true
            ),
        ],
        "a cancel the person asked for says nothing"
    );
    // Told once: later wakes add nothing.
    feed(&mut rt, |_| {});
    assert_eq!(rt.state.notices.len(), 2);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn the_chip_shows_one_transfer_with_its_percent_several_as_a_count_and_nothing_otherwise() {
    let (ws, _rx, dir) = workspace_13();
    let mut rt = runtime(ws);
    assert!(!has(&rt, "download-chip"), "no chip with nothing running");
    feed(&mut rt, |list| {
        list.push(transfer(1, "build.log", &dir, receiving(42, 100)));
    });
    assert!(has(&rt, "download-chip"));
    assert!(shows(&rt, "↓ build.log"), "{:?}", texts(&rt));
    assert!(shows(&rt, "42%"));
    assert_eq!(
        label_of(&rt, "download-chip"),
        Some(("button", "Downloads: build.log 42%".to_owned()))
    );
    feed(&mut rt, |list| {
        list.push(transfer(2, "b.txt", &dir, receiving(0, 0)));
        list.push(transfer(3, "c.txt", &dir, receiving(5, 9)));
    });
    assert!(shows(&rt, "↓ 3 files"));
    assert!(!shows(&rt, "42%"));
    feed(&mut rt, |list| {
        list[1].state = State::Cancelled;
        list[2].state = State::Cancelled;
    });
    assert!(shows(&rt, "↓ build.log"), "back to one");
    feed(&mut rt, |list| list[0].state = State::Cancelled);
    assert!(!has(&rt, "download-chip"), "gone when nothing runs");
    // Finished transfers never bring the chip back.
    feed(&mut rt, |_| {});
    assert!(!has(&rt, "download-chip"));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_transfer_still_waiting_for_rex_or_for_an_answer_has_no_chip() {
    let (ws, _rx, dir) = workspace_13();
    let mut rt = runtime(ws);
    feed(&mut rt, |list| {
        list.push(transfer(1, "a.iso", &dir, State::Requested));
        list.push(transfer(
            2,
            "b.iso",
            &dir,
            State::AwaitingConfirmation(Confirmation {
                reason: Reason::Large,
                hint: 2_000_000_000,
                free: None,
            }),
        ));
    });
    assert!(!has(&rt, "download-chip"), "nothing is being received");
    feed(&mut rt, |list| list[0].state = receiving(1, 10));
    assert!(has(&rt, "download-chip"));
    assert!(shows(&rt, "↓ a.iso"), "only the receiving one counts");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_long_name_is_cut_in_the_middle_and_the_chip_keeps_its_percent() {
    let (ws, _rx, dir) = workspace_13();
    let mut rt = runtime(ws);
    let long = format!("{}.tar.gz", "very-long-file-name-".repeat(12));
    assert!(long.len() <= 255);
    feed(&mut rt, |list| {
        list.push(transfer(1, &long, &dir, receiving(7, 100)))
    });
    let chip = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("download-chip"))
        .unwrap()
        .rect;
    assert!(
        chip[2] <= 190.5,
        "the chip stays within its width: {chip:?}"
    );
    assert!(shows(&rt, "7%"));
    assert!(shows(&rt, "…"), "cut in the middle");
    assert!(
        shows(&rt, "tar.gz") || shows(&rt, ".gz"),
        "the ending stays"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn the_popover_has_a_row_for_every_state() {
    let (mut ws, rx, dir) = workspace_13();
    std::fs::write(dir.join("notes.txt"), b"plain text").unwrap();
    ws.perform(Intent::Downloads, &mut rune::Tx::default());
    let mut rt = runtime(ws);
    assert!(shows(&rt, "No downloads yet"), "the empty state");
    feed(&mut rt, |list| {
        list.push(transfer(
            1,
            "notes.txt",
            &dir,
            State::Saved {
                path: dir.join("notes.txt"),
                bytes: 10,
                short_of: None,
            },
        ));
        list.push(transfer(
            2,
            "online",
            &dir,
            receiving(12_400_000, 80_000_000),
        ));
        list.push(transfer(
            3,
            "sysfs",
            &dir,
            State::Saved {
                path: dir.join("sysfs"),
                bytes: 6,
                short_of: Some(4096),
            },
        ));
        list.push(transfer(4, "nope", &dir, State::Cancelled));
        list.push(transfer(
            5,
            "bad",
            &dir,
            State::Failed(Failure::Server("Permission denied".into())),
        ));
        list.push(transfer(
            6,
            "old",
            &dir,
            State::Failed(Failure::ServerTooOld),
        ));
        list.push(transfer(7, "wait", &dir, State::Requested));
    });
    assert!(!shows(&rt, "No downloads yet"));
    let state = |id: u64| element_text(&rt, &format!("download-state:{id}"));
    assert_eq!(
        state(1),
        [format!(
            "test-0 · Saved to {}",
            crate::app::downloads::tilde(&dir)
        )]
    );
    assert_eq!(state(2), ["test-0 · 12.4 of 80 MB"]);
    assert_eq!(state(3), ["test-0 · Saved 6 bytes; Rex reported 4096"]);
    assert_eq!(state(4), ["test-0 · Cancelled"]);
    assert_eq!(state(5), ["test-0 · Failed: Permission denied"]);
    assert_eq!(
        state(6),
        ["test-0 · Failed: Rex on test-0 is too old to send files; update Rex there."]
    );
    assert_eq!(state(7), ["test-0 · Waiting for test-0…"]);
    // Running rows first, each with Cancel; finished rows have none.
    for (id, cancel) in [
        (2, true),
        (7, true),
        (1, false),
        (3, false),
        (4, false),
        (5, false),
    ] {
        assert_eq!(
            has(&rt, &format!("download-cancel:{id}")),
            cancel,
            "row {id}"
        );
    }
    assert!(has(&rt, "download-progress:2"), "a bar while receiving");
    let ys: Vec<f32> = [7, 2, 6, 5, 4, 3, 1]
        .iter()
        .map(|id| {
            rt.elements()
                .into_iter()
                .find(|e| e.key.as_deref() == Some(&format!("download:{id}")))
                .unwrap()
                .rect[1]
        })
        .collect();
    assert!(
        ys.windows(2).all(|w| w[0] < w[1]),
        "running first, then newest first: {ys:?}"
    );
    // The row's tooltip carries the resolved path; every control is named.
    assert_eq!(
        label_of(&rt, "download-cancel:2"),
        Some(("button", "Cancel download of online".to_owned()))
    );
    assert_eq!(
        label_of(&rt, "download-progress:2").map(|(role, _)| role),
        Some("progressbar")
    );
    assert_eq!(
        label_of(&rt, "downloads-popover").map(|(r, _)| r),
        Some("dialog")
    );
    assert_eq!(
        label_of(&rt, "downloads-list").map(|(r, _)| r),
        Some("list")
    );
    assert_eq!(
        label_of(&rt, "download:1").map(|(r, _)| r),
        Some("listitem")
    );
    assert!(asked(&rx).is_empty());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn cancel_on_a_row_sends_download_cancel_for_that_transfer() {
    let (mut ws, rx, dir) = workspace_13();
    ws.perform(Intent::Downloads, &mut rune::Tx::default());
    let mut rt = runtime(ws);
    feed(&mut rt, |list| {
        list.push(transfer(11, "a", &dir, receiving(1, 10)));
        list.push(transfer(12, "b", &dir, receiving(1, 10)));
    });
    click(&mut rt, "download-cancel:11");
    let sent = asked(&rx);
    assert!(
        matches!(&sent[..], [Asked::Cancel(11)]),
        "{} commands, popover {}",
        sent.len(),
        has(&rt, "downloads-popover")
    );
    // The core finishes the cancel; the row then says so and has no Cancel.
    feed(&mut rt, |list| list[0].state = State::Cancelled);
    assert!(!has(&rt, "download-cancel:11"));
    assert!(has(&rt, "download-cancel:12"));
    assert!(shows(&rt, "Cancelled"));
    assert!(
        rt.state
            .notices
            .iter()
            .all(|n| !n.text.contains("Cancelled"))
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_finished_row_offers_only_show_in_folder_and_nothing_is_shown_without_a_click() {
    let (ws, _rx, dir) = workspace_13();
    let mut rt = runtime(ws);
    // Whatever the file is, a finished row has the one action: no Open for any name or content.
    let files: [(&str, &[u8]); 8] = [
        ("notes.txt", b"plain text"),
        ("Report.PDF", b"%PDF-1.7\n"),
        ("run.sh", b"echo hi\n"),
        ("prog", b"\x7fELF\x02\x01\x01"),
        ("page", b"<!doctype html><script>alert(1)</script>"),
        ("logo.svg", b"plain words under a svg name"),
        ("backup.tar.gz", b"\x1f\x8b\x08\0"),
        ("plain", b"no extension, plain words"),
    ];
    for (name, bytes) in &files {
        std::fs::write(dir.join(name), bytes).unwrap();
    }
    feed(&mut rt, |list| {
        for (i, (name, bytes)) in files.iter().enumerate() {
            list.push(transfer(
                i as u64 + 1,
                name,
                &dir,
                State::Saved {
                    path: dir.join(name),
                    bytes: bytes.len() as u64,
                    short_of: None,
                },
            ));
        }
    });
    // Finishing showed nothing, not even with the popover on screen.
    rt.update(|s, tx| s.perform(Intent::Downloads, tx));
    rt.frame(1. / 60.);
    assert!(
        rt.state.downloads.shown.is_empty(),
        "nothing without a click"
    );
    for (i, (name, _)) in files.iter().enumerate() {
        let id = i + 1;
        assert!(has(&rt, &format!("download-show:{id}")), "{name}");
        assert!(!has(&rt, &format!("download-open:{id}")), "{name}");
        assert_eq!(
            label_of(&rt, &format!("download-show:{id}")),
            Some(("button", format!("Show {name} in its folder")))
        );
    }
    assert!(
        rt.state.downloads.shown.is_empty(),
        "nothing is shown by drawing"
    );

    click(&mut rt, "download-show:3");
    assert_eq!(rt.state.downloads.shown, [dir.join("run.sh")]);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn commands_downloads_opens_the_popover_with_nothing_running() {
    let (mut ws, _rx, dir) = workspace_13();
    ws.picker(false);
    ws.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("downloads");
    let index = ws
        .entries()
        .iter()
        .position(|e| matches!(e.action, Intent::Downloads))
        .expect("Downloads in Commands");
    ws.panel.as_mut().unwrap().selected = index;
    let mut rt = runtime(ws);
    press(&mut rt, Key::Enter);
    assert!(matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::Downloads)
    ));
    assert!(has(&rt, "downloads-popover"));
    assert!(shows(&rt, "No downloads yet"));
    press(&mut rt, Key::Escape);
    assert!(rt.state.panel.is_none());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn the_chip_opens_the_popover_and_a_finishing_transfer_never_moves_focus() {
    let (ws, _rx, dir) = workspace_13();
    let mut rt = runtime(ws);
    feed(&mut rt, |list| {
        list.push(transfer(1, "a.txt", &dir, receiving(1, 2)))
    });
    click(&mut rt, "download-chip");
    assert!(has(&rt, "downloads-popover"));
    let before = focused(&rt);
    assert!(
        before
            .as_deref()
            .is_some_and(|p| p.ends_with("downloads-scroll")),
        "opens on the list, not on a button: {before:?}"
    );
    // Tab to the row's Cancel; when its transfer ends, focus must not slide onto the Show in
    // folder that replaces it, or an Enter meant for Cancel would press it.
    press(&mut rt, Key::Tab);
    assert!(focused(&rt).is_some_and(|p| p.ends_with("download-cancel:1")));
    feed(&mut rt, |list| {
        list[0].state = State::Saved {
            path: dir.join("a.txt"),
            bytes: 2,
            short_of: None,
        }
    });
    assert!(has(&rt, "downloads-popover"), "the popover stays");
    assert!(
        !has(&rt, "download-chip"),
        "the chip goes with the transfer"
    );
    assert!(has(&rt, "download-show:1"));
    let now = focused(&rt);
    assert!(
        now.as_deref().is_none_or(|p| !p.contains("download-show")),
        "focus is on {now:?}"
    );
    press(&mut rt, Key::Enter);
    assert!(rt.state.downloads.shown.is_empty(), "Enter showed nothing");

    // The same with a dialog open: Rename keeps its field and its text.
    press(&mut rt, Key::Escape);
    rt.update(|s, tx| s.perform(Intent::RenameSession, tx));
    rt.frame(1. / 60.);
    let before = focused(&rt);
    assert!(before.as_deref().is_some_and(|p| p.contains("field:0")));
    feed(&mut rt, |list| {
        list.push(transfer(2, "b.txt", &dir, receiving(1, 2)));
    });
    feed(&mut rt, |list| {
        list[1].state = State::Failed(Failure::Stalled)
    });
    assert_eq!(focused(&rt), before);
    assert!(matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::Form(Form::RenameSession(None)))
    ));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn change_edits_the_folder_in_the_dialog_and_saves_downloads_directory() {
    let (mut ws, rx, dir) = workspace_13();
    pane_menu_download(&mut ws, "block:b");
    let mut rt = open_dialog(ws);
    click(&mut rt, "change-folder");
    let path = focused(&rt).expect("the folder field has focus");
    assert!(path.contains("field:1"), "{path}");
    // Escape leaves the field only; the dialog and the folder stay.
    press(&mut rt, Key::Escape);
    assert!(rt.state.panel.is_some());
    assert!(!has(&rt, "field:1"));
    assert_eq!(
        rt.state.settings.downloads.directory.as_deref(),
        Some(dir.display().to_string().as_str())
    );
    click(&mut rt, "change-folder");
    let other = scratch();
    set_field(&mut rt, 1, &other.display().to_string());
    press(&mut rt, Key::Enter);
    assert!(
        rt.state.panel.is_some(),
        "Enter in the folder field keeps the dialog"
    );
    assert_eq!(
        rt.state.settings.downloads.directory.as_deref(),
        Some(other.display().to_string().as_str())
    );
    let saved = rex_linux::settings::Settings::load(&rt.state.settings_path).unwrap();
    assert_eq!(
        saved.downloads.directory.as_deref(),
        Some(other.display().to_string().as_str()),
        "written to workspace.json"
    );
    assert!(shows(&rt, &other.display().to_string()));
    // The next download goes to it.
    set_field(&mut rt, 0, "/x");
    press(&mut rt, Key::Enter);
    assert!(matches!(&asked(&rx)[..], [Asked::Download(r)] if r.directory == other));

    // A relative folder is refused; an empty one goes back to the system's folder.
    let mut rt2 = {
        let (mut ws, _rx, d) = workspace_13();
        pane_menu_download(&mut ws, "block:b");
        std::fs::remove_dir_all(d).ok();
        open_dialog(ws)
    };
    click(&mut rt2, "change-folder");
    set_field(&mut rt2, 1, "relative/dir");
    press(&mut rt2, Key::Enter);
    assert!(shows(&rt2, "absolute folder"));
    set_field(&mut rt2, 1, "");
    press(&mut rt2, Key::Enter);
    assert_eq!(rt2.state.settings.downloads.directory, None);
    std::fs::remove_dir_all(dir).ok();
    std::fs::remove_dir_all(other).ok();
}

#[test]
fn the_downloads_directory_survives_a_relaunch() {
    let (mut ws, _rx, dir) = workspace_13();
    ws.save();
    let again = relaunch(&ws);
    assert_eq!(
        again.settings.downloads.directory.as_deref(),
        Some(dir.display().to_string().as_str())
    );
    assert_eq!(again.download_folder(), dir);
    std::fs::remove_file(&ws.settings_path).ok();
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn sizes_read_as_the_dialog_and_popover_write_them() {
    use crate::app::downloads::progress_text;
    use rex_linux::download::size_text;
    assert_eq!(size_text(0), "0 bytes");
    assert_eq!(size_text(1), "1 byte");
    assert_eq!(size_text(999), "999 bytes");
    assert_eq!(size_text(1_000), "1 kB");
    assert_eq!(size_text(3_200_000_000), "3.2 GB");
    assert_eq!(size_text(41_000_000_000), "41 GB");
    assert_eq!(size_text(41_370_000_000), "41.4 GB");
    assert_eq!(size_text(80_000_000), "80 MB");
    assert_eq!(size_text(250_000_000), "250 MB");
    assert_eq!(size_text(999_990_000), "1 GB");
    assert_eq!(progress_text(12_400_000, 80_000_000), "12.4 of 80 MB");
    assert_eq!(progress_text(0, 3_000_000_000), "0 of 3 GB");
    assert_eq!(progress_text(5_000_000, 0), "5 MB");
    assert_eq!(progress_text(300, 900), "300 of 900 bytes");
}

/// The right edge of the element with `key`, and of the dialog.
fn right_edges(rt: &Runtime<Workspace>, key: &str) -> (f32, f32) {
    let rect_of = |wanted: &str| {
        rt.elements()
            .into_iter()
            .find(|e| !e.exiting && e.key.as_deref() == Some(wanted))
            .unwrap_or_else(|| panic!("missing {wanted}"))
            .rect
    };
    let (one, dialog) = (rect_of(key), rect_of("dialog"));
    (one[0] + one[2], dialog[0] + dialog[2])
}

#[test]
fn text_with_no_break_opportunity_wraps_inside_the_dialog_instead_of_running_through_it() {
    // The longest name the library allows, a path and an error that echo it, with no space or
    // hyphen to break at.
    let long = "L".repeat(250);
    let (mut ws, rx, dir) = workspace_13();
    pane_menu_download(&mut ws, "block:b");
    let mut rt = open_dialog(ws);
    set_field(&mut rt, 0, &format!("~/{long}"));
    press(&mut rt, Key::Enter);
    let id = match &asked(&rx)[..] {
        [Asked::Download(r)] => r.id,
        _ => panic!("one download"),
    };
    let mut waiting = transfer(id, &long, &dir, State::Requested);
    waiting.name = None;
    feed(&mut rt, |list| list.push(waiting));
    let (text, dialog) = right_edges(&rt, "download-waiting");
    assert!(
        text <= dialog,
        "waiting line ends at {text}, dialog at {dialog}"
    );

    let mut t = transfer(
        id,
        &long,
        &dir,
        State::AwaitingConfirmation(Confirmation {
            reason: Reason::Large,
            hint: 2_147_483_648,
            free: Some(41_000_000_000),
        }),
    );
    t.resolved = Some(format!("/home/dev/{long}"));
    feed(&mut rt, |list| list[0] = t);
    for key in ["download-question", "download-resolved"] {
        let (text, dialog) = right_edges(&rt, key);
        assert!(text <= dialog, "{key} ends at {text}, dialog at {dialog}");
    }
    // Wrapped, and not cut to a stub: the name is still there to read before accepting it.
    let question = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("download-question"))
        .unwrap();
    assert!(
        question.rect[3] > 60.,
        "wraps onto several lines: {:?}",
        question.rect
    );
    assert!(
        question.rect[3] < 120.,
        "but is bounded: {:?}",
        question.rect
    );

    // Rex's error echoing a long path, back in the form.
    press(&mut rt, Key::Escape);
    let (mut ws, rx, dir2) = workspace_13();
    pane_menu_download(&mut ws, "block:b");
    let mut rt = open_dialog(ws);
    set_field(&mut rt, 0, &format!("~/{long}"));
    press(&mut rt, Key::Enter);
    let id = match &asked(&rx)[..] {
        [Asked::Download(r)] => r.id,
        _ => panic!("one download"),
    };
    let message = format!("not found: /home/dev/{long} does not exist");
    feed(&mut rt, |list| {
        list.push(transfer(
            id,
            &long,
            &dir2,
            State::Failed(Failure::Server(message)),
        ))
    });
    let (text, dialog) = right_edges(&rt, "download-error");
    assert!(text <= dialog, "error ends at {text}, dialog at {dialog}");
    let error = rect_of(&rt, "download-error").unwrap();
    assert!(error[3] > 45., "the error wraps too: {error:?}");
    std::fs::remove_dir_all(dir).ok();
    std::fs::remove_dir_all(dir2).ok();
}

#[test]
fn a_connection_failure_reads_as_a_sentence_not_the_websocket_stacks_words() {
    let (ws, _rx, dir) = workspace_13();
    let mut rt = runtime(ws);
    feed(&mut rt, |list| {
        list.push(transfer(
            1,
            "img-a.iso",
            &dir,
            State::Failed(Failure::Connection(
                "WebSocket protocol error: Connection reset without closing handshake".into(),
            )),
        ))
    });
    let notice = rt
        .state
        .notices
        .iter()
        .find(|n| n.sticky)
        .expect("a notice");
    assert_eq!(
        notice.text,
        "Download of img-a.iso from test-0 failed: the connection to test-0 was lost"
    );
    assert!(!notice.text.contains("WebSocket"));
    std::fs::remove_dir_all(dir).ok();
}

/// A session with eight tabs, the last active, in a small window.
fn crowded_bar(width: f32) -> (Runtime<Workspace>, PathBuf) {
    let (mut ws, _rx, dir) = workspace_13();
    let windows: Vec<_> = (0..8)
        .map(|i| {
            serde_json::json!({
                "window_id": format!("t{i}"), "label": format!("tab-number-{i}"),
                "focused_block_id": format!("block:{i}"),
                "layers": [{"kind": "tiled", "layout": {"block_id": format!("block:{i}")},
                            "blocks": [{"block_id": format!("block:{i}")}]}]
            })
        })
        .collect();
    ws.hosts[0].model.views.insert(
        "s".into(),
        serde_json::from_value(serde_json::json!({
            "revision": 1, "session_id": "s", "active_window_id": "t7", "windows": windows
        }))
        .expect("a view"),
    );
    (runtime_at(ws, (width, 500.)), dir)
}

fn rect_of(rt: &Runtime<Workspace>, key: &str) -> Option<[f32; 4]> {
    rt.elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some(key))
        .map(|e| e.rect)
}

/// Run the app's poll (which scrolls the active tab into view) and let it settle.
fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..6 {
        rt.update(|s, tx| {
            crate::app::poll(s, 0., tx);
        });
        for _ in 0..30 {
            rt.frame(1. / 60.);
        }
    }
}

fn active_tab_is_visible(rt: &Runtime<Workspace>) -> bool {
    let (strip, tab) = (rect_of(rt, "tabs").unwrap(), rect_of(rt, "tab:t7").unwrap());
    tab[0] + 40. <= strip[0] + strip[2] && tab[0] + tab[2] >= strip[0] + 40.
}

#[test]
fn the_chip_shortens_in_a_narrow_window_and_the_active_tab_stays_in_view() {
    let (mut rt, dir) = crowded_bar(700.);
    settle(&mut rt);
    assert!(active_tab_is_visible(&rt), "without a chip");
    feed(&mut rt, |list| {
        list.push(transfer(1, "img-a.iso", &dir, receiving(30, 600)))
    });
    settle(&mut rt);
    let chip = rect_of(&rt, "download-chip").unwrap();
    assert!(chip[2] < 80., "short in 700 pt: {chip:?}");
    assert!(
        shows(&rt, "5%") && !shows(&rt, "img-a.iso"),
        "{:?}",
        texts(&rt)
    );
    assert_eq!(
        label_of(&rt, "download-chip"),
        Some(("button", "Downloads: img-a.iso 5%".to_owned())),
        "the full text is still its accessible name"
    );
    assert!(
        active_tab_is_visible(&rt),
        "with a chip: {:?}",
        rect_of(&rt, "tabs")
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn the_chip_keeps_its_name_in_a_wide_window() {
    let (ws, _rx, dir) = workspace_13();
    let mut rt = runtime_at(ws, (1440., 900.));
    feed(&mut rt, |list| {
        list.push(transfer(1, "img-a.iso", &dir, receiving(30, 600)))
    });
    assert!(shows(&rt, "↓ img-a.iso"), "{:?}", texts(&rt));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn the_popover_opens_under_the_chip_and_from_commands_at_the_bars_right_end() {
    let (ws, _rx, dir) = workspace_13();
    let mut rt = runtime_at(ws, (1440., 900.));
    feed(&mut rt, |list| {
        list.push(transfer(1, "img-a.iso", &dir, receiving(30, 600)))
    });
    let chip = rect_of(&rt, "download-chip").unwrap();
    click(&mut rt, "download-chip");
    for _ in 0..40 {
        rt.frame(1. / 60.);
    }
    let popover = rect_of(&rt, "downloads-popover").expect("the popover");
    assert!(
        (popover[0] - chip[0]).abs() < 1.5,
        "popover at {popover:?}, chip at {chip:?}"
    );
    assert!(popover[1] <= 60., "just under the bar: {popover:?}");
    press(&mut rt, Key::Escape);
    rt.update(|s, tx| s.perform(Intent::Downloads, tx));
    for _ in 0..40 {
        rt.frame(1. / 60.);
    }
    let popover = rect_of(&rt, "downloads-popover").expect("the popover");
    assert!(
        popover[0] > 800.,
        "from Commands, at the right: {popover:?}"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_long_name_in_a_finished_row_is_cut_by_width_inside_the_row() {
    // What is drawn (the middle cut that keeps the extension) is Rune's `truncate_middle`, which
    // the headless element list reports whole; the render shows the extension. Here: the name's
    // box never pushes the buttons out of the popover.
    let (ws, _rx, dir) = workspace_13();
    let mut rt = runtime_at(ws, (1440., 900.));
    for name in [
        "quarterly-report-final-FINAL-v2-reviewed-quarterly-report-final-2026.tar.gz",
        "ünï-cödé-ファイル-日本語-ファイル-日本語-ファイル-日本語-ファイル-日本語-final.iso",
    ] {
        std::fs::write(dir.join("shown.txt"), b"x").unwrap();
        let done = transfer(
            1,
            name,
            &dir,
            State::Saved {
                path: dir.join("shown.txt"),
                bytes: 1,
                short_of: None,
            },
        );
        feed(&mut rt, |list| {
            list.clear();
            list.push(done)
        });
        rt.update(|s, tx| s.perform(Intent::Downloads, tx));
        for _ in 0..40 {
            rt.frame(1. / 60.);
        }
        let show = rect_of(&rt, "download-show:1").expect("Show in folder");
        let popover = rect_of(&rt, "downloads-popover").unwrap();
        let name_box = rect_of(&rt, "download-name:1").unwrap();
        assert!(
            show[0] + show[2] <= popover[0] + popover[2],
            "{show:?} {popover:?}"
        );
        assert!(
            name_box[0] + name_box[2] <= show[0],
            "{name_box:?} ends before {show:?}"
        );
        press(&mut rt, Key::Escape);
    }
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_long_error_with_no_spaces_wraps_inside_a_failed_row_and_the_popover() {
    let (ws, _rx, dir) = workspace_13();
    let mut rt = runtime_at(ws, (1440., 900.));
    let unbroken = "E".repeat(300);
    feed(&mut rt, |list| {
        list.push(transfer(
            1,
            "report.txt",
            &dir,
            State::Failed(Failure::Server(unbroken)),
        ))
    });
    rt.update(|s, tx| s.perform(Intent::Downloads, tx));
    for _ in 0..40 {
        rt.frame(1. / 60.);
    }
    let popover = rect_of(&rt, "downloads-popover").unwrap();
    let state = rect_of(&rt, "download-state:1").expect("the state line");
    let row = rect_of(&rt, "download:1").unwrap();
    assert!(
        state[0] + state[2] <= popover[0] + popover[2] + 0.5,
        "state {state:?} runs past the popover {popover:?}"
    );
    assert!(
        row[0] + row[2] <= popover[0] + popover[2] + 0.5,
        "row {row:?} runs past the popover {popover:?}"
    );
    // The text breaks anywhere to fit (an unbroken word left to run on would sit on one line
    // beside the "Failed:" and out of the box), and a line cap keeps one bad error from taking
    // the list.
    assert!(
        (3. * 14.0..=4. * 16.).contains(&state[3]),
        "state is {} pt tall",
        state[3]
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_chip_coming_into_a_wide_enough_bar_scrolls_the_active_tab_back_into_view() {
    let (mut rt, dir) = crowded_bar(1010.);
    settle(&mut rt);
    assert!(active_tab_is_visible(&rt), "without a chip");
    feed(&mut rt, |list| {
        list.push(transfer(
            1,
            "a-fairly-long-file-name.tar.gz",
            &dir,
            receiving(30, 600),
        ))
    });
    settle(&mut rt);
    let chip = rect_of(&rt, "download-chip").unwrap();
    assert!(chip[2] > 150., "the full chip at 1010 pt: {chip:?}");
    assert!(
        active_tab_is_visible(&rt),
        "with the chip: tab {:?} in strip {:?}",
        rect_of(&rt, "tab:t7"),
        rect_of(&rt, "tabs")
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn folders_are_written_with_a_tilde_and_cut_in_the_middle_by_whole_folders() {
    use crate::app::downloads::{folder_text_in, tilde_in};
    let home = Path::new("/home/dev");
    assert_eq!(tilde_in(Path::new("/home/dev"), Some(home)), "~");
    assert_eq!(
        tilde_in(Path::new("/home/dev/Downloads"), Some(home)),
        "~/Downloads"
    );
    assert_eq!(
        tilde_in(Path::new("/home/devious/x"), Some(home)),
        "/home/devious/x",
        "a folder that only starts with the same letters is not under home"
    );
    assert_eq!(tilde_in(Path::new("/home/dev/x"), None), "/home/dev/x");
    // Short enough: as it is.
    assert_eq!(
        folder_text_in(Path::new("/home/dev/Downloads"), Some(home), 36),
        "~/Downloads"
    );
    // Too long: the start (`~`), then `…`, then the folders that fit, the end kept whole.
    let deep = Path::new("/home/dev/projects/client-work/2026/quarterly-reports/final/scans");
    let text = folder_text_in(deep, Some(home), 36);
    assert_eq!(text, "~/…/quarterly-reports/final/scans");
    assert!(text.chars().count() <= 36);
    // Outside home the first folder stays.
    let text = folder_text_in(
        Path::new("/mnt/storage/archive/2026/november/reports"),
        Some(home),
        28,
    );
    assert_eq!(text, "/mnt/…/2026/november/reports");
    // One folder too long to fit at all is cut in the middle by characters.
    let long = format!("/home/dev/{}", "x".repeat(80));
    let text = folder_text_in(Path::new(&long), Some(home), 36);
    assert_eq!(text.chars().count(), 36, "{text}");
    assert!(text.contains('…') && text.starts_with("~/x") && text.ends_with('x'));
}

#[test]
fn the_low_space_question_says_what_saving_leaves_and_large_keeps_its_words() {
    let ask = |reason, hint, free| {
        crate::app::downloads::question(
            "build.img",
            "~/Downloads",
            &Confirmation { reason, hint, free },
        )
    };
    assert_eq!(
        ask(Reason::Large, 3_200_000_000, Some(41_000_000_000)),
        "build.img is 3.2 GB. ~/Downloads has 41 GB free."
    );
    assert_eq!(
        ask(Reason::LowSpace, 300_000_000, Some(400_000_000)),
        "build.img is 300 MB. ~/Downloads has 400 MB free, so saving it leaves about 100 MB."
    );
    assert_eq!(
        ask(Reason::LowSpace, 300_000_000, None),
        "build.img is 300 MB.",
        "unknown free space: only the size"
    );
    assert_eq!(
        ask(Reason::Large, 5_000_000_000, None),
        "build.img is 5 GB."
    );
}

#[test]
fn a_low_space_question_shows_in_the_dialog_with_cancel_focused() {
    let (mut ws, rx, dir) = workspace_13();
    pane_menu_download(&mut ws, "block:b");
    let mut rt = open_dialog(ws);
    set_field(&mut rt, 0, "/home/dev/build.img");
    press(&mut rt, Key::Enter);
    let id = match &asked(&rx)[..] {
        [Asked::Download(r)] => r.id,
        _ => panic!("one download"),
    };
    feed(&mut rt, |list| {
        list.push(transfer(
            id,
            "build.img",
            &dir,
            State::AwaitingConfirmation(Confirmation {
                reason: Reason::LowSpace,
                hint: 300_000_000,
                free: Some(400_000_000),
            }),
        ))
    });
    assert!(
        shows(&rt, "build.img is 300 MB. ")
            && shows(&rt, "has 400 MB free, so saving it leaves about 100 MB."),
        "{:?}",
        texts(&rt)
    );
    assert!(
        focused(&rt).is_some_and(|p| p.ends_with("cancel-download")),
        "Cancel has focus"
    );
    assert!(asked(&rx).is_empty());
    std::fs::remove_dir_all(dir).ok();
}

/// A folder far deeper than any line, inside a new scratch folder.
fn deep_folder() -> (PathBuf, PathBuf) {
    let root = scratch();
    let deep = root
        .join("projects")
        .join("client-work-with-a-long-name")
        .join("2026")
        .join("quarterly-reports")
        .join("final-scans");
    std::fs::create_dir_all(&deep).unwrap();
    (root, deep)
}

#[test]
fn a_deep_folder_is_cut_in_the_middle_in_the_save_in_line_and_in_a_saved_row() {
    let (root, deep) = deep_folder();
    let (mut ws, rx, dir) = workspace_13();
    ws.settings.downloads.directory = Some(deep.display().to_string());
    pane_menu_download(&mut ws, "block:b");
    let mut rt = open_dialog(ws);
    let line = element_text(&rt, "save-in-folder");
    let line = line.concat();
    assert!(
        line.contains('…') && line.ends_with("final-scans") && line.chars().count() <= 36,
        "Save in line: {line}"
    );
    assert!(
        !shows(&rt, &deep.display().to_string()),
        "the full path is not a line of the dialog"
    );
    assert_eq!(
        label_of(&rt, "save-in").map(|(_, label)| label),
        Some(format!("Save in {}", deep.display())),
        "the accessible name keeps all of it"
    );
    press(&mut rt, Key::Escape);
    asked(&rx);

    let (mut ws, _rx, _dir) = workspace_13();
    ws.perform(Intent::Downloads, &mut rune::Tx::default());
    let mut rt = runtime(ws);
    feed(&mut rt, |list| {
        list.push(transfer(
            1,
            "report.pdf",
            &deep,
            State::Saved {
                path: deep.join("report.pdf"),
                bytes: 10,
                short_of: None,
            },
        ))
    });
    let state = element_text(&rt, "download-state:1").concat();
    assert!(
        state.starts_with("test-0 · Saved to /")
            && state.contains('…')
            && state.ends_with("final-scans"),
        "{state}"
    );
    let row = rect_of(&rt, "download-state:1").unwrap();
    assert!(
        row[3] < 40.,
        "the state line stays one or two lines: {row:?}"
    );
    std::fs::remove_dir_all(root).ok();
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn the_launch_cleanup_deletes_a_recorded_partial_of_a_dead_process_and_nothing_else() {
    use rex_linux::download::store::Records;
    let dir = scratch();
    let state = scratch();
    // A process that has ended: its id is no longer a running process's.
    let mut child = std::process::Command::new("true").spawn().unwrap();
    let dead = child.id();
    child.wait().unwrap();
    let leftover = ".a.iso.11111111-2222-3333-4444-555555555555.rex-part";
    let other = ".b.iso.11111111-2222-3333-4444-555555555556.rex-part";
    let unrecorded = "notes.txt";
    for name in [leftover, other, unrecorded] {
        std::fs::write(dir.join(name), b"x").unwrap();
    }
    let records = state.join("downloads.json");
    // This process's own start time, as the core reads it, so its record says it is running.
    let stat = std::fs::read_to_string("/proc/self/stat").unwrap();
    let started: u64 = stat[stat.rfind(')').unwrap() + 1..]
        .split_whitespace()
        .nth(19)
        .unwrap()
        .parse()
        .unwrap();
    let record = |name: &str, pid: u32, started: u64| serde_json::json!({"dir": dir, "name": name, "pid": pid, "started": started, "created": 1});
    // `other` belongs to this very process, which is running: it must stay.
    std::fs::write(
        &records,
        serde_json::json!({"partials": [record(leftover, dead, 1), record(other, std::process::id(), started)]})
            .to_string(),
    )
    .unwrap();
    let worker = Workspace::clean_up_leftovers_in(Records::at(records.clone()))
        .expect("the cleanup starts a worker");
    worker.join().unwrap();
    assert!(
        !dir.join(leftover).exists(),
        "the dead process's partial is gone"
    );
    assert!(
        dir.join(other).exists(),
        "a running process's partial is kept"
    );
    assert!(
        dir.join(unrecorded).exists(),
        "nothing unrecorded is touched"
    );
    let left = Records::at(records).list();
    assert_eq!(
        left.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        [other],
        "only the dead process's record is forgotten"
    );
    std::fs::remove_dir_all(dir).ok();
    std::fs::remove_dir_all(state).ok();
}

/// Change the host's model the way its worker does (views, connection), then deliver the wake.
fn change_model(
    rt: &mut Runtime<Workspace>,
    index: usize,
    f: impl FnOnce(&mut rex_linux::connection::HostModel),
) {
    rt.update(|s, tx| {
        let mut model = s.hosts[index].model.clone();
        f(&mut model);
        *s.hosts[index].connection.model.lock().unwrap() = model;
        s.sync(index, tx);
    });
    for _ in 0..10 {
        rt.frame(1. / 60.);
    }
}

/// What the dialog's transfer looks like to the client when the host's pane or connection goes.
#[derive(Clone, Copy, Debug)]
enum Waiting {
    /// Asked, and the worker has not listed it yet.
    Unlisted,
    /// Listed, Rex has not answered.
    ForRex,
    /// Rex answered with the large-file question.
    ForAnswer,
}

/// Open the dialog for `block:b`, press Enter on a path and put the transfer in `waiting`.
fn dialog_waiting(
    waiting: Waiting,
) -> (
    Runtime<Workspace>,
    flume::Receiver<HostCommand>,
    PathBuf,
    u64,
) {
    let (mut ws, rx, dir) = workspace_13();
    pane_menu_download(&mut ws, "block:b");
    let mut rt = open_dialog(ws);
    set_field(&mut rt, 0, "/home/dev/build.img");
    press(&mut rt, Key::Enter);
    let id = match &asked(&rx)[..] {
        [Asked::Download(r)] => r.id,
        _ => panic!("one download"),
    };
    let state = match waiting {
        Waiting::Unlisted => None,
        Waiting::ForRex => Some(State::Requested),
        Waiting::ForAnswer => Some(State::AwaitingConfirmation(Confirmation {
            reason: Reason::Large,
            hint: 3_200_000_000,
            free: Some(41_000_000_000),
        })),
    };
    if let Some(state) = state {
        feed(&mut rt, |list| {
            list.push(transfer(id, "build.img", &dir, state))
        });
    }
    assert!(rt.state.panel.is_some(), "{waiting:?}: the dialog is open");
    assert!(asked(&rx).is_empty());
    (rt, rx, dir, id)
}

#[test]
fn the_pane_closing_under_a_waiting_dialog_closes_it_and_cancels_once() {
    for waiting in [Waiting::Unlisted, Waiting::ForRex, Waiting::ForAnswer] {
        let (mut rt, rx, dir, id) = dialog_waiting(waiting);
        change_model(&mut rt, 0, |model| {
            model.views.get_mut("s").unwrap().windows[1].layers[0]
                .blocks
                .retain(|b| b.block_id.as_deref() != Some("block:b"));
        });
        assert!(rt.state.panel.is_none(), "{waiting:?}: the dialog closes");
        assert!(
            rt.state
                .notices
                .iter()
                .any(|n| n.text.contains("no longer available"))
        );
        let sent = asked(&rx);
        assert!(
            matches!(&sent[..], [Asked::Cancel(c)] if *c == id),
            "{waiting:?}: {} commands, want one cancel",
            sent.len()
        );
        // Later wakes do not ask again.
        change_model(&mut rt, 0, |_| {});
        assert!(asked(&rx).is_empty(), "{waiting:?}: not asked twice");
        std::fs::remove_dir_all(dir).ok();
    }
}

#[test]
fn the_host_disconnecting_under_a_waiting_dialog_closes_it_and_cancels_once() {
    for waiting in [Waiting::Unlisted, Waiting::ForRex, Waiting::ForAnswer] {
        let (mut rt, rx, dir, id) = dialog_waiting(waiting);
        // A reconnect: the connection drops, and the new one starts with no views.
        change_model(&mut rt, 0, |model| {
            model.connected = false;
            model.connection_epoch += 1;
            model.views.clear();
        });
        assert!(rt.state.panel.is_none(), "{waiting:?}: the dialog closes");
        let sent = asked(&rx);
        assert!(
            matches!(&sent[..], [Asked::Cancel(c)] if *c == id),
            "{waiting:?}: {} commands, want one cancel",
            sent.len()
        );
        change_model(&mut rt, 0, |_| {});
        assert!(asked(&rx).is_empty(), "{waiting:?}: not asked twice");
        std::fs::remove_dir_all(dir).ok();
    }
}

#[test]
fn rows_from_two_hosts_each_name_their_host_and_cancel_goes_to_that_host() {
    let mut ws = workspace_hosts(2);
    let first = capture_host(&mut ws, 0);
    let second = capture_host(&mut ws, 1);
    let dir = scratch();
    ws.settings.downloads.directory = Some(dir.display().to_string());
    for host in &mut ws.hosts {
        host.model.connected = true;
        host.model.protocol.api_revision = 13;
    }
    ws.perform(Intent::Downloads, &mut rune::Tx::default());
    let mut rt = runtime(ws);
    change_model(&mut rt, 0, |m| {
        m.downloads
            .push(transfer(21, "one.log", &dir, receiving(1, 10)))
    });
    change_model(&mut rt, 1, |m| {
        m.downloads
            .push(transfer(22, "two.log", &dir, receiving(2, 10)));
        m.downloads.push(transfer(
            23,
            "three.log",
            &dir,
            State::Failed(Failure::Stalled),
        ));
    });
    let (_, one) = label_of(&rt, "download:21").expect("a row for host 0");
    let (_, two) = label_of(&rt, "download:22").expect("a row for host 1");
    let (_, three) = label_of(&rt, "download:23").expect("a failed row for host 1");
    assert!(one.contains("one.log") && one.contains("test-0"), "{one}");
    assert!(two.contains("two.log") && two.contains("test-1"), "{two}");
    assert!(
        three.contains("test-1") && !three.contains("test-0"),
        "{three}"
    );
    assert!(shows(&rt, "test-0 · 1 of 10 bytes"), "{:?}", texts(&rt));
    assert!(shows(&rt, "test-1 · 2 of 10 bytes"), "{:?}", texts(&rt));

    click(&mut rt, "download-cancel:22");
    let to_second = asked(&second);
    assert!(
        matches!(&to_second[..], [Asked::Cancel(22)]),
        "{} commands to host 1",
        to_second.len()
    );
    assert!(asked(&first).is_empty(), "host 0 is not told");

    click(&mut rt, "download-cancel:21");
    let to_first = asked(&first);
    assert!(
        matches!(&to_first[..], [Asked::Cancel(21)]),
        "{} commands to host 0",
        to_first.len()
    );
    assert!(asked(&second).is_empty(), "host 1 is not told");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn show_in_folder_gets_the_absolute_saved_path_and_the_file_manager_uri_is_encoded() {
    let (ws, _rx, dir) = workspace_13();
    let mut rt = runtime(ws);
    let names = [
        "-rf %41 #x?.txt",
        "two  spaces & 100%.txt",
        "-dash.txt",
        "a#b?c=d%2Fe.md",
        "ünï ファイル.txt",
    ];
    for (i, name) in names.iter().enumerate() {
        std::fs::write(dir.join(name), b"plain words").unwrap();
        let saved = State::Saved {
            path: dir.join(name),
            bytes: 11,
            short_of: None,
        };
        feed(&mut rt, |list| {
            list.push(transfer(i as u64 + 1, name, &dir, saved))
        });
    }
    rt.update(|s, tx| s.perform(Intent::Downloads, tx));
    rt.frame(1. / 60.);
    for (i, name) in names.iter().enumerate() {
        click(&mut rt, &format!("download-show:{}", i + 1));
        let want = dir.join(name);
        let shown = std::mem::take(&mut rt.state.downloads.shown);
        assert_eq!(shown, std::slice::from_ref(&want), "{name}");
        // The fallback opens the folder with one argument, the absolute path itself, never
        // relative and never split or quoted (`shown` only records that the click got that far).
        assert!(want.is_absolute());
        assert_eq!(want.file_name().unwrap(), std::ffi::OsStr::new(name));
        // What the real ShowItems call sends: a file: URI that names this path alone.
        let uri = url::Url::from_file_path(&want).unwrap().to_string();
        assert!(uri.starts_with("file:///"), "{uri}");
        assert!(!uri.contains(['#', '?', ' ', '\n']), "{uri}");
        let back = url::Url::parse(&uri).unwrap().to_file_path().unwrap();
        assert_eq!(back, want, "{uri}");
        if name.contains('%') {
            assert!(uri.contains("%25"), "a percent sign is encoded: {uri}");
        }
        if name.contains('#') {
            assert!(uri.contains("%23"), "{uri}");
        }
        if name.contains('?') {
            assert!(uri.contains("%3F"), "{uri}");
        }
        if name.contains(' ') {
            assert!(uri.contains("%20"), "{uri}");
        }
    }
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn xdg_open_gets_one_argument_the_absolute_path() {
    for target in [
        "/home/u/Downloads",
        "/home/u/Downloads/-rf %41 #x?",
        "/home/u/Downloads/two  spaces & 100%",
        "/home/u/Downloads/\u{fc}n\u{ef} \u{30d5}",
        "/",
    ] {
        let command = crate::app::downloads::xdg_open_command(Path::new(target)).unwrap();
        assert_eq!(command.get_program(), "xdg-open");
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args, [std::ffi::OsStr::new(target)], "{target}");
        assert_eq!(
            command.get_envs().count(),
            0,
            "the environment is inherited as is"
        );
    }
    // A relative path could be read as an option or an URL.
    for target in ["notes", "-n", "--help", "./a", "file:///etc/passwd", ""] {
        assert!(
            crate::app::downloads::xdg_open_command(Path::new(target)).is_none(),
            "{target}"
        );
    }
}

#[test]
fn settings_interface_holds_the_download_folder_and_its_form_checks_like_change() {
    let (mut ws, _rx, dir) = workspace_13();
    let mut tx = rune::Tx::default();
    ws.perform(Intent::Settings, &mut tx);
    ws.open_section(crate::app::panel::SettingsSection::Interface);
    let mut rt = runtime(ws);
    for _ in 0..30 {
        rt.frame(1. / 60.);
    }
    // The row shows where files go now, and opens a form from Settings.
    assert!(
        has(&rt, "download-folder"),
        "a Download folder row on the Interface page"
    );
    rt.update(|_, tx| tx.scroll_into_view("download-folder"));
    settle(&mut rt);
    click(&mut rt, "download-folder");
    let panel = rt.state.panel.as_ref().expect("the form");
    assert!(matches!(
        panel.kind,
        PanelKind::Form(crate::app::panel::Form::DownloadFolder)
    ));
    assert_eq!(panel.inputs[0].text, dir.display().to_string(), "prefilled");
    // The same check as Change in the dialog.
    set_field(&mut rt, 0, "relative/dir");
    press(&mut rt, Key::Enter);
    assert!(shows(&rt, "absolute folder"));
    assert_eq!(
        rt.state.settings.downloads.directory.as_deref(),
        Some(dir.display().to_string().as_str()),
        "nothing changed"
    );
    let other = scratch();
    set_field(&mut rt, 0, &format!("  {}  ", other.display()));
    press(&mut rt, Key::Enter);
    assert_eq!(
        rt.state.settings.downloads.directory.as_deref(),
        Some(other.display().to_string().as_str())
    );
    assert!(
        matches!(
            rt.state.panel.as_ref().map(|p| &p.kind),
            Some(PanelKind::Settings)
        ),
        "back in Settings"
    );
    assert_eq!(rt.state.download_folder(), other);
    // Escape leaves it as it was; an empty folder goes back to the system's.
    rt.update(|_, tx| tx.scroll_into_view("download-folder"));
    settle(&mut rt);
    click(&mut rt, "download-folder");
    set_field(&mut rt, 0, "/never/saved");
    press(&mut rt, Key::Escape);
    assert_eq!(
        rt.state.settings.downloads.directory.as_deref(),
        Some(other.display().to_string().as_str())
    );
    rt.update(|_, tx| tx.scroll_into_view("download-folder"));
    settle(&mut rt);
    click(&mut rt, "download-folder");
    set_field(&mut rt, 0, "");
    press(&mut rt, Key::Enter);
    assert_eq!(rt.state.settings.downloads.directory, None);
    std::fs::remove_file(&rt.state.settings_path).ok();
    std::fs::remove_dir_all(dir).ok();
    std::fs::remove_dir_all(other).ok();
}

#[test]
fn a_binding_for_pane_download_is_the_hint_in_the_pane_menu_and_commands() {
    let (mut ws, _rx, dir) = workspace_13();
    ws.settings.shortcut_file.overrides = rex_linux::keymap::Overrides::parse(
        &serde_json::json!([{"keys":"ctrl+alt+d","action":"pane.download"}]),
    );
    ws.local_keymap = actions::resolve(&ws.settings.shortcut_file.overrides);
    ws.keymap_generation += 1;
    ws.pane_menu("block:b".into(), None);
    let hint = |ws: &Workspace| {
        ws.entries()
            .into_iter()
            .find(|e| e.label == "Download file…")
            .map(|e| e.detail)
    };
    assert_eq!(hint(&ws).as_deref(), Some("Ctrl Alt D"), "pane menu");
    ws.cancel_panel_quiet();
    ws.picker(false);
    assert_eq!(hint(&ws).as_deref(), Some("Ctrl Alt D"), "Commands");
    std::fs::remove_dir_all(dir).ok();
}
