//! A few lines of terminal content drawn by the pane's own row and cursor code, for the
//! previews in Settings: what they show is what a terminal shows, selection colours, rounded
//! corners and cursor shapes included. Like a terminal, a preview never animates.
use super::{
    Marks, Metrics, RowPass, RunStyle, TermColors, cursor_el, glyph_run, row_pass,
    selection_corners,
};
use crate::app::theme::rgb;
use rex_linux::terminal::{Cell, CursorShape};
use rune::{El, el};

/// A run of one line's text in one style; `color` none is the terminal's default text.
#[derive(Clone, Copy, Default)]
pub struct Run<'a> {
    pub text: &'a str,
    pub color: Option<u32>,
    pub bold: bool,
    pub italic: bool,
}

/// Plain text in the default colour.
pub fn plain(text: &str) -> Run<'_> {
    Run {
        text,
        ..Run::default()
    }
}

impl<'a> Run<'a> {
    pub fn color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
}

/// What a preview draws: lines of runs, and at most one selection and one cursor.
#[derive(Default)]
pub struct Sample<'a> {
    pub lines: Vec<Vec<Run<'a>>>,
    /// (line, first column, last column), inclusive.
    pub selection: Option<(usize, usize, usize)>,
    /// (column, line) and the shape.
    pub cursor: Option<((usize, usize), CursorShape)>,
    /// The terminal's default text and cursor colours.
    pub text: u32,
    pub cursor_color: u32,
}

impl Sample<'_> {
    /// The columns of the widest line.
    pub fn columns(&self) -> usize {
        let cursor = self.cursor.map_or(0, |((col, _), _)| col + 1);
        self.lines
            .iter()
            .map(|line| line.iter().map(|run| run.text.chars().count()).sum())
            .max()
            .unwrap_or(0)
            .max(cursor)
    }
}

/// The sample's size at `m`: its widest line and its lines, in whole cells.
pub fn size(sample: &Sample<'_>, m: &Metrics) -> (f32, f32) {
    (
        sample.columns() as f32 * m.cell.0,
        sample.lines.len() as f32 * m.cell.1,
    )
}

/// The sample drawn at `m` in `colors`, on a transparent ground (the caller fills behind it
/// with [`TermColors::background`]). Its elements are keyed under `key`.
pub fn sample(key: &str, sample: &Sample<'_>, m: &Metrics, colors: &TermColors) -> El {
    let ground = colors.background;
    let (width, height) = size(sample, m);
    let mut grid = el()
        .key(key.to_owned())
        .size(width, height)
        .instant_changes();
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    for line in &sample.lines {
        let mut cells = Vec::new();
        for run in line {
            for ch in run.text.chars() {
                cells.push(Cell {
                    text: ch.to_string(),
                    spacer_tail: false,
                    fg: run.color.unwrap_or(sample.text),
                    bg: ground,
                    underline_color: None,
                    style: ghostty_vt::style::Style {
                        bold: run.bold,
                        italic: run.italic,
                        ..Default::default()
                    },
                    link: None,
                });
            }
        }
        rows.push(cells);
    }
    let selected_on = |y: usize| {
        sample
            .selection
            .filter(|&(line, _, _)| line == y)
            .map(|(_, a, b)| (a, b))
    };
    for (y, cells) in rows.iter().enumerate() {
        let selection = selected_on(y).filter(|&(a, _)| a < cells.len());
        let marks = Marks {
            selection,
            corners: selection.map_or([false; 4], |on| {
                selection_corners(
                    on,
                    y.checked_sub(1).and_then(selected_on),
                    selected_on(y + 1),
                )
            }),
            ..Marks::default()
        };
        grid = grid.child(
            row_pass(
                cells,
                y as f32 * m.cell.1,
                ground,
                &marks,
                m,
                RowPass::All,
                colors,
            )
            .key(format!("{key}/r{y}")),
        );
    }
    if let Some(((col, line), shape)) = sample.cursor {
        let at = (col as f32 * m.cell.0, line as f32 * m.cell.1);
        grid = grid.child(
            cursor_el(shape, at, m.cell, rgb(sample.cursor_color)).key(format!("{key}/cursor")),
        );
        // As in a pane: the character under a block cursor is drawn again in its text colour.
        if shape == CursorShape::Block
            && let Some(cell) = rows
                .get(line)
                .and_then(|cells| cells.get(col))
                .filter(|cell| !cell.text.trim().is_empty())
        {
            let style = RunStyle {
                fg: colors.cursor_text.unwrap_or(ground),
                ..RunStyle::of(cell, ground, ground)
            };
            grid = grid.child(glyph_run(cell.text.clone(), style, at.0, at.1, m));
        }
    }
    grid
}
