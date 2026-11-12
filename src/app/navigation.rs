//! Window-local visit history and sidebar-order navigation, adapted from joegoldin.
//! Stable host ids keep removed hosts from retargeting a visit to another server.
use super::workspace::{HostId, Workspace};
use rex_linux::connection::HostCommand;
use rune::Tx;
use std::time::{Duration, Instant};

const SETTLE: Duration = Duration::from_millis(150);
const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Visit {
    pub host: HostId,
    pub session: String,
    pub window: Option<String>,
}

pub struct Pending {
    pub visit: Visit,
    pub id: u64,
    pub deadline: Instant,
    pub dispatch_at: Option<Instant>,
    /// The exact history occurrence to restore, remapped together with the cursor.
    origin: usize,
}

#[derive(Default)]
pub struct History {
    entries: Vec<Visit>,
    cursor: usize,
    pub pending: Option<Pending>,
    serial: u64,
}
impl History {
    fn observe(&mut self, visit: Visit) {
        if let Some(target) = &self.pending
            && (target.visit.host != visit.host
                || target.visit.session != visit.session
                || target.visit.window.is_some() && target.visit.window != visit.window)
        {
            return;
        }
        self.pending = None;
        if self.entries.get(self.cursor) == Some(&visit) {
            return;
        }
        self.entries.truncate(self.cursor + 1);
        self.entries.push(visit);
        if self.entries.len() > 128 {
            self.entries.remove(0);
        }
        self.cursor = self.entries.len() - 1;
    }

    fn origin(&self) -> usize {
        self.pending.as_ref().map_or(self.cursor, |p| p.origin)
    }

    pub fn cancel(&mut self) {
        if let Some(pending) = self.pending.take() {
            self.cursor = pending.origin;
        }
    }

    fn retain(&mut self, valid: impl Fn(&Visit) -> bool) {
        if self.pending.as_ref().is_some_and(|p| !valid(&p.visit)) {
            self.cancel();
        }
        let old = self.cursor;
        let old_origin = self.origin();
        let mut kept = Vec::new();
        let mut cursor = 0;
        let mut origin = 0;
        for (index, entry) in self.entries.drain(..).enumerate() {
            if valid(&entry) {
                if kept.last() != Some(&entry) {
                    kept.push(entry);
                }
                if index <= old {
                    cursor = kept.len() - 1;
                }
                if index <= old_origin {
                    origin = kept.len() - 1;
                }
            }
        }
        self.entries = kept;
        self.cursor = cursor;
        if let Some(pending) = &mut self.pending {
            pending.origin = origin;
        }
    }

    fn step(&mut self, direction: isize) -> Option<Visit> {
        let next = self.cursor.checked_add_signed(direction)?;
        let visit = self.entries.get(next)?.clone();
        self.cursor = next;
        Some(visit)
    }
}

impl Workspace {
    fn current_visit(&self) -> Option<Visit> {
        let host = self.hosts.get(self.active)?;
        let session = host.model.selected.clone()?;
        host.model.view()?;
        Some(Visit {
            host: host.id,
            session,
            window: host.model.window().and_then(|w| w.window_id.clone()),
        })
    }

    pub fn observe_navigation(&mut self) {
        let hosts = &self.hosts;
        // Sessions establish validity even when reconnect empties the view cache.
        // A loaded view also lets us prune closed windows and dedupe their neighbours.
        self.navigation.retain(|v| {
            hosts.iter().find(|h| h.id == v.host).is_some_and(|h| {
                h.model.sessions.iter().any(|s| s.session_id == v.session)
                    && v.window.as_ref().is_none_or(|window| {
                        h.model.views.get(&v.session).is_none_or(|view| {
                            view.windows
                                .iter()
                                .any(|w| w.window_id.as_ref() == Some(window))
                        })
                    })
            })
        });
        if let Some(pending) = &self.navigation.pending {
            let result = hosts
                .iter()
                .find(|h| h.id == pending.visit.host)
                .and_then(|h| h.model.navigation_result);
            match result {
                Some((id, false)) if id == pending.id => self.cancel_navigation(),
                // The worker refreshed the view before reporting completion. Even if
                // focus moved meanwhile, this request is finished, not waiting to time out.
                Some((id, true)) if id == pending.id => self.navigation.pending = None,
                // Neither an unrelated notice nor a stale view acknowledges our request.
                _ => return,
            }
        }
        if let Some(visit) = self.current_visit() {
            self.navigation.observe(visit);
        }
    }

