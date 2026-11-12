//! The find bar of a searched terminal: the field, the selected match's place among all
//! matches, Find next, Find previous and Close, and where the bar sits.
//!
//! The bar floats over a corner of the terminal and never moves or resizes it. It starts in
//! the top-right corner and leaves a corner only when the selected match would be under it,
//! for the first corner that keeps the match in view; it then stays there, so stepping through
//! matches moves it as seldom as possible. Closing the bar sends it home.
//!
//! The bar takes whole rows and whole columns of the terminal: an apron in the terminal's
//! own background hides the cells it only partly covers, so no sliver of a row shows above,
//! below or beside it. The apron hides content and never recolours it.
//!
//! Find next and Find previous also run with the bar closed: they bring it back on the last
//! text searched in that terminal, without taking the keyboard from the program.
use super::{
    input::search_key,
    pane_header::{self, BUTTON, Placement, UNAVAILABLE},
    term::Term,
    theme::*,
    widgets::glyph,
    workspace::{HostId, Workspace},
};
use rex_linux::connection::Command;
use rexrs::api::session::Window;
use rune::{Cursor, El, Icon, KeyEvent, TextEdit, Tx, el, row, text};
use rune_kit::TextInput;
use serde_json::Value;

pub const NEXT: &str = "client.search.next";
pub const PREVIOUS: &str = "client.search.previous";

/// The bar: at most this wide, narrower in a pane that is.
pub const WIDTH: f32 = 320.;
pub const HEIGHT: f32 = 34.;
/// From the bar to the terminal's left or right edge.
pub const INSET: f32 = 6.;
/// The least the apron shows around the bar, above, below and on its open side.
const CLEAR: f32 = 3.;
/// Where a top bar's rows start while the exception strip floats in the pane's top-right
/// corner: at the first row under the strip (8 pt down, 22 pt tall).
const UNDER_STRIP: f32 = 30.;
const ICON: f32 = 14.;

/// The corner of the terminal the bar sits in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Corner {
    #[default]
    TopRight,
    BottomRight,
    TopLeft,
    BottomLeft,
}

impl Corner {
    /// The order corners are tried in. The right side comes first: output starts at the left.
    const ALL: [Corner; 4] = [
        Corner::TopRight,
        Corner::BottomRight,
        Corner::TopLeft,
        Corner::BottomLeft,
    ];

    fn top(self) -> bool {
        matches!(self, Corner::TopRight | Corner::TopLeft)
    }

    fn right(self) -> bool {
        matches!(self, Corner::TopRight | Corner::BottomRight)
    }
}

/// What a terminal keeps of its search between frames and after the bar closes.
#[derive(Default)]
pub struct Memory {
    /// The last text searched here, for Find next and Find previous once the bar is closed.
    pub last: String,
    /// Where the bar is.
    pub corner: Corner,
}

/// The terminal's grid as the bar sees it, in points from the grid's top-left corner.
#[derive(Clone, Copy)]
pub struct Pane {
    pub size: (f32, f32),
    pub cell: (f32, f32),
    /// The exception strip floats in the top-right corner, so a top bar starts under it.
    pub lowered: bool,
}

impl Pane {
    fn width(&self) -> f32 {
        WIDTH.min(self.size.0 - 2. * INSET).max(0.)
    }

    fn cell(&self) -> (f32, f32) {
        (self.cell.0.max(1.), self.cell.1.max(1.))
    }

    /// The whole rows and columns the bar takes in `corner`, as a rect: enough rows for the
    /// bar with `CLEAR` above and below it, from the pane's edge to the first column boundary
    /// at least `CLEAR` past the bar. A top one starts at the first row, or at the first row
    /// under a floating strip; a bottom one ends with the last row and runs on to the pane's
    /// edge.
    pub fn apron(&self, corner: Corner) -> [f32; 4] {
        let (cw, ch) = self.cell();
        let band = ((HEIGHT + 2. * CLEAR) / ch).ceil() * ch;
        let (y, height) = if corner.top() {
            let start = if self.lowered {
                (UNDER_STRIP / ch).ceil() * ch
            } else {
                0.
            };
            (start, band)
        } else {
            let rows = (self.size.1 / ch).floor().max(1.);
            let start = (rows * ch - band).max(0.);
            (start, self.size.1 - start)
        };
        let reach = INSET + self.width() + CLEAR;
        let (x, width) = if corner.right() {
            let start = ((self.size.0 - reach) / cw).floor().max(0.) * cw;
            (start, self.size.0 - start)
        } else {
            (0., ((reach / cw).ceil() * cw).min(self.size.0))
        };
        [x, y, width.max(0.), height.max(0.)]
    }

