//! Add host: the machines on this device's tailnet, each added with one press. A machine whose
//! Rex server answers over the tailnet (built-in Tailscale, `tailscale serve`, or a listener on
//! its tailnet address) is attached to directly; any other is reached by forwarding its Rex
//! socket over SSH as a user you name (Tailscale SSH needs no keys).
use super::panel::{Entry, Form, Intent, PanelKind};
use super::theme::*;
use super::workspace::Workspace;
use rex_rune::settings::Host;
use rex_rune::tailnet::{self, ListenerDescription, Machine, Probe};
use rune::{El, Span, Tx, el, row, text};
use std::time::{Duration, Instant};

/// One machine of the tailnet in the Add host list.
#[derive(Clone, Debug)]
pub struct Candidate {
    pub machine: Machine,
    /// What checking the machine for a Rex server found; none while that is being checked, and
    /// for machines that are offline.
    pub probe: Option<Probe>,
}

/// What the Add host list says about a machine after its name. [`MachineWords::detail`] is
/// all of it as one line; the list draws the parts in columns (`Workspace::machine_columns`).
#[derive(Clone, Debug, PartialEq)]
pub struct MachineWords {
    /// The labels of the saved hosts that already reach the machine.
    pub added: Vec<String>,
    /// What checking it found, in reading order; `true` marks trouble.
    pub status: Vec<(String, bool)>,
    /// Its operating system as words, or empty when unknown.
    pub os: String,
    /// What the machine calls itself, when that isn't its name on the tailnet.
    pub host_name: Option<String>,
    /// The group it is listed under, which already says that it is offline.
    pub group: &'static str,
}

impl MachineWords {
    /// Every part in one line, as a tooltip or a screen reader gets it.
    pub fn detail(&self) -> String {
        let mut words: Vec<String> = Vec::new();
        if let Some(added) = self.added_as() {
            words.push(added);
        }
        words.extend(self.status.iter().map(|(word, _)| word.clone()));
        if !self.os.is_empty() {
            words.push(self.os.clone());
        }
        words.extend(self.host_name.clone());
        words.join(" · ")
    }

    /// `Added as <labels>`, for a machine a saved host already reaches.
    pub fn added_as(&self) -> Option<String> {
        (!self.added.is_empty()).then(|| format!("Added as {}", self.added.join(", ")))
    }
}

/// The groups of the Add host list, in the order it shows them.
const REX_SERVERS: &str = "Rex servers";
const TAILNET: &str = "Tailnet";
const OFFLINE: &str = "Offline";

/// The Add host list, and what its rows keep for their columns.
pub const LIST_WIDTH: f32 = 520.;
/// The operating system's column: wide enough for "Windows" and "Android".
const OS_COLUMN: f32 = 52.;
/// The "Added" chip's column, kept only while a listed machine is already added.
const ADDED_COLUMN: f32 = 50.;
/// The least a row keeps for what follows the name, before the name itself is cut.
const STATUS_FLOOR: f32 = 56.;

