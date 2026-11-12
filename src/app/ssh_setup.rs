//! Explicit personal-server setup, separate from automatic attachment and reconnect.
use super::{panel::PanelKind, workspace::Workspace};
use rex_rune::{
    connection::HostCommand,
    ssh::{Target, setup},
};
use rune::Tx;

#[derive(Default)]
pub struct Setup {
    pub running: Option<String>,
    pub attachment: std::collections::HashMap<String, setup::Attachment>,
    /// One-shot starts headless tests would have run.
    #[cfg(test)]
    pub ran: Vec<String>,
}

impl Workspace {
    pub fn can_start_ssh(&self, host: usize) -> bool {
        self.settings
            .hosts
            .get(host)
            .zip(self.hosts.get(host))
            .is_some_and(|(saved, state)| {
                !state.model.connected
                    && state.model.ssh_server_missing
                    && Target::parse(&saved.endpoint)
                        .ok()
                        .flatten()
                        .is_some_and(|t| t.socket.is_none())
            })
    }

    pub fn preview_ssh_start(&mut self, host: usize) {
        if !self.can_start_ssh(host) {
            return;
        }
        let endpoint = self.settings.hosts[host].endpoint.clone();
        let label = self.settings.hosts[host].label.clone();
        self.open_panel(
            PanelKind::StartSsh { endpoint },
            &format!("Start a personal server on {label}?"),
            &[],
        );
    }

    pub fn start_ssh(&mut self, endpoint: String) {
        if self.ssh_setup.running.is_some()
            || self.ssh_setup.attachment.contains_key(&endpoint)
            || !self.panel.as_ref().is_some_and(
                |p| matches!(&p.kind, PanelKind::StartSsh { endpoint: e } if e == &endpoint),
            )
        {
            return;
        }
        // The preview captures the endpoint. An edit/removal or successful attachment must
        // not turn a stale confirmation into a start on a different host.
        if !self
            .settings
            .hosts
            .iter()
            .enumerate()
            .any(|(i, h)| h.endpoint == endpoint && self.can_start_ssh(i))
        {
            if let Some(panel) = &mut self.panel {
                panel.error = Some("This host changed or already connected. Close this dialog and check its connection.".into());
            }
            return;
        }
        self.ssh_setup.running = Some(endpoint.clone());
        if let Some(panel) = &mut self.panel {
            panel.error = None;
        }
        #[cfg(test)]
        self.ssh_setup.ran.push(endpoint);
        #[cfg(not(test))]
        self.spawn_ssh_start(endpoint);
    }

    #[cfg(not(test))]
    fn spawn_ssh_start(&mut self, endpoint: String) {
        let handle = self.handle.clone();
        let task_endpoint = endpoint.clone();
        let spawned = std::thread::Builder::new()
            .name("rex-ssh-setup".into())
            .spawn(move || {
                let result = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| setup::Failure::from(anyhow::Error::from(e)))
                    .and_then(|runtime| runtime.block_on(setup::start_personal(&task_endpoint)));
                handle.update(move |s, tx| s.ssh_start_finished(&task_endpoint, result, tx));
            });
        if let Err(error) = spawned {
            self.ssh_setup.running = None;
            if let Some(panel) = &mut self.panel {
                panel.error = Some(format!("Couldn't start setup: {error}"));
            }
        }
    }

    /// One read-only lookup, only while this saved host still needs a connection.
    fn reconnect_ssh_if_needed(&mut self, endpoint: &str) -> &'static str {
        let Some(host) = self
            .settings
            .hosts
            .iter()
            .position(|h| h.endpoint == endpoint)
        else {
            return "The host is no longer saved; no connection was requested.";
        };
        if self.hosts.get(host).is_none_or(|h| h.model.connected) {
            return "No new connection was requested; the host is already connected or unavailable.";
        }
        self.send(host, HostCommand::Reconnect);
        "Checking the connection through SSH without starting a server."
    }

    pub fn save_ssh_socket(&mut self, endpoint: &str, tx: &mut Tx) {
        let Some(setup::Attachment::Review {
            endpoint: Some(proposed),
            ..
        }) = self.ssh_setup.attachment.get(endpoint).cloned()
        else {
            return;
        };
        let Some(host) = self
            .settings
            .hosts
            .iter()
            .position(|h| h.endpoint == endpoint)
        else {
            return;
        };
        if self.hosts.get(host).is_none_or(|h| h.model.connected) {
            return;
        }
        let saved = self.settings.hosts[host].clone();
        self.close_panel(tx);
        self.edit_host(host, saved.label, proposed, saved.shell);
        self.ssh_setup.attachment.remove(endpoint);
        self.inform(
            "Saved the reported socket endpoint. Connecting through SSH without starting a server.",
        );
    }

    pub fn ssh_start_finished(
        &mut self,
        endpoint: &str,
        result: Result<setup::Attachment, setup::Failure>,
        tx: &mut Tx,
    ) {
        if self.ssh_setup.running.as_deref() != Some(endpoint) {
            return;
        }
        self.ssh_setup.running = None;
        let showing = self.panel.as_ref().is_some_and(
            |p| matches!(&p.kind, PanelKind::StartSsh { endpoint: e } if e == endpoint),
        );
        match result {
            Ok(setup::Attachment::Automatic) => {
                if showing {
                    self.close_panel(tx);
                }
                let connection = self.reconnect_ssh_if_needed(endpoint);
                self.inform(format!(
                    "Personal server answered on {endpoint}. {connection} No session was created."
                ));
            }
            Ok(review @ setup::Attachment::Review { .. }) => {
                if let setup::Attachment::Review {
                    reason,
                    endpoint: proposed,
                } = &review
                    && !showing
                {
                    let proposal = proposed
                        .as_ref()
                        .map(|p| format!(" Save {p} in Edit host."))
                        .unwrap_or_default();
                    self.notify(format!("{reason}{proposal}"));
                }
                self.ssh_setup.attachment.insert(endpoint.into(), review);
            }
            Err(error) => {
                let connection = if error.may_have_started {
                    self.reconnect_ssh_if_needed(endpoint)
                } else {
                    "Preflight failed; no server start was requested."
                };
                let message = format!(
                    "{error}\n{connection} No automatic setup retry. Check the host before choosing Start personal server again."
                );
                if showing {
                    self.panel.as_mut().unwrap().error = Some(message);
                } else {
                    self.notify(message);
                }
            }
        }
    }
}

