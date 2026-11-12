//! Settings → Terminal and what its cursor, keyboard and selection settings do in a pane.
use super::pane_verb_tests::{Camera, framed_terminal, rect};
use super::*;
use crate::app::{panel::SettingsSection, select_copy::memory, term};
use rex_linux::{
    connection::Command,
    settings::terminal::{CursorStyle, SelectionColor},
    terminal::{CursorShape, Renderer, selection},
};
use std::sync::Arc;

fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
}

/// Settings open on Terminal over a live pane, in a window that fits the dialog.
fn terminal_settings(prepare: impl FnOnce(&mut Workspace)) -> Runtime<Workspace> {
    let mut ws = workspace_hosts(2);
    prepare(&mut ws);
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    ws.open_section(SettingsSection::Terminal);
    let mut rt = runtime_at(ws, (1440., 1000.));
    settle(&mut rt);
    rt
}

fn element(rt: &Runtime<Workspace>, key: &str) -> rune::ElementInfo {
    rt.elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("missing {key}"))
}

/// Scroll the page until the control keyed `key` is well inside it.
fn reveal(rt: &mut Runtime<Workspace>, key: &str) {
    let (page, control) = (rect(rt, "settings-scroll:terminal"), rect(rt, key));
    let (x, middle) = (page[0] + page[2] / 2., page[1] + page[3] / 2.);
    let off = control[1] + control[3] / 2. - middle;
    if off.abs() > page[3] / 2. - 60. {
        rt.pointer_move(x, middle);
        rt.wheel(x, middle, -off);
        settle(rt);
    }
}

/// Flip the switch keyed `key`, wherever it is on the page.
fn flip(rt: &mut Runtime<Workspace>, key: &str) {
    reveal(rt, key);
    click(rt, key);
    settle(rt);
}

/// Choose `label` from the popup of the control keyed `control`.
fn choose(rt: &mut Runtime<Workspace>, control: &str, label: &str) {
    reveal(rt, control);
    click(rt, control);
    settle(rt);
    let row = rt
        .elements()
        .into_iter()
        .find(|e| !e.exiting && e.role == "menuitem" && e.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("no {label} in the list of {control}"))
        .rect;
    // The list is against its control wherever the page has scrolled to: right under it,
    // or right above.
    let (at, list) = (rect(rt, control), rect(rt, "menu-surface"));
    let (under, over) = (list[1] - (at[1] + at[3]), at[1] - (list[1] + list[3]));
    assert!(
        (0. ..=8.).contains(&under) || (0. ..=8.).contains(&over),
        "the list of {control} is against it: {list:?} and {at:?}"
    );
    rt.pointer_down(row[0] + row[2] / 2., row[1] + row[3] / 2.);
    rt.pointer_up(row[0] + row[2] / 2., row[1] + row[3] / 2.);
    settle(rt);
}

