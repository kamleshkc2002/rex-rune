//! The cell and the text style under the typography settings: line height, character
//! spacing, the face ordinary text is drawn in, and whether bold and italic are drawn.
use super::*;
use rex_linux::settings::typography::{BoldItalic, FontStyle, SpacingUnit, Typography};
use rune::{Cx, Env, Runtime};

fn font(edit: impl FnOnce(&mut Typography)) -> Typography {
    let mut font = Typography::default();
    edit(&mut font);
    font
}

#[test]
fn the_default_typography_is_the_cell_it_always_was() {
    let _families = rune::text::with_families("Inter", "Inter");
    for size in [8., 13., 15., 22.5, 48.] {
        let m = metrics_for(size, &Typography::default());
        let plain = metrics(size);
        let advance = rune::text::advance("M", m.style);
        assert!(m == plain);
        let previous = TextStyle::new(size, 400.)
            .lh((size * 1.4).ceil())
            .features(FEATURES)
            .mono();
        assert_eq!(m.style, previous, "the pre-M2.6 style is unchanged");
        assert_eq!(m.cell.1, (size * 1.4_f32).ceil());
        assert_eq!(m.cell.0, advance.max(1.));
        assert_eq!((m.style.weight, m.style.italic), (400., false));
        assert_eq!(m.style.tracking, 0.);
        assert_eq!(m.style.line_height, m.cell.1);
        assert!(m.bold && m.italic);
        assert_eq!((m.inset, m.leading), (0., 0.));
    }
}

#[test]
fn line_height_scales_the_rows_and_keeps_the_text_in_the_middle_of_them() {
    let _families = rune::text::with_families("Inter", "Inter");
    let (plain, tall, short, most) = (
        metrics(15.),
        metrics_for(15., &font(|f| f.line_height = 20)),
        metrics_for(15., &font(|f| f.line_height = -20)),
        metrics_for(15., &font(|f| f.line_height = 5000)),
    );
    // 15 pt gives 21 pt rows; a fifth more is 25.2, in whole points.
    assert_eq!((plain.cell.1, tall.cell.1), (21., 26.));
    assert_eq!(tall.cell.0, plain.cell.0, "columns are as wide as before");
    assert_eq!(tall.style.line_height, 26.);
    // The text's own line box sits 2 pt down the row (half of the 5 pt added, in whole
    // points), which is where underlines and overlines are drawn from.
    assert_eq!(tall.leading, 2.);
    assert_eq!(short.cell.1, 17.);
    assert_eq!(short.leading, -2.);
    // A value past the range, which a file can't be kept from holding, is read within it.
    assert_eq!(most.cell.1, 42.);
}

#[test]
fn character_spacing_widens_the_cell_and_the_text_runs_alike() {
    let _families = rune::text::with_families("Inter", "Inter");
    // One character repeated: a headless runtime sets the grid in a proportional font.
    const RUNS: [(&str, f32); 3] = [("M", 1.), ("MMMM", 4.), ("MMMMMMMMMMMMMMMMMMMMMMMMM", 25.)];
    let plain = metrics(15.);
    let wide = metrics_for(15., &font(|f| f.spacing = 2));
    let tight = metrics_for(15., &font(|f| f.spacing = -1));
    let tenth = metrics_for(
        15.,
        &font(|f| {
            f.spacing = 10;
            f.spacing_unit = SpacingUnit::Percent;
        }),
    );
    let runs = RUNS.map(|(text, _)| rune::text::advance(text, wide.style));
    assert!((wide.cell.0 - plain.cell.0 - 2.).abs() < 1e-4);
    assert_eq!(wide.cell.1, plain.cell.1, "rows are as tall as before");
    assert_eq!(wide.inset, 1., "half before the glyph, half after");
    // A run of characters advances exactly one cell each, so one text element can hold a
    // whole row and its last character is still in its own cell.
    for ((text, cells), run) in RUNS.into_iter().zip(runs) {
        assert!(
            (run - cells * wide.cell.0).abs() < 0.02,
            "{text:?} is {run} wide, not {cells} cells of {}",
            wide.cell.0
        );
    }
    assert!((tight.cell.0 - plain.cell.0 + 1.).abs() < 1e-4);
    assert_eq!(tight.inset, -0.5);
    // In percent, of the font's own character width.
    assert!((tenth.cell.0 - plain.cell.0 * 1.1).abs() < 1e-3);
    // The memoized rows and the fit cache follow the spacing: the style differs.
    assert!(wide.style != plain.style && wide != plain);
}

