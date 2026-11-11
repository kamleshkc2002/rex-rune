//! Files downloaded from a terminal's machine through Rex (`download_begin`, API revision 13).
//!
//! This module is the library side: file names, the destination folder, the partial file, the
//! limits, the transfer state machine and the stream that fills the file. The connection layer
//! (`connection::HostCommand::Download`) drives it; nothing here has a UI or opens or runs a
//! downloaded file.
//!
//! The wire contract is in `docs/server-api13.md` ("Download contract (D0)"): the ticket's data
//! stream carries the file as binary messages of at most 32 KiB, then one empty message, then a
//! normal close (1000). Anything else is a failure, and a failed or cancelled transfer deletes
//! its partial file. Nothing is ever retried.
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

pub mod name;
pub mod store;
pub mod stream;

pub use store::{Destination, Partial, Placement, Records};

/// The API revision that introduced `download_begin`.
pub const MIN_API_REVISION: u32 = 13;
/// The method that reserves a file for download.
pub const METHOD: &str = "com.superlogical.terminal.download_begin";
/// The data stream's subprotocol, the same one uploads use.
pub const PROTOCOL: &str = crate::upload::PROTOCOL;

const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;

/// The data reference of a new download: `download:` and a random lower-case UUID.
pub fn new_ref() -> anyhow::Result<String> {
    Ok(format!("download:{}", crate::upload::new_uuid()?))
}

/// A fresh transfer id, unique in this process. The UI chooses one when it starts a download and
/// uses it to find the transfer in the model and to confirm or cancel it.
pub fn next_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

// -- Limits ----------------------------------------------------------------------------------

/// Where the free space of the destination comes from. Tests substitute their own.
pub trait FreeSpace: Send + Sync {
    /// Bytes available on the destination's filesystem.
    fn free(&self) -> std::io::Result<u64>;
}

impl FreeSpace for Destination {
    fn free(&self) -> std::io::Result<u64> {
        self.free_space()
    }
}

impl<F: Fn() -> std::io::Result<u64> + Send + Sync> FreeSpace for F {
    fn free(&self) -> std::io::Result<u64> {
        self()
    }
}

/// The limits of a download. The defaults are the product's; tests shrink them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// A size hint at or above this needs the person's confirmation: 1 GiB.
    pub confirm_at: u64,
    /// Abort when the destination has less than this left; also kept free by the confirmation
    /// rule: 256 MiB.
    pub reserve: u64,
    /// What a yes to the low-space question still keeps free: 16 MiB. The transfer may eat into
    /// the reserve but never below this, so a server that sends more than it said cannot fill
    /// the disk to nothing. A file that cannot fit with this left over is refused up front.
    pub low_floor: u64,
    /// The stream may exceed the hint by this much, or by the hint itself again if that is more:
    /// 64 MiB.
    pub growth: u64,
    /// Abort after this long with no bytes: 30 seconds.
    pub stall: Duration,
    /// Transfers running at once for one host: 3.
    pub per_host: usize,
    /// Free space is looked at again after this many bytes: 4 MiB.
    pub check_every: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            confirm_at: GIB,
            reserve: 256 * MIB,
            low_floor: 16 * MIB,
            growth: 64 * MIB,
            stall: Duration::from_secs(30),
            per_host: 3,
            check_every: 4 * MIB,
        }
    }
}

/// Why a download needs a yes before its stream is opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// The size hint is [`Limits::confirm_at`] or more. The destination has room for it and the
    /// reserve: a free-space check during the transfer still stops it.
    Large,
    /// The file fits in the free space but not with [`Limits::reserve`] left over. A yes means
    /// this transfer may use the reserve down to [`Limits::low_floor`]; below that, the growth
    /// cap, the stall timer and a full disk stop it.
    LowSpace,
}

/// A decision the person must make before the stream is opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Confirmation {
    pub reason: Reason,
    /// The file's size as the server reported it, in bytes.
    pub hint: u64,
    /// Free space in the destination, in bytes; `None` when the filesystem wouldn't say.
    pub free: Option<u64>,
}

/// What to do with a reserved file, from its size hint and the destination's free space.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Admission {
    /// Open the stream.
    Start,
    /// Ask first: no stream is open until the person says yes.
    Ask(Confirmation),
    /// It cannot succeed: do not ask, free Rex's reservation and fail with this.
    Refuse(Failure),
}

