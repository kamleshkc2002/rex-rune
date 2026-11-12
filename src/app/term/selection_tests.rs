//! The selection's colours and corners, cell by cell and row by row.
use super::*;
use rex_rune::settings::terminal::SelectionColor::{CellBackground, CellForeground, Theme};

const TEXT: u32 = 0xd0d0d0;
const GROUND: u32 = 0x101010;

fn colors(foreground: SelectionColor, background: SelectionColor) -> TermColors {
    TermColors::default().with_selection(SelectionLook {
        foreground,
        background,
        rounded: true,
    })
}

#[test]
fn theme_colours_are_as_before_and_cell_colours_keep_selected_text_readable() {
    let fill = TermColors::default().selection_bg;
    // The default: the palette's fill under text that keeps its colour, even text that
    // happens to be the fill's colour.
    assert_eq!(
        selected_colors(&colors(Theme, Theme), TEXT, GROUND),
        (fill, TEXT)
    );
    assert_eq!(
        selected_colors(&colors(Theme, Theme), fill, GROUND),
        (fill, fill)
    );
    // A declared theme's selection foreground replaces the text's colour.
    let declared = TermColors {
        selection_fg: Some(0x123456),
        ..colors(Theme, Theme)
    };
    assert_eq!(selected_colors(&declared, TEXT, GROUND), (fill, 0x123456));
    assert_eq!(
        selected_colors(
            &TermColors {
                selection_fg: Some(0x123456),
                ..colors(CellForeground, Theme)
            },
            TEXT,
            GROUND
        ),
        (fill, TEXT),
        "Cell foreground keeps the text's own colour over a theme's"
    );

    // Cell foreground as the fill inverts the cell, whatever the foreground setting leaves
    // the text as.
    for foreground in [Theme, CellForeground, CellBackground] {
        assert_eq!(
            selected_colors(&colors(foreground, CellForeground), TEXT, GROUND),
            (TEXT, GROUND),
            "{foreground:?}"
        );
    }
    // Cell background as the text over the theme's fill.
    assert_eq!(
        selected_colors(&colors(CellBackground, Theme), TEXT, GROUND),
        (fill, GROUND)
    );
    // Both from the cell's background would hide the text: it takes the cell's text colour.
    assert_eq!(
        selected_colors(&colors(CellBackground, CellBackground), TEXT, GROUND),
        (GROUND, TEXT)
    );
}

#[test]
fn only_the_outer_corners_of_a_selection_of_several_rows_are_round() {
    // One row: all four.
    assert_eq!(selection_corners((2, 9), None, None), [true; 4]);
    // From the middle of the first row, through a whole second, to the middle of the third.
    let (first, second, third) = ((5, 79), (0, 79), (0, 10));
    assert_eq!(
        selection_corners(first, None, Some(second)),
        [true, true, false, false]
    );
    assert_eq!(
        selection_corners(second, Some(first), Some(third)),
        [true, false, true, false],
        "the left end sticks out above, the right end below"
    );
    assert_eq!(
        selection_corners(third, Some(second), None),
        [false, false, true, true]
    );
}

fn cells(text: &str) -> Vec<Cell> {
    text.chars()
        .map(|ch| Cell {
            text: ch.to_string(),
            spacer_tail: false,
            fg: if ch.is_ascii_uppercase() {
                0xff0000
            } else {
                TEXT
            },
            bg: GROUND,
            underline_color: None,
            style: Default::default(),
            link: None,
        })
        .collect()
}

/// A selection fill as drawn: (first column, columns, corner radii).
type Drawn = (u32, u32, [f32; 4]);

struct RowState {
    cells: Vec<Cell>,
    corners: [bool; 4],
    colors: TermColors,
}

/// A fixed cell, since the measured one follows whichever fonts another test has loaded.
fn fixed_metrics() -> Metrics {
    Metrics {
        cell: (10., 20.),
        ..metrics(15.)
    }
}

fn row_view(s: &RowState, _: &mut rune::Cx<RowState>) -> El {
    let marks = Marks {
        selection: Some((1, 6)),
        corners: s.corners,
        ..Marks::default()
    };
    el().w_full().h_full().child(row_pass(
        &s.cells,
        0.,
        GROUND,
        &marks,
        &fixed_metrics(),
        RowPass::All,
        &s.colors,
    ))
}

