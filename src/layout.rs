//! Translate authoritative Rex split trees to normalized native pane geometry.
use rexrs::api::{
    session::{ViewLayoutNode, Window},
    terminal::SetTheme,
};
use serde_json::{Value, json};

/// One new terminal block for a layout, starting `shell` when one is set (else the server
/// user's login shell), with this client's declared `theme` when it has one. Rex counts a
/// layout's theme as its creator's declaration.
pub fn shell_layout(shell: Option<&str>, theme: Option<&SetTheme>) -> Value {
    let mut block = json!({"flavor": "com.superlogical.terminal.shell"});
    if let Some(shell) = shell.map(str::trim).filter(|s| !s.is_empty()) {
        block["options"]["shell"] = json!(shell);
    }
    if let Some(theme) = theme {
        block["options"]["theme"] = json!(theme);
    }
    json!({"block": block})
}

/// Set the server-side starting directory of a new terminal, without changing shell or theme.
/// A layout that is not a single block is left alone.
pub fn with_cwd(layout: &mut Value, cwd: &str) {
    if let Some(block) = layout.get_mut("block").filter(|b| b.is_object()) {
        block["options"]["cwd"] = json!(cwd);
    }
}

#[cfg(test)]
#[test]
fn with_cwd_keeps_the_shell_and_theme_and_leaves_splits_alone() {
    let theme = SetTheme::new().background("#001528");
    let mut layout = shell_layout(Some("zsh"), Some(&theme));
    with_cwd(&mut layout, "/srv/app");
    assert_eq!(
        layout["block"]["options"],
        json!({"shell":"zsh","theme":{"background":"#001528"},"cwd":"/srv/app"})
    );
    let mut bare = shell_layout(None, None);
    with_cwd(&mut bare, "/srv/app");
    assert_eq!(bare["block"]["options"], json!({"cwd":"/srv/app"}));
    let mut split = json!({"split":{}});
    with_cwd(&mut split, "/srv/app");
    assert_eq!(split, json!({"split":{}}));
}

#[cfg(test)]
#[test]
fn shell_layout_sets_the_shell_and_theme_only_when_given() {
    assert_eq!(
        shell_layout(None, None),
        json!({"block":{"flavor":"com.superlogical.terminal.shell"}})
    );
    assert_eq!(shell_layout(Some("  "), None), shell_layout(None, None));
    assert_eq!(
        shell_layout(Some("zsh"), None)["block"]["options"],
        json!({"shell":"zsh"})
    );
    let theme = SetTheme::new().background("#001528");
    assert_eq!(
        shell_layout(Some("zsh"), Some(&theme))["block"]["options"],
        json!({"shell":"zsh","theme":{"background":"#001528"}})
    );
    assert_eq!(
        shell_layout(None, Some(&SetTheme::new()))["block"]["options"],
        json!({"theme":{}}),
        "Rex's own colours"
    );
}

#[derive(Clone, Debug)]
pub struct Placement {
    pub block: String,
    pub rect: [f64; 4],
}
#[derive(Clone, Debug)]
pub struct Divider {
    pub before: String,
    pub after: String,
    pub horizontal: bool,
    pub rect: [f64; 4],
    pub ratio: f64,
}
impl Divider {
    pub fn key(&self) -> String {
        format!("{}:{}", self.before, self.after)
    }
}
pub fn placements(window: &Window, drag: Option<(&str, f64)>) -> (Vec<Placement>, Vec<Divider>) {
    if let Some(block) = &window.zoomed_block_id {
        return (
            vec![Placement {
                block: block.clone(),
                rect: [0., 0., 1., 1.],
            }],
            vec![],
        );
    }
    let mut panes = Vec::new();
    let mut dividers = Vec::new();
    for layer in &window.layers {
        // Its blocks keep running, but a minimized layer isn't drawn. Unknown states draw.
        if layer.state.as_deref() == Some("minimized") {
            continue;
        }
        if let Some(node) = &layer.layout {
            let rect = layer
                .bounds
                .as_slice()
                .try_into()
                .unwrap_or([0., 0., 1., 1.]);
            walk(node, rect, drag, &mut panes, &mut dividers);
        }
    }
    (panes, dividers)
}
fn first(node: &ViewLayoutNode) -> Option<&str> {
    node.block_id
        .as_deref()
        .or_else(|| first(node.split.as_ref()?.before.as_deref()?))
}
fn walk(
    node: &ViewLayoutNode,
    rect: [f64; 4],
    drag: Option<(&str, f64)>,
    panes: &mut Vec<Placement>,
    dividers: &mut Vec<Divider>,
) {
    if let Some(block) = &node.block_id {
        panes.push(Placement {
            block: block.clone(),
            rect,
        });
        return;
    }
    let Some(split) = &node.split else {
        return;
    };
    let (Some(before), Some(after)) = (&split.before, &split.after) else {
        return;
    };
    let horizontal = split.direction != rexrs::api::Direction::Vertical;
    let key = format!(
        "{}:{}",
        first(before).unwrap_or(""),
        first(after).unwrap_or("")
    );
    let ratio = drag
        .filter(|(id, _)| *id == key)
        .map_or(split.ratio, |(_, r)| r);
    if let (Some(before), Some(after)) = (first(before), first(after)) {
        dividers.push(Divider {
            before: before.into(),
            after: after.into(),
            horizontal,
            rect,
            ratio,
        });
    }
    let [x, y, w, h] = rect;
    let (a, b) = if horizontal {
        (
            [x, y, w * ratio, h],
            [x + w * ratio, y, w * (1. - ratio), h],
        )
    } else {
        (
            [x, y, w, h * ratio],
            [x, y + h * ratio, w, h * (1. - ratio)],
        )
    };
    walk(before, a, drag, panes, dividers);
    walk(after, b, drag, panes, dividers);
}

