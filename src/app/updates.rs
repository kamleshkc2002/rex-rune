//! Settings → Updates and the non-blocking in-app update notice.
//! Checks signed release metadata over HTTPS without self-updating or downloading binaries.
use super::{
    panel::{Intent, PanelKind, SettingsSection},
    settings_kit::{self, button, card, page, row_frame, setting, unavailable, value, words},
    theme::*,
    widgets::{Pill, glyph, pill},
    workspace::Workspace,
};
use rex_linux::update::{
    AvailableUpdate, CHECK_INTERVAL_SECS, CheckConfig, CheckOutcome, InstallChannel, STARTUP_DELAY,
    UpdateState, check_for_updates, now_epoch_secs,
};
use rune::{Cursor, El, Tx, col, el, row, text};
use rune_kit::Switch;
use std::time::{Duration, Instant};

pub struct Updates {
    pub channel: InstallChannel,
    pub state: UpdateState,
    pub config: CheckConfig,
    pub checking: bool,
    pub last_error: Option<String>,
    pub next_check: Option<Instant>,
    pub env_opt_out: bool,
}

impl Default for Updates {
    fn default() -> Self {
        #[cfg(test)]
        {
            use std::sync::atomic::{AtomicUsize, Ordering};
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let config = CheckConfig {
                state_path: std::env::temp_dir().join(format!(
                    "rex-linux-test-updates-{}-{id}.json",
                    std::process::id()
                )),
                ..CheckConfig::default()
            };
            Self {
                channel: InstallChannel::Source,
                state: UpdateState::default(),
                config,
                checking: false,
                last_error: None,
                next_check: None,
                env_opt_out: false,
            }
        }
        #[cfg(not(test))]
        {
            let channel = InstallChannel::detect();
            let config = CheckConfig::default();
            let state = UpdateState::load(&config.state_path);
            let env_opt_out = rex_linux::update::env_disabled();
            Self {
                channel,
                state,
                config,
                checking: false,
                last_error: None,
                next_check: None,
                env_opt_out,
            }
        }
    }
}

impl Updates {
    /// Available update for the running version, if one has been verified.
    pub fn available(&self) -> Option<AvailableUpdate> {
        self.state.available_update(&self.config.current_version)
    }

    /// Available update that has not been dismissed for this version, when update checks are not opted out.
    pub fn visible_notice(&self, setting_enabled: bool) -> Option<AvailableUpdate> {
        if !setting_enabled || self.env_opt_out {
            return None;
        }
        self.available().filter(|u| !u.dismissed)
    }

    /// Whether automatic background checks are enabled by channel, environment, and preference.
    pub fn auto_check_enabled(&self, setting_enabled: bool) -> bool {
        setting_enabled && !self.env_opt_out && self.channel.checks_by_default()
    }

    /// Schedule or clear the next background check deadline from `last_checked`.
    pub fn schedule(&mut self, setting_enabled: bool, now: Instant) {
        if !self.auto_check_enabled(setting_enabled) {
            self.next_check = None;
            return;
        }
        let now_secs = now_epoch_secs();
        let wait = match self.state.last_checked {
            Some(last) => {
                let elapsed = now_secs.saturating_sub(last);
                if elapsed >= CHECK_INTERVAL_SECS {
                    STARTUP_DELAY
                } else {
                    Duration::from_secs(CHECK_INTERVAL_SECS - elapsed).max(STARTUP_DELAY)
                }
            }
            None => STARTUP_DELAY,
        };
        self.next_check = Some(now + wait);
    }

    /// Back off for a full check interval (~24h) after a failed check so offline/429/rollback errors never retry every 30s.
    pub fn schedule_after_failure(&mut self, setting_enabled: bool, now: Instant) {
        if !self.auto_check_enabled(setting_enabled) {
            self.next_check = None;
            return;
        }
        self.next_check = Some(now + Duration::from_secs(CHECK_INTERVAL_SECS));
    }
}