    /// The bar's rect in `corner`: centred on its apron's rows.
    pub fn bar(&self, corner: Corner) -> [f32; 4] {
        let ch = self.cell().1;
        let band = ((HEIGHT + 2. * CLEAR) / ch).ceil() * ch;
        let width = self.width();
        let x = if corner.right() {
            self.size.0 - INSET - width
        } else {
            INSET
        };
        let y = self.apron(corner)[1] + (band - HEIGHT) / 2.;
        [x, y, width, HEIGHT]
    }

    /// Whether the bar in `corner` hides any of `pieces`: the selected match as (row in the
    /// pane, first column, last column), a piece a row. What its apron covers is hidden.
    fn covers(&self, corner: Corner, pieces: &[(usize, usize, usize)]) -> bool {
        let [x, y, w, h] = self.apron(corner);
        let (cw, ch) = self.cell();
        pieces.iter().any(|&(row, first, last)| {
            let (left, right) = (first as f32 * cw, (last + 1) as f32 * cw);
            let (top, bottom) = (row as f32 * ch, (row + 1) as f32 * ch);
            left < x + w && right > x && top < y + h && bottom > y
        })
    }

    /// Where the bar goes: it stays in `current` unless the selected match is under it there,
    /// and then takes the first corner that leaves the match in view. With none (a pane
    /// smaller than the bar's reach) it stays.
    pub fn settle(&self, current: Corner, pieces: &[(usize, usize, usize)]) -> Corner {
        if !self.covers(current, pieces) {
            return current;
        }
        Corner::ALL
            .into_iter()
            .find(|corner| !self.covers(*corner, pieces))
            .unwrap_or(current)
    }
}

/// Whether `block`'s exception strip floats in its top-right corner: a terminal that is not
/// live, in a pane with no header row to hold the strip.
pub fn lowered(s: &Workspace, window: &Window, block: &str, t: &Term) -> bool {
    !t.status().live && pane_header::placement(s, window, block) != Placement::Row
}

impl Term {
    /// The selected match's cells on the rows this pane shows.
    fn selected_match(&self, cell: (f32, f32)) -> Vec<(usize, usize, usize)> {
        let Some(found) = self.frame.as_ref().and_then(|f| f.search.as_ref()) else {
            return Vec::new();
        };
        let start = self.display_row(cell);
        (start..start + self.visible_rows(cell))
            .flat_map(|y| {
                found
                    .on_row(y)
                    .iter()
                    .filter(|piece| piece.2)
                    .map(move |&(first, last, _)| (y - start, first, last))
            })
            .collect()
    }

    /// Keep the bar off the selected match, now that the frame or the pane's size changed.
    fn settle_find(&mut self, cell: (f32, f32), lowered: bool) {
        if self.search.is_none() || self.rect[2] <= 0. || self.rect[3] <= 0. {
            return;
        }
        let pane = Pane {
            size: (self.rect[2], self.rect[3]),
            cell,
            lowered,
        };
        self.find.corner = pane.settle(self.find.corner, &self.selected_match(cell));
    }
}

impl Workspace {
    /// Find next (`older`: the match above, as Enter in the field) or Find previous in the
    /// focused terminal. An open bar steps, wherever the keyboard is. A closed one comes back
    /// on the last text searched here, at its newest match, and the keyboard stays with the
    /// program; with nothing searched yet it opens for typing.
    pub fn find_step(&mut self, older: bool) {
        let Some(term) = self.active_terminal_mut() else {
            return;
        };
        match &mut term.search {
            Some(edit) if edit.text.is_empty() => edit.focused = true,
            Some(_) => term.send(Command::SearchStep(older)),
            None if term.find.last.is_empty() => self.open_search(),
            None => {
                let needle = term.find.last.clone();
                term.search = Some(TextEdit::new(needle.as_str()));
                term.send(Command::Search(needle));
            }
        }
    }

    /// Whether Find next and Find previous have a terminal to act on.
    pub fn can_find(&self) -> bool {
        self.active_terminal().is_some()
    }

