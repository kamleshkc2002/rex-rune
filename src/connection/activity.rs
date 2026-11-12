//! What programs in a terminal report about themselves: OSC 7501 status records, OSC 9;4
//! progress, the bell, the foreground process, the terminal title, and OSC 9 / OSC 777 desktop
//! notifications. All of it arrives as the terminal creator's `block_event`s, or from its
//! `program_status`, `process` and `title` methods.
use super::titles::Place;
use serde_json::Value;

pub const TERMINAL: &str = "com.superlogical.terminal";

/// A record's state, in priority order: a tab shows the highest state among its records.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProgramState {
    #[default]
    Idle,
    Done,
    Working,
    Error,
    Blocked,
}

impl ProgramState {
    /// Unknown states read as idle, so a newer server's states never raise an alarm.
    fn parse(value: &str) -> Self {
        match value {
            "working" => Self::Working,
            "done" => Self::Done,
            "blocked" => Self::Blocked,
            "error" => Self::Error,
            _ => Self::Idle,
        }
    }
}

/// Why a blocked program is waiting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockedKind {
    Permission,
    Question,
    Auth,
}

/// A terminal's OSC 9;4 progress report, with its percentage when the program gave one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalProgress {
    Set(Option<u8>),
    Indeterminate,
    Paused(Option<u8>),
    Error(Option<u8>),
}

