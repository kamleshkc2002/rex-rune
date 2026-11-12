//! What selecting text with the pointer copies, when Settings → Terminal says so: the
//! clipboard (Copy on selection) and the desktop's primary selection, which a middle-click
//! pastes.
//!
//! A selection's text is made by the pane's worker and arrives with a frame, so the copy
//! happens when that frame does, once. Nothing here is input to a program or a request to a
//! server: selecting and copying never claim a pane, and only the paste is sent, as any
//! paste is.
use super::workspace::{HostId, Workspace};
use rex_rune::connection::Command;
use rune::Tx;

impl Workspace {
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
        let config = &self.settings.terminal;
        if config.primary_selection {
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
        if !self.settings.terminal.primary_selection {
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
                || !s.settings.terminal.primary_selection
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
            system::read(move |text| handle.update(move |s, _| arrived(s, text)));
        }
    }

    fn set_primary(&mut self, text: String) {
        #[cfg(test)]
        memory::set(text);
        #[cfg(not(test))]
        {
            let handle = self.handle.clone();
            system::write(text, move |error| {
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

/// The desktop's primary selection, through arboard on a thread of its own: X11's `PRIMARY`,
/// or Wayland's through the compositor's data-control protocol. The thread keeps the one
/// handle for as long as the app runs, because the owner of a selection has to stay around
/// to hand it over.
#[cfg_attr(test, allow(dead_code))]
mod system {
    use arboard::{Clipboard, GetExtLinux, LinuxClipboardKind, SetExtLinux};
    use std::sync::{OnceLock, mpsc};

    enum Request {
        Write(String, Box<dyn FnOnce(String) + Send>),
        Read(Box<dyn FnOnce(Result<String, String>) + Send>),
    }

    fn worker() -> &'static mpsc::Sender<Request> {
        static WORKER: OnceLock<mpsc::Sender<Request>> = OnceLock::new();
        WORKER.get_or_init(|| {
            let (sender, requests) = mpsc::channel();
            std::thread::Builder::new()
                .name("rex-primary".into())
                .spawn(move || serve(requests))
                .expect("primary selection thread");
            sender
        })
    }

    fn serve(requests: mpsc::Receiver<Request>) {
        let mut clipboard: Option<Clipboard> = None;
        for request in requests {
            if clipboard.is_none() {
                clipboard = Clipboard::new().ok();
            }
            let Some(board) = clipboard.as_mut() else {
                let error = "no clipboard connection".to_owned();
                match request {
                    Request::Write(_, failed) => failed(error),
                    Request::Read(then) => then(Err(error)),
                }
                continue;
            };
            match request {
                Request::Write(text, failed) => {
                    if let Err(error) = board
                        .set()
                        .clipboard(LinuxClipboardKind::Primary)
                        .text(text)
                    {
                        failed(error.to_string());
                    }
                }
                Request::Read(then) => then(read_text(board)),
            }
        }
    }

    fn read_text(board: &mut Clipboard) -> Result<String, String> {
        match board.get().clipboard(LinuxClipboardKind::Primary).text() {
            Ok(text) => Ok(text),
            // Nothing selected anywhere, or something that isn't text.
            Err(arboard::Error::ContentNotAvailable) => Ok(String::new()),
            Err(error) => Err(error.to_string()),
        }
    }

    /// Make `text` the primary selection; `failed` hears why if the desktop refuses.
    pub fn write(text: String, failed: impl FnOnce(String) + Send + 'static) {
        let _ = worker().send(Request::Write(text, Box::new(failed)));
    }

    /// Read the primary selection off the calling thread: its text, empty when there is
    /// none, or why it could not be read.
    pub fn read(then: impl FnOnce(Result<String, String>) + Send + 'static) {
        let _ = worker().send(Request::Read(Box::new(then)));
    }

    /// Two processes on one disposable display server (an Xvfb; never the desktop's own),
    /// so the text really crosses it: one owns the primary selection, the other reads it.
    ///
    /// ```sh
    /// Xvfb :97 & export DISPLAY=:97; unset WAYLAND_DISPLAY
    /// REX_TEST_PRIMARY=own cargo test --all-features primary_selection_crosses -- --ignored &
    /// REX_TEST_PRIMARY=read cargo test --all-features primary_selection_crosses -- --ignored
    /// ```
    #[cfg(test)]
    mod tests {
        use super::*;
        use std::time::Duration;

        #[test]
        #[ignore = "needs a disposable display server and a second process"]
        fn primary_selection_crosses_the_display_server() {
            match std::env::var("REX_TEST_PRIMARY").as_deref() {
                Ok("own") => {
                    let (sender, refused) = mpsc::channel();
                    write("from rex".into(), move |error| sender.send(error).unwrap());
                    // The worker drops the callback unused once the write is done.
                    let refused = refused.recv();
                    assert!(refused.is_err(), "{refused:?}");
                    // Stay the owner while the other process asks.
                    std::thread::sleep(Duration::from_secs(6));
                }
                Ok("read") => {
                    let (sender, answer) = mpsc::channel();
                    // The owner may still be starting.
                    for _ in 0..40 {
                        let sender = sender.clone();
                        read(move |text| sender.send(text).unwrap());
                        if answer.recv().unwrap().as_deref() == Ok("from rex") {
                            return;
                        }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    panic!("the other process's primary selection never arrived");
                }
                _ => {}
            }
        }
    }
}
