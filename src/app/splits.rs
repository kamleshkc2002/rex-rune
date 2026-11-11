//! Where a new split goes: on any side of a pane, or along its longer side. The client names
//! the anchor, the direction and the side; the server places the pane. Also the focus moves
//! through a tab's panes in the server's layout order.
use super::{
    panel::{Entry, Intent},
    workspace::Workspace,
};
use serde_json::{Value, json};

pub const FOCUS_NEXT: &str = "pane.focus_next";
pub const FOCUS_PREVIOUS: &str = "pane.focus_previous";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitWay {
    Right,
    Left,
    Down,
    Up,
    /// Right of a pane wider than it is tall, otherwise below it.
    BestFit,
}

impl SplitWay {
    /// In the order menus list them.
    pub const ALL: [SplitWay; 5] = [
        SplitWay::Right,
        SplitWay::Left,
        SplitWay::Down,
        SplitWay::Up,
        SplitWay::BestFit,
    ];

    /// The `direction` argument of `pane.split`.
    pub fn arg(self) -> &'static str {
        match self {
            SplitWay::Right => "right",
            SplitWay::Left => "left",
            SplitWay::Down => "down",
            SplitWay::Up => "up",
            SplitWay::BestFit => "best_fit",
        }
    }

    pub fn parse(arg: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|way| way.arg() == arg)
    }

    pub fn label(self) -> &'static str {
        match self {
            SplitWay::Right => "Split right",
            SplitWay::Left => "Split left",
            SplitWay::Down => "Split down",
            SplitWay::Up => "Split up",
            SplitWay::BestFit => "Split best fit",
        }
    }

    /// Rex's `direction` and `side` for a split of a pane `size` points wide and tall. A pane
    /// that has not been laid out yet (no size) splits right.
    pub fn place(self, size: [f32; 2]) -> (&'static str, &'static str) {
        match self {
            SplitWay::Right => ("horizontal", "after"),
            SplitWay::Left => ("horizontal", "before"),
            SplitWay::Down => ("vertical", "after"),
            SplitWay::Up => ("vertical", "before"),
            SplitWay::BestFit if size[1] > size[0] => ("vertical", "after"),
            SplitWay::BestFit => ("horizontal", "after"),
        }
    }
}

impl Workspace {
    /// The `session.new_split` payload that puts `layout` beside `block` of `host`.
    pub(super) fn split_payload(
        &self,
        host: usize,
        block: &str,
        way: SplitWay,
        layout: Value,
    ) -> Value {
        let size = self
            .terminals
            .get(&(self.hosts[host].id, block.to_owned()))
            .map_or([0.; 2], |term| [term.rect[2], term.rect[3]]);
        let (direction, side) = way.place(size);
        json!({"anchor_id":block,"direction":direction,"side":side,"layout":layout,"focus":true})
    }

    /// Move focus to the next (`forward`) or previous pane of the tab on screen, going round
    /// at the ends. The server orders the panes and moves the focus.
    pub(super) fn cycle_pane(&mut self, forward: bool) {
        let method = if forward {
            "session.focus_next_pane"
        } else {
            "session.focus_previous_pane"
        };
        self.request(method, json!({"wrap": true}));
    }

    /// Commands' rows for the focus moves, which need a second pane to move to: next and
    /// previous in the tab's order, then the pane on each side.
    pub(super) fn focus_commands(&self) -> [Entry; 6] {
        let model = self.model();
        let panes = model.window().map_or(0, |window| {
            window.layers.iter().map(|layer| layer.blocks.len()).sum()
        });
        let direction = |side: &str| ("pane.focus", json!({ "direction": side }));
        [
            ("Focus next pane", (FOCUS_NEXT, Value::Null)),
            ("Focus previous pane", (FOCUS_PREVIOUS, Value::Null)),
            ("Focus pane left", direction("left")),
            ("Focus pane right", direction("right")),
            ("Focus pane up", direction("up")),
            ("Focus pane down", direction("down")),
        ]
        .map(|(label, (action, args))| {
            let mut entry = Entry::new(label, "", "", Intent::ClientAction(action, args));
            entry.enabled = model.connected && panes > 1;
            entry
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_way_names_a_direction_and_a_side() {
        let wide = [800., 400.];
        for (way, placed) in [
            (SplitWay::Right, ("horizontal", "after")),
            (SplitWay::Left, ("horizontal", "before")),
            (SplitWay::Down, ("vertical", "after")),
            (SplitWay::Up, ("vertical", "before")),
            (SplitWay::BestFit, ("horizontal", "after")),
        ] {
            assert_eq!(way.place(wide), placed, "{way:?}");
            assert_eq!(SplitWay::parse(way.arg()), Some(way));
        }
        assert_eq!(SplitWay::parse("sideways"), None);
    }

    #[test]
    fn best_fit_splits_the_longer_side() {
        assert_eq!(
            SplitWay::BestFit.place([400., 800.]),
            ("vertical", "after"),
            "a tall pane splits down"
        );
        assert_eq!(
            SplitWay::BestFit.place([600., 600.]),
            ("horizontal", "after"),
            "a square pane splits right"
        );
        assert_eq!(
            SplitWay::BestFit.place([0., 0.]),
            ("horizontal", "after"),
            "a pane with no size yet splits right"
        );
    }
}
