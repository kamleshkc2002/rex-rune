//! A read-only index of already-known tab activity. No queries or subscriptions.
use super::{
    activity::{TabActivity, blocked_word, program_icon, tab_activity},
    notify::NoteTarget,
    panel::{Entry, Intent, PanelKind},
    theme::{accent, danger, muted, warning},
    workspace::Workspace,
};
use rex_rune::connection::{
    HostModel,
    activity::{ProgramState, time::Age},
};
use rune::{Icon, Tx};
use std::time::{Duration, Instant};

pub const ACTION: &str = "client.activity";
pub const COVERAGE: &str = "Reading activity…";
pub const INCOMPLETE: &str = "Some activity could not be read.";

pub struct Presentation {
    pub icon: &'static Icon,
    pub place: String,
    pub tone: u32,
    pub elapsed: Option<Elapsed>,
}

pub struct TabTime {
    epoch: u64,
    state: ProgramState,
    age: Option<Age>,
}

pub struct Elapsed {
    pub text: String,
    pub label: String,
}

pub(super) fn elapsed(age: Age, state: ProgramState, now: Instant) -> Elapsed {
    let seconds = age.elapsed(now).as_secs();
    let (short, long) = if seconds < 60 {
        ("now".into(), "less than a minute".into())
    } else {
        let (n, unit, suffix) = if seconds < 3600 {
            (seconds / 60, "minute", "m")
        } else if seconds < 86400 {
            (seconds / 3600, "hour", "h")
        } else {
            (seconds / 86400, "day", "d")
        };
        (
            format!("{n}{suffix}"),
            format!("{n} {unit}{}", if n == 1 { "" } else { "s" }),
        )
    };
    let state = match state {
        ProgramState::Blocked => "waiting",
        ProgramState::Error => "in error",
        ProgramState::Working => "working",
        ProgramState::Done => "done",
        ProgramState::Idle => "idle",
    };
    if age.reported {
        Elapsed {
            text: format!("reported {short}"),
            label: format!("Known {state} from a report {long} ago; state may have begun earlier"),
        }
    } else {
        Elapsed {
            text: short,
            label: format!("Observed {state} for {long}"),
        }
    }
}

/// Oldest known current record in the winning group, or none when any record in that
/// group has no known age: an exact-looking time from one pane must not stand for a tab
/// whose other pane has been in the state for an unknown, perhaps longer, time. A
/// completion latch without a matching Done record has no timestamp. Never let a progress
/// change choose a new age.
fn tab_age(
    model: &HostModel,
    session: &str,
    id: &str,
    state: ProgramState,
    now: Instant,
) -> Option<Age> {
    let window = model
        .views
        .get(session)?
        .windows
        .iter()
        .find(|w| w.window_id.as_deref() == Some(id))?;
    window
        .layers
        .iter()
        .flat_map(|l| &l.blocks)
        .filter_map(|b| model.activity.get(b.block_id.as_deref()?))
        .flat_map(|r| r.values())
        .filter(|r| r.state == state)
        .map(|r| r.age)
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .max_by_key(|a| a.elapsed(now))
}