    /// A terminal's frame or size changed: move its find bar off the selected match.
    pub fn place_find(&mut self, key: &(HostId, String)) {
        if self.terminals.get(key).is_none_or(|t| t.search.is_none()) {
            return;
        }
        let cell = self.metrics().cell;
        // Only a pane of the tab on screen has a strip to stay under.
        let lowered = *key == self.term_key(&key.1)
            && self.model().window().is_some_and(|window| {
                self.terminals
                    .get(key)
                    .is_some_and(|t| lowered(self, window, &key.1, t))
            });
        if let Some(term) = self.terminals.get_mut(key) {
            term.settle_find(cell, lowered);
        }
    }
}

/// What a press on one of the bar's buttons does.
type Run = Box<dyn Fn(&mut Workspace, &mut Tx)>;

/// One of the bar's buttons, as a pane header's: no fill at rest, a rounded one under the
/// pointer. A press never takes the keyboard, from the field or from the terminal. With no
/// `run` it cannot act now: it keeps its place, dim, and its tooltip says why.
fn button(key: String, icon: &'static Icon, label: &str, help: String, run: Option<Run>) -> El {
    let b = row()
        .key(key)
        .role("button")
        .label(label.to_owned())
        .size(BUTTON, BUTTON)
        .center()
        .radius(6.)
        .continuous_corners()
        .preserve_focus_on_press(true)
        .tab_index(-1)
        .help(help);
    let Some(run) = run else {
        return b.child(glyph(icon, ICON, rgb(muted())).opacity(UNAVAILABLE));
    };
    b.cursor(Cursor::Pointer)
        .hover(white(0.09))
        .press(white(0.13))
        .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| run(s, tx))
        .child(glyph(icon, ICON, rgb(muted())).group_color(rgb(ink())))
}

/// The apron behind `block`'s bar, a child of the pane's card drawn under the bar: the bar's
/// whole rows and columns in `fill`, the terminal's own background. It is terminal area, not
/// chrome, so it changes corner at once while the bar springs, and a press on it is the
/// terminal's. `inset` is from the card's left and top edges to the grid.
pub fn apron(block: &str, t: &Term, pane: Pane, inset: (f32, f32), fill: u32) -> El {
    let corner = t.find.corner;
    let [x, y, width, height] = pane.apron(corner);
    el().key(format!("find-apron:{block}:{corner:?}"))
        .absolute(inset.0 + x, inset.1 + y)
        .size(width, height)
        .fill(rgb(fill))
        .inert()
        .motion(STILL)
        .transition(no_presence())
}

