//! The terminal verbs of the pane menu and Commands: Copy, Paste, Select all, Copy pane ID,
//! Clear terminal and Reset terminal, on the pane the menu names, and their `pane.*` actions.
use super::*;
use crate::app::actions;
use rex_rune::{
    connection::{Command, Connection, TerminalSender},
    terminal::{Renderer, selection},
};
use serde_json::Value;
use std::sync::Arc;

/// Make `block` a live terminal showing `output`, with its commands on `sender`. The returned
/// terminal stands in for the pane's worker.
pub(super) fn framed_terminal(
    ws: &mut Workspace,
    block: &str,
    sender: &flume::Sender<HostCommand>,
    output: &[u8],
) -> ghostty_vt::Terminal {
    let connection = Connection {
        model: Default::default(),
        commands: TerminalSender::new(sender.clone(), "s".into(), block.into()),
        wake: flume::unbounded().1,
    };
    let mut terminal = ghostty_vt::Terminal::new(ghostty_vt::TerminalOptions {
        cols: 40,
        rows: 4,
        ..Default::default()
    })
    .unwrap();
    terminal.vt_write(output);
    {
        let mut m = connection.model.lock().unwrap();
        m.connected = true;
        m.control_connected = true;
        m.frame = Some(Arc::new(Renderer::new().unwrap().frame(&terminal).unwrap()));
    }
    let key = ws.term_key(block);
    let mut term = crate::app::term::Term::new(connection);
    term.refresh();
    ws.terminals.insert(key, term);
    terminal
}

/// What terminals were sent, as (block, verb), without keys and pointer reports.
pub(super) fn sent_verbs(rx: &flume::Receiver<HostCommand>) -> Vec<(String, String)> {
    rx.try_iter()
        .filter_map(|c| match c {
            HostCommand::Terminal { block, command, .. } => Some((
                block,
                match command {
                    Command::Reset => "reset".to_owned(),
                    Command::Clear => "clear".to_owned(),
                    Command::Paste(text) => format!("paste {text}"),
                    Command::Selection(input) => format!("selection {:?}", input.phase),
                    _ => return None,
                },
            )),
            _ => None,
        })
        .collect()
}

/// The open menu's rows as (label, enabled).
fn rows(rt: &Runtime<Workspace>) -> Vec<(String, bool)> {
    rt.state
        .entries()
        .into_iter()
        .map(|e| (e.label, e.enabled))
        .collect()
}

fn enabled(rt: &Runtime<Workspace>, label: &str) -> bool {
    rows(rt)
        .into_iter()
        .find(|(l, _)| l == label)
        .unwrap_or_else(|| panic!("no row {label}"))
        .1
}

/// Click the open menu's row `label`.
fn choose(rt: &mut Runtime<Workspace>, label: &str) {
    let index = rows(rt)
        .iter()
        .position(|(l, _)| l == label)
        .unwrap_or_else(|| panic!("no row {label}"));
    click(rt, &format!("entry:{index}"));
}

pub(super) fn rect(rt: &Runtime<Workspace>, key: &str) -> [f32; 4] {
    rt.elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("missing {key}"))
        .rect
}

/// A split tab: `block:b` (focused) and `block:c`, both live, with a clipboard.
fn split() -> (
    Runtime<Workspace>,
    flume::Receiver<HostCommand>,
    flume::Receiver<HostCommand>,
    rune::clipboard::MemoryClipboard,
) {
    let (mut ws, host) = tabbed_workspace("split");
    let (sender, sent) = flume::unbounded();
    framed_terminal(&mut ws, "block:b", &sender, b"left pane");
    framed_terminal(&mut ws, "block:c", &sender, b"right pane");
    let mut rt = runtime(ws);
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    let clipboard = rune::clipboard::memory();
    clipboard.set_text("sentinel");
    rt.use_clipboard(&clipboard);
    (rt, host, sent, clipboard)
}

