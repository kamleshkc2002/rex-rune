//! The find bar: its count and buttons, Find next and Find previous with the bar closed, and
//! the corner it takes so that the selected match stays in view.
use super::pane_verb_tests::{Camera, rect};
use super::*;
use crate::app::{actions, find, term};
use rex_rune::{
    connection::{Command, Connection, Model, TerminalSender},
    terminal::{Renderer, search::Controller},
};
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// A live terminal on one pane, with what stands in for the pane's worker: the terminal, its
/// search, and the commands the pane was sent.
struct Searched {
    key: (crate::app::workspace::HostId, String),
    terminal: ghostty_vt::Terminal,
    search: Controller,
    model: Arc<Mutex<Model>>,
    sent: flume::Receiver<HostCommand>,
}

impl Searched {
    /// Make `block` a live terminal of `cols` by `rows` showing `output`.
    fn new(ws: &mut Workspace, block: &str, (cols, rows): (u16, u16), output: &[u8]) -> Self {
        let (sender, sent) = flume::unbounded();
        let connection = Connection {
            model: Default::default(),
            commands: TerminalSender::new(sender, "s".into(), block.into()),
            wake: flume::unbounded().1,
        };
        let mut terminal = ghostty_vt::Terminal::new(ghostty_vt::TerminalOptions {
            cols,
            rows,
            ..Default::default()
        })
        .unwrap();
        terminal.vt_write(output);
        let model = connection.model.clone();
        {
            let mut m = model.lock().unwrap();
            m.connected = true;
            m.control_connected = true;
            m.frame = Some(Arc::new(Renderer::new().unwrap().frame(&terminal).unwrap()));
        }
        let key = ws.term_key(block);
        let mut term = term::Term::new(connection);
        term.refresh();
        ws.terminals.insert(key.clone(), term);
        Self {
            key,
            terminal,
            search: Controller::default(),
            model,
            sent,
        }
    }

    /// Do what the pane's worker does with the commands sent since the last call: search,
    /// step, and hand the pane a frame with the results. Returns the commands, as words.
    fn work(&mut self, rt: &mut Runtime<Workspace>) -> Vec<String> {
        let mut words = Vec::new();
        for command in self.sent.try_iter() {
            let HostCommand::Terminal { command, .. } = command else {
                continue;
            };
            match command {
                Command::Search(needle) => {
                    words.push(format!("search {needle}"));
                    self.search
                        .set_needle(Some(&self.terminal), needle)
                        .unwrap();
                    while self.search.update(&self.terminal).unwrap() {}
                }
                Command::SearchStep(older) => {
                    words.push(format!("step older={older}"));
                    self.search.step(&self.terminal, older).unwrap();
                }
                Command::Key(_) => words.push("key".into()),
                _ => {}
            }
        }
        let mut frame = Renderer::new().unwrap().frame(&self.terminal).unwrap();
        frame.search = self.search.results(&self.terminal).unwrap();
        self.model.lock().unwrap().frame = Some(Arc::new(frame));
        let key = self.key.clone();
        rt.update(move |s, _| s.refresh_terminal(&key));
        settle(rt);
        words
    }
}

fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
}

fn element(rt: &Runtime<Workspace>, key: &str) -> rune::ElementInfo {
    rt.elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("missing {key}"))
}

fn type_text(rt: &mut Runtime<Workspace>, text: &str) {
    for c in text.chars() {
        press(rt, Key::Text(c.to_string()));
    }
}

fn shift_press(rt: &mut Runtime<Workspace>, key: Key) {
    rt.key(KeyEvent {
        key,
        mods: Mods {
            shift: true,
            ..Mods::default()
        },
    });
    for _ in 0..3 {
        rt.frame(1. / 60.);
    }
}

fn run(rt: &mut Runtime<Workspace>, action: &'static str) {
    rt.update(move |s, tx| assert!(actions::run(s, action, &Value::Null, tx)));
    for _ in 0..3 {
        rt.frame(1. / 60.);
    }
}

fn field_focused(rt: &Runtime<Workspace>) -> bool {
    focused(rt).is_some_and(|path| path.contains("search-field:block:a"))
}

/// The count's words and colour, when the bar shows one.
fn count(rt: &Runtime<Workspace>) -> Option<(String, rune::Color)> {
    rt.elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some("find-count:block:a"))
        .map(|e| (e.text.unwrap(), e.text_color.unwrap()))
}

