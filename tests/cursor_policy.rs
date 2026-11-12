//! Verify Ghostty's actual default/explicit distinction before using it for local policy.
use ghostty_vt::{Terminal, TerminalOptions, terminal::CursorStyle};
use rex_rune::terminal::{CursorShape, Renderer};

#[test]
fn ghostty_preserves_default_vs_explicit_cursor_in_snapshots() {
    for explicit in [false, true] {
        let mut server = Terminal::new(TerminalOptions {
            cols: 12,
            rows: 2,
            ..Default::default()
        })
        .unwrap();
        if explicit {
            server.vt_write(b"\x1b[2 q");
        }
        let bytes = ghostty_vt::snapshot::encode(&server).unwrap();
        let mut stream = rex_rune::terminal::TerminalStream::new(42);
        stream.feed(&bytes).unwrap();
        let mut local = stream.terminal.take().unwrap();
        local
            .set_default_cursor_style(Some(CursorStyle::Bar))
            .unwrap();
        let mut renderer = Renderer::new().unwrap();
        assert_eq!(
            renderer.frame(&local).unwrap().cursor_shape,
            if explicit {
                CursorShape::Block
            } else {
                CursorShape::Bar
            }
        );
        local.vt_write(b"\x1b[2 q");
        local
            .set_default_cursor_style(Some(CursorStyle::Underline))
            .unwrap();
        assert_eq!(
            renderer.frame(&local).unwrap().cursor_shape,
            CursorShape::Block
        );
        local.vt_write(b"\x1b[0 q");
        assert_eq!(
            renderer.frame(&local).unwrap().cursor_shape,
            CursorShape::Underline
        );
        assert_eq!(
            renderer.frame(&server).unwrap().cursor_shape,
            CursorShape::Block
        );
    }
}