impl Limits {
    /// Decides what happens to a file of `hint` bytes when `free` bytes are available (`None`
    /// when the filesystem wouldn't say):
    /// - a hint above the free space less [`Limits::low_floor`]: refused with
    ///   [`Failure::LowSpace`], since no answer could make it fit with the floor left over;
    /// - less free space than the reserve, or a hint above the free space less the reserve: a
    ///   [`Reason::LowSpace`] question;
    /// - a hint of [`Limits::confirm_at`] or more: a [`Reason::Large`] question;
    /// - otherwise it starts. Unknown free space asks only the large-file question.
    pub fn admission(&self, hint: u64, free: Option<u64>) -> Admission {
        let Some(free) = free else {
            return self
                .confirmation_without_space(hint)
                .map_or(Admission::Start, Admission::Ask);
        };
        if free
            .checked_sub(self.low_floor)
            .is_none_or(|room| hint > room)
        {
            return Admission::Refuse(Failure::LowSpace { free });
        }
        self.confirmation(hint, free)
            .map_or(Admission::Start, Admission::Ask)
    }

    /// Whether a file of `hint` bytes that fits in `free` bytes needs confirming. A hint that is
    /// both large and over the free space less the reserve is reported as [`Reason::LowSpace`].
    /// [`Limits::admission`] is what decides; it refuses a hint that doesn't fit at all. With
    /// less free space than the reserve every file is a [`Reason::LowSpace`] question.
    pub fn confirmation(&self, hint: u64, free: u64) -> Option<Confirmation> {
        let reason = if free < self.reserve || hint > free - self.reserve {
            Reason::LowSpace
        } else if hint >= self.confirm_at {
            Reason::Large
        } else {
            return None;
        };
        Some(Confirmation {
            reason,
            hint,
            free: Some(free),
        })
    }

    /// The same question when the destination's free space can't be read: only the size can
    /// make it a large file. A filesystem that can't say is not a reason to skip the question,
    /// or to stop.
    pub fn confirmation_without_space(&self, hint: u64) -> Option<Confirmation> {
        (hint >= self.confirm_at).then_some(Confirmation {
            reason: Reason::Large,
            hint,
            free: None,
        })
    }

    /// Looks again at a transfer whose question was answered yes while `confirmed` was asked,
    /// since the answer may have waited a long while and the disk may have filled meanwhile.
    /// Fails with what [`Limits::admission`] now refuses with; and with [`Failure::LowSpace`]
    /// when a [`Reason::Large`] yes meets space that now needs the low-space question (the
    /// person answered a different question). Otherwise the transfer goes on.
    pub(crate) fn recheck(
        &self,
        confirmed: Reason,
        hint: u64,
        free: Option<u64>,
    ) -> Result<(), Failure> {
        match self.admission(hint, free) {
            Admission::Refuse(failure) => Err(failure),
            Admission::Ask(Confirmation {
                reason: Reason::LowSpace,
                free: Some(free),
                ..
            }) if confirmed == Reason::Large => Err(Failure::LowSpace { free }),
            _ => Ok(()),
        }
    }

    /// The most bytes a stream may carry for a hint: the larger of twice the hint and the hint
    /// plus [`Limits::growth`].
    pub fn growth_cap(&self, hint: u64) -> u64 {
        hint.saturating_mul(2).max(hint.saturating_add(self.growth))
    }
}

/// Counts a stream's bytes against the limits. Time is passed in, so tests control the clock.
pub struct Meter {
    limits: Limits,
    cap: u64,
    received: u64,
    since_check: u64,
    last_data: Instant,
}

impl Meter {
    pub fn new(limits: Limits, hint: u64, now: Instant) -> Self {
        Self {
            limits,
            cap: limits.growth_cap(hint),
            received: 0,
            since_check: 0,
            last_data: now,
        }
    }

    pub fn received(&self) -> u64 {
        self.received
    }

    /// When the stream counts as stalled if nothing more arrives.
    pub fn deadline(&self) -> Instant {
        self.last_data + self.limits.stall
    }

    pub fn stalled(&self, now: Instant) -> bool {
        now >= self.deadline()
    }

