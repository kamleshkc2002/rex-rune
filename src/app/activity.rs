//! Program activity per tab: one state folded from every record of every block in a Rex
//! window, the program its pill shows, the word trouble carries, and which changes become
//! desktop notifications.
use super::{
    notify::{Command, Kind, Note, NoteTarget},
    theme::*,
    workspace::{HostId, Workspace},
};
use rex_rune::connection::{
    DEFAULT_TAB_LABEL, HostModel,
    activity::{
        BlockedKind, ProgramState, StatusRecord, TERMINAL as TERMINAL_CREATOR, TerminalProgress,
    },
};
use rexrs::api::session;
use rune::{El, Tx, el, row};
use std::{borrow::Cow, collections::HashMap};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TabActivity {
    pub state: ProgramState,
    pub kind: Option<BlockedKind>,
    /// The deciding record's message, for the tooltip; plain text from the program.
    pub msg: Option<String>,
    /// The deciding record arrived live rather than from a query.
    pub live: bool,
    /// The program behind the deciding record.
    pub source: Option<String>,
    /// The focused block's program: the one reporting, else the foreground process.
    pub program: Option<String>,
    /// The tab's OSC 9;4 progress: an error, else a pause, else progress, across its panes.
    pub progress: Option<TerminalProgress>,
}

fn blocks(window: &session::Window) -> impl Iterator<Item = &str> + Clone {
    window
        .layers
        .iter()
        .flat_map(|l| &l.blocks)
        .filter_map(|b| b.block_id.as_deref())
}

fn focused_block(window: &session::Window) -> Option<&str> {
    window
        .focused_block_id
        .as_deref()
        .filter(|f| blocks(window).any(|b| b == *f))
        .or_else(|| blocks(window).next())
}

/// Whether a tab still has a default label, so it shows its program's title: no label, the
/// label this client gives new tabs, or the one the server makes from the window's ID.
pub fn default_title(window: &session::Window) -> bool {
    let label = window.label.as_deref().filter(|l| !l.is_empty());
    // The server's default uses the last eight ID characters. Match this window's
    // own ID so an explicitly named "Window ..." on another window stays intact.
    let generated = label
        .and_then(|l| l.strip_prefix("Window "))
        .is_some_and(|suffix| {
            suffix.len() == 8
                && window
                    .window_id
                    .as_deref()
                    .is_some_and(|id| id.ends_with(suffix))
        });
    generated || label.is_none_or(|l| l == DEFAULT_TAB_LABEL)
}

/// What a tab is called: its chosen name, or for a default label, its focused program's
/// terminal title (or generated program and directory, when preferred).
pub fn tab_name(model: &HostModel, window: &session::Window, generated: bool) -> String {
    let label = window.label.as_deref().filter(|l| !l.is_empty());
    let title = default_title(window)
        .then(|| focused_block(window).and_then(|b| model.shown_title(b, generated)))
        .flatten();
    title.as_deref().or(label).unwrap_or("Terminal").to_owned()
}

/// What a pane is called: its server label, or while that is still the default its terminal
/// gets (its flavor's last word, such as `shell`), its program's terminal title. Display only:
/// the server label is never changed.
pub fn pane_name(model: &HostModel, block: &session::LayerBlock, generated: bool) -> String {
    let label = block.label.as_deref().filter(|l| !l.is_empty());
    let default = block.creator_name.as_deref() == Some(TERMINAL_CREATOR)
        && label
            .is_none_or(|l| block.flavor.as_deref().and_then(|f| f.rsplit('.').next()) == Some(l));
    let title = default
        .then(|| {
            block
                .block_id
                .as_deref()
                .and_then(|b| model.shown_title(b, generated))
        })
        .flatten();
    title.as_deref().or(label).unwrap_or("Terminal").to_owned()
}

