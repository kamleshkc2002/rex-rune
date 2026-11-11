//! One terminal pane: its stream, local view state (selection, preedit, retained-frame offset)
//! and the cell grid, drawn on a fixed column grid.
use super::{
    keys,
    sprites::Sprite,
    theme::{MIDNIGHT, Palette, mix, rgb},
};
use ghostty_vt::style::Underline;
use rex_linux::{
    connection::{Command, Connection},
    links::Link,
    terminal::{
        Cell, CursorShape, Frame, SelectionUnit,
        graphics::{Image as TermImage, ImagePixels},
    },
    terminal_theme::{TerminalTheme, is_light},
};
use rune::{
    El, Family, LineCap, Mods, Path, Shape, Span, TextEdit, TextStyle, canvas, el, memo, rich, text,
};
use serde_json::json;
use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
    time::{Duration, Instant},
};

pub struct Term {
    pub connection: Connection,
    pub frame: Option<Arc<Frame>>,
    pub last_size: Option<(u16, u16)>,
    /// The grid's rect in window points (inside the padding).
    pub rect: [f32; 4],
    /// While this client owns a live pane, its frames stay hidden until one arrives at this
    /// grid's size.
    initial_size: InitialSize,
    pub selection: Option<(usize, usize)>,
    selection_anchor: Option<(usize, usize)>,
    selection_unit: SelectionUnit,
    pub selecting: bool,
    pub(super) selection_revision: u64,
    last_selection_input: Option<rex_linux::terminal::selection::Input>,
    restore_generation: u64,
    pub preedit: String,
    pub notice: Option<String>,
    pub scroll: f32,
    /// A button press forwarded to the program, until its release: the gesture keeps going to
    /// the program even if the program stops tracking or Shift goes down mid-drag.
    pub mouse: Option<MouseGesture>,
    /// The last cell a motion report named, so moves within a cell send nothing.
    pub mouse_cell: Option<(u16, u16)>,
    images: HashMap<usize, (Arc<ImagePixels>, rune::Image)>,
    /// The link under the pointer while Ctrl is held, with the cell the pointer is on.
    pub link: Option<(usize, Link)>,
    /// The link a Ctrl+press went down on; releasing over the same link opens it.
    pub pressed_link: Option<String>,
    /// The search field over the grid, while this terminal is being searched.
    pub search: Option<TextEdit>,
}

#[derive(Clone, Copy)]
pub struct MouseGesture {
    pub button: &'static str,
}

/// The first frame of a pane is at whatever size the server last had for it; the resize this
/// client sends only lands a moment later. During each uninterrupted live ownership epoch, an
/// owner hides frames until one matches its first grid size. A non-owner never resizes, so its
/// frames may never match and are drawn as they come.
/// A frame at the asked size may never come (the server clamps or drops a resize), so the
/// wait is bounded: after this long the frame is drawn as it is, which is at worst the brief
/// flash of a stale size the gate exists to avoid.
const INITIAL_SIZE_PATIENCE: Duration = Duration::from_millis(500);

#[derive(Default)]
struct InitialSize {
    expected: Option<(u16, u16)>,
    ready: bool,
    live_owner: bool,
    /// When the wait began, for the bound above.
    since: Option<Instant>,
}

impl InitialSize {
    fn transition(&mut self, status: &Status) {
        let live_owner = status.live && status.owner;
        if live_owner != self.live_owner {
            self.expected = None;
            self.ready = false;
            self.since = None;
            self.live_owner = live_owner;
        }
    }

    fn layout(&mut self, size: (u16, u16), frame: Option<(u16, u16)>) {
        if !self.live_owner || self.ready {
            return;
        }
        self.expected = Some(size);
        self.since.get_or_insert_with(Instant::now);
        self.observe(frame);
    }

    fn observe(&mut self, frame: Option<(u16, u16)>) {
        if self.expected.is_some() && self.expected == frame {
            self.expected = None;
            self.since = None;
            self.ready = true;
        }
    }

    fn renders(&self, status: &Status) -> bool {
        self.renders_at(status, Instant::now())
    }

    fn renders_at(&self, status: &Status, now: Instant) -> bool {
        !status.live
            || !status.owner
            || (self.live_owner && self.ready)
            || self.deadline().is_some_and(|deadline| now >= deadline)
    }

    /// When the wait gives up, while one is on.
    fn deadline(&self) -> Option<Instant> {
        self.since
            .filter(|_| self.live_owner && !self.ready)
            .map(|since| since + INITIAL_SIZE_PATIENCE)
    }