fn help(rt: &Runtime<Workspace>, key: &str) -> String {
    element(rt, key).help.unwrap_or_default()
}

/// One pane, `block:a`, showing `output` in a terminal as wide as the pane.
fn single(output: impl Fn(usize) -> Vec<u8>) -> (Runtime<Workspace>, Searched) {
    let (mut ws, _host) = tabbed_workspace("single");
    let cols = columns(&ws);
    let pane = Searched::new(&mut ws, "block:a", (cols as u16, 24), &output(cols));
    let mut rt = runtime(ws);
    settle(&mut rt);
    (rt, pane)
}

/// The columns a single pane has in a 1024 pt window with the sidebar closed.
fn columns(ws: &Workspace) -> usize {
    let cell = term::metrics(ws.settings.font_size).cell;
    ((1024. - 2. * ws.settings.pane_padding) / cell.0).floor() as usize
}

fn two_alphas(_: usize) -> Vec<u8> {
    b"alpha one\r\nalpha two\r\n$ ".to_vec()
}

#[test]
fn the_bar_counts_matches_and_its_buttons_step_and_close() {
    use crate::app::theme::{danger, muted, rgb};
    let (mut rt, mut pane) = single(two_alphas);
    ctrl_shift(&mut rt, 'f');
    assert!(field_focused(&rt));

    // Field, Find next, Find previous, Close: three 24 pt buttons inside the bar, in order.
    let bar = rect(&rt, "search:block:a");
    assert_eq!((bar[2], bar[3]), (find::WIDTH, find::HEIGHT));
    let mut left = rect(&rt, "search-field:block:a")[0];
    for (key, label) in [
        ("find-next:block:a", "Find next"),
        ("find-previous:block:a", "Find previous"),
        ("find-close:block:a", "Close search"),
    ] {
        let button = element(&rt, key);
        assert_eq!(button.role, "button");
        assert_eq!(button.label.as_deref(), Some(label));
        let [x, y, w, h] = button.rect;
        assert_eq!((w, h), (24., 24.), "{key}: a 24 pt target");
        assert!(x > left, "{key} follows what is before it");
        assert!(
            y >= bar[1] && y + h <= bar[1] + bar[3] && x + w <= bar[0] + bar[2],
            "{key} is inside the bar"
        );
        left = x;
    }

    // Nothing typed: no count, nothing to step through, and Close says how the keyboard does it.
    assert_eq!(count(&rt), None);
    assert_eq!(help(&rt, "find-next:block:a"), "Find next · no matches");
    assert_eq!(
        help(&rt, "find-previous:block:a"),
        "Find previous · no matches"
    );
    assert_eq!(help(&rt, "find-close:block:a"), "Close search · Esc");
    click(&mut rt, "find-next:block:a");
    click(&mut rt, "find-previous:block:a");
    assert!(pane.work(&mut rt).is_empty(), "the arrows do nothing yet");
    assert!(field_focused(&rt), "and leave the keyboard in the field");

    // Two matches: the newest is selected, the arrows step, the field keeps the keyboard.
    type_text(&mut rt, "alpha");
    pane.work(&mut rt);
    assert_eq!(count(&rt), Some(("1/2".into(), rgb(muted()))));
    assert_eq!(help(&rt, "find-next:block:a"), "Find next · Enter");
    assert_eq!(
        help(&rt, "find-previous:block:a"),
        "Find previous · Shift Enter"
    );
    click(&mut rt, "find-next:block:a");
    assert_eq!(pane.work(&mut rt), ["step older=true"]);
    assert_eq!(count(&rt), Some(("2/2".into(), rgb(muted()))));
    click(&mut rt, "find-previous:block:a");
    assert_eq!(pane.work(&mut rt), ["step older=false"]);
    assert_eq!(count(&rt), Some(("1/2".into(), rgb(muted()))));
    assert!(field_focused(&rt));
    // Enter and Shift+Enter are the same steps.
    press(&mut rt, Key::Enter);
    shift_press(&mut rt, Key::Enter);
    assert_eq!(pane.work(&mut rt), ["step older=true", "step older=false"]);

    // No matches: said in DANGER, the arrows wait, and Close is still there.
    type_text(&mut rt, "x");
    assert_eq!(pane.work(&mut rt), ["search alphax"]);
    assert_eq!(count(&rt), Some(("No matches".into(), rgb(danger()))));
    assert_eq!(help(&rt, "find-next:block:a"), "Find next · no matches");
    click(&mut rt, "find-next:block:a");
    click(&mut rt, "find-previous:block:a");
    assert!(pane.work(&mut rt).is_empty());

    click(&mut rt, "find-close:block:a");
    assert!(!has(&rt, "search:block:a"));
    assert_eq!(pane.work(&mut rt), ["search "], "Close ends the search");
    assert_eq!(focused(&rt), None, "the terminal has the keyboard again");
    press(&mut rt, Key::Text("q".into()));
    assert_eq!(pane.work(&mut rt), ["key"]);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn find_next_and_previous_run_with_the_bar_closed() {
    let (mut rt, mut pane) = single(two_alphas);

    // Nothing searched here yet: Find next opens the field for typing.
    run(&mut rt, find::NEXT);
    assert!(field_focused(&rt));
    assert!(pane.work(&mut rt).is_empty());
    // An empty field has nothing to remember.
    press(&mut rt, Key::Escape);
    assert_eq!(pane.work(&mut rt), ["search "]);
    assert_eq!(rt.state.terminals[&pane.key].find.last, "");

    ctrl_shift(&mut rt, 'f');
    type_text(&mut rt, "alpha");
    pane.work(&mut rt);
    press(&mut rt, Key::Escape);
    assert_eq!(pane.work(&mut rt), ["search "]);
    assert!(!has(&rt, "search:block:a"));
    assert_eq!(rt.state.terminals[&pane.key].find.last, "alpha");

    // The bar is closed: Find next brings it back on that text, at the newest match, and the
    // keyboard stays with the program.
    run(&mut rt, find::NEXT);
    assert_eq!(pane.work(&mut rt), ["search alpha"]);
    assert!(has(&rt, "search:block:a"));
    assert_eq!(focused(&rt), None);
    assert_eq!(
        rt.state.terminals[&pane.key].search.as_ref().unwrap().text,
        "alpha"
    );
    assert_eq!(count(&rt).unwrap().0, "1/2");
    press(&mut rt, Key::Text("q".into()));
    assert_eq!(
        pane.work(&mut rt),
        ["key"],
        "typing still reaches the program"
    );

    // With the bar open and the keyboard in the terminal, both step.
    run(&mut rt, find::NEXT);
    assert_eq!(pane.work(&mut rt), ["step older=true"]);
    assert_eq!(count(&rt).unwrap().0, "2/2");
    run(&mut rt, find::PREVIOUS);
    assert_eq!(pane.work(&mut rt), ["step older=false"]);
    assert_eq!(count(&rt).unwrap().0, "1/2");
    assert_eq!(focused(&rt), None);

    // Find previous brings a closed bar back too.
    click(&mut rt, "find-close:block:a");
    pane.work(&mut rt);
    run(&mut rt, find::PREVIOUS);
    assert_eq!(pane.work(&mut rt), ["search alpha"]);
    assert_eq!(count(&rt).unwrap().0, "1/2");

    // An open bar with nothing typed gets the keyboard instead.
    click(&mut rt, "search-field:block:a");
    for _ in 0..5 {
        press(&mut rt, Key::Backspace);
    }
    click(&mut rt, "grid:block:a");
    assert_eq!(focused(&rt), None);
    pane.work(&mut rt);
    run(&mut rt, find::NEXT);
    assert!(field_focused(&rt));
    assert!(pane.work(&mut rt).is_empty());
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_find_actions_are_bindable_local_only_and_in_commands() {
    let (mut rt, mut pane) = single(two_alphas);
    for name in [find::NEXT, find::PREVIOUS] {
        let spec = actions::find(name).unwrap();
        assert!(
            matches!(spec.remote(), actions::Remote::Never(_)),
            "{name} is never remote"
        );
        assert!(spec.repeats(), "{name} repeats while its key is held");
        assert!(spec.accepts(&Value::Null));
        assert!(!spec.accepts(&serde_json::json!({"older": true})));
    }
    assert_eq!(actions::find(find::NEXT).unwrap().title(), "Find next");
    assert_eq!(
        actions::find(find::PREVIOUS).unwrap().title(),
        "Find previous"
    );
    // Neither takes a key from terminal programs by default.
    assert!(
        rex_rune::keymap::defaults()
            .bindings
            .iter()
            .all(|b| b.action != find::NEXT && b.action != find::PREVIOUS)
    );

    // Commands lists both, without a hint while they are unbound.
    let rows = |rt: &mut Runtime<Workspace>| {
        rt.update(|s, _| s.picker(false));
        rt.frame(1. / 60.);
        let rows: Vec<_> = rt
            .state
            .entries()
            .into_iter()
            .filter(|e| e.label.starts_with("Find "))
            .map(|e| (e.label, e.detail, e.enabled))
            .collect();
        press(rt, Key::Escape);
        rows
    };
    assert_eq!(
        rows(&mut rt),
        [
            ("Find next".to_owned(), String::new(), true),
            ("Find previous".to_owned(), String::new(), true)
        ]
    );

    // Bound in shortcuts.json, the keys show in Commands and in the buttons' tooltips, and
    // step from the terminal and from the field alike.
    rt.update(|s, _| {
        s.settings.shortcut_file.overrides =
            rex_rune::keymap::Overrides::parse(&serde_json::json!([
                {"keys": "f3", "action": find::NEXT},
                {"keys": "shift+f3", "action": find::PREVIOUS}
            ]));
        s.local_keymap = actions::resolve(&s.settings.shortcut_file.overrides);
        s.keymap_generation += 1;
    });
    assert_eq!(
        rows(&mut rt),
        [
            ("Find next".to_owned(), "F3".to_owned(), true),
            ("Find previous".to_owned(), "Shift F3".to_owned(), true)
        ]
    );
    ctrl_shift(&mut rt, 'f');
    type_text(&mut rt, "alpha");
    pane.work(&mut rt);
    assert_eq!(help(&rt, "find-next:block:a"), "Find next · Enter · F3");
    assert_eq!(
        help(&rt, "find-previous:block:a"),
        "Find previous · Shift Enter · Shift F3"
    );
    press(&mut rt, Key::Function(3));
    assert_eq!(pane.work(&mut rt), ["step older=true"], "from the field");
    assert!(field_focused(&rt));
    click(&mut rt, "grid:block:a");
    pane.work(&mut rt);
    press(&mut rt, Key::Function(3));
    shift_press(&mut rt, Key::Function(3));
    assert_eq!(
        pane.work(&mut rt),
        ["step older=true", "step older=false"],
        "from the terminal, and the program gets neither key"
    );

    // With no terminal to search, the rows wait.
    rt.update(|s, _| s.terminals.clear());
    assert_eq!(
        rows(&mut rt)
            .into_iter()
            .map(|row| row.2)
            .collect::<Vec<_>>(),
        [false, false]
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

/// A prompt whose right end holds `shuv`, as a right prompt does, above a line that starts
/// with it, in a terminal of `cols` columns.
fn right_prompt(cols: usize) -> Vec<u8> {
    let mut output = format!("$ {}shuv@host", " ".repeat(cols - 12)).into_bytes();
    output.extend_from_slice(b"\r\n\r\n\r\n\r\n\r\nshuv left\r\n$ ");
    output
}

/// The first column of the right prompt's `shuv`.
fn prompt_column(rt: &Runtime<Workspace>) -> usize {
    columns(&rt.state) - 10
}

/// The selected match's rect in the window: `row` of the pane, `cols` wide from `col`.
fn cells(rt: &Runtime<Workspace>, row: usize, col: usize, cols: usize) -> [f32; 4] {
    let grid = rect(rt, "grid:block:a");
    let (cw, ch) = rt.state.metrics().cell;
    [
        grid[0] + col as f32 * cw,
        grid[1] + row as f32 * ch,
        cols as f32 * cw,
        ch,
    ]
}

fn overlap(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}

/// `block`'s bar and the apron under it, once the apron is seen to be whole rows of the
/// grid with the bar inside it: no row shows in part above or below the bar.
fn bar_on_rows(rt: &Runtime<Workspace>, block: &str) -> ([f32; 4], [f32; 4]) {
    let key = rt.state.term_key(block);
    let corner = rt.state.terminals[&key].find.corner;
    let (bar, grid) = (
        rect(rt, &format!("search:{block}")),
        rect(rt, &format!("grid:{block}")),
    );
    let apron = rect(rt, &format!("find-apron:{block}:{corner:?}"));
    let ch = rt.state.metrics().cell.1;
    let whole = |y: f32| {
        let rows = (y - grid[1]) / ch;
        (rows - rows.round()).abs() < 0.01
    };
    assert!(whole(apron[1]), "{apron:?} starts on a row of {grid:?}");
    let bottom = apron[1] + apron[3];
    assert!(
        whole(bottom) || (bottom - (grid[1] + grid[3])).abs() < 0.01,
        "{apron:?} ends on a row of {grid:?}, or with it"
    );
    assert!(
        apron[0] <= bar[0]
            && apron[1] < bar[1]
            && bar[0] + bar[2] <= apron[0] + apron[2]
            && bar[1] + bar[3] < bottom,
        "{bar:?} is inside {apron:?}"
    );
    (bar, apron)
}

#[test]
fn typography_repositions_find_on_adjusted_cells_without_an_unowned_resize_reply() {
    let (mut rt, mut pane) = single(two_alphas);
    ctrl_shift(&mut rt, 'f');
    type_text(&mut rt, "alpha");
    pane.work(&mut rt);
    assert!(!pane.model.lock().unwrap().owner);
    rt.update(|s, _| {
        s.edit_typography(|font| {
            font.line_height = 20;
            font.spacing_unit = rex_rune::settings::typography::SpacingUnit::Percent;
            font.spacing = 20;
        })
    });
    settle(&mut rt);
    assert!(pane.sent.try_iter().all(|c| !matches!(
        c,
        HostCommand::Terminal {
            command: Command::Resize(..),
            ..
        }
    )));
    bar_on_rows(&rt, "block:a");
    let selected = rt.state.terminals[&pane.key]
        .frame
        .as_ref()
        .unwrap()
        .search
        .as_ref()
        .unwrap()
        .selected;
    assert!(selected.is_some(), "existing search is preserved");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_bar_leaves_the_corner_the_selected_match_is_in() {
    let (mut rt, mut pane) = single(right_prompt);
    let grid = rect(&rt, "grid:block:a");
    let key = pane.key.clone();
    let corner = |rt: &Runtime<Workspace>| rt.state.terminals[&key].find.corner;
    // Home: 6 pt from the right edge, on the terminal's first rows.
    let home = |rt: &Runtime<Workspace>| {
        let (bar, apron) = bar_on_rows(rt, "block:a");
        (bar[0] + bar[2] - (grid[0] + grid[2] - find::INSET)).abs() < 0.01
            && (apron[1] - grid[1]).abs() < 0.01
    };
    let prompt = cells(&rt, 0, prompt_column(&rt), 4);
    assert!(
        prompt[0] + prompt[2] <= grid[0] + grid[2],
        "the right prompt is in the pane"
    );
    ctrl_shift(&mut rt, 'f');
    assert!(home(&rt), "in the top-right");

    // The newest match is the one at the left, six rows down: the bar has no reason to move.
    type_text(&mut rt, "shuv");
    pane.work(&mut rt);
    assert_eq!(count(&rt).unwrap().0, "1/2");
    assert_eq!(corner(&rt), find::Corner::TopRight);
    assert!(home(&rt));
    let (bar, apron) = bar_on_rows(&rt, "block:a");
    let unselected = cells(&rt, 0, prompt_column(&rt), 4);
    assert!(
        overlap(bar, unselected),
        "the right prompt's match is under the bar while it is not the selected one"
    );
    assert!(
        apron[1] <= unselected[1] && unselected[1] + unselected[3] <= apron[1] + apron[3],
        "and its whole row is hidden there, not a sliver of it left above the bar"
    );

    // The older match is in the right prompt on the first row, where the bar is. The bar
    // goes to the bottom-right corner, and the match is in view.
    press(&mut rt, Key::Enter);
    assert_eq!(pane.work(&mut rt), ["step older=true"]);
    assert_eq!(count(&rt).unwrap().0, "2/2");
    assert_eq!(corner(&rt), find::Corner::BottomRight);
    let (bar, apron) = bar_on_rows(&rt, "block:a");
    assert!(
        !overlap(apron, cells(&rt, 0, prompt_column(&rt), 4)),
        "{apron:?}"
    );
    assert!(
        !has(&rt, "find-apron:block:a:TopRight"),
        "nothing is left hiding the corner the bar came from"
    );
    assert!(
        (bar[0] + bar[2] - (grid[0] + grid[2] - find::INSET)).abs() < 0.01
            && (apron[1] + apron[3] - (grid[1] + grid[3])).abs() < 0.01,
        "in the bottom-right, on the last rows: {bar:?} in {grid:?}"
    );
    assert!(
        field_focused(&rt),
        "the field keeps the keyboard as it moves"
    );
    assert_eq!(
        rect(&rt, "grid:block:a"),
        grid,
        "the terminal is not moved or resized"
    );

    // It stays there while the selected match is clear of it, so stepping does not send it
    // back and forth.
    press(&mut rt, Key::Enter);
    pane.work(&mut rt);
    assert_eq!(count(&rt).unwrap().0, "1/2");
    assert_eq!(corner(&rt), find::Corner::BottomRight);

    // Closed, it goes home: the next search starts in the top-right corner.
    press(&mut rt, Key::Escape);
    pane.work(&mut rt);
    assert_eq!(corner(&rt), find::Corner::TopRight);
    run(&mut rt, find::NEXT);
    pane.work(&mut rt);
    assert!(home(&rt));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_bar_springs_to_its_new_corner_and_the_terminal_stays_still() {
    let (mut rt, mut pane) = single(right_prompt);
    ctrl_shift(&mut rt, 'f');
    type_text(&mut rt, "shuv");
    pane.work(&mut rt);
    let top = rect(&rt, "search:block:a")[1];
    press(&mut rt, Key::Enter);
    // Hand the pane the frame, and look two frames into the move.
    for command in pane.sent.try_iter() {
        if let HostCommand::Terminal {
            command: Command::SearchStep(older),
            ..
        } = command
        {
            pane.search.step(&pane.terminal, older).unwrap();
        }
    }
    let mut frame = Renderer::new().unwrap().frame(&pane.terminal).unwrap();
    frame.search = pane.search.results(&pane.terminal).unwrap();
    pane.model.lock().unwrap().frame = Some(Arc::new(frame));
    let key = pane.key.clone();
    rt.update(move |s, _| s.refresh_terminal(&key));
    rt.frame(1. / 60.);
    rt.frame(1. / 60.);
    let moving = element(&rt, "search:block:a");
    assert!(
        moving.rect[1] > top && moving.rect[1] < moving.target[1],
        "on its way down: {:?} towards {:?}",
        moving.rect,
        moving.target
    );
    settle(&mut rt);
    let landed = element(&rt, "search:block:a");
    assert_eq!(landed.rect, landed.target);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_split_panes_bar_is_under_its_header_and_inside_its_card() {
    let (mut ws, _host) = tabbed_workspace("split");
    let mut left = Searched::new(&mut ws, "block:b", (40, 10), b"alpha left\r\n$ ");
    let _right = Searched::new(&mut ws, "block:c", (40, 10), b"alpha right\r\n$ ");
    let mut rt = runtime(ws);
    settle(&mut rt);
    ctrl_shift(&mut rt, 'f');
    type_text(&mut rt, "alpha");
    left.work(&mut rt);
    assert!(has(&rt, "search:block:b"));
    assert!(
        !has(&rt, "search:block:c"),
        "only the focused pane is searched"
    );
    let (bar, grid, header) = (
        rect(&rt, "search:block:b"),
        rect(&rt, "grid:block:b"),
        rect(&rt, "pane-header:block:b"),
    );
    assert!(bar[1] >= header[1] + header[3], "below the header row");
    let (_, apron) = bar_on_rows(&rt, "block:b");
    assert!((apron[1] - grid[1]).abs() < 0.01, "on the first rows");
    assert!(apron[0] + apron[2] <= grid[0] + grid[2] + 0.01);
    assert!((bar[0] + bar[2] - (grid[0] + grid[2] - find::INSET)).abs() < 0.01);
    assert_eq!(bar[2], find::WIDTH.min(grid[2] - 2. * find::INSET));
    // The match is at the left of the first row, clear of the bar.
    assert_eq!(
        rt.state.terminals[&left.key].find.corner,
        find::Corner::TopRight
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_reconnecting_panes_bar_sits_under_the_floating_strip() {
    let (mut rt, mut pane) = single(right_prompt);
    ctrl_shift(&mut rt, 'f');
    type_text(&mut rt, "left");
    pane.work(&mut rt);
    let grid = rect(&rt, "grid:block:a");
    let (_, apron) = bar_on_rows(&rt, "block:a");
    assert!((apron[1] - grid[1]).abs() < 0.01, "on the first rows");

    // The connection goes: the strip floats in the corner, and the bar starts under it.
    pane.model.lock().unwrap().connected = false;
    pane.work(&mut rt);
    let (strip, bar) = (rect(&rt, "strip"), rect(&rt, "search:block:a"));
    assert!(!overlap(strip, bar), "{strip:?} and {bar:?}");
    assert!(bar[1] >= strip[1] + strip[3]);
    let (_, apron) = bar_on_rows(&rt, "block:a");
    assert!(
        apron[1] >= strip[1] + strip[3],
        "its rows start under the strip"
    );
    assert_eq!(
        rt.state.terminals[&pane.key].find.corner,
        find::Corner::TopRight
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

/// The app's own fonts, for the camera: the terminal's text is then on the grid as in the
/// app, where a headless runtime sets it in Inter. Loading them changes the cell size for the
/// whole process, so take the pictures in a run of this test alone.
fn app_fonts() -> rune::text::TextSystem {
    let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
    rune::text::TextSystem::new(fonts.files, fonts.families)
}

/// A window at 2×, in the app's fonts while the camera is on.
fn studio(ws: Workspace, camera: bool) -> Runtime<Workspace> {
    let (size, env) = ((1024., 700.), Env::new(true, 2.));
    let mut rt = if camera {
        Runtime::with_text(ws, view, size, env, app_fonts())
    } else {
        Runtime::new(ws, view, size, env)
    };
    rt.on_key = Some(crate::app::input::on_key);
    settle(&mut rt);
    rt
}

/// The bar as drawn, for a look at it (see `Camera`): empty, with the selected match beside
/// it, moved off the match in the right prompt, with no matches, and in a split.
#[test]
fn the_find_bar_draws_its_states() {
    let mut camera = Camera::new();
    let on = camera.is_some();
    if on {
        // Before anything is measured.
        drop(app_fonts());
    }
    let mut snap = |rt: &mut Runtime<Workspace>, name: &str| {
        if let Some(camera) = &mut camera {
            camera.snap(rt, name);
        }
    };
    let (mut ws, _host) = tabbed_workspace("single");
    let cols = columns(&ws);
    let mut output = right_prompt(cols);
    output.extend_from_slice(
        b"ls -la ~/shuv\r\ntotal 24\r\ndrwxr-xr-x  shuv  staff  notes\r\n-rw-r--r--  shuv  staff  \x1b[34mshuv.toml\x1b[0m\r\n$ ",
    );
    let mut pane = Searched::new(&mut ws, "block:a", (cols as u16, 24), &output);
    let mut rt = studio(ws, on);

    ctrl_shift(&mut rt, 'f');
    settle(&mut rt);
    snap(&mut rt, "find-empty");
    type_text(&mut rt, "shuv");
    pane.work(&mut rt);
    snap(&mut rt, "find-results");
    for _ in 0..5 {
        press(&mut rt, Key::Enter);
    }
    pane.work(&mut rt);
    assert_eq!(count(&rt).unwrap().0, "6/6");
    assert_eq!(
        rt.state.terminals[&pane.key].find.corner,
        find::Corner::BottomRight
    );
    snap(&mut rt, "find-right-prompt");
    type_text(&mut rt, "x");
    pane.work(&mut rt);
    snap(&mut rt, "find-no-matches");
    std::fs::remove_file(&rt.state.settings_path).ok();

    let (mut ws, _host) = tabbed_workspace("split");
    let mut left = Searched::new(&mut ws, "block:b", (48, 20), &output[cols - 1..]);
    let _right = Searched::new(&mut ws, "block:c", (48, 20), b"$ tail -f server.log");
    let mut rt = studio(ws, on);
    ctrl_shift(&mut rt, 'f');
    type_text(&mut rt, "shuv");
    left.work(&mut rt);
    snap(&mut rt, "find-split");
    std::fs::remove_file(&rt.state.settings_path).ok();
}