    /// Poll just the two deadlines; no per-frame server requests or background prefetch.
    pub fn poll_navigation(&mut self, now: Instant, tx: &mut Tx) -> bool {
        let Some(pending) = &self.navigation.pending else {
            return false;
        };
        if now >= pending.deadline {
            self.cancel_navigation();
            self.update_title(tx);
            self.inform("Navigation timed out. Try again.");
            self.observe_navigation();
            return true;
        }
        if let Some(at) = pending.dispatch_at {
            if now >= at {
                self.dispatch_navigation(tx);
                return true;
            }
            tx.wake_after(at - now);
        } else {
            tx.wake_after(pending.deadline - now);
        }
        false
    }

    fn cancel_navigation(&mut self) {
        // A failed cross-host replay must return to its origin, not record the target
        // host's previously selected tab and truncate Forward as if we visited it.
        let origin = self
            .navigation
            .entries
            .get(self.navigation.origin())
            .cloned();
        self.navigation.cancel();
        if let Some(host) = origin.and_then(|v| self.host_index(v.host))
            && host != self.active
        {
            self.active = host;
            self.update_visibility();
            self.save();
        }
    }

    pub fn select_visit(&mut self, host: usize, session: String, tx: &mut Tx) {
        // Selecting a session preserves its server-side current window; a cached active
        // window may be stale because another client has changed it.
        self.observe_navigation();
        self.navigate_to(
            Visit {
                host: self.hosts[host].id,
                session,
                window: None,
            },
            tx,
        );
    }

    pub(super) fn navigate_to(&mut self, visit: Visit, tx: &mut Tx) {
        let origin = self.navigation.origin();
        self.begin_navigation(visit, origin, false, TIMEOUT, tx);
    }

    pub(super) fn navigate_remote(&mut self, visit: Visit, tx: &mut Tx) -> Option<u64> {
        self.observe_navigation();
        let origin = self.navigation.origin();
        self.begin_navigation(
            visit,
            origin,
            false,
            rex_linux::connection::perform::MAX_WAIT,
            tx,
        );
        tx.wake_after(rex_linux::connection::perform::MAX_WAIT);
        self.navigation.pending.as_ref().map(|p| p.id)
    }

    fn begin_navigation(
        &mut self,
        visit: Visit,
        origin: usize,
        defer: bool,
        timeout: Duration,
        tx: &mut Tx,
    ) {
        self.navigation.serial += 1;
        let now = Instant::now();
        self.navigation.pending = Some(Pending {
            visit,
            id: self.navigation.serial,
            deadline: now + timeout,
            dispatch_at: defer.then_some(now + SETTLE),
            origin,
        });
        if defer {
            tx.wake_after(SETTLE);
        } else {
            self.dispatch_navigation(tx);
        }
    }

    fn dispatch_navigation(&mut self, tx: &mut Tx) {
        let Some(pending) = self.navigation.pending.as_mut() else {
            return;
        };
        pending.dispatch_at = None;
        let visit = pending.visit.clone();
        let id = pending.id;
        let deadline = pending.deadline;
        let Some(host) = self.host_index(visit.host) else {
            self.navigation.cancel();
            return;
        };
        self.active = host;
        self.drag = None;
        self.settings.hosts[host].collapsed = false;
        self.update_visibility();
        if !self.hosts[host].model.connected {
            // An explicit offline selection is a preference for the next bootstrap,
            // not a focus request to reject and bounce back from. Preserve Select's
            // reconnect behaviour; don't queue an old window focus for replay.
            self.navigation.cancel();
            self.send(host, HostCommand::Select(visit.session));
            self.save();
            self.update_title(tx);
            return;
        }
        if self.hosts[host]
            .connection
            .commands
            .send(HostCommand::Navigate {
                id,
                deadline,
                session: visit.session,
                window: visit.window,
            })
            .is_err()
        {
            self.cancel_navigation();
            self.notify("Host worker stopped. Restart Rex to reconnect.");
        }
        self.save();
        self.update_title(tx);
        tx.wake_after(TIMEOUT);
    }