/// Proposed resize requests that give each branch in a same-direction chain equal space.
/// A perpendicular subtree occupies one slot; it is balanced independently. Only the returned
/// ratios are sent to the server: the caller must not install the proposed layout locally.
pub fn balanced(window: &Window) -> Vec<Divider> {
    fn slots(node: &ViewLayoutNode, direction: rexrs::api::Direction) -> usize {
        if node.block_id.is_none()
            && let Some(split) = &node.split
            && split.direction == direction
            && let (Some(before), Some(after)) = (&split.before, &split.after)
        {
            slots(before, direction) + slots(after, direction)
        } else {
            1
        }
    }
    fn balance(node: &mut ViewLayoutNode) {
        if node.block_id.is_some() {
            return;
        }
        let Some(split) = &mut node.split else {
            return;
        };
        let (Some(before), Some(after)) = (&mut split.before, &mut split.after) else {
            return;
        };
        let a = slots(before, split.direction);
        let b = slots(after, split.direction);
        split.ratio = a as f64 / (a + b) as f64;
        balance(before);
        balance(after);
    }
    let mut proposal = window.clone();
    proposal.zoomed_block_id = None;
    for layer in &mut proposal.layers {
        if let Some(node) = &mut layer.layout {
            balance(node);
        }
    }
    placements(&proposal, None).1
}

/// A side of a pane: where its neighbour is, or which way one of its borders moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Up,
    Down,
}

impl Side {
    /// In the order menus list them.
    pub const ALL: [Side; 4] = [Side::Left, Side::Right, Side::Up, Side::Down];

    /// The `direction` argument of an action that names a side.
    pub fn arg(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
            Side::Up => "up",
            Side::Down => "down",
        }
    }

    pub fn parse(arg: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|side| side.arg() == arg)
    }

    fn sideways(self) -> bool {
        matches!(self, Side::Left | Side::Right)
    }
}

/// How far a keyboard resize moves a border: this fraction of its split.
pub const RESIZE_STEP: f64 = 0.05;
/// The ratios a border can be given, as a divider drag has them.
pub const RATIO_RANGE: (f64, f64) = (0.1, 0.9);
/// Rects are fractions of the window; two edges this close are the same edge.
const SAME: f64 = 1e-6;

/// The panes and borders of the layer `block` is in, as the server laid them out. None while
/// a pane is zoomed, since no other pane is beside it then, or while its layer is minimized.
fn beside(window: &Window, block: &str) -> Option<(Vec<Placement>, Vec<Divider>)> {
    if window.zoomed_block_id.is_some() {
        return None;
    }
    window.layers.iter().find_map(|layer| {
        if layer.state.as_deref() == Some("minimized") {
            return None;
        }
        let (mut panes, mut dividers) = (Vec::new(), Vec::new());
        // Only how panes lie against each other matters here, so the layer's own bounds do not.
        walk(
            layer.layout.as_ref()?,
            [0., 0., 1., 1.],
            None,
            &mut panes,
            &mut dividers,
        );
        panes
            .iter()
            .any(|pane| pane.block == block)
            .then_some((panes, dividers))
    })
}