#[test]
fn the_page_is_headed_cards_in_order_with_a_preview_in_three_of_them() {
    let rt = terminal_settings(|_| {});
    let page = rect(&rt, "settings-scroll:terminal");
    let mut headings: Vec<(f32, String)> = rt
        .elements()
        .into_iter()
        .filter(|e| !e.exiting && e.role == "heading" && e.path.contains("settings-scroll:"))
        .map(|e| (e.rect[1], e.text.unwrap_or_default()))
        .collect();
    headings.sort_by(|a, b| a.0.total_cmp(&b.0));
    let names: Vec<&str> = headings.iter().map(|(_, name)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Typography",
            "Layout",
            "Colours",
            "Cursor",
            "Keyboard",
            "Selection",
            "Default shell",
            "Titles",
            "Path browser"
        ]
    );
    // Each preview is the first row of its card: under its heading, inside the page, an
    // image to assistive technology and never a control.
    for (key, heading) in [
        ("typography-preview", "Typography"),
        ("cursor-preview", "Cursor"),
        ("selection-preview", "Selection"),
    ] {
        let well = element(&rt, key);
        let top = headings.iter().find(|(_, name)| name == heading).unwrap().0;
        let next = headings
            .iter()
            .map(|(y, _)| *y)
            .find(|y| *y > top)
            .unwrap_or(f32::MAX);
        assert!(
            well.rect[1] > top && well.rect[1] < top + 40. && well.rect[1] < next,
            "{key} opens the {heading} card: {:?} under {top}",
            well.rect
        );
        assert!(
            well.rect[0] > page[0] && well.rect[0] + well.rect[2] < page[0] + page[2],
            "{key} is inside the page: {:?} in {page:?}",
            well.rect
        );
        assert_eq!(well.role, "img");
        assert!(!well.focusable);
        assert!(well.label.is_some_and(|label| label.contains("preview")));
    }
    // The settings file's path moved to About.
    let path = rt.state.settings_path.display().to_string();
    let shown = |rt: &Runtime<Workspace>| {
        rt.elements()
            .iter()
            .any(|e| !e.exiting && e.text.as_deref() == Some(path.as_str()))
    };
    assert!(!shown(&rt), "not on the Terminal page");
    let mut rt = rt;
    rt.update(|s, _| s.open_section(SettingsSection::About));
    settle(&mut rt);
    assert!(shown(&rt), "About names the file");
    assert_eq!(
        element(&rt, "about-settings-path").text.as_deref(),
        Some(path.as_str())
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_cursor_preview_draws_the_chosen_shape_and_the_choice_is_saved() {
    let mut rt = terminal_settings(|_| {});
    let cell = term::metrics(rt.state.settings.font_size).cell;
    let cursor = |rt: &Runtime<Workspace>| element(rt, "cursor-preview-sample/cursor");
    // Where the page stays while the styles are chosen.
    reveal(&mut rt, "cursor-style");
    let block = cursor(&rt);
    assert!(
        (block.rect[2] - cell.0).abs() < 0.5 && (block.rect[3] - cell.1).abs() < 0.5,
        "a block is one cell: {:?}",
        block.rect
    );
    assert!(block.fill.a > 0.);

    choose(&mut rt, "cursor-style", "Bar");
    assert_eq!(rt.state.settings.terminal.cursor_style, CursorStyle::Bar);
    assert!(matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::Settings)
    ));
    let bar = cursor(&rt);
    assert!(
        (bar.rect[2] - 2.).abs() < 0.5 && (bar.rect[3] - cell.1).abs() < 0.5,
        "a bar is 2 pt wide and a cell tall: {:?}",
        bar.rect
    );
    assert!(
        (bar.rect[0] - block.rect[0]).abs() < 0.5,
        "on the same cell"
    );

    choose(&mut rt, "cursor-style", "Underline");
    let under = cursor(&rt);
    assert!(
        (under.rect[2] - cell.0).abs() < 0.5 && (under.rect[3] - 2.).abs() < 0.5,
        "an underline is a cell wide and 2 pt tall: {:?}",
        under.rect
    );
    assert!(
        (under.rect[1] + under.rect[3] - block.rect[1] - block.rect[3]).abs() < 0.5,
        "at the cell's foot"
    );

    flip(&mut rt, "cursor-force");
    assert!(rt.state.settings.terminal.cursor_force);
    let saved = Settings::load(&rt.state.settings_path).unwrap();
    assert_eq!(saved.terminal.cursor_style, CursorStyle::Underline);
    assert!(saved.terminal.cursor_force);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

/// What a pane's worker was sent, besides selection events.
fn other_commands(rx: &flume::Receiver<HostCommand>) -> Vec<String> {
    rx.try_iter()
        .filter_map(|c| match c {
            HostCommand::Terminal { block, command, .. } => Some(match command {
                Command::Selection(_) => return None,
                Command::CursorStyle(shape) => format!("{block} cursor {shape:?}"),
                Command::Paste(text) => format!("{block} paste {text}"),
                Command::Mouse(mouse) => format!("{block} mouse {}", mouse["action"]),
                Command::Resize(..) => format!("{block} resize"),
                Command::TakeControl => format!("{block} take control"),
                Command::Key(_) | Command::KeyWithPrefix { .. } | Command::SendKey(_) => {
                    format!("{block} key")
                }
                _ => format!("{block} other"),
            }),
            HostCommand::Request { method, .. } => Some(format!("request {method}")),
            _ => Some("host command".into()),
        })
        .collect()
}

