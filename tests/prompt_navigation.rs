//! Ghostty owns prompt groups and viewport movement; no PTY writes or mark index.
use ghostty_vt::{
    Terminal, TerminalOptions,
    terminal::{Point, PointCoordinate, ScrollViewport},
};
use rex_linux::terminal::{Renderer, TerminalStream};

fn terminal(rows: u16) -> Terminal {
    Terminal::new(TerminalOptions {
        cols: 40,
        rows,
        ..Default::default()
    })
    .unwrap()
}

fn transcript(t: &mut Terminal) {
    for i in 0..4 {
        t.vt_write(format!("\x1b]133;A\x07prompt {i}> \x1b]133;B\x07command\x1b]133;C\x07\r\noutput\r\nmore output\r\n\x1b]133;D;0\x07").as_bytes());
    }
}

fn offset(t: &Terminal) -> u64 {
    t.scrollbar().unwrap().offset
}

fn jump(t: &mut Terminal, older: bool) -> u64 {
    t.scroll_viewport(ScrollViewport::DeltaPrompt(if older { -1 } else { 1 }));
    offset(t)
}

#[test]
fn restored_scrollback_repeated_steps_stop_at_boundaries_and_render_immediately() {
    let mut source = terminal(3);
    transcript(&mut source);
    let bytes = ghostty_vt::snapshot::encode(&source).unwrap();
    let mut stream = TerminalStream::new(7);
    // Exercise snapshot framing, not just vt_write on a fresh terminal.
    for chunk in bytes.chunks(17) {
        stream.feed(chunk).unwrap();
    }
    let text = stream.text().unwrap();
    let t = stream.terminal.as_mut().unwrap();
    let cursor = (t.cursor_x().unwrap(), t.cursor_y().unwrap());
    assert_eq!(offset(t), 10);
    for expected in [9, 6, 3, 0, 0, 0] {
        assert_eq!(jump(t, true), expected);
        let mut decoder = Renderer::new().unwrap();
        let mut frame = decoder.frame(t).unwrap();
        // The data worker sets this only for an explicit, successful prompt jump.
        frame.prompt_anchor = true;
        let line: String = frame.cells[0].iter().map(|c| c.text.as_str()).collect();
        assert!(
            line.starts_with(&format!("prompt {}>", expected / 3)),
            "{line}"
        );
        assert_eq!(
            frame.display_row_start(1),
            0,
            "a smaller non-owner pane must show the prompt"
        );
    }
    for expected in [3, 6, 9, 9, 9] {
        assert_eq!(jump(t, false), expected);
    }
    // Navigation does not move the program cursor, change text or write more output.
    assert_eq!((t.cursor_x().unwrap(), t.cursor_y().unwrap()), cursor);
    assert_eq!(stream.text().unwrap(), text);
}

#[test]
fn ordinary_scrolling_in_a_cropped_pane_keeps_every_row_reachable() {
    let mut t = terminal(10);
    for i in 0..40 {
        t.vt_write(format!("line{i:02}\r\n").as_bytes());
    }
    t.vt_write(b"$ ");
    let mut renderer = Renderer::new().unwrap();
    for first in [37, 36, 35, 34, 33] {
        let frame = renderer.frame(&t).unwrap();
        let start = frame.display_row_start(4);
        assert_eq!(start, 6);
        let line: String = frame.cells[start].iter().map(|c| c.text.as_str()).collect();
        assert!(line.starts_with(&format!("line{first:02}")), "{line}");
        assert!(!frame.prompt_anchor);
        t.scroll_viewport(ScrollViewport::Delta(-1));
    }
    // An old search result must not push an explicitly selected prompt out of view.
    let mut frame = renderer.frame(&t).unwrap();
    frame.prompt_anchor = true;
    let mut search = rex_linux::terminal::search::Results::default();
    search.selected = Some(rex_linux::terminal::search::Match {
        start: (0, 7),
        end: (2, 7),
    });
    frame.search = Some(search);
    assert_eq!(frame.display_row_start(4), 0);
}

