//! Tab management: a tab's menu on its toolbar pill or its sidebar row, its place in its
//! session's order, muting it on this client, closing the others, and its default title.
//! Order and labels are the server's (`session.move_window`, `session.set_window_label`);
//! mute is this client's alone.
use super::{
    activity::{default_title, tab_name},
    panel::{Entry, Form, Intent, PanelKind},
    theme::*,
    widgets::glyph,
    workspace::Workspace,
};
use rex_linux::{
    connection::{DEFAULT_TAB_LABEL, HostCommand},
    settings::MutedTab,
};
use rexrs::api::session;
use rune::{El, Tx, el};
use serde_json::{Value, json};

/// Muted tabs are remembered until they close; a server that is never visited again can't
/// grow the list without end.
const MUTED_LIMIT: usize = 256;

/// The tab a tab menu, its rename form or its close confirmations act on: any host's and any
/// session's, not only the one on screen, so every request names it explicitly.
#[derive(Clone, Debug, PartialEq)]
pub struct TabTarget {
    pub host: usize,
    pub session: String,
    pub window: String,
    /// What the tab is called on screen (`activity::tab_name`).
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabAction {
    Rename,
    /// Mute, or unmute a muted tab.
    Mute,
    /// One place toward the start of the session's tab order.
    MoveLeft,
    /// One place toward its end.
    MoveRight,
    /// A new tab in the target's session.
    New,
    CloseOthers,
    Close,
}

/// The mark a muted tab carries where its activity word would be.
pub fn muted_mark(window: &str) -> El {
    el().key(format!("tab-muted:{window}"))
        .help("Muted: no activity word or notifications from this tab")
        .child(glyph(&BELL_OFF, 14., rgb(muted())))
}

/// The outline of the tab a menu, rename form or close confirmation acts on. An overlay, so
/// it never shifts the tab.
pub fn target_ring(window: &str, radius: f32) -> El {
    el().key(format!("tab-target:{window}"))
        .pin(Some(0.), Some(0.), Some(0.), Some(0.))
        .radius(radius)
        .continuous_corners()
        .border(1.5, rgb(accent()))
}

impl Workspace {
    fn tab_window(&self, target: &TabTarget) -> Option<&session::Window> {
        let view = self
            .hosts
            .get(target.host)?
            .model
            .views
            .get(&target.session)?;
        view.windows
            .iter()
            .find(|w| w.window_id.as_deref() == Some(&target.window))
    }

    /// The target for `window` of `session` on host `host`, while that tab is known.
    pub fn tab_target(&self, host: usize, session: &str, window: &str) -> Option<TabTarget> {
        let mut target = TabTarget {
            host,
            session: session.to_owned(),
            window: window.to_owned(),
            name: String::new(),
        };
        target.name = tab_name(&self.hosts[host].model, self.tab_window(&target)?);
        Some(target)
    }

    /// The target for `window` of the session on screen.
    pub fn current_tab(&self, window: &str) -> Option<TabTarget> {
        self.tab_target(self.active, self.model().selected.as_deref()?, window)
    }

    /// The target for the tab on screen.
    pub fn active_tab(&self) -> Option<TabTarget> {
        self.current_tab(self.model().window()?.window_id.as_deref()?)
    }

    /// The tab on screen and its place in the order, so the toolbar reveals it again after
    /// it moves.
    pub fn active_tab_place(&self) -> Option<(String, usize)> {
        let view = self.model().view()?;
        let id = view.active_window_id.clone()?;
        let place = view
            .windows
            .iter()
            .position(|w| w.window_id.as_ref() == Some(&id))
            .unwrap_or(0);
        Some((id, place))
    }

    /// The target's place in its session's tab order and that order, or none once it closed.
    fn tab_order(&self, target: &TabTarget) -> Option<(usize, Vec<String>)> {
        let view = self
            .hosts
            .get(target.host)?
            .model
            .views
            .get(&target.session)?;
        let order: Vec<String> = view
            .windows
            .iter()
            .filter_map(|w| w.window_id.clone())
            .collect();
        let place = order.iter().position(|id| *id == target.window)?;
        Some((place, order))
    }

    /// Whether the target is a tab of the session on screen.
    pub(super) fn tab_on_screen(&self, target: &TabTarget) -> bool {
        target.host == self.active && self.model().selected.as_ref() == Some(&target.session)
    }

