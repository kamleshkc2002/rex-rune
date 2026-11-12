//! Local-only spellings from Rex's action reference. Aliases reuse the original handlers;
//! declaring one here must never enlarge the remote allowed set.
use super::{
    actions,
    panel::{Intent, PaneAction, PanelKind},
    workspace::Workspace,
};
use rex_linux::{
    connection::{Command, HostCommand},
    layout,
};
use rune::Tx;
use serde_json::{Value, json};

pub const NEW_TAB: &str = "Move pane to new tab is recognized but not supported by the supported Rex server API: session.new_window creates new programs, not a window for an existing pane. Move to an existing tab instead.";

pub fn unsupported(name: &str) -> Option<&'static str> {
    match name {
        "pane.move_to_new_tab" => Some(NEW_TAB),
        _ => None,
    }
}

pub fn alias(name: &str) -> Option<(&'static str, Value)> {
    let direction = name.rsplit('.').next()?;
    if name.starts_with("pane.focus.") {
        return Some(("pane.focus", json!({"direction":direction})));
    }
    if name.starts_with("pane.split.") {
        return Some((
            "pane.split",
            json!({"direction":if direction == "auto" {"best_fit"} else {direction}}),
        ));
    }
    if name.starts_with("pane.move.") {
        return Some(("pane.move", json!({"direction":direction})));
    }
    Some(match name {
        "pane.prompt_previous" => ("client.prompt.previous", Value::Null),
        "pane.prompt_next" => ("client.prompt.next", Value::Null),
        "client.find.open" => (actions::SEARCH, Value::Null),
        "client.find.next" => (super::find::NEXT, Value::Null),
        "client.find.previous" => (super::find::PREVIOUS, Value::Null),
        "client.theme.change" => (super::commands::CHOOSE_THEME, Value::Null),
        "client.tab.move.backward" => ("client.tab.move", json!({"direction":"left"})),
        "client.tab.move.forward" => ("client.tab.move", json!({"direction":"right"})),
        _ => return None,
    })
}

pub fn title(name: &str) -> &'static str {
    match name {
        "client.config.reload" => "Reload config",
        "client.config.copy_path" => "Copy config file path",
        "client.prompt.previous" | "pane.prompt_previous" => "Previous prompt",
        "client.prompt.next" | "pane.prompt_next" => "Next prompt",
        "pane.resize" => "Resize pane",
        "pane.balance" => "Balance splits",
        "pane.rename" => "Rename pane",
        "pane.close_others" => "Close other panes",
        "pane.move" => "Move pane",
        "pane.move.left" => "Move pane left",
        "pane.move.right" => "Move pane right",
        "pane.move.up" => "Move pane up",
        "pane.move.down" => "Move pane down",
        "pane.move_to_tab" => "Move pane to tab",
        "pane.move_to_new_tab" => "Move pane to new tab (not supported)",
        "pane.focus.left" => "Focus pane left",
        "pane.focus.right" => "Focus pane right",
        "pane.focus.up" => "Focus pane up",
        "pane.focus.down" => "Focus pane down",
        "pane.split.left" => "Split left",
        "pane.split.right" => "Split right",
        "pane.split.up" => "Split up",
        "pane.split.down" => "Split down",
        "pane.split.auto" => "Split best fit",
        "client.find.open" => "Search terminal",
        "client.find.next" => "Find next",
        "client.find.previous" => "Find previous",
        "client.tab.move.backward" => "Move tab backward",
        "client.tab.move.forward" => "Move tab forward",
        "client.host.switch" => "Switch host",
        "client.host.add" => "Add host",
        "client.host.remove" => "Remove host",
        "client.theme.change" => "Change terminal theme",
        "client.mode.enter" => "Enter key mode",
        "client.mode.exit" => "Exit key mode",
        "client.mode.exit_all" => "Exit all key modes",
        "client.scroll.page_up" => "Scroll page up",
        "client.scroll.page_down" => "Scroll page down",
        _ => unreachable!("unregistered compatibility action {name}"),
    }
}

