//! A read-only index of already-known tab activity. No queries or subscriptions.
use super::{
    activity::{TabActivity, blocked_word, program_icon, tab_activity},
    notify::NoteTarget,
    panel::{Entry, Intent, PanelKind},
    theme::{accent, danger, muted, warning},
    workspace::Workspace,
};
use rex_linux::connection::activity::ProgramState;
use rune::{Icon, Tx};

pub const ACTION: &str = "client.activity";
pub const COVERAGE: &str = "Tabs in sessions not opened since connecting may be missing.";

pub struct Presentation {
    pub icon: &'static Icon,
    pub place: String,
    pub tone: u32,
}

impl Workspace {
    pub fn show_activity(&mut self) {
        self.open_panel(
            PanelKind::Activity,
            "Activity",
            &[("", "Filter activity…", "")],
        );
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
            if !format!("{title} {place}").to_lowercase().contains(lower) {
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
