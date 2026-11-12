//! Desktop notifications through the freedesktop notification service, on a worker thread
//! so D-Bus never blocks the UI. Clicking one raises the window (with the activation token
//! the server sends just before the click, so Wayland lets it take focus) and shows its tab.
use super::workspace::{HostId, Workspace};
use futures_util::StreamExt;
use rune::AppHandle;
use std::collections::{HashMap, VecDeque};
use zbus::{Connection, Proxy, Variant};

const DEST: &str = "org.freedesktop.Notifications";
const PATH: &str = "/org/freedesktop/Notifications";
/// Clicks are remembered for this many recent notifications.
const REMEMBERED: usize = 64;
/// The picture each notification shows. GNOME draws only a small, plain app icon without one.
const IMAGE: &[u8] = include_bytes!("../../resources/icons/128x128/apps/dev.shuv.rex-linux.png");

/// `IMAGE` as the spec's `image-data` hint, `(iiibiiay)`: width, height, row stride, alpha,
/// bits per sample, channels and RGBA bytes. Sent with every notification, so it needs no
/// installed icon file.
fn image_data() -> Option<Variant<'static>> {
    let image = image::load_from_memory_with_format(IMAGE, image::ImageFormat::Png)
        .ok()?
        .into_rgba8();
    let (width, height) = image.dimensions();
    let (width, height) = (width as i32, height as i32);
    Some(Variant::from((
        width,
        height,
        width * 4,
        true,
        8i32,
        4i32,
        image.into_raw(),
    )))
}

#[derive(Clone, Debug, PartialEq)]
pub struct NoteTarget {
    pub host: HostId,
    pub session: String,
    pub window: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub summary: String,
    pub body: String,
    pub target: NoteTarget,
}

pub struct Notifier {
    notes: flume::Sender<Note>,
}

impl Notifier {
    pub fn spawn(handle: AppHandle<Workspace>) -> Self {
        let (notes, receiver) = flume::unbounded();
        let _ = std::thread::Builder::new()
            .name("rex-notifications".into())
            .spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    return;
                };
                if let Err(error) = runtime.block_on(run(receiver, handle)) {
                    eprintln!("rex-linux: desktop notifications unavailable: {error}");
                }
            });
        Self { notes }
    }

    pub fn send(&self, note: Note) {
        let _ = self.notes.send(note);
    }
}

/// The window and its notification worker are already gone after a failed re-exec.
/// Send one critical, persistent notification before exiting (also for desktop launches).
pub fn notify_restart_failure(message: &str) -> anyhow::Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            let conn = Connection::session().await?;
            let proxy = Proxy::new(&conn, DEST, PATH, DEST).await?;
            let hints = HashMap::from([
                ("desktop-entry", Variant::from("dev.shuv.rex-linux")),
                ("urgency", Variant::from(2u8)),
            ]);
            let _: u32 = proxy
                .call(
                    "Notify",
                    &(
                        "Rex",
                        0u32,
                        "dev.shuv.rex-linux",
                        "Rex restart failed",
                        message,
                        Vec::<String>::new(),
                        hints,
                        0i32,
                    ),
                )
                .await?;
            Ok::<_, zbus::Error>(())
        })
        .await??;
        Ok(())
    })
}

async fn run(notes: flume::Receiver<Note>, handle: AppHandle<Workspace>) -> zbus::Result<()> {
    let conn = Connection::session().await?;
    let proxy = Proxy::new(&conn, DEST, PATH, DEST).await?;
    // One stream keeps ActivationToken ahead of the ActionInvoked it belongs to.
    let mut signals = proxy.receive_all_signals().await?;
    let mut targets: HashMap<u32, NoteTarget> = HashMap::new();
    let mut order: VecDeque<u32> = VecDeque::new();
    let mut tokens: HashMap<u32, String> = HashMap::new();
    let image = image_data();
    loop {
        tokio::select! {
            note = notes.recv_async() => {
                let Ok(note) = note else { return Ok(()) };
                let mut hints = HashMap::from([("desktop-entry", Variant::from("dev.shuv.rex-linux"))]);
                if let Some(image) = &image {
                    hints.insert("image-data", image.try_clone()?);
                }
                let id: u32 = proxy
                    .call(
                        "Notify",
                        &("Rex", 0u32, "dev.shuv.rex-linux", &note.summary, &note.body,
                          vec!["default", "Show"], hints, -1i32),
                    )
                    .await?;
                targets.insert(id, note.target);
                order.push_back(id);
                while order.len() > REMEMBERED {
                    if let Some(old) = order.pop_front() {
                        targets.remove(&old);
                        tokens.remove(&old);
                    }
                }
            }
            signal = signals.next() => {
                let Some(signal) = signal else { return Ok(()) };
                let header = signal.header();
                match header.member().map(|m| m.as_str()) {
                    Some("ActivationToken") => {
                        if let Ok((id, token)) = signal.body().deserialize::<(u32, String)>()
                            && targets.contains_key(&id)
                        {
                            tokens.insert(id, token);
                        }
                    }
                    Some("ActionInvoked") => {
                        if let Ok((id, _action)) = signal.body().deserialize::<(u32, String)>()
                            && let Some(target) = targets.remove(&id)
                        {
                            let token = tokens.remove(&id);
                            handle.update(move |s, tx| s.open_note(target, token, tx));
                        }
                    }
                    Some("NotificationClosed") => {
                        if let Ok((id, _reason)) = signal.body().deserialize::<(u32, u32)>() {
                            targets.remove(&id);
                            tokens.remove(&id);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_notification_picture_matches_the_freedesktop_image_data_format() {
        let image = super::image_data().expect("embedded Rex icon decodes");
        let (width, height, stride, alpha, bits, channels, pixels): (
            i32,
            i32,
            i32,
            bool,
            i32,
            i32,
            Vec<u8>,
        ) = image.try_into().expect("image-data is (iiibiiay)");
        assert_eq!((width, height, stride), (128, 128, 128 * 4));
        assert_eq!((alpha, bits, channels), (true, 8, 4));
        assert_eq!(pixels.len(), (stride * height) as usize);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    }
}