    /// Whether `n` more bytes may be written: not if they would take the stream past its growth
    /// cap. Asked before writing, so a hostile server never makes the client write past it.
    pub fn admit(&self, n: u64) -> Result<(), Failure> {
        if self.received.saturating_add(n) > self.cap {
            return Err(Failure::TooMuchData { limit: self.cap });
        }
        Ok(())
    }

    /// Counts `n` bytes that arrived and were written. Fails when the stream passes its growth
    /// cap, or when the destination's free space (looked at every [`Limits::check_every`]
    /// bytes) has fallen below the reserve.
    pub fn wrote(&mut self, n: u64, now: Instant, space: &dyn FreeSpace) -> Result<(), Failure> {
        self.last_data = now;
        self.received += n;
        if self.received > self.cap {
            return Err(Failure::TooMuchData { limit: self.cap });
        }
        self.since_check += n;
        if self.since_check >= self.limits.check_every {
            self.since_check = 0;
            // A filesystem that can't say is not a reason to stop.
            if let Ok(free) = space.free()
                && free < self.limits.reserve
            {
                return Err(Failure::LowSpace { free });
            }
        }
        Ok(())
    }
}

// -- Sizes in words ---------------------------------------------------------------------------

const UNITS: [&str; 5] = ["kB", "MB", "GB", "TB", "PB"];

/// The unit a size is written in: 0 is bytes, 1 kB, 2 MB, and so on.
pub fn power_of(bytes: u64) -> usize {
    let mut value = bytes as f64;
    let mut power = 0;
    while value >= 999.95 && power < UNITS.len() {
        value /= 1000.;
        power += 1;
    }
    power
}

/// `3.2 GB`, `41 GB`, `12.4 MB`, `6 bytes`: decimal units, one decimal below 100.
pub fn size_text(bytes: u64) -> String {
    let power = power_of(bytes);
    if power == 0 {
        return format!("{bytes} byte{}", if bytes == 1 { "" } else { "s" });
    }
    format!(
        "{} {}",
        number_text(bytes as f64 / 1000f64.powi(power as i32)),
        UNITS[power - 1]
    )
}

pub fn number_text(value: f64) -> String {
    if value >= 99.95 {
        format!("{value:.0}")
    } else {
        let text = format!("{value:.1}");
        text.strip_suffix(".0").map_or(text.clone(), str::to_owned)
    }
}

// -- Configuration ---------------------------------------------------------------------------

/// What a transfer needs beyond its request. `Config::default()` is the product; tests replace
/// the pieces.
#[derive(Clone)]
pub struct Config {
    pub limits: Limits,
    pub records: Records,
    /// Free space of the destination; the folder's own filesystem when none.
    pub space: Option<Arc<dyn FreeSpace>>,
    pub placement: Placement,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            limits: Limits::default(),
            records: Records::default(),
            space: None,
            placement: Placement::Auto,
        }
    }
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("limits", &self.limits)
            .field("records", &self.records)
            .finish_non_exhaustive()
    }
}

// -- The transfer ---------------------------------------------------------------------------

