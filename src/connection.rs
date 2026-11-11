//! One control connection per host; stable, independently streamed terminals by block ID.
use crate::{
    terminal::{Frame, Renderer, TerminalStream},
    transport::{self, Client},
    wire,
};
use activity::{BlockEvent, DesktopNotice, StatusRecord, TerminalProgress};
use anyhow::{Context, Result};
use rexrs::api::{BlockMethod, session, terminal::SetTheme};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub mod activity;
pub mod clipboard;
pub mod downloads;
mod prefetch;

/// The label new tabs get. A tab that still has it shows its program's terminal title.
pub const DEFAULT_TAB_LABEL: &str = "Shell";

#[derive(Clone, Debug, clap::Parser)]
#[command(
    version,
    about = "Native multi-host workspace for existing Rex servers"
)]
pub struct Options {
    #[arg(long, default_value_t = transport::local_endpoint())]
    pub endpoint: String,
    /// Initially select a session by ID or exact label.
    #[arg(long)]
    pub session: Option<String>,
    /// Initially focus this terminal block.
    #[arg(long)]
    pub block: Option<String>,
    /// Explicitly create a persistent session.
    #[arg(long, conflicts_with = "session")]
    pub new: Option<String>,
    /// Override the saved terminal family for this launch.
    #[arg(long, value_parser = crate::settings::parse_font_family)]
    pub font: Option<String>,
    /// Override saved fallback families, in order; repeat for more than one.
    #[arg(long, value_parser = crate::settings::parse_font_family)]
    pub font_fallback: Vec<String>,
    /// Override the saved terminal size (8–48 pt).
    #[arg(long, value_parser = parse_font_size)]
    pub font_size: Option<f32>,
    /// Use a separate workspace preferences file.
    #[arg(long)]
    pub config: Option<PathBuf>,
    #[arg(long, default_value = "1440x900")]
    pub window_size: String,
    /// Check signed release metadata for updates and exit (0 = up to date, 10 = update available, 1 = error).
    #[arg(long)]
    pub check_update: bool,
    /// Rune's own flags (`--snap out.png`, `--tree`, `--script steps.json`), after `--`.
    #[arg(last = true, hide = true)]
    pub rune: Vec<String>,
}

impl Options {
    pub fn fonts<'a>(&'a self, settings: &'a crate::settings::Settings) -> (&'a str, &'a [String]) {
        (
            self.font.as_deref().unwrap_or(&settings.font_family),
            if self.font_fallback.is_empty() {
                &settings.font_fallbacks
            } else {
                &self.font_fallback
            },
        )
    }
}

fn parse_font_size(value: &str) -> std::result::Result<f32, String> {
    let size: f32 = value
        .parse()
        .map_err(|_| "Enter a font size in points.".to_owned())?;
    if !size.is_finite() {
        return Err("Font size must be finite.".into());
    }
    Ok(size.clamp(8., 48.))
}

pub mod actions;
pub mod keymap;
pub mod perform;

pub enum Command {
    Selection(crate::terminal::selection::Input),
    Key(Value),
    KeyWithPrefix {
        key: Value,
        prefix: Vec<String>,
    },
    /// Explicit pane.send_key only; normal keys always use client.key.
    SendKey(Value),
    Paste(String),
    /// A `block.event` mouse payload: `action`, `button`, `x`, `y` (cells) and `mods`.
    Mouse(Value),
    /// A `block.event` wheel payload: `x`, `y`, `delta_x` and `delta_y`.
    Wheel(Value),
    Resize(u16, u16, u16, u16),
    Scroll(isize),
    /// Search the terminal's screen and scrollback for this text; empty stops searching.
    Search(String),
    /// Select the next older search match (`true`) or the next newer one.
    SearchStep(bool),
    TakeControl,
    Reconnect,
}

#[derive(Default)]
pub struct Model {
    pub frame: Option<Arc<Frame>>,
    pub status: String,
    pub session: String,
    pub block: String,
    pub connected: bool,
    pub control_connected: bool,
    pub owner: bool,
    pub revision: u64,
    pub restore_generation: u64,
    pub input_events: u64,
    /// Words for the user from the pane's worker (a failed search), taken once shown.
    pub notice: Option<String>,
}

#[derive(Clone)]
pub struct TerminalSender {
    host: flume::Sender<HostCommand>,
    session: String,
    block: String,
}
impl TerminalSender {
    /// Commands for one block, sent through its host's worker.
    pub fn new(host: flume::Sender<HostCommand>, session: String, block: String) -> Self {
        Self {
            host,
            session,
            block,
        }
    }
    pub fn focus(&self) -> Result<()> {
        self.host
            .send(HostCommand::Request {
                method: "session.focus_block".into(),
                payload: json!({"session_id":self.session,"block_id":self.block}),
            })
            .map_err(|_| anyhow::anyhow!("host worker stopped"))
    }
    pub fn try_send(&self, command: Command) -> Result<()> {
        self.host
            .send(HostCommand::Terminal {
                session: self.session.clone(),
                block: self.block.clone(),
                command,
            })
            .map_err(|_| anyhow::anyhow!("host worker stopped"))
    }
}

#[derive(Clone)]
pub struct Connection {
    pub model: Arc<Mutex<Model>>,
    pub commands: TerminalSender,
    pub wake: flume::Receiver<()>,
}

#[derive(Clone, Default)]
pub struct ProtocolStats {
    pub api_revision: u32,
    pub bootstrap_requests: u64,
    pub attach_requests: u64,
    pub bundled_tickets_used: u64,
    pub data_connect_requests: u64,
}

#[derive(Clone, Default)]
pub struct HostModel {
    /// A successful SSH lookup found no default server; setup may be offered, never run.
    pub ssh_server_missing: bool,
    pub visible: bool,
    pub connected: bool,
    pub status: String,
    pub notice: Option<String>,
    pub client_id: String,
    pub keymap: Option<Arc<keymap::Snapshot>>,
    pub keymap_error: Option<String>,
    pub keymap_revision: u64,
    /// Taken once before cloning the worker model into the UI.
    pub client_actions: Vec<actions::Queued>,
    pub performs: Vec<perform::Pending>,
    /// This launch's downloads from the host, oldest first; see [`crate::download`].
    pub downloads: Vec<crate::download::Transfer>,
    pub connection_epoch: u64,
    pub protocol: ProtocolStats,
    pub sessions: Vec<session::Session>,
    pub selected: Option<String>,
    /// Completion of the latest client navigation request (id, success), not a notice.
    pub navigation_result: Option<(u64, bool)>,
    pub views: HashMap<String, session::ViewResponse>,
    pub terminals: HashMap<String, Connection>,
    /// Each terminal block's OSC 7501 records, by record id (empty for the root record).
    pub activity: HashMap<String, BTreeMap<String, StatusRecord>>,
    /// Each terminal block's foreground program, when the server named one.
    pub programs: HashMap<String, String>,
    /// Each terminal block's foreground process and its ancestors, by pid.
    pub foreground: HashMap<String, Vec<u64>>,
    /// Each terminal block's title, as a program last set it, and the pid that set it.
    pub titles: HashMap<String, (String, Option<u64>)>,
    /// Each terminal block's OSC 9;4 progress, until its program removes it.
    pub progress: HashMap<String, TerminalProgress>,
    /// Program notifications not yet taken by the UI.
    pub notices: Vec<DesktopNotice>,
    /// Bells not yet taken by the UI, as (session, block).
    pub bells: Vec<(String, String)>,
    /// Latest terminal-initiated copy, consumed once by the foreground UI.
    pub clipboard: Option<clipboard::ClipboardCopy>,
    /// Bumped whenever `activity` is dropped wholesale (on reconnect), so the UI starts its
    /// before-and-after comparison over instead of reading the gap as programs finishing.
    pub activity_epoch: u64,
    /// The server refused `set_theme`, so this host is never sent a theme again.
    pub themes_unsupported: bool,
    /// A theme was declared to this server, by `set_theme` or in a new block's layout.
    pub theme_declared: bool,
}
impl HostModel {
    /// Record a block's owner (none when nobody owns it), in its session's view and its pane.
    fn set_owner(&mut self, block: &str, owner: Option<&str>) {
        for view in self.views.values_mut() {
            for b in view
                .windows
                .iter_mut()
                .flat_map(|w| w.layers.iter_mut())
                .flat_map(|l| l.blocks.iter_mut())
                .filter(|b| b.block_id.as_deref() == Some(block))
            {
                b.owner = owner.map(str::to_owned);
            }
        }
        if let Some(pane) = self.terminals.get(block) {
            pane.model.lock().expect("terminal model").owner = owner == Some(&self.client_id);
        }
    }

    /// Whether Rex applies this client's theme to `block` at once: this client owns it, or
    /// nobody does.
    fn themable(&self, session: &str, block: &str) -> bool {
        self.views.get(session).is_some_and(|view| {
            view.windows
                .iter()
                .flat_map(|w| &w.layers)
                .flat_map(|l| &l.blocks)
                .find(|b| b.block_id.as_deref() == Some(block))
                .is_some_and(|b| b.owner.as_deref().is_none_or(|o| o == self.client_id))
        })
    }

