use crate::wire::SnapshotBuffer;
use anyhow::{Result, ensure};
use ghostty_vt::{
    RenderState, Terminal,
    render::{CellIterator, CursorVisualStyle, RowIterator},
    style::{RgbColor, StyleColor},
    terminal::{Mode, Point, PointCoordinate},
};
use serde_json::Value;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

pub mod graphics;
pub mod search;
pub mod selection;

#[derive(Debug, Clone, PartialEq)]
pub enum Part {
    Output(Vec<u8>),
    Metadata(Value),
}

/// Retains partial escape prefixes across frames and reconnects. Only a Rex
/// APC with this channel's nonce is interpreted as transport metadata.
pub struct ApcParser {
    prefix: Vec<u8>,
    pending: Vec<u8>,
}
impl ApcParser {
    pub fn new(nonce: u64) -> Self {
        Self {
            prefix: format!("\x1b_rex1;{nonce};").into_bytes(),
            pending: Vec::new(),
        }
    }
    pub fn feed(&mut self, bytes: &[u8]) -> Result<Vec<Part>> {
        let mut parts = Vec::new();
        let mut output = Vec::new();
        for &byte in bytes {
            self.pending.push(byte);
            loop {
                if self.pending.starts_with(&self.prefix) {
                    ensure!(self.pending.len() <= 1024 * 1024, "Rex APC exceeds 1 MiB");
                    if self.pending.ends_with(b"\x1b\\") {
                        if !output.is_empty() {
                            parts.push(Part::Output(std::mem::take(&mut output)));
                        }
                        let json = &self.pending[self.prefix.len()..self.pending.len() - 2];
                        parts.push(Part::Metadata(serde_json::from_slice(json)?));
                        self.pending.clear();
                    }
                    break;
                }
                if self.prefix.starts_with(&self.pending) {
                    break;
                }
                output.push(self.pending.remove(0));
            }
        }
        if !output.is_empty() {
            parts.push(Part::Output(output));
        }
        Ok(parts)
    }
}

/// How long a program may hold rendering with synchronized output (mode 2026) before the
/// terminal is drawn anyway, as in Ghostty.
pub const RENDER_HOLD_LIMIT: Duration = Duration::from_secs(1);

pub struct TerminalStream {
    pub terminal: Option<Terminal>,
    snapshot: SnapshotBuffer,
    parser: ApcParser,
    pub received: u64,
    pub snapshot_count: usize,
    pub exit_code: Option<i64>,
    graphics_ready: bool,
    /// When the running program began its current synchronized update.
    held_since: Option<Instant>,
    /// The cursor this client draws until the program asks for a shape of its own.
    cursor_style: CursorShape,
}
impl TerminalStream {
    pub fn new(nonce: u64) -> Self {
        Self {
            terminal: None,
            snapshot: SnapshotBuffer::default(),
            parser: ApcParser::new(nonce),
            received: 0,
            snapshot_count: 0,
            exit_code: None,
            graphics_ready: false,
            held_since: None,
            cursor_style: CursorShape::Block,
        }
    }

    /// The cursor to draw while the program has not asked for a shape (see
    /// [`set_default_cursor`]), on this terminal and on the one a later snapshot restores.
    pub fn set_cursor_style(&mut self, shape: CursorShape) -> Result<()> {
        self.cursor_style = shape;
        if let Some(terminal) = self.terminal.as_mut() {
            set_default_cursor(terminal, shape)?;
        }
        Ok(())
    }