pub fn dialog(s: &Workspace, endpoint: &str) -> rune::El {
    use super::{
        theme::*,
        widgets::{Pill, pill},
    };
    use rune::{col, row, text};
    let panel = s.panel.as_ref().expect("setup panel");
    let busy = s.ssh_setup.running.is_some();
    let review = s.ssh_setup.attachment.get(endpoint);
    let target = Target::parse(endpoint).ok().flatten();
    let command = target
        .as_ref()
        .and_then(|t| setup::command_preview(t).ok())
        .unwrap_or_default();
    let mut body = col().gap(12.)
        .child(text(panel.title.clone()).font(strong(17.)).color(rgb(ink())).wrap())
        .child(text(format!("{endpoint}\n\nUses your SSH configuration and the Rex executable already installed on both machines (preview.14 or newer). Starts a personal server as the SSH login user only if none is running. No packages, system services or sessions are created. The server may create its own user config/state files and keeps running when this app closes.\n\nThis is one attempt, not permission to auto-start on reconnect. SSH must already work without a password prompt."))
            .font(ui(12.)).color(rgb(muted())).wrap())
        .child(text(format!("On this machine: {command}")).font(ui(12.).mono()).color(rgb(ink())).wrap())
        .child(text("On host, via your SSH: rex version --json, then rex server start --json (chosen by Rex)")
            .font(ui(12.).mono()).color(rgb(ink())).wrap());
    if let Some(running) = &s.ssh_setup.running {
        body = body.child(
            text(format!(
                "Starting {running}… Closing this dialog does not cancel the requested start."
            ))
            .font(ui(12.))
            .color(rgb(muted()))
            .wrap(),
        );
    }
    if let Some(error) = &panel.error {
        body = body.child(
            col()
                .max_h(140.)
                .scroll(rune::ScrollSpec {
                    stick_bottom: false,
                })
                .child(
                    text(error.clone())
                        .font(ui(12.))
                        .color(rgb(danger()))
                        .wrap(),
                ),
        );
    }
    if let Some(setup::Attachment::Review {
        reason,
        endpoint: proposed,
    }) = review
    {
        body = body.child(text(reason.clone()).font(ui(12.)).color(rgb(ink())).wrap());
        if let Some(proposed) = proposed {
            body = body.child(
                text(proposed.clone())
                    .font(ui(12.).mono())
                    .color(rgb(ink()))
                    .wrap(),
            );
            let endpoint = endpoint.to_owned();
            body = body.child(
                pill(
                    "save-ssh-socket",
                    "Save socket endpoint",
                    Pill::Primary,
                    32.,
                    move |s, tx| s.save_ssh_socket(&endpoint, tx),
                )
                .tab_index(3),
            );
        }
    }
    let start = pill(
        "confirm-ssh-start",
        if panel.error.is_some() {
            "Start personal server again"
        } else {
            "Start personal server"
        },
        Pill::Primary,
        32.,
        |s, tx| s.submit(tx),
    )
    .tab_index(2);
    let start = if busy || review.is_some() {
        start.opacity(0.45).inert()
    } else {
        start
    };
    body.child(
        row()
            .gap(8.)
            .justify_end()
            .child(
                pill(
                    "cancel-ssh-start",
                    if busy { "Close" } else { "Not now" },
                    Pill::Neutral,
                    32.,
                    |s, tx| s.cancel_panel(tx),
                )
                .default_button()
                .tab_index(1),
            )
            .child(start),
    )
}