#[test]
fn the_font_style_is_the_face_of_ordinary_text_and_bold_and_italic_can_be_left_undrawn() {
    let bold = metrics_for(15., &font(|f| f.style = FontStyle::Bold));
    assert_eq!((bold.style.weight, bold.style.italic), (700., false));
    let slanted = metrics_for(15., &font(|f| f.style = FontStyle::BoldItalic));
    assert_eq!((slanted.style.weight, slanted.style.italic), (700., true));
    let italic = metrics_for(15., &font(|f| f.style = FontStyle::Italic));
    assert_eq!((italic.style.weight, italic.style.italic), (400., true));

    let drawn = |choice: BoldItalic| {
        let m = metrics_for(15., &font(|f| f.bold_italic = choice));
        let face = face(&m, true, true);
        (face.weight == 700., face.italic)
    };
    assert_eq!(drawn(BoldItalic::Both), (true, true));
    assert_eq!(drawn(BoldItalic::BoldOnly), (true, false));
    assert_eq!(drawn(BoldItalic::ItalicOnly), (false, true));
    assert_eq!(drawn(BoldItalic::Neither), (false, false));
    // Text that isn't marked is the ordinary face whatever is drawn.
    let m = metrics_for(15., &font(|f| f.bold_italic = BoldItalic::Neither));
    assert!(face(&m, false, false) == m.style);
    for style in FontStyle::ALL {
        for drawing in BoldItalic::ALL {
            let m = metrics_for(
                15.,
                &font(|f| {
                    f.style = style;
                    f.bold_italic = drawing;
                }),
            );
            for (bold, italic) in [(false, false), (true, false), (false, true), (true, true)] {
                let actual = face(&m, bold, italic);
                assert_eq!(
                    actual.weight == 700.,
                    style.bold() || bold && drawing.bold()
                );
                assert_eq!(actual.italic, style.italic() || italic && drawing.italic());
            }
        }
    }
}

struct Rows {
    rows: Vec<Vec<Cell>>,
    font: Typography,
    px: f32,
}

fn rows_view(s: &Rows, _: &mut Cx<Rows>) -> El {
    let m = Metrics {
        px: s.px,
        ..metrics_for(15., &s.font)
    };
    let mut grid = el().w_full().h_full();
    for (y, cells) in s.rows.iter().enumerate() {
        grid = grid.child(row_el(cells, y as f32 * m.cell.1, 0, None, &m));
    }
    grid
}

/// Every rectangle two rows of `text` draw under `font` at two device pixels per point, in
/// device pixels.
fn drawn(text: &str, font: Typography, px: f32) -> Vec<[f32; 4]> {
    let cells: Vec<Cell> = text
        .chars()
        .map(|ch| Cell {
            text: ch.to_string(),
            spacer_tail: false,
            fg: 0xffffff,
            bg: 0,
            underline_color: None,
            style: Default::default(),
            link: None,
        })
        .collect();
    let mut rt = Runtime::new(
        Rows {
            rows: vec![cells.clone(), cells],
            font,
            px,
        },
        rows_view,
        (600., 200.),
        Env::new(true, px),
    );
    rt.frame(1. / 60.);
    rt.instances
        .iter()
        .filter(|q| q.params[1] == rune::render::KIND_RECT)
        .map(|q| q.rect)
        .collect()
}

#[test]
fn adjusted_full_blocks_have_no_seams_at_fractional_display_scales() {
    for px in [1., 1.25, 1.5, 2.] {
        let mut rects = drawn(
            "████",
            font(|f| {
                f.line_height = 20;
                f.spacing = 20;
                f.spacing_unit = SpacingUnit::Percent;
            }),
            px,
        );
        rects.sort_by(|a, b| a[1].total_cmp(&b[1]).then(a[0].total_cmp(&b[0])));
        assert_eq!(rects.len(), 8);
        for row in rects.chunks(4) {
            for pair in row.windows(2) {
                assert!(
                    pair[0][0] + pair[0][2] >= pair[1][0] - 0.01,
                    "{px}: {rects:?}"
                );
            }
        }
        assert!(
            rects[0][1] + rects[0][3] >= rects[4][1] - 0.01,
            "{px}: {rects:?}"
        );
    }
}