/// A pane's legacy icon source: its deciding status record's app, else foreground process.
/// Kept shared so count one is exactly the old TabActivity.program policy.
fn block_program(model: &HostModel, block: &str) -> Option<String> {
    model
        .activity
        .get(block)
        .and_then(|records| deciding(records.values()))
        .and_then(|r| r.app.clone())
        .or_else(|| model.programs.get(block).cloned())
}

/// Cached programs, focused pane first then stable server layer/block order.
/// Aggregate tab severity never sorts panes. Missing focus falls back to the first block.
pub fn tab_programs(model: &HostModel, window: &session::Window) -> Vec<Option<String>> {
    let focused = focused_block(window);
    focused
        .into_iter()
        .chain(blocks(window).filter(|b| Some(*b) != focused))
        .map(|b| block_program(model, b))
        .collect()
}

/// Fold a tab's records with Blocked > Error > Working > Done > Idle.
pub fn tab_activity(model: &HostModel, window: &session::Window) -> TabActivity {
    fold_activity(model, blocks(window), focused_block(window))
}

/// One pane's activity: the same fold as its tab's, over its own records.
pub fn pane_activity(model: &HostModel, block: &str) -> TabActivity {
    fold_activity(model, std::iter::once(block), Some(block))
}

/// The record that speaks for a set: the highest state, and among equals one that reports
/// progress, so a percentage doesn't depend on record or pane order.
fn deciding<'a>(records: impl Iterator<Item = &'a StatusRecord>) -> Option<&'a StatusRecord> {
    records.max_by_key(|r| (r.state, r.progress.is_some()))
}

fn fold_activity<'a>(
    model: &'a HostModel,
    blocks: impl Iterator<Item = &'a str> + Clone,
    focused: Option<&str>,
) -> TabActivity {
    let top = deciding(
        blocks
            .clone()
            .filter_map(|b| model.activity.get(b))
            .flat_map(|records| records.values()),
    );
    let program = focused.and_then(|b| block_program(model, b));
    // A program's own OSC 9;4 report wins over the percentage in its status record.
    let progress = blocks
        .filter_map(|b| model.progress.get(b).copied())
        .max_by_key(|p| p.rank())
        .or_else(|| {
            top.filter(|r| r.state == ProgramState::Working)
                .and_then(|r| r.progress)
                .map(|n| TerminalProgress::Set(Some(n)))
        });
    TabActivity {
        state: top.map(|r| r.state).unwrap_or_default(),
        kind: top.and_then(|r| r.kind),
        msg: top.and_then(|r| r.msg.clone().or_else(|| r.title.clone())),
        live: top.is_some_and(|r| r.live),
        source: top.and_then(|r| r.app.clone()),
        program,
        progress,
    }
}

/// The icon for a program; shells and anything unknown keep the terminal glyph.
pub fn program_icon(program: Option<&str>) -> &'static rune::Icon {
    match program.map(str::to_ascii_lowercase).as_deref() {
        Some("claude" | "claude-code") => &CLAUDE,
        Some("codex") => &CODEX,
        Some("opencode") => &OPENCODE,
        // OMP is built on Pi and shares its skill.
        Some("pi" | "omp") => &PI,
        Some("nvim" | "vim" | "vi") => &VIM,
        Some("git" | "lazygit" | "tig") => &GIT,
        Some("cargo" | "rustc" | "rustup") => &CRATE,
        Some("ssh" | "mosh") => &KEY,
        _ => &TERMINAL,
    }
}

/// The word a pill shows after its label, with its colour: trouble always, then a bell that
/// rang while the tab was out of sight, then a paused or failed progress report, and Done
/// until the tab is seen. Idle and Working records show nothing: a program's progress while
/// it works is a bar ([`tab_progress`]), not a word.
pub fn tab_word(
    activity: &TabActivity,
    unseen_done: bool,
    unseen_bell: bool,
) -> Option<(Cow<'static, str>, u32)> {
    use TerminalProgress::{Error, Paused};
    match activity.state {
        ProgramState::Blocked => Some((blocked_word(activity.kind).into(), warning())),
        ProgramState::Error => Some(("Error".into(), danger())),
        _ if unseen_bell => Some(("Bell".into(), warning())),
        _ => match activity.progress {
            Some(Error(_)) => Some(("Error".into(), danger())),
            Some(Paused(Some(n))) => Some((format!("Paused {n}%").into(), warning())),
            Some(Paused(None)) => Some(("Paused".into(), warning())),
            _ if unseen_done => Some(("Done".into(), accent())),
            _ => None,
        },
    }
}

