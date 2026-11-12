//! Read-only browsing, captured targets, shell quoting and keyboard behavior.
use super::pane_verb_tests::{Camera, framed_terminal};
use super::*;
use crate::app::{
    actions,
    directory::{self, ACTION},
};
use rex_rune::connection::{
    Command,
    directory::{Failure, Reply, Request},
};
use rexrs::api::terminal::{Entry, ListDirResponse};
use std::sync::Arc;

fn response() -> ListDirResponse {
    ListDirResponse {
        path: "/home/dev".into(),
        home: "/home/dev".into(),
        status: "ok".into(),
        truncated: false,
        entries: [
            ("z.txt", true),
            ("a b", true),
            ("Alice's", true),
            ("folder", false),
            (".hidden", true),
            ("bad\nname", true),
        ]
        .into_iter()
        .map(|(name, file)| Entry {
            name: name.into(),
            file: Some(file),
            symlink: None,
        })
        .collect(),
    }
}
fn fixture() -> (Workspace, flume::Receiver<HostCommand>) {
    let (mut ws, rx) = tabbed_workspace("split");
    let sender = ws.hosts[0].connection.commands.clone();
    framed_terminal(&mut ws, "block:b", &sender, b"dev@build:~$ ls -d ");
    framed_terminal(&mut ws, "block:c", &sender, b"other pane");
    (ws, rx)
}
fn open(ws: &mut Workspace) {
    assert!(actions::run(
        ws,
        ACTION,
        &serde_json::Value::Null,
        &mut rune::Tx::default()
    ));
}
fn request(rx: &flume::Receiver<HostCommand>) -> Box<Request> {
    let commands: Vec<_> = rx.try_iter().collect();
    assert_eq!(
        commands.len(),
        1,
        "exactly one command, no ownership/size/input"
    );
    match commands.into_iter().next().unwrap() {
        HostCommand::ListDir(r) => r,
        _ => panic!("not a list request"),
    }
}
fn reply(ws: &mut Workspace, request: &Request, result: Result<ListDirResponse, Failure>) {
    ws.hosts[0].model.directory = Some(Arc::new(Reply {
        id: request.id,
        epoch: request.epoch,
        result,
    }));
    ws.process_directory(0);
}
fn edit(ws: &mut Workspace, text: &str) {
    let mut field = rune::TextEdit::new(text);
    field.focused = true;
    ws.panel.as_mut().unwrap().inputs[0] = field;
    ws.query_changed();
}
fn error(ws: &Workspace) -> &str {
    ws.panel.as_ref().unwrap().error.as_deref().unwrap()
}
fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
}

#[test]
fn quoting_is_posix_and_only_when_needed() {
    for (path, quoted) in [
        ("/tmp/a", "/tmp/a"),
        ("/tmp/a b", "'/tmp/a b'"),
        ("/tmp/Alice's", "'/tmp/Alice'\\''s'"),
        ("'", "''\\'''"),
        ("$HOME", "'$HOME'"),
        ("`id`", "'`id`'"),
        ("!", "'!'"),
        ("*", "'*'"),
        ("-rf", "./-rf"),
        ("-a b", "'./-a b'"),
        ("/tmp/é雪", "'/tmp/é雪'"),
        ("", "''"),
        ("/a//b/", "/a//b/"),
        ("a;b", "'a;b'"),
        ("$(id)", "'$(id)'"),
        ("a\\b", "'a\\b'"),
        ("~", "'~'"),
        ("a=b", "'a=b'"),
        ("a#b", "'a#b'"),
    ] {
        assert_eq!(directory::quote(path).unwrap(), quoted, "{path}");
    }
    for c in [
        '\0', '\n', '\r', '\t', '\u{1b}', '\u{7f}', '\u{85}', '\u{9b}',
    ] {
        assert!(directory::quote(&format!("/tmp/a{c}b")).is_err());
    }
    // Separators and invisible formatting are refused too: single quotes would keep them
    // literal, but a right-to-left override could disguise a name (`invoice<RLO>fdp.sh`).
    for c in [
        '\u{00AD}',
        '\u{034F}',
        '\u{115F}',
        '\u{180E}',
        '\u{3164}',
        '\u{FFA0}',
        '\u{061C}',
        '\u{200B}',
        '\u{200E}',
        '\u{200F}',
        '\u{2028}',
        '\u{2029}',
        '\u{202A}',
        '\u{202E}',
        '\u{2060}',
        '\u{2066}',
        '\u{2069}',
        '\u{FEFF}',
        '\u{E0001}',
        '\u{E007F}',
    ] {
        assert!(directory::refused(c), "{:04X}", c as u32);
        assert!(directory::quote(&format!("/tmp/a{c}b")).is_err());
    }
    // Joiners that scripts and emoji sequences need, and ordinary text, are kept.
    for name in [
        "/tmp/👩\u{200D}💻",
        "/tmp/می\u{200C}خواهم",
        "/tmp/שלום",
        "/tmp/a\u{00A0}b",
    ] {
        assert_eq!(directory::quote(name).unwrap(), format!("'{name}'"));
    }
}

#[test]
fn returned_names_with_refused_characters_are_left_out() {
    let mut reply = response();
    for name in [
        "invoice\u{202E}fdp.sh",
        "line\u{2028}break",
        "zero\u{200B}width",
        "ok\u{200D}joined",
    ] {
        reply.entries.push(rexrs::api::terminal::Entry {
            name: name.into(),
            file: Some(true),
            ..Default::default()
        });
    }
    let labels: Vec<_> = directory::rows(&reply, "")
        .into_iter()
        .map(|row| row.label)
        .collect();
    assert!(labels.contains(&"ok\u{200D}joined".to_owned()));
    assert!(
        labels
            .iter()
            .all(|label| !label.chars().any(directory::refused)),
        "{labels:?}"
    );
    assert!(
        !labels
            .iter()
            .any(|l| l.starts_with("invoice") || l.starts_with("line") || l.starts_with("zero"))
    );
}