    pub fn navigate_history(&mut self, direction: isize, tx: &mut Tx) {
        if self.panel.is_some() {
            return;
        }
        self.observe_navigation();
        let origin = self.navigation.origin();
        let current = self
            .navigation
            .pending
            .as_ref()
            .map(|p| p.visit.clone())
            .or_else(|| self.current_visit());
        let cursor = self.navigation.cursor;
        while let Some(visit) = self.navigation.step(direction) {
            let valid = self
                .hosts
                .iter()
                .find(|h| h.id == visit.host)
                .is_some_and(|h| {
                    h.model.connected
                        && visit.window.as_ref().is_none_or(|window| {
                            h.model.views.get(&visit.session).is_none_or(|view| {
                                view.windows
                                    .iter()
                                    .any(|w| w.window_id.as_ref() == Some(window))
                            })
                        })
                });
            if valid && Some(&visit) != current.as_ref() {
                self.begin_navigation(visit, origin, false, TIMEOUT, tx);
                return;
            }
        }
        self.navigation.cursor = cursor;
    }

    /// The sessions `cycle_session` moves through: every connected host's, in sidebar order.
    fn session_stops(&self) -> Vec<Visit> {
        self.hosts
            .iter()
            .filter(|host| host.model.connected)
            .flat_map(|host| {
                host.model.sessions.iter().map(|session| Visit {
                    host: host.id,
                    session: session.session_id.clone(),
                    window: None,
                })
            })
            .collect()
    }

    /// How many sessions Next session and Previous session can reach.
    pub(super) fn session_stop_count(&self) -> usize {
        self.session_stops().len()
    }

    /// Show the session after or before the one on screen, in sidebar order across connected
    /// hosts, wrapping. A session keeps the tab it last showed, as selecting it does.
    pub fn cycle_session(&mut self, direction: isize, tx: &mut Tx) {
        self.observe_navigation();
        let current = self
            .navigation
            .pending
            .as_ref()
            .map(|p| (p.visit.host, p.visit.session.clone()))
            .or_else(|| {
                let host = self.hosts.get(self.active)?;
                Some((host.id, host.model.selected.clone()?))
            });
        let stops = self.session_stops();
        if stops.is_empty() {
            return;
        }
        let index = current.and_then(|(host, session)| {
            stops
                .iter()
                .position(|v| v.host == host && v.session == session)
        });
        let next = index.map_or_else(
            || if direction < 0 { stops.len() - 1 } else { 0 },
            |index| (index as isize + direction).rem_euclid(stops.len() as isize) as usize,
        );
        // The only session there is: nowhere to go.
        if index == Some(next) {
            return;
        }
        self.navigate_to(stops[next].clone(), tx);
    }