#[test]
fn box_lines_and_blocks_fill_a_taller_and_wider_cell_edge_to_edge() {
    // 21 pt rows a fifth taller are 26 pt: 52 device pixels at two per point.
    let ch = 52.;
    // Two rows of two full blocks, two horizontal lines and a vertical one, drawn at once.
    let rects = drawn(
        "██──│",
        font(|f| {
            f.line_height = 20;
            f.spacing = 2;
        }),
        2.,
    );

    // Full blocks: each is its whole cell, so two columns and two rows of them are one
    // rectangle with no seam, however wide and tall the settings make the cell.
    let mut blocks: Vec<_> = rects.iter().filter(|r| r[3] == ch).collect();
    blocks.sort_by(|a, b| a[1].total_cmp(&b[1]).then(a[0].total_cmp(&b[0])));
    assert_eq!(blocks.len(), 4, "{rects:?}");
    // The cell's width as drawn, in device pixels.
    let cw = blocks[1][0];
    assert!(cw > 4., "{blocks:?}");
    for (block, (col, row)) in blocks.iter().zip([(0., 0.), (1., 0.), (0., 1.), (1., 1.)]) {
        assert_eq!(
            (block[0], block[1]),
            (col * cw, row * ch),
            "{block:?} starts its cell"
        );
    }
    assert!(
        blocks[0][0] + blocks[0][2] >= blocks[1][0],
        "no seam between columns: {blocks:?}"
    );

    // The horizontal lines of the first row run the width of their two cells, unbroken.
    let mut dashes: Vec<_> = rects
        .iter()
        .filter(|r| r[3] <= 4. && r[1] < ch && r[2] > 4.)
        .collect();
    dashes.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let mut reach = dashes[0][0];
    for dash in &dashes {
        assert!(dash[0] <= reach, "a gap before {dash:?} in {dashes:?}");
        reach = f32::max(reach, dash[0] + dash[2]);
    }
    assert!(
        (dashes[0][0] - 2. * cw).abs() <= 1. && (reach - 4. * cw).abs() <= 1.,
        "{dashes:?} span columns 2 and 3 of {cw}"
    );

    // The vertical line is one unbroken stroke from the top of the first row to the foot
    // of the second.
    let mut lines: Vec<_> = rects
        .iter()
        .filter(|r| r[2] <= 4. && r[0] > 4. * cw)
        .collect();
    lines.sort_by(|a, b| a[1].total_cmp(&b[1]));
    assert!(lines.iter().all(|r| r[0] == lines[0][0]), "one column");
    let mut reach = 0.;
    for line in &lines {
        assert!(line[1] <= reach, "a gap before {line:?} in {lines:?}");
        reach = f32::max(reach, line[1] + line[3]);
    }
    assert_eq!((lines[0][1], reach), (0., 2. * ch), "{lines:?}");
}

#[test]
fn an_underline_stays_with_its_text_in_a_taller_row() {
    let underline = |font: &Typography| {
        let m = metrics_for(15., font);
        let style = RunStyle {
            fg: 0xffffff,
            bold: false,
            italic: false,
            faint: false,
            underline: Underline::Single,
            underline_color: None,
            overline: false,
            strike: false,
        };
        // Where `row_pass` draws it: in the text's line box, `leading` down the row.
        let line = m.cell.1 - 2. * m.leading;
        assert!(!decoration_shape(&style, 40., line, 1.).is_empty());
        (m.leading + line_centre(line - 2., 1.), m.cell.1)
    };
    let (plain, row) = underline(&Typography::default());
    let (tall, tall_row) = underline(&font(|f| f.line_height = 40));
    // 21 pt rows become 30 pt: the text moves down half of the 9 pt added, and its
    // underline with it (on a whole point) instead of dropping to the foot of the row.
    assert_eq!((row, tall_row), (21., 30.));
    assert!((tall - plain - 4.5).abs() <= 0.5, "{plain} then {tall}");
    assert!(tall < tall_row - 4., "not at the row's foot: {tall}");
}