    /// The program is in the middle of a synchronized update (mode 2026), so drawing now
    /// would show it half done: a full-screen program redrawing after a resize would flash
    /// blank. A hold past [`RENDER_HOLD_LIMIT`] is ended here, as a program may never end it.
    pub fn render_held(&mut self, now: Instant) -> Result<bool> {
        let Some(terminal) = self.terminal.as_mut() else {
            return Ok(false);
        };
        if !terminal.mode(Mode::SYNC_OUTPUT)? {
            self.held_since = None;
            return Ok(false);
        }
        let since = *self.held_since.get_or_insert(now);
        if now.duration_since(since) < RENDER_HOLD_LIMIT {
            return Ok(true);
        }
        terminal.set_mode(Mode::SYNC_OUTPUT, false)?;
        self.held_since = None;
        Ok(false)
    }
    pub fn feed(&mut self, bytes: &[u8]) -> Result<()> {
        if !self.graphics_ready {
            graphics::install_decoder()?;
            self.graphics_ready = true;
        }
        self.received += bytes.len() as u64;
        let tail;
        let bytes = if self.terminal.is_none() {
            let Some((mut terminal, rest)) = self.snapshot.push(bytes)? else {
                return Ok(());
            };
            graphics::configure(&mut terminal)?;
            // A restored terminal has the server's default cursor, a block.
            if self.cursor_style != CursorShape::Block {
                set_default_cursor(&mut terminal, self.cursor_style)?;
            }
            self.terminal = Some(terminal);
            self.snapshot_count += 1;
            tail = rest;
            &tail
        } else {
            bytes
        };
        let terminal = self.terminal.as_mut().expect("snapshot installed terminal");
        for part in self.parser.feed(bytes)? {
            match part {
                Part::Output(bytes) => terminal.vt_write(&bytes),
                Part::Metadata(value) => match value["type"].as_str() {
                    Some("resize") => {
                        let cols = value["cols"]
                            .as_u64()
                            .filter(|v| (1..=65535).contains(v))
                            .ok_or_else(|| anyhow::anyhow!("invalid resize columns"))?
                            as u16;
                        let rows = value["rows"]
                            .as_u64()
                            .filter(|v| (1..=65535).contains(v))
                            .ok_or_else(|| anyhow::anyhow!("invalid resize rows"))?
                            as u16;
                        terminal.resize(
                            cols,
                            rows,
                            value["cell_width_px"].as_u64().unwrap_or(0) as u32,
                            value["cell_height_px"].as_u64().unwrap_or(0) as u32,
                        )?;
                    }
                    Some("theme") => {
                        terminal.set_default_fg_color(parse_color(value["foreground"].as_str()))?;
                        terminal.set_default_bg_color(parse_color(value["background"].as_str()))?;
                        terminal.set_default_cursor_color(parse_color(value["cursor"].as_str()))?;
                        // Each theme replaces the previous palette, including omissions.
                        terminal.set_default_color_palette(None)?;
                        if let Some(entries) = value["palette"].as_array() {
                            let mut palette = terminal.color_palette()?;
                            for entry in entries.iter().filter_map(Value::as_str) {
                                if let Some((index, color)) = entry.split_once('=')
                                    && let (Ok(index), Some(color)) =
                                        (index.parse::<u8>(), parse_color(Some(color)))
                                {
                                    palette.0[index as usize] = color;
                                }
                            }
                            terminal.set_default_color_palette(Some(palette))?;
                        }
                    }
                    Some("child_exited") => self.exit_code = value["exit_code"].as_i64(),
                    _ => {}
                },
            }
        }
        Ok(())
    }
    pub fn text(&self) -> Result<String> {
        let terminal = self
            .terminal
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("snapshot not ready"))?;
        let mut formatter = ghostty_vt::fmt::Formatter::new(
            terminal,
            ghostty_vt::fmt::FormatterOptions::default(),
        )?;
        Ok(String::from_utf8_lossy(&formatter.format_alloc()?).into_owned())
    }
}

fn parse_color(value: Option<&str>) -> Option<RgbColor> {
    let value = value?.trim_start_matches('#');
    if value.len() != 6 {
        return None;
    }
    let color = u32::from_str_radix(value, 16).ok()?;
    Some(RgbColor {
        r: (color >> 16) as u8,
        g: (color >> 8) as u8,
        b: color as u8,
    })
}
pub fn rgb(color: RgbColor) -> u32 {
    (color.r as u32) << 16 | (color.g as u32) << 8 | color.b as u32
}