    /// Whether a panel (menu, rename form or confirmation) acts on this tab.
    pub fn tab_targeted(&self, host: usize, window: &str) -> bool {
        self.panel
            .as_ref()
            .and_then(|p| p.tab.as_ref())
            .is_some_and(|t| t.host == host && t.window == window)
    }

    /// Whether the user chose a name for the target, so it has a default title to go back to.
    pub fn tab_named(&self, target: &TabTarget) -> bool {
        self.tab_window(target).is_some_and(|w| !default_title(w))
    }

    fn session_label(&self, host: usize, session: &str) -> String {
        let model = &self.hosts[host].model;
        model
            .sessions
            .iter()
            .find(|s| s.session_id == session)
            .map(|s| s.label.clone())
            .or_else(|| model.views.get(session).and_then(|v| v.label.clone()))
            .unwrap_or_else(|| "Session".into())
    }

    /// Whether this client keeps `window` on host `host` quiet.
    pub fn tab_muted(&self, host: usize, window: &str) -> bool {
        let endpoint = &self.settings.hosts[host].endpoint;
        self.settings
            .muted_tabs
            .iter()
            .any(|m| m.window == window && m.endpoint == *endpoint)
    }

    /// Whether `window` shows its activity word and progress bar: Show activity in tabs is
    /// on and the tab isn't muted.
    pub fn tab_activity_shown(&self, host: usize, window: &str) -> bool {
        self.settings.notifications.show_in_tabs && !self.tab_muted(host, window)
    }

    /// Mute or unmute a tab on this client and remember it. Either way its unseen Done and
    /// Bell are dropped: what happened while it was muted is not news afterwards.
    pub fn set_tab_muted(&mut self, target: &TabTarget, muted: bool) {
        let endpoint = self.settings.hosts[target.host].endpoint.clone();
        self.settings
            .muted_tabs
            .retain(|m| !(m.endpoint == endpoint && m.window == target.window));
        if muted {
            self.settings.muted_tabs.push(MutedTab {
                endpoint,
                session: target.session.clone(),
                window: target.window.clone(),
            });
            let extra = self.settings.muted_tabs.len().saturating_sub(MUTED_LIMIT);
            self.settings.muted_tabs.drain(..extra);
        }
        let key = (self.hosts[target.host].id, target.window.clone());
        self.unseen_done.remove(&key);
        self.unseen_bells.remove(&key);
        self.save();
    }

    /// After host `index` changed: forget muted tabs that closed (their session's view is
    /// loaded and no longer lists them) or whose host was removed, and leave a panel whose
    /// tab closed.
    pub(super) fn sync_tabs(&mut self, index: usize, tx: &mut Tx) {
        if !self.settings.muted_tabs.is_empty() {
            let hosts = &self.settings.hosts;
            let endpoint = &hosts[index].endpoint;
            let views = &self.hosts[index].model.views;
            let before = self.settings.muted_tabs.len();
            self.settings.muted_tabs.retain(|m| {
                let closed = m.endpoint == *endpoint
                    && views.get(&m.session).is_some_and(|v| {
                        !v.windows
                            .iter()
                            .any(|w| w.window_id.as_ref() == Some(&m.window))
                    });
                !closed && hosts.iter().any(|h| h.endpoint == m.endpoint)
            });
            if self.settings.muted_tabs.len() != before {
                self.save();
            }
        }
        if self
            .panel
            .as_ref()
            .and_then(|p| p.tab.as_ref())
            .is_some_and(|t| t.host == index && self.tab_order(t).is_none())
        {
            self.close_panel(tx);
            self.inform("That tab is no longer available.");
        }
    }