#[test]
fn a_cursor_style_reaches_every_pane_worker_once_and_claims_nothing() {
    let (mut ws, _host) = tabbed_workspace("split");
    let (sender, sent) = flume::unbounded();
    framed_terminal(&mut ws, "block:b", &sender, b"$ ");
    framed_terminal(&mut ws, "block:c", &sender, b"$ ");
    assert!(other_commands(&sent).is_empty());

    ws.set_cursor_style(CursorStyle::Bar);
    let mut told = other_commands(&sent);
    told.sort();
    assert_eq!(told, ["block:b cursor Bar", "block:c cursor Bar"]);
    // The same style again says nothing, and the block every pane starts with is never sent.
    ws.set_cursor_style(CursorStyle::Bar);
    assert!(other_commands(&sent).is_empty());
    ws.set_cursor_style(CursorStyle::Block);
    assert_eq!(other_commands(&sent).len(), 2);
    let mut fresh = term::Term::new(ws.terminals[&ws.term_key("block:b")].connection.clone());
    fresh.set_cursor_style(CursorShape::Block);
    assert!(
        other_commands(&sent).is_empty(),
        "a new pane at the default"
    );
    fresh.set_cursor_style(CursorShape::Underline);
    assert_eq!(other_commands(&sent), ["block:b cursor Underline"]);
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn force_style_draws_the_chosen_cursor_over_a_programs_own() {
    let (mut ws, _host) = tabbed_workspace("single");
    let (sender, _sent) = flume::unbounded();
    // The program asks for a steady block: `printf '\e[2 q'`.
    framed_terminal(&mut ws, "block:a", &sender, b"$ \x1b[2 q");
    ws.settings.terminal.cursor_style = CursorStyle::Bar;
    let mut rt = runtime(ws);
    settle(&mut rt);
    let cell = term::metrics(rt.state.settings.font_size).cell;
    let width = |rt: &Runtime<Workspace>| element(rt, "gblock:a/cursor").rect[2];
    assert!(
        (width(&rt) - cell.0).abs() < 0.5,
        "the program's block, without Force style"
    );
    rt.update(|s, _| s.settings.terminal.cursor_force = true);
    settle(&mut rt);
    assert!(
        (width(&rt) - 2.).abs() < 0.5,
        "a bar whatever was asked: {}",
        width(&rt)
    );
    // A pane without the keyboard still shows the hollow block.
    rt.update(|s, _| s.window_focused = false);
    settle(&mut rt);
    let hollow = element(&rt, "gblock:a/cursor");
    assert!((hollow.rect[2] - cell.0).abs() < 0.5 && hollow.fill.a == 0.);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

/// A pane with the terminal, renderer and selection controller that stand in for its worker.
struct Pane {
    rt: Runtime<Workspace>,
    sent: flume::Receiver<HostCommand>,
    terminal: ghostty_vt::Terminal,
    renderer: Renderer,
    controller: selection::Controller,
    clipboard: rune::clipboard::MemoryClipboard,
    /// What the pane was sent besides selection events, so far.
    others: Vec<String>,
}

impl Pane {
    fn new(output: &[u8], prepare: impl FnOnce(&mut Workspace)) -> Self {
        let (mut ws, _host) = tabbed_workspace("single");
        prepare(&mut ws);
        let (sender, sent) = flume::unbounded();
        let terminal = framed_terminal(&mut ws, "block:a", &sender, output);
        let mut rt = runtime(ws);
        settle(&mut rt);
        let clipboard = rune::clipboard::memory();
        clipboard.set_text("sentinel");
        rt.use_clipboard(&clipboard);
        memory::set("primary sentinel".into());
        Self {
            rt,
            sent,
            terminal,
            renderer: Renderer::new().unwrap(),
            controller: selection::Controller::new().unwrap(),
            clipboard,
            others: Vec::new(),
        }
    }

    /// The point `col` cells along the first row, at its middle height.
    fn at(&self, col: f32) -> (f32, f32) {
        let grid = rect(&self.rt, "grid:block:a");
        let cell = term::metrics(self.rt.state.settings.font_size).cell;
        (grid[0] + cell.0 * col, grid[1] + cell.1 / 2.)
    }

    /// Press, release and let the handlers run.
    fn click(&mut self, col: f32) {
        let (x, y) = self.at(col + 0.5);
        self.rt.pointer_down(x, y);
        self.rt.pointer_up(x, y);
        self.rt.frame(1. / 60.);
    }

    fn middle_click(&mut self, col: f32) {
        let (x, y) = self.at(col + 0.5);
        self.rt.button_down(x, y, rune::PointerButton::Middle);
        self.rt.frame(1. / 60.);
        self.rt.button_up(x, y, rune::PointerButton::Middle);
        self.rt.frame(1. / 60.);
    }

    /// Drag across cells `from` to `to` of the first row, both whole, and let go.
    fn drag(&mut self, from: f32, to: f32) {
        let (start, end) = (self.at(from + 0.1), self.at(to + 0.9));
        self.rt.pointer_down(start.0, start.1);
        self.rt
            .pointer_move((start.0 + end.0) / 2., (start.1 + end.1) / 2.);
        self.rt.pointer_move(end.0, end.1);
        self.rt.pointer_up(end.0, end.1);
        self.rt.frame(1. / 60.);
    }

    /// The worker's turn: apply the selection events sent so far and publish the frame.
    fn worker(&mut self) {
        for command in self.sent.try_iter() {
            match command {
                HostCommand::Terminal {
                    command: Command::Selection(input),
                    ..
                } => self.controller.apply(&self.terminal, input).unwrap(),
                HostCommand::Terminal { command, .. } => self.others.push(match command {
                    Command::Paste(text) => format!("paste {text}"),
                    Command::Mouse(mouse) => format!("mouse {}", mouse["action"]),
                    Command::Resize(..) => "resize".into(),
                    Command::TakeControl => "take control".into(),
                    _ => "other".into(),
                }),
                HostCommand::Request { method, .. } => {
                    self.others.push(format!("request {method}"))
                }
                _ => self.others.push("host command".into()),
            }
        }
        let mut frame = self.renderer.frame(&self.terminal).unwrap();
        frame.selection_revision = self.controller.revision;
        let key = self.rt.state.term_key("block:a");
        self.rt.state.terminals[&key]
            .connection
            .model
            .lock()
            .unwrap()
            .frame = Some(Arc::new(frame));
        self.rt.update(move |s, tx| s.terminal_woke(&key, tx));
        self.rt.frame(1. / 60.);
    }

    fn clipboard(&self) -> String {
        self.clipboard
            .text()
            .as_deref()
            .cloned()
            .unwrap_or_default()
    }
}

#[test]
fn selecting_copies_nothing_until_copy_on_selection_is_on() {
    let mut pane = Pane::new(b"alpha beta gamma", |_| {});
    pane.drag(0., 4.);
    pane.worker();
    assert!(pane.rt.state.terminals.values().all(|t| t.has_selection()));
    assert_eq!(pane.clipboard(), "sentinel");
    assert_eq!(memory::get(), "primary sentinel");
    assert!(pane.others.is_empty(), "{:?}", pane.others);
    std::fs::remove_file(&pane.rt.state.settings_path).ok();
}

#[test]
fn copy_on_selection_copies_a_finished_selection_once_its_text_arrives() {
    let mut pane = Pane::new(b"alpha beta gamma", |ws| {
        ws.settings.terminal.copy_on_selection = true;
    });
    pane.drag(0., 4.);
    assert_eq!(
        pane.clipboard(),
        "sentinel",
        "the text comes with the worker's frame, not before"
    );
    pane.worker();
    assert_eq!(pane.clipboard(), "alpha");
    assert_eq!(memory::get(), "primary sentinel", "its own setting");

    // Once: a later frame of the same selection copies nothing again.
    pane.clipboard.set_text("something else since");
    pane.worker();
    assert_eq!(pane.clipboard(), "something else since");

    // A double-click selects a word, and copies it.
    pane.click(7.);
    pane.click(7.);
    pane.worker();
    assert_eq!(pane.clipboard(), "beta");

    // A click alone selects nothing, so the clipboard keeps what it had.
    pane.rt.frame(1.);
    pane.click(12.);
    pane.worker();
    assert!(!pane.rt.state.terminals.values().any(|t| t.has_selection()));
    assert_eq!(pane.clipboard(), "beta");

    // Select all from the menu is not a pointer selection: nothing is copied.
    pane.clipboard.set_text("kept");
    pane.rt.update(|s, _| {
        let key = s.term_key("block:a");
        s.terminals.get_mut(&key).unwrap().select_all();
    });
    pane.worker();
    assert_eq!(pane.clipboard(), "kept");

    // Selecting is no input to the program and no request to the server: the pane was
    // sent selection events only, so nothing claimed it or resized it.
    assert!(pane.others.is_empty(), "{:?}", pane.others);
    std::fs::remove_file(&pane.rt.state.settings_path).ok();
}

#[test]
fn a_selection_changed_before_its_text_arrives_is_not_copied() {
    let mut pane = Pane::new(b"alpha beta gamma", |ws| {
        ws.settings.terminal.copy_on_selection = true;
    });
    pane.drag(0., 4.);
    // Before the worker answers, a click clears the selection.
    pane.rt.frame(1.);
    pane.click(12.);
    pane.worker();
    assert_eq!(pane.clipboard(), "sentinel");
    std::fs::remove_file(&pane.rt.state.settings_path).ok();
}

#[test]
fn the_primary_selection_is_set_by_selecting_and_pasted_by_a_middle_click() {
    let mut pane = Pane::new(b"alpha beta gamma", |ws| {
        ws.settings.terminal.primary_selection = true;
    });
    pane.drag(0., 4.);
    pane.worker();
    assert_eq!(memory::get(), "alpha");
    assert_eq!(
        pane.clipboard(),
        "sentinel",
        "the clipboard has its own setting"
    );

    let (x, y) = pane.at(20.5);
    pane.rt.button_down(x, y, rune::PointerButton::Middle);
    pane.rt.frame(1. / 60.);
    assert!(
        other_commands(&pane.sent).is_empty(),
        "nothing on the press"
    );
    pane.rt.button_up(x, y, rune::PointerButton::Middle);
    pane.rt.frame(1. / 60.);
    assert_eq!(other_commands(&pane.sent), ["block:a paste alpha"]);

    // An empty primary selection pastes nothing.
    memory::set(String::new());
    pane.middle_click(20.);
    assert!(other_commands(&pane.sent).is_empty());
    std::fs::remove_file(&pane.rt.state.settings_path).ok();
}

#[test]
fn a_middle_click_goes_to_a_program_that_tracks_the_mouse_and_nowhere_when_off() {
    // Tracking (mode 1000): the program gets the button, as before, and nothing is pasted.
    let mut pane = Pane::new(b"\x1b[?1000hmenu", |ws| {
        ws.settings.terminal.primary_selection = true;
    });
    memory::set("not for a program".into());
    pane.middle_click(2.);
    let sent = other_commands(&pane.sent);
    assert!(
        sent.iter().any(|c| c.contains("mouse")) && !sent.iter().any(|c| c.contains("paste")),
        "{sent:?}"
    );
    std::fs::remove_file(&pane.rt.state.settings_path).ok();

    // The setting off, the default: a middle-click on a pane does nothing at all.
    let mut pane = Pane::new(b"plain", |_| {});
    memory::set("unused".into());
    pane.middle_click(2.);
    assert!(other_commands(&pane.sent).is_empty());
    std::fs::remove_file(&pane.rt.state.settings_path).ok();
}

#[test]
fn selection_colours_and_corners_follow_their_settings_in_panes_and_the_preview() {
    let mut rt = terminal_settings(|_| {});
    assert_eq!(
        rt.state.term_colors().selection,
        term::SelectionLook {
            foreground: SelectionColor::Theme,
            background: SelectionColor::Theme,
            rounded: false,
        }
    );
    choose(&mut rt, "selection-background", "Cell foreground");
    choose(&mut rt, "selection-foreground", "Cell background");
    flip(&mut rt, "selection-rounded");
    flip(&mut rt, "copy-on-selection");
    flip(&mut rt, "primary-selection");
    let look = rt.state.term_colors().selection;
    assert_eq!(look.background, SelectionColor::CellForeground);
    assert_eq!(look.foreground, SelectionColor::CellBackground);
    assert!(look.rounded);
    let saved = Settings::load(&rt.state.settings_path).unwrap().terminal;
    assert_eq!(saved, rt.state.settings.terminal);
    assert!(saved.copy_on_selection && saved.primary_selection && saved.selection_rounded);
    assert!(matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::Settings)
    ));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn key_sequence_hints_can_be_turned_off_without_changing_the_sequence() {
    use rune::{PhysicalInput, PhysicalKeyCode as Code};
    let pending = |hints: bool, hidden: bool| {
        let mut ws = workspace();
        ws.settings.terminal.key_sequence_hints = hints;
        if hidden {
            ws.settings.interface.toolbar = rex_linux::settings::interface::ToolbarBehavior::Hidden;
        }
        ws.settings.shortcut_file.overrides = rex_linux::keymap::Overrides::parse(
            &serde_json::json!([{"keys":"ctrl+b>c","action":"client.tab.new"}]),
        );
        ws.local_keymap = crate::app::actions::resolve(&ws.settings.shortcut_file.overrides);
        ws.keymap_generation += 1;
        let mut rt = runtime(ws);
        let mods = Mods {
            ctrl: true,
            ..Mods::default()
        };
        rt.physical_key_input(
            PhysicalInput::Key {
                key: Code::KeyB,
                mods,
                pressed: true,
                repeat: false,
            },
            KeyEvent {
                key: Key::Command('b'),
                mods,
            },
            None,
        );
        rt.frame(1. / 60.);
        let state = (
            rt.state.bindings.sequence.depth,
            has(&rt, "pending-sequence"),
        );
        std::fs::remove_file(&rt.state.settings_path).ok();
        state
    };
    for hidden in [false, true] {
        assert_eq!(pending(true, hidden), (1, true), "shown by default");
        assert_eq!(
            pending(false, hidden),
            (1, false),
            "the sequence still waits"
        );
    }
}

#[test]
fn held_selection_and_lost_focus_never_copy() {
    for cancel in [false, true] {
        let mut pane = Pane::new(b"alpha beta", |s| {
            s.settings.terminal.copy_on_selection = true;
            s.settings.terminal.primary_selection = true;
        });
        let (a, b) = (pane.at(0.1), pane.at(4.9));
        pane.rt.pointer_down(a.0, a.1);
        pane.rt.pointer_move(b.0, b.1);
        pane.worker();
        assert_eq!(pane.clipboard(), "sentinel", "not before release");
        assert_eq!(memory::get(), "primary sentinel");
        if cancel {
            pane.rt.update(|s, _| s.window_focus(false));
            pane.rt.pointer_move(b.0 + 5., b.1);
            pane.rt.frame(1. / 60.);
            assert!(!pane.rt.state.terminals.values().any(|t| t.selecting));
        }
        pane.rt.pointer_up(b.0, b.1);
        if !cancel {
            // The release happened, but focus was lost before the worker replied.
            pane.rt.update(|s, _| s.window_focus(false));
        }
        pane.worker();
        assert_eq!(pane.clipboard(), "sentinel");
        assert_eq!(memory::get(), "primary sentinel");
    }
}

#[test]
fn primary_drag_and_shift_over_mouse_program_never_paste() {
    let mut pane = Pane::new(b"plain", |s| s.settings.terminal.primary_selection = true);
    let (a, b) = (pane.at(1.), pane.at(10.));
    pane.rt.button_down(a.0, a.1, rune::PointerButton::Middle);
    pane.rt.pointer_move(b.0, b.1);
    pane.rt.pointer_move(a.0, a.1);
    pane.rt.button_up(a.0, a.1, rune::PointerButton::Middle);
    assert!(
        other_commands(&pane.sent).is_empty(),
        "dragging back is not a click"
    );

    let mut pane = Pane::new(b"\x1b[?1000hmenu", |s| {
        s.settings.terminal.primary_selection = true
    });
    pane.rt.physical_input(rune::PhysicalInput::Modifiers(Mods {
        shift: true,
        ..Default::default()
    }));
    pane.middle_click(2.);
    assert!(
        !other_commands(&pane.sent)
            .iter()
            .any(|c| c.contains("paste"))
    );
}

#[test]
fn terminal_controls_are_indexed_and_reachable_at_minimum_size() {
    for size in [(1024., 700.), (640., 480.)] {
        let mut ws = workspace_hosts(1);
        ws.perform(Intent::Settings, &mut rune::Tx::default());
        let mut rt = runtime_at(ws, size);
        settle(&mut rt);
        for (query, label, key) in [
            ("cursor style", "Cursor style", "cursor-style"),
            ("force style", "Force style", "cursor-force"),
            (
                "copy on selection",
                "Copy on selection",
                "copy-on-selection",
            ),
            (
                "primary selection",
                "Primary selection",
                "primary-selection",
            ),
            ("selection foreground", "Foreground", "selection-foreground"),
            ("selection background", "Background", "selection-background"),
            ("rounded corners", "Rounded corners", "selection-rounded"),
            ("key sequence", "Show key sequence hints", "sequence-hints"),
            (
                "recently used paths",
                "Show recently used paths",
                "show-recent-paths",
            ),
            (
                "generated titles",
                "Prefer generated titles",
                "generated-titles",
            ),
        ] {
            rt.update(|s, tx| {
                s.settings_nav.search.text = query.into();
                s.settings_search_changed();
                let index = s
                    .settings_nav
                    .hits()
                    .iter()
                    .position(|h| h.label == label && h.place == "Terminal")
                    .unwrap_or_else(|| panic!("missing {query}"));
                s.open_settings_hit(index, tx);
            });
            settle(&mut rt);
            let (control, scroll) = (rect(&rt, key), rect(&rt, "settings-scroll:terminal"));
            assert!(
                control[0] >= scroll[0] && control[0] + control[2] <= scroll[0] + scroll[2] + 0.5,
                "{size:?} {key} horizontal: {control:?} in {scroll:?}"
            );
            assert!(
                control[1] >= scroll[1] && control[1] + control[3] <= scroll[1] + scroll[3] + 0.5,
                "{size:?} {key} vertical: {control:?} in {scroll:?}"
            );
        }
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

/// The app's own fonts, for the camera: the previews are then on the terminal's grid as in
/// the app, where a headless runtime sets them in Inter. Loading them changes the cell size
/// for the whole process, so take the pictures in a run of this test alone.
fn app_fonts() -> rune::text::TextSystem {
    let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
    rune::text::TextSystem::new(fonts.files, fonts.families)
}

/// A window at 2×, in the app's fonts while the camera is on.
fn studio(ws: Workspace, camera: bool) -> Runtime<Workspace> {
    let size = if std::env::var_os("REX_M25_MINIMUM").is_some() {
        (640., 480.)
    } else {
        (1024., 700.)
    };
    let env = Env::new(true, 2.);
    let mut rt = if camera {
        Runtime::with_text(ws, view, size, env, app_fonts())
    } else {
        Runtime::new(ws, view, size, env)
    };
    settle(&mut rt);
    rt
}

/// The Terminal page as drawn from top to bottom, then with a bar cursor and an inverted,
/// square selection, and a selection of several rows in a pane with each corner style (see
/// `Camera`; run it alone: `-- the_terminal_page_draws --test-threads=1`).
#[test]
fn the_terminal_page_draws_its_cards_and_previews() {
    let on = Camera::new().is_some();
    if on {
        // Before anything is measured.
        drop(app_fonts());
    }
    // A camera per window: a painter keeps the glyphs of the first window it drew.
    let snap = |camera: &mut Option<Camera>, rt: &mut Runtime<Workspace>, name: &str| {
        if let Some(camera) = camera {
            camera.snap(rt, name);
        }
    };
    let page = |prepare: fn(&mut Workspace)| {
        let mut ws = workspace_hosts(2);
        prepare(&mut ws);
        ws.perform(Intent::Settings, &mut rune::Tx::default());
        ws.open_section(SettingsSection::Terminal);
        studio(ws, on)
    };
    let scroll_by = |rt: &mut Runtime<Workspace>, points: f32| {
        let scroll = rect(rt, "settings-scroll:terminal");
        let (x, y) = (scroll[0] + scroll[2] / 2., scroll[1] + scroll[3] / 2.);
        rt.pointer_move(x, y);
        rt.wheel(x, y, -points);
        settle(rt);
    };
    let mut rt = page(|_| {});
    let mut camera = Camera::new();
    snap(&mut camera, &mut rt, "terminal-page-1");
    for key in ["cursor-style", "selection-background", "generated-titles"] {
        reveal(&mut rt, key);
        snap(&mut camera, &mut rt, &format!("terminal-page-{key}"));
    }
    scroll_by(&mut rt, -10000.);
    for shot in 2..=4 {
        scroll_by(&mut rt, 420.);
        snap(&mut camera, &mut rt, &format!("terminal-page-{shot}"));
    }
    std::fs::remove_file(&rt.state.settings_path).ok();

    let mut rt = page(|ws| {
        let t = &mut ws.settings.terminal;
        t.cursor_style = CursorStyle::Bar;
        t.selection_background = SelectionColor::CellForeground;
        t.selection_rounded = false;
    });
    scroll_by(&mut rt, 330.);
    snap(&mut Camera::new(), &mut rt, "terminal-page-bar-inverted");
    assert!(has(&rt, "selection-preview") || has(&rt, "cursor-preview"));
    std::fs::remove_file(&rt.state.settings_path).ok();

    for (control, label) in [
        ("cursor-style", "Bar"),
        ("selection-background", "Cell foreground"),
        ("selection-foreground", "Cell background"),
    ] {
        let mut rt = page(|_| {});
        reveal(&mut rt, control);
        click(&mut rt, control);
        settle(&mut rt);
        snap(&mut Camera::new(), &mut rt, &format!("picker-{control}"));
        press(&mut rt, Key::Escape);
        choose(&mut rt, control, label);
        std::fs::remove_file(&rt.state.settings_path).ok();
    }

    // A list opened from a control on the scrolled page, and the path's new home.
    let mut rt = page(|_| {});
    let mut camera = Camera::new();
    reveal(&mut rt, "selection-background");
    click(&mut rt, "selection-background");
    settle(&mut rt);
    snap(&mut camera, &mut rt, "terminal-page-list");
    press(&mut rt, Key::Escape);
    rt.update(|s, _| s.open_section(SettingsSection::About));
    settle(&mut rt);
    snap(&mut camera, &mut rt, "about-settings-path");
    std::fs::remove_file(&rt.state.settings_path).ok();

    // A large font: the samples grow with the terminals and are cut at the well's edge.
    let mut rt = page(|ws| ws.settings.font_size = 32.);
    snap(&mut Camera::new(), &mut rt, "terminal-page-large-font");
    let (well, sample) = (
        rect(&rt, "typography-preview"),
        rect(&rt, "typography-preview-sample"),
    );
    assert!(
        sample[1] >= well[1] && sample[1] + sample[3] <= well[1] + well[3],
        "every line is inside the well: {sample:?} in {well:?}"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();

    for (name, style, force) in [
        ("pane-cursor-program-block", CursorStyle::Bar, false),
        ("pane-cursor-forced-bar", CursorStyle::Bar, true),
        ("pane-cursor-forced-underline", CursorStyle::Underline, true),
    ] {
        let (mut ws, _host) = tabbed_workspace("single");
        ws.settings.terminal.cursor_style = style;
        ws.settings.terminal.cursor_force = force;
        let (sender, _sent) = flume::unbounded();
        framed_terminal(
            &mut ws,
            "block:a",
            &sender,
            b"Program requested a block (CSI 2 SP q)\r\n$ \x1b[2 q",
        );
        let mut rt = studio(ws, on);
        snap(&mut Camera::new(), &mut rt, name);
        std::fs::remove_file(&rt.state.settings_path).ok();
    }

    // A selection from the middle of one row to the middle of the third, in a pane.
    for (name, rounded, background) in [
        ("pane-selection-rounded", true, SelectionColor::Theme),
        ("pane-selection-square", false, SelectionColor::Theme),
        (
            "pane-selection-inverted",
            true,
            SelectionColor::CellForeground,
        ),
    ] {
        let (mut ws, _host) = tabbed_workspace("single");
        ws.settings.terminal.selection_rounded = rounded;
        ws.settings.terminal.selection_background = background;
        let (sender, _sent) = flume::unbounded();
        framed_terminal(
            &mut ws,
            "block:a",
            &sender,
            b"$ git status\r\nOn branch \x1b[35mfeature/tabs\x1b[0m\r\nnothing to commit, \x1b[32mworking tree clean\x1b[0m\r\n$ ",
        );
        let mut rt = studio(ws, on);
        let grid = rect(&rt, "grid:block:a");
        let cell = term::metrics(rt.state.settings.font_size).cell;
        let at = |col: f32, row: f32| (grid[0] + cell.0 * col, grid[1] + cell.1 * (row + 0.5));
        let (from, to) = (at(6.2, 0.), at(28.8, 2.));
        rt.pointer_down(from.0, from.1);
        rt.pointer_move(to.0 - 20., to.1);
        rt.pointer_move(to.0, to.1);
        rt.pointer_up(to.0, to.1);
        settle(&mut rt);
        assert!(rt.state.terminals.values().all(|t| t.has_selection()));
        snap(&mut Camera::new(), &mut rt, name);
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}