pub fn category(name: &str) -> &'static str {
    if name.starts_with("pane.") {
        "Panes"
    } else if name.starts_with("client.find.")
        || name.starts_with("client.scroll.")
        || name.starts_with("client.prompt.")
    {
        "Text and view"
    } else if name.starts_with("client.tab.") || name == "client.host.switch" {
        "Navigate"
    } else {
        "Interface"
    }
}

pub fn schema(name: &str) -> Option<Value> {
    let direction = json!({"type":"string","enum":["left","right","up","down"]});
    Some(match name {
        "pane.resize" => {
            json!({"type":"object","properties":{"direction":direction,"amount":{"type":"number","exclusiveMinimum":0,"default":5}},"required":["direction"],"additionalProperties":false})
        }
        "pane.move" => {
            json!({"type":"object","properties":{"direction":direction},"required":["direction"],"additionalProperties":false})
        }
        "pane.move_to_tab" => {
            json!({"type":"object","properties":{"window_id":{"type":"string","minLength":1}},"additionalProperties":false})
        }
        "client.mode.enter" => {
            json!({"type":"object","properties":{"name":{"type":"string","pattern":"^[a-z0-9_-]+$"},"once":{"type":"boolean"}},"required":["name"],"additionalProperties":false})
        }
        "client.find.next" | "client.find.previous" => {
            json!({"type":"object","properties":{"when":{"type":"string","enum":["searching"]}},"additionalProperties":false})
        }
        _ => return None,
    })
}

pub fn accepts(name: &str, args: &Value) -> bool {
    let Some(object) = args.as_object() else {
        return args.is_null()
            && !matches!(name, "pane.resize" | "pane.move" | "client.mode.enter");
    };
    match name {
        "pane.resize" => {
            object.keys().all(|k| k == "direction" || k == "amount")
                && super::pane_moves::side(args).is_some()
                && object
                    .get("amount")
                    .is_none_or(|n| n.as_f64().is_some_and(|n| n.is_finite() && n > 0.))
        }
        "pane.move" => object.len() == 1 && super::pane_moves::side(args).is_some(),
        "pane.move_to_tab" => {
            object.is_empty()
                || object.len() == 1 && args["window_id"].as_str().is_some_and(|s| !s.is_empty())
        }
        "client.mode.enter" => {
            object.keys().all(|k| k == "name" || k == "once")
                && args["name"].as_str().is_some_and(|s| {
                    !s.is_empty()
                        && s.bytes().all(|c| {
                            c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_' || c == b'-'
                        })
                })
                && object.get("once").is_none_or(Value::is_boolean)
        }
        "client.find.next" | "client.find.previous" => {
            object.is_empty() || object.len() == 1 && args["when"] == "searching"
        }
        _ => object.is_empty(),
    }
}