#[test]
fn insert_path_is_never_remote_and_a_host_cannot_open_it() {
    use crate::app::actions::Remote;
    let spec = actions::find(ACTION).expect("registered");
    assert!(matches!(spec.remote(), Remote::Never(_)));
    assert!(spec.requires_person());
    // A client.perform request is refused, with consent on, and lists nothing.
    let (mut ws, rx) = fixture();
    ws.settings.hosts[0].remote_control = true;
    ws.remote_perform(
        0,
        crate::app::tests::remote_tests::pending(ACTION, serde_json::json!({})),
        &mut rune::Tx::default(),
    );
    crate::app::tests::remote_tests::code(
        crate::app::tests::remote_tests::reply(&rx),
        "not_supported",
    );
    assert!(ws.panel.is_none(), "no browser");
    // Nor can an action the host returns from a key binding.
    let mut model = ws.hosts[0].model.clone();
    model
        .client_actions
        .push(rex_rune::connection::actions::Queued {
            generation: ws.keymap_generation,
            session: Some("s".into()),
            block: Some("block:b".into()),
            action: rexrs::api::client::ClientAction {
                action: ACTION.into(),
                args: None,
            },
        });
    *ws.hosts[0].connection.model.lock().unwrap() = model;
    ws.sync(0, &mut rune::Tx::default());
    assert!(ws.panel.is_none());
    assert!(
        rx.try_iter()
            .all(|c| !matches!(c, HostCommand::ListDir(_) | HostCommand::Terminal { .. }))
    );
    assert!(ws.notices.iter().any(|n| n.text.contains("cannot start")));
}
#[test]
fn path_prefix_splitting_preserves_spaces_empty_components_and_home() {
    for (text, expected) in [
        ("", ("", "")),
        ("~", ("~", "")),
        ("~/", ("~/", "")),
        ("~/my dir/x", ("~/my dir/", "x")),
        ("/", ("/", "")),
        ("/a//b", ("/a//", "b")),
        ("rel", ("", "rel")),
        ("/a/ b ", ("/a/", " b ")),
        ("../.h", ("../", ".h")),
    ] {
        assert_eq!(directory::split_path(text), expected);
    }
}
#[test]
fn rows_are_sorted_filtered_and_hostile_names_never_become_paths() {
    let mut r = response();
    for name in [".", "..", "a/b", "", "bad\u{1b}name"] {
        r.entries.push(Entry {
            name: name.into(),
            ..Default::default()
        });
    }
    let rows = directory::rows(&r, "");
    assert_eq!(
        rows.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(),
        ["~", "a b", "Alice's", "folder", "z.txt"]
    );
    assert!(!rows[1].directory && rows[3].directory);
    assert_eq!(directory::rows(&r, "AL")[1].path, "/home/dev/Alice's");
    assert_eq!(directory::rows(&r, ".")[1].label, ".hidden");
    assert_eq!(directory::rows(&r, " a ")[0].path, "/home/dev/ a ");
}
#[test]
fn action_and_commands_capture_target_and_only_issue_a_list() {
    let (mut ws, rx) = fixture();
    open(&mut ws);
    let r = request(&rx);
    assert_eq!((r.session.as_str(), r.block.as_str()), ("s", "block:b"));
    assert_eq!(r.args.path.as_deref(), Some(""));
    assert_eq!(r.args.files, Some(true));
    assert_eq!(
        ws.panel.as_ref().unwrap().target.as_ref().unwrap().block,
        "block:b"
    );
    ws.close_panel(&mut rune::Tx::default());
    assert_eq!(r.open.strong_count(), 0);
    ws.picker(false);
    edit(&mut ws, "Insert path");
    let e = ws
        .entries()
        .into_iter()
        .find(|e| e.label == "Insert path…")
        .unwrap();
    ws.perform(e.action, &mut rune::Tx::default());
    assert!(matches!(
        ws.panel.as_ref().unwrap().kind,
        PanelKind::Directory(_)
    ));
    request(&rx);
    assert!(matches!(
        actions::find(ACTION).unwrap().remote(),
        actions::Remote::Never(_)
    ));
    assert_eq!(actions::find(ACTION).unwrap().title(), "Insert path");
    assert_eq!(ws.shortcut_detail(ACTION, &serde_json::Value::Null), "");
}
#[test]
fn edits_coalesce_and_stale_replies_never_replace_the_new_path() {
    let (mut ws, rx) = fixture();
    open(&mut ws);
    let old = request(&rx);
    edit(&mut ws, "/tmp/a");
    edit(&mut ws, "/tmp/ab");
    edit(&mut ws, "/tmp/abc");
    assert!(rx.is_empty());
    assert!(ws.entries().is_empty());
    reply(&mut ws, &old, Ok(response()));
    let new = request(&rx);
    assert_eq!(new.args.path.as_deref(), Some("/tmp/"));
    assert_eq!(new.args.prefix.as_deref(), Some("abc"));
    reply(&mut ws, &old, Ok(response()));
    assert!(ws.entries().is_empty());
    assert!(rx.is_empty());
    let mut r = response();
    r.path = "/tmp".into();
    reply(&mut ws, &new, Ok(r));
    assert_eq!(ws.entries()[0].label, "/tmp/abc");
    assert!(rx.is_empty());
}
#[test]
fn listing_errors_are_inline_and_never_retry_until_an_edit() {
    let (mut ws, rx) = fixture();
    for (status, message) in [
        ("missing", "doesn't exist"),
        ("not_dir", "isn't a folder"),
        ("denied", "isn't allowed"),
        ("unknown", "Couldn't list"),
    ] {
        open(&mut ws);
        let r = request(&rx);
        let mut answer = response();
        answer.status = status.into();
        reply(&mut ws, &r, Ok(answer));
        assert!(error(&ws).contains(message));
        for _ in 0..3 {
            ws.process_directory(0);
        }
        assert!(rx.is_empty());
        assert!(ws.entries().is_empty());
        edit(&mut ws, "/next/");
        request(&rx);
    }
    for failure in [
        Failure::Request,
        Failure::Timeout,
        Failure::Unsupported,
        Failure::Disconnected,
    ] {
        open(&mut ws);
        let r = request(&rx);
        reply(&mut ws, &r, Err(failure.clone()));
        assert_eq!(error(&ws), failure.message());
        ws.process_directory(0);
        assert!(rx.is_empty());
    }
    ws.hosts[0].model.connected = false;
    open(&mut ws);
    assert!(rx.is_empty());
    assert_eq!(error(&ws), Failure::Disconnected.message());
}
#[test]
fn disconnect_and_new_epoch_invalidate_pending_reply_without_retry() {
    let (mut ws, rx) = fixture();
    open(&mut ws);
    let r = request(&rx);
    ws.hosts[0].model.connection_epoch += 1;
    reply(&mut ws, &r, Ok(response()));
    assert!(ws.entries().is_empty());
    assert_eq!(error(&ws), Failure::Disconnected.message());
    assert!(rx.is_empty());
    edit(&mut ws, "~/");
    let new = request(&rx);
    reply(&mut ws, &r, Ok(response()));
    assert!(ws.entries().is_empty());
    reply(&mut ws, &new, Ok(response()));
    assert_eq!(ws.entries()[0].label, "~");
}
#[test]
fn tab_descends_hidden_filter_and_escape_restore_terminal_focus() {
    let _fonts = rune::text::with_families("Inter", "Inter");
    let (mut ws, rx) = fixture();
    open(&mut ws);
    let r = request(&rx);
    reply(&mut ws, &r, Ok(response()));
    assert_eq!(ws.panel.as_ref().unwrap().inputs[0].text, "~/");
    let mut rt = runtime(ws);
    assert!(focused(&rt).unwrap().contains("popup-search"));
    press(&mut rt, Key::Down);
    press(&mut rt, Key::Down);
    press(&mut rt, Key::Down);
    assert_eq!(
        rt.state.entries()[rt.state.panel.as_ref().unwrap().selected].label,
        "folder"
    );
    press(&mut rt, Key::Tab);
    let r = request(&rx);
    assert_eq!(r.args.path.as_deref(), Some("/home/dev/folder/"));
    let mut answer = response();
    answer.path = "/home/dev/folder".into();
    reply(&mut rt.state, &r, Ok(answer));
    rt.update(|s, _| edit(s, "/home/dev/."));
    let r = request(&rx);
    reply(&mut rt.state, &r, Ok(response()));
    assert_eq!(rt.state.entries()[1].label, ".hidden");
    press(&mut rt, Key::Escape);
    assert!(rt.state.panel.is_none());
    assert!(focused(&rt).is_none());
    assert!(
        rx.is_empty(),
        "browse and escape send no terminal input or other host commands"
    );
    assert_eq!(r.open.strong_count(), 0);
}
#[test]
fn enter_uses_captured_panes_paste_once_without_enter_or_space() {
    let _fonts = rune::text::with_families("Inter", "Inter");
    let (mut ws, rx) = fixture();
    open(&mut ws);
    let r = request(&rx);
    reply(&mut ws, &r, Ok(response()));
    ws.hosts[0].model.views.get_mut("s").unwrap().windows[1].focused_block_id =
        Some("block:c".into());
    let mut rt = runtime(ws);
    press(&mut rt, Key::Down);
    press(&mut rt, Key::Down);
    press(&mut rt, Key::Enter);
    assert!(rt.state.panel.is_none());
    let sent: Vec<_> = rx.try_iter().collect();
    assert_eq!(sent.len(), 1);
    match &sent[0] {
        HostCommand::Terminal {
            block,
            command: Command::Paste(text),
            ..
        } => {
            assert_eq!(block, "block:b");
            assert_eq!(text, "'/home/dev/Alice'\\''s'");
        }
        _ => panic!("expected only paste"),
    }
    rt.state.insert_path(2, &mut rune::Tx::default());
    assert!(rx.is_empty());
}
fn go(ws: &mut Workspace) {
    assert!(actions::run(
        ws,
        directory::GO,
        &serde_json::Value::Null,
        &mut rune::Tx::default()
    ));
}