/// The bar a pill draws under its label for a program's progress report: how far along, when
/// the program said, and its colour (`ACCENT` while it works, `WARNING` paused, `DANGER`
/// failed). None without a report, or while OSC 7501 trouble has the pill's attention.
pub fn tab_progress(activity: &TabActivity) -> Option<(Option<u8>, u32)> {
    use TerminalProgress::{Error, Indeterminate, Paused, Set};
    match activity.state {
        ProgramState::Blocked | ProgramState::Error => None,
        _ => match activity.progress? {
            Set(n) => Some((n, accent())),
            Indeterminate => Some((None, accent())),
            Paused(n) => Some((n, warning())),
            Error(n) => Some((n, danger())),
        },
    }
}

/// A 2 pt progress bar pinned along the bottom of a pill, inset to its text. A known
/// percentage fills that much of a faint track; an unknown one is the whole track in the
/// report's colour, dimmed. It never animates.
pub fn progress_bar(key: String, (percent, tone): (Option<u8>, u32), inset: f32) -> El {
    let bar = row()
        .key(key)
        .pin(Some(inset), None, Some(inset), Some(3.))
        .h(2.)
        .radius(1.);
    match percent {
        Some(percent) => {
            let fraction = (f32::from(percent.min(100)) / 100.).max(0.02);
            bar.fill(white(0.08))
                .child(el().grow(fraction).h_full().radius(1.).fill(rgb(tone)))
                .child(el().grow(1. - fraction))
        }
        None => bar.fill(rgb(tone)).opacity(0.35),
    }
}

pub(super) fn blocked_word(kind: Option<BlockedKind>) -> &'static str {
    match kind {
        Some(BlockedKind::Permission) => "Needs approval",
        Some(BlockedKind::Question) => "Question",
        Some(BlockedKind::Auth) => "Needs sign-in",
        None => "Needs input",
    }
}

fn finished(before: ProgramState, now: ProgramState) -> bool {
    matches!(before, ProgramState::Working | ProgramState::Blocked)
        && matches!(now, ProgramState::Done | ProgramState::Idle)
}

impl Workspace {
    /// The tab this window shows, whether or not the window has focus.
    pub fn tab_in_front(&self, index: usize, session: &str, window: &str) -> bool {
        let model = &self.hosts[index].model;
        index == self.active
            && model.selected.as_deref() == Some(session)
            && model
                .views
                .get(session)
                .is_some_and(|v| v.active_window_id.as_deref() == Some(window))
    }

    /// The tab is in front of the user: this window has focus and shows that tab.
    pub fn tab_seen(&self, index: usize, session: &str, window: &str) -> bool {
        self.window_focused && self.tab_in_front(index, session, window)
    }