#[test]
fn next_clamps_to_active_area_and_manual_scroll_is_the_navigation_origin() {
    let mut t = terminal(4);
    transcript(&mut t);
    t.vt_write(b"\x1b]133;A\x07current> \x1b]133;B\x07");
    assert_eq!(offset(&t), 9);
    assert_eq!(
        jump(&mut t, true),
        6,
        "top row prompt is not selected twice"
    );
    assert_eq!(jump(&mut t, false), 9);
    assert_eq!(
        jump(&mut t, false),
        9,
        "newest prompt is already in active area"
    );
    t.scroll_viewport(ScrollViewport::Row(4));
    assert_eq!(jump(&mut t, true), 3);
    t.scroll_viewport(ScrollViewport::Row(4));
    assert_eq!(jump(&mut t, false), 6);
}

#[test]
fn unmarked_and_alternate_terminals_do_not_move() {
    let mut t = terminal(3);
    t.vt_write(b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix");
    t.scroll_viewport(ScrollViewport::Row(1));
    for older in [true, true, false, false] {
        assert_eq!(jump(&mut t, older), 1);
    }
    t.vt_write(b"\x1b[?1049h\x1b]133;A\x07alternate> ");
    for older in [true, false] {
        assert_eq!(jump(&mut t, older), 0);
    }
    t.vt_write(b"\x1b[?1049l");
    assert_eq!(offset(&t), 1);
}

#[test]
fn continuation_rows_are_one_prompt_group() {
    let mut t = terminal(2);
    t.vt_write(b"\x1b]133;P;k=i\x07first\r\n\x1b]133;P;k=c\x07continued\r\n\x1b]133;B\x07cmd\r\n\x1b]133;C\x07out\r\n\x1b]133;A\x07second\r\n\x1b]133;B\x07cmd\r\n\x1b]133;C\x07out");
    t.scroll_viewport(ScrollViewport::Top);
    assert_eq!(jump(&mut t, false), 4);
    assert_eq!(jump(&mut t, true), 0);
    // A soft-wrapped primary prompt is likewise one group.
    let mut t = terminal(2);
    t.resize(5, 2, 0, 0).unwrap();
    t.vt_write(b"\x1b]133;A\x07abcdefghij\x1b]133;B\x07\r\n\x1b]133;C\x07out\r\n\x1b]133;A\x07next\x1b]133;B\x07\r\n\x1b]133;C\x07out");
    t.scroll_viewport(ScrollViewport::Top);
    assert_eq!(jump(&mut t, false), 3);
    assert_eq!(jump(&mut t, true), 0);
}

#[test]
fn live_marks_after_snapshot_and_resize_remain_navigable() {
    let t = terminal(3);
    let mut stream = TerminalStream::new(1);
    stream
        .feed(&ghostty_vt::snapshot::encode(&t).unwrap())
        .unwrap();
    // Deliberately split OSC prefixes and terminators across feed calls.
    let bytes = b"\x1b]133;A\x07first> \x1b]133;B\x07cmd\x1b]133;C\x07\r\nout\r\nout\r\n\x1b]133;D;0\x07\x1b]133;A\x07second> \x1b]133;B\x07cmd\x1b]133;C\x07\r\nout\r\nout\r\n";
    for chunk in bytes.chunks(2) {
        stream.feed(chunk).unwrap();
    }
    let t = stream.terminal.as_mut().unwrap();
    t.resize(60, 3, 0, 0).unwrap();
    assert_eq!(jump(t, true), 3);
    assert_eq!(jump(t, true), 0);
    assert_eq!(jump(t, false), 3);
    let row = t
        .grid_ref(Point::Viewport(PointCoordinate { x: 0, y: 0 }))
        .unwrap();
    assert_eq!(row.cell().unwrap().codepoint().unwrap(), u32::from('s'));
}