/// The pane that lies against `side` of `block` in the server's layout: of several, the one
/// that shares the most of that edge, and of equals the first in layout order.
pub fn neighbour(window: &Window, block: &str, side: Side) -> Option<String> {
    let (panes, _) = beside(window, block)?;
    let [x, y, w, h] = panes.iter().find(|pane| pane.block == block)?.rect;
    let mut best: Option<(f64, &Placement)> = None;
    for pane in panes.iter().filter(|pane| pane.block != block) {
        let [px, py, pw, ph] = pane.rect;
        let touches = match side {
            Side::Left => (px + pw - x).abs() < SAME,
            Side::Right => (px - (x + w)).abs() < SAME,
            Side::Up => (py + ph - y).abs() < SAME,
            Side::Down => (py - (y + h)).abs() < SAME,
        };
        let shared = if side.sideways() {
            (py + ph).min(y + h) - py.max(y)
        } else {
            (px + pw).min(x + w) - px.max(x)
        };
        if touches && shared > SAME && best.is_none_or(|(most, _)| shared > most + SAME) {
            best = Some((shared, pane));
        }
    }
    best.map(|(_, pane)| pane.block.clone())
}

/// The border of `block` a keyboard resize toward `side` moves, with the ratio that moves it
/// one step that way: the border on the pane's right (or under it), or the one on its left
/// (or above it) for a pane that has none there. None when the pane has no such border, or
/// the border is already as far that way as a ratio goes.
pub fn resized(window: &Window, block: &str, side: Side) -> Option<Divider> {
    resized_by(window, block, side, RESIZE_STEP)
}