    /// A tab's menu: Rename and Mute; Move left and right within its session, disabled at the
    /// ends; New tab; then Close other tabs and Close, last. Mute is local, so it stays
    /// available on an offline host.
    pub fn tab_entries(&self, target: &TabTarget) -> Vec<Entry> {
        let connected = self.hosts[target.host].model.connected;
        let (place, count) = self
            .tab_order(target)
            .map_or((0, 0), |(place, order)| (place, order.len()));
        // The menu is the tab's own, so its rows needn't say "tab" to fit their shortcuts.
        let (left, right) = if self.sidebar_tabs() {
            ("Move up", "Move down")
        } else {
            ("Move left", "Move right")
        };
        let mut entries = Vec::new();
        for (label, action, enabled, separated) in [
            ("Rename tab…", TabAction::Rename, connected, false),
            (self.mute_label(target), TabAction::Mute, count > 0, false),
            (left, TabAction::MoveLeft, connected && place > 0, true),
            (
                right,
                TabAction::MoveRight,
                connected && place + 1 < count,
                false,
            ),
            ("New tab", TabAction::New, connected, true),
            (
                "Close other tabs…",
                TabAction::CloseOthers,
                connected && count > 1,
                true,
            ),
            ("Close tab…", TabAction::Close, connected, false),
        ] {
            // On screen, New tab is the toolbar's: it starts in the focused pane's directory.
            let intent = if action == TabAction::New && self.tab_on_screen(target) {
                Intent::NewWindow
            } else {
                Intent::Window(target.clone(), action)
            };
            let mut entry = Entry::new(label, "", "", intent);
            entry.enabled = enabled && count > 0;
            entry.separated = separated;
            entries.push(entry);
        }
        entries
    }

    /// The tab actions Commands lists for the tab on screen, beside Rename tab and Close tab.
    pub fn tab_commands(&self) -> Vec<Entry> {
        let Some(target) = self.active_tab() else {
            return Vec::new();
        };
        let connected = self.model().connected;
        let (place, count) = self
            .tab_order(&target)
            .map_or((0, 0), |(place, order)| (place, order.len()));
        let (left, right) = self.move_labels();
        [
            (self.mute_label(&target), TabAction::Mute, true),
            (left, TabAction::MoveLeft, connected && place > 0),
            (right, TabAction::MoveRight, connected && place + 1 < count),
            (
                "Close other tabs…",
                TabAction::CloseOthers,
                connected && count > 1,
            ),
        ]
        .into_iter()
        .map(|(label, action, enabled)| {
            let mut entry = Entry::new(
                label,
                "",
                "Workspace",
                Intent::Window(target.clone(), action),
            );
            entry.enabled = enabled;
            entry
        })
        .collect()
    }

    fn mute_label(&self, target: &TabTarget) -> &'static str {
        if self.tab_muted(target.host, &target.window) {
            "Unmute tab"
        } else {
            "Mute tab"
        }
    }

    /// Commands' names for the moves: tabs run left to right in the toolbar and top to
    /// bottom in the sidebar.
    fn move_labels(&self) -> (&'static str, &'static str) {
        if self.sidebar_tabs() {
            ("Move tab up", "Move tab down")
        } else {
            ("Move tab left", "Move tab right")
        }
    }

    /// A tab's right-click menu, at the pointer (window points): on its toolbar pill or on
    /// its sidebar row.
    pub fn tab_menu(&mut self, host: usize, session: &str, window: &str, anchor: (f32, f32)) {
        let Some(target) = self.tab_target(host, session, window) else {
            return;
        };
        // A tab of another session says whose it is.
        let mut title = if self.tab_on_screen(&target) {
            target.name.clone()
        } else {
            format!("{} · {}", self.session_label(host, session), target.name)
        };
        if !self.hosts[host].model.connected {
            title.push_str(" · Offline");
        }
        let scale = self.ui_scale();
        self.open_panel(PanelKind::Window, &title, &[]);
        let panel = self.panel.as_mut().unwrap();
        panel.target = None;
        panel.tab = Some(target);
        panel.anchor = Some((anchor.0 / scale, anchor.1 / scale));
        if let Some(first) = self.entries().iter().position(|e| e.enabled) {
            self.panel.as_mut().unwrap().selected = first;
        }
    }

    /// Every tab request names its own host and session, never the ones on screen.
    fn tab_request(&mut self, target: &TabTarget, method: &str, mut payload: Value) {
        payload["session_id"] = json!(target.session);
        self.send(
            target.host,
            HostCommand::Request {
                method: method.into(),
                payload,
            },
        );
    }

    /// Give a tab a name, or with an empty one its default title back.
    pub(super) fn set_tab_label(&mut self, target: &TabTarget, label: &str) {
        self.tab_request(
            target,
            "session.set_window_label",
            json!({"window_id": target.window, "label": label}),
        );
    }