    pub fn apply_block_event(&mut self, session: &str, block: String, event: BlockEvent) {
        match event {
            BlockEvent::StatusChanged { id, record } => {
                self.activity.entry(block).or_default().insert(id, record);
            }
            BlockEvent::StatusRemoved { ids } => {
                if let Some(records) = self.activity.get_mut(&block) {
                    for id in ids {
                        records.remove(&id);
                    }
                    if records.is_empty() {
                        self.activity.remove(&block);
                    }
                }
            }
            BlockEvent::Process { name, pids } => {
                // A progress report belongs to the program in the foreground; one that ends
                // without clearing its report (a crash, a kill) must not leave it on the tab.
                // Another run of the same program has the same name, so its pid counts too;
                // a child taking the foreground leaves its parent's pid in the chain.
                let was = self.programs.get(&block).cloned();
                let left = self
                    .foreground
                    .get(&block)
                    .and_then(|chain| chain.first())
                    .is_some_and(|pid| !pids.contains(pid));
                match name {
                    Some(name) => self.programs.insert(block.clone(), name),
                    None => self.programs.remove(&block),
                };
                if left || self.programs.get(&block) != was.as_ref() {
                    self.progress.remove(&block);
                }
                if pids.is_empty() {
                    self.foreground.remove(&block);
                } else {
                    self.foreground.insert(block, pids);
                }
            }
            BlockEvent::Title { text, owner } => {
                if text.is_empty() {
                    self.titles.remove(&block);
                } else {
                    self.titles.insert(block, (text, owner));
                }
            }
            BlockEvent::Notification { title, body } => {
                // A program spamming notifications can't grow this without bound.
                if self.notices.len() < 32 {
                    self.notices.push(DesktopNotice {
                        session: session.into(),
                        block,
                        title,
                        body,
                    });
                }
            }
            BlockEvent::Progress(Some(progress)) => {
                self.progress.insert(block, progress);
            }
            BlockEvent::Progress(None) => {
                self.progress.remove(&block);
            }
            BlockEvent::Bell => {
                // A program ringing in a loop can't grow this without bound.
                if self.bells.len() < 32 {
                    self.bells.push((session.into(), block));
                }
            }
        }
    }
    fn forget_block(&mut self, block: &str) {
        self.activity.remove(block);
        self.programs.remove(block);
        self.foreground.remove(block);
        self.titles.remove(block);
        self.progress.remove(block);
    }

    /// The block's terminal title while it's current. Shells rarely clear a title when the
    /// program that set it exits, so it counts only while that program, or one it runs under,
    /// is still in the foreground.
    pub fn title(&self, block: &str) -> Option<&str> {
        let (text, owner) = self.titles.get(block)?;
        match (owner, self.foreground.get(block)) {
            (Some(pid), Some(pids)) if !pids.contains(pid) => None,
            _ => Some(text),
        }
    }

    pub fn view(&self) -> Option<&session::ViewResponse> {
        self.views.get(self.selected.as_ref()?)
    }
    pub fn window(&self) -> Option<&session::Window> {
        let view = self.view()?;
        view.windows
            .iter()
            .find(|w| w.window_id == view.active_window_id)
            .or(view.windows.first())
    }
    pub fn focused(&self) -> Option<&str> {
        self.window()?.focused_block_id.as_deref()
    }
}

pub enum HostCommand {
    PerformReply {
        epoch: u64,
        call_id: String,
        outcome: perform::Outcome,
    },
    Keymap(Box<keymap::Declaration>),
    Action {
        generation: u64,
        request: Box<rexrs::api::client::Action>,
    },
    Visibility(bool),
    /// Take size ownership of these blocks, as the native Rex clients do when they attach or
    /// come forward. Failures are silent: another client may have closed a block meanwhile.
    Claim {
        session: String,
        blocks: Vec<String>,
    },
    Select(String),
    /// Select a session and optionally focus a window as one correlated navigation.
    Navigate {
        id: u64,
        /// The UI's deadline, including time spent queued behind other commands.
        deadline: Instant,
        session: String,
        window: Option<String>,
    },
    /// The sessions whose views to load in the background, for listing their tabs; empty
    /// stops loading.
    PrefetchViews(HashSet<String>),
    /// A new session: the server picks a unique label when `label` is none; `shell` starts
    /// its first terminal.
    CreateSession {
        label: Option<String>,
        shell: Option<String>,
    },
    Request {
        method: String,
        payload: Value,
    },
    /// Explicit tab/split creation, inheriting the named source block's shell directory.
    /// A failed or slow read leaves the server's default starting directory unchanged.
    NewTerminal {
        method: String,
        payload: Value,
        cwd_from: String,
    },
    Terminal {
        session: String,
        block: String,
        command: Command,
    },
    /// Upload files to the block's machine; Rex types their paths into it once all arrive.
    Upload {
        session: String,
        block: String,
        files: Vec<crate::upload::File>,
    },
    /// Download a file from the block's machine into a folder here (API revision 13). The
    /// transfer appears in `HostModel::downloads` under the request's id.
    Download(Box<downloads::Request>),
    /// Start the stream of a download that is `AwaitingConfirmation`.
    DownloadConfirm(u64),
    /// Stop a download at any point; its partial file is deleted.
    DownloadCancel(u64),
    Reconnect,
    /// The terminal theme to declare from now on, also while disconnected; none declares
    /// nothing (Follow server).
    Theme(Option<SetTheme>),
    Shutdown,
    Ticket {
        session: String,
        block: String,
        reply: tokio::sync::oneshot::Sender<std::result::Result<TerminalTicket, String>>,
    },
}

// Tokens deliberately have no Debug implementation.
pub struct TerminalTicket {
    token: String,
    bundled: bool,
}

struct PendingTicket {
    token: String,
    expires: Instant,
}

#[derive(Default)]
struct ControlState {
    // Earliest possible arrival of events buffered during the preceding busy step.
    performs_since: Option<Instant>,
    bootstrap_seq: u64,
    list_seq: u64,
    view_seq: HashMap<String, u64>,
    /// Blocks whose program status (and process, if not streamed) were asked for.
    queried: HashSet<String>,
    /// Unattached sessions fetched once with session.view to place their block events.
    viewed: HashSet<String>,
    /// The theme last declared to each block on this connection.
    themed: HashMap<String, SetTheme>,
    // This pool is scoped to the terminal data ref and subprotocol; other sources
    // are filtered out before insertion. It lives only as long as this control socket.
    tickets: HashMap<(String, String), PendingTicket>,
}

impl ControlState {
    fn add_tickets(&mut self, tickets: Vec<session::DataTicket>, started: Instant) {
        for ticket in tickets {
            if ticket.data_ref_id != "terminal"
                || ticket
                    .subprotocol
                    .as_deref()
                    .is_some_and(|p| p != wire::PROTOCOL)
            {
                continue;
            }
            if let Some(expires) = started.checked_add(Duration::from_millis(ticket.expires_in_ms))
            {
                self.tickets.insert(
                    (ticket.session_id, ticket.block_id),
                    PendingTicket {
                        token: ticket.ticket,
                        expires,
                    },
                );
            }
        }
    }

    fn take_ticket(&mut self, session: &str, block: &str) -> Option<TerminalTicket> {
        let ticket = self.tickets.remove(&(session.into(), block.into()))?;
        (Instant::now() < ticket.expires).then_some(TerminalTicket {
            token: ticket.token,
            bundled: true,
        })
    }

    fn prune(&mut self, model: &HostModel) {
        self.view_seq.retain(|id, _| model.views.contains_key(id));
        self.tickets.retain(|(id, block), ticket| {
            Instant::now() < ticket.expires
                && model.views.get(id).is_some_and(|view| {
                    view.windows
                        .iter()
                        .flat_map(|w| &w.layers)
                        .flat_map(|l| &l.blocks)
                        .any(|b| b.block_id.as_deref() == Some(block))
                })
        });
    }

    fn includes(&self, event: &rexrs::Event) -> bool {
        let mut floor = self.bootstrap_seq;
        if matches!(
            event.name.as_str(),
            "session_created" | "session_moved" | "session_destroyed" | "session_label_changed"
        ) {
            floor = floor.max(self.list_seq);
        }
        // Views contain state, not transient clipboard events.
        if event.name != "session_destroyed"
            && !(event.name == "block_event" && clipboard::is_copy(&event.payload))
            && let Some(id) = &event.session_id
        {
            floor = floor.max(self.view_seq.get(id).copied().unwrap_or(0));
        }
        event.seq > floor
    }
}

fn select_session(sessions: &[session::Session], wanted: Option<&str>) -> Option<String> {
    let Some(wanted) = wanted else {
        return sessions.first().map(|s| s.session_id.clone());
    };
    let mut matches = sessions
        .iter()
        .filter(|s| wanted == s.session_id || wanted == s.label);
    let first = matches.next()?;
    matches.next().is_none().then(|| first.session_id.clone())
}