/// Make `shape` the cursor of this client's copy of a terminal for as long as its program
/// has not chosen one: Ghostty shows it at once unless a DECSCUSR shape is in effect, and
/// goes back to it when the program resets its own (`CSI 0 SP q`) or the terminal. A local
/// drawing choice: the server's terminal, and what it answers a program that asks for the
/// cursor style, are unchanged.
pub fn set_default_cursor(terminal: &mut Terminal, shape: CursorShape) -> Result<()> {
    use ghostty_vt::terminal::CursorStyle;
    terminal.set_default_cursor_style(Some(match shape {
        CursorShape::Block => CursorStyle::Block,
        CursorShape::Bar => CursorStyle::Bar,
        CursorShape::Underline => CursorStyle::Underline,
        CursorShape::Hollow => CursorStyle::BlockHollow,
    }))?;
    Ok(())
}

/// The cursor shape a program asked for with DECSCUSR (or Ghostty's hollow block).
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum CursorShape {
    Block,
    Bar,
    Underline,
    Hollow,
}

#[derive(Clone, Debug)]
pub struct Cell {
    pub text: String,
    pub spacer_tail: bool,
    pub fg: u32,
    pub bg: u32,
    /// The SGR 58 underline colour; none draws underlines in the text colour.
    pub underline_color: Option<u32>,
    pub style: ghostty_vt::style::Style,
    /// The cell's OSC 8 hyperlink target.
    pub link: Option<Arc<str>>,
}
#[derive(Clone, Debug)]
pub struct Frame {
    pub cols: u16,
    pub rows: u16,
    pub cells: Vec<Vec<Cell>>,
    pub fg: u32,
    pub bg: u32,
    pub cursor: Option<(u16, u16)>,
    /// An explicit prompt jump anchors a cropped pane at the viewport top. Ordinary
    /// scrolling keeps its usual bottom/cursor crop. Set by the data worker, not decoding.
    pub prompt_anchor: bool,
    pub cursor_color: u32,
    pub cursor_shape: CursorShape,
    /// The program asked for mouse reports (DEC 9, 1000, 1002 or 1003).
    pub mouse_tracking: bool,
    pub images: Vec<graphics::Image>,
    pub cell_pixels: [u32; 2],
    pub selection_rows: Vec<Option<(usize, usize)>>,
    pub selection_text: Option<std::sync::Arc<str>>,
    pub selection_error: Option<String>,
    pub selection_revision: u64,
    /// Per row: the row soft-wraps into the next one.
    pub wrapped: Vec<bool>,
    /// The user's search of this terminal, while there is one.
    pub search: Option<search::Results>,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum SelectionUnit {
    #[default]
    Cell,
    Word,
    Line,
}

impl Frame {
    /// Inclusive cell bounds, preserving wide glyphs and whole words/paths.
    pub fn selection_range(&self, index: usize, unit: SelectionUnit) -> Option<(usize, usize)> {
        let cols = self.cols as usize;
        let row = index / cols;
        let cells = self.cells.get(row)?;
        let mut start = index % cols;
        cells.get(start)?;
        while start > 0 && cells[start].spacer_tail {
            start -= 1;
        }
        let mut end = start;
        if cells.get(end + 1).is_some_and(|c| c.spacer_tail) {
            end += 1;
        }
        match unit {
            SelectionUnit::Cell => {}
            SelectionUnit::Line => {
                start = 0;
                end = cells.len() - 1;
            }
            SelectionUnit::Word => {
                let kind = |mut col: usize| {
                    while col > 0 && cells[col].spacer_tail {
                        col -= 1;
                    }
                    match cells[col].text.chars().next() {
                        None => 0,
                        Some(c) if c.is_whitespace() => 0,
                        Some(c) if c.is_alphanumeric() || "_-.~/\\:@%+=".contains(c) => 1,
                        Some(_) => 2,
                    }
                };
                let selected = kind(start);
                while start > 0 && kind(start - 1) == selected {
                    start -= 1;
                }
                while end + 1 < cells.len() && kind(end + 1) == selected {
                    end += 1;
                }
            }
        }
        Some((row * cols + start, row * cols + end))
    }
    /// An explicit prompt jump keeps the viewport top visible in a smaller pane. Otherwise
    /// cropping retains the newest rows, the program cursor, or a selected search match.
    /// Search navigation clears the prompt anchor; an old match must not hide a new prompt.
    /// This never changes the server size.
    pub fn display_row_start(&self, visible_rows: usize) -> usize {
        let visible = visible_rows.max(1);
        let bottom = (self.rows as usize).saturating_sub(visible);
        if self.prompt_anchor {
            return 0;
        }
        let start = self.cursor.map_or(bottom, |(_, y)| bottom.min(y as usize));
        let Some(found) = self.search.as_ref().and_then(|s| s.selected) else {
            return start;
        };
        let row = usize::from(found.start.1);
        if row < start {
            row
        } else {
            start.max((row + 1).saturating_sub(visible)).min(bottom)
        }
    }