    /// The rename form's Use default title: the server's default label, which shows the
    /// title the tab's program sets.
    pub fn use_default_tab_title(&mut self, tx: &mut Tx) {
        let Some(target) = self.panel.as_ref().and_then(|p| p.tab.clone()) else {
            return;
        };
        self.close_panel(tx);
        self.set_tab_label(&target, "");
    }

    pub fn tab_action(&mut self, target: TabTarget, action: TabAction, tx: &mut Tx) {
        let Some((place, order)) = self.tab_order(&target) else {
            if self.panel.is_some() {
                self.close_panel(tx);
            }
            self.inform("That tab is no longer available.");
            return;
        };
        let host = self.settings.hosts[target.host].label.clone();
        match action {
            TabAction::Rename => {
                let name = target.name.clone();
                self.open_panel(
                    PanelKind::Form(Form::RenameWindow(target.window.clone())),
                    "Rename tab",
                    &[("Tab name", "", &name)],
                );
                self.panel.as_mut().unwrap().tab = Some(target);
            }
            TabAction::Mute => {
                let muted = !self.tab_muted(target.host, &target.window);
                self.set_tab_muted(&target, muted);
            }
            TabAction::MoveLeft | TabAction::MoveRight => {
                let before = match action {
                    TabAction::MoveLeft => place.checked_sub(1),
                    _ => Some(place + 2),
                };
                let mut payload = json!({"window_id": target.window});
                match before.map(|i| order.get(i)) {
                    Some(Some(next)) => payload["before_window_id"] = json!(next),
                    // Right from the next-to-last place: to the end.
                    Some(None) if place + 1 < order.len() => {}
                    _ => return,
                }
                self.tab_request(&target, "session.move_window", payload);
            }
            TabAction::New => {
                let from = self
                    .tab_window(&target)
                    .and_then(|w| w.focused_block_id.clone());
                let layout = self.new_terminal(target.host);
                let payload =
                    json!({"window_label": DEFAULT_TAB_LABEL, "layout": layout, "focus": true});
                self.open_terminal(
                    target.host,
                    "session.new_window",
                    payload,
                    &target.session,
                    from.as_deref(),
                );
            }
            TabAction::CloseOthers => {
                let others: Vec<&String> =
                    order.iter().filter(|id| **id != target.window).collect();
                if others.is_empty() {
                    return;
                }
                let session = self.session_label(target.host, &target.session);
                let (title, which) = match others.len() {
                    1 => (
                        "Close the other tab?".to_owned(),
                        "the other tab".to_owned(),
                    ),
                    n => (
                        format!("Close {n} other tabs?"),
                        format!("the {n} other tabs"),
                    ),
                };
                let description = format!(
                    "Ends every program in {which} of {session} on {host}; {} stays open. Closing the app itself leaves programs running.",
                    target.name
                );
                // The tabs named now are the ones that close, not ones opened meanwhile.
                self.confirm(
                    &title,
                    "session.close_window",
                    json!({"window_ids": others}),
                    &description,
                );
                self.panel.as_mut().unwrap().tab = Some(target);
            }
            TabAction::Close => {
                let title = format!("Close {}?", target.name);
                let description = format!(
                    "Ends every program in this tab on {host}. Closing the app itself leaves programs running."
                );
                self.confirm(
                    &title,
                    "session.close_window",
                    json!({"window_id": target.window}),
                    &description,
                );
                self.panel.as_mut().unwrap().tab = Some(target);
            }
        }
    }

    /// A confirmed Close tab or Close other tabs. The other tabs close one request each;
    /// one that closed since the question was asked is skipped, and nothing closes when the
    /// tab that was to stay is gone.
    pub(super) fn confirmed_tab_close(&mut self, target: &TabTarget, method: &str, payload: Value) {
        let Some(named) = payload.get("window_ids").and_then(Value::as_array) else {
            self.tab_request(target, method, payload);
            return;
        };
        let open = self.tab_order(target).map(|(_, order)| order);
        for id in named.iter().filter_map(Value::as_str) {
            if open.as_ref().is_some_and(|o| o.iter().any(|w| w == id)) {
                self.tab_request(target, method, json!({"window_id": id}));
            }
        }
    }
}