#[test]
fn go_to_directory_lists_folders_and_types_cd_without_enter() {
    let _fonts = rune::text::with_families("Inter", "Inter");
    let (mut ws, rx) = fixture();
    go(&mut ws);
    assert_eq!(
        ws.panel.as_ref().unwrap().title,
        format!("Go to directory · {}", ws.settings.hosts[0].label)
    );
    let r = request(&rx);
    assert_eq!(r.args.files, Some(false), "folders only");
    assert_eq!(r.block, "block:b");
    // A server that lists files anyway does not get them offered.
    let mut listing = response();
    listing.entries.push(Entry {
        name: "my dir".into(),
        file: Some(false),
        symlink: None,
    });
    reply(&mut ws, &r, Ok(listing));
    let rows: Vec<_> = ws
        .entries()
        .iter()
        .map(|e| (e.label.clone(), e.detail.clone()))
        .collect();
    assert_eq!(
        rows,
        [
            ("~".to_owned(), "Go to typed folder".to_owned()),
            ("folder".to_owned(), "Folder".to_owned()),
            ("my dir".to_owned(), "Folder".to_owned()),
        ]
    );
    // Focus moving elsewhere does not move the target.
    ws.hosts[0].model.views.get_mut("s").unwrap().windows[1].focused_block_id =
        Some("block:c".into());
    let mut rt = runtime(ws);
    press(&mut rt, Key::Down);
    press(&mut rt, Key::Down);
    press(&mut rt, Key::Enter);
    assert!(rt.state.panel.is_none());
    let sent: Vec<_> = rx.try_iter().collect();
    assert_eq!(sent.len(), 1, "one paste: no Enter, no other key, no claim");
    match &sent[0] {
        HostCommand::Terminal {
            block,
            command: Command::Paste(text),
            ..
        } => {
            assert_eq!(block, "block:b");
            assert_eq!(text, "cd '/home/dev/my dir'");
            assert!(!text.contains(['\n', '\r']));
        }
        _ => panic!("expected only paste"),
    }
}