    /// Compare each tab of host `index` with its last state: latch unseen Done, clear what
    /// the user now sees, and notify about live changes in tabs they can't see.
    pub fn track_activity(&mut self, index: usize) {
        self.track_activity_times_at(index, std::time::Instant::now());
        let id = self.hosts[index].id;
        let epoch = self.hosts[index].model.activity_epoch;
        if self.activity_epochs.insert(id, epoch) != Some(epoch) {
            self.tab_states.retain(|(host, _), _| *host != id);
        }
        let mut now = HashMap::new();
        let mut notes = Vec::new();
        let mut seen_tabs = Vec::new();
        let model = &self.hosts[index].model;
        for (session, view) in &model.views {
            for window in &view.windows {
                let Some(window_id) = window.window_id.clone() else {
                    continue;
                };
                let activity = tab_activity(model, window);
                let key = (id, window_id.clone());
                let seen = self.tab_seen(index, session, &window_id);
                if seen {
                    seen_tabs.push(NoteTarget {
                        host: id,
                        session: session.clone(),
                        window: Some(window_id.clone()),
                    });
                }
                let before = self.tab_states.get(&key).copied();
                let state = activity.state;
                let done = before.is_some_and(|b| finished(b, state));
                // Queries only restore what was already true; only live reports are news.
                let news = before != Some(state)
                    && (done || (activity.live && state != ProgramState::Idle));
                let shown = if done { ProgramState::Done } else { state };
                let latch = news && !seen && shown == ProgramState::Done;
                now.insert(key.clone(), (state, seen, latch));
                if news && !seen {
                    notes.push((shown, session.clone(), window_id, activity));
                }
            }
        }
        for (key, (state, seen, latch)) in &now {
            if *latch {
                self.unseen_done.insert(key.clone());
            }
            if *seen || matches!(state, ProgramState::Working | ProgramState::Blocked) {
                self.unseen_done.remove(key);
            }
        }
        self.unseen_done
            .retain(|key| key.0 != id || now.contains_key(key));
        // A bell shows until its tab is in front, and goes with its tab.
        self.unseen_bells
            .retain(|key| key.0 != id || now.get(key).is_some_and(|(_, seen, _)| !seen));
        self.tab_states.retain(|(host, _), _| *host != id);
        self.tab_states
            .extend(now.into_iter().map(|(k, (state, ..))| (k, state)));
        for target in seen_tabs {
            self.close_tab_notification(target);
        }
        for (state, session, window, activity) in notes {
            self.notify_state(index, state, session, window, &activity);
        }
    }

    fn notify_state(
        &mut self,
        index: usize,
        state: ProgramState,
        session: String,
        window: String,
        activity: &TabActivity,
    ) {
        let config = &self.settings.notifications;
        let wanted = match state {
            ProgramState::Working => config.working,
            ProgramState::Blocked => config.blocked,
            ProgramState::Error => config.error,
            ProgramState::Done => config.done,
            ProgramState::Idle => false,
        };
        // A muted tab stays quiet whatever its programs do.
        if !wanted || self.tab_muted(index, &window) {
            return;
        }
        let desktop = config.system_notifications;
        // The same news, visibility, mute and event-kind decision feeds both channels, so
        // a sound is heard for exactly what a notification would be posted for. That
        // includes the usual way a program finishes: its status record is removed, which
        // leaves the tab with no record at all (and so nothing marked live).
        if config.play_sounds
            && let Some(sound) = super::sounds::Sound::for_state(state)
        {
            self.sound_request(super::sounds::Request::Event(sound));
        }
        if !desktop {
            return;
        }
        let word = match state {
            ProgramState::Blocked => blocked_word(activity.kind),
            ProgramState::Error => "Error",
            ProgramState::Done => "Done",
            _ => "Working",
        };
        let (tab, place) = self.describe_tab(index, &session, Some(&window));
        let summary = match activity.source.as_ref().or(activity.program.as_ref()) {
            Some(program) => format!("{program} · {word}"),
            None => format!("{tab} · {word}"),
        };
        let body = match &activity.msg {
            Some(msg) => format!("{msg}\n{place}"),
            None => place,
        };
        let host = self.hosts[index].id;
        self.deliver(Note {
            summary,
            body,
            target: Some(NoteTarget {
                host,
                session,
                window: Some(window),
            }),
            kind: Kind::Activity(state),
        });
    }