    /// Traverse sidebar order using the pending target until the server acknowledges it.
    pub fn cycle_tab(&mut self, direction: isize, tx: &mut Tx) {
        self.observe_navigation();
        let current = self
            .navigation
            .pending
            .as_ref()
            .map(|p| p.visit.clone())
            .or_else(|| self.current_visit())
            .map(|mut v| {
                if v.window.is_none() {
                    v.window = self
                        .hosts
                        .iter()
                        .find(|h| h.id == v.host)
                        .and_then(|h| h.model.views.get(&v.session))
                        .and_then(|view| {
                            view.active_window_id
                                .clone()
                                .or_else(|| view.windows.first()?.window_id.clone())
                        });
                }
                v
            });
        let mut tabs = Vec::new();
        let listing = self.sidebar_tabs();
        for (index, host) in self
            .hosts
            .iter()
            .enumerate()
            .filter(|(_, h)| h.model.connected)
        {
            // A layout read in the background for activity is not a session the person
            // opened: it stays one stop unless its tabs are listed in the sidebar, so
            // stepping never changes the active tab of a session they have not been in.
            let listed = listing && !self.settings.hosts[index].collapsed;
            for session in &host.model.sessions {
                let stepped = listed || !host.model.background_views.contains(&session.session_id);
                if let Some(view) = host
                    .model
                    .views
                    .get(&session.session_id)
                    .filter(|_| stepped)
                {
                    tabs.extend(view.windows.iter().filter_map(|window| {
                        Some(Visit {
                            host: host.id,
                            session: session.session_id.clone(),
                            window: Some(window.window_id.clone()?),
                        })
                    }));
                } else {
                    // Unopened sessions are one stop; landing there loads or shows the view.
                    tabs.push(Visit {
                        host: host.id,
                        session: session.session_id.clone(),
                        window: None,
                    });
                }
            }
        }
        if tabs.is_empty() {
            return;
        }
        let index = current.as_ref().and_then(|current| {
            tabs.iter().position(|v| {
                v == current
                    || v.host == current.host && v.session == current.session && v.window.is_none()
            })
        });
        let next = index.map_or_else(
            || if direction < 0 { tabs.len() - 1 } else { 0 },
            |index| (index as isize + direction).rem_euclid(tabs.len() as isize) as usize,
        );
        let visit = tabs[next].clone();
        // Same-session focus remains immediate. Crossing sessions/hosts settles for
        // 150 ms so key repeat never attaches every session passed along the way.
        let defer = self
            .navigation
            .pending
            .as_ref()
            .is_some_and(|p| p.dispatch_at.is_some())
            || visit.host != self.hosts[self.active].id
            || self.model().selected.as_ref() != Some(&visit.session);
        let origin = self.navigation.origin();
        self.begin_navigation(visit, origin, defer, TIMEOUT, tx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn visit(id: &str) -> Visit {
        Visit {
            host: HostId(0),
            session: "s".into(),
            window: Some(id.into()),
        }
    }
    #[test]
    fn history_branches_without_readding_replayed_visits() {
        let mut h = History::default();
        for id in ["a", "a", "b", "c"] {
            h.observe(visit(id));
        }
        assert_eq!(h.entries.len(), 3);
        assert_eq!(h.step(-1), Some(visit("b")));
        h.pending = Some(Pending {
            visit: visit("b"),
            id: 1,
            deadline: Instant::now() + TIMEOUT,
            dispatch_at: None,
            origin: 2,
        });
        h.observe(visit("c"));
        assert!(h.pending.is_some());
        h.observe(visit("b"));
        assert_eq!(h.entries.len(), 3);
        assert_eq!(h.step(1), Some(visit("c")));
        h.step(-1);
        h.observe(visit("d"));
        assert!(h.step(1).is_none());
        assert_eq!(h.step(-1), Some(visit("b")));
        assert_eq!(h.step(-1), Some(visit("a")));
        assert!(h.step(-1).is_none());
    }
    #[test]
    fn removed_targets_are_skipped_and_history_is_bounded() {
        let mut h = History::default();
        for id in ["a", "closed", "b"] {
            h.observe(visit(id));
        }
        h.retain(|v| v.window.as_deref() != Some("closed"));
        assert_eq!(h.step(-1), Some(visit("a")));
        assert_eq!(h.step(1), Some(visit("b")));
        for i in 0..200 {
            h.observe(visit(&i.to_string()));
        }
        assert_eq!(h.entries.len(), 128);
        assert_eq!(h.cursor, 127);
    }
    #[test]
    fn retain_remaps_the_pending_origin_as_well_as_the_cursor() {
        for origin in 0..5 {
            for cursor in 0..5 {
                let mut h = History::default();
                for id in ["a", "gone", "a", "b", "c"] {
                    h.observe(visit(id));
                }
                h.cursor = cursor;
                h.pending = Some(Pending {
                    visit: visit("c"),
                    id: 1,
                    deadline: Instant::now() + TIMEOUT,
                    dispatch_at: None,
                    origin,
                });
                h.retain(|v| v.window.as_deref() != Some("gone"));
                assert_eq!(h.cursor, [0, 0, 0, 1, 2][cursor]);
                h.cancel();
                assert_eq!(h.cursor, [0, 0, 0, 1, 2][origin]);
            }
        }
    }

    #[test]
    fn retain_deduplicates_and_remaps_every_cursor_position() {
        for cursor in 0..5 {
            let mut h = History::default();
            for id in ["a", "gone", "a", "b", "c"] {
                h.observe(visit(id));
            }
            h.cursor = cursor;
            h.retain(|v| v.window.as_deref() != Some("gone"));
            assert_eq!(h.entries, [visit("a"), visit("b"), visit("c")]);
            assert_eq!(h.cursor, [0, 0, 0, 1, 2][cursor]);
            h.retain(|_| false);
            assert_eq!(h.cursor, 0);
            assert!(h.step(-1).is_none());
            h.observe(visit("new"));
            assert_eq!(h.entries, [visit("new")]);
        }
    }
}