impl Workspace {
    /// Follow the folded tab state even while the popup is closed. Otherwise an old
    /// Working pane re-exposed by a blocked sibling finishing would look as though the
    /// tab had been in Working throughout. This cache has no notification policy.
    pub(super) fn track_activity_times_at(&mut self, index: usize, now: Instant) {
        let host = &self.hosts[index];
        let id = host.id;
        let model = &host.model;
        let mut next = Vec::new();
        if model.connected {
            for (session, view) in &model.views {
                for window in &view.windows {
                    let Some(window_id) = window.window_id.as_deref() else {
                        continue;
                    };
                    let state = tab_activity(model, window).state;
                    let candidate = tab_age(model, session, window_id, state, now);
                    let key = (id, window_id.to_owned());
                    let previous = self
                        .activity_times
                        .get(&key)
                        .filter(|t| t.epoch == model.activity_epoch);
                    let age = match previous {
                        // The tab's own clock, unless its records say something younger:
                        // model wakes coalesce, so a tab can leave a state and return to
                        // it between two of them, and the records then know better. With
                        // no known record age there is nothing to show.
                        Some(p) if p.state == state => match (p.age, candidate) {
                            (Some(kept), Some(record)) => {
                                Some(if record.elapsed(now) < kept.elapsed(now) {
                                    record
                                } else {
                                    kept
                                })
                            }
                            (_, record) => record,
                        },
                        Some(_) => candidate.map(|a| if a.reported { a } else { Age::live(now) }),
                        None => candidate,
                    };
                    next.push((
                        key,
                        TabTime {
                            epoch: model.activity_epoch,
                            state,
                            age,
                        },
                    ));
                }
            }
        }
        self.activity_times
            .retain(|(h, _), _| *h != id && self.hosts.iter().any(|host| host.id == *h));
        self.activity_times.extend(next);
    }

    /// Returns a redraw and its next wake delay. No activity tracking, selection reveal,
    /// host command or scroll command belongs on this path.
    pub(super) fn refresh_activity_at(&mut self, now: Instant) -> (bool, Option<Duration>) {
        let Some(panel) = self
            .panel
            .as_mut()
            .filter(|p| matches!(p.kind, PanelKind::Activity))
        else {
            return (false, None);
        };
        let deadline = panel
            .activity_refresh
            .get_or_insert(now + Duration::from_secs(60));
        let changed = now >= *deadline;
        if changed {
            *deadline = now + Duration::from_secs(60);
        }
        (changed, Some(deadline.saturating_duration_since(now)))
    }

    pub fn show_activity(&mut self) {
        self.open_panel(
            PanelKind::Activity,
            "Activity",
            &[("", "Filter activity…", "")],
        );
    }

    /// Opened from the toolbar chip, which counts tabs other than the one on screen: start
    /// on the first row that is one of them, not on the tab already in front.
    pub fn show_activity_elsewhere(&mut self) {
        self.show_activity();
        let elsewhere = self.entries().iter().position(|e| match &e.action {
            Intent::ActivityTab(t) => !self.host_index(t.host).is_some_and(|host| {
                t.window
                    .as_deref()
                    .is_some_and(|window| self.tab_in_front(host, &t.session, window))
            }),
            _ => false,
        });
        if let (Some(panel), Some(index)) = (&mut self.panel, elsewhere) {
            panel.selected = index;
            panel.reveal = true;
        }
    }

    /// The list's one quiet line: what it cannot know. A host that is not connected (offline,
    /// still connecting or reconnecting) is named first, so an empty list beside a host that
    /// is down does not read as all clear.
    pub fn activity_coverage(&self) -> String {
        let mut parts = Vec::new();
        match self.hosts.iter().filter(|h| !h.model.connected).count() {
            0 => {}
            1 => parts.push("1 host is not connected.".into()),
            n => parts.push(format!("{n} hosts are not connected.")),
        }
        let connected = || self.hosts.iter().filter(|h| h.model.connected);
        if connected().any(|h| h.model.activity_reading) {
            parts.push(COVERAGE.into());
        } else if connected().any(|h| h.model.activity_incomplete) {
            parts.push(INCOMPLETE.into());
        }
        parts.join(" ")
    }

    /// The list and its toolbar entry point share this cached, connected, unmuted inventory.
    /// Server session/window order is preserved; nothing is fetched or marked seen.
    pub fn activity_tabs(&self) -> impl Iterator<Item = (usize, &str, &str, TabActivity)> {
        self.hosts
            .iter()
            .enumerate()
            .filter(|(_, h)| h.model.connected)
            .flat_map(move |(index, host)| {
                host.model.sessions.iter().flat_map(move |session| {
                    host.model
                        .views
                        .get(&session.session_id)
                        .into_iter()
                        .flat_map(move |view| {
                            view.windows.iter().filter_map(move |window| {
                                let id = window.window_id.as_deref()?;
                                (!self.tab_muted(index, id)).then(|| {
                                    (
                                        index,
                                        session.session_id.as_str(),
                                        id,
                                        tab_activity(&host.model, window),
                                    )
                                })
                            })
                        })
                })
            })
    }