/// Why a transfer failed. [`Display`](std::fmt::Display) gives the sentence the UI shows after
/// "Failed:"; the UI words the old-server case itself with the host's name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// The host's Rex is older than API revision 13 (nothing was sent), or answered
    /// `revision_too_low` or `method_not_found`. The UI says "Rex on <host> is too old to send
    /// files; update Rex there."
    ServerTooOld,
    /// Three transfers are already running for this host.
    TooManyActive { limit: usize },
    /// The host was not connected when the download was asked for.
    Disconnected,
    /// Rex refused the request, in its own words (no such file, a folder, no permission...).
    Server(String),
    /// Rex didn't answer the request in time. It may still have reserved the file.
    NoAnswer,
    /// The destination folder can't be used.
    Destination(String),
    /// Rex's reply isn't one a download can use (not modified, or no ticket).
    UnexpectedReply(String),
    /// The person didn't answer the confirmation before the ticket expired.
    ConfirmationExpired,
    /// The data endpoint refused the ticket (reason byte of `REJECT`).
    Rejected(u8),
    /// The data connection couldn't be made or broke.
    Connection(String),
    /// The control connection ended, or was replaced by a reconnect, so Rex ended the stream.
    ConnectionLost,
    /// The destination has less than the reserve left, or (before the stream is opened) less
    /// room than the file needs.
    LowSpace { free: u64 },
    /// The stream went past the growth cap.
    TooMuchData { limit: u64 },
    /// No bytes for the stall timeout.
    Stalled,
    /// The stream ended another way than Rex's normal end.
    Closed { code: Option<u16>, reason: String },
    /// The stream broke the data contract (data after the end marker, a text message...).
    Protocol(String),
    /// Writing the file failed.
    Disk(String),
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServerTooOld => write!(f, "Rex is too old to send files; update Rex there"),
            Self::TooManyActive { limit } => {
                write!(f, "{limit} downloads from this host are already running")
            }
            Self::Disconnected => write!(f, "the host isn't connected"),
            Self::Server(message) => write!(f, "{}", display_text(message)),
            Self::NoAnswer => write!(f, "Rex didn't answer in time; the download was not retried"),
            Self::Destination(message) | Self::Disk(message) | Self::Connection(message) => {
                write!(f, "{}", display_text(message))
            }
            Self::UnexpectedReply(what) => {
                write!(f, "unexpected reply from Rex: {}", display_text(what))
            }
            Self::ConfirmationExpired => {
                write!(
                    f,
                    "not confirmed in time; Rex stops holding a file after a minute"
                )
            }
            Self::Rejected(reason) => write!(f, "Rex refused the download (reason {reason})"),
            Self::ConnectionLost => write!(
                f,
                "the connection to Rex was lost; the download was not retried"
            ),
            Self::LowSpace { free } => {
                write!(
                    f,
                    "the destination has only {} free, too little to save this file and keep some space free",
                    size_text(*free)
                )
            }
            Self::TooMuchData { limit } => write!(
                f,
                "the file grew past {} MB while downloading",
                limit / 1_000_000
            ),
            Self::Stalled => write!(f, "no data for 30 seconds"),
            Self::Closed { code, reason } => match (code, display_text(reason)) {
                (Some(code), reason) if !reason.trim().is_empty() => {
                    write!(f, "Rex ended the transfer: {reason} ({code})")
                }
                (Some(code), _) => write!(f, "Rex ended the transfer ({code})"),
                (None, _) => write!(f, "the connection ended before the file was complete"),
            },
            Self::Protocol(what) => write!(f, "unexpected data from Rex: {}", display_text(what)),
        }
    }
}

/// Where a transfer is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Asked for; Rex hasn't answered.
    Requested,
    /// Rex has reserved the file and the size needs a yes. The UI must answer with
    /// `HostCommand::DownloadConfirm` or `DownloadCancel`; no stream is open.
    AwaitingConfirmation(Confirmation),
    /// Bytes are arriving. `hint` is the server's size, for progress only.
    Receiving {
        received: u64,
        hint: u64,
    },
    /// The file Rex sent is at `path`. `short_of` is Rex's size hint when the stream ended
    /// normally with fewer bytes than that (a file that shrank, or one whose reported size is
    /// wrong, like most files under `/sys`): the file is kept and the UI says so.
    Saved {
        path: PathBuf,
        bytes: u64,
        short_of: Option<u64>,
    },
    Cancelled,
    Failed(Failure),
}

impl State {
    /// Saved, cancelled or failed: nothing more will happen.
    pub fn is_final(&self) -> bool {
        matches!(self, Self::Saved { .. } | Self::Cancelled | Self::Failed(_))
    }
}

/// Something that happened to a transfer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Rex reserved the file: its resolved path and name, the size hint, and whether the size
    /// needs confirming.
    Begun {
        resolved: String,
        name: String,
        hint: u64,
        confirmation: Option<Confirmation>,
    },
    Confirmed,
    Progress {
        received: u64,
    },
    Saved {
        path: PathBuf,
        bytes: u64,
        short_of: Option<u64>,
    },
    Cancelled,
    Failed(Failure),
}

/// One download, as the UI sees it. The host's model keeps the list for this launch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transfer {
    pub id: u64,
    pub session: String,
    pub block: String,
    /// The path as it was asked for.
    pub requested: String,
    /// The destination folder.
    pub directory: PathBuf,
    /// The path Rex resolved, for display (control characters removed). It can differ from what
    /// was asked for: a relative path resolves against the foreground program's directory.
    pub resolved: Option<String>,
    /// The file name it is saved under, sanitized. Known once Rex answers.
    pub name: Option<String>,
    /// The connection epoch (`HostModel::connection_epoch`) it was started on.
    pub epoch: u64,
    pub state: State,
}