fn session_id(value: &str) -> bool {
    let Some(uuid) = value.strip_prefix("session:") else {
        return false;
    };
    uuid.len() == 36
        && uuid.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

fn bootstrap_request(
    wanted: Option<&str>,
    creating: bool,
    retained: impl Iterator<Item = String>,
) -> session::Bootstrap {
    let mut ids: Vec<_> = retained.collect();
    if let Some(id) = wanted.filter(|s| session_id(s)) {
        ids.push(id.into());
    }
    ids.sort();
    ids.dedup();
    let request = session::Bootstrap::new("all").data_tickets([session::BootstrapDataTicket::new(
        "com.superlogical.terminal",
        "terminal",
    )
    .subprotocol(wire::PROTOCOL)]);
    if wanted.is_some() || creating || !ids.is_empty() {
        request.attach_session_ids(ids)
    } else {
        request
    }
}

pub struct HostConnection {
    pub model: Arc<Mutex<HostModel>>,
    pub commands: flume::Sender<HostCommand>,
    pub wake: flume::Receiver<()>,
}
impl Drop for HostConnection {
    fn drop(&mut self) {
        let _ = self.commands.send(HostCommand::Shutdown);
    }
}

#[derive(Clone)]
struct Shared {
    model: Arc<Mutex<HostModel>>,
    wake: flume::Sender<()>,
}
impl Shared {
    fn update(&self, change: impl FnOnce(&mut HostModel)) {
        change(&mut self.model.lock().expect("host model"));
        let _ = self.wake.try_send(());
    }
    fn status(&self, status: impl Into<String>, connected: bool) {
        self.update(|m| {
            m.status = status.into();
            m.connected = connected;
            if !connected {
                m.clipboard = None;
                m.keymap = None;
                m.keymap_revision += 1;
            }
            for terminal in m.terminals.values() {
                terminal
                    .model
                    .lock()
                    .expect("terminal model")
                    .control_connected = connected;
            }
        });
    }
}

struct PaneShared {
    model: Arc<Mutex<Model>>,
    wake: flume::Sender<()>,
    host_wake: flume::Sender<()>,
}
impl PaneShared {
    fn update(&self, change: impl FnOnce(&mut Model)) {
        let mut m = self.model.lock().expect("terminal model");
        change(&mut m);
        m.revision += 1;
        let _ = self.wake.try_send(());
    }
    fn status(&self, status: impl Into<String>, connected: bool) {
        self.update(|m| {
            m.status = status.into();
            m.connected = connected;
        });
        let _ = self.host_wake.try_send(());
    }
}

enum DataCommand {
    Selection(crate::terminal::selection::Input),
    Visible(bool),
    Scroll(isize),
    Search(String),
    SearchStep(bool),
    Bottom,
    Reconnect,
    Stop,
}
struct PaneWorker {
    session: String,
    commands: flume::Sender<DataCommand>,
    task: tokio::task::JoinHandle<()>,
    size: Option<(u16, u16, u16, u16)>,
}
impl Drop for PaneWorker {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl HostConnection {
    pub fn spawn(
        endpoint: String,
        preferred: Option<String>,
        // A session to create on first connect: its label and the shell it starts.
        create: Option<(String, Option<String>)>,
        block: Option<String>,
        visible: bool,
        // The terminal theme to declare; none declares nothing.
        theme: Option<SetTheme>,
    ) -> Self {
        Self::spawn_labeled(
            endpoint,
            preferred,
            create,
            block,
            visible,
            theme,
            crate::settings::hostname(),
        )
    }

    pub fn spawn_labeled(
        endpoint: String,
        preferred: Option<String>,
        create: Option<(String, Option<String>)>,
        block: Option<String>,
        visible: bool,
        theme: Option<SetTheme>,
        label: String,
    ) -> Self {
        let model = Arc::new(Mutex::new(HostModel {
            visible,
            status: "Connecting…".into(),
            ..Default::default()
        }));
        let (commands, receiver) = flume::unbounded();
        let (wake, notifications) = flume::bounded(1);
        let shared = Shared {
            model: model.clone(),
            wake,
        };
        let sender = commands.clone();
        let startup = Startup {
            preferred,
            create,
            block,
            theme,
            label,
        };
        std::thread::Builder::new()
            .name("rex-host".into())
            .spawn(move || {
                let result = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(anyhow::Error::from)
                    .and_then(|runtime| {
                        tokio::task::LocalSet::new().block_on(
                            &runtime,
                            run_host(endpoint, startup, receiver, sender, &shared),
                        )
                    });
                if let Err(error) = result {
                    shared.status(format!("{error:#}"), false);
                }
            })
            .expect("start host worker");
        Self {
            model,
            commands,
            wake: notifications,
        }
    }
}

async fn request(control: &mut Client, method: &str, payload: Value) -> Result<Value> {
    Ok(
        tokio::time::timeout(Duration::from_secs(8), control.call_raw(method, payload))
            .await
            .context("Rex request timed out; action was not retried")??,
    )
}

/// Read on the server, not this machine or an OSC 7 cache. Bound the optional read so a
/// missing/unsupported process query cannot hold up creation indefinitely.
async fn shell_dir_of(control: &mut Client, session: &str, block: &str) -> Option<String> {
    let method = format!("{}.process", activity::TERMINAL);
    let args = json!({"session_id":session,"block_id":block,"args":{}});
    let reply = tokio::time::timeout(Duration::from_secs(1), control.call_raw(&method, args));
    activity::shell_dir(&reply.await.ok()?.ok()?)
}

async fn attach(
    control: &mut Client,
    shared: &Shared,
    state: &mut ControlState,
    id: &str,
) -> Result<()> {
    shared.update(|m| m.protocol.attach_requests += 1);
    let reply = tokio::time::timeout(
        Duration::from_secs(8),
        control.call_with_seq(session::Attach::new(id)),
    )
    .await??;
    state.view_seq.insert(id.into(), reply.seq);
    apply_view(shared, id, reply.value.view);
    Ok(())
}
fn apply_view(shared: &Shared, id: &str, view: session::ViewResponse) {
    shared.update(|m| {
        if m.views
            .get(id)
            .is_some_and(|old| old.revision > view.revision)
        {
            return;
        }
        if let Some(label) = &view.label
            && let Some(s) = m.sessions.iter_mut().find(|s| s.session_id == id)
        {
            s.label = label.clone();
        }
        for block in view
            .windows
            .iter()
            .flat_map(|w| &w.layers)
            .flat_map(|l| &l.blocks)
        {
            if let Some(connection) = block.block_id.as_ref().and_then(|id| m.terminals.get(id)) {
                connection.model.lock().expect("terminal model").owner =
                    block.owner.as_deref() == Some(&m.client_id);
            }
        }
        m.views.insert(id.into(), view);
    });
}
async fn refresh(
    control: &mut Client,
    shared: &Shared,
    state: &mut ControlState,
    id: &str,
) -> Result<()> {
    let reply = tokio::time::timeout(
        Duration::from_secs(8),
        control.call_with_seq(session::View::new(id)),
    )
    .await??;
    state.view_seq.insert(id.into(), reply.seq);
    apply_view(shared, id, reply.value);
    Ok(())
}
async fn list(control: &mut Client, shared: &Shared, state: &mut ControlState) -> Result<()> {
    let reply = tokio::time::timeout(
        Duration::from_secs(8),
        control.call_with_seq(session::AttachAll::new("all")),
    )
    .await??;
    state.list_seq = reply.seq;
    shared.update(|m| {
        m.sessions = reply.value.sessions;
        m.views
            .retain(|id, _| m.sessions.iter().any(|s| &s.session_id == id));
        if m.selected
            .as_ref()
            .is_some_and(|id| !m.sessions.iter().any(|s| &s.session_id == id))
        {
            m.selected = None;
        }
    });
    Ok(())
}

fn reconcile(
    endpoint: &str,
    shared: &Shared,
    sender: &flume::Sender<HostCommand>,
    workers: &mut HashMap<String, PaneWorker>,
) {
    let model = shared.model.lock().expect("host model").clone();
    // A block can disappear while control is disconnected, or before its
    // block_closed event is drained. The current view is authoritative.
    let stale: Vec<_> = workers
        .iter()
        .filter(|(id, worker)| {
            !model
                .sessions
                .iter()
                .any(|s| s.session_id == worker.session)
                || model.views.get(&worker.session).is_some_and(|view| {
                    !view
                        .windows
                        .iter()
                        .flat_map(|w| &w.layers)
                        .flat_map(|l| &l.blocks)
                        .any(|b| b.block_id.as_ref() == Some(id))
                })
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in stale {
        workers.remove(&id);
        shared.update(|m| {
            m.terminals.remove(&id);
        });
    }
    let mut visible = HashSet::new();
    if model.visible
        && let (Some(session_id), Some(window)) = (&model.selected, model.window())
    {
        for block in window.layers.iter().flat_map(|l| &l.blocks) {
            let Some(id) = &block.block_id else {
                continue;
            };
            if block.creator_name.as_deref() != Some("com.superlogical.terminal") {
                continue;
            }
            if window
                .zoomed_block_id
                .as_ref()
                .is_some_and(|zoom| zoom != id)
            {
                continue;
            }
            visible.insert(id.clone());
            if workers.contains_key(id) {
                continue;
            }
            let state = Arc::new(Mutex::new(Model {
                status: "Restoring terminal…".into(),
                session: session_id.clone(),
                block: id.clone(),
                owner: block.owner.as_deref() == Some(&model.client_id),
                control_connected: model.connected,
                ..Default::default()
            }));
            let (wake, receiver) = flume::bounded(1);
            let connection = Connection {
                model: state.clone(),
                commands: TerminalSender::new(sender.clone(), session_id.clone(), id.clone()),
                wake: receiver,
            };
            shared.update(|m| {
                m.terminals.insert(id.clone(), connection);
            });
            let pane = PaneShared {
                model: state,
                wake,
                host_wake: shared.wake.clone(),
            };
            let (commands, data_commands) = flume::unbounded();
            let task = tokio::task::spawn_local(run_data(
                endpoint.to_owned(),
                session_id.clone(),
                id.clone(),
                sender.clone(),
                data_commands,
                pane,
            ));
            workers.insert(
                id.clone(),
                PaneWorker {
                    session: session_id.clone(),
                    commands,
                    task,
                    size: None,
                },
            );
        }
    }
    for (id, worker) in workers {
        let _ = worker
            .commands
            .send(DataCommand::Visible(visible.contains(id)));
    }
}

fn view_blocks(view: &session::ViewResponse) -> impl Iterator<Item = String> + '_ {
    view.windows
        .iter()
        .flat_map(|w| &w.layers)
        .flat_map(|l| &l.blocks)
        .filter(|b| b.creator_name.as_deref() == Some(activity::TERMINAL))
        .filter_map(|b| b.block_id.clone())
}

/// Ask once per block of the selected session what its programs last reported, its title,
/// and which program runs in the blocks that aren't streamed (streams get process_changed
/// anyway).
/// A failed query leaves the block without activity; it isn't retried.
async fn query_activity(
    control: &mut Client,
    shared: &Shared,
    state: &mut ControlState,
    workers: &HashMap<String, PaneWorker>,
) {
    let (session, blocks) = {
        let m = shared.model.lock().expect("host model");
        if !m.connected {
            return;
        }
        let Some(view) = m.view() else { return };
        let Some(session) = m.selected.clone() else {
            return;
        };
        let blocks: Vec<_> = view_blocks(view)
            .filter(|b| !state.queried.contains(b))
            .collect();
        (session, blocks)
    };
    for block in blocks {
        state.queried.insert(block.clone());
        let target = json!({"session_id":session, "block_id":block, "args":{}});
        let method = format!("{}.program_status", activity::TERMINAL);
        if let Ok(reply) = request(control, &method, target.clone()).await {
            let records = activity::parse_status(&reply);
            shared.update(|m| {
                m.activity.remove(&block);
                if !records.is_empty() {
                    m.activity
                        .insert(block.clone(), records.into_iter().collect());
                }
            });
        }
        let method = format!("{}.title", activity::TERMINAL);
        if let Ok(reply) = request(control, &method, target.clone()).await {
            let title = activity::parse_title(&reply);
            shared.update(|m| m.apply_block_event(&session, block.clone(), title));
        }
        if workers.contains_key(&block) {
            continue;
        }
        let method = format!("{}.process", activity::TERMINAL);
        if let Ok(reply) = request(control, &method, target).await {
            let process = activity::parse_process(&reply);
            shared.update(|m| m.apply_block_event(&session, block, process));
        }
    }
}

/// Declare `theme` to each streamed block Rex would apply it to at once: blocks this client
/// owns or nobody owns. A block another client owns is left alone; Rex would hold the theme
/// until this client takes it, and taking it brings the block here again. Under Follow server
/// (`theme` none), a block this connection declared to gets one empty theme, Rex's own
/// colours, so the last declaration doesn't stay behind. Repeats are skipped.
async fn declare_themes(
    control: &mut Client,
    shared: &Shared,
    state: &mut ControlState,
    workers: &HashMap<String, PaneWorker>,
    theme: Option<&SetTheme>,
) {
    let due: Vec<_> = {
        let m = shared.model.lock().expect("host model");
        if m.themes_unsupported || !m.connected {
            return;
        }
        workers
            .iter()
            .filter_map(|(block, worker)| {
                let last = state.themed.get(block);
                let wanted = match theme {
                    Some(theme) => theme.clone(),
                    None if last.is_some_and(|t| *t != SetTheme::new()) => SetTheme::new(),
                    None => return None,
                };
                (last != Some(&wanted) && m.themable(&worker.session, block))
                    .then(|| (worker.session.clone(), block.clone(), wanted))
            })
            .collect()
    };
    for (session, block, wanted) in due {
        let call = control.call(wanted.clone().on(session, block.clone()));
        match tokio::time::timeout(Duration::from_secs(8), call).await {
            Ok(Err(rexrs::Error::Api(
                rexrs::ApiError::MethodNotFound { .. } | rexrs::ApiError::NotSupported { .. },
            ))) => {
                shared.update(|m| {
                    m.themes_unsupported = true;
                    m.notice = Some(
                        "This Rex server doesn't take terminal themes; its panes keep the server's colours."
                            .into(),
                    );
                });
                return;
            }
            // A block that closed meanwhile or a slow reply isn't retried on this connection.
            _ => {
                state.themed.insert(block, wanted);
                shared.update(|m| m.theme_declared = true);
            }
        }
    }
}

fn remove_session(shared: &Shared, workers: &mut HashMap<String, PaneWorker>, id: &str) {
    let blocks: Vec<_> = workers
        .iter()
        .filter(|(_, w)| w.session == id)
        .map(|(id, _)| id.clone())
        .collect();
    for block in &blocks {
        workers.remove(block);
    }
    shared.update(|m| {
        m.sessions.retain(|s| s.session_id != id);
        if let Some(view) = m.views.remove(id) {
            for block in view_blocks(&view) {
                m.forget_block(&block);
            }
        }
        for block in blocks {
            m.terminals.remove(&block);
        }
        if m.selected.as_deref() == Some(id) {
            m.selected = None;
        }
    });
}

fn apply_bootstrap(
    shared: &Shared,
    state: &mut ControlState,
    reply: rexrs::Reply<session::BootstrapResponse>,
    started: Instant,
) {
    state.bootstrap_seq = reply.seq;
    state.list_seq = reply.seq;
    let boot = reply.value;
    shared.update(|m| m.sessions = boot.attach_all.sessions);
    for attached in boot
        .attached_sessions
        .into_iter()
        .chain(boot.selected_session.map(|s| session::AttachedSession {
            session: s.session,
            view: s.view,
        }))
    {
        state
            .view_seq
            .insert(attached.session.session_id.clone(), reply.seq);
        apply_view(shared, &attached.session.session_id, attached.view);
    }
    state.add_tickets(boot.data_tickets, started);
}

async fn handle_event(
    event: rexrs::Event,
    control: &mut Client,
    shared: &Shared,
    workers: &mut HashMap<String, PaneWorker>,
    state: &mut ControlState,
    keymap: Option<&keymap::Declaration>,
) -> Result<()> {
    // This is a directed request, not a session update: even an unattached session or a
    // sequence below bootstrap's cutoff needs an answer on this same connection.
    if event.name == "client_perform_requested" {
        return perform::receive(control, shared, event.payload, state.performs_since).await;
    }
    // Config-wide events have no session sequence. Do not broaden includes().
    if event.name == "keymap_changed" {
        let refresh = {
            let m = shared.model.lock().expect("host model");
            m.connected && keymap::changed(m.keymap.as_deref(), &event.payload)
        };
        if refresh && let Some(declaration) = keymap {
            keymap::refresh(control, shared, declaration).await;
        }
        return Ok(());
    }
    if !state.includes(&event) {
        return Ok(());
    }
    match event.name.as_str() {
        "session_created" | "session_moved" => {
            list(control, shared, state).await?;
        }
        "session_destroyed" => {
            if let Some(id) = event.payload["session_id"]
                .as_str()
                .or(event.session_id.as_deref())
            {
                remove_session(shared, workers, id);
            }
        }
        "session_label_changed" => {
            if let Some(id) = event.session_id.as_deref()
                && let Some(label) = event.payload["label"].as_str()
            {
                shared.update(|m| {
                    if let Some(s) = m.sessions.iter_mut().find(|s| s.session_id == id) {
                        s.label = label.into();
                    }
                });
            }
        }
        "session_view_changed" => {
            if let Some(id) = event.session_id.as_deref() {
                let revision = shared
                    .model
                    .lock()
                    .expect("host model")
                    .views
                    .get(id)
                    .map(|v| v.revision);
                if revision.is_some_and(|revision| {
                    event.payload["revision"]
                        .as_u64()
                        .is_some_and(|new| new > revision)
                }) {
                    refresh(control, shared, state, id).await?;
                }
            }
        }
        "block_closed" => {
            if let Some(block) = event.payload["block_id"].as_str() {
                if let Some(worker) = workers.remove(block) {
                    let _ = worker.commands.send(DataCommand::Stop);
                }
                shared.update(|m| {
                    m.terminals.remove(block);
                    m.forget_block(block);
                });
            }
            if let Some(id) = event.session_id.as_deref()
                && shared
                    .model
                    .lock()
                    .expect("host model")
                    .views
                    .contains_key(id)
            {
                refresh(control, shared, state, id).await?;
            }
        }
        "block_event" => {
            if clipboard::is_copy(&event.payload) {
                if let (Some(session), Some(block)) = (
                    event.session_id.as_deref(),
                    event.payload["block_id"].as_str(),
                ) {
                    let accepts = clipboard::accepts(
                        &shared.model.lock().expect("host model"),
                        session,
                        block,
                    );
                    if accepts {
                        match clipboard::read(control, session, &event.payload).await {
                            Ok(copy) => shared.update(|m| {
                                if clipboard::accepts(m, &copy.session, &copy.block) {
                                    m.clipboard = Some(copy);
                                }
                            }),
                            Err(_) => shared.update(|m| {
                                m.notice = Some("Copy failed: terminal clipboard text could not be read (limit 8 MiB).".into());
                            }),
                        }
                    }
                }
                return Ok(());
            }
            let (Some(id), Some((block, change))) = (
                event.session_id.as_deref(),
                activity::parse_block_event(&event.payload),
            ) else {
                return Ok(());
            };
            let unplaced = {
                let m = shared.model.lock().expect("host model");
                !m.views.contains_key(id) && m.sessions.iter().any(|s| s.session_id == id)
            };
            shared.update(|m| m.apply_block_event(id, block, change));
            // Read-only, and allowed for any session once attach_all is active. Once per
            // connection: a failure leaves that session's tabs without activity.
            if unplaced && state.viewed.insert(id.to_owned()) {
                let _ = refresh(control, shared, state, id).await;
            }
        }
        // Becoming the owner, even without a resize, makes the block themable.
        "block_owner_changed" => {
            if let Some(block) = event.payload["block_id"].as_str() {
                shared.update(|m| m.set_owner(block, event.payload["owner"].as_str()));
            }
        }
        _ => {}
    }
    Ok(())
}

/// What a host connection starts with.
struct Startup {
    label: String,
    preferred: Option<String>,
    create: Option<(String, Option<String>)>,
    block: Option<String>,
    theme: Option<SetTheme>,
}

async fn run_host(
    configured: String,
    startup: Startup,
    commands: flume::Receiver<HostCommand>,
    sender: flume::Sender<HostCommand>,
    shared: &Shared,
) -> Result<()> {
    let Startup {
        mut preferred,
        mut create,
        block: mut initial_block,
        mut theme,
        label,
    } = startup;
    let mut token: Option<String> = None;
    let mut workers = HashMap::<String, PaneWorker>::new();
    let mut retry = 0;
    let mut prefetch = prefetch::Prefetch::default();
    let mut keymap: Option<Box<keymap::Declaration>> = None;
    // An ssh:// host is reached through a forward of its Rex socket; every attempt opens a
    // fresh one, and its end is a disconnect like any other.
    let ssh = crate::ssh::Target::parse(&configured)?;
    let mut tunnel: Option<crate::ssh::Tunnel> = None;
    loop {
        let result: Result<bool> = async {
            let endpoint = match &ssh {
                Some(target) => {
                    shared.status("Connecting over SSH…", false);
                    let opened = crate::ssh::Tunnel::open(target).await?;
                    let local = opened.endpoint().to_owned();
                    tunnel = Some(opened);
                    local
                }
                None => configured.clone(),
            };
            let mut control = transport::control_with_label(&endpoint, token.as_deref(), &label).await?;
            let identity = control.welcome().client.as_ref().context("Rex client identity")?;
            token = Some(identity.reconnect_token.clone());
            let wanted = preferred.clone().or_else(|| shared.model.lock().expect("host model").selected.clone());
            shared.update(|m| {
                m.connection_epoch += 1;
                m.performs.clear();
                m.client_id = identity.id.clone();
                m.protocol.api_revision = control.welcome().api_revision;
                m.views.clear();
                // Reports made while disconnected were missed; queries refill what's shown.
                m.activity.clear();
                m.programs.clear();
                m.foreground.clear();
                m.titles.clear();
                m.progress.clear();
                m.activity_epoch += 1;
            });
            let mut state = ControlState::default();
            let mut downloads = downloads::Downloads::default();
            let mut busy_since = Some(Instant::now());
            let boot = bootstrap_request(wanted.as_deref(), create.is_some(), workers.values().map(|w| w.session.clone()));
            let started = Instant::now();
            shared.update(|m| m.protocol.bootstrap_requests += 1);
            let result = tokio::time::timeout(Duration::from_secs(8), control.call_with_seq(boot)).await;
            let reply = match result {
                Ok(Ok(reply)) => reply,
                error => {
                    // Setup may be partially applied. A graceful close ends the identity;
                    // never reuse its token or continue this partially initialized socket.
                    token = None;
                    let _ = tokio::time::timeout(Duration::from_secs(2), control.close()).await;
                    return match error {
                        Ok(Err(error)) => Err(error.into()),
                        Err(error) => Err(error.into()),
                        _ => unreachable!(),
                    };
                }
            };
            apply_bootstrap(shared, &mut state, reply, started);
            if let Some((label, shell)) = create.take() {
                // Take before calling: an uncertain response must never create twice.
                let layout = crate::layout::shell_layout(shell.as_deref(), new_block_theme(shared, theme.as_ref()));
                let created = request(&mut control, "session.create", json!({"label":label,"initial_windows":[{"layout":layout}]})).await?;
                preferred = created["session_id"].as_str().map(str::to_owned);
                list(&mut control, shared, &mut state).await?;
            }
            let selected = {
                let model = shared.model.lock().expect("host model");
                let wanted = preferred.clone().or(wanted);
                select_session(&model.sessions, wanted.as_deref())
            };
            if selected.is_some() { preferred = None; }
            shared.update(|m| m.selected = selected.clone());
            let session_ids: HashSet<_> = shared.model.lock().expect("host model").sessions.iter().map(|s| s.session_id.clone()).collect();
            let stale: HashSet<_> = workers.values().filter(|w| !session_ids.contains(&w.session)).map(|w| w.session.clone()).collect();
            for id in stale { remove_session(shared, &mut workers, &id); }
            let mut attached: HashSet<String> = workers.values().map(|w| w.session.clone()).collect();
            if let Some(id) = &selected { attached.insert(id.clone()); }
            for id in attached {
                if !state.view_seq.contains_key(&id) { attach(&mut control, shared, &mut state, &id).await?; }
            }
            if let (Some(id), Some(block)) = (&selected, initial_block.take()) {
                request(&mut control, "session.focus_block", json!({"session_id":id,"block_id":block})).await?;
                refresh(&mut control, shared, &mut state, id).await?;
            }
            state.performs_since = busy_since;
            let queued: Vec<_> = control.drain_events().collect();
            for event in queued { handle_event(event, &mut control, shared, &mut workers, &mut state, keymap.as_deref()).await?; }
            state.prune(&shared.model.lock().expect("host model"));
            shared.update(|m| m.ssh_server_missing = false);
            shared.status("Connected", true);
            reconcile(&endpoint, shared, &sender, &mut workers);
            for (block, worker) in &workers {
                if let Some((cols, rows, w, h)) = worker.size { resize(&mut control, &worker.session, block, cols, rows, w, h).await?; }
            }
            reconcile(&endpoint, shared, &sender, &mut workers);
            declare_themes(&mut control, shared, &mut state, &workers, theme.as_ref()).await;
            query_activity(&mut control, shared, &mut state, &workers).await;
            // Restore terminal sizes and state before a best-effort keymap can wait.
            if let Some(declaration) = &keymap { keymap::refresh(&mut control, shared, declaration).await; }
            retry = 0;
            prefetch.reset();
            let mut pace = tokio::time::interval(prefetch::PACE);
            pace.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            let mut expiry = tokio::time::interval(Duration::from_millis(100));
            loop {
                state.performs_since = state.performs_since.or(busy_since.take());
                if state.performs_since.is_some() {
                    // Buffered requests share the busy step's start, even when it took less
                    // than MAX_WAIT. They must not gain another five seconds in the UI queue.
                    use futures_util::FutureExt;
                    if let Some(event) = control.next_event().now_or_never() {
                        handle_event(event?, &mut control, shared, &mut workers, &mut state, keymap.as_deref()).await?;
                        state.prune(&shared.model.lock().expect("host model"));
                        reconcile(&endpoint, shared, &sender, &mut workers);
                        declare_themes(&mut control, shared, &mut state, &workers, theme.as_ref()).await;
                        query_activity(&mut control, shared, &mut state, &workers).await;
                        continue;
                    }
                    state.performs_since = None;
                }
                let due = prefetch.due(&shared.model.lock().expect("host model"), Instant::now());
                tokio::select! {
                    _ = expiry.tick() => {
                        busy_since = Some(Instant::now());
                        perform::expire(&mut control, shared).await?;
                    }
                    _ = prefetch::wait(&mut pace, due.unwrap_or_else(Instant::now)), if due.is_some() => {
                        busy_since = Some(Instant::now());
                        let next = prefetch.next(&shared.model.lock().expect("host model"), Instant::now());
                        if let Some(id) = next {
                            prefetch.start(&mut control, id).await?;
                        }
                    }
                    (id, text) = prefetch.finished(), if prefetch.in_flight() => {
                        if let Some((seq, view)) = prefetch.result(&id, text, Instant::now()) {
                            let placed = {
                                let m = shared.model.lock().expect("host model");
                                m.views.contains_key(&id) || !m.sessions.iter().any(|s| s.session_id == id)
                            };
                            if !placed {
                                state.view_seq.insert(id.clone(), seq);
                                apply_view(shared, &id, view);
                            }
                        }
                    }
                    event = control.next_event() => {
                        busy_since = Some(Instant::now());
                        handle_event(event?, &mut control, shared, &mut workers, &mut state, keymap.as_deref()).await?;
                        state.prune(&shared.model.lock().expect("host model"));
                        reconcile(&endpoint, shared, &sender, &mut workers);
                        declare_themes(&mut control, shared, &mut state, &workers, theme.as_ref()).await;
                        query_activity(&mut control, shared, &mut state, &workers).await;
                    }
                    error = crate::ssh::closed(&mut tunnel) => return Err(error),
                    command = commands.recv_async() => {
                        busy_since = Some(Instant::now());
                        let Ok(command) = command else { control.close().await?; return Ok(true); };
                        if matches!(command, HostCommand::Shutdown) { control.close().await?; return Ok(true); }
                        if matches!(command, HostCommand::Reconnect) { return Ok(false); }
                        if let HostCommand::Keymap(new) = command {
                            keymap::refresh(&mut control, shared, &new).await;
                            keymap = Some(new);
                            continue;
                        }
                        if let HostCommand::PrefetchViews(sessions) = command { prefetch.set_eligible(sessions); continue; }
                        if let HostCommand::Upload { session, block, files } = command {
                            if let Err(error) = upload(&mut control, shared, &endpoint, &session, &block, files).await {
                                shared.update(|m| m.notice = Some(format!("Upload failed: {error:#}")));
                                if !matches!(error.downcast_ref::<rexrs::Error>(), Some(rexrs::Error::Api(_)) | None) { return Err(error); }
                            }
                            continue;
                        }
                        if let HostCommand::Download(request) = command {
                            downloads.start(&mut control, shared, &endpoint, *request).await?;
                            continue;
                        }
                        if let HostCommand::DownloadConfirm(id) = command { downloads.signal(id, downloads::Signal::Confirm); continue; }
                        if let HostCommand::DownloadCancel(id) = command { downloads.signal(id, downloads::Signal::Cancel); continue; }
                        if let HostCommand::Navigate { id, deadline, .. } = &command
                            && Instant::now() >= *deadline
                        {
                            shared.update(|m| {
                                m.navigation_result = Some((*id, false));
                                m.notice = Some("Navigation expired before it could be sent. Try again.".into());
                            });
                            continue;
                        }
                        if matches!(command, HostCommand::Select(_) | HostCommand::Navigate { .. } | HostCommand::CreateSession { .. }) { preferred = None; }
                        let navigation = match &command { HostCommand::Navigate { id, .. } => Some(*id), _ => None };
                        let outcome = handle_command(command, &mut control, shared, &mut workers, &mut state, &mut theme).await;
                        if let Some(id) = navigation {
                            shared.update(|m| m.navigation_result = Some((id, outcome.is_ok())));
                        }
                        if let Err(error) = outcome {
                            shared.update(|m| m.notice = Some(format!("{error:#}")));
                            if !matches!(error.downcast_ref::<rexrs::Error>(), Some(rexrs::Error::Api(_))) { return Err(error); }
                        }
                        state.prune(&shared.model.lock().expect("host model"));
                        reconcile(&endpoint, shared, &sender, &mut workers);
                        declare_themes(&mut control, shared, &mut state, &workers, theme.as_ref()).await;
                        query_activity(&mut control, shared, &mut state, &workers).await;
                    }
                }
            }
        }.await;
        tunnel = None;
        if matches!(result, Ok(true)) {
            return Ok(());
        }
        shared.update(|m| {
            m.performs.clear();
            m.ssh_server_missing = result
                .as_ref()
                .err()
                .is_some_and(|e| e.downcast_ref::<crate::ssh::MissingServer>().is_some())
        });
        shared.status(
            match result {
                Err(e) => format!("Disconnected · {e:#}"),
                _ => "Reconnecting…".into(),
            },
            false,
        );
        retry += 1;
        let delay = tokio::time::sleep(Duration::from_secs((1_u64 << retry.min(3)).min(8)));
        tokio::pin!(delay);
        loop {
            tokio::select! {
                _ = &mut delay => break,
                command = commands.recv_async() => match command {
                    Ok(HostCommand::Shutdown) | Err(_) => return Ok(()),
                    Ok(HostCommand::Reconnect) => break,
                    Ok(HostCommand::Select(id)) => preferred = Some(id),
                    Ok(HostCommand::Navigate { id, .. }) => reject_navigation(shared, id),
                    Ok(HostCommand::Visibility(visible)) => shared.update(|m|m.visible=visible),
                    Ok(HostCommand::PrefetchViews(sessions)) => prefetch.set_eligible(sessions),
                    Ok(HostCommand::Theme(new)) => theme = new,
                    Ok(HostCommand::Keymap(new)) => keymap = Some(new),
                    Ok(HostCommand::Ticket { reply, .. }) => { let _ = reply.send(Err("Host is reconnecting".into())); }
                    Ok(HostCommand::Claim { .. } | HostCommand::PerformReply { .. }) => {}
                    Ok(HostCommand::Download(request)) => downloads::refuse(shared, *request, crate::download::Failure::Disconnected),
                    Ok(HostCommand::DownloadConfirm(_) | HostCommand::DownloadCancel(_)) => {}
                    Ok(HostCommand::Terminal { block, command: Command::Selection(input), .. }) => {
                        if let Some(worker) = workers.get(&block) { let _ = worker.commands.send(DataCommand::Selection(input)); }
                    }
                    Ok(HostCommand::Terminal { block, command: Command::Resize(a,b,c,d), .. }) => {
                        if let Some(worker) = workers.get_mut(&block) { worker.size = Some((a,b,c,d)); }
                    }
                    Ok(HostCommand::Terminal { block, command, .. }) if forward_search(&workers, &block, &command) => {}
                    Ok(_) => shared.update(|m| m.notice = Some("Action not sent while disconnected. Try it again after reconnecting.".into())),
                }
            }
        }
        // Discard input already queued before a failure; never replay it after reconnect.
        while let Ok(command) = commands.try_recv() {
            match command {
                HostCommand::Shutdown => return Ok(()),
                HostCommand::Select(id) => preferred = Some(id),
                HostCommand::Navigate { id, .. } => reject_navigation(shared, id),
                HostCommand::Visibility(visible) => shared.update(|m| m.visible = visible),
                HostCommand::PrefetchViews(sessions) => prefetch.set_eligible(sessions),
                HostCommand::Theme(new) => theme = new,
                HostCommand::Keymap(new) => keymap = Some(new),
                HostCommand::Ticket { reply, .. } => {
                    let _ = reply.send(Err("Host is reconnecting".into()));
                }
                HostCommand::Terminal { block, command, .. } => {
                    forward_search(&workers, &block, &command);
                }
                HostCommand::Download(request) => {
                    downloads::refuse(shared, *request, crate::download::Failure::Disconnected)
                }
                _ => {}
            }
        }
    }
}

fn reject_navigation(shared: &Shared, id: u64) {
    shared.update(|m| {
        m.navigation_result = Some((id, false));
        m.notice =
            Some("Navigation not sent while disconnected. Try again after reconnecting.".into());
    });
}

/// A search runs on the pane's own terminal, so it needs no server: it reaches the pane's worker
/// even while the host reconnects, or closing a search then would leave its highlights behind.
/// False for any other command.
fn forward_search(workers: &HashMap<String, PaneWorker>, block: &str, command: &Command) -> bool {
    let command = match command {
        Command::Search(needle) => DataCommand::Search(needle.clone()),
        Command::SearchStep(older) => DataCommand::SearchStep(*older),
        _ => return false,
    };
    if let Some(worker) = workers.get(block) {
        let _ = worker.commands.send(command);
    }
    true
}

/// Reserves every file of one drop or paste as a batch, then streams them off the control
/// loop. Nothing is retried: a failure leaves nothing typed and nothing on the server.
async fn upload(
    control: &mut Client,
    shared: &Shared,
    endpoint: &str,
    session: &str,
    block: &str,
    files: Vec<crate::upload::File>,
) -> Result<()> {
    use crate::upload;
    if files.is_empty() {
        return Ok(());
    }
    anyhow::ensure!(
        control.welcome().api_revision >= upload::MIN_API_REVISION,
        "this host's Rex server is too old to receive files; update Rex there"
    );
    let sizes = files
        .iter()
        .map(upload::File::size)
        .collect::<Result<Vec<_>>>()?;
    let batch = if files.len() > 1 {
        Some(upload::new_uuid()?)
    } else {
        None
    };
    let mut tickets = Vec::with_capacity(files.len());
    for (index, (file, size)) in files.iter().zip(&sizes).enumerate() {
        let mut begin = rexrs::api::terminal::UploadBegin::new(
            format!("upload:{}", upload::new_uuid()?),
            file.name.clone(),
            i64::try_from(*size).context("file too large")?,
        );
        if let Some(batch) = &batch {
            begin = begin
                .batch(batch.clone())
                .index(index as i64)
                .count(files.len() as i64);
        }
        let reply = tokio::time::timeout(
            Duration::from_secs(8),
            control.call(begin.on(session.to_owned(), block.to_owned())),
        )
        .await
        .context("Rex request timed out; upload was not retried")??;
        tickets.push(reply.ticket);
    }
    let shared = shared.clone();
    let endpoint = endpoint.to_owned();
    tokio::task::spawn_local(async move {
        let sends = files
            .iter()
            .zip(&tickets)
            .zip(&sizes)
            .map(|((file, ticket), size)| upload::send(&endpoint, ticket, file, *size));
        let failed = futures_util::future::join_all(sends)
            .await
            .into_iter()
            .find_map(Result::err);
        if let Some(error) = failed {
            shared.update(|m| m.notice = Some(format!("Upload failed: {error:#}")));
        }
    });
    Ok(())
}

/// Clear a successful request's notice and reconcile its session, which may have ended.
async fn after_request(
    control: &mut Client,
    shared: &Shared,
    workers: &mut HashMap<String, PaneWorker>,
    state: &mut ControlState,
    session: Option<&str>,
) -> Result<()> {
    shared.update(|m| m.notice = None);
    if let Some(id) = session {
        // Destroying a session, or closing its last tab, ends it: nothing to re-read.
        match refresh(control, shared, state, id).await {
            Err(error)
                if matches!(
                    error.downcast_ref::<rexrs::Error>(),
                    Some(rexrs::Error::Api(rexrs::ApiError::SessionNotFound { .. }))
                ) =>
            {
                remove_session(shared, workers, id)
            }
            result => result?,
        }
    }
    Ok(())
}

async fn handle_command(
    command: HostCommand,
    control: &mut Client,
    shared: &Shared,
    workers: &mut HashMap<String, PaneWorker>,
    state: &mut ControlState,
    theme: &mut Option<SetTheme>,
) -> Result<()> {
    match command {
        HostCommand::Action {
            generation,
            request,
        } => actions::run(control, shared, generation, *request).await?,
        HostCommand::PerformReply {
            epoch,
            call_id,
            outcome,
        } => {
            if shared.model.lock().expect("host model").connection_epoch == epoch {
                perform::reply(control, &call_id, &outcome).await?;
            }
        }
        HostCommand::Visibility(visible) => shared.update(|m| m.visible = visible),
        // Declared to every block it is due for right after this command.
        HostCommand::Theme(new) => *theme = new,
        HostCommand::Claim { session, blocks } => {
            if blocks.is_empty() {
                return Ok(());
            }
            let Ok(result) = request(
                control,
                "session.own_block",
                json!({"session_id":session,"block_ids":blocks,"mode":"take"}),
            )
            .await
            else {
                return Ok(());
            };
            shared.update(|m| {
                for b in result["blocks"].as_array().into_iter().flatten() {
                    if let Some(id) = b["block_id"].as_str() {
                        m.set_owner(id, b["owner"].as_str());
                    }
                }
            });
        }
        HostCommand::Navigate {
            session, window, ..
        } => {
            let selected = shared.model.lock().expect("host model").selected.clone();
            if selected.as_ref() != Some(&session) {
                attach(control, shared, state, &session).await?;
            }
            if let Some(window) = window {
                request(
                    control,
                    "session.focus_window",
                    json!({"session_id":session,"window_id":window}),
                )
                .await?;
                refresh(control, shared, state, &session).await?;
            }
            shared.update(|m| {
                m.selected = Some(session);
                m.notice = None;
            });
        }
        HostCommand::Select(id) => {
            attach(control, shared, state, &id).await?;
            shared.update(|m| {
                m.selected = Some(id);
                m.notice = None;
            });
        }
        HostCommand::CreateSession { label, shell } => {
            let layout = crate::layout::shell_layout(
                shell.as_deref(),
                new_block_theme(shared, theme.as_ref()),
            );
            let mut payload =
                json!({"initial_windows":[{"window_label":DEFAULT_TAB_LABEL,"layout":layout}]});
            if let Some(label) = label.filter(|l| !l.trim().is_empty()) {
                payload["label"] = json!(label);
            }
            let created = request(control, "session.create", payload).await?;
            let id = created["session_id"].as_str().context("new session ID")?;
            list(control, shared, state).await?;
            attach(control, shared, state, id).await?;
            shared.update(|m| {
                m.selected = Some(id.into());
                m.notice = None;
            });
        }
        HostCommand::Request { method, payload } => {
            let session_id = payload["session_id"].as_str().map(str::to_owned);
            request(control, &method, payload).await?;
            after_request(control, shared, workers, state, session_id.as_deref()).await?;
        }
        HostCommand::NewTerminal {
            method,
            mut payload,
            cwd_from,
        } => {
            let session = payload["session_id"]
                .as_str()
                .context("new terminal session")?
                .to_owned();
            if let Some(dir) = shell_dir_of(control, &session, &cwd_from).await {
                crate::layout::with_cwd(&mut payload["layout"], &dir);
            }
            // Creation is sent once. A failure follows the ordinary request path; never replay.
            request(control, &method, payload).await?;
            after_request(control, shared, workers, state, Some(&session)).await?;
        }
        HostCommand::Ticket {
            session,
            block,
            reply,
        } => {
            state.prune(&shared.model.lock().expect("host model"));
            if let Some(ticket) = state.take_ticket(&session, &block) {
                if reply.send(Ok(ticket)).is_ok() {
                    shared.update(|m| m.protocol.bundled_tickets_used += 1);
                }
                return Ok(());
            }
            shared.update(|m| m.protocol.data_connect_requests += 1);
            let result = request(control, "data.connect", json!({"session_id":session,"block_id":block,"data_ref_id":"terminal","subprotocol":wire::PROTOCOL})).await;
            let _ = reply.send(result.as_ref().map_err(|e| format!("{e:#}")).and_then(|v| {
                v["ticket"]
                    .as_str()
                    .map(|token| TerminalTicket {
                        token: token.into(),
                        bundled: false,
                    })
                    .ok_or_else(|| "Missing data ticket".into())
            }));
            result?;
        }
        HostCommand::Terminal {
            session,
            block,
            command,
        } => {
            let Some(worker) = workers.get_mut(&block) else {
                return Ok(());
            };
            match command {
                Command::Selection(input) => {
                    let _ = worker.commands.send(DataCommand::Selection(input));
                }
                Command::Resize(cols, rows, w, h) => {
                    worker.size = Some((cols, rows, w, h));
                    resize(control, &session, &block, cols, rows, w, h).await?;
                }
                Command::Scroll(delta) => {
                    let _ = worker.commands.send(DataCommand::Scroll(delta));
                }
                Command::Search(needle) => {
                    let _ = worker.commands.send(DataCommand::Search(needle));
                }
                Command::SearchStep(older) => {
                    let _ = worker.commands.send(DataCommand::SearchStep(older));
                }
                Command::Reconnect => {
                    let _ = worker.commands.send(DataCommand::Reconnect);
                }
                input => {
                    let (connected, owner) = {
                        let m = shared.model.lock().expect("host model");
                        let Some(connection) = m.terminals.get(&block) else {
                            return Ok(());
                        };
                        let m = connection.model.lock().expect("terminal model");
                        (m.connected && m.control_connected, m.owner)
                    };
                    if !connected {
                        return Ok(());
                    }
                    // Releases, pointer motion and wheel turns never claim the block; presses do.
                    let passive = match &input {
                        Command::Key(key)
                        | Command::KeyWithPrefix { key, .. }
                        | Command::SendKey(key) => key["action"] == "release",
                        Command::Mouse(mouse) => mouse["action"] != "press",
                        Command::Wheel(_) => true,
                        _ => false,
                    };
                    if !owner && !passive {
                        let result = request(
                            control,
                            "session.own_block",
                            json!({"session_id":session,"block_ids":[block],"mode":"take"}),
                        )
                        .await?;
                        shared.update(|m| {
                            match result["blocks"]
                                .as_array()
                                .and_then(|blocks| blocks.iter().find(|b| b["block_id"] == block))
                            {
                                Some(b) => m.set_owner(&block, b["owner"].as_str()),
                                None => {
                                    if let Some(connection) = m.terminals.get(&block) {
                                        connection.model.lock().expect("terminal model").owner =
                                            false;
                                    }
                                }
                            }
                        });
                    }
                    // Typing, pasting and clicking return the view to the live screen; pointer
                    // motion, releases and wheel turns leave it (and its frame) alone.
                    if !matches!(&input, Command::Mouse(m) if m["action"] != "press")
                        && !matches!(input, Command::Wheel(_))
                    {
                        let _ = worker.commands.send(DataCommand::Bottom);
                    }
                    if matches!(
                        input,
                        Command::Key(_)
                            | Command::KeyWithPrefix { .. }
                            | Command::SendKey(_)
                            | Command::Paste(_)
                    ) && let Some(connection) = shared
                        .model
                        .lock()
                        .expect("host model")
                        .terminals
                        .get(&block)
                    {
                        connection
                            .model
                            .lock()
                            .expect("terminal model")
                            .input_events += 1;
                    }
                    match input {
                        Command::Key(key) => {
                            request(
                                control,
                                "client.key",
                                json!({"session_id":session,"block_id":block,"key":key}),
                            )
                            .await?;
                        }
                        Command::KeyWithPrefix { key, prefix } => {
                            request(control, "client.key", json!({"session_id":session,"block_id":block,"key":key,"prefix":prefix})).await?;
                        }
                        Command::SendKey(key) => {
                            request(control, "block.event", json!({"session_id":session,"block_id":block,"event":{"type":"key","payload":key}})).await?;
                        }
                        Command::Paste(text) => {
                            request(control, "block.event", json!({"session_id":session,"block_id":block,"event":{"type":"paste","payload":{"text":text}}})).await?;
                        }
                        Command::Mouse(payload) => {
                            request(control, "block.event", json!({"session_id":session,"block_id":block,"event":{"type":"mouse","payload":payload}})).await?;
                        }
                        Command::Wheel(payload) => {
                            request(control, "block.event", json!({"session_id":session,"block_id":block,"event":{"type":"wheel","payload":payload}})).await?;
                        }
                        Command::TakeControl => {}
                        _ => unreachable!(),
                    }
                }
            }
        }
        HostCommand::Reconnect
        | HostCommand::Shutdown
        | HostCommand::PrefetchViews(_)
        | HostCommand::Keymap(_)
        | HostCommand::Download(_)
        | HostCommand::DownloadConfirm(_)
        | HostCommand::DownloadCancel(_)
        | HostCommand::Upload { .. } => {
            unreachable!()
        }
    }
    Ok(())
}

/// The theme a block this client creates starts with, unless the server refused themes.
fn new_block_theme<'a>(shared: &Shared, theme: Option<&'a SetTheme>) -> Option<&'a SetTheme> {
    let mut m = shared.model.lock().expect("host model");
    let theme = theme.filter(|_| !m.themes_unsupported);
    m.theme_declared |= theme.is_some();
    theme
}

async fn resize(
    control: &mut Client,
    session: &str,
    block: &str,
    cols: u16,
    rows: u16,
    w: u16,
    h: u16,
) -> Result<()> {
    request(control, "com.superlogical.terminal.resize", json!({"session_id":session,"block_id":block,"args":{"columns":cols,"rows":rows,"cell_width_px":w,"cell_height_px":h}})).await?;
    Ok(())
}

async fn run_data(
    endpoint: String,
    session: String,
    block: String,
    host: flume::Sender<HostCommand>,
    commands: flume::Receiver<DataCommand>,
    shared: PaneShared,
) {
    let mut welcome: Option<wire::Welcome> = None;
    let mut stream: Option<TerminalStream> = None;
    let mut selection = None;
    let mut search = crate::terminal::search::Controller::default();
    let mut visible = true;
    let mut renderer = match Renderer::new() {
        Ok(r) => r,
        Err(e) => {
            shared.status(format!("Renderer failed: {e:#}"), false);
            return;
        }
    };
    loop {
        let result: Result<bool> = async {
            let mut socket = None;
            if let (Some(previous), Some(state)) = (&welcome, &stream) {
                let mut next = transport::socket(&endpoint, "/api/data", wire::PROTOCOL).await?;
                if let Ok(resumed) = wire::resume(&mut next, &previous.token, state.received).await {
                    anyhow::ensure!(resumed.nonce == previous.nonce, "Rex changed nonce during resume");
                    welcome = Some(resumed); socket = Some(next);
                    shared.status("Live · stream resumed", state.terminal.is_some());
                }
            }
            if socket.is_none() {
                let (next, fresh) = open_data(&endpoint, &session, &block, &host).await?;
                selection = Some(crate::terminal::selection::Controller::new()?);
                search.forget_terminal();
                stream = Some(TerminalStream::new(fresh.nonce)); welcome = Some(fresh);
                shared.update(|m|m.restore_generation+=1);
                renderer = Renderer::new()?; socket = Some(next);
                shared.status("Restoring terminal…", false);
            }
            let mut socket = socket.context("data socket")?;
            let stream = stream.as_mut().context("terminal stream")?;
            let selection = selection.as_mut().context("selection controller")?;
            let mut dirty = true;
            let mut tick = tokio::time::interval(Duration::from_millis(16));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    bytes = wire::next_binary(&mut socket) => {
                        if let Err(error) = stream.feed(&bytes?) { welcome = None; return Err(error); }
                        dirty = true;
                    }
                    command = commands.recv_async() => match command {
                        Ok(DataCommand::Selection(input)) => if input.generation == shared.model.lock().expect("terminal model").restore_generation && let Some(terminal) = &stream.terminal { selection.apply(terminal, input)?; dirty = true; },
                        Ok(DataCommand::Visible(value)) => { if value && !visible { dirty = true; } else if !value && let Some(terminal) = &stream.terminal { selection.stop(terminal)?; } visible = value; }
                        Ok(DataCommand::Scroll(delta)) => if let Some(terminal) = &mut stream.terminal { terminal.scroll_viewport(ghostty_vt::terminal::ScrollViewport::Delta(delta)); selection.after_scroll(terminal)?; dirty = true; },
                        Ok(DataCommand::Bottom) => if let Some(terminal) = &mut stream.terminal { terminal.scroll_viewport(ghostty_vt::terminal::ScrollViewport::Bottom); dirty = true; },
                        // A search that fails is dropped with a notice; the terminal's stream is not its to end.
                        Ok(DataCommand::Search(needle)) => { if let Err(error) = search.set_needle(stream.terminal.as_ref(), needle) { search = Default::default(); shared.update(|m| m.notice = Some(format!("Search failed: {error:#}"))); } dirty = true; },
                        // Ghostty scrolls the selected match into view.
                        Ok(DataCommand::SearchStep(older)) => if let Some(terminal) = &stream.terminal { if let Err(error) = search.step(terminal, older) { search = Default::default(); shared.update(|m| m.notice = Some(format!("Search failed: {error:#}"))); } selection.after_scroll(terminal)?; dirty = true; },
                        Ok(DataCommand::Reconnect) => { shared.status("Reconnecting terminal…", false); return Ok(false); }
                        Ok(DataCommand::Stop) | Err(_) => return Ok(true),
                    },
                    _ = tick.tick(), if visible && commands.is_empty() && (dirty || stream.terminal.as_ref().is_some_and(|t| selection.autoscrolling(t))) => {
                        // Mid synchronized update: keep showing the last frame, check again next tick.
                        if stream.render_held(Instant::now())? { continue; }
                        if let Some(terminal) = &mut stream.terminal {
                            selection.tick(terminal)?;
                            let search_pending = match search.update(terminal) {
                                Ok(pending) => pending,
                                Err(error) => { search = Default::default(); shared.update(|m| m.notice = Some(format!("Search failed: {error:#}"))); false }
                            };
                            let mut frame = renderer.frame(terminal)?;
                            frame.selection_revision = selection.revision;
                            frame.search = match search.results(terminal) {
                                Ok(results) => results,
                                Err(error) => { search = Default::default(); shared.update(|m| m.notice = Some(format!("Search failed: {error:#}"))); None }
                            };
                            let frame = Arc::new(frame);
                            let mut transitioned = false;
                            shared.update(|m| {
                                m.frame = Some(frame);
                                if !m.connected { m.connected = true; m.status = "Live · snapshot restored".into(); transitioned = true; }
                                if let Some(code) = stream.exit_code { m.status = format!("Process exited · {code}"); }
                            });
                            if transitioned { let _ = shared.host_wake.try_send(()); }
                            dirty = search_pending;
                        } else {
                            dirty = false;
                        }
                    }
                }
            }
        }.await;
        match result {
            Ok(true) => return,
            Ok(false) => {}
            Err(error) => {
                if error
                    .downcast_ref::<wire::DataClosed>()
                    .is_some_and(|closed| closed.code == Some(4000))
                {
                    shared.status("Terminal closed on the server", false);
                    return;
                }
                shared.status(format!("Reconnecting · {error:#}"), false);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

async fn open_data(
    endpoint: &str,
    session: &str,
    block: &str,
    host: &flume::Sender<HostCommand>,
) -> Result<(transport::Socket, wire::Welcome)> {
    loop {
        let (reply, ticket) = tokio::sync::oneshot::channel();
        host.send(HostCommand::Ticket {
            session: session.into(),
            block: block.into(),
            reply,
        })
        .map_err(|_| anyhow::anyhow!("host worker stopped"))?;
        let ticket = ticket.await?.map_err(anyhow::Error::msg)?;
        let mut next = transport::socket(endpoint, "/api/data", wire::PROTOCOL).await?;
        match wire::handshake(&mut next, &ticket.token).await {
            Ok(welcome) => return Ok((next, welcome)),
            // The pool removed this single-use token before handing it out. The
            // next request goes through data.connect on a fresh socket.
            Err(error) if ticket.bundled && error.is::<wire::HandshakeRejected>() => {}
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
mod tests;
