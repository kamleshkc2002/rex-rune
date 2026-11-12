//! Files dropped on a pane, or pasted, go to its terminal's machine through Rex, which types
//! their paths into it.
use super::workspace::Workspace;
use rex_rune::{
    connection::{Command, HostCommand},
    upload::File,
};
use rune::Tx;
use std::path::PathBuf;

impl Workspace {
    /// Text into a captured pane as a paste, the event clipboard text is sent as: for Insert
    /// path and Go to directory. Sent once, with no key after it; a pane that is gone or
    /// not live is told so. Returns whether the existing input path accepted the paste,
    /// not a receipt from the remote terminal.
    pub fn paste_text(&mut self, target: &super::panel::PaneTarget, text: String) -> bool {
        if !self.target_exists(target) {
            self.inform("That pane is no longer available.");
            return false;
        }
        let key = (self.hosts[target.host].id, target.block.clone());
        if !self.hosts[target.host].model.connected
            || !self.terminals.get(&key).is_some_and(|t| t.status().live)
        {
            self.inform("Nothing was typed: the terminal is not live.");
            return false;
        }
        self.terminals
            .get_mut(&key)
            .unwrap()
            .try_send(Command::Paste(text))
    }

    /// Upload `files` to `block` of the active session. The block is fixed now; later focus
    /// changes don't move where the paths are typed.
    pub fn upload(&mut self, block: &str, files: Vec<File>) {
        let Some(session) = self.model().selected.clone() else {
            return;
        };
        let live = self
            .terminals
            .get(&self.term_key(block))
            .is_some_and(|t| t.status().live);
        if !live {
            self.inform("Files can be uploaded only to a live terminal.");
            return;
        }
        self.send(
            self.active,
            HostCommand::Upload {
                session,
                block: block.to_owned(),
                files,
            },
        );
    }

    pub fn drop_files(&mut self, block: &str, paths: &[PathBuf]) {
        if self.panel.is_some() || paths.is_empty() {
            return;
        }
        if self.focused_block().as_deref() != Some(block)
            && let Some(t) = self.terminals.get(&self.term_key(block))
        {
            let _ = t.connection.commands.focus();
        }
        self.upload(block, paths.iter().cloned().map(File::path).collect());
    }

    /// Ctrl+Shift+V: copied files upload, else text pastes, else an image uploads. The read
    /// answers later and goes only to the pane that asked, if it is still focused by then.
    pub fn paste_clipboard(&mut self, tx: &mut Tx) {
        if let Some(block) = self.focused_block() {
            self.paste_into(&block, true, tx);
        }
    }

    /// Paste into `block` of the session on screen. A keyboard paste follows the focus
    /// (`while_focused`); a pane menu's names its pane, focused or not, and reaches it as long
    /// as it is still a pane of that session. Either way nothing is pasted under a dialog.
    pub fn paste_into(&mut self, block: &str, while_focused: bool, tx: &mut Tx) {
        let asked = self.term_key(block);
        if !self.terminals.get(&asked).is_some_and(|t| t.status().live) {
            return;
        }
        let session = self.model().selected.clone();
        tx.read_clipboard(move |s: &mut Workspace, content, _| {
            let still = if while_focused {
                s.focused_block().map(|block| s.term_key(&block)).as_ref() == Some(&asked)
            } else {
                s.hosts[s.active].id == asked.0
                    && s.model().selected == session
                    && s.target(&asked.1).is_some_and(|t| s.target_exists(&t))
            };
            if !still || s.panel.is_some() {
                return;
            }
            if !content.files().is_empty() {
                let files = content.files().iter().cloned().map(File::path).collect();
                s.upload(&asked.1, files);
            } else if !content.text().is_empty() {
                if let Some(term) = s.terminals.get_mut(&asked) {
                    term.send(Command::Paste(content.text().to_owned()));
                }
            } else if let Some(image) = content.image() {
                s.upload(&asked.1, vec![File::bytes("clipboard.png", image.png())]);
            }
        });
    }
}