/// The fills a row draws for the selection of its cells 1 to 6, left to right: (first
/// column, columns, radii) and the fill itself.
fn fills(colors: TermColors, corners: [bool; 4]) -> Vec<(Drawn, [f32; 4])> {
    let mut rt = rune::Runtime::new(
        RowState {
            cells: cells("ab CD ef"),
            corners,
            colors,
        },
        row_view,
        (400., 100.),
        rune::Env::default(),
    );
    rt.frame(1. / 60.);
    let cw = fixed_metrics().cell.0;
    let mut drawn: Vec<_> = rt
        .instances
        .iter()
        .filter(|q| q.params[1] == rune::render::KIND_RECT && q.fill[3] > 0.1)
        .map(|q| {
            (
                (
                    (q.rect[0] / cw).round() as u32,
                    (q.rect[2] / cw).round() as u32,
                    q.radii,
                ),
                q.fill,
            )
        })
        .collect();
    drawn.sort_by_key(|(shape, _)| shape.0);
    drawn
}

#[test]
fn wide_selected_cells_share_fill_and_leave_original_cells_unchanged() {
    let mut terminal = ghostty_vt::Terminal::new(ghostty_vt::TerminalOptions {
        cols: 8,
        rows: 1,
        ..Default::default()
    })
    .unwrap();
    terminal.vt_write("\x1b[31m界e\u{301}".as_bytes());
    let frame = rex_rune::terminal::Renderer::new()
        .unwrap()
        .frame(&terminal)
        .unwrap();
    assert!(frame.cells[0][1].spacer_tail);
    let range = frame.selection_range(1, SelectionUnit::Cell).unwrap();
    assert_eq!(range, (0, 1), "a wide tail selects the whole glyph");
    let original = frame.cells[0].clone();
    for rounded in [false, true] {
        let look = colors(Theme, CellForeground).with_selection(SelectionLook {
            foreground: Theme,
            background: CellForeground,
            rounded,
        });
        let view = |s: &(Vec<Cell>, TermColors), _: &mut rune::Cx<(Vec<Cell>, TermColors)>| {
            row_pass(
                &s.0,
                0.,
                0,
                &Marks {
                    selection: Some((0, 1)),
                    corners: [true; 4],
                    ..Default::default()
                },
                &fixed_metrics(),
                RowPass::All,
                &s.1,
            )
        };
        let mut rt = rune::Runtime::new(
            (original.clone(), look),
            view,
            (200., 100.),
            rune::Env::default(),
        );
        rt.frame(1. / 60.);
        let fills: Vec<_> = rt
            .instances
            .iter()
            .filter(|q| q.params[1] == rune::render::KIND_RECT && q.fill[3] > 0.1)
            .collect();
        assert!(
            fills
                .iter()
                .any(|q| (q.rect[2] - 20.).abs() < 0.1
                    && q.radii == [if rounded { 3. } else { 0. }; 4]),
            "the two cells are one selection shape"
        );
        assert_eq!(frame.selected_text(0, 2), "界e\u{301}");
        for (a, b) in frame.cells[0].iter().zip(&original) {
            assert_eq!((a.fg, a.bg, &a.text), (b.fg, b.bg, &b.text));
        }
    }
}

#[test]
fn a_rows_selection_is_one_fill_or_one_per_run_of_cell_colour() {
    let r = SELECTION_RADIUS;
    let shapes = |colors, corners| -> Vec<Drawn> {
        fills(colors, corners)
            .into_iter()
            .map(|(shape, _)| shape)
            .collect()
    };

    // The theme's fill: one shape over the span, round at the corners that are the
    // selection's own (top-left, top-right, bottom-right, bottom-left).
    let theme = colors(Theme, Theme);
    assert_eq!(
        shapes(theme, [true, false, false, true]),
        [(1, 6, [r, 0., 0., r])]
    );
    assert_eq!(shapes(theme, [true; 4]), [(1, 6, [r; 4])]);

    // Square corners when the setting says so.
    let square = theme.with_selection(SelectionLook {
        rounded: false,
        ..theme.selection
    });
    assert_eq!(shapes(square, [true; 4]), [(1, 6, [0.; 4])]);

    // Each cell's own text colour: "b ", "CD" and " e" are three runs, and only the ends
    // of the span are round.
    let drawn = fills(colors(Theme, CellForeground), [true; 4]);
    let runs: Vec<_> = drawn.iter().map(|(shape, _)| *shape).collect();
    assert_eq!(
        runs,
        [
            (1, 2, [r, 0., 0., r]),
            (3, 2, [0.; 4]),
            (5, 2, [0., r, r, 0.]),
        ]
    );
    assert_eq!(drawn[0].1, drawn[2].1, "the same text colour");
    assert_ne!(drawn[0].1, drawn[1].1, "the capitals are another");
}