/// The bar over `block`'s terminal, a child of the pane's card. `pane` is the terminal's grid,
/// and `inset` is from the card's edges to that grid, as (left and right, top, bottom).
pub fn bar(
    s: &Workspace,
    block: &str,
    key: &(HostId, String),
    edit: &TextEdit,
    t: &Term,
    pane: Pane,
    inset: (f32, f32, f32),
) -> El {
    let results = t.frame.as_ref().and_then(|f| f.search.as_ref());
    let total = results.map_or(0, |r| r.total);
    // Nothing while the field is empty or the first pass is still running.
    let count = match results {
        Some(_) if edit.text.is_empty() => None,
        Some(r) if r.total == 0 => Some(("No matches".to_owned(), danger())),
        Some(r) => Some((
            format!("{}/{}", r.index.map_or(0, |i| i + 1), r.total),
            muted(),
        )),
        None => None,
    };
    let count = match count {
        Some((words, colour)) => text(words)
            .key(format!("find-count:{block}"))
            .font(medium(11.))
            .color(rgb(colour)),
        None => el(),
    };
    // The field's own key, then the action's while it has one.
    let help = |label: &str, field: &str, action: &str| {
        let keys = s.shortcut_detail(action, &Value::Null);
        match (total > 0, keys.is_empty()) {
            (false, _) => format!("{label} · no matches"),
            (true, true) => format!("{label} · {field}"),
            (true, false) => format!("{label} · {field} · {keys}"),
        }
    };
    let step = |older: bool| -> Option<Run> {
        let key = key.clone();
        (total > 0).then(|| {
            Box::new(move |s: &mut Workspace, _: &mut Tx| {
                if let Some(term) = s.terminals.get_mut(&key) {
                    term.send(Command::SearchStep(older));
                }
            }) as Run
        })
    };
    let (k1, k2, k3, k4) = (key.clone(), key.clone(), key.clone(), key.clone());
    let corner = t.find.corner;
    let [_, y, width, _] = pane.bar(corner);
    let (side, top, bottom) = inset;
    let (above, below) = if corner.top() {
        (Some(top + y), None)
    } else {
        (None, Some(bottom + (pane.size.1 - y - HEIGHT).max(0.)))
    };
    let (left, right) = if corner.right() {
        (None, Some(side + INSET))
    } else {
        (Some(side + INSET), None)
    };
    row()
        .key(format!("search:{block}"))
        .w(width)
        .h(HEIGHT)
        .pl(2.)
        .pr(5.)
        .center_y()
        .radius(8.)
        .continuous_corners()
        .fill(rgb(popover()))
        .border(1., white(0.12))
        .box_shadow(shadow(0., 4., 16., black(0.45)))
        .pin(left, above, right, below)
        // The bar is chrome: it springs to another corner. It comes and goes at once.
        .motion(SNAPPY)
        .transition(no_presence())
        // A press on the bar is the bar's: it neither reaches the terminal under it nor
        // takes the keyboard from the field.
        .preserve_focus_on_press(true)
        .on_press(|_: &mut Workspace| {})
        .child(
            TextInput::new(edit)
                .placeholder("Search terminal")
                .leading_icon(&SEARCH)
                .leading_icon_size(14.)
                .text_style(ui(13.))
                .height(32.)
                .plain()
                .trailing(count)
                .on_edit(move |s: &mut Workspace| s.search_input(&k1))
                .on_change(move |s: &mut Workspace, _: &mut Tx| {
                    s.last_keypress = Some(std::time::Instant::now());
                    s.search_changed(&k2);
                })
                .on_key(move |s: &mut Workspace, ev: &KeyEvent, tx: &mut Tx| {
                    search_key(s, &k3, ev, tx)
                })
                .width(None)
                .build(kit())
                .key(format!("search-field:{block}"))
                .grow(1.)
                .min_w(0.),
        )
        .child(el().w(1.).h(16.).mx(4.).fill(white(0.09)))
        .child(button(
            format!("find-next:{block}"),
            &STEP_UP,
            "Find next",
            help("Find next", "Enter", NEXT),
            step(true),
        ))
        .child(button(
            format!("find-previous:{block}"),
            &STEP_DOWN,
            "Find previous",
            help("Find previous", "Shift Enter", PREVIOUS),
            step(false),
        ))
        .child(button(
            format!("find-close:{block}"),
            &CLOSE,
            "Close search",
            "Close search · Esc".into(),
            Some(Box::new(move |s: &mut Workspace, tx: &mut Tx| {
                s.close_search(&k4, tx)
            })),
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An 80 by 24 terminal of 8 by 20 pt cells.
    fn pane() -> Pane {
        Pane {
            size: (640., 480.),
            cell: (8., 20.),
            lowered: false,
        }
    }

    #[test]
    fn the_bar_keeps_its_corner_until_the_selected_match_is_under_it() {
        let pane = pane();
        assert_eq!(pane.bar(Corner::TopRight), [314., 3., 320., 34.]);
        // Nothing selected on screen, a match at the left, and one below the bar's rows.
        for pieces in [vec![], vec![(0, 0, 5)], vec![(2, 70, 75)]] {
            assert_eq!(
                pane.settle(Corner::TopRight, &pieces),
                Corner::TopRight,
                "{pieces:?}"
            );
        }
        // On the first row at the right: under the bar, so the bar goes below.
        let under = [(0, 60, 63)];
        assert_eq!(pane.settle(Corner::TopRight, &under), Corner::BottomRight);
        assert_eq!(pane.bar(Corner::BottomRight), [314., 443., 320., 34.]);
        // The second row is the bar's too, though the bar ends 3 pt short of its bottom.
        assert_eq!(
            pane.settle(Corner::TopRight, &[(1, 60, 63)]),
            Corner::BottomRight
        );
        // It stays below while matches there are clear of it, wherever the next match is.
        for pieces in [vec![(0, 60, 63)], vec![(5, 0, 3)], vec![]] {
            assert_eq!(
                pane.settle(Corner::BottomRight, &pieces),
                Corner::BottomRight
            );
        }
        // And returns only when the selected match is under it there.
        assert_eq!(
            pane.settle(Corner::BottomRight, &[(23, 70, 75)]),
            Corner::TopRight
        );
    }

    #[test]
    fn the_bar_takes_whole_rows_and_columns() {
        // 8 by 20 pt cells: two rows hold the 34 pt bar, from a column boundary to the edge.
        let pane = pane();
        assert_eq!(pane.apron(Corner::TopRight), [304., 0., 336., 40.]);
        assert_eq!(pane.apron(Corner::TopLeft), [0., 0., 336., 40.]);
        assert_eq!(pane.apron(Corner::BottomRight), [304., 440., 336., 40.]);
        // Cells that don't divide the pane: the apron still starts on a row and a column, a
        // bottom one runs past the last whole row to the pane's edge, and the bar sits
        // inside it with room on every side.
        let odd = Pane {
            size: (643., 487.),
            cell: (9., 21.),
            lowered: false,
        };
        for corner in Corner::ALL {
            let [ax, ay, aw, ah] = odd.apron(corner);
            let [bx, by, bw, bh] = odd.bar(corner);
            assert_eq!(ay % 21., 0., "{corner:?} starts on a row");
            assert_eq!(ax % 9., 0., "{corner:?} starts on a column");
            if corner.top() {
                assert_eq!(ah % 21., 0., "{corner:?} ends on a row");
            } else {
                assert_eq!(ay + ah, 487., "{corner:?} runs to the pane's edge");
            }
            if corner.right() {
                assert_eq!(ax + aw, 643.);
                assert!(bx - ax >= CLEAR);
            } else {
                assert_eq!((ax + aw) % 9., 0., "{corner:?} ends on a column");
                assert!(ax + aw - (bx + bw) >= CLEAR);
            }
            assert!(
                by - ay >= CLEAR && ay + ah - (by + bh) >= CLEAR,
                "{corner:?}"
            );
        }
        // Small and large cells alike.
        for cell in [(5., 12.), (29., 68.)] {
            let pane = Pane { cell, ..pane };
            let [_, ay, _, ah] = pane.apron(Corner::TopRight);
            let [_, by, _, bh] = pane.bar(Corner::TopRight);
            assert_eq!((ay, ah % cell.1), (0., 0.));
            assert!(by - ay >= CLEAR && ay + ah - (by + bh) >= CLEAR);
        }
    }

    #[test]
    fn a_match_that_wraps_or_a_short_pane_sends_the_bar_elsewhere() {
        // Three rows: the two right corners share the middle one.
        let short = Pane {
            size: (640., 60.),
            ..pane()
        };
        assert_eq!(
            short.settle(Corner::TopRight, &[(1, 60, 63)]),
            Corner::TopLeft
        );
        // A match from the end of row 0 onto row 1 is clear of neither top corner.
        let wrapped = [(0, 76, 79), (1, 0, 3)];
        assert_eq!(
            pane().settle(Corner::TopRight, &wrapped),
            Corner::BottomRight
        );
        assert_eq!(
            short.settle(Corner::TopRight, &wrapped),
            Corner::BottomRight
        );
        // With every corner over the match the bar stays where it is.
        let everywhere = [(1, 0, 79)];
        assert_eq!(
            short.settle(Corner::TopRight, &everywhere),
            Corner::TopRight
        );
    }

    #[test]
    fn a_narrow_pane_gets_a_narrower_bar_and_a_floating_strip_lowers_it() {
        let narrow = Pane {
            size: (200., 480.),
            ..pane()
        };
        assert_eq!(narrow.bar(Corner::TopRight), [6., 3., 188., 34.]);
        assert_eq!(narrow.bar(Corner::TopLeft), [6., 3., 188., 34.]);
        assert_eq!(narrow.apron(Corner::TopRight), [0., 0., 200., 40.]);
        let lowered = Pane {
            lowered: true,
            ..pane()
        };
        // The first row boundary under the strip is at 40 pt.
        assert_eq!(lowered.apron(Corner::TopRight), [304., 40., 336., 40.]);
        assert_eq!(lowered.bar(Corner::TopRight), [314., 43., 320., 34.]);
        // Row 0 is clear of a lowered bar; row 2 (40 to 60 pt) is under it.
        assert_eq!(
            lowered.settle(Corner::TopRight, &[(0, 60, 63)]),
            Corner::TopRight
        );
        assert_eq!(
            lowered.settle(Corner::TopRight, &[(2, 60, 63)]),
            Corner::BottomRight
        );
    }
}
