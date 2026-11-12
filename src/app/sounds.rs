//! Optional local activity audio. Only typed, fixed sound names cross this boundary.
//! One coalescing slot, no playback queue; all discovery and player I/O is worker-only.
use super::workspace::Workspace;
use rex_linux::connection::activity::ProgramState;
use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};

mod player;
#[cfg(test)]
mod tests;

const GATHER: Duration = Duration::from_millis(100);
const INTERVAL: Duration = Duration::from_secs(1);
/// How long exit waits for the worker to stop its player and return.
const EXIT_WAIT: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Sound {
    Done,
    Error,
    NeedsYou,
}
impl Sound {
    fn id(self) -> &'static str {
        match self {
            Self::NeedsYou => "dialog-warning",
            Self::Error => "dialog-error",
            Self::Done => "complete",
        }
    }
    pub fn for_state(state: ProgramState) -> Option<Self> {
        match state {
            ProgramState::Blocked => Some(Self::NeedsYou),
            ProgramState::Error => Some(Self::Error),
            ProgramState::Done => Some(Self::Done),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Health {
    #[default]
    Unchecked,
    Available,
    Missing,
    Failed,
    TimedOut,
    WorkerStopped,
}
impl Health {
    pub fn text(self) -> &'static str {
        match self {
            // What the switch does, until there is something to report about it.
            Self::Unchecked | Self::Available => {
                "For tabs you aren't looking at, for the kinds switched on below."
            }
            Self::Missing => "No sound player or sound files were found on this computer.",
            Self::Failed => "The last sound could not be played.",
            Self::TimedOut => "The last sound did not finish and was stopped.",
            Self::WorkerStopped => "Sounds could not be started.",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Request {
    Event(Sound),
    Test,
    Check,
    Cancel,
}

struct Pending {
    due: Duration,
    sound: Sound,
    preview: bool,
}

/// Clock supplied by the caller: tests advance it without sleeping. Events arriving
/// during the gather window compete by severity; during playback/cooldown they vanish.
#[derive(Default)]
struct Gate {
    pending: Option<Pending>,
    next: Duration,
    busy: bool,
    check: bool,
    stopped: bool,
}
impl Gate {
    fn offer(&mut self, request: Request, now: Duration) {
        match request {
            Request::Cancel => {
                if self.pending.as_ref().is_some_and(|p| !p.preview) {
                    self.pending = None;
                }
            }
            Request::Check => self.check = true,
            Request::Test if !self.busy => {
                self.pending = Some(Pending {
                    due: now,
                    sound: Sound::Done,
                    preview: true,
                });
            }
            Request::Event(sound) if !self.busy && now >= self.next => {
                if let Some(pending) = &mut self.pending {
                    if !pending.preview {
                        pending.sound = pending.sound.max(sound);
                    }
                } else {
                    self.pending = Some(Pending {
                        due: now + GATHER,
                        sound,
                        preview: false,
                    });
                }
            }
            _ => {}
        }
    }
    fn take(&mut self, now: Duration) -> Option<Sound> {
        let pending = self.pending.as_ref()?;
        let (due, sound) = (pending.due, pending.sound);
        // A slow page check must not turn this slot into a stale playback queue.
        if now > due + INTERVAL {
            self.pending = None;
            return None;
        }
        if now < due || self.busy || self.stopped {
            return None;
        }
        self.pending = None;
        self.busy = true;
        self.next = now + INTERVAL;
        Some(sound)
    }
}

struct Shared {
    gate: Mutex<Gate>,
    thread: Mutex<Option<JoinHandle<()>>>,
    wake: flume::Sender<()>,
    receiver: flume::Receiver<()>,
}
impl Default for Shared {
    fn default() -> Self {
        let (wake, receiver) = flume::bounded(1);
        Self {
            gate: Default::default(),
            thread: Default::default(),
            wake,
            receiver,
        }
    }
}
impl Shared {
    fn stop(&self) {
        self.gate.lock().unwrap().stopped = true;
        let _ = self.wake.try_send(());
    }
}

/// Joined only after Rune's event loop has exited, never while drawing or handling input.
#[derive(Clone, Default)]
pub struct Shutdown(Arc<Shared>);
impl Shutdown {
    pub fn finish(&self) {
        self.0.stop();
        let Some(thread) = self.0.thread.lock().unwrap().take() else {
            return;
        };
        // The worker stops a player within a poll and exits. Looking for one touches
        // every `PATH` and data directory, though, and one of those on a hung mount must
        // not keep the process alive after its window closed: wait briefly, then let go.
        let until = Instant::now() + EXIT_WAIT;
        while !thread.is_finished() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        if thread.is_finished() {
            let _ = thread.join();
        }
    }
}

#[derive(Default)]
pub struct Sounds {
    shutdown: Shutdown,
    started: bool,
    #[cfg(test)]
    pub fake: tests::Fake,
}
impl Drop for Sounds {
    fn drop(&mut self) {
        self.shutdown.0.stop();
    }
}
impl Sounds {
    pub fn shutdown(&self) -> Shutdown {
        self.shutdown.clone()
    }

    fn request(&mut self, request: Request, handle: rune::AppHandle<Workspace>) -> bool {
        let shared = self.shutdown.0.clone();
        if !self.started {
            if matches!(request, Request::Cancel) {
                return true;
            }
            let receiver = shared.receiver.clone();
            let worker_shared = shared.clone();
            let Ok(thread) = std::thread::Builder::new()
                .name("rex-sounds".into())
                .spawn(move || run(worker_shared, receiver, handle))
            else {
                return false;
            };
            *shared.thread.lock().unwrap() = Some(thread);
            self.started = true;
        }
        // All timestamps share the process monotonic clock, including the worker.
        shared.gate.lock().unwrap().offer(request, now());
        shared.wake.try_send(()).ok();
        true
    }
}

fn now() -> Duration {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    START.get_or_init(Instant::now).elapsed()
}

fn run(shared: Arc<Shared>, wake: flume::Receiver<()>, handle: rune::AppHandle<Workspace>) {
    let mut last = Health::Unchecked;
    loop {
        let (sound, check) = {
            let mut gate = shared.gate.lock().unwrap();
            if gate.stopped {
                return;
            }
            (gate.take(now()), std::mem::take(&mut gate.check))
        };
        if sound.is_some() || check {
            let health = player::attempt(&mut player::System, sound, || {
                shared.gate.lock().unwrap().stopped
            });
            shared.gate.lock().unwrap().busy = false;
            // A page check must not erase the last playback failure.
            let health = if sound.is_none()
                && health == Health::Available
                && matches!(last, Health::Failed | Health::TimedOut)
            {
                last
            } else {
                health
            };
            if last != health {
                last = health;
                handle.update(move |s, _| s.sound_health = health);
            }
        }
        let delay = {
            let gate = shared.gate.lock().unwrap();
            if gate.stopped {
                return;
            }
            gate.pending.as_ref().map(|p| p.due.saturating_sub(now()))
        };
        // No idle polling. Requests and shutdown wake the worker; only the short
        // gather window needs a timer. A running child checks stop every 20 ms.
        match delay {
            Some(delay) => {
                let _ = wake.recv_timeout(delay);
            }
            None => {
                let _ = wake.recv();
            }
        }
    }
}

impl Workspace {
    pub fn sound_request(&mut self, request: Request) {
        #[cfg(test)]
        {
            self.sound_health = self.sounds.fake.request(request);
            return;
        }
        #[allow(unreachable_code)]
        if !self.sounds.request(request, self.handle.clone()) {
            self.sound_health = Health::WorkerStopped;
        }
    }
}