    pub fn selected_text(&self, a: usize, b: usize) -> String {
        let (start, end) = (a.min(b), a.max(b));
        let cols = self.cols as usize;
        let mut lines = Vec::new();
        for row in start / cols..=end / cols {
            let Some(cells) = self.cells.get(row) else {
                break;
            };
            let mut line = String::new();
            for (col, cell) in cells.iter().enumerate() {
                let index = row * cols + col;
                if index >= start
                    && index <= end
                    && !cell.spacer_tail
                    && !cell.text.starts_with('\u{10eeee}')
                {
                    line.push_str(if cell.text.is_empty() {
                        " "
                    } else {
                        &cell.text
                    });
                }
            }
            lines.push(line.trim_end().to_owned());
        }
        lines.join("\n")
    }
}
pub struct Renderer {
    state: RenderState,
    rows: RowIterator,
    cells: CellIterator,
    images: graphics::Images,
    copy_buffer: Vec<u8>,
}
impl Renderer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            state: RenderState::new()?,
            rows: RowIterator::new()?,
            cells: CellIterator::new()?,
            images: graphics::Images::new()?,
            copy_buffer: Vec::new(),
        })
    }
    pub fn frame(&mut self, terminal: &Terminal) -> Result<Frame> {
        let (selection_text, selection_error) =
            match selection::text(terminal, &mut self.copy_buffer) {
                Ok(text) => (text, None),
                Err(error) => (None, Some(format!("Copy failed: {error}"))),
            };
        let snapshot = self.state.update(terminal)?;
        let colors = snapshot.colors()?;
        let fg = rgb(colors.foreground);
        let bg = rgb(colors.background);
        let mut frame = Frame {
            cols: snapshot.cols()?,
            rows: snapshot.rows()?,
            cells: Vec::new(),
            selection_rows: Vec::new(),
            selection_text,
            selection_error,
            selection_revision: 0,
            fg,
            bg,
            cursor: if snapshot.cursor_visible()? {
                snapshot.cursor_viewport()?.map(|p| (p.x, p.y))
            } else {
                None
            },
            prompt_anchor: false,
            cursor_color: snapshot.cursor_color()?.map(rgb).unwrap_or(fg),
            // Blinking is ignored: terminal content never animates.
            cursor_shape: match snapshot.cursor_visual_style()? {
                CursorVisualStyle::Bar => CursorShape::Bar,
                CursorVisualStyle::Underline => CursorShape::Underline,
                CursorVisualStyle::BlockHollow => CursorShape::Hollow,
                _ => CursorShape::Block,
            },
            mouse_tracking: terminal.is_mouse_tracking()?,
            images: self.images.frame(terminal, &snapshot)?,
            cell_pixels: [
                terminal.width_px()? / u32::from(snapshot.cols()?).max(1),
                terminal.height_px()? / u32::from(snapshot.rows()?).max(1),
            ],
            wrapped: Vec::new(),
            search: None,
        };
        let mut uri = Vec::new();
        let mut rows = self.rows.update(&snapshot)?;
        let mut y = 0;
        while let Some(row) = rows.next() {
            frame.selection_rows.push(
                row.selection()?
                    .map(|s| (usize::from(s.start_x), usize::from(s.end_x))),
            );
            let raw_row = row.raw_row()?;
            frame.wrapped.push(raw_row.is_wrapped()?);
            let row_links = raw_row.has_hyperlink()?;
            let mut cells = self.cells.update(row)?;
            let mut line: Vec<Cell> = Vec::new();
            while let Some(cell) = cells.next() {
                let mut text = String::new();
                cell.graphemes_utf8(&mut text)?;
                let style = cell.style()?;
                let mut fg = cell.fg_color()?.map(rgb).unwrap_or(fg);
                let mut bg = cell.bg_color()?.map(rgb).unwrap_or(bg);
                if style.inverse {
                    std::mem::swap(&mut fg, &mut bg);
                }
                if style.invisible {
                    text.clear();
                }
                let raw = cell.raw_cell()?;
                let link = if row_links && raw.has_hyperlink()? {
                    let at = (line.len() as u16, y);
                    hyperlink(terminal, at, &mut uri)?.then(|| {
                        // Neighbouring cells of one link share its string.
                        match line.last().and_then(|c| c.link.clone()) {
                            Some(prev) if prev.as_bytes() == uri => prev,
                            _ => Arc::from(String::from_utf8_lossy(&uri)),
                        }
                    })
                } else {
                    None
                };
                let underline_color = match style.underline_color {
                    StyleColor::None => None,
                    StyleColor::Palette(index) => Some(rgb(colors.palette[index.0 as usize])),
                    StyleColor::Rgb(color) => Some(rgb(color)),
                };
                line.push(Cell {
                    text,
                    spacer_tail: raw.wide()? == ghostty_vt::screen::CellWide::SpacerTail,
                    fg,
                    bg,
                    underline_color,
                    style,
                    link,
                });
            }
            frame.cells.push(line);
            y += 1;
        }
        snapshot.clean()?;
        Ok(frame)
    }
}