#[test]
fn go_to_directory_types_the_typed_folder_and_refuses_what_insert_path_refuses() {
    let (mut ws, rx) = fixture();
    go(&mut ws);
    let r = request(&rx);
    reply(&mut ws, &r, Ok(response()));
    ws.submit(&mut rune::Tx::default());
    match rx.try_recv().unwrap() {
        HostCommand::Terminal {
            command: Command::Paste(text),
            ..
        } => assert_eq!(text, "cd /home/dev"),
        _ => panic!(),
    }
    assert!(rx.is_empty());
    // A name with a control character is never typed after cd either.
    go(&mut ws);
    let r = request(&rx);
    reply(&mut ws, &r, Ok(response()));
    edit(&mut ws, "~/bad\nname");
    assert!(rx.is_empty());
    assert!(error(&ws).contains("control or invisible formatting characters"));
    // Tab descends, and the next read is still folders only.
    edit(&mut ws, "~/");
    let r = request(&rx);
    reply(&mut ws, &r, Ok(response()));
    ws.panel.as_mut().unwrap().selected = 1;
    ws.descend_directory();
    let r = request(&rx);
    assert_eq!(r.args.path.as_deref(), Some("/home/dev/folder/"));
    assert_eq!(r.args.files, Some(false));
}

#[test]
fn go_to_directory_is_never_remote_and_a_host_cannot_open_it() {
    use crate::app::actions::Remote;
    let spec = actions::find(directory::GO).expect("registered");
    assert!(matches!(spec.remote(), Remote::Never(_)));
    assert!(spec.requires_person());
    assert_eq!(spec.title(), "Go to directory");
    let (mut ws, rx) = fixture();
    ws.settings.hosts[0].remote_control = true;
    ws.remote_perform(
        0,
        crate::app::tests::remote_tests::pending(directory::GO, serde_json::json!({})),
        &mut rune::Tx::default(),
    );
    crate::app::tests::remote_tests::code(
        crate::app::tests::remote_tests::reply(&rx),
        "not_supported",
    );
    assert!(ws.panel.is_none(), "no browser");
    let mut model = ws.hosts[0].model.clone();
    model
        .client_actions
        .push(rex_rune::connection::actions::Queued {
            generation: ws.keymap_generation,
            session: Some("s".into()),
            block: Some("block:b".into()),
            action: rexrs::api::client::ClientAction {
                action: directory::GO.into(),
                args: None,
            },
        });
    *ws.hosts[0].connection.model.lock().unwrap() = model;
    ws.sync(0, &mut rune::Tx::default());
    assert!(ws.panel.is_none());
    assert!(
        rx.try_iter()
            .all(|c| !matches!(c, HostCommand::ListDir(_) | HostCommand::Terminal { .. }))
    );
}

#[test]
fn typed_directory_enter_and_paused_refusal() {
    let (mut ws, rx) = fixture();
    open(&mut ws);
    let r = request(&rx);
    reply(&mut ws, &r, Ok(response()));
    ws.submit(&mut rune::Tx::default());
    match rx.try_recv().unwrap() {
        HostCommand::Terminal {
            command: Command::Paste(text),
            ..
        } => assert_eq!(text, "/home/dev"),
        _ => panic!(),
    }
    open(&mut ws);
    let r = request(&rx);
    reply(&mut ws, &r, Ok(response()));
    ws.terminals
        .get(&ws.term_key("block:b"))
        .unwrap()
        .connection
        .model
        .lock()
        .unwrap()
        .connected = false;
    ws.submit(&mut rune::Tx::default());
    assert!(rx.is_empty());
    assert!(ws.panel.is_none());
    assert_eq!(
        ws.notices.last().unwrap().text,
        "Nothing was typed: the terminal is not live."
    );
}
#[test]
fn disappearing_target_dismisses_instead_of_retargeting() {
    let (mut ws, rx) = fixture();
    open(&mut ws);
    let r = request(&rx);
    reply(&mut ws, &r, Ok(response()));
    ws.hosts[0]
        .model
        .views
        .get_mut("s")
        .unwrap()
        .windows
        .remove(1);
    ws.submit(&mut rune::Tx::default());
    assert!(ws.panel.is_none());
    assert!(rx.is_empty());
    assert!(
        ws.notices
            .last()
            .unwrap()
            .text
            .contains("no longer available")
    );
}
#[test]
fn truncated_listing_sends_prefix_and_controls_are_refused() {
    let (mut ws, rx) = fixture();
    open(&mut ws);
    let r = request(&rx);
    let mut answer = response();
    answer.truncated = true;
    reply(&mut ws, &r, Ok(answer));
    assert!(matches!(&ws.panel.as_ref().unwrap().kind,PanelKind::Directory(b) if b.truncated));
    edit(&mut ws, "~/Al");
    let r = request(&rx);
    assert_eq!(r.args.prefix.as_deref(), Some("Al"));
    reply(&mut ws, &r, Ok(response()));
    assert_eq!(ws.entries().len(), 2);
    edit(&mut ws, "~/bad\nname");
    assert!(rx.is_empty());
    assert!(error(&ws).contains("control or invisible formatting characters"));
    edit(&mut ws, "~/invoice\u{202E}fdp.sh");
    assert!(rx.is_empty());
    assert!(error(&ws).contains("control or invisible formatting characters"));
    // A pasted megabyte is not a path and never becomes a request.
    edit(&mut ws, &format!("/{}", "x".repeat(1_200_000)));
    assert!(rx.is_empty());
    assert_eq!(error(&ws), "That path is too long.");
    edit(&mut ws, &format!("/{}", "x".repeat(4095)));
    let r = request(&rx);
    assert_eq!(r.args.path.as_deref(), Some("/"));
    reply(&mut ws, &r, Ok(response()));
    edit(&mut ws, "~/");
    let r = request(&rx);
    let mut answer = response();
    answer.path = "/bad\npath".into();
    reply(&mut ws, &r, Ok(answer));
    assert!(ws.entries().is_empty());
    assert!(error(&ws).contains("can't be inserted"));
}