    /// Ends a wait past its bound for good; true when that just happened.
    fn give_up(&mut self, now: Instant) -> bool {
        let due = self.deadline().is_some_and(|deadline| now >= deadline);
        if due {
            self.expected = None;
            self.since = None;
            self.ready = true;
        }
        due
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Metrics {
    pub cell: (f32, f32),
    pub style: TextStyle,
    /// Device pixels per point the grid is drawn at, so box drawing lands on whole pixels:
    /// 1 from `metrics`, the window's own in the pane view.
    pub px: f32,
}

const FEATURES: &str = "'liga' 0, 'calt' 0, 'clig' 0, 'kern' 0";

pub fn metrics(size: f32) -> Metrics {
    let base = TextStyle::new(size, 400.)
        .lh((size * 1.4).ceil())
        .features(FEATURES);
    // The mono slot keeps the primary face first and supplies missing glyphs from fallbacks.
    let style = base.mono();
    let width = rune::text::advance("M", style).max(1.);
    Metrics {
        cell: (width, (size * 1.4).ceil()),
        style,
        px: 1.,
    }
}

pub struct Status {
    pub live: bool,
    pub owner: bool,
    pub status: String,
}

impl Term {
    pub fn new(connection: Connection) -> Self {
        let (frame, restore_generation) = {
            let model = connection.model.lock().expect("terminal model");
            (model.frame.clone(), model.restore_generation)
        };
        Self {
            connection,
            frame,
            last_size: None,
            rect: [0.; 4],
            initial_size: InitialSize::default(),
            selection: None,
            selection_anchor: None,
            selection_unit: SelectionUnit::Cell,
            selecting: false,
            selection_revision: 0,
            last_selection_input: None,
            restore_generation,
            preedit: String::new(),
            notice: None,
            scroll: 0.,
            mouse: None,
            mouse_cell: None,
            images: HashMap::new(),
            link: None,
            pressed_link: None,
            search: None,
        }
    }

    pub fn refresh(&mut self) {
        let (frame, generation, notice) = {
            let mut model = self.connection.model.lock().expect("terminal model");
            (
                model.frame.clone(),
                model.restore_generation,
                model.notice.take(),
            )
        };
        if notice.is_some() {
            self.notice = notice;
        }
        if generation != self.restore_generation {
            self.selection = None;
            self.selection_anchor = None;
            self.selecting = false;
            self.last_selection_input = None;
            self.restore_generation = generation;
        }
        if let Some(frame) = &frame {
            if frame.selection_error != self.frame.as_ref().and_then(|f| f.selection_error.clone())
            {
                self.notice = frame.selection_error.clone();
            }
            if frame.selection_revision == self.selection_revision {
                if frame.selection_text.is_some() && self.selection.is_none() {
                    self.selection = Some((0, 0));
                } else if !self.selecting && frame.selection_text.is_none() {
                    self.selection = None;
                }
            }
        }
        self.initial_size
            .observe(frame.as_ref().map(|frame| (frame.cols, frame.rows)));
        self.frame = frame;
        // The same cell may hold a different link, or none, after output.
        if let Some((index, _)) = self.link.take() {
            self.link = self
                .frame
                .as_ref()
                .and_then(|f| f.link_at(index))
                .map(|l| (index, l));
        }
    }

    pub fn cache_images(&mut self, cache: Option<&rune::ImageCache>) {
        let (Some(cache), Some(frame)) = (cache, &self.frame) else {
            self.images.clear();
            return;
        };
        // Keeping the pixel Arc beside the texture prevents pointer reuse from making
        // a fresh snapshot look like the previous image with the same protocol id.
        for image in &frame.images {
            let pixels = &image.pixels;
            self.images
                .entry(Arc::as_ptr(pixels) as usize)
                .or_insert_with(|| {
                    (
                        Arc::clone(pixels),
                        cache.rgba(pixels.width, pixels.height, Arc::clone(&pixels.rgba)),
                    )
                });
        }
        self.images.retain(|key, _| {
            frame
                .images
                .iter()
                .any(|image| Arc::as_ptr(&image.pixels) as usize == *key)
        });
    }

    fn display_frame(&self) -> Option<&Arc<Frame>> {
        if self.initial_size.renders(&self.status()) {
            self.frame.as_ref()
        } else {
            None
        }
    }

    /// When this pane stops waiting for a frame at its size and draws anyway, while it waits.
    pub fn draws_anyway_at(&self) -> Option<Instant> {
        self.initial_size.deadline()
    }

    /// Stops waiting for a frame at this pane's size once the bound has passed; true the one
    /// time that changes what it draws.
    pub fn draw_anyway(&mut self, now: Instant) -> bool {
        self.initial_size.give_up(now)
    }

    pub fn status(&self) -> Status {
        let m = self.connection.model.lock().expect("terminal model");
        Status {
            live: m.connected && m.control_connected,
            owner: m.owner,
            status: m.status.clone(),
        }
    }

    pub fn send(&mut self, command: Command) {
        if let Err(error) = self.connection.commands.try_send(command) {
            self.notice = Some(format!("Input not sent: {error}"));
        }
    }

    pub fn visible_rows(&self, cell: (f32, f32)) -> usize {
        (self.rect[3] / cell.1).floor().max(1.) as usize
    }

    pub fn display_row(&self, cell: (f32, f32)) -> usize {
        self.frame
            .as_ref()
            .map_or(0, |f| f.display_row_start(self.visible_rows(cell)))
    }

    pub fn hidden_rows(&self, cell: (f32, f32)) -> usize {
        if self.rect[3] <= 0. {
            return 0;
        }
        self.frame.as_ref().map_or(0, |f| {
            (f.rows as usize).saturating_sub(self.visible_rows(cell))
        })
    }

    /// A non-owner never resizes; an owner sends its grid size whenever it changes.
    pub fn resize_if_owner(&mut self, font: f32) {
        let status = self.status();
        self.initial_size.transition(&status);
        if self.rect[2] <= 0. || self.rect[3] <= 0. {
            return;
        }
        let Metrics { cell, .. } = metrics(font);
        let cols = (self.rect[2] / cell.0).floor().clamp(1., 65535.) as u16;
        let rows = (self.rect[3] / cell.1).floor().clamp(1., 65535.) as u16;
        self.initial_size.layout(
            (cols, rows),
            self.frame.as_ref().map(|frame| (frame.cols, frame.rows)),
        );
        if !status.owner || !status.live {
            self.last_size = None;
        } else if self.last_size != Some((cols, rows)) {
            self.last_size = Some((cols, rows));
            self.send(Command::Resize(
                cols,
                rows,
                cell.0.ceil() as u16,
                cell.1.ceil() as u16,
            ));
        }
    }

    /// The cell index under a point in the grid's own coordinates.
    pub fn index(&self, x: f32, y: f32, cell: (f32, f32)) -> usize {
        let cols = self.frame.as_ref().map_or(1, |f| f.cols) as usize;
        let rows = self.frame.as_ref().map_or(1, |f| f.rows) as usize;
        let col = (x / cell.0).max(0.) as usize;
        let row = ((y / cell.1).max(0.) as usize).min(self.visible_rows(cell).saturating_sub(1));
        (row + self.display_row(cell)).min(rows - 1) * cols + col.min(cols - 1)
    }

    /// The program has asked for mouse reports.
    pub fn tracking(&self) -> bool {
        self.frame.as_ref().is_some_and(|f| f.mouse_tracking)
    }

    /// The openable link under a point in the grid's own coordinates, with its cell index.
    pub fn link_at(&self, x: f32, y: f32, cell: (f32, f32)) -> Option<(usize, Link)> {
        let index = self.index(x, y, cell);
        self.frame.as_ref()?.link_at(index).map(|l| (index, l))
    }

    /// The server's cell (column, row) under a point in the grid's own coordinates, clamped
    /// to the grid, so a drag past the edge reports the edge cell.
    pub fn cell_at(&self, x: f32, y: f32, cell: (f32, f32)) -> (u16, u16) {
        let cols = self.frame.as_ref().map_or(1, |f| f.cols).max(1);
        let rows = self.frame.as_ref().map_or(1, |f| f.rows).max(1);
        let col = (x / cell.0).floor().max(0.) as usize;
        let row = (y / cell.1).floor().max(0.) as usize + self.display_row(cell);
        (
            col.min(cols as usize - 1) as u16,
            row.min(rows as usize - 1) as u16,
        )
    }

    /// One mouse report. Shift never goes along: it is the local-selection override, and a
    /// Shift press leaves the server's button reporting stuck for the block.
    pub fn send_mouse(&mut self, action: &str, button: Option<&str>, at: (u16, u16), mods: Mods) {
        let mut payload = json!({
            "action": action,
            "x": at.0,
            "y": at.1,
            "mods": keys::mods(Mods { shift: false, ..mods }),
        });
        if let Some(button) = button {
            payload["button"] = json!(button);
        }
        self.mouse_cell = Some(at);
        self.send(Command::Mouse(payload));
    }

    /// A wheel turn of whole lines at a cell: negative scrolls up, positive down.
    pub fn send_wheel(&mut self, at: (u16, u16), lines: i32) {
        let step = lines.signum();
        for _ in 0..lines.unsigned_abs().min(8) {
            self.send(Command::Wheel(
                json!({"x": at.0, "y": at.1, "delta_x": 0, "delta_y": step}),
            ));
        }
    }

    pub fn begin_selection(&mut self, index: usize, clicks: u32) {
        self.selection_unit = match clicks {
            0 | 1 => SelectionUnit::Cell,
            2 => SelectionUnit::Word,
            _ => SelectionUnit::Line,
        };
        self.selection_anchor = self
            .frame
            .as_ref()
            .and_then(|f| f.selection_range(index, self.selection_unit));
        // A focus click alone must not copy the character under the pointer.
        self.selection = (self.selection_unit != SelectionUnit::Cell)
            .then_some(self.selection_anchor)
            .flatten();
    }

    pub fn extend_selection(&mut self, index: usize) {
        let Some(anchor) = self.selection_anchor else {
            return;
        };
        let Some(target) = self
            .frame
            .as_ref()
            .and_then(|f| f.selection_range(index, self.selection_unit))
        else {
            return;
        };
        self.selection = if self.selection_unit == SelectionUnit::Cell && anchor == target {
            None
        } else {
            Some((anchor.0.min(target.0), anchor.1.max(target.1)))
        };
    }

    pub fn selection_event(
        &mut self,
        phase: rex_linux::terminal::selection::Phase,
        x: f32,
        y: f32,
        cell: (f32, f32),
    ) {
        use rex_linux::terminal::selection::{Geometry, Input, Phase};
        let index = self.index(x, y, cell);
        let cols = usize::from(self.frame.as_ref().map_or(1, |f| f.cols));
        self.selection_revision = self.selection_revision.wrapping_add(1);
        let input = Input {
            generation: self.restore_generation,
            revision: self.selection_revision,
            phase,
            at: ((index % cols) as u16, (index / cols) as u16),
            position: (f64::from(x), f64::from(y)),
            geometry: Geometry {
                columns: cols as u32,
                cell_width: cell.0.ceil().max(1.) as u32,
                padding_left: 0,
                screen_height: self.rect[3].ceil().max(1.) as u32,
            },
        };
        self.selecting = matches!(phase, Phase::Down(_) | Phase::Move);
        self.last_selection_input = Some(input);
        self.send(Command::Selection(input));
    }

    pub fn end_selection(&mut self) {
        if self.selecting
            && let Some(mut input) = self.last_selection_input
        {
            self.selection_revision = self.selection_revision.wrapping_add(1);
            input.revision = self.selection_revision;
            input.phase = rex_linux::terminal::selection::Phase::Up;
            self.send(Command::Selection(input));
        }
        self.selecting = false;
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
        self.selection_anchor = None;
        self.selecting = false;
        if let Some(mut input) = self.last_selection_input.take() {
            self.selection_revision = self.selection_revision.wrapping_add(1);
            input.revision = self.selection_revision;
            input.phase = rex_linux::terminal::selection::Phase::Clear;
            self.send(Command::Selection(input));
        }
    }

    pub fn copy_text(&self) -> Option<String> {
        let (a, b) = self.selection?;
        let frame = self.frame.as_ref()?;
        if frame.selection_revision == self.selection_revision {
            return frame.selection_text.as_ref().map(ToString::to_string);
        }
        Some(frame.selected_text(a, b))
    }
}

/// How the cursor draws: owning the keyboard (the shape the program asked for, a filled block
/// by default), visible but not (hollow), or not at all.
#[derive(Clone, Copy, Hash, PartialEq)]
pub enum CursorLook {
    Block,
    Hollow,
}

fn hash_cell(cell: &Cell, h: &mut DefaultHasher) {
    cell.text.hash(h);
    (cell.fg, cell.bg, cell.underline_color, cell.spacer_tail).hash(h);
    let s = &cell.style;
    (
        s.bold,
        s.italic,
        s.faint,
        s.strikethrough,
        s.underline,
        s.overline,
    )
        .hash(h);
}

#[derive(Clone, Copy, PartialEq)]
struct RunStyle {
    fg: u32,
    bold: bool,
    italic: bool,
    faint: bool,
    underline: Underline,
    /// None draws the underline in `fg`.
    underline_color: Option<u32>,
    overline: bool,
    strike: bool,
}
impl RunStyle {
    /// `default_bg` is the server's default background, which draws as `body` wherever it
    /// shows (inverse video's foreground included).
    fn of(cell: &Cell, default_bg: u32, body: u32) -> Self {
        let s = &cell.style;
        let tint = |c: u32| if c == default_bg { body } else { c };
        Self {
            // Faint text is pre-mixed toward its cell in sRGB.
            fg: if s.faint {
                mix(tint(cell.fg), tint(cell.bg), 0.65)
            } else {
                tint(cell.fg)
            },
            bold: s.bold,
            italic: s.italic,
            faint: s.faint,
            underline: s.underline,
            underline_color: cell.underline_color.map(tint),
            overline: s.overline,
            strike: s.strikethrough,
        }
    }
}

fn glyph_run(content: String, style: RunStyle, x: f32, y: f32, m: &Metrics) -> El {
    let font = face(m, style.bold, style.italic);
    let color = rgb(style.fg);
    let len = content.len();
    let mut e = text(content);
    if style.strike {
        e = e.strike(0, len, Some(color));
    }
    e.no_wrap().font(font).color(color).absolute(x, y)
}

/// (character, bold, italic, size bits, family) → advances exactly one cell.
type FitCache = std::collections::HashMap<(char, bool, bool, u32, &'static str), bool>;

thread_local! {
    /// Whether a character, in a given face, advances exactly one cell: only those join
    /// multi-character segments, so nothing after a fallback or wider glyph drifts.
    static FITS: std::cell::RefCell<FitCache> = Default::default();
}

fn face(m: &Metrics, bold: bool, italic: bool) -> TextStyle {
    let mut font = m.style;
    if bold {
        font.weight = 700.;
    }
    if italic {
        font = font.italic();
    }
    font
}

fn fits_cell(ch: char, bold: bool, italic: bool, m: &Metrics) -> bool {
    if ch.is_ascii_graphic() || ch == ' ' {
        return true;
    }
    let family = match m.style.family {
        Family::Named(name) => name,
        _ => "",
    };
    let key = (ch, bold, italic, m.style.size.to_bits(), family);
    FITS.with_borrow_mut(|cache| {
        *cache.entry(key).or_insert_with(|| {
            let advance = rune::text::advance(&ch.to_string(), face(m, bold, italic));
            (advance - m.cell.0).abs() < 0.01
        })
    })
}

/// One text element for a run of single-cell characters starting at column `col`, with a span
/// per style change, so colours and weights never add elements.
struct Segment {
    col: usize,
    text: String,
    spans: Vec<Span>,
    strikes: Vec<(usize, usize, u32)>,
    style: Option<RunStyle>,
    start: usize,
}

impl Segment {
    fn new(col: usize) -> Self {
        Self {
            col,
            text: String::new(),
            spans: Vec::new(),
            strikes: Vec::new(),
            style: None,
            start: 0,
        }
    }
    fn close_span(&mut self) {
        if let Some(style) = self.style.take() {
            let end = self.text.len();
            if end > self.start {
                let color = rgb(style.fg);
                let mut span = Span::new(self.start, end);
                span.color = Some(color);
                if style.bold {
                    span.weight = Some(700.);
                }
                span.italic = style.italic;
                self.spans.push(span);
                if style.strike {
                    self.strikes.push((self.start, end, style.fg));
                }
            }
        }
    }
    fn push(&mut self, text: &str, style: RunStyle) {
        if self.style != Some(style) {
            self.close_span();
            self.style = Some(style);
            self.start = self.text.len();
        }
        self.text.push_str(text);
    }
    fn finish(mut self, m: &Metrics) -> Option<El> {
        self.close_span();
        // Trailing blanks need no glyph: underlines and overlines are drawn on their own.
        while self.text.ends_with(' ')
            && !self.strikes.iter().any(|&(_, e, _)| e == self.text.len())
        {
            self.text.pop();
            if let Some(last) = self.spans.last_mut() {
                last.end = last.end.min(self.text.len() as u32);
                if last.end <= last.start {
                    self.spans.pop();
                }
            }
        }
        if self.text.trim().is_empty() && self.strikes.is_empty() {
            return None;
        }
        let mut e = rich(self.text, self.spans)
            // The server owns line breaks. Fractional-DPI rounding must not wrap a run again.
            .no_wrap()
            .font(m.style)
            .absolute(self.col as f32 * m.cell.0, 0.);
        for (a, b, fg) in self.strikes {
            e = e.strike(a, b, Some(rgb(fg)));
        }
        Some(e)
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq)]
enum RowPass {
    All,
    Background,
    Text,
}

/// What the client marks on a row itself, as inclusive columns: the selection, and search
/// matches with whether each is the selected match.
#[derive(Default, Hash)]
struct Marks<'a> {
    selection: Option<(usize, usize)>,
    search: &'a [(usize, usize, bool)],
}

#[cfg(test)]
fn row_el(cells: &[Cell], y: f32, frame_bg: u32, sel: Option<(usize, usize)>, m: &Metrics) -> El {
    let marks = Marks {
        selection: sel,
        ..Marks::default()
    };
    row_pass(
        cells,
        y,
        frame_bg,
        &marks,
        m,
        RowPass::All,
        &TermColors::default(),
    )
}

fn row_pass(
    cells: &[Cell],
    y: f32,
    frame_bg: u32,
    marks: &Marks<'_>,
    m: &Metrics,
    pass: RowPass,
    colors: &TermColors,
) -> El {
    let body = colors.body(frame_bg);
    let (cw, ch) = m.cell;
    let sel = marks.selection;
    // Memoized rows are flattened on their own, so each says again that it never animates:
    // otherwise a glyph run that leaves a changed row fades out over its replacement.
    let mut row = el().w_full().h(ch).absolute(0., y).instant_changes();
    // Images below text belong between the background and glyph passes.
    if pass != RowPass::Text {
        let mut start = 0;
        while start < cells.len() {
            let bg = cells[start].bg;
            let mut end = start + 1;
            while end < cells.len() && cells[end].bg == bg {
                end += 1;
            }
            if bg != frame_bg {
                row = row.child(
                    el().absolute(start as f32 * cw, 0.)
                        .size((end - start) as f32 * cw, ch)
                        .fill(rgb(bg)),
                );
            }
            start = end;
        }
        for &(a, b, selected) in marks.search {
            let alpha = if selected { 0.55 } else { 0.25 };
            row = row.child(
                el().absolute(a as f32 * cw, 0.)
                    .size((b + 1 - a) as f32 * cw, ch)
                    .fill(rgb(colors.search_match).alpha(alpha)),
            );
        }
        if let Some((a, b)) = sel {
            row = row.child(
                el().absolute(a as f32 * cw, 0.)
                    .size((b + 1 - a) as f32 * cw, ch)
                    .fill(rgb(colors.selection_bg)),
            );
        }
    }
    if pass == RowPass::Background {
        return row;
    }
    // Selected text takes the theme's selection foreground, decorations included.
    let style_at = |col: usize| {
        let mut style = RunStyle::of(&cells[col], frame_bg, body);
        if let (Some(fg), Some((a, b))) = (colors.selection_fg, sel)
            && (a..=b).contains(&col)
        {
            style.fg = fg;
        }
        style
    };
    let mut segment: Option<Segment> = None;
    for (col, cell) in cells.iter().enumerate() {
        if cell.spacer_tail || cell.text.starts_with('\u{10eeee}') {
            continue;
        }
        let style = style_at(col);
        let text = if cell.text.is_empty() {
            " "
        } else {
            cell.text.as_str()
        };
        let mut chars = text.chars();
        let single = chars.next().filter(|_| chars.next().is_none());
        let wide = cells.get(col + 1).is_some_and(|c| c.spacer_tail);
        if let Some(sprite) = single.and_then(Sprite::of) {
            if let Some(done) = segment.take().and_then(|s| s.finish(m)) {
                row = row.child(done);
            }
            let width = if wide { 2. * cw } else { cw };
            row = sprite.draw(row, col as f32 * cw, (width, ch), m.px, rgb(style.fg));
            continue;
        }
        let fits = !wide && single.is_some_and(|c| fits_cell(c, style.bold, style.italic, m));
        if fits {
            if text == " " && segment.is_none() && !style.strike {
                continue;
            }
            segment
                .get_or_insert_with(|| Segment::new(col))
                .push(text, style);
        } else {
            if let Some(done) = segment.take().and_then(|s| s.finish(m)) {
                row = row.child(done);
            }
            row = row.child(glyph_run(text.to_owned(), style, col as f32 * cw, 0., m));
        }
    }
    if let Some(done) = segment.and_then(|s| s.finish(m)) {
        row = row.child(done);
    }
    // Underlines and overlines in the style and colour the program asked for, rather than
    // the font's single underline, one shape per run of cells that draw them alike.
    let mut start = 0;
    while start < cells.len() {
        let style = style_at(start);
        let mut end = start + 1;
        while end < cells.len() && same_decoration(&style_at(end), &style) {
            end += 1;
        }
        if !cells[start].text.starts_with('\u{10eeee}') {
            let width = (end - start) as f32 * cw;
            let shape = decoration_shape(&style, width, ch, m.px);
            if !shape.is_empty() {
                row = row.child(
                    canvas(shape)
                        .absolute(start as f32 * cw, 0.)
                        .size(width, ch),
                );
            }
        }
        start = end;
    }
    row
}

fn same_decoration(a: &RunStyle, b: &RunStyle) -> bool {
    (a.underline, a.underline_color, a.overline, a.fg)
        == (b.underline, b.underline_color, b.overline, b.fg)
}

/// A straight line's thickness in points: one point in whole device pixels, as sprites' are.
fn line_thickness(px: f32) -> f32 {
    px.round().max(1.) / px
}

/// The centre of a line that starts on a whole device pixel `top` points down the cell.
fn line_centre(top: f32, px: f32) -> f32 {
    (top * px).round() / px + line_thickness(px) / 2.
}

/// A run's underline (single, double, curly, dotted or dashed) and overline, `width` wide in a
/// cell `height` tall, drawn at `px` device pixels per point.
fn decoration_shape(style: &RunStyle, width: f32, height: f32, px: f32) -> Shape {
    let mut shape = Shape::new();
    let color = rgb(style.underline_color.unwrap_or(style.fg));
    let thick = line_thickness(px);
    let centre = |top: f32| line_centre(top, px);
    let y = centre(height - 2.);
    match style.underline {
        Underline::Single => {
            shape = shape.line([0., y], [width, y], thick, LineCap::Butt, color);
        }
        Underline::Double => {
            let upper = y - (2. * px).round() / px;
            shape = shape
                .line([0., upper], [width, upper], thick, LineCap::Butt, color)
                .line([0., y], [width, y], thick, LineCap::Butt, color);
        }
        Underline::Curly => {
            // Half waves 4 points long, alternately below and above the line.
            let mut path = Path::new().move_to([0., y]);
            let (mut x, mut below) = (0., true);
            while x < width {
                let end = (x + 4.).min(width);
                let bend = if below { 1.5 } else { -1.5 };
                path = path.quad_to([(x + end) / 2., y + bend], [end, y]);
                (x, below) = (end, !below);
            }
            shape = shape.stroke_path(&path, thick, LineCap::Round, color);
        }
        Underline::Dotted => {
            let mut x = 0.;
            while x < width {
                shape = shape.line([x, y], [x + 0.1, y], 1.5, LineCap::Round, color);
                x += 3.;
            }
        }
        Underline::Dashed => {
            let mut x = 0.;
            while x < width {
                shape = shape.line(
                    [x, y],
                    [(x + 4.).min(width), y],
                    thick,
                    LineCap::Butt,
                    color,
                );
                x += 7.;
            }
        }
        _ => {}
    }
    if style.overline {
        let y = centre(0.5);
        shape = shape.line([0., y], [width, y], thick, LineCap::Butt, rgb(style.fg));
    }
    shape
}

fn image_el(
    image: &TermImage,
    texture: &rune::Image,
    frame: &Frame,
    m: &Metrics,
    start: usize,
) -> El {
    let p = &image.placement;
    let scale = [
        m.cell.0 / frame.cell_pixels[0].max(1) as f32,
        m.cell.1 / frame.cell_pixels[1].max(1) as f32,
    ];
    let width = p.size[0] as f32 * scale[0];
    let height = p.size[1] as f32 * scale[1];
    let sx = width / p.source[2] as f32;
    let sy = height / p.source[3] as f32;
    el().absolute(
        p.cell[0] as f32 * m.cell.0 + p.offset[0] as f32 * scale[0],
        (p.cell[1] as f32 - start as f32) * m.cell.1 + p.offset[1] as f32 * scale[1],
    )
    .size(width, height)
    .clip_rounded(0.)
    .instant_changes()
    .child(
        el().absolute(-(p.source[0] as f32) * sx, -(p.source[1] as f32) * sy)
            .size(
                image.pixels.width as f32 * sx,
                image.pixels.height as f32 * sy,
            )
            .pixels(texture.pixels()),
    )
}

pub struct GridLook {
    pub cursor: Option<CursorLook>,
    pub ime: bool,
    pub colors: TermColors,
}

/// The colours the client draws in the terminal area itself. The terminal's own colours reach
/// the frame through the server; these are the default background's stand-in and the
/// selection, cursor-text and search colours, which the protocol has no field for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TermColors {
    /// The declared theme's background, else the card.
    pub background: u32,
    pub selection_bg: u32,
    pub selection_fg: Option<u32>,
    pub cursor_text: Option<u32>,
    /// Search matches' highlight: the chrome's accent.
    pub search_match: u32,
}
impl Default for TermColors {
    fn default() -> Self {
        Self::of(None, &MIDNIGHT)
    }
}
impl TermColors {
    /// For the theme this client declares, if any, in chrome `palette`.
    pub fn of(theme: Option<&TerminalTheme>, palette: &Palette) -> Self {
        let theme = theme.cloned().unwrap_or_default();
        Self {
            background: theme.background.unwrap_or(palette.card),
            selection_bg: theme
                .selection_background
                .unwrap_or(palette.terminal_selection),
            selection_fg: theme.selection_foreground,
            cursor_text: theme.cursor_text,
            search_match: palette.accent,
        }
    }