pub fn section(s: &Workspace) -> El {
    let k = kit();
    let u = &s.updates;
    let current = &rex_linux::format_version(&u.config.current_version);
    let channel_label = u.channel.label();
    let auto_allowed = u.channel.checks_by_default() && !u.env_opt_out;
    let switch_on = s.settings.updates.check && auto_allowed;

    let toggle_detail = if u.env_opt_out {
        "Turned off by REX_LINUX_NO_UPDATE_CHECK=1 in the environment."
    } else if !u.channel.checks_by_default() {
        "Source builds never check on their own; use Check now."
    } else {
        "Reads signed release metadata once a day. Never downloads or installs."
    };

    let status_detail = if u.checking {
        "Checking signed release metadata…".to_owned()
    } else if let Some(err) = &u.last_error {
        format!("Check failed: {err}")
    } else if let Some(avail) = u.available() {
        match &avail.unsupported_floor {
            Some(floor) => format!(
                "Rex Linux {} is available (requires upgrading to {floor}+ first).",
                rex_linux::format_version(&avail.version)
            ),
            None => format!(
                "Rex Linux {} is available.",
                rex_linux::format_version(&avail.version)
            ),
        }
    } else if u.state.last_checked.is_some() {
        format!("Rex Linux {current} is up to date.")
    } else {
        "Not checked yet.".into()
    };

    let check_button = if u.checking {
        unavailable("updates-checking", "Checking…", "")
    } else {
        button("updates-check-now", "Check now", Pill::Neutral, |s, _| {
            s.check_updates_now();
        })
    };

    let mut sections = vec![settings_kit::section(
        "",
        [card([
            setting(
                "Installed version",
                &channel_label,
                value(current.clone()).key("updates-current-version"),
            ),
            setting(
                "Automatic update checks",
                toggle_detail,
                Switch::new(switch_on)
                    .label("Check for updates automatically")
                    .hide_label()
                    .disabled(!auto_allowed)
                    .on_toggle(|s: &mut Workspace| {
                        s.set_update_check(!s.settings.updates.check);
                    })
                    .build(k)
                    .key("updates-auto-switch"),
            ),
            setting("Release status", &status_detail, check_button).key("updates-status-row"),
        ])],
    )];

    if let Some(avail) = u.available() {
        let guidance = u.channel.guidance(&avail.version);
        let notes_url = avail.notes_url.clone();
        let version_for_dismiss = avail.version.clone();
        let mut actions = row().gap(8.).center_y().child(button(
            "updates-whats-new",
            "What's new",
            Pill::Primary,
            move |s, _| s.open_link(notes_url.clone()),
        ));
        if !avail.dismissed {
            actions = actions.child(button(
                "updates-dismiss",
                "Dismiss for this version",
                Pill::Neutral,
                move |s, _| s.dismiss_update_version(&version_for_dismiss),
            ));
        }
        let detail = match &avail.unsupported_floor {
            Some(floor) => format!(
                "{} Manual upgrade required: this release supports direct updates from {floor} and newer.",
                guidance.detail
            ),
            None => guidance.detail.clone(),
        };
        let mut rows = vec![
            setting(
                &format!("Rex Linux {}", rex_linux::format_version(&avail.version)),
                &detail,
                actions,
            )
            .key("updates-available-row"),
        ];
        if let Some(cmd) = guidance.command {
            let cmd_copy = cmd.clone();
            rows.push(
                row_frame(true)
                    .key("updates-command-row")
                    .child(
                        words("Upgrade command", "").child(
                            text(cmd)
                                .font(mono(11.5))
                                .color(rgb(muted()))
                                .wrap()
                                .selectable(true),
                        ),
                    )
                    .child(button(
                        "updates-copy-command",
                        "Copy command",
                        Pill::Neutral,
                        move |s, tx| {
                            tx.copy(cmd_copy.clone());
                            s.inform("Copied upgrade command to clipboard.");
                        },
                    )),
            );
        }
        sections.push(settings_kit::section("Available update", [card(rows)]));
    }

    page(sections)
}

/// Non-blocking toast card for an available update, with What's new, Updates…, and Dismiss.
pub fn toast_card(s: &Workspace) -> Option<El> {
    let avail = s.updates.visible_notice(s.settings.updates.check)?;
    let guidance = s.updates.channel.guidance(&avail.version);
    let summary = match &avail.unsupported_floor {
        Some(floor) => format!(
            "Rex Linux {} is available (manual upgrade via {floor}+ required).",
            rex_linux::format_version(&avail.version)
        ),
        None => guidance.summary,
    };
    let notes_url = avail.notes_url;
    let version = avail.version;
    let version_dismiss = version.clone();

    Some(
        glass(
            col()
                .key("toast-update")
                .role("status")
                .w_full()
                .pl(14.)
                .pr(8.)
                .py(10.)
                .gap(8.),
            12.,
            false,
        )
        .fill(rgb(popover()).alpha(0.96))
        .transition(pop_in())
        .preserve_focus_on_press(true)
        .child(
            row()
                .w_full()
                .gap(10.)
                .center_y()
                .child(
                    el().size(7., 7.)
                        .radius(3.5)
                        .fill(rgb(accent()))
                        .box_shadow(shadow(0., 0., 8., rgb(accent()).alpha(0.6))),
                )
                .child(
                    text(summary)
                        .font(ui(12.5))
                        .color(rgb(ink()))
                        .wrap()
                        .grow(1.)
                        .shrink(),
                )
                .child(
                    row()
                        .key("toast-update-dismiss")
                        .role("button")
                        .label("Dismiss for this version")
                        .size(24., 24.)
                        .center()
                        .radius(8.)
                        .hover(white(0.08))
                        .cursor(Cursor::Pointer)
                        .preserve_focus_on_press(true)
                        .on_press(move |s: &mut Workspace| {
                            s.dismiss_update_version(&version_dismiss);
                        })
                        .child(glyph(&CLOSE, 13., rgb(muted()))),
                ),
        )
        .child(
            row()
                .pl(17.)
                .gap(8.)
                .center_y()
                .child(
                    pill(
                        "toast-update-whats-new",
                        "What's new",
                        Pill::Primary,
                        24.,
                        move |s: &mut Workspace, _: &mut Tx| {
                            s.open_link(notes_url.clone());
                        },
                    )
                    .preserve_focus_on_press(true),
                )
                .child(
                    pill(
                        "toast-update-open",
                        "Updates…",
                        Pill::Neutral,
                        24.,
                        |s: &mut Workspace, tx: &mut Tx| {
                            s.perform(Intent::Updates, tx);
                        },
                    )
                    .preserve_focus_on_press(true),
                ),
        ),
    )
}