#[test]
fn typing_in_the_real_field_only_lists_and_closed_replies_are_ignored() {
    let _fonts = rune::text::with_families("Inter", "Inter");
    let (mut ws, rx) = fixture();
    open(&mut ws);
    let r = request(&rx);
    reply(&mut ws, &r, Ok(response()));
    let mut rt = runtime(ws);
    // Select all, with the modifier this platform's keyboard sends for it: Command on
    // macOS; Control elsewhere, where Rune reports `cmd` along with it.
    rt.key(KeyEvent {
        key: Key::Command('a'),
        mods: Mods {
            ctrl: !cfg!(target_os = "macos"),
            cmd: true,
            ..Default::default()
        },
    });
    press(&mut rt, Key::Text("/tmp/".into()));
    let next = request(&rx);
    assert_eq!(next.args.path.as_deref(), Some("/tmp/"));
    press(&mut rt, Key::Escape);
    reply(&mut rt.state, &next, Ok(response()));
    assert!(rt.state.panel.is_none());
    assert!(rx.is_empty());
}

#[test]
fn model_changes_dismiss_a_gone_target_and_disconnect_clears_completed_rows() {
    let (mut ws, rx) = fixture();
    open(&mut ws);
    let r = request(&rx);
    reply(&mut ws, &r, Ok(response()));
    ws.hosts[0].model.connected = false;
    ws.process_directory(0);
    assert!(ws.entries().is_empty());
    assert_eq!(error(&ws), Failure::Disconnected.message());
    ws.hosts[0].model.connected = true;
    let mut model = ws.hosts[0].model.clone();
    model.views.get_mut("s").unwrap().windows.remove(1);
    *ws.hosts[0].connection.model.lock().unwrap() = model;
    ws.sync(0, &mut rune::Tx::default());
    assert!(ws.panel.is_none());
    assert!(
        ws.notices
            .iter()
            .any(|n| n.text.contains("no longer available"))
    );
}

fn draw(size: (f32, f32), name: &str) {
    let Some(mut camera) = Camera::new() else {
        return;
    };
    let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
    let _families = rune::text::with_families("Inter", "IBM Plex Mono");
    let (mut ws, rx) = fixture();
    ws.viewport = [size.0, size.1];
    ws.settings.hosts[0].label = "Build machine".into();
    open(&mut ws);
    let r = request(&rx);
    let mut answer = response();
    answer.truncated = true;
    reply(&mut ws, &r, Ok(answer));
    let mut rt = Runtime::with_text(
        ws,
        view,
        size,
        Env::new(true, 1.),
        rune::text::TextSystem::new(fonts.files, fonts.families),
    );
    rt.on_key = Some(crate::app::input::on_key);
    settle(&mut rt);
    press(&mut rt, Key::Down);
    camera.snap(&mut rt, name);
    let surface = super::pane_verb_tests::rect(&rt, "menu-surface");
    assert!(
        surface[0] >= 0.
            && surface[1] >= 0.
            && surface[0] + surface[2] <= size.0
            && surface[1] + surface[3] <= size.1
    );
    rt.update(|s, _| edit(s, "/missing/"));
    let r = request(&rx);
    let mut answer = response();
    answer.status = "missing".into();
    reply(&mut rt.state, &r, Ok(answer));
    settle(&mut rt);
    camera.snap(&mut rt, &format!("{name}-missing"));
}
/// Go to directory at both sizes: the folder-only list with a row selected.
fn draw_go(size: (f32, f32), name: &str) {
    let Some(mut camera) = Camera::new() else {
        return;
    };
    let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
    let _families = rune::text::with_families("Inter", "IBM Plex Mono");
    let (mut ws, rx) = fixture();
    ws.viewport = [size.0, size.1];
    ws.settings.hosts[0].label = "Build machine".into();
    go(&mut ws);
    let r = request(&rx);
    let mut answer = response();
    for name in ["my dir", "src", "target"] {
        answer.entries.push(Entry {
            name: name.into(),
            file: Some(false),
            symlink: None,
        });
    }
    reply(&mut ws, &r, Ok(answer));
    let mut rt = Runtime::with_text(
        ws,
        view,
        size,
        Env::new(true, 1.),
        rune::text::TextSystem::new(fonts.files, fonts.families),
    );
    rt.on_key = Some(crate::app::input::on_key);
    settle(&mut rt);
    press(&mut rt, Key::Down);
    press(&mut rt, Key::Down);
    camera.snap(&mut rt, name);
    let surface = super::pane_verb_tests::rect(&rt, "menu-surface");
    assert!(
        surface[0] >= 0.
            && surface[1] >= 0.
            && surface[0] + surface[2] <= size.0
            && surface[1] + surface[3] <= size.1
    );
}
#[test]
fn go_to_directory_draw_large() {
    draw_go((1024., 700.), "go-to-directory-1024");
}
#[test]
fn go_to_directory_draw_minimum() {
    draw_go((640., 480.), "go-to-directory-640");
}
#[test]
fn insert_path_draw_large() {
    draw((1024., 700.), "insert-path-1024");
}
#[test]
fn insert_path_draw_minimum() {
    draw((640., 480.), "insert-path-640");
}