impl TerminalProgress {
    /// Which report a tab shows when several of its terminals report: errors, then pauses,
    /// then progress.
    pub fn rank(self) -> u8 {
        match self {
            Self::Error(_) => 2,
            Self::Paused(_) => 1,
            Self::Set(_) | Self::Indeterminate => 0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StatusRecord {
    pub state: ProgramState,
    pub kind: Option<BlockedKind>,
    /// The program that reported, as `effective_app` (or `app`) names it.
    pub app: Option<String>,
    /// The program's own words; shown only as plain text.
    pub title: Option<String>,
    pub msg: Option<String>,
    /// How far along the program says it is, 0–100; only working and blocked records have one.
    pub progress: Option<u8>,
    /// Reported while this client was listening, not read back by a query: only live
    /// changes are news worth a notification.
    pub live: bool,
}

impl StatusRecord {
    pub fn parse(value: &Value) -> Self {
        let text = |key: &str| {
            value[key]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        };
        Self {
            state: ProgramState::parse(value["state"].as_str().unwrap_or_default()),
            kind: match value["kind"].as_str() {
                Some("permission") => Some(BlockedKind::Permission),
                Some("question") => Some(BlockedKind::Question),
                Some("auth") => Some(BlockedKind::Auth),
                _ => None,
            },
            app: text("effective_app").or_else(|| text("app")),
            title: text("title"),
            msg: text("msg"),
            progress: value["progress"]
                .as_u64()
                .filter(|p| *p <= 100)
                .map(|p| p as u8),
            live: false,
        }
    }
}

/// The name to show for a foreground process: the basename of the first of `script`,
/// `argv0` and `name` that is present, without a login shell's leading `-`.
pub fn program_name(foreground: &Value) -> Option<String> {
    ["script", "argv0", "name"]
        .iter()
        .filter_map(|key| foreground[*key].as_str())
        .find(|s| !s.is_empty())
        .map(|s| s.rsplit('/').next().unwrap_or(s).trim_start_matches('-'))
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

#[derive(Clone, Debug, PartialEq)]
pub enum BlockEvent {
    StatusChanged {
        id: String,
        record: StatusRecord,
    },
    StatusRemoved {
        ids: Vec<String>,
    },
    /// The foreground program's name, and the pids of it and its ancestors; empty when the
    /// server couldn't tell.
    Process {
        name: Option<String>,
        pids: Vec<u64>,
        /// Foreground directory and the terminal's own shell, for display-only titles.
        place: Place,
    },
    /// A program set the terminal's title (empty: cleared), with the pid that set it.
    Title {
        text: String,
        owner: Option<u64>,
    },
    Notification {
        title: String,
        body: String,
    },
    /// An OSC 9;4 report; none when the program removed its progress.
    Progress(Option<TerminalProgress>),
    /// The program rang the terminal's bell (BEL).
    Bell,
}

/// A terminal `block_event`'s block and meaning; none for other creators and other events.
pub fn parse_block_event(payload: &Value) -> Option<(String, BlockEvent)> {
    if payload["creator_name"] != TERMINAL {
        return None;
    }
    let block = payload["block_id"].as_str()?.to_owned();
    let body = &payload["payload"];
    let event = match payload["name"].as_str()? {
        "program_status_changed" => BlockEvent::StatusChanged {
            id: body["id"].as_str().unwrap_or_default().to_owned(),
            record: StatusRecord {
                live: true,
                ..StatusRecord::parse(&body["record"])
            },
        },
        "program_status_removed" => BlockEvent::StatusRemoved {
            ids: body["ids"]
                .as_array()?
                .iter()
                .filter_map(|id| id.as_str().map(str::to_owned))
                .collect(),
        },
        "process_changed" => parse_process(body),
        "title_changed" => parse_title(body),
        "desktop_notification" => BlockEvent::Notification {
            title: body["title"].as_str().unwrap_or_default().to_owned(),
            body: body["body"].as_str().unwrap_or_default().to_owned(),
        },
        "bell" => BlockEvent::Bell,
        "progress_report" => BlockEvent::Progress({
            let percent = body["progress"]
                .as_u64()
                .filter(|p| *p <= 100)
                .map(|p| p as u8);
            match body["state"].as_str()? {
                "remove" => None,
                "set" => Some(TerminalProgress::Set(percent)),
                "indeterminate" => Some(TerminalProgress::Indeterminate),
                "pause" => Some(TerminalProgress::Paused(percent)),
                "error" => Some(TerminalProgress::Error(percent)),
                _ => return None,
            }
        }),
        _ => return None,
    };
    Some((block, event))
}

/// A `process_changed` payload or a `process` reply.
pub fn parse_process(body: &Value) -> BlockEvent {
    let foreground = &body["foreground"];
    let pids = std::iter::once(foreground)
        .chain(body["ancestors"].as_array().into_iter().flatten())
        .filter_map(|p| p["pid"].as_u64())
        .collect();
    BlockEvent::Process {
        name: program_name(foreground),
        pids,
        place: Place::parse(body),
    }
}

/// A `title_changed` payload or a `title` reply.
pub fn parse_title(body: &Value) -> BlockEvent {
    BlockEvent::Title {
        text: body["title"].as_str().unwrap_or_default().trim().to_owned(),
        owner: body["owner"]["pid"].as_u64(),
    }
}

/// A `program_status` reply's records by id; the root record's id is empty.
pub fn parse_status(reply: &Value) -> Vec<(String, StatusRecord)> {
    reply["records"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| {
            (
                r["id"].as_str().unwrap_or_default().to_owned(),
                StatusRecord::parse(r),
            )
        })
        .collect()
}

/// The shell's directory from a server `process` reply, even without OSC 7. Prefer the
/// terminal's child, not an editor's foreground directory. macOS wraps the shell in login;
/// use the process just below it only when the ancestry is available.
pub fn shell_dir(process: &Value) -> Option<String> {
    let mut shell = &process["child"];
    if shell["name"] == "login" {
        shell = match process["ancestors"].as_array().map(Vec::as_slice) {
            Some([.., below, _login]) => below,
            Some([_login]) => &process["foreground"],
            _ => return None,
        };
        if shell["name"] == "login" {
            return None;
        }
    }
    shell["cwd"]
        .as_str()
        .filter(|dir| dir.starts_with('/'))
        .map(str::to_owned)
}

/// An OSC 9 or OSC 777 notification a program sent, with where it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct DesktopNotice {
    pub session: String,
    pub block: String,
    pub title: String,
    pub body: String,
}