    /// Pass on what programs asked to tell the user, unless their tab is in front.
    pub fn deliver_program_notices(
        &mut self,
        index: usize,
        notices: Vec<rex_rune::connection::activity::DesktopNotice>,
    ) {
        let config = &self.settings.notifications;
        if !config.system_notifications || !config.program_messages {
            return;
        }
        for notice in notices {
            let window = self.hosts[index]
                .model
                .views
                .get(&notice.session)
                .and_then(|v| {
                    v.windows
                        .iter()
                        .find(|w| blocks(w).any(|b| b == notice.block))
                        .and_then(|w| w.window_id.clone())
                });
            if window.as_deref().is_some_and(|w| {
                self.tab_seen(index, &notice.session, w) || self.tab_muted(index, w)
            }) {
                continue;
            }
            let (tab, place) = self.describe_tab(index, &notice.session, window.as_deref());
            let summary = if notice.title.is_empty() {
                tab
            } else {
                notice.title
            };
            let body = if notice.body.is_empty() {
                place
            } else {
                format!("{}\n{place}", notice.body)
            };
            let host = self.hosts[index].id;
            self.deliver(Note {
                summary,
                body,
                target: Some(NoteTarget {
                    host,
                    session: notice.session,
                    window,
                }),
                kind: Kind::Program,
            });
        }
    }

    /// A tab's label, and "session · host" for a notification body.
    pub(super) fn describe_tab(
        &self,
        index: usize,
        session: &str,
        window: Option<&str>,
    ) -> (String, String) {
        let model = &self.hosts[index].model;
        let view = model.views.get(session);
        let tab = view
            .and_then(|v| v.windows.iter().find(|w| w.window_id.as_deref() == window))
            .map_or_else(
                || "Terminal".into(),
                |w| tab_name(model, w, self.generated_titles()),
            );
        let session = model
            .sessions
            .iter()
            .find(|s| s.session_id == session)
            .map(|s| s.label.clone())
            .or_else(|| view.and_then(|v| v.label.clone()))
            .unwrap_or_else(|| "session".into());
        (
            tab,
            format!("{session} · {}", self.settings.hosts[index].label),
        )
    }

    fn deliver(&mut self, note: Note) {
        if cfg!(test) {
            self.outbox.push(note.clone());
        }
        // Only a tab can be seen; a message whose tab is unknown closes on a click.
        if let Some(target) = note.target.as_ref().filter(|t| t.window.is_some()) {
            self.notified_tabs.insert(target.clone());
        }
        self.notification_command(Command::Send(note));
    }

    /// A notification was clicked: raise the window and show its session and tab.
    pub fn open_note(&mut self, target: NoteTarget, token: Option<String>, tx: &mut Tx) {
        self.handle.activate(token.unwrap_or_default());
        self.show_noted_tab(target, tx);
    }

    /// Show the tab a notification or an Activity row is about, and close what this client
    /// sent about it. The window is already in front for a row; only a click raises it.
    pub fn show_noted_tab(&mut self, target: NoteTarget, tx: &mut Tx) {
        self.close_notifications(target.clone());
        let Some(index) = self.host_index(target.host) else {
            return;
        };
        if let Some(window) = target.window {
            self.select_window(index, target.session, window, tx);
        } else {
            self.select(index, target.session, tx);
        }
    }

    /// Latch the bells programs rang, as (session, block), on tabs the user can't see.
    pub fn ring_bells(&mut self, index: usize, bells: Vec<(String, String)>) {
        let id = self.hosts[index].id;
        for (session, block) in bells {
            let window = self.hosts[index]
                .model
                .views
                .get(&session)
                .and_then(|v| v.windows.iter().find(|w| blocks(w).any(|b| b == block)))
                .and_then(|w| w.window_id.clone());
            if let Some(window) = window
                && !self.tab_seen(index, &session, &window)
            {
                self.unseen_bells.insert((id, window));
            }
        }
    }

    /// Whether `window` on the active host shows "Done" until seen.
    pub fn unseen(&self, window: &str) -> bool {
        self.unseen_on(self.active, window)
    }

