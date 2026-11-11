//! Moving and resizing the focused pane from the keyboard and Commands, and closing every
//! other pane of a tab.
//!
//! The server owns the layout. Move pane names two panes for `session.swap_blocks`: the
//! focused one and the one lying against the named side, so they trade places and every
//! split keeps its size. Resize pane names one border of the focused pane and its next ratio
//! for `session.resize_pane`, as a divider drag does. Which pane is beside which is read
//! from the server's layout (`layout::neighbour`, `layout::resized`); nothing is placed here.
use super::{panel::PaneTarget, workspace::Workspace};
use rex_linux::{
    connection::HostCommand,
    layout::{self, Side},
};
use serde_json::{Value, json};

/// The side a `direction` argument names.
pub fn side(args: &Value) -> Option<Side> {
    args["direction"].as_str().and_then(Side::parse)
}

impl Workspace {
    /// The pane lying against `side` of the focused one, in the tab on screen.
    fn pane_beside(&self, side: Side) -> Option<(String, String)> {
        let model = self.model();
        let block = model.focused()?;
        let other = layout::neighbour(model.window()?, block, side)?;
        Some((block.to_owned(), other))
    }

    /// Trade the focused pane's place with the pane on that side of it. The server swaps the
    /// two, and focus follows the pane. At the tab's edge, or with a pane zoomed, there is
    /// no such pane and nothing is asked.
    pub(super) fn move_pane(&mut self, side: Side) {
        if let Some((block, other)) = self.pane_beside(side) {
            self.request(
                "session.swap_blocks",
                json!({"block_id": block, "other_block_id": other}),
            );
        }
    }

    fn pane_border(&self, side: Side, amount: f64) -> Option<layout::Divider> {
        let model = self.model();
        layout::resized_by(model.window()?, model.focused()?, side, amount / 100.)
    }

    /// Move a border of the focused pane one step toward `side`: the one on its right or
    /// under it, else the one on its left or above it. The step is from the ratio the server
    /// last reported, so a held key never runs ahead of the server.
    pub(super) fn resize_pane(&mut self, side: Side, amount: f64) {
        if let Some(border) = self.pane_border(side, amount) {
            self.request(
                "session.resize_pane",
                json!({
                    "direction": if border.horizontal { "horizontal" } else { "vertical" },
                    "before_block_id": border.before,
                    "after_block_id": border.after,
                    "ratio": border.ratio,
                }),
            );
        }
    }

    /// The other panes of `target`'s tab, in every layer of it.
    pub(super) fn other_panes(&self, target: &PaneTarget) -> Vec<String> {
        self.pane_window(target)
            .into_iter()
            .flat_map(|window| &window.layers)
            .flat_map(|layer| &layer.blocks)
            .filter_map(|block| block.block_id.clone())
            .filter(|id| *id != target.block)
            .collect()
    }

    /// Ask before closing every pane of `target`'s tab but `target`.
    pub(super) fn close_other_panes(&mut self, target: PaneTarget) {
        let others = self.other_panes(&target);
        if others.is_empty() {
            return;
        }
        let (title, which) = match others.len() {
            1 => (
                format!("Close the other pane in {}?", target.tab),
                "the other pane".to_owned(),
            ),
            n => (
                format!("Close {n} other panes in {}?", target.tab),
                format!("each of the {n} other panes"),
            ),
        };
        let description = format!(
            "Ends the program in {which} on {}; {} stays open. Closing the app itself keeps programs running.",
            self.settings.hosts[target.host].label, target.name
        );
        // The panes named now are the ones that close, not ones opened meanwhile.
        self.confirm(
            &title,
            "block.close",
            json!({"session_id": target.session, "block_ids": others}),
            &description,
        );
        self.panel.as_mut().unwrap().target = Some(target);
    }

    /// A confirmed Close other panes: one `block.close` a pane, once. A pane that closed
    /// since the question was asked is skipped, and nothing closes when the pane that was
    /// to stay is gone.
    pub(super) fn confirmed_pane_close(&mut self, target: &PaneTarget, payload: &Value) {
        if !self.target_exists(target) {
            self.inform("That pane is no longer available.");
            return;
        }
        let open = self.other_panes(target);
        let named = payload["block_ids"].as_array().into_iter().flatten();
        for id in named.filter_map(Value::as_str) {
            if open.iter().any(|block| block == id) {
                self.send(
                    target.host,
                    HostCommand::Request {
                        method: "block.close".into(),
                        payload: json!({"session_id": target.session, "block_id": id}),
                    },
                );
            }
        }
    }
}