    pub fn activity_entries(&self, lower: &str) -> Vec<Entry> {
        self.activity_entries_at(lower, Instant::now())
    }

    pub(super) fn activity_entries_at(&self, lower: &str, now: Instant) -> Vec<Entry> {
        let mut entries = Vec::new();
        for (index, session, id, activity) in self.activity_tabs() {
            let (rank, group, word, tone) = match activity.state {
                ProgramState::Blocked => (0, "Needs you", blocked_word(activity.kind), warning()),
                ProgramState::Error => (1, "Error", "Error", danger()),
                ProgramState::Working => (3, "Working", "Working", muted()),
                _ if self.unseen_on(index, id) => (2, "Done", "Done", accent()),
                _ => continue,
            };
            let (title, place) = self.describe_tab(index, session, Some(id));
            // The state is searchable too: "error", "needs", "working".
            if !format!("{title} {place} {word} {group}")
                .to_lowercase()
                .contains(lower)
            {
                continue;
            }
            let mut entry = Entry::new(
                title,
                word,
                group,
                Intent::ActivityTab(NoteTarget {
                    host: self.hosts[index].id,
                    session: session.to_owned(),
                    window: Some(id.to_owned()),
                }),
            );
            entry.activity = Some(Presentation {
                icon: program_icon(activity.program.as_deref()),
                place,
                tone,
                elapsed: {
                    let state = if rank == 2 {
                        ProgramState::Done
                    } else {
                        activity.state
                    };
                    self.activity_times
                        .get(&(self.hosts[index].id, id.to_owned()))
                        .filter(|t| {
                            t.epoch == self.hosts[index].model.activity_epoch && t.state == state
                        })
                        .map(|t| t.age)
                        .unwrap_or_else(|| {
                            tab_age(&self.hosts[index].model, session, id, state, now)
                        })
                        .map(|age| elapsed(age, state, now))
                },
            });
            entries.push((rank, entry));
        }
        entries.sort_by_key(|(rank, _)| *rank);
        entries.into_iter().map(|(_, entry)| entry).collect()
    }

    pub fn activity_selection(&self) -> Option<NoteTarget> {
        let panel = self
            .panel
            .as_ref()
            .filter(|p| matches!(p.kind, PanelKind::Activity))?;
        match &self.entries().get(panel.selected)?.action {
            Intent::ActivityTab(target) => Some(target.clone()),
            _ => None,
        }
    }

    pub fn restore_activity_selection(&mut self, target: Option<NoteTarget>) {
        if !self
            .panel
            .as_ref()
            .is_some_and(|p| matches!(p.kind, PanelKind::Activity))
        {
            return;
        }
        let entries = self.entries();
        let panel = self.panel.as_mut().unwrap();
        let selected = entries
            .iter()
            .position(|e| matches!(&e.action, Intent::ActivityTab(t) if Some(t) == target.as_ref()))
            .unwrap_or(panel.selected.min(entries.len().saturating_sub(1)));
        panel.reveal |= selected != panel.selected;
        panel.selected = selected;
    }

    pub fn choose_activity(&mut self, target: NoteTarget, tx: &mut Tx) {
        let valid = self.host_index(target.host).is_some_and(|index| {
            let model = &self.hosts[index].model;
            model.connected
                && model
                    .views
                    .get(&target.session)
                    .is_some_and(|v| v.windows.iter().any(|w| w.window_id == target.window))
        });
        if valid {
            // The notification path owns navigation and notification closing, without
            // asking the desktop to raise a window that is already in front. Done/Bell
            // clear only after the selected view is acknowledged, through track_activity.
            self.show_noted_tab(target, tx);
        } else {
            self.inform("That tab is no longer available.");
        }
    }
}