pub fn run(s: &mut Workspace, name: &str, args: &Value, tx: &mut Tx) {
    if args["when"] == "searching" && s.active_terminal().is_none_or(|t| t.search.is_none()) {
        return;
    }
    if let Some(reason) = unsupported(name) {
        s.notify(reason);
        return;
    }
    if let Some((action, args)) = alias(name) {
        actions::run(s, action, &args, tx);
        return;
    }
    match name {
        "client.config.reload" => {
            s.reload_shortcuts();
            s.bindings.clear_modes();
            s.send(
                s.active,
                HostCommand::Request {
                    method: "server.config_reload".into(),
                    payload: json!({}),
                },
            );
        }
        "client.config.copy_path" => {
            if let Some(snapshot) = s.keymap_snapshot() {
                tx.copy(snapshot.reply.config.path.clone());
            } else {
                s.notify("The server config path is not available until its keymap arrives.");
            }
        }
        "pane.rename" => {
            if let Some(target) = s.focused_block().and_then(|b| s.target(&b)) {
                s.perform(Intent::Pane(target, PaneAction::Rename), tx);
            }
        }
        "pane.close_others" => {
            if let Some(target) = s.focused_block().and_then(|b| s.target(&b)) {
                s.close_other_panes(target);
            }
        }
        "pane.move" => s.move_pane(super::pane_moves::side(args).unwrap()),
        "pane.resize" => s.resize_pane(
            super::pane_moves::side(args).unwrap(),
            args["amount"].as_f64().unwrap_or(5.),
        ),
        "pane.balance" => {
            if let Some(window) = s.model().window() {
                for divider in layout::balanced(window) {
                    let direction = if divider.horizontal {
                        "horizontal"
                    } else {
                        "vertical"
                    };
                    s.request(
                        "session.resize_pane",
                        json!({
                            "direction": direction,
                            "before_block_id": divider.before,
                            "after_block_id": divider.after,
                            "ratio": divider.ratio,
                        }),
                    );
                }
            }
        }
        "pane.move_to_tab" => {
            if let Some(id) = args["window_id"].as_str() {
                s.move_pane_to_tab(id, tx);
            } else if let Some(target) = s.focused_block().and_then(|b| s.target(&b)) {
                s.open_panel(
                    PanelKind::MovePane,
                    "Move pane to tab",
                    &[("", "Filter tabs…", "")],
                );
                s.panel.as_mut().unwrap().target = Some(target);
            }
        }
        "client.host.switch" => s.open_panel(
            PanelKind::SwitchHost,
            "Switch host",
            &[("", "Filter hosts…", "")],
        ),
        "client.host.add" => s.perform(Intent::AddHost, tx),
        "client.host.remove" => {
            if s.hosts.len() > 1 {
                s.perform(Intent::RemoveHost(s.active), tx);
            } else {
                s.notify("Keep at least one host. Add another host before removing this one.");
            }
        }
        "client.mode.enter" => {
            let name = args["name"].as_str().unwrap();
            let known = s
                .effective_keymap()
                .bindings
                .iter()
                .any(|b| b.binding.mode.as_deref() == Some(name))
                || s.keymap_snapshot()
                    .is_some_and(|k| k.reply.modes.iter().any(|m| m.name == name));
            if known {
                s.bindings
                    .enter_mode(name, args["once"].as_bool().unwrap_or(false));
                s.inform(format!("Key mode: {name} · Escape to leave"));
            } else {
                s.notify(format!(
                    "Key mode {name:?} is not defined by this host's keymap."
                ));
            }
        }
        "client.mode.exit" => {
            s.bindings.exit_mode();
        }
        "client.mode.exit_all" => s.bindings.clear_modes(),
        "client.prompt.previous" | "client.prompt.next" => {
            if let Some(term) = s.active_terminal_mut() {
                term.send(Command::PromptStep(name == "client.prompt.previous"));
            }
        }
        "client.scroll.page_up" | "client.scroll.page_down" => {
            let cell = super::term::metrics(s.settings.font_size).cell;
            if let Some(term) = s.active_terminal_mut() {
                let rows = term.visible_rows(cell).max(1) as isize;
                term.send(Command::Scroll(if name.ends_with("up") {
                    -rows
                } else {
                    rows
                }));
            }
        }
        _ => unreachable!("undispatched compatibility action {name}"),
    }
}

impl Workspace {
    pub(super) fn move_pane_to_tab(&mut self, id: &str, tx: &mut Tx) {
        let target = self
            .panel
            .as_ref()
            .and_then(|p| p.target.clone())
            .or_else(|| self.focused_block().and_then(|b| self.target(&b)));
        let Some(target) = target.filter(|t| self.target_exists(t)) else {
            return;
        };
        let anchor = self.hosts[target.host]
            .model
            .views
            .get(&target.session)
            .and_then(|view| {
                view.windows
                    .iter()
                    .find(|w| w.window_id.as_deref() == Some(id))
            })
            .filter(|w| {
                !w.layers
                    .iter()
                    .flat_map(|l| &l.blocks)
                    .any(|b| b.block_id.as_deref() == Some(&target.block))
            })
            .and_then(|w| {
                w.focused_block_id.clone().or_else(|| {
                    w.layers
                        .iter()
                        .flat_map(|l| &l.blocks)
                        .find_map(|b| b.block_id.clone())
                })
            });
        let Some(anchor) = anchor else {
            self.notify("Choose another tab with a pane in the same session.");
            return;
        };
        self.close_panel(tx);
        self.send(
            target.host,
            HostCommand::Request {
                method: "session.move_block".into(),
                payload: json!({
                    "session_id": target.session,
                    "block_id": target.block,
                    "anchor_block_id": anchor,
                    "direction": "horizontal",
                    "side": "after",
                }),
            },
        );
    }
}