    /// The word that speaks for a session whose tabs are out of sight: the most pressing of
    /// its tabs' own words (needs you, then error, then a bell or pause, then an unseen
    /// Done), with the tab it comes from. Tabs that show no word say nothing here either:
    /// muted ones, and all of them when activity in tabs is switched off.
    pub fn session_word(
        &self,
        host: usize,
        session: &str,
    ) -> Option<(Cow<'static, str>, u32, String)> {
        let model = &self.hosts[host].model;
        // What a host said before its connection dropped is not news: the Activity list
        // and the toolbar chip leave such hosts out too.
        if !model.connected {
            return None;
        }
        let generated = self.generated_titles();
        model
            .views
            .get(session)?
            .windows
            .iter()
            .filter_map(|window| {
                let id = window.window_id.as_deref()?;
                if !self.tab_activity_shown(host, id) {
                    return None;
                }
                let activity = tab_activity(model, window);
                let (word, tone) =
                    tab_word(&activity, self.unseen_on(host, id), self.bell_on(host, id))?;
                let rank = match activity.state {
                    ProgramState::Blocked => 0,
                    _ if tone == danger() => 1,
                    _ if tone == warning() => 2,
                    _ => 3,
                };
                Some((rank, word, tone, tab_name(model, window, generated)))
            })
            // The first tab of the most pressing rank, in the server's tab order.
            .min_by_key(|(rank, ..)| *rank)
            .map(|(_, word, tone, tab)| (word, tone, tab))
    }

    pub fn unseen_on(&self, host: usize, window: &str) -> bool {
        self.unseen_done
            .contains(&(self.hosts[host].id, window.to_owned()))
    }

    /// Whether `window` on host `host` rang its bell while out of sight.
    pub fn bell_on(&self, host: usize, window: &str) -> bool {
        self.unseen_bells
            .contains(&(self.hosts[host].id, window.to_owned()))
    }
}

