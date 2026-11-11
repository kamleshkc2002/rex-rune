//! The terminal verbs of the pane menu and Commands: Copy, Paste, Select all, Copy pane ID,
//! Clear terminal and Reset terminal, and when each of the menu's rows can run. Each acts on the pane the menu was opened for, which
//! need not be the focused one; the matching `pane.*` actions name the focused pane.
use super::{
    pane_header,
    panel::{PaneAction, PaneTarget},
    term::Term,
    workspace::Workspace,
};
use rex_linux::connection::Command;
use rexrs::api::session::Window;
use rune::Tx;

impl Workspace {
    fn pane_term(&self, target: &PaneTarget) -> Option<&Term> {
        let host = self.hosts.get(target.host)?.id;
        self.terminals.get(&(host, target.block.clone()))
    }

    /// The tab `target` is in, as its session's view has it.
    fn pane_window(&self, target: &PaneTarget) -> Option<&Window> {
        let view = self
            .hosts
            .get(target.host)?
            .model
            .views
            .get(&target.session)?;
        view.windows.iter().find(|window| {
            window
                .layers
                .iter()
                .flat_map(|layer| &layer.blocks)
                .any(|block| block.block_id.as_deref() == Some(&target.block))
        })
    }

    /// Whether a pane menu row can run now: Copy needs a selection, Select all something to
    /// select, Paste, Clear and Reset a live terminal, and Zoom a tab with more than this one
    /// pane (or a zoomed pane to restore), as the header's button does. Every other row
    /// always can.
    pub fn pane_verb_available(&self, target: &PaneTarget, action: PaneAction) -> bool {
        let term = self.pane_term(target);
        match action {
            PaneAction::Copy => term.is_some_and(Term::has_selection),
            PaneAction::SelectAll => term.is_some_and(|t| t.frame.is_some()),
            PaneAction::Paste | PaneAction::Clear | PaneAction::Reset => {
                term.is_some_and(|t| t.status().live)
            }
            PaneAction::Zoom => self.pane_window(target).is_some_and(pane_header::can_zoom),
            _ => true,
        }
    }

    /// Give `target` the keyboard when it is another pane than the focused one, so what is
    /// typed after a Paste or copied after a Select all goes where the menu pointed.
    fn focus_pane(&self, target: &PaneTarget) {
        if (self.active != target.host
            || self.focused_block().as_deref() != Some(target.block.as_str()))
            && let Some(term) = self.pane_term(target)
        {
            let _ = term.connection.commands.focus();
        }
    }

    pub(super) fn pane_verb(&mut self, target: PaneTarget, action: PaneAction, tx: &mut Tx) {
        let key = (self.hosts[target.host].id, target.block.clone());
        match action {
            PaneAction::Copy => {
                let text = self.terminals.get(&key).and_then(Term::copy_text);
                if let Some(text) = text.filter(|t| !t.is_empty()) {
                    tx.copy(text);
                }
            }
            PaneAction::Paste => {
                // The clipboard is read for the session on screen only; a menu is always its.
                if self.active == target.host
                    && self.model().selected.as_ref() == Some(&target.session)
                {
                    self.focus_pane(&target);
                    self.paste_into(&target.block, false, tx);
                }
            }
            PaneAction::SelectAll => {
                self.focus_pane(&target);
                if let Some(term) = self.terminals.get_mut(&key) {
                    term.select_all();
                }
            }
            PaneAction::CopyId => {
                tx.copy(target.block);
                self.inform("Copied pane ID to clipboard.");
            }
            PaneAction::Clear | PaneAction::Reset => {
                let reset = matches!(action, PaneAction::Reset);
                // Sent once. A terminal that is not live is told so instead: nothing waits
                // for it to come back.
                match self.terminals.get_mut(&key).filter(|t| t.status().live) {
                    Some(term) => term.send(if reset {
                        Command::Reset
                    } else {
                        Command::Clear
                    }),
                    None => self.inform(if reset {
                        "Only a live terminal can be reset."
                    } else {
                        "Only a live terminal can be cleared."
                    }),
                }
            }
            _ => {}
        }
    }
}