/// Reads the OSC 8 target of the viewport cell `at` into `uri`; false if it has none.
fn hyperlink(terminal: &Terminal, at: (u16, u16), uri: &mut Vec<u8>) -> Result<bool> {
    let point = Point::Viewport(PointCoordinate {
        x: at.0,
        y: at.1.into(),
    });
    let cell = terminal.grid_ref(point)?;
    uri.resize(uri.capacity().max(256), 0);
    let len = match cell.hyperlink_uri(uri) {
        Err(ghostty_vt::Error::OutOfSpace { required }) => {
            uri.resize(required, 0);
            cell.hyperlink_uri(uri)?
        }
        result => result?,
    };
    uri.truncate(len);
    Ok(len > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn word_and_line_selection_preserve_paths_unicode_and_wide_cells() {
        let mut terminal = Terminal::new(ghostty_vt::TerminalOptions {
            cols: 40,
            rows: 3,
            ..Default::default()
        })
        .unwrap();
        terminal.vt_write("alpha /tmp/beta-gamma.txt 世界\r\ne\u{301} 🦀 tail".as_bytes());
        let frame = Renderer::new().unwrap().frame(&terminal).unwrap();
        let word = frame.selection_range(12, SelectionUnit::Word).unwrap();
        assert_eq!(frame.selected_text(word.0, word.1), "/tmp/beta-gamma.txt");
        let wide = frame.selection_range(27, SelectionUnit::Cell).unwrap();
        assert_eq!(wide, (26, 27));
        assert_eq!(frame.selected_text(wide.0, wide.1), "世");
        let word = frame.selection_range(27, SelectionUnit::Word).unwrap();
        assert_eq!(frame.selected_text(word.0, word.1), "世界");
        let combining = frame.selection_range(40, SelectionUnit::Word).unwrap();
        assert_eq!(frame.selected_text(combining.0, combining.1), "e\u{301}");
        let emoji = frame.selection_range(43, SelectionUnit::Cell).unwrap();
        assert_eq!(frame.selected_text(emoji.0, emoji.1), "🦀");
        let line = frame.selection_range(43, SelectionUnit::Line).unwrap();
        assert_eq!(frame.selected_text(line.0, line.1), "e\u{301} 🦀 tail");
        assert!(frame.selection_range(120, SelectionUnit::Word).is_none());
    }

    #[test]
    fn cropped_snapshot_keeps_cursor_and_copy_coordinates() {
        let mut terminal = Terminal::new(ghostty_vt::TerminalOptions {
            cols: 12,
            rows: 5,
            ..Default::default()
        })
        .unwrap();
        terminal.vt_write(b"old\r\nolder\r\nrecent\r\nnewest\r\nprompt");
        let mut renderer = Renderer::new().unwrap();
        let frame = renderer.frame(&terminal).unwrap();
        let start = frame.display_row_start(3);
        assert_eq!(start, 2);
        assert_eq!(frame.selected_text(start * 12, start * 12 + 5), "recent");
        assert_eq!(frame.cursor.unwrap().1 as usize - start, 2);
        assert_eq!(frame.display_row_start(8), 0);

        // A TUI cursor on an older row takes precedence over bottom alignment.
        terminal.vt_write(b"\x1b[2;1H");
        assert_eq!(renderer.frame(&terminal).unwrap().display_row_start(3), 1);
        terminal.vt_write(b"\x1b[?25l");
        let hidden_cursor = renderer.frame(&terminal).unwrap();
        assert!(hidden_cursor.cursor.is_none());
        assert_eq!(hidden_cursor.display_row_start(3), 2);
    }

    #[test]
    fn programs_choose_the_cursor_shape() {
        let mut terminal = Terminal::new(ghostty_vt::TerminalOptions {
            cols: 12,
            rows: 2,
            ..Default::default()
        })
        .unwrap();
        let mut renderer = Renderer::new().unwrap();
        assert_eq!(
            renderer.frame(&terminal).unwrap().cursor_shape,
            CursorShape::Block
        );
        for (sequence, shape) in [
            (&b"\x1b[5 q"[..], CursorShape::Bar),
            (b"\x1b[6 q", CursorShape::Bar),
            (b"\x1b[3 q", CursorShape::Underline),
            (b"\x1b[4 q", CursorShape::Underline),
            (b"\x1b[2 q", CursorShape::Block),
        ] {
            terminal.vt_write(sequence);
            assert_eq!(renderer.frame(&terminal).unwrap().cursor_shape, shape);
        }
    }
    #[test]
    fn the_default_cursor_shows_until_a_program_chooses_and_again_when_it_resets() {
        let mut terminal = Terminal::new(ghostty_vt::TerminalOptions {
            cols: 12,
            rows: 2,
            ..Default::default()
        })
        .unwrap();
        let mut renderer = Renderer::new().unwrap();
        let mut shape = |terminal: &Terminal| renderer.frame(terminal).unwrap().cursor_shape;
        set_default_cursor(&mut terminal, CursorShape::Bar).unwrap();
        assert_eq!(shape(&terminal), CursorShape::Bar, "nothing was asked for");
        terminal.vt_write(b"\x1b[2 q");
        assert_eq!(shape(&terminal), CursorShape::Block, "the program's choice");
        // A new default waits for the program to let go of its own.
        set_default_cursor(&mut terminal, CursorShape::Underline).unwrap();
        assert_eq!(shape(&terminal), CursorShape::Block);
        terminal.vt_write(b"\x1b[0 q");
        assert_eq!(shape(&terminal), CursorShape::Underline);
        set_default_cursor(&mut terminal, CursorShape::Block).unwrap();
        assert_eq!(shape(&terminal), CursorShape::Block);
    }
    #[test]
    fn a_restored_terminal_takes_the_clients_cursor_unless_its_program_chose_one() {
        let options = || ghostty_vt::TerminalOptions {
            cols: 12,
            rows: 2,
            ..Default::default()
        };
        let mut renderer = Renderer::new().unwrap();
        let mut shape = |stream: &TerminalStream| {
            renderer
                .frame(stream.terminal.as_ref().unwrap())
                .unwrap()
                .cursor_shape
        };

        // A server terminal whose program asked for nothing: the snapshot says so.
        let untouched = Terminal::new(options()).unwrap();
        let mut stream = TerminalStream::new(42);
        stream.set_cursor_style(CursorShape::Bar).unwrap();
        stream
            .feed(&ghostty_vt::snapshot::encode(&untouched).unwrap())
            .unwrap();
        assert_eq!(shape(&stream), CursorShape::Bar);
        // Live output: the program takes a block, then gives it up.
        stream.feed(b"\x1b[2 q").unwrap();
        assert_eq!(shape(&stream), CursorShape::Block);
        stream.feed(b"\x1b[0 q").unwrap();
        assert_eq!(shape(&stream), CursorShape::Bar);
        // A new choice shows at once, with no output.
        stream.set_cursor_style(CursorShape::Underline).unwrap();
        assert_eq!(shape(&stream), CursorShape::Underline);

        // A server terminal whose program chose a block before this client attached.
        let mut chosen = Terminal::new(options()).unwrap();
        chosen.vt_write(b"\x1b[2 q");
        let mut stream = TerminalStream::new(42);
        stream.set_cursor_style(CursorShape::Bar).unwrap();
        stream
            .feed(&ghostty_vt::snapshot::encode(&chosen).unwrap())
            .unwrap();
        assert_eq!(shape(&stream), CursorShape::Block, "the program's own");
        stream.feed(b"\x1b[0 q").unwrap();
        assert_eq!(shape(&stream), CursorShape::Bar);
    }
    #[test]
    fn copy_preserves_graphemes_without_wide_cell_padding() {
        let mut terminal = Terminal::new(ghostty_vt::TerminalOptions {
            cols: 20,
            rows: 3,
            ..Default::default()
        })
        .unwrap();
        terminal.vt_write("界 e\u{301}\r\nsecond".as_bytes());
        let frame = Renderer::new().unwrap().frame(&terminal).unwrap();
        assert_eq!(frame.selected_text(0, 25), "界 e\u{301}\nsecond");
        assert_eq!(frame.selected_text(25, 0), "界 e\u{301}\nsecond");
    }

    #[test]
    fn underline_styles_colours_and_overlines_reach_the_frame() {
        use ghostty_vt::style::Underline;
        let mut terminal = Terminal::new(ghostty_vt::TerminalOptions {
            cols: 12,
            rows: 2,
            ..Default::default()
        })
        .unwrap();
        terminal.vt_write(b"\x1b[4:3;58:2::1:2:3mA\x1b[0;4:2;58:5:1mB\x1b[0;4:4mC\x1b[0;53mD");
        let frame = Renderer::new().unwrap().frame(&terminal).unwrap();
        let cell = |text: &str| frame.cells[0].iter().find(|c| c.text == text).unwrap();
        assert_eq!(cell("A").style.underline, Underline::Curly);
        assert_eq!(cell("A").underline_color, Some(0x010203));
        assert_eq!(cell("B").style.underline, Underline::Double);
        assert!(cell("B").underline_color.is_some());
        assert_eq!(cell("C").style.underline, Underline::Dotted);
        assert_eq!(cell("C").underline_color, None);
        assert!(cell("D").style.overline);
        assert_eq!(cell("D").style.underline, Underline::None);
    }

    #[test]
    fn snapshot_with_kitty_image_in_history_installs_a_terminal() {
        // A Kitty image payload record follows READY in the snapshot; decoding every record
        // kind (not only page progress) is what makes such a snapshot usable.
        let mut terminal = Terminal::new(ghostty_vt::TerminalOptions {
            cols: 10,
            rows: 3,
            max_scrollback: 1 << 20,
        })
        .unwrap();
        terminal.set_kitty_image_storage_limit(1 << 20).unwrap();
        terminal.vt_write(b"\x1b_Ga=T,f=24,s=1,v=1;CQkJ\x1b\\\r\n");
        for line in 0..2000 {
            terminal.vt_write(format!("line {line}\r\n").as_bytes());
        }
        let snapshot = ghostty_vt::snapshot::encode(&terminal).unwrap();
        let mut stream = TerminalStream::new(42);
        stream.feed(&snapshot).unwrap();
        assert!(stream.terminal.is_some());
        assert!(stream.text().unwrap().contains("line 0\n"));
    }

    #[test]
    fn corrupted_snapshot_never_installs_a_terminal() {
        let terminal = Terminal::new(ghostty_vt::TerminalOptions::default()).unwrap();
        let mut bytes = ghostty_vt::snapshot::encode(&terminal).unwrap().to_vec();
        bytes[16] ^= 1; // First record's CRC, not a transport boundary.
        let mut stream = TerminalStream::new(42);
        assert!(stream.feed(&bytes).is_err());
        assert!(stream.terminal.is_none());
    }
    #[test]
    fn synchronized_output_holds_rendering_for_at_most_the_limit() {
        let terminal = Terminal::new(ghostty_vt::TerminalOptions::default()).unwrap();
        let mut stream = TerminalStream::new(42);
        stream
            .feed(&ghostty_vt::snapshot::encode(&terminal).unwrap())
            .unwrap();
        let start = Instant::now();
        assert!(!stream.render_held(start).unwrap());
        // A redraw: begin the update and clear, then (later) draw and end it.
        stream.feed(b"\x1b[?2026h\x1b[2J").unwrap();
        assert!(
            stream.render_held(start).unwrap(),
            "the cleared screen stays hidden"
        );
        stream.feed(b"redrawn\x1b[?2026l").unwrap();
        assert!(
            !stream.render_held(start).unwrap(),
            "the finished update draws"
        );

        stream.feed(b"\x1b[?2026h").unwrap();
        assert!(stream.render_held(start).unwrap());
        let late = start + RENDER_HOLD_LIMIT;
        assert!(
            !stream.render_held(late).unwrap(),
            "a hold never outlasts the limit"
        );
        assert!(
            !stream.render_held(late).unwrap(),
            "and is ended, not restarted"
        );
    }
    #[test]
    fn every_apc_split_and_false_nonce() {
        let message = b"a\x1b_rex1;123;{\"type\":\"resize\",\"cols\":80,\"rows\":24}\x1b\\z";
        for split in 0..=message.len() {
            let mut parser = ApcParser::new(123);
            let mut parts = parser.feed(&message[..split]).unwrap();
            parts.extend(parser.feed(&message[split..]).unwrap());
            let text: Vec<u8> = parts
                .iter()
                .filter_map(|p| {
                    if let Part::Output(b) = p {
                        Some(b.as_slice())
                    } else {
                        None
                    }
                })
                .flatten()
                .copied()
                .collect();
            assert_eq!(text, b"az");
            assert_eq!(
                parts
                    .iter()
                    .filter(|p| matches!(p, Part::Metadata(_)))
                    .count(),
                1
            );
        }
        let mut parser = ApcParser::new(456);
        assert_eq!(
            parser.feed(message).unwrap(),
            vec![Part::Output(message.to_vec())]
        );
    }
    #[test]
    fn snapshot_tail_and_parser_continuation_survive_fragmentation() {
        let mut terminal = Terminal::new(ghostty_vt::TerminalOptions {
            cols: 20,
            rows: 3,
            ..Default::default()
        })
        .unwrap();
        terminal.set_continuation_max_bytes(1024).unwrap();
        terminal.vt_write(b"before\x1b[31");
        let snapshot = ghostty_vt::snapshot::encode(&terminal).unwrap();
        let mut bytes = snapshot.to_vec();
        bytes.extend_from_slice(
            b"mAFTER\x1b_rex1;42;{\"type\":\"resize\",\"cols\":30,\"rows\":4}\x1b\\",
        );
        for size in [1, 7, 10, 64, 1024, bytes.len()] {
            let mut stream = TerminalStream::new(42);
            for chunk in bytes.chunks(size) {
                stream.feed(chunk).unwrap();
            }
            assert_eq!(stream.received, bytes.len() as u64);
            assert_eq!(stream.snapshot_count, 1);
            assert!(stream.text().unwrap().contains("beforeAFTER"));
            assert_eq!(stream.terminal.as_ref().unwrap().cols().unwrap(), 30);
        }
    }
}