impl Workspace {
    pub fn init_updates(&mut self) {
        self.updates
            .schedule(self.settings.updates.check, Instant::now());
    }

    pub fn set_update_check(&mut self, enabled: bool) {
        self.settings.updates.check = enabled;
        self.save();
        self.updates.schedule(enabled, Instant::now());
    }

    pub fn dismiss_update_version(&mut self, version: &str) {
        self.updates.state.dismissed_version = Some(version.to_owned());
        self.updates.state.announced_version = Some(version.to_owned());
        let _ = self.updates.state.save(&self.updates.config.state_path);
    }

    pub fn open_updates_settings(&mut self) {
        self.settings_nav.opened();
        self.bindings.sequence.clear();
        self.reload_shortcuts();
        let back = self.panel.take();
        self.settings_section = SettingsSection::Updates;
        self.panel = Some(super::panel::Panel::new(
            PanelKind::Settings,
            "Settings",
            &[],
        ));
        self.panel.as_mut().unwrap().target = back.and_then(|p| p.target);
    }

    /// Explicit user-requested check (from Settings → Updates or Commands).
    pub fn check_updates_now(&mut self) {
        self.spawn_update_check(true);
    }

    /// Poll background update timer in `app::poll`.
    pub fn poll_updates(&mut self, now: Instant, tx: &mut Tx) -> bool {
        if self.updates.checking {
            return false;
        }
        let Some(deadline) = self.updates.next_check else {
            return false;
        };
        if now >= deadline {
            self.updates.next_check = Some(now + Duration::from_secs(CHECK_INTERVAL_SECS));
            if self.updates.auto_check_enabled(self.settings.updates.check) {
                self.spawn_update_check(false);
            }
            return true;
        }
        tx.wake_after(deadline - now);
        false
    }

    fn spawn_update_check(&mut self, manual: bool) {
        if self.updates.checking {
            return;
        }
        self.updates.checking = true;
        if manual {
            self.updates.last_error = None;
        }
        let config = self.updates.config.clone();
        let handle = self.handle.clone();
        let _ = std::thread::Builder::new()
            .name("rex-update-check".into())
            .spawn(move || {
                let result = check_for_updates(&config, manual);
                handle.update(move |s, _| {
                    s.finish_update_check(manual, result);
                });
            });
    }

    pub fn finish_update_check(
        &mut self,
        manual: bool,
        result: anyhow::Result<(CheckOutcome, UpdateState)>,
    ) {
        self.updates.checking = false;
        let now = Instant::now();
        match result {
            Ok((outcome, mut state)) => {
                self.updates.last_error = None;
                if state.dismissed_version.is_none()
                    && let Some(dismissed) = &self.updates.state.dismissed_version
                {
                    state.dismissed_version = Some(dismissed.clone());
                }
                match &outcome {
                    CheckOutcome::UpdateAvailable { version, .. }
                    | CheckOutcome::UnsupportedFloor {
                        latest: version, ..
                    } => {
                        if manual && state.dismissed_version.as_deref() == Some(version.as_str()) {
                            // An explicit Check now resurfaces a previously dismissed notice.
                            state.dismissed_version = None;
                        }
                        if state.announced_version.as_deref() != Some(version.as_str()) {
                            state.announced_version = Some(version.clone());
                        }
                        let _ = state.save(&self.updates.config.state_path);
                    }
                    CheckOutcome::UpToDate { version } => {
                        if manual {
                            self.inform(format!("Rex Linux {version} is up to date."));
                        }
                    }
                }
                self.updates.state = state;
                self.updates.schedule(self.settings.updates.check, now);
            }
            Err(error) => {
                // Background checks fail silently: no toast, no modal, no crash.
                if manual {
                    let msg = format!("{error:#}");
                    self.updates.last_error = Some(msg.clone());
                    self.notify(format!("Update check failed: {msg}"));
                }
                self.updates
                    .schedule_after_failure(self.settings.updates.check, now);
            }
        }
    }
}
