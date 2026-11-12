//! Per-host consent and the deliberately small view-only remote surface.
use super::{
    actions::{self, Remote},
    navigation::Visit,
    workspace::{HostId, Workspace},
};
use rex_rune::connection::{
    HostCommand,
    perform::{MAX_WAIT, Outcome, Pending},
};
use rune::Tx;
use std::time::{Duration, Instant};

pub struct Navigation {
    host: HostId,
    pending: Pending,
    id: u64,
    deadline: Instant,
}

const TYPING: Duration = Duration::from_millis(1500);

impl Workspace {
    pub fn toggle_remote_control(&mut self, host: usize) {
        self.settings.hosts[host].remote_control = !self.settings.hosts[host].remote_control;
        self.save();
    }

    fn remote_policy(
        &self,
        host: usize,
        pending: &Pending,
        field_focused: bool,
    ) -> Result<(), Outcome> {
        let request = &pending.request;
        let Some(spec) = actions::find(&request.action) else {
            return Err(Outcome::error(
                "not_supported",
                "This remote action is not supported by Rex Rune; nothing was done.",
            ));
        };
        let Remote::View { navigation } = spec.remote() else {
            let Remote::Never(message) = spec.remote() else {
                unreachable!()
            };
            return Err(Outcome::error(
                "not_supported",
                format!(
                    "{} is not available remotely. {message} Nothing was done.",
                    request.action
                ),
            ));
        };
        if !self.settings.hosts[host].remote_control {
            return Err(Outcome::error(
                "refused",
                "Remote control is off for this host in Rex Rune. The person at that computer can turn it on in Settings → Remote Hosts. Nothing was done.",
            ));
        }
        if !spec.accepts(&request.args) {
            return Err(Outcome::error(
                "bad_request",
                format!(
                    "Invalid arguments for {}; nothing was done.",
                    request.action
                ),
            ));
        }
        let selecting = request.action == actions::SELECT_SESSION;
        let model = &self.hosts[host].model;
        let refused = if self.panel.is_some() {
            Some("A dialog or popup is open")
        } else if field_focused {
            Some("A text field has focus")
        } else if self.bindings.sequence.depth > 0 {
            Some("A key sequence is pending")
        } else if !selecting && self.active != host {
            Some("This host is not on screen")
        } else if request.session_id.as_ref().is_some_and(|id| {
            if selecting {
                request.args["session_id"].as_str() != Some(id.as_str())
            } else {
                model.selected.as_ref() != Some(id)
            }
        }) {
            Some("The requested session is not on screen")
        } else if request
            .block_id
            .as_deref()
            .is_some_and(|id| model.focused() != Some(id))
        {
            Some("The requested pane is not focused")
        } else if navigation && self.last_keypress.is_some_and(|at| at.elapsed() < TYPING) {
            Some("The person is typing; try again after a pause")
        } else if self.navigation.pending.is_some() || self.remote_navigation.is_some() {
            Some("Another navigation is still in progress")
        } else {
            None
        };
        if let Some(reason) = refused {
            return Err(Outcome::error(
                "refused",
                format!("{reason}; nothing was done."),
            ));
        }
        if !self.hosts[host].model.connected || pending.received.elapsed() >= MAX_WAIT {
            return Err(Outcome::error(
                "internal",
                "Rex Rune did not act in time or the host disconnected; nothing was done.",
            ));
        }
        if selecting {
            self.session_visit(host, &request.args)?;
            return Ok(());
        }
        if request.action != "client.sidebar.toggle" && self.model().window().is_none() {
            return Err(Outcome::error(
                "internal",
                "No tab is on screen; nothing was done.",
            ));
        }
        if request.action == "client.tab.goto"
            && actions::tab_index(
                &request.args,
                self.model().view().map_or(0, |v| v.windows.len()),
            )
            .is_none()
        {
            return Err(Outcome::error(
                "bad_request",
                "That tab index is out of range; nothing was done.",
            ));
        }
        if request.action == "pane.scroll" && self.active_terminal().is_none() {
            return Err(Outcome::error(
                "internal",
                "No terminal is on screen; nothing was done.",
            ));
        }
        Ok(())
    }