/// Resize by a fraction of the containing split, from its last server-reported ratio.
pub fn resized_by(window: &Window, block: &str, side: Side, amount: f64) -> Option<Divider> {
    if !amount.is_finite() || amount <= 0. {
        return None;
    }
    let (panes, dividers) = beside(window, block)?;
    let [x, y, w, h] = panes.iter().find(|pane| pane.block == block)?.rect;
    let sideways = side.sideways();
    // A border of the pane: a split the right way round, as long as the pane or longer,
    // whose dividing line is on the pane's edge at `at`.
    let border = |at: f64| {
        dividers.iter().find(|divider| {
            let [dx, dy, dw, dh] = divider.rect;
            let (line, from, to, start, end) = if sideways {
                (dx + dw * divider.ratio, dy, dy + dh, y, y + h)
            } else {
                (dy + dh * divider.ratio, dx, dx + dw, x, x + w)
            };
            divider.horizontal == sideways
                && (line - at).abs() < SAME
                && from < start + SAME
                && to > end - SAME
        })
    };
    let (far, near) = if sideways { (x + w, x) } else { (y + h, y) };
    let mut divider = border(far).or_else(|| border(near))?.clone();
    let step = match side {
        Side::Left | Side::Up => -amount,
        Side::Right | Side::Down => amount,
    };
    // Whole steps, so a ratio does not gather rounding noise as it is stepped.
    let ratio = ((divider.ratio + step) * 1e4).round() / 1e4;
    let ratio = ratio.clamp(RATIO_RANGE.0, RATIO_RANGE.1);
    // A ratio the server holds outside that range (any client may set one between 0 and 1)
    // is left alone rather than pulled back in, which would move the border the other way.
    if (ratio - divider.ratio) * step.signum() < SAME {
        return None;
    }
    divider.ratio = ratio;
    Some(divider)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn nested_layout_and_drag_keep_server_order_and_area() {
        let window: Window = serde_json::from_value(json!({"layers":[{"kind":"tiled","bounds":[0,0,1,1],"layout":{"split":{"direction":"horizontal","ratio":0.6,"before":{"block_id":"a"},"after":{"split":{"direction":"vertical","ratio":0.4,"before":{"block_id":"b"},"after":{"block_id":"c"}}}}},"blocks":[]}]})).unwrap();
        let (panes, borders) = placements(&window, None);
        assert_eq!(
            panes.iter().map(|p| p.block.as_str()).collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
        assert_eq!(panes[1].rect, [0.6, 0., 0.4, 0.4]);
        let key = borders[0].key();
        let (panes, _) = placements(&window, Some((&key, 0.7)));
        assert_eq!(panes[0].rect, [0., 0., 0.7, 1.]);
        assert!((panes.iter().map(|p| p.rect[2] * p.rect[3]).sum::<f64>() - 1.).abs() < 1e-9);
    }
    #[test]
    fn zoom_and_floating_layers_use_window_coordinates() {
        let mut window: Window = serde_json::from_value(json!({"layers":[{"kind":"floating","bounds":[0.2,0.1,0.5,0.6],"layout":{"block_id":"floating"}}]})).unwrap();
        assert_eq!(placements(&window, None).0[0].rect, [0.2, 0.1, 0.5, 0.6]);
        window.zoomed_block_id = Some("floating".into());
        assert_eq!(placements(&window, None).0[0].rect, [0., 0., 1., 1.]);
        assert!(placements(&window, None).1.is_empty());
    }
    #[test]
    fn minimized_layers_are_not_drawn_and_unknown_states_are() {
        let window: Window = serde_json::from_value(json!({"layers":[
            {"kind":"tiled","bounds":[0,0,1,1],"layout":{"block_id":"tiled"}},
            {"kind":"floating","state":"minimized","bounds":[0.2,0.1,0.5,0.6],"layout":{"block_id":"hidden"}},
            {"kind":"floating","state":"docked","bounds":[0.2,0.1,0.5,0.6],"layout":{"block_id":"later"}}
        ]}))
        .unwrap();
        let (panes, _) = placements(&window, None);
        assert_eq!(
            panes.iter().map(|p| p.block.as_str()).collect::<Vec<_>>(),
            ["tiled", "later"]
        );
    }
    /// `a` on the left 60%; on the right `b` (the top 40%) over `c` beside `d`. A floating
    /// pane lies over them.
    fn three_by_two() -> Window {
        serde_json::from_value(json!({"layers":[
            {"kind":"tiled","bounds":[0,0,1,1],"layout":{"split":{"direction":"horizontal","ratio":0.6,
                "before":{"block_id":"a"},
                "after":{"split":{"direction":"vertical","ratio":0.4,
                    "before":{"block_id":"b"},
                    "after":{"split":{"direction":"horizontal","ratio":0.5,
                        "before":{"block_id":"c"},"after":{"block_id":"d"}}}}}}}},
            {"kind":"floating","bounds":[0.2,0.1,0.5,0.6],"layout":{"block_id":"float"}}
        ]}))
        .unwrap()
    }
    #[test]
    fn balance_equalizes_three_pane_chains_without_mutating_server_geometry() {
        for direction in ["horizontal", "vertical"] {
            let window: Window = serde_json::from_value(json!({"layers":[{
                "kind":"tiled", "bounds":[0,0,1,1],
                "layout":{"split":{"direction":direction,"ratio":0.5,
                    "before":{"block_id":"a"},
                    "after":{"split":{"direction":direction,"ratio":0.7,
                        "before":{"block_id":"b"},"after":{"block_id":"c"}}}}}
            }]}))
            .unwrap();
            let borders = balanced(&window);
            assert_eq!(borders.len(), 2);
            assert_eq!((&*borders[0].before, &*borders[0].after), ("a", "b"));
            assert_eq!(borders[0].ratio, 1. / 3.);
            assert_eq!(borders[1].ratio, 0.5);
            assert_eq!(borders[0].horizontal, direction == "horizontal");
            let original = placements(&window, None).1;
            assert_eq!(original[0].ratio, 0.5);
            assert_eq!(original[1].ratio, 0.7);
        }
        let mut mixed = three_by_two();
        mixed.zoomed_block_id = Some("a".into());
        let borders = balanced(&mixed);
        assert_eq!(borders.len(), 3);
        assert!(borders.iter().all(|d| d.ratio == 0.5));
        mixed.layers[0].state = Some("minimized".into());
        assert!(balanced(&mixed).is_empty());
    }

    #[test]
    fn a_neighbour_is_the_pane_against_that_side_sharing_the_most_edge() {
        let window = three_by_two();
        let beside = |block: &str, side| neighbour(&window, block, side);
        assert_eq!(beside("a", Side::Left), None);
        assert_eq!(beside("a", Side::Up), None);
        // `a` meets `b` along 40% of its right edge and `c` along 60%.
        assert_eq!(beside("a", Side::Right).as_deref(), Some("c"));
        assert_eq!(beside("b", Side::Left).as_deref(), Some("a"));
        // Equal shares: the first in layout order.
        assert_eq!(beside("b", Side::Down).as_deref(), Some("c"));
        assert_eq!(beside("c", Side::Up).as_deref(), Some("b"));
        assert_eq!(beside("c", Side::Right).as_deref(), Some("d"));
        assert_eq!(beside("d", Side::Left).as_deref(), Some("c"));
        assert_eq!(beside("d", Side::Right), None);
        assert_eq!(beside("d", Side::Down), None);
        // A floating pane has no neighbour in the tiled layer, nor a tiled pane in it.
        for side in Side::ALL {
            assert_eq!(beside("float", side), None);
            assert_ne!(beside("a", side).as_deref(), Some("float"));
            assert_eq!(beside("missing", side), None);
            assert_eq!(Side::parse(side.arg()), Some(side));
        }
        assert_eq!(Side::parse("sideways"), None);
        let mut zoomed = window.clone();
        zoomed.zoomed_block_id = Some("b".into());
        assert_eq!(neighbour(&zoomed, "b", Side::Left), None);
        assert!(resized(&zoomed, "b", Side::Left).is_none());
    }
    #[test]
    fn a_resize_steps_the_border_after_the_pane_or_else_the_one_before_it() {
        let window = three_by_two();
        let step = |block: &str, side| {
            resized(&window, block, side).map(|d| (d.before, d.after, d.horizontal, d.ratio))
        };
        let border = |before: &str, after: &str, horizontal, ratio| {
            Some((before.to_owned(), after.to_owned(), horizontal, ratio))
        };
        // `a` has a border on its right only, and none above or below.
        assert_eq!(step("a", Side::Left), border("a", "b", true, 0.55));
        assert_eq!(step("a", Side::Right), border("a", "b", true, 0.65));
        assert_eq!(step("a", Side::Up), None);
        assert_eq!(step("a", Side::Down), None);
        // `b` reaches the right edge: sideways it moves the border on its left.
        assert_eq!(step("b", Side::Right), border("a", "b", true, 0.65));
        assert_eq!(step("b", Side::Down), border("b", "c", false, 0.45));
        assert_eq!(step("b", Side::Up), border("b", "c", false, 0.35));
        // `c`: the border it shares with `d`, not the outer one on its left; `d` has that
        // one alone. Up and down, both have only the border above them.
        assert_eq!(step("c", Side::Left), border("c", "d", true, 0.45));
        assert_eq!(step("d", Side::Left), border("c", "d", true, 0.45));
        assert_eq!(step("d", Side::Up), border("b", "c", false, 0.35));
        assert_eq!(step("float", Side::Left), None);
    }
    #[test]
    fn a_resize_stops_at_the_ratios_a_drag_stops_at() {
        let at = |ratio: f64| -> Window {
            serde_json::from_value(json!({"layers":[{"kind":"tiled","bounds":[0,0,1,1],
                "layout":{"split":{"direction":"horizontal","ratio":ratio,
                    "before":{"block_id":"a"},"after":{"block_id":"b"}}}}]}))
            .unwrap()
        };
        assert_eq!(resized(&at(0.12), "a", Side::Left).unwrap().ratio, 0.1);
        assert!(resized(&at(0.1), "a", Side::Left).is_none());
        assert_eq!(resized(&at(0.1), "a", Side::Right).unwrap().ratio, 0.15);
        assert_eq!(resized(&at(0.88), "b", Side::Right).unwrap().ratio, 0.9);
        assert!(resized(&at(0.9), "b", Side::Right).is_none());
        // A ratio another client set past those stops never moves back against the key.
        assert!(resized(&at(0.95), "a", Side::Right).is_none());
        assert!(resized(&at(0.05), "a", Side::Left).is_none());
        assert_eq!(resized(&at(0.95), "a", Side::Left).unwrap().ratio, 0.9);
        assert_eq!(resized(&at(0.05), "a", Side::Right).unwrap().ratio, 0.1);
        // Stepping there and back leaves no rounding noise in the ratio.
        assert_eq!(resized(&at(0.3), "a", Side::Right).unwrap().ratio, 0.35);
        assert_eq!(resized(&at(0.35), "a", Side::Left).unwrap().ratio, 0.3);
    }
}