fn remember(ws: &mut Workspace, host: usize, path: &str, directory: bool) {
    let endpoint = ws.settings.hosts[host].endpoint.clone();
    ws.settings.remember_path(&endpoint, path.into(), directory);
}
fn browser_rows(ws: &Workspace) -> Vec<directory::PathRow> {
    let PanelKind::Directory(b) = &ws.panel.as_ref().unwrap().kind else {
        panic!()
    };
    b.rows.clone()
}
fn pasted(rx: &flume::Receiver<HostCommand>) -> String {
    let sent: Vec<_> = rx.try_iter().collect();
    assert_eq!(sent.len(), 1, "one ordinary paste, no key or other command");
    match &sent[0] {
        HostCommand::Terminal {
            command: Command::Paste(text),
            ..
        } => text.clone(),
        _ => panic!("expected paste"),
    }
}

#[test]
fn recents_record_accepted_pastes_and_repeat_the_exact_quoted_text() {
    for is_go in [false, true] {
        let (mut ws, rx) = fixture();
        if is_go {
            go(&mut ws);
        } else {
            open(&mut ws);
        }
        reply(&mut ws, &request(&rx), Ok(response()));
        let index = if is_go { 1 } else { 2 };
        let chosen = browser_rows(&ws)[index].clone();
        ws.insert_path(index, &mut rune::Tx::default());
        let expected = pasted(&rx);
        let endpoint = ws.settings.hosts[0].endpoint.clone();
        assert_eq!(ws.settings.recent_paths[&endpoint][0].path, chosen.path);
        assert_eq!(
            ws.settings.recent_paths[&endpoint][0].directory,
            chosen.directory
        );
        assert_eq!(
            Settings::load(&ws.settings_path).unwrap().recent_paths,
            ws.settings.recent_paths
        );
        if is_go {
            go(&mut ws);
        } else {
            open(&mut ws);
        }
        assert!(
            ws.entries().is_empty(),
            "wait for the ordinary first listing"
        );
        reply(&mut ws, &request(&rx), Ok(response()));
        assert_eq!(ws.entries()[1].group, "Recent");
        ws.insert_path(1, &mut rune::Tx::default());
        assert_eq!(pasted(&rx), expected);
        assert_eq!(ws.settings.recent_paths[&endpoint].len(), 1);
        std::fs::remove_file(&ws.settings_path).unwrap();
    }
}

#[test]
fn recents_record_typed_known_folders_without_changing_the_typed_rows_behavior() {
    let (mut ws, rx) = fixture();
    open(&mut ws);
    reply(&mut ws, &request(&rx), Ok(response()));
    edit(&mut ws, "~/folder");
    reply(&mut ws, &request(&rx), Ok(response()));
    assert!(
        !browser_rows(&ws)[0].directory,
        "typed-row Tab semantics stay unchanged"
    );
    ws.insert_path(0, &mut rune::Tx::default());
    assert_eq!(pasted(&rx), "/home/dev/folder");
    assert!(ws.settings.recent_paths[&ws.settings.hosts[0].endpoint][0].directory);
    std::fs::remove_file(&ws.settings_path).unwrap();
}

#[test]
fn recents_never_record_cancelled_refused_or_failed_sends() {
    for failure in ["cancel", "paused", "gone", "host", "channel"] {
        let (mut ws, rx) = fixture();
        open(&mut ws);
        reply(&mut ws, &request(&rx), Ok(response()));
        match failure {
            // Looking and leaving is not using a path.
            "cancel" => {
                let mut rt = runtime(ws);
                press(&mut rt, Key::Down);
                press(&mut rt, Key::Escape);
                assert!(rt.state.panel.is_none());
                assert!(rt.state.settings.recent_paths.is_empty());
                assert!(rx.is_empty());
                assert!(!rt.state.settings_path.exists());
                continue;
            }
            "paused" => {
                ws.terminals[&ws.term_key("block:b")]
                    .connection
                    .model
                    .lock()
                    .unwrap()
                    .connected = false
            }
            "gone" => {
                ws.hosts[0]
                    .model
                    .views
                    .get_mut("s")
                    .unwrap()
                    .windows
                    .remove(1);
            }
            "host" => ws.hosts[0].model.connected = false,
            "channel" => {
                drop(rx);
                ws.insert_path(1, &mut rune::Tx::default());
                assert!(ws.settings.recent_paths.is_empty());
                continue;
            }
            _ => unreachable!(),
        }
        ws.insert_path(1, &mut rune::Tx::default());
        assert!(ws.settings.recent_paths.is_empty(), "{failure}");
        assert!(rx.is_empty());
        assert!(!ws.settings_path.exists());
    }
}

#[test]
fn recents_are_shared_across_sessions_but_not_hosts_and_forget_removed_or_edited_hosts() {
    let (mut ws, rx) = fixture();
    remember(&mut ws, 0, "/home/dev/previous", true);
    let mut other = ws.hosts[0].model.views["s"].clone();
    other.session_id = Some("other".into());
    ws.hosts[0].model.views.insert("other".into(), other);
    ws.hosts[0].model.selected = Some("other".into());
    open(&mut ws);
    let r = request(&rx);
    assert_eq!(r.session, "other");
    reply(&mut ws, &r, Ok(response()));
    assert_eq!(ws.entries()[1].label, "~/previous");
    ws.close_panel(&mut rune::Tx::default());
    let mut ws = workspace_hosts(2);
    remember(&mut ws, 0, "/only-first", true);
    remember(&mut ws, 1, "/only-second", false);
    let first = ws.settings.hosts[0].endpoint.clone();
    let second = ws.settings.hosts[1].endpoint.clone();
    assert_eq!(ws.settings.recent_paths[&first][0].path, "/only-first");
    assert_eq!(ws.settings.recent_paths[&second][0].path, "/only-second");
    ws.edit_host(0, "Renamed".into(), first.clone(), None);
    assert!(ws.settings.recent_paths.contains_key(&first));
    ws.edit_host(
        0,
        "Different".into(),
        "unix:///nonexistent/other-recent-test.sock".into(),
        None,
    );
    assert!(!ws.settings.recent_paths.contains_key(&first));
    assert!(
        !ws.settings
            .recent_paths
            .contains_key(&ws.settings.hosts[0].endpoint)
    );
    ws.remove_host(1, &mut rune::Tx::default());
    assert!(ws.settings.recent_paths.is_empty());
    assert!(
        Settings::load(&ws.settings_path)
            .unwrap()
            .recent_paths
            .is_empty()
    );
    std::fs::remove_file(&ws.settings_path).unwrap();
}

