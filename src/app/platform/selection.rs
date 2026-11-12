//! The primary selection: what selecting text sets and a middle-click pastes, on desktops
//! that have one.
//!
//! X11 has `PRIMARY` and Wayland the compositor's data-control protocol; arboard reaches
//! both. macOS and Windows have only the clipboard, so there [`AVAILABLE`] is false, the
//! setting is not offered and nothing is read or written. Headless tests keep a selection
//! in memory on every platform (`select_copy::memory`).

/// Whether this platform has a primary selection to set and paste.
pub const AVAILABLE: bool = cfg!(any(test, all(unix, not(target_os = "macos"))));

// Tests use the in-memory selection instead.
#[cfg_attr(test, allow(unused_imports))]
pub use system::{read, write};

/// Nothing to own and nothing to read.
#[cfg(not(all(unix, not(target_os = "macos"))))]
#[cfg_attr(test, allow(dead_code))]
mod system {
    pub fn write(_text: String, _failed: impl FnOnce(String) + Send + 'static) {}

    pub fn read(then: impl FnOnce(Result<String, String>) + Send + 'static) {
        then(Ok(String::new()));
    }
}

/// Through arboard on a thread of its own. The thread keeps the one handle for as long as
/// the app runs, because the owner of a selection has to stay around to hand it over.
#[cfg(all(unix, not(target_os = "macos")))]
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