pub type TabStates = HashMap<(HostId, String), ProgramState>;

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(id: &str, label: Option<&str>) -> (HostModel, session::Window) {
        let mut model = HostModel::default();
        model
            .titles
            .insert("block".into(), ("dotfiles".into(), None));
        let window = session::Window {
            window_id: Some(id.into()),
            label: label.map(str::to_owned),
            focused_block_id: Some("block".into()),
            layers: vec![session::Layer {
                blocks: vec![session::LayerBlock {
                    block_id: Some("block".into()),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        (model, window)
    }

    #[test]
    fn generated_window_labels_use_current_terminal_titles() {
        for (id, label, title) in [
            (
                "window:01a1aa89-4009-757a-a7b9-1ce89dcd0a4c",
                "Window 9dcd0a4c",
                "dotfiles",
            ),
            (
                "window:01a1ad21-2a48-7290-8bd0-6c4d624e75b5",
                "Window 624e75b5",
                "bedcontrol",
            ),
        ] {
            let (mut model, window) = tab(id, Some(label));
            model.titles.insert("block".into(), (title.into(), None));
            assert_eq!(tab_name(&model, &window, false), title);
        }
    }

    #[test]
    fn explicit_window_names_keep_their_labels() {
        for label in ["Work", "Window 624e75b5", "Window notes", "Shell commands"] {
            let (model, window) = tab("window:01a1aa89-4009-757a-a7b9-1ce89dcd0a4c", Some(label));
            assert_eq!(tab_name(&model, &window, false), label);
        }
        let (model, mut window) = tab("short", Some("Window 9dcd0a4c"));
        assert_eq!(tab_name(&model, &window, false), "Window 9dcd0a4c");
        window.window_id = None;
        assert_eq!(tab_name(&model, &window, false), "Window 9dcd0a4c");
    }

    #[test]
    fn generated_window_labels_survive_missing_and_stale_titles() {
        let (mut model, window) = tab(
            "window:01a1aa89-4009-757a-a7b9-1ce89dcd0a4c",
            Some("Window 9dcd0a4c"),
        );
        model.titles.clear();
        assert_eq!(tab_name(&model, &window, false), "Window 9dcd0a4c");
        model
            .titles
            .insert("block".into(), ("stale".into(), Some(42)));
        model.foreground.insert("block".into(), vec![99]);
        assert_eq!(tab_name(&model, &window, false), "Window 9dcd0a4c");
    }

    fn pane(label: Option<&str>) -> (HostModel, session::LayerBlock) {
        let mut model = HostModel::default();
        model
            .titles
            .insert("block".into(), ("Claude Code".into(), Some(7)));
        model.foreground.insert("block".into(), vec![7, 1]);
        let block = session::LayerBlock {
            block_id: Some("block".into()),
            label: label.map(str::to_owned),
            creator_name: Some("com.superlogical.terminal".into()),
            flavor: Some("com.superlogical.terminal.shell".into()),
            ..Default::default()
        };
        (model, block)
    }

    #[test]
    fn default_pane_labels_show_the_current_terminal_title() {
        for label in [Some("shell"), None, Some("")] {
            let (model, block) = pane(label);
            assert_eq!(pane_name(&model, &block, false), "Claude Code");
        }
    }

    #[test]
    fn untitled_panes_and_tabs_share_generated_names_but_user_labels_win() {
        // Adapted from ohporter/e94fe1d's untitled pane test; use the M2.10 spelling
        // and preference for both panes and tabs, not a separate pane-only fallback.
        for generated in [false, true] {
            for label in [Some("shell"), None, Some("")] {
                let (mut model, block) = pane(label);
                let (_, window) = tab("window", Some(DEFAULT_TAB_LABEL));
                model.titles.clear();
                model.programs.insert("block".into(), "vim".into());
                model.places.insert(
                    "block".into(),
                    rex_rune::connection::titles::Place {
                        cwd: Some("/home/u/rex".into()),
                        home: Some("/home/u".into()),
                        shell: Some(1),
                    },
                );
                assert_eq!(
                    pane_name(&model, &block, generated),
                    if generated {
                        "vim - ~/rex"
                    } else {
                        label.filter(|l| !l.is_empty()).unwrap_or("Terminal")
                    }
                );
                assert_eq!(
                    tab_name(&model, &window, generated),
                    if generated {
                        "vim - ~/rex"
                    } else {
                        DEFAULT_TAB_LABEL
                    }
                );

                let mut named_block = block.clone();
                named_block.label = Some("build".into());
                let mut named_tab = window.clone();
                named_tab.label = Some("work".into());
                assert_eq!(pane_name(&model, &named_block, generated), "build");
                assert_eq!(tab_name(&model, &named_tab, generated), "work");

                // A program's own title remains authoritative on both surfaces.
                model
                    .titles
                    .insert("block".into(), ("notes - VIM".into(), Some(7)));
                model.foreground.insert("block".into(), vec![7, 1]);
                assert_eq!(pane_name(&model, &block, generated), "notes - VIM");
                assert_eq!(tab_name(&model, &window, generated), "notes - VIM");
            }
        }
    }

    #[test]
    fn renamed_panes_keep_their_labels() {
        for label in ["build", "Shell", "server", "Claude"] {
            let (model, block) = pane(Some(label));
            assert_eq!(pane_name(&model, &block, false), label);
        }
    }

    #[test]
    fn pane_titles_go_stale_with_their_program() {
        let (mut model, block) = pane(Some("shell"));
        model.foreground.insert("block".into(), vec![1]);
        assert_eq!(pane_name(&model, &block, false), "shell");
        model.titles.clear();
        let (_, mut bare) = pane(None);
        bare.flavor = None;
        assert_eq!(pane_name(&model, &bare, false), "Terminal");
    }

    #[test]
    fn only_terminal_blocks_take_titles() {
        let (model, mut block) = pane(Some("shell"));
        block.creator_name = Some("com.example.other".into());
        assert_eq!(pane_name(&model, &block, false), "shell");
    }

    #[test]
    fn shell_and_unlabelled_windows_keep_their_title_fallback() {
        for label in [Some(DEFAULT_TAB_LABEL), None, Some("")] {
            let (model, window) = tab("window", label);
            assert_eq!(tab_name(&model, &window, false), "dotfiles");
        }
    }
}