#[test]
fn recents_filter_files_and_hostile_disk_paths_without_any_extra_read() {
    use rex_rune::settings::recent_paths::RecentPath;
    let (mut ws, rx) = fixture();
    remember(&mut ws, 0, "/home/dev/old folder", true);
    remember(&mut ws, 0, "/home/dev/old file", false);
    let endpoint = ws.settings.hosts[0].endpoint.clone();
    for path in [
        "relative".into(),
        format!("/{}", "x".repeat(4096)),
        "/bad\nname".into(),
        "/bad\u{202e}name".into(),
    ] {
        ws.settings
            .recent_paths
            .get_mut(&endpoint)
            .unwrap()
            .push(RecentPath {
                path,
                directory: true,
            });
    }
    // An edited on-disk list could belong to an absent/different host.
    ws.settings.recent_paths.insert(
        "ssh://elsewhere".into(),
        vec![RecentPath {
            path: "/not-here".into(),
            directory: true,
        }],
    );
    for is_go in [false, true] {
        if is_go {
            go(&mut ws);
        } else {
            open(&mut ws);
        }
        reply(&mut ws, &request(&rx), Ok(response()));
        let recent: Vec<_> = ws
            .entries()
            .into_iter()
            .filter(|e| e.group == "Recent")
            .map(|e| e.label)
            .collect();
        assert_eq!(
            recent,
            if is_go {
                vec!["~/old folder"]
            } else {
                vec!["~/old file", "~/old folder"]
            }
        );
        assert!(rx.is_empty(), "recents never list or check existence");
    }
}

#[test]
fn recents_hide_on_edit_or_tab_and_absent_history_has_identical_rows() {
    let _fonts = rune::text::with_families("Inter", "Inter");
    let (mut ws, rx) = fixture();
    open(&mut ws);
    reply(&mut ws, &request(&rx), Ok(response()));
    let baseline = browser_rows(&ws);
    assert_eq!(baseline, directory::rows(&response(), ""));
    assert!(ws.entries().iter().all(|e| e.group.is_empty()));
    remember(&mut ws, 0, "/home/dev/old folder", true);
    open(&mut ws);
    reply(&mut ws, &request(&rx), Ok(response()));
    assert_eq!(browser_rows(&ws).len(), baseline.len() + 1);
    let mut rt = runtime(ws);
    press(&mut rt, Key::Text("a".into()));
    let r = request(&rx);
    reply(&mut rt.state, &r, Ok(response()));
    assert_eq!(browser_rows(&rt.state), directory::rows(&response(), "a"));
    edit(&mut rt.state, "~/");
    reply(&mut rt.state, &request(&rx), Ok(response()));
    assert_eq!(
        browser_rows(&rt.state),
        baseline,
        "restoring field does not restore recents"
    );
    assert!(rt.state.entries().iter().all(|e| e.group.is_empty()));
    open(&mut rt.state);
    reply(&mut rt.state, &request(&rx), Ok(response()));
    rt.state.panel.as_mut().unwrap().selected = 1;
    rt.state.descend_directory();
    let r = request(&rx);
    assert_eq!(r.args.path.as_deref(), Some("/home/dev/old folder/"));
    let mut answer = response();
    answer.path = "/home/dev/old folder".into();
    reply(&mut rt.state, &r, Ok(answer.clone()));
    assert_eq!(browser_rows(&rt.state), directory::rows(&answer, ""));
}

#[test]
fn recents_leave_the_typed_row_selected_so_a_bare_enter_is_unchanged() {
    let _fonts = rune::text::with_families("Inter", "Inter");
    let (mut ws, rx) = fixture();
    remember(&mut ws, 0, "/home/dev/old folder", true);
    open(&mut ws);
    reply(&mut ws, &request(&rx), Ok(response()));
    assert_eq!(ws.entries()[1].group, "Recent");
    assert_eq!(ws.panel.as_ref().unwrap().selected, 0);
    let mut rt = runtime(ws);
    press(&mut rt, Key::Enter);
    assert_eq!(
        pasted(&rx),
        "/home/dev",
        "Enter at open inserts the typed path, as it does with no recents"
    );
    std::fs::remove_file(&rt.state.settings_path).unwrap();
}

#[test]
fn go_does_not_remember_a_typed_name_that_was_never_listed() {
    let (mut ws, rx) = fixture();
    go(&mut ws);
    reply(&mut ws, &request(&rx), Ok(response()));
    edit(&mut ws, "~/fol");
    reply(&mut ws, &request(&rx), Ok(response()));
    ws.insert_path(0, &mut rune::Tx::default());
    assert_eq!(pasted(&rx), "cd /home/dev/fol", "typed as asked");
    assert!(
        ws.settings.recent_paths.is_empty(),
        "an unlisted name is not known to be a folder"
    );
    // The listed folder itself, and a listed child, are.
    go(&mut ws);
    reply(&mut ws, &request(&rx), Ok(response()));
    ws.insert_path(0, &mut rune::Tx::default());
    assert_eq!(pasted(&rx), "cd /home/dev");
    let endpoint = ws.settings.hosts[0].endpoint.clone();
    assert_eq!(
        ws.settings.recent_paths[&endpoint]
            .iter()
            .map(|r| (r.path.as_str(), r.directory))
            .collect::<Vec<_>>(),
        [("/home/dev", true)]
    );
    std::fs::remove_file(&ws.settings_path).unwrap();
}