fn clipboard_text(clipboard: &rune::clipboard::MemoryClipboard) -> String {
    clipboard.text().as_deref().cloned().unwrap_or_default()
}

#[test]
fn zoom_cannot_run_on_the_only_pane_of_a_tab_in_the_menu_or_commands() {
    let (ws, _host) = tabbed_workspace("single");
    let mut rt = runtime(ws);
    rt.update(|s, _| s.pane_menu("block:a".into(), Some((300., 200.))));
    rt.frame(1. / 60.);
    assert!(
        !enabled(&rt, "Zoom pane"),
        "the only pane has the whole tab"
    );
    for label in ["Split right", "Rename pane…", "Close pane…"] {
        assert!(enabled(&rt, label), "{label}");
    }
    press(&mut rt, Key::Escape);
    let zoom = |rt: &Runtime<Workspace>, label: &str| {
        rt.state
            .entries()
            .into_iter()
            .find(|e| e.label == label)
            .unwrap_or_else(|| panic!("no {label} in Commands"))
            .enabled
    };
    rt.update(|s, _| s.picker(false));
    assert!(!zoom(&rt, "Zoom pane"), "Commands agrees with the menu");
    press(&mut rt, Key::Escape);
    std::fs::remove_file(&rt.state.settings_path).ok();

    // A zoomed pane can always be restored, in a tab of two.
    let (mut ws, _host) = tabbed_workspace("split");
    let view = ws.hosts[0].model.views.get_mut("s").unwrap();
    view.windows[1].zoomed_block_id = Some("block:b".into());
    let mut rt = runtime(ws);
    rt.update(|s, _| s.pane_menu("block:b".into(), Some((300., 200.))));
    rt.frame(1. / 60.);
    assert!(enabled(&rt, "Restore pane"));
    press(&mut rt, Key::Escape);
    rt.update(|s, _| s.picker(false));
    assert!(zoom(&rt, "Restore pane"));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_pane_menu_lists_the_verbs_in_groups_and_marks_what_cannot_run() {
    let (mut ws, _host) = tabbed_workspace("split");
    let (sender, _sent) = flume::unbounded();
    framed_terminal(&mut ws, "block:b", &sender, b"left pane");
    let mut rt = runtime(ws);
    rt.update(|s, _| s.pane_menu("block:b".into(), Some((300., 200.))));
    rt.frame(1. / 60.);
    assert_eq!(
        rt.state
            .entries()
            .iter()
            .map(|e| (e.label.as_str(), e.section))
            .collect::<Vec<_>>(),
        [
            ("Copy", 0),
            ("Paste", 0),
            ("Select all", 0),
            ("Split right", 1),
            ("Split left", 1),
            ("Split down", 1),
            ("Split up", 1),
            ("Zoom pane", 1),
            ("Rename pane…", 2),
            ("Copy pane ID", 2),
            ("Download file…", 2),
            ("Clear terminal", 3),
            ("Reset terminal", 3),
            ("Reconnect terminal", 3),
            ("Close pane…", 4),
        ],
        "clipboard, layout, the pane, its terminal, Close"
    );
    assert!(!enabled(&rt, "Copy"), "nothing is selected");
    for label in ["Paste", "Select all", "Clear terminal", "Reset terminal"] {
        assert!(enabled(&rt, label), "{label} on a live terminal");
    }
    assert!(enabled(&rt, "Zoom pane"), "one pane of two can be zoomed");
    assert_eq!(
        rt.state.panel.as_ref().unwrap().selected,
        1,
        "the menu opens on its first row that can run"
    );
    press(&mut rt, Key::Escape);

    // A pane with no terminal yet: only what needs none.
    rt.update(|s, _| s.pane_menu("block:c".into(), Some((300., 200.))));
    rt.frame(1. / 60.);
    for label in [
        "Copy",
        "Paste",
        "Select all",
        "Clear terminal",
        "Reset terminal",
    ] {
        assert!(!enabled(&rt, label), "{label} without a terminal");
    }
    for label in ["Split right", "Copy pane ID", "Close pane…"] {
        assert!(enabled(&rt, label), "{label}");
    }
    press(&mut rt, Key::Escape);

    // A terminal whose connection dropped keeps its text but takes no paste, clear or reset.
    rt.update(|s, _| {
        let key = s.term_key("block:b");
        let term = &s.terminals[&key];
        term.connection.model.lock().unwrap().control_connected = false;
        s.pane_menu("block:b".into(), Some((300., 200.)));
    });
    rt.frame(1. / 60.);
    for label in ["Paste", "Clear terminal", "Reset terminal"] {
        assert!(!enabled(&rt, label), "{label} while disconnected");
    }
    assert!(enabled(&rt, "Select all"));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn copy_is_unavailable_until_text_is_dragged_then_copies_it() {
    let (mut ws, _host) = tabbed_workspace("single");
    let (sender, _sent) = flume::unbounded();
    framed_terminal(&mut ws, "block:a", &sender, b"alpha beta gamma");
    let mut rt = runtime(ws);
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    let clipboard = rune::clipboard::memory();
    clipboard.set_text("sentinel");
    rt.use_clipboard(&clipboard);

    secondary_click(&mut rt, "grid:block:a");
    assert!(matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::Pane)
    ));
    assert!(!enabled(&rt, "Copy"), "no selection yet");
    press(&mut rt, Key::Escape);
    assert_eq!(clipboard_text(&clipboard), "sentinel");

    // Drag over the first five cells.
    let grid = rect(&rt, "grid:block:a");
    let cell = crate::app::term::metrics(rt.state.settings.font_size).cell;
    let y = grid[1] + cell.1 / 2.;
    rt.pointer_down(grid[0] + cell.0 * 0.5, y);
    rt.pointer_move(grid[0] + cell.0 * 2.5, y);
    rt.pointer_move(grid[0] + cell.0 * 4.5, y);
    rt.pointer_up(grid[0] + cell.0 * 4.5, y);
    for _ in 0..4 {
        rt.frame(1. / 60.);
    }

    // The right-click that opens the menu leaves the selection alone.
    secondary_click(&mut rt, "grid:block:a");
    assert!(enabled(&rt, "Copy"), "text is selected");
    assert_eq!(
        rt.state.panel.as_ref().unwrap().selected,
        0,
        "and Copy is the row the menu opens on"
    );
    choose(&mut rt, "Copy");
    assert!(rt.state.panel.is_none(), "the menu closes");
    assert_eq!(clipboard_text(&clipboard), "alpha");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_menus_verbs_go_to_the_pane_it_names_not_the_focused_one() {
    let (mut rt, host, sent, clipboard) = split();
    assert_eq!(rt.state.focused_block().as_deref(), Some("block:b"));
    host.try_iter().count();

    // Paste: into the clicked pane, which also takes the keyboard for what is typed next.
    clipboard.set_text("pasted");
    secondary_click(&mut rt, "grid:block:c");
    assert_eq!(
        rt.state
            .panel
            .as_ref()
            .and_then(|p| p.target.as_ref())
            .map(|t| t.block.as_str()),
        Some("block:c")
    );
    choose(&mut rt, "Paste");
    for _ in 0..3 {
        rt.frame(1. / 60.);
    }
    let commands: Vec<HostCommand> = sent.try_iter().collect();
    let pastes: Vec<(&str, &str)> = commands
        .iter()
        .filter_map(|c| match c {
            HostCommand::Terminal {
                block,
                command: Command::Paste(text),
                ..
            } => Some((block.as_str(), text.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(
        pastes,
        [("block:c", "pasted")],
        "one paste, none to the focused pane"
    );
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, HostCommand::Request { method, payload }
                if method == "session.focus_block" && payload["block_id"] == "block:c")),
        "and the pane takes the keyboard"
    );

    // Clear and Reset: one command each, to that pane only.
    for (label, verb) in [("Clear terminal", "clear"), ("Reset terminal", "reset")] {
        secondary_click(&mut rt, "grid:block:c");
        choose(&mut rt, label);
        assert!(rt.state.panel.is_none(), "{label} asks nothing");
        assert_eq!(
            sent_verbs(&sent),
            [("block:c".to_owned(), verb.to_owned())],
            "{label}"
        );
    }

    // Copy pane ID: the block ID, as `rex` names it, and a word that it happened.
    secondary_click(&mut rt, "grid:block:c");
    choose(&mut rt, "Copy pane ID");
    assert_eq!(clipboard_text(&clipboard), "block:c");
    assert!(
        rt.state
            .notices
            .iter()
            .any(|n| n.text == "Copied pane ID to clipboard.")
    );
    assert!(sent_verbs(&sent).is_empty(), "the server hears nothing");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_menu_paste_focuses_its_pane_and_is_dropped_if_the_pane_goes() {
    for (gone, expected) in [(false, 1), (true, 0)] {
        let (mut ws, _host) = tabbed_workspace("split");
        let (sender, sent) = flume::unbounded();
        framed_terminal(&mut ws, "block:b", &sender, b"left pane");
        framed_terminal(&mut ws, "block:c", &sender, b"right pane");
        let mut rt = runtime(ws);
        let clipboard = rune::clipboard::memory();
        clipboard.set_text("pasted");
        rt.use_clipboard(&clipboard);
        rt.update(move |s, tx| {
            let target = s.target("block:c").unwrap();
            s.perform(
                Intent::Pane(target, crate::app::panel::PaneAction::Paste),
                tx,
            );
            if gone {
                // The server closed the pane before the clipboard answered.
                let view = s.hosts[0].model.views.get_mut("s").unwrap();
                view.windows[1].layers[0]
                    .blocks
                    .retain(|b| b.block_id.as_deref() != Some("block:c"));
            }
        });
        for _ in 0..3 {
            rt.frame(1. / 60.);
        }
        let commands: Vec<HostCommand> = sent.try_iter().collect();
        assert!(
            commands
                .iter()
                .any(|c| matches!(c, HostCommand::Request { method, payload }
                if method == "session.focus_block" && payload["block_id"] == "block:c")),
            "the pane the menu named takes the keyboard"
        );
        let pastes = commands
            .iter()
            .filter(|c| {
                matches!(
                    c,
                    HostCommand::Terminal {
                        command: Command::Paste(_),
                        ..
                    }
                )
            })
            .count();
        assert_eq!(pastes, expected, "pane gone: {gone}");
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn select_all_selects_the_whole_terminal_through_its_worker_and_copy_takes_it() {
    let (mut ws, _host) = tabbed_workspace("single");
    let (sender, sent) = flume::unbounded();
    let mut terminal = framed_terminal(&mut ws, "block:a", &sender, b"");
    for n in 0..12 {
        terminal.vt_write(format!("line-{n:02}\r\n").as_bytes());
    }
    terminal.vt_write(b"tail");
    let key = ws.term_key("block:a");
    let model = ws.terminals[&key].connection.model.clone();
    let mut renderer = Renderer::new().unwrap();
    let mut controller = selection::Controller::new().unwrap();
    model.lock().unwrap().frame = Some(Arc::new(renderer.frame(&terminal).unwrap()));
    let mut rt = runtime(ws);
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    let clipboard = rune::clipboard::memory();
    rt.use_clipboard(&clipboard);

    secondary_click(&mut rt, "grid:block:a");
    choose(&mut rt, "Select all");
    // The pane's worker: apply what the client asked, and publish the frame.
    let inputs: Vec<selection::Input> = sent
        .try_iter()
        .filter_map(|c| match c {
            HostCommand::Terminal {
                command: Command::Selection(input),
                ..
            } => Some(input),
            _ => None,
        })
        .collect();
    assert!(
        matches!(inputs.last().map(|i| i.phase), Some(selection::Phase::All)),
        "{inputs:?}"
    );
    assert!(
        !rt.state.terminals[&key].has_selection(),
        "nothing is selected until the worker's frame says so"
    );
    for input in inputs {
        controller.apply(&terminal, input).unwrap();
    }
    let mut frame = renderer.frame(&terminal).unwrap();
    frame.selection_revision = controller.revision;
    model.lock().unwrap().frame = Some(Arc::new(frame));
    rt.update(move |s, _| s.refresh_terminal(&key));
    rt.frame(1. / 60.);

    secondary_click(&mut rt, "grid:block:a");
    assert!(enabled(&rt, "Copy"), "everything is selected");
    choose(&mut rt, "Copy");
    let copied = clipboard_text(&clipboard);
    assert!(copied.starts_with("line-00\nline-01"), "{copied:?}");
    assert!(
        copied.ends_with("line-11\ntail"),
        "scrollback too: {copied:?}"
    );

    // A click on the terminal starts over, and the worker is told.
    click(&mut rt, "grid:block:a");
    let key = rt.state.term_key("block:a");
    assert!(!rt.state.terminals[&key].has_selection());
    assert!(
        sent_verbs(&sent)
            .iter()
            .any(|(_, verb)| verb.starts_with("selection Down")),
        "the click reaches the worker, which drops the selection"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn commands_lists_the_verbs_for_the_focused_pane() {
    let (mut rt, _host, sent, clipboard) = split();
    rt.update(|s, _| s.picker(false));
    rt.frame(1. / 60.);
    let pane: Vec<(String, bool)> = rt
        .state
        .entries()
        .into_iter()
        .filter(|e| e.group.starts_with("Pane · "))
        .map(|e| (e.label, e.enabled))
        .collect();
    for (label, on) in [
        ("Copy selection", false),
        ("Paste", true),
        ("Select all", true),
        ("Copy pane ID", true),
        ("Clear terminal", true),
        ("Reset terminal", true),
    ] {
        assert!(
            pane.contains(&(label.to_owned(), on)),
            "{label} in Commands: {pane:?}"
        );
    }

    // Typed, found and run: on the focused pane, once.
    for c in "reset".chars() {
        press(&mut rt, Key::Text(c.to_string()));
    }
    assert_eq!(
        rows(&rt).first(),
        Some(&("Reset terminal".to_owned(), true)),
        "the pane's row is the first match, before the view's resets"
    );
    press(&mut rt, Key::Enter);
    assert!(rt.state.panel.is_none());
    assert_eq!(
        sent_verbs(&sent),
        [("block:b".to_owned(), "reset".to_owned())]
    );

    rt.update(|s, _| s.picker(false));
    rt.frame(1. / 60.);
    clipboard.set_text("from commands");
    for c in "paste".chars() {
        press(&mut rt, Key::Text(c.to_string()));
    }
    press(&mut rt, Key::Enter);
    for _ in 0..3 {
        rt.frame(1. / 60.);
    }
    assert_eq!(
        sent_verbs(&sent),
        [("block:b".to_owned(), "paste from commands".to_owned())]
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_verbs_are_actions_on_the_focused_pane_and_never_remote() {
    let (mut rt, _host, sent, clipboard) = split();
    for (name, title) in [
        ("pane.select_all", "Select all"),
        ("pane.copy_id", "Copy pane ID"),
        ("pane.clear", "Clear terminal"),
        ("pane.reset", "Reset terminal"),
    ] {
        let spec = actions::find(name).unwrap_or_else(|| panic!("{name} is registered"));
        assert_eq!(spec.title(), title);
        assert!(
            matches!(spec.remote(), actions::Remote::Never(_)),
            "{name} is never remote"
        );
        assert!(spec.accepts(&Value::Null) && !spec.accepts(&serde_json::json!({"x": 1})));
        assert!(!spec.repeats() && !spec.global());
    }
    assert!(
        rex_rune::keymap::defaults()
            .bindings
            .iter()
            .all(|b| !matches!(
                b.action.as_str(),
                "pane.select_all" | "pane.copy_id" | "pane.clear" | "pane.reset"
            )),
        "no new key is taken from terminal programs by default"
    );

    let run = |rt: &mut Runtime<Workspace>, name: &'static str| {
        rt.update(move |s, tx| {
            assert!(actions::run(s, name, &Value::Null, tx), "{name} ran");
        });
        rt.frame(1. / 60.);
    };
    run(&mut rt, "pane.clear");
    run(&mut rt, "pane.reset");
    run(&mut rt, "pane.select_all");
    assert_eq!(
        sent_verbs(&sent),
        [
            ("block:b".to_owned(), "clear".to_owned()),
            ("block:b".to_owned(), "reset".to_owned()),
            ("block:b".to_owned(), "selection All".to_owned()),
        ]
    );
    run(&mut rt, "pane.copy_id");
    assert_eq!(clipboard_text(&clipboard), "block:b");

    // A terminal that is not live is not sent a reset to act on later.
    rt.update(|s, _| {
        let key = s.term_key("block:b");
        s.terminals[&key].connection.model.lock().unwrap().connected = false;
    });
    run(&mut rt, "pane.reset");
    run(&mut rt, "pane.clear");
    assert!(sent_verbs(&sent).is_empty());
    let notices: Vec<&str> = rt.state.notices.iter().map(|n| n.text.as_str()).collect();
    assert!(
        notices.contains(&"Only a live terminal can be reset.")
            && notices.contains(&"Only a live terminal can be cleared."),
        "{notices:?}"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_rebound_verb_shows_its_keys_in_the_menu() {
    let (mut rt, _host, sent, _clipboard) = split();
    rt.update(|s, _| {
        s.settings.shortcut_file.overrides = rex_rune::keymap::Overrides::parse(
            &serde_json::json!([{"keys":"ctrl+shift+a","action":"pane.select_all"}]),
        );
        s.local_keymap = actions::resolve(&s.settings.shortcut_file.overrides);
        s.keymap_generation += 1;
    });
    rt.update(|s, _| s.pane_menu("block:b".into(), Some((300., 200.))));
    rt.frame(1. / 60.);
    let detail = |rt: &Runtime<Workspace>, label: &str| {
        rt.state
            .entries()
            .into_iter()
            .find(|e| e.label == label)
            .unwrap()
            .detail
    };
    assert_eq!(detail(&rt, "Select all"), "Ctrl Shift A");
    assert_eq!(detail(&rt, "Copy"), "Ctrl Shift C");
    assert_eq!(detail(&rt, "Paste"), "Ctrl Shift V");
    assert_eq!(detail(&rt, "Reset terminal"), "", "unbound by default");
    press(&mut rt, Key::Escape);
    ctrl_shift(&mut rt, 'a');
    assert_eq!(
        sent_verbs(&sent),
        [("block:b".to_owned(), "selection All".to_owned())],
        "and the key runs it on the focused pane"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

/// Draws the window to `REX_TEST_SNAP_DIR/<name>.png` at 2×, when that directory is named.
/// One painter for a run: the glyph atlas uploads each glyph once.
/// With the directory named, run the draw tests one at a time (`-- draw --test-threads=1`):
/// the find bar's draws in the app's own fonts, and text is measured process-wide, so tests
/// running beside it measure in whichever fonts were loaded last.
pub(super) struct Camera {
    dir: std::path::PathBuf,
    gpu: rune::render::Gpu,
    painter: rune::render::Renderer,
}

impl Camera {
    pub(super) fn new() -> Option<Self> {
        let dir = std::path::PathBuf::from(std::env::var_os("REX_TEST_SNAP_DIR")?);
        let gpu = rune::render::Gpu::headless();
        let painter = rune::render::Renderer::new(&gpu.device, rune::render::OFFSCREEN_FORMAT);
        Some(Self { dir, gpu, painter })
    }

    pub(super) fn snap(&mut self, rt: &mut Runtime<Workspace>, name: &str) {
        let (w, h) = (
            (rt.viewport.0 * rt.env.scale).round() as u32,
            (rt.viewport.1 * rt.env.scale).round() as u32,
        );
        let (device, queue) = (&self.gpu.device, &self.gpu.queue);
        self.painter.upload_images(device, queue, &mut rt.images);
        self.painter
            .prepare(device, queue, &mut rt.text.atlas, &rt.instances, (w, h));
        let target = rune::render::Offscreen::new(device, w, h);
        let mut encoder = device.create_command_encoder(&Default::default());
        self.painter
            .draw(&mut encoder, &target.view, rt.clear_color());
        target.copy(&mut encoder);
        queue.submit([encoder.finish()]);
        let path = self.dir.join(format!("{name}.png"));
        rune::render::write_png(&path, w, h, &target.read(device)).unwrap();
    }
}

/// The pane menu and Commands as drawn, for a look at the groups (see `Camera`).
#[test]
fn the_pane_menu_and_commands_draw_their_groups() {
    let (mut ws, _host) = tabbed_workspace("split");
    let (sender, _sent) = flume::unbounded();
    framed_terminal(
        &mut ws,
        "block:b",
        &sender,
        b"$ cargo test\r\nrunning 12 tests\r\ntest result: ok",
    );
    framed_terminal(&mut ws, "block:c", &sender, b"$ tail -f server.log");
    let mut rt = Runtime::new(ws, view, (1024., 700.), Env::new(true, 2.));
    rt.on_key = Some(crate::app::input::on_key);
    let settle = |rt: &mut Runtime<Workspace>| {
        for _ in 0..90 {
            rt.frame(1. / 60.);
        }
    };
    settle(&mut rt);
    let mut camera = Camera::new();
    let mut snap = |rt: &mut Runtime<Workspace>, name: &str| {
        if let Some(camera) = &mut camera {
            camera.snap(rt, name);
        }
    };
    let grid = rect(&rt, "grid:block:b");
    let cell = crate::app::term::metrics(rt.state.settings.font_size).cell;
    let y = grid[1] + cell.1 * 1.5;
    rt.pointer_down(grid[0] + cell.0 * 0.5, y);
    rt.pointer_move(grid[0] + cell.0 * 8.5, y);
    rt.pointer_move(grid[0] + cell.0 * 15.5, y);
    rt.pointer_up(grid[0] + cell.0 * 15.5, y);
    settle(&mut rt);

    // No selection in the right pane: Copy is unavailable there.
    rt.update(|s, _| s.pane_menu("block:c".into(), Some((560., 96.))));
    settle(&mut rt);
    assert!(!enabled(&rt, "Copy"));
    let separators = |rt: &Runtime<Workspace>| {
        rt.elements()
            .iter()
            .filter(|e| !e.exiting && e.role == "separator")
            .count()
    };
    assert_eq!(separators(&rt), 4, "a line between each of the five groups");
    let menu = rect(&rt, "menu-surface");
    assert!(
        menu[1] >= 0. && menu[1] + menu[3] <= 700.,
        "the whole menu is on screen: {menu:?}"
    );
    snap(&mut rt, "pane-menu-no-selection");
    press(&mut rt, Key::Escape);

    // The left pane has one.
    rt.update(|s, _| s.pane_menu("block:b".into(), Some((120., 96.))));
    settle(&mut rt);
    assert!(enabled(&rt, "Copy"));
    snap(&mut rt, "pane-menu-selection");
    press(&mut rt, Key::Escape);

    rt.update(|s, _| s.picker(false));
    settle(&mut rt);
    snap(&mut rt, "commands");
    for c in "term".chars() {
        press(&mut rt, Key::Text(c.to_string()));
    }
    settle(&mut rt);
    snap(&mut rt, "commands-filtered");
    std::fs::remove_file(&rt.state.settings_path).ok();
}