/// How wide a machine's name may be in a list `width` wide before it is cut: the columns
/// after it keep their places, and its status keeps [`STATUS_FLOOR`].
pub fn name_limit(width: f32, added_column: bool) -> f32 {
    // The surface's padding, the scroller's, the row's own, then the chevron and the gaps.
    let row = width - 10. - 4. - 18.;
    let after = 14. + 8. + OS_COLUMN + 8. + STATUS_FLOOR + 8.;
    let added = if added_column { ADDED_COLUMN + 8. } else { 0. };
    (row - after - added).max(80.)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Scan {
    #[default]
    Idle,
    Scanning,
    Ready,
    Unavailable(String),
}

/// A scan that hasn't answered in this long is given up on, so the list can look again.
const SCAN_PATIENCE: Duration = Duration::from_secs(20);

#[derive(Default)]
pub struct Discovery {
    pub scan: Scan,
    pub candidates: Vec<Candidate>,
    /// Counts scans, so results of an earlier one are dropped when a newer one runs.
    pub generation: u64,
    /// When the current scan started.
    pub started: Option<Instant>,
    /// Scans headless tests would have started.
    pub ran: usize,
}

/// The name SSH logins use unless the form says otherwise.
pub fn local_user() -> String {
    std::env::var("USER")
        .ok()
        .filter(|u| !u.is_empty())
        .unwrap_or_default()
}

/// Whether `socket`, as typed in the form, is an absolute path an `ssh://` endpoint can carry;
/// empty means "look it up".
fn socket_error(socket: &str) -> Option<String> {
    if socket.is_empty() {
        return None;
    }
    (!socket.starts_with('/')
        || socket.len() == 1
        || socket
            .contains(|c: char| c.is_whitespace() || matches!(c, '\'' | '"' | '$' | '`' | '\\')))
    .then(|| "Enter an absolute socket path, such as /run/rex/server.sock".to_owned())
}

impl Workspace {
    /// Open the Add host list and look at the tailnet again.
    pub fn open_add_host(&mut self) {
        self.open_panel(
            PanelKind::Discover,
            "Add host",
            &[("", "Search your tailnet…", "")],
        );
        self.scan_tailnet();
    }

    /// Ask Tailscale for the machines, then check each online one for a Rex server, off the
    /// UI thread. Machines appear as soon as Tailscale answers; their kinds follow.
    pub fn scan_tailnet(&mut self) {
        if self.discovery.scan == Scan::Scanning
            && self
                .discovery
                .started
                .is_some_and(|started| started.elapsed() < SCAN_PATIENCE)
        {
            return;
        }
        self.discovery.scan = Scan::Scanning;
        self.discovery.started = Some(Instant::now());
        self.discovery.generation += 1;
        let generation = self.discovery.generation;
        if cfg!(test) {
            self.discovery.ran += 1;
            return;
        }
        let handle = self.handle.clone();
        let _ = std::thread::Builder::new()
            .name("rex-tailnet".into())
            .spawn(move || {
                let result = tailnet::status();
                let machines = result.clone().unwrap_or_default();
                handle.update(move |s, _| s.apply_scan(generation, result));
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                let Ok(runtime) = runtime else { return };
                runtime.block_on(async {
                    use futures_util::{StreamExt, stream::FuturesUnordered};
                    let mut probes: FuturesUnordered<_> = machines
                        .iter()
                        .filter(|m| m.online && !m.this_machine && !m.is_ios())
                        .map(|m| async move { (m.address.clone(), tailnet::probe(m).await) })
                        .collect();
                    while let Some((address, probe)) = probes.next().await {
                        handle.update(move |s, _| s.apply_probe(generation, &address, probe));
                    }
                });
            });
    }

    pub fn apply_scan(&mut self, generation: u64, result: Result<Vec<Machine>, String>) {
        if generation != self.discovery.generation {
            return;
        }
        match result {
            Ok(machines) => {
                self.discovery.candidates = machines
                    .into_iter()
                    .map(|machine| Candidate {
                        machine,
                        probe: None,
                    })
                    .collect();
                self.discovery.scan = Scan::Ready;
            }
            Err(error) => {
                self.discovery.candidates.clear();
                self.discovery.scan = Scan::Unavailable(error);
            }
        }
        self.discovery.started = None;
        self.query_changed();
    }

    pub fn apply_probe(&mut self, generation: u64, address: &str, probe: Probe) {
        if generation != self.discovery.generation {
            return;
        }
        if let Some(c) = self
            .discovery
            .candidates
            .iter_mut()
            .find(|c| c.machine.address == address)
        {
            c.probe = Some(probe);
        }
    }

    /// The saved hosts that reach `machine`, by their endpoints' host names.
    fn saved_hosts_for(&self, machine: &Machine) -> Vec<usize> {
        let address = machine.address.to_lowercase();
        let short = machine.name.to_lowercase();
        self.settings
            .hosts
            .iter()
            .enumerate()
            .filter(|(_, h)| {
                let host = rex_rune::settings::endpoint_label(&h.endpoint).to_lowercase();
                host == address || host == short
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// The Rex endpoint a check found on candidate `c`, if one answered.
    fn found_rex(c: &Candidate) -> Option<&str> {
        match &c.probe {
            Some(Probe::Rex { endpoint, .. }) => Some(endpoint),
            _ => None,
        }
    }

    /// What the Add host list says about candidate `c`: the saved hosts that reach it, what
    /// the check for a Rex server found, its system and its own name.
    pub fn machine_words(&self, c: &Candidate) -> MachineWords {
        let m = &c.machine;
        let added = self
            .saved_hosts_for(m)
            .into_iter()
            .map(|h| self.settings.hosts[h].label.clone())
            .collect();
        let method = if m.ssh { "Tailscale SSH" } else { "SSH" };
        let status = match (&c.probe, m.online) {
            (
                Some(Probe::Rex {
                    endpoint,
                    listeners,
                    reason,
                }),
                _,
            ) => {
                let access = if listeners.is_empty() {
                    "listener details unavailable"
                } else {
                    "view listeners"
                };
                // Trouble leads, so a row short of room cuts the address, not the reason.
                match reason {
                    Some(reason) => vec![(reason.clone(), true), (endpoint.clone(), false)],
                    None => vec![(endpoint.clone(), false), (access.to_owned(), false)],
                }
            }
            (_, false) => vec![("Offline".to_owned(), false)],
            (None, true) => vec![("Checking for Rex…".to_owned(), false)],
            (Some(Probe::Nothing), true) => vec![(method.to_owned(), false)],
            (Some(Probe::Unknown(_)), true) => {
                vec![
                    ("Rex check failed".to_owned(), true),
                    (method.to_owned(), false),
                ]
            }
        };
        let group = if Self::found_rex(c).is_some() {
            REX_SERVERS
        } else if m.online {
            TAILNET
        } else {
            OFFLINE
        };
        MachineWords {
            added,
            status,
            os: m.os_name().to_owned(),
            host_name: m.host_name.clone(),
            group,
        }
    }

    /// The columns of a machine's row in the Add host list, after its name: what it calls
    /// itself, then, from the trailing edge, what the check found (cut with "…" when the row
    /// is short of room, trouble in `WARNING`), its system and, in a list with a machine
    /// already added, the "Added" chip. None for a row that isn't a machine's.
    pub fn machine_columns(
        &self,
        action: &Intent,
        selected: bool,
        added_column: bool,
    ) -> Option<El> {
        let (Intent::AddMachine(index) | Intent::MachineForm(index)) = action else {
            return None;
        };
        let words = self.machine_words(self.discovery.candidates.get(*index)?);
        let tone = if selected {
            rgb(ink()).alpha(0.75)
        } else {
            rgb(muted())
        };
        // What the machine calls itself gets only the room the other columns leave, and
        // fades out rather than ending in an ellipsis that would read as the status's.
        let mut columns = row().grow(1.).shrink().gap(8.).center_y().child(
            match &words.host_name {
                Some(host_name) => text(host_name.clone())
                    .font(ui(11.))
                    .color(tone)
                    .truncate_fade(),
                None => el(),
            }
            .grow(1.),
        );
        // The group's heading already says a machine is offline.
        let mut line = String::new();
        let mut trouble = Vec::new();
        for (word, is_trouble) in words.status.iter().filter(|_| words.group != OFFLINE) {
            if !line.is_empty() {
                line.push_str(" · ");
            }
            if *is_trouble {
                trouble.push(Span::new(line.len(), line.len() + word.len()).color(rgb(warning())));
            }
            line.push_str(word);
        }
        if !line.is_empty() {
            columns = columns.child(
                text(line.clone())
                    .key(format!("machine-status:{index}"))
                    .font(ui(11.))
                    .color(tone)
                    .spans(trouble)
                    .truncate()
                    .help_when_truncated(line),
            );
        }
        columns = columns.child(
            row()
                .w(OS_COLUMN)
                .child(text(words.os.clone()).font(ui(11.)).color(tone).truncate()),
        );
        if added_column {
            columns = columns.child(row().w(ADDED_COLUMN).justify_end().children(
                words.added_as().map(|added| {
                    row()
                        .key(format!("machine-added:{index}"))
                        .h(18.)
                        .px(7.)
                        .center()
                        .radius(6.)
                        .continuous_corners()
                        .fill(white(0.08))
                        .help(added)
                        .child(text("Added").font(medium(10.5)).color(tone))
                }),
            ));
        }
        Some(columns)
    }

    /// The Add host list: Rex servers first, then the rest of the tailnet, then machines that
    /// are offline, each group under its icon; the footer takes an endpoint by hand. A machine
    /// already added stays listed, marked, so another login to it can be added.
    pub fn discovery_entries(&self, lower: &str) -> Vec<Entry> {
        let mut rex = Vec::new();
        let mut online = Vec::new();
        let mut offline = Vec::new();
        for (i, c) in self.discovery.candidates.iter().enumerate() {
            let m = &c.machine;
            // This machine's Rex is the local socket; iPhones and iPads run no Rex or sshd.
            if m.this_machine || m.is_ios() {
                continue;
            }
            if !format!(
                "{} {} {}",
                m.name,
                m.address,
                m.host_name.as_deref().unwrap_or("")
            )
            .to_lowercase()
            .contains(lower)
            {
                continue;
            }
            let words = self.machine_words(c);
            let found = Self::found_rex(c);
            let found_saved =
                found.is_some_and(|e| self.settings.hosts.iter().any(|h| h.endpoint == e));
            let action = if found.is_some() && !found_saved {
                Intent::AddMachine(i)
            } else {
                Intent::MachineForm(i)
            };
            let mut e = Entry::new(&m.name, words.detail(), words.group, action);
            e.group_icon = Some(match words.group {
                REX_SERVERS => &SERVER,
                TAILNET => &NETWORK,
                _ => &MOON,
            });
            e.current = !words.added.is_empty();
            match words.group {
                REX_SERVERS => rex.push(e),
                TAILNET => online.push(e),
                _ => offline.push(e),
            }
        }
        let mut entries = rex;
        entries.extend(online);
        entries.extend(offline);
        let mut manual = Entry::new("Enter an endpoint…", "", "", Intent::AddHostManually);
        manual.footer = true;
        entries.push(manual);
        entries
    }

    /// What the Add host list says in place of rows.
    pub fn discovery_empty_message(&self) -> String {
        match &self.discovery.scan {
            Scan::Idle | Scan::Scanning => "Looking at your tailnet…".into(),
            Scan::Unavailable(words) => words.clone(),
            Scan::Ready => "No machine by that name on your tailnet.".into(),
        }
    }

    /// Attach to the Rex server a check found on machine `index` at once, then return to
    /// Settings when the list came from there. Without one, the SSH form opens instead.
    pub fn add_machine(&mut self, index: usize, tx: &mut Tx) {
        let Some(c) = self.discovery.candidates.get(index) else {
            return;
        };
        let Some(endpoint) = Self::found_rex(c) else {
            self.machine_form(index);
            return;
        };
        if let Some(Probe::Rex { listeners, .. }) = &c.probe
            && !listeners.is_empty()
        {
            let note = format!(
                "Reachable from this device: {endpoint}\n\nServer-reported listeners (bound addresses are not reachability or permission checks):\n{}",
                listeners
                    .iter()
                    .map(|l| l.description())
                    .collect::<Vec<_>>()
                    .join("\n\n")
            );
            let name = c.machine.name.clone();
            let endpoint = endpoint.to_owned();
            self.open_panel(
                PanelKind::Form(Form::Discovered(note)),
                "Connect to Rex",
                &[
                    ("Host name", "my-host", &name),
                    ("Endpoint", "https://host", &endpoint),
                ],
            );
            return;
        }
        let host = Host {
            label: c.machine.name.clone(),
            endpoint: endpoint.to_owned(),
            remote_control: false,
            collapsed: false,
            shell: None,
        };
        let back = self.settings_back();
        self.close_panel(tx);
        self.add_host(host, tx);
        self.panel = back;
    }

    /// The form that reaches machine `index` over SSH: its name, the user to log in as, and
    /// the Rex socket there when it isn't where Rex usually keeps it.
    pub fn machine_form(&mut self, index: usize) {
        let Some(c) = self.discovery.candidates.get(index) else {
            return;
        };
        let name = c.machine.name.clone();
        let user = local_user();
        self.open_panel(
            PanelKind::Form(Form::AddSsh(index)),
            &format!("Add {name} over SSH"),
            &[
                ("Host name", "my-host", &name),
                ("SSH user", "The user Rex runs as there", &user),
                (
                    "Rex socket",
                    "Found automatically — or e.g. /run/rex/server.sock",
                    "",
                ),
            ],
        );
    }

    /// Words under the SSH form: how the machine is reached and signed in to, and what the
    /// check for a Rex listener found.
    pub fn machine_form_note(&self, index: usize) -> String {
        let Some(c) = self.discovery.candidates.get(index) else {
            return String::new();
        };
        let m = &c.machine;
        let mut note = format!(
            "Forwards the Rex server's socket on {} over SSH; Rex must be running there.",
            m.address
        );
        match (&c.probe, m.online) {
            (
                Some(Probe::Rex {
                    endpoint,
                    listeners,
                    reason,
                }),
                _,
            ) => {
                note.push_str(&format!(
                    " Rex also answers at {endpoint}; this adds another login to it."
                ));
                if let Some(reason) = reason {
                    note.push_str(&format!(" Hello failed: {reason}."));
                }
                if !listeners.is_empty() {
                    note.push_str("\nServer-reported listeners: bound addresses are not reachability or permission checks.");
                }
                for listener in listeners {
                    note.push_str(&format!("\n{}", listener.description()));
                }
            }
            (Some(Probe::Nothing), _) => {
                note.push_str(
                    " Nothing answered on ports 443 or 7297 there, so SSH is the way in.",
                );
            }
            (Some(Probe::Unknown(why)), _) => {
                note.push_str(&format!(
                    " Couldn't tell whether Rex listens there ({why}); an https:// endpoint can also be entered by hand."
                ));
            }
            (None, true) => note.push_str(" Still checking whether Rex listens there."),
            (None, false) => {}
        }
        if m.ssh {
            note.push_str(
                " Tailscale SSH is on, so the tailnet's policy signs you in with no keys.",
            );
        } else {
            note.push_str(" Your SSH keys and configuration apply.");
        }
        note.push_str(
            " Leave Rex socket empty to look it up there (rex server status, the user's runtime directory, /run/rex).",
        );
        note
    }

    /// The error in the SSH form's fields, if any: `values` are its host name, user and socket.
    pub fn machine_form_error(&self, index: usize, values: &[String]) -> Option<String> {
        let user = values.get(1).map(String::as_str).unwrap_or_default();
        let socket = values.get(2).map(String::as_str).unwrap_or_default();
        if user.is_empty() {
            Some("Enter the user to log in as".to_owned())
        } else if user.contains(|c: char| c.is_whitespace() || c == '@' || c == ':' || c == '/') {
            Some("Enter one user name, without the host".to_owned())
        } else if let Some(error) = socket_error(socket) {
            Some(error)
        } else {
            match self.discovery.candidates.get(index) {
                None => Some("That machine is no longer listed".to_owned()),
                Some(c) => rex_rune::ssh::Target::parse(&c.machine.ssh_endpoint(user, socket))
                    .err()
                    .map(|e| e.to_string()),
            }
        }
    }

    /// Save `host`, make it the active one and connect. Nothing happens for an endpoint the
    /// workspace already has; it is selected instead.
    pub fn add_host(&mut self, host: Host, tx: &mut Tx) {
        self.navigation.cancel();
        if let Some(existing) = self
            .settings
            .hosts
            .iter()
            .position(|h| h.endpoint == host.endpoint)
        {
            self.active = existing;
        } else {
            self.settings.hosts.push(host);
            self.active = self.settings.hosts.len() - 1;
            self.connect_host(self.active, false);
        }
        self.update_visibility();
        self.save();
        self.update_title(tx);
    }
}