#[test]
fn switching_recents_off_with_a_browser_open_removes_its_recent_rows() {
    let (mut ws, rx) = fixture();
    remember(&mut ws, 0, "/home/dev/old folder", true);
    open(&mut ws);
    reply(&mut ws, &request(&rx), Ok(response()));
    assert_eq!(ws.entries()[1].group, "Recent");
    ws.set_show_recent_paths(false);
    assert_eq!(browser_rows(&ws), directory::rows(&response(), ""));
    assert!(ws.entries().iter().all(|e| e.group.is_empty()));
    assert_eq!(ws.panel.as_ref().unwrap().selected, 0);
    // The second row is the listing's again, not the forgotten path.
    ws.insert_path(1, &mut rune::Tx::default());
    assert_eq!(pasted(&rx), "'/home/dev/a b'");
    assert!(ws.settings.recent_paths.is_empty());
    std::fs::remove_file(&ws.settings_path).unwrap();
}

#[test]
fn recents_stay_hidden_after_edits_before_the_first_reply() {
    let (mut ws, rx) = fixture();
    remember(&mut ws, 0, "/old", true);
    open(&mut ws);
    let first = request(&rx);
    edit(&mut ws, "~/folder/");
    edit(&mut ws, "");
    reply(&mut ws, &first, Ok(response()));
    assert_eq!(browser_rows(&ws), directory::rows(&response(), ""));
    assert!(rx.is_empty());
}

fn open_recent_setting(rt: &mut Runtime<Workspace>) {
    rt.update(|s, tx| {
        s.perform(Intent::Settings, tx);
    });
    settle(rt);
    rt.update(|s, tx| {
        s.settings_nav.search.text = "Show recently used paths".into();
        s.settings_search_changed();
        let hit = s
            .settings_nav
            .hits()
            .iter()
            .position(|h| h.label == "Show recently used paths" && h.place == "Terminal")
            .expect("actual Settings label indexed");
        s.open_settings_hit(hit, tx);
    });
    settle(rt);
}

#[test]
fn recents_setting_is_searchable_and_off_forgets_stops_showing_and_recording() {
    let _fonts = rune::text::with_families("Inter", "Inter");
    let (mut ws, rx) = fixture();
    remember(&mut ws, 0, "/private-folder", true);
    let mut rt = runtime(ws);
    open_recent_setting(&mut rt);
    click(&mut rt, "show-recent-paths");
    assert!(!rt.state.settings.terminal.show_recent_paths);
    assert!(rt.state.settings.recent_paths.is_empty());
    let saved = Settings::load(&rt.state.settings_path).unwrap();
    assert!(!saved.terminal.show_recent_paths && saved.recent_paths.is_empty());
    assert!(
        !std::fs::read_to_string(&rt.state.settings_path)
            .unwrap()
            .contains("/private-folder")
    );
    rt.state.close_panel(&mut rune::Tx::default());
    open(&mut rt.state);
    reply(&mut rt.state, &request(&rx), Ok(response()));
    assert_eq!(browser_rows(&rt.state), directory::rows(&response(), ""));
    rt.state.insert_path(1, &mut rune::Tx::default());
    assert_eq!(pasted(&rx), "'/home/dev/a b'");
    assert!(rt.state.settings.recent_paths.is_empty());
    open_recent_setting(&mut rt);
    click(&mut rt, "show-recent-paths");
    assert!(rt.state.settings.terminal.show_recent_paths);
    assert!(rt.state.settings.recent_paths.is_empty());
    std::fs::remove_file(&rt.state.settings_path).unwrap();
}

fn draw_recents(size: (f32, f32), name: &str, settings: bool) {
    let Some(mut camera) = Camera::new() else {
        return;
    };
    let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
    let _families = rune::text::with_families("Inter", "IBM Plex Mono");
    let (mut ws, rx) = fixture();
    ws.viewport = [size.0, size.1];
    ws.settings.hosts[0].label = "Build machine".into();
    remember(
        &mut ws,
        0,
        "/home/dev/projects/a-long-folder-name/another-long-folder/release-notes-for-the-next-preview.md",
        false,
    );
    remember(&mut ws, 0, "/home/dev/Alice's", false);
    remember(&mut ws, 0, "/home/dev/projects/rex", true);
    open(&mut ws);
    reply(&mut ws, &request(&rx), Ok(response()));
    let mut rt = Runtime::with_text(
        ws,
        view,
        size,
        Env::new(true, 1.),
        rune::text::TextSystem::new(fonts.files, fonts.families),
    );
    rt.on_key = Some(crate::app::input::on_key);
    settle(&mut rt);
    if settings {
        open_recent_setting(&mut rt);
        let control = super::pane_verb_tests::rect(&rt, "show-recent-paths");
        let scroll = super::pane_verb_tests::rect(&rt, "settings-scroll:terminal");
        assert!(control[0] >= scroll[0] && control[0] + control[2] <= scroll[0] + scroll[2] + 0.5);
        assert!(control[1] >= scroll[1] && control[1] + control[3] <= scroll[1] + scroll[3] + 0.5);
    } else {
        press(&mut rt, Key::Down);
        assert_eq!(rt.state.entries()[1].group, "Recent");
        let surface = super::pane_verb_tests::rect(&rt, "menu-surface");
        assert!(
            surface[0] >= 0.
                && surface[1] >= 0.
                && surface[0] + surface[2] <= size.0
                && surface[1] + surface[3] <= size.1
        );
    }
    camera.snap(&mut rt, name);
}

#[test]
fn recent_paths_draw_large() {
    draw_recents((1024., 700.), "recent-paths-1024", false);
}
#[test]
fn recent_paths_draw_minimum() {
    draw_recents((640., 480.), "recent-paths-640", false);
}
#[test]
fn recent_paths_settings_draw_large() {
    draw_recents((1024., 700.), "recent-paths-settings-1024", true);
}
#[test]
fn recent_paths_settings_draw_minimum() {
    draw_recents((640., 480.), "recent-paths-settings-640", true);
}