    /// What the server's default background `frame_bg` draws as: `background` when both are
    /// equally dark or light (Rex's built-in black, or another owner's theme), so a single
    /// pane reads as one surface with the chrome; otherwise the server's own, so the default
    /// text drawn for it stays readable. Only that exact colour is replaced, so explicit
    /// backgrounds and inverse video's block keep theirs.
    pub fn body(&self, frame_bg: u32) -> u32 {
        if is_light(frame_bg) == is_light(self.background) {
            self.background
        } else {
            frame_bg
        }
    }
}

impl Term {
    /// The fill behind this pane's grid.
    pub fn body(&self, colors: &TermColors) -> u32 {
        self.frame
            .as_ref()
            .map_or(colors.background, |f| colors.body(f.bg))
    }
}

/// The visible rows of `term`'s frame, each memoized by its content.
pub fn grid(key: &str, term: &Term, m: &Metrics, look: GridLook) -> El {
    let Some(frame) = term.display_frame() else {
        return el().w_full().h_full();
    };
    let (cw, ch) = m.cell;
    let visible = term.visible_rows(m.cell);
    let start = frame.display_row_start(visible);
    let cols = frame.cols as usize;
    let colors = &look.colors;
    let body = colors.body(frame.bg);
    let selection = term.selection.map(|(a, b)| (a.min(b), a.max(b)));
    let rows = |mut grid: El, pass: RowPass| {
        for (y, cells) in frame.cells.iter().enumerate().skip(start).take(visible) {
            let selected = if term.selection.is_some()
                && frame.selection_revision == term.selection_revision
            {
                frame.selection_rows.get(y).copied().flatten()
            } else {
                selection.and_then(|(a, b)| {
                    let (row0, row1) = (y * cols, y * cols + cols - 1);
                    (a <= row1 && b >= row0).then(|| (a.max(row0) - row0, b.min(row1) - row0))
                })
            };
            let marks = Marks {
                selection: selected,
                search: frame
                    .search
                    .as_ref()
                    .map(|search| search.on_row(y))
                    .unwrap_or_default(),
            };
            let mut h = DefaultHasher::new();
            for cell in cells {
                hash_cell(cell, &mut h);
            }
            (
                &marks,
                frame.bg,
                cw.to_bits(),
                ch.to_bits(),
                m.px.to_bits(),
                m.style,
                pass,
                colors,
            )
                .hash(&mut h);
            let top = (y - start) as f32 * ch;
            let id = format!("{key}/r{}:{pass:?}", y - start);
            grid = grid.child(memo(id, h.finish(), || {
                row_pass(cells, top, frame.bg, &marks, m, pass, colors)
            }));
        }
        grid
    };
    let images = |mut grid: El, layer: i32| {
        for (index, image) in frame
            .images
            .iter()
            .enumerate()
            .filter(|(_, image)| image.placement.layer == layer)
        {
            if let Some((_, texture)) = term.images.get(&(Arc::as_ptr(&image.pixels) as usize)) {
                grid = grid.child(
                    image_el(image, texture, frame, m, start).key(format!("{key}/image:{index}")),
                );
            }
        }
        grid
    };
    let mut grid = el().w_full().h_full().clip_rounded(0.).instant_changes();
    if frame.images.is_empty() {
        grid = rows(grid, RowPass::All);
    } else {
        grid = images(grid, 0);
        grid = rows(grid, RowPass::Background);
        grid = images(grid, 1);
        grid = rows(grid, RowPass::Text);
        grid = images(grid, 2);
    }
    if let Some((_, link)) = &term.link {
        for mark in link_marks(key, frame, link, start..start + visible, m, body) {
            grid = grid.child(mark);
        }
    }
    if let Some((x, y)) = frame.cursor
        && let Some(cursor_look) = look.cursor
        && (y as usize) >= start
        && (y as usize) < start + visible
    {
        let (px, py) = (x as f32 * cw, (y as usize - start) as f32 * ch);
        let cell = frame.cells.get(y as usize).and_then(|r| r.get(x as usize));
        let color = rgb(frame.cursor_color);
        // A pane without the keyboard always shows a hollow block, whatever the program asked.
        let shape = match cursor_look {
            CursorLook::Block => frame.cursor_shape,
            CursorLook::Hollow => CursorShape::Hollow,
        };
        let cursor = el().key(format!("{key}/cursor"));
        let mut cursor = match shape {
            CursorShape::Block => cursor.absolute(px, py).size(cw, ch).fill(color),
            CursorShape::Bar => cursor.absolute(px, py).size(2., ch).fill(color),
            CursorShape::Underline => cursor.absolute(px, py + ch - 2.).size(cw, 2.).fill(color),
            CursorShape::Hollow => cursor.absolute(px, py).size(cw, ch).border(1., color),
        };
        if look.ime {
            cursor = cursor.ime_cursor();
        }
        grid = grid.child(cursor);
        if shape == CursorShape::Block
            && let Some(cell) =
                cell.filter(|c| !c.text.is_empty() && !c.text.starts_with('\u{10eeee}'))
        {
            let style = RunStyle {
                fg: colors.cursor_text.unwrap_or(body),
                ..RunStyle::of(cell, frame.bg, body)
            };
            grid = grid.child(glyph_run(cell.text.clone(), style, px, py, m));
        }
        if !term.preedit.is_empty() {
            let len = term.preedit.len();
            grid = grid.child(
                rich(
                    term.preedit.clone(),
                    vec![Span {
                        underline: Some(rgb(frame.fg)),
                        ..Span::new(0, len)
                    }],
                )
                .no_wrap()
                .font(m.style)
                .color(rgb(frame.fg))
                .fill(rgb(body))
                .absolute(px, py),
            );
        }
    }
    grid
}

/// The link Ctrl+click would open, underlined in its own colour, row by row, over the
/// `rows` on screen. Its tooltip names the target itself, since OSC 8 text can show a
/// different address.
fn link_marks(
    key: &str,
    frame: &Frame,
    link: &Link,
    rows: std::ops::Range<usize>,
    m: &Metrics,
    body: u32,
) -> Vec<El> {
    let (cw, ch) = m.cell;
    let cols = frame.cols as usize;
    let help = format!("Ctrl+click to open {}", link.uri);
    let color = frame
        .cells
        .get(link.start / cols)
        .and_then(|row| row.get(link.start % cols))
        .map_or(frame.fg, |c| RunStyle::of(c, frame.bg, body).fg);
    (link.start / cols..=link.end / cols)
        .filter(|y| rows.contains(y))
        .map(|y| {
            let c0 = if y == link.start / cols {
                link.start % cols
            } else {
                0
            };
            let c1 = if y == link.end / cols {
                link.end % cols
            } else {
                cols - 1
            };
            let width = (c1 + 1 - c0) as f32 * cw;
            el().key(format!("{key}/link{y}"))
                .absolute(c0 as f32 * cw, (y - rows.start) as f32 * ch)
                .size(width, ch)
                .help(help.clone())
                .child(el().absolute(0., ch - 2.).size(width, 1.).fill(rgb(color)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Cx, Env, Runtime};

    struct RowState {
        cells: Vec<Cell>,
    }
    fn view(s: &RowState, _: &mut Cx<RowState>) -> El {
        let m = metrics(15.);
        el().w_full()
            .h_full()
            .child(row_el(&s.cells, 0., 0, None, &m))
    }

    #[test]
    fn an_owner_waits_once_per_live_ownership_epoch_and_a_non_owner_never_waits() {
        let status = |live, owner| Status {
            live,
            owner,
            status: String::new(),
        };
        let mut size = InitialSize::default();
        size.transition(&status(true, true));
        assert!(!size.renders(&status(true, true)), "hidden before layout");
        size.layout((80, 24), Some((160, 50)));
        assert!(
            !size.renders(&status(true, true)),
            "stale size stays hidden"
        );
        size.observe(Some((80, 24)));
        assert!(size.renders(&status(true, true)), "this size shows");
        size.layout((100, 30), Some((80, 24)));
        assert!(
            size.renders(&status(true, true)),
            "a later resize keeps drawing the previous frame"
        );

        size.transition(&status(true, false));
        assert!(
            size.renders(&status(true, false)),
            "a non-owner draws the size it gets"
        );
        size.transition(&status(true, true));
        size.layout((100, 30), Some((80, 24)));
        assert!(
            !size.renders(&status(true, true)),
            "taking ownership starts a new wait"
        );
        size.transition(&status(false, true));
        assert!(
            size.renders(&status(false, true)),
            "retained output stays visible offline"
        );
        size.transition(&status(true, true));
        size.layout((100, 30), Some((80, 24)));
        assert!(
            !size.renders(&status(true, true)),
            "reconnecting starts a new wait"
        );

        // A frame at the asked size may never come; the wait is bounded.
        let deadline = size.deadline().expect("a wait is on");
        assert!(
            !size.renders_at(&status(true, true), deadline - Duration::from_millis(1)),
            "hidden until the bound"
        );
        assert!(
            size.renders_at(&status(true, true), deadline),
            "drawn as it is once the bound passes"
        );
        size.layout((100, 30), Some((80, 24)));
        assert_eq!(
            size.deadline(),
            Some(deadline),
            "a relayout keeps the first wait's bound"
        );
        size.observe(Some((100, 30)));
        assert_eq!(size.deadline(), None, "no bound once the size arrived");
    }

    #[test]
    fn a_wait_that_runs_out_ends_once_even_if_the_size_never_comes() {
        let status = Status {
            live: true,
            owner: true,
            status: String::new(),
        };
        let mut size = InitialSize::default();
        size.transition(&status);
        size.layout((100, 30), Some((80, 24)));
        let deadline = size.deadline().expect("a wait is on");
        assert!(!size.give_up(deadline - Duration::from_millis(1)));
        assert!(size.give_up(deadline), "the bound passed");
        assert!(!size.give_up(deadline), "only once");
        assert_eq!(size.deadline(), None, "nothing left to wake for");
        assert!(size.renders(&status));
        size.layout((100, 30), Some((80, 24)));
        assert!(
            size.renders(&status),
            "the same ownership epoch doesn't wait again"
        );
    }

    #[test]
    fn typing_styled_terminal_runs_never_creates_a_second_ui_line() {
        let mut terminal = ghostty_vt::Terminal::new(ghostty_vt::TerminalOptions {
            cols: 253,
            rows: 4,
            ..Default::default()
        })
        .unwrap();
        let mut renderer = rex_linux::terminal::Renderer::new().unwrap();
        let fonts = crate::app::fonts::load("JetBrainsMono Nerd Font", &[]);
        let mut rt = Runtime::with_text(
            RowState { cells: vec![] },
            view,
            (2300., 100.),
            Env::new(true, 1.5),
            rune::text::TextSystem::new_with_paths(fonts.files, fonts.paths, fonts.families),
        );
        // Shell prompts mix fallback glyphs, styles and ASCII. Rebuild on every keystroke,
        // including runs preceded by blanks, at the user's fractional display scale.
        for n in 1..80 {
            terminal.vt_write(
                format!(
                    "\x1b[2J\x1b[H\u{f179} \u{e0b1} \x1b[36m\u{f015}~\x1b[0m   {}",
                    "a".repeat(n)
                )
                .as_bytes(),
            );
            let frame = renderer.frame(&terminal).unwrap();
            rt.update(|s, _| s.cells = frame.cells[0].clone());
            rt.frame(1. / 60.);
            for e in rt.elements().into_iter().filter(|e| e.text.is_some()) {
                assert!(
                    e.rect[3] <= 22.,
                    "keystroke {n}: {:?} wraps to {:?}",
                    e.text,
                    e.rect
                );
            }
            for q in rt.instances.iter().filter(|q| rune::compose::is_glyph(q)) {
                assert!(
                    q.rect[1] + q.rect[3] <= 33.,
                    "keystroke {n}: glyph drawn on the next line: {:?}",
                    q.rect
                );
            }
        }
    }

    /// A fixed cell, since the measured one follows whichever fonts another test has loaded.
    fn clipped_metrics() -> Metrics {
        Metrics {
            cell: (10., 20.),
            ..metrics(15.)
        }
    }

    struct ClippedState {
        term: Term,
    }
    fn clipped_view(s: &ClippedState, _: &mut Cx<ClippedState>) -> El {
        let look = GridLook {
            cursor: None,
            ime: false,
            colors: TermColors::default(),
        };
        el().w_full()
            .h_full()
            .child(grid("g", &s.term, &clipped_metrics(), look))
    }

    /// The search highlights `term` draws: (first column, row in the pane, selected).
    fn drawn_matches(term: Term) -> Vec<(u32, u32, bool)> {
        let (cw, ch) = clipped_metrics().cell;
        let mut rt = Runtime::new(
            ClippedState { term },
            clipped_view,
            (600., 200.),
            Env::default(),
        );
        rt.frame(1. / 60.);
        let mut drawn: Vec<_> = rt
            .instances
            .iter()
            .filter(|q| q.params[1] == rune::render::KIND_RECT && q.fill[3] > 0.1)
            .map(|q| {
                (
                    (q.rect[0] / cw).round() as u32,
                    (q.rect[1] / ch).round() as u32,
                    q.fill[3] > 0.4,
                )
            })
            .collect();
        drawn.sort_unstable();
        drawn
    }

    #[test]
    fn search_matches_stay_on_their_rows_when_the_pane_clips_a_taller_terminal() {
        use rex_linux::{
            connection::{Connection, TerminalSender},
            terminal::{Renderer, search::Controller},
        };
        // Another client owns five rows; this pane shows two, the newest, so a screen row and
        // a row in the pane differ by three.
        let mut terminal = ghostty_vt::Terminal::new(ghostty_vt::TerminalOptions {
            cols: 20,
            rows: 5,
            ..Default::default()
        })
        .unwrap();
        terminal.vt_write(b"    needle\r\n\r\n\r\nneedle\r\n");
        let mut search = Controller::default();
        search.set_needle(Some(&terminal), "needle".into()).unwrap();
        while search.update(&terminal).unwrap() {}
        let term_at = |search: &Controller| {
            let mut frame = Renderer::new().unwrap().frame(&terminal).unwrap();
            frame.search = search.results(&terminal).unwrap();
            let (sender, _) = flume::unbounded();
            let mut term = Term::new(Connection {
                model: Default::default(),
                commands: TerminalSender::new(sender, "s".into(), "b".into()),
                wake: flume::unbounded().1,
            });
            term.frame = Some(Arc::new(frame));
            term.rect = [0., 0., 600., 2. * clipped_metrics().cell.1];
            term
        };

        let term = term_at(&search);
        assert_eq!(term.display_row(clipped_metrics().cell), 3);
        assert_eq!(
            drawn_matches(term),
            [(0, 0, true)],
            "the newest match, screen row 3, is the pane's first; the clipped one draws nothing"
        );

        // The older match is above what the pane shows, so the pane follows it there.
        search.step(&terminal, true).unwrap();
        let term = term_at(&search);
        assert_eq!(term.display_row(clipped_metrics().cell), 0);
        assert_eq!(drawn_matches(term), [(4, 0, true)]);
    }

    /// A row at 1.5 device pixels per point, placed off the pixel grid.
    fn scaled_view(s: &RowState, _: &mut Cx<RowState>) -> El {
        let m = Metrics {
            px: 1.5,
            ..metrics(15.)
        };
        el().absolute(0.37, 0.71)
            .w_full()
            .h_full()
            .child(row_el(&s.cells, 0., 0, None, &m))
    }

    /// The rectangles drawn for `text` in one row at 1.5 device pixels per point, in device
    /// pixels, sorted from the left. Every one has whole-pixel edges.
    fn scaled_rects(text: &str) -> Vec<[f32; 4]> {
        let mut terminal = ghostty_vt::Terminal::new(ghostty_vt::TerminalOptions {
            cols: 40,
            rows: 2,
            ..Default::default()
        })
        .unwrap();
        terminal.vt_write(text.as_bytes());
        let frame = rex_linux::terminal::Renderer::new()
            .unwrap()
            .frame(&terminal)
            .unwrap();
        let fonts = crate::app::fonts::load("JetBrainsMono Nerd Font", &[]);
        let mut rt = Runtime::with_text(
            RowState {
                cells: frame.cells[0].clone(),
            },
            scaled_view,
            (600., 100.),
            Env::new(true, 1.5),
            rune::text::TextSystem::new_with_paths(fonts.files, fonts.paths, fonts.families),
        );
        rt.frame(1. / 60.);
        let mut rects: Vec<[f32; 4]> = rt
            .instances
            .iter()
            .filter(|q| q.params[1] == rune::render::KIND_RECT)
            .map(|q| q.rect)
            .collect();
        rects.sort_by(|a, b| a[0].total_cmp(&b[0]));
        for r in &rects {
            assert!(
                r.iter().all(|v| v.fract() == 0.),
                "{r:?} is off the pixel grid"
            );
        }
        rects
    }

    /// Each rectangle starts where an earlier one ends, or before.
    fn assert_unbroken(rects: &[[f32; 4]]) {
        let mut end = rects[0][0];
        for r in rects {
            assert!(r[0] <= end, "a gap before {r:?} in {rects:?}");
            end = end.max(r[0] + r[2]);
        }
    }

    #[test]
    fn box_lines_land_on_whole_pixels_and_meet_their_neighbours() {
        // Horizontal strokes: everything no taller than a heavy line.
        let lines: Vec<_> = scaled_rects("╭─┬──━━")
            .into_iter()
            .filter(|r| r[3] <= 4.)
            .collect();
        let (light, heavy): (Vec<&[f32; 4]>, Vec<_>) = lines.iter().partition(|r| r[3] == 2.);
        assert!(
            light.len() >= 6 && light.iter().all(|r| r[1] == light[0][1]),
            "light lines are one point, in whole pixels, on one row: {lines:?}"
        );
        assert_eq!(heavy.len(), 4, "heavy lines are twice as thick: {lines:?}");
        assert_unbroken(&lines);
    }

    #[test]
    fn decoration_lines_land_on_whole_device_pixels() {
        for px in [1., 1.25, 1.5, 1.75, 2., 2.5, 3.] {
            let thick = line_thickness(px) * px;
            assert_eq!(thick, thick.round(), "whole pixels thick at {px}");
            assert_eq!(thick, f32::max(px, 1.).round(), "one point at {px}");
            for top in [0.5, 17., 19., 20.5] {
                let edge = (line_centre(top, px) - line_thickness(px) / 2.) * px;
                assert!(
                    (edge - edge.round()).abs() < 1e-3,
                    "{top} at {px} starts on a pixel: {edge}"
                );
            }
        }
    }

    #[test]
    fn block_elements_fill_whole_pixels_without_seams() {
        let blocks = scaled_rects("████▌▐▀▄");
        assert_eq!(blocks.len(), 8, "one rectangle per block: {blocks:?}");
        let full = &blocks[..4];
        assert!(
            full.iter()
                .all(|r| r[1] == full[0][1] && r[3] == full[0][3]),
            "full blocks share their top and height: {full:?}"
        );
        assert_unbroken(full);
    }

    #[test]
    fn a_link_tooltip_names_the_hyperlink_target_not_its_text() {
        let mut terminal = ghostty_vt::Terminal::new(ghostty_vt::TerminalOptions {
            cols: 40,
            rows: 2,
            ..Default::default()
        })
        .unwrap();
        terminal
            .vt_write(b"\x1b]8;;https://evil.example/\x1b\\https://good.example/\x1b]8;;\x1b\\");
        let frame = rex_linux::terminal::Renderer::new()
            .unwrap()
            .frame(&terminal)
            .unwrap();
        let link = frame.link_at(3).expect("hyperlink");
        assert_eq!(link.uri, "https://evil.example/");
        struct LinkState {
            frame: Frame,
            link: Link,
        }
        let view = |s: &LinkState, _: &mut Cx<LinkState>| {
            let m = metrics(15.);
            let marks = link_marks("g", &s.frame, &s.link, 0..2, &m, MIDNIGHT.card);
            el().w_full().h_full().children(marks)
        };
        let mut rt = Runtime::new(
            LinkState { frame, link },
            view,
            (600., 100.),
            Env::default(),
        );
        rt.frame(0.);
        let help: Vec<String> = rt.elements().into_iter().filter_map(|e| e.help).collect();
        assert_eq!(help, ["Ctrl+click to open https://evil.example/"]);
    }

    #[test]
    fn search_highlights_follow_their_rows_when_the_pane_clips_a_taller_terminal() {
        use rex_linux::connection::{Connection, TerminalSender};
        let mut terminal = ghostty_vt::Terminal::new(ghostty_vt::TerminalOptions {
            cols: 12,
            rows: 5,
            ..Default::default()
        })
        .unwrap();
        terminal.vt_write(b"needle\r\na\r\nb\r\nc\r\nxx needle");
        let mut search = rex_linux::terminal::search::Controller::default();
        search.set_needle(Some(&terminal), "needle".into()).unwrap();
        while search.update(&terminal).unwrap() {}
        let mut frame = rex_linux::terminal::Renderer::new()
            .unwrap()
            .frame(&terminal)
            .unwrap();
        frame.search = search.results(&terminal).unwrap();
        assert_eq!(frame.search.as_ref().unwrap().matches.len(), 2);

        let m = clipped_metrics();
        let mut term = Term::new(Connection {
            model: Default::default(),
            commands: TerminalSender::new(flume::unbounded().0, "s".into(), "b".into()),
            wake: flume::unbounded().1,
        });
        term.frame = Some(Arc::new(frame));
        term.rect = [0., 0., 200., 2. * m.cell.1];
        assert_eq!(term.display_row(m.cell), 3, "rows 3 and 4 are on screen");
        let colors = TermColors::default();
        let view = |term: &Term, _: &mut Cx<Term>| {
            let look = GridLook {
                cursor: None,
                ime: false,
                colors: TermColors::default(),
            };
            grid("g", term, &clipped_metrics(), look)
        };
        let mut rt = Runtime::new(term, view, (200., 100.), Env::default());
        rt.frame(0.);
        let highlight = |alpha| rgb(colors.search_match).alpha(alpha);
        let marks: Vec<_> = rt
            .elements()
            .into_iter()
            .filter(|e| e.fill == highlight(0.55) || e.fill == highlight(0.25))
            .map(|e| {
                let [x, y, w, _] = e.rect;
                let cells = [x / m.cell.0, y / m.cell.1, w / m.cell.0].map(f32::round);
                (cells, e.fill == highlight(0.55))
            })
            .collect();
        assert_eq!(
            marks,
            [([3., 1., 6.], true)],
            "only the selected match on row 4, the second row on screen; the one on row 0 is clipped"
        );
    }

    /// A frame of default text, an explicit red background and inverse video, on a server
    /// whose default colours are `bg` and `fg`.
    fn themed_frame(bg: u32, fg: u32) -> Frame {
        let color = |c: u32| {
            Some(ghostty_vt::style::RgbColor {
                r: (c >> 16) as u8,
                g: (c >> 8) as u8,
                b: c as u8,
            })
        };
        let mut terminal = ghostty_vt::Terminal::new(ghostty_vt::TerminalOptions {
            cols: 4,
            rows: 1,
            ..Default::default()
        })
        .unwrap();
        terminal.set_default_bg_color(color(bg)).unwrap();
        terminal.set_default_fg_color(color(fg)).unwrap();
        terminal.vt_write(b"x\x1b[48;2;200;0;0mR\x1b[0m\x1b[7mI\x1b[0m");
        rex_linux::terminal::Renderer::new()
            .unwrap()
            .frame(&terminal)
            .unwrap()
    }

    #[test]
    fn the_default_background_takes_the_chromes_only_on_its_own_side() {
        let follow = TermColors::default();
        let card = MIDNIGHT.card;
        assert_eq!(follow.body(0x000000), card, "Rex's built-in black");
        assert_eq!(follow.body(0x1e1e2e), card, "another owner's dark theme");
        assert_eq!(
            follow.body(0xfdf6e3),
            0xfdf6e3,
            "another owner's light theme"
        );
        let light = TermColors::of(
            Some(&TerminalTheme {
                background: Some(0xfdf6e3),
                ..TerminalTheme::default()
            }),
            &MIDNIGHT,
        );
        assert_eq!(light.body(0xfdf6e3), 0xfdf6e3);
        assert_eq!(light.body(0xeeeeee), 0xfdf6e3);
        assert_eq!(
            light.body(0x000000),
            0x000000,
            "a server without set_theme keeps its black under its light default text"
        );
        let midnight = TermColors::of(Some(&TerminalTheme::midnight()), &MIDNIGHT);
        assert_eq!(midnight, follow, "Midnight's background is the card's");
        assert_eq!(
            TermColors::of(Some(&TerminalTheme::default()), &MIDNIGHT),
            follow,
            "Rex's own colours draw like Follow server"
        );
        let rose = TermColors::of(None, &crate::app::theme::ROSE_PINE);
        assert_eq!(rose.body(0x000000), 0x191724, "another chrome's card");
        assert_eq!(rose.selection_bg, 0x403d52, "and its selection");
        assert_ne!(rose, follow, "so rows memoized on the colours draw again");

        for (bg, fg, body) in [(0x1e1e2e, 0xcdd6f4, card), (0xfdf6e3, 0x586e75, 0xfdf6e3)] {
            let frame = themed_frame(bg, fg);
            assert_eq!(frame.bg, bg);
            let body_drawn = follow.body(frame.bg);
            assert_eq!(body_drawn, body);
            let [plain, red, inverse] = [0, 1, 2].map(|i| &frame.cells[0][i]);
            assert_eq!(plain.bg, frame.bg, "drawn as the pane's fill, {body:06x}");
            assert_eq!(red.bg, 0xc80000, "an explicit background is drawn as it is");
            assert_eq!(inverse.bg, fg, "inverse video's block keeps its colour");
            assert_eq!(RunStyle::of(plain, frame.bg, body_drawn).fg, fg);
            assert_eq!(RunStyle::of(red, frame.bg, body_drawn).fg, fg);
            assert_eq!(
                RunStyle::of(inverse, frame.bg, body_drawn).fg,
                body,
                "inverse text is cut out of the block in the pane's fill"
            );
        }
    }
}
