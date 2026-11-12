//! What selecting text with the pointer copies, when Settings → Terminal says so: the
//! clipboard (Copy on selection) and the desktop's primary selection, which a middle-click
//! pastes.
//!
//! A selection's text is made by the pane's worker and arrives with a frame, so the copy
//! happens when that frame does, once. Nothing here is input to a program or a request to a
//! server: selecting and copying never claim a pane, and only the paste is sent, as any
//! paste is.
use super::platform::selection;
use super::workspace::{HostId, Workspace};
use rex_rune::connection::Command;
use rune::Tx;

impl Workspace {
    /// Whether selecting sets the primary selection and a middle-click pastes it: the
    /// setting, on a desktop that has one.
    pub fn primary_selection(&self) -> bool {
        selection::AVAILABLE && self.settings.terminal.primary_selection
    }

    /// A pane's worker has a new frame. With it may come the text of a selection the
    /// pointer just finished.
    pub fn terminal_woke(&mut self, key: &(HostId, String), tx: &mut Tx) {
        self.refresh_terminal(key);
        self.copy_selected(key, tx);
    }

    fn copy_selected(&mut self, key: &(HostId, String), tx: &mut Tx) {
        let Some(text) = self
            .terminals
            .get_mut(key)
            .and_then(|term| term.take_selected_text())
        else {
            return;
        };
        if !self.window_focused || self.panel.is_some() || key.0 != self.hosts[self.active].id {
            return;
        }
        if self.primary_selection() {
            self.set_primary(text.clone());
        }
        if self.settings.terminal.copy_on_selection {
            tx.copy(text);
        }
    }

    /// A middle-click on `block` that no program asked for: paste the primary selection
    /// there, if the setting is on. The read answers later and reaches the pane only if it
    /// is still a pane of the session on screen and no dialog has opened since.
    pub fn paste_primary(&mut self, block: &str) {
        if !self.primary_selection() {
            return;
        }
        let asked = self.term_key(block);
        if !self
            .terminals
            .get(&asked)
            .is_some_and(|t| t.status().live && !t.tracking())
        {
            return;
        }
        let session = self.model().selected.clone();
        let arrived = move |s: &mut Workspace, text: Result<String, String>| {
            let text = match text {
                Ok(text) => text,
                Err(error) => return s.primary_unavailable(error),
            };
            let still = s.hosts[s.active].id == asked.0
                && s.model().selected == session
                && s.target(&asked.1).is_some_and(|t| s.target_exists(&t));
            if !still
                || !s.window_focused
                || s.panel.is_some()
                || !s.primary_selection()
                || text.is_empty()
            {
                return;
            }
            if let Some(term) = s.terminals.get_mut(&asked)
                && term.status().live
                && !term.tracking()
            {
                term.send(Command::Paste(text));
            }
        };
        #[cfg(test)]
        arrived(self, Ok(memory::get()));
        #[cfg(not(test))]
        {
            let handle = self.handle.clone();
            selection::read(move |text| handle.update(move |s, _| arrived(s, text)));
        }
    }

    fn set_primary(&mut self, text: String) {
        #[cfg(test)]
        memory::set(text);
        #[cfg(not(test))]
        {
            let handle = self.handle.clone();
            selection::write(text, move |error| {
                handle.update(move |s, _| s.primary_unavailable(error))
            });
        }
    }

    /// Say once per launch that this desktop has no primary selection to use.
    fn primary_unavailable(&mut self, error: String) {
        use std::sync::atomic::{AtomicBool, Ordering};
        static SAID: AtomicBool = AtomicBool::new(false);
        if !SAID.swap(true, Ordering::Relaxed) {
            self.notify(format!(
                "The primary selection isn't available on this desktop: {error}"
            ));
        }
    }
}

/// The primary selection of a headless test: one per test thread.
#[cfg(test)]
pub mod memory {
    use std::cell::RefCell;
    thread_local! {
        static PRIMARY: RefCell<String> = const { RefCell::new(String::new()) };
    }
    pub fn get() -> String {
        PRIMARY.with_borrow(Clone::clone)
    }
    pub fn set(text: String) {
        PRIMARY.set(text);
    }
}