impl Transfer {
    pub fn new(
        id: u64,
        session: String,
        block: String,
        requested: String,
        directory: PathBuf,
        epoch: u64,
    ) -> Self {
        Self {
            id,
            session,
            block,
            requested,
            directory,
            resolved: None,
            name: None,
            epoch,
            state: State::Requested,
        }
    }

    /// The name to show: the sanitized name from Rex, or until it answers, the last component
    /// of the requested path, sanitized.
    pub fn display_name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| name::sanitize(&self.requested))
    }

    /// Progress from 0 to 1 while receiving a file of known size.
    pub fn fraction(&self) -> Option<f32> {
        match self.state {
            State::Receiving { received, hint } if hint > 0 => {
                Some((received as f64 / hint as f64).min(1.0) as f32)
            }
            _ => None,
        }
    }

    /// Applies an event. Returns whether it changed anything: events that don't fit the state
    /// (including every event after a final state) are ignored.
    pub fn apply(&mut self, event: Event) -> bool {
        use State::*;
        match (&self.state, event) {
            (
                Requested,
                Event::Begun {
                    resolved,
                    name,
                    hint,
                    confirmation,
                },
            ) => {
                self.resolved = Some(resolved);
                self.name = Some(name);
                self.state = match confirmation {
                    Some(confirmation) => AwaitingConfirmation(confirmation),
                    None => Receiving { received: 0, hint },
                };
            }
            (AwaitingConfirmation(confirmation), Event::Confirmed) => {
                self.state = Receiving {
                    received: 0,
                    hint: confirmation.hint,
                };
            }
            (Receiving { hint, received }, Event::Progress { received: now })
                if now >= *received =>
            {
                self.state = Receiving {
                    received: now,
                    hint: *hint,
                };
            }
            (
                Receiving { .. },
                Event::Saved {
                    path,
                    bytes,
                    short_of,
                },
            ) => {
                self.state = Saved {
                    path,
                    bytes,
                    short_of,
                };
            }
            (Requested | AwaitingConfirmation(_) | Receiving { .. }, Event::Cancelled) => {
                self.state = Cancelled;
            }
            (Requested | AwaitingConfirmation(_) | Receiving { .. }, Event::Failed(failure)) => {
                self.state = Failed(failure);
            }
            _ => return false,
        }
        true
    }
}

/// Finished transfers kept in the model, oldest dropped first.
pub const KEEP_FINISHED: usize = 50;

/// Adds a transfer to a list, dropping the oldest finished ones past [`KEEP_FINISHED`].
pub fn push(list: &mut Vec<Transfer>, transfer: Transfer) {
    list.push(transfer);
    let finished = list.iter().filter(|t| t.state.is_final()).count();
    let mut drop = finished.saturating_sub(KEEP_FINISHED);
    list.retain(|t| {
        if drop > 0 && t.state.is_final() {
            drop -= 1;
            false
        } else {
            true
        }
    });
}

/// How many transfers in the list are not finished.
pub fn active(list: &[Transfer]) -> usize {
    list.iter().filter(|t| !t.state.is_final()).count()
}

/// Server text for display: control characters become spaces, direction controls and other
/// invisible format characters (the set file names are cleaned of) are removed, and a text over
/// 300 characters is shortened in the middle, keeping its start and its end (the end of a path
/// is its file name) around an ellipsis. Rex's messages repeat the whole path, which can be
/// thousands of characters. Cleaning text that was already cleaned changes nothing.
pub fn display_text(text: &str) -> String {
    const LIMIT: usize = 300;
    const HEAD: usize = 100;
    let cleaned: Vec<char> = text
        .chars()
        .filter_map(|c| match c {
            c if c.is_control() => Some(' '),
            c if name::dropped(c) => None,
            c => Some(c),
        })
        .collect();
    if cleaned.len() <= LIMIT {
        return cleaned.into_iter().collect();
    }
    let tail = LIMIT - HEAD - 1;
    let mut short: String = cleaned[..HEAD].iter().collect();
    short.push('…');
    short.extend(&cleaned[cleaned.len() - tail..]);
    short
}

#[cfg(test)]
pub(crate) mod tests;