    pub(super) fn remote_perform(&mut self, host: usize, pending: Pending, tx: &mut Tx) {
        // A sync wake may outlive its socket. Do not execute or answer it on the replacement.
        if pending.epoch != self.hosts[host].model.connection_epoch {
            return;
        }
        // Rune supplies Tx focus for input handlers, but not AppHandle::update.
        // Remote requests arrive through the latter, so inspect mounted search editors too.
        let field_focused = tx.focused().is_some()
            || self.model().window().is_some_and(|window| {
                rex_rune::layout::placements(window, None)
                    .0
                    .iter()
                    .any(|p| {
                        self.terminals
                            .get(&self.term_key(&p.block))
                            .and_then(|t| t.search.as_ref())
                            .is_some_and(|edit| edit.focused)
                    })
            });
        let outcome = match self.remote_policy(host, &pending, field_focused) {
            Err(outcome) => {
                if !self.settings.hosts[host].remote_control
                    && matches!(
                        actions::find(&pending.request.action).map(|s| s.remote()),
                        Some(Remote::View { .. })
                    )
                    && self.remote_off_notices.insert(self.hosts[host].id)
                {
                    self.inform(format!(
                        "{} on {} asked for {}. Remote control is off (Settings → Remote Hosts).",
                        pending.request.requester,
                        self.settings.hosts[host].label,
                        pending.request.action
                    ));
                }
                outcome
            }
            Ok(()) => {
                if pending.request.action == actions::SELECT_SESSION {
                    let visit = self
                        .session_visit(host, &pending.request.args)
                        .expect("validated above");
                    if let Some(id) = self.navigate_remote(visit, tx) {
                        self.remote_navigation = Some(Navigation {
                            host: self.hosts[host].id,
                            pending,
                            id,
                            deadline: Instant::now() + MAX_WAIT,
                        });
                        return;
                    }
                    self.remote_reply(
                        host,
                        pending,
                        Outcome::error("internal", "Navigation could not start."),
                    );
                    return;
                }
                if actions::run(self, &pending.request.action, &pending.request.args, tx) {
                    Outcome::Performed
                } else {
                    Outcome::error("internal", "The action could not run; nothing was done.")
                }
            }
        };
        self.remote_reply(host, pending, outcome);
    }

    pub(super) fn session_visit(
        &self,
        host: usize,
        args: &serde_json::Value,
    ) -> Result<Visit, Outcome> {
        let model = &self.hosts[host].model;
        let session = args["session_id"].as_str().unwrap_or_default();
        let window = args["window_id"].as_str();
        if !model.sessions.iter().any(|s| s.session_id == session) {
            return Err(Outcome::error(
                "not_found",
                "That session does not exist on this host.",
            ));
        }
        // For an uncached session the server checks window membership during focus;
        // attachment fills the cache so a failed acknowledgement can name not_found.
        if let (Some(window), Some(view)) = (window, model.views.get(session))
            && !view
                .windows
                .iter()
                .any(|w| w.window_id.as_deref() == Some(window))
        {
            return Err(Outcome::error(
                "not_found",
                "That tab does not exist in the requested session.",
            ));
        }
        Ok(Visit {
            host: self.hosts[host].id,
            session: session.into(),
            window: window.map(str::to_owned),
        })
    }

    /// Success means the worker acknowledged navigation and the target is showing.
    /// A disconnect abandons the call: its id must never reach a replacement socket.
    pub(super) fn poll_remote_navigation(&mut self, now: Instant) -> bool {
        let Some(navigation) = self.remote_navigation.take() else {
            return false;
        };
        let Some(host) = self.host_index(navigation.host) else {
            return true;
        };
        let model = &self.hosts[host].model;
        if !model.connected || model.connection_epoch != navigation.pending.epoch {
            return true;
        }
        let result = model
            .navigation_result
            .filter(|(id, _)| *id == navigation.id);
        let outcome = if let Some((_, success)) = result {
            let args = &navigation.pending.request.args;
            let showing = self.active == host
                && model.selected.as_deref() == args["session_id"].as_str()
                && args["window_id"].as_str().is_none_or(|id| {
                    model.window().and_then(|w| w.window_id.as_deref()) == Some(id)
                });
            if success && showing {
                Some(Outcome::Performed)
            } else if let Err(error) = self.session_visit(host, args) {
                Some(error)
            } else {
                Some(Outcome::error(
                    "internal",
                    "Navigation failed or the view changed before it completed.",
                ))
            }
        } else if now >= navigation.deadline {
            Some(Outcome::error(
                "internal",
                "Rex Rune did not finish navigation in five seconds.",
            ))
        } else if self
            .navigation
            .pending
            .as_ref()
            .is_none_or(|p| p.id != navigation.id)
        {
            Some(Outcome::error("internal", "Navigation was interrupted."))
        } else {
            None
        };
        if let Some(outcome) = outcome {
            self.remote_reply(host, navigation.pending, outcome);
            true
        } else {
            self.remote_navigation = Some(navigation);
            false
        }
    }

    pub(super) fn remote_reply(&mut self, host: usize, pending: Pending, outcome: Outcome) {
        log::info!(
            "Remote control on {}: {} requested {}: {:?}",
            self.settings.hosts[host].label,
            pending.request.requester,
            pending.request.action,
            outcome
        );
        if outcome == Outcome::Performed {
            self.inform(format!(
                "{}: {} performed {}",
                self.settings.hosts[host].label, pending.request.requester, pending.request.action
            ));
        }
        self.send(
            host,
            HostCommand::PerformReply {
                epoch: pending.epoch,
                call_id: pending.request.call_id,
                outcome,
            },
        );
    }
}
