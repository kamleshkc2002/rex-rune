//! Commands: every row the command list offers, in one table.
//!
//! The rows are grouped by what they act on (the session, tab, pane and host on screen, then
//! the view and the app) and listed under those headings until something is typed; a search
//! shows the matches as one list, best match first. A row's keyboard hint is the effective
//! binding of its action, so it follows `shortcuts.json` and the host's keymap.
//!
//! To add a command, add its row to its group's function below and its id to the pinned list
//! in `command_tests.rs`. A row that runs a registered action uses `Command::action`; one
//! that has no action of its own adds a `Run` variant. Every action in `actions::SPECS` needs
//! a row here or an entry in `NOT_LISTED`: a test fails otherwise.
//!
//! The host on screen adds its own: the actions its Rex configuration defines and the titled
//! actions of its block types, under Server actions. One that a command above already offers
//! by the same name is not listed twice.
use super::{
    about::{self, REPOSITORY},
    actions, find, help, keycaps, pane_header,
    panel::{Entry, Intent, PaneAction, SessionAction, SessionTarget},
    splits::SplitWay,
    workspace::Workspace,
};
use rex_linux::{connection::HostCommand, settings::PaneDensity};
use rune::Tx;
use serde_json::Value;
use std::borrow::Cow;

pub const CLOSE_SESSION: &str = "session.close";
pub const NEXT_SESSION: &str = "session.next";
pub const PREVIOUS_SESSION: &str = "session.previous";
pub const REFRESH_SESSIONS: &str = "session.refresh";
pub const COLLAPSE_OTHER_HOSTS: &str = "client.sidebar.collapse_others";
pub const CHOOSE_THEME: &str = "client.theme.choose";
pub const COPY_VERSION: &str = "client.version.copy";
pub const REPORT_ISSUE: &str = "client.issue.report";

/// Registered actions with no row: each needs an argument Commands cannot ask for, or is
/// the list itself.
#[cfg(test)]
pub const NOT_LISTED: [(&str, &str); 6] = [
    ("client.palette.toggle", "opens this list"),
    (
        actions::SELECT_SESSION,
        "needs a session; a search lists them",
    ),
    ("client.tab.goto", "needs a tab number; a search lists tabs"),
    ("pane.scroll", "needs a line count"),
    ("pane.send_key", "needs a key"),
    ("pane.actions", "opens the pane menu, whose rows are here"),
];

/// The section of a group's destructive rows, after every other.
const LAST: u8 = u8::MAX;

/// Where a new issue is written.
pub fn issue_url() -> String {
    format!("{REPOSITORY}/issues/new")
}

/// What a command acts on, in the order the list shows the groups.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Session,
    Tab,
    Pane,
    Host,
    /// Actions the host's Rex server offers, not this client.
    Server,
    View,
    App,
}

impl Group {
    pub fn title(self) -> &'static str {
        match self {
            Group::Session => "Session",
            Group::Tab => "Tab",
            Group::Pane => "Pane",
            Group::Host => "Host",
            Group::Server => "Server actions",
            Group::View => "View",
            Group::App => "App",
        }
    }
}

/// What a command without a registered client action does: a saved preference, the window
/// itself, or an action of the host's server.
#[derive(Clone, Debug, PartialEq)]
pub enum Run {
    /// List tabs under their sessions in the sidebar, or in the toolbar.
    TabsInSidebar(bool),
    PaneDensity(PaneDensity),
    CloseApp,
    /// A named action of the host on screen, run there for the session and pane on screen.
    ServerAction(String),
}

impl Run {
    pub fn run(self, s: &mut Workspace, tx: &mut Tx) {
        match self {
            Run::TabsInSidebar(sidebar) => s.set_vertical_tabs(sidebar),
            Run::PaneDensity(density) => {
                s.settings.pane_density = density;
                s.save();
                tx.instant();
            }
            Run::CloseApp => tx.close_window(),
            Run::ServerAction(name) => {
                // Asked once, as a bound key asks: a failure is a notice, never a retry.
                let mut request = rexrs::api::client::Action::new(name).origin("palette");
                request.session_id = s.model().selected.clone();
                if request.session_id.is_some() {
                    request.block_id = s.focused_block();
                }
                s.send(
                    s.active,
                    HostCommand::Action {
                        generation: s.keymap_generation,
                        request: Box::new(request),
                    },
                );
            }
        }
    }
}

pub struct Command {
    /// Stable across releases, unlike the label; a test pins the list of them.
    #[cfg_attr(not(test), allow(dead_code))]
    pub id: Cow<'static, str>,
    pub group: Group,
    /// More words a search finds the row by, in lower case.
    pub keywords: Cow<'static, str>,
    pub entry: Entry,
}

/// Whether a server action cannot run without arguments, which Commands has no way to ask
/// for.
fn needs_arguments(schema: Option<&Value>) -> bool {
    schema
        .and_then(|schema| schema["required"].as_array())
        .is_some_and(|required| !required.is_empty())
}

/// A label without its case or the ellipsis of a command that asks first.
fn plain(label: &str) -> String {
    label.trim_end_matches('…').trim().to_lowercase()
}

impl Command {
    fn new(
        id: impl Into<Cow<'static, str>>,
        group: Group,
        label: impl Into<String>,
        action: Intent,
    ) -> Self {
        Self::row(id, group, Entry::new(label, "", "", action))
    }

    /// A row that runs a registered action without arguments.
    fn action(
        id: impl Into<Cow<'static, str>>,
        group: Group,
        label: impl Into<String>,
        name: &'static str,
    ) -> Self {
        Self::new(id, group, label, Intent::ClientAction(name, Value::Null))
    }

    fn row(id: impl Into<Cow<'static, str>>, group: Group, entry: Entry) -> Self {
        Self {
            id: id.into(),
            group,
            keywords: Cow::Borrowed(""),
            entry,
        }
    }

    fn enabled(mut self, enabled: bool) -> Self {
        self.entry.enabled = enabled;
        self
    }

    /// Puts the row in its group's section `section`; a hairline separates one from the next.
    fn section(mut self, section: u8) -> Self {
        self.entry.section = section;
        self
    }

    /// A row that ends something (Close…, Remove…): in `DANGER`, and in the group's last
    /// section, after a hairline.
    fn destructive(mut self) -> Self {
        self.entry.danger = true;
        self.entry.section = LAST;
        self
    }

    fn keywords(mut self, keywords: &'static str) -> Self {
        self.keywords = Cow::Borrowed(keywords);
        self
    }

    /// Words in the hint column for a row that has no shortcut to show there.
    fn detail(mut self, detail: impl Into<String>) -> Self {
        self.entry.detail = detail.into();
        self
    }

    /// How well the row answers a search, lower first; none when it does not match. Every
    /// word of the query must be in the label, group or keywords, or the query as typed must
    /// be in the hint ("ctrl shift d").
    fn rank(&self, query: &str) -> Option<u8> {
        let label = self.entry.label.to_lowercase();
        let named = format!(
            "{label} {} {}",
            self.group.title().to_lowercase(),
            self.keywords
        );
        let words: Vec<&str> = query.split_whitespace().collect();
        if !words.iter().all(|word| named.contains(word))
            && !self.entry.detail.to_lowercase().contains(query)
        {
            return None;
        }
        let starts_a_word = |word: &&str| label.split(' ').any(|l| l.starts_with(*word));
        // "close pane" is Close pane… before Close pane or tab….
        Some(if plain(&label) == query {
            0
        } else if label.starts_with(query) {
            1
        } else if words.iter().all(starts_a_word) {
            2
        } else if words.iter().all(|word| label.contains(word)) {
            3
        } else {
            4
        })
    }
}

fn split_id(way: SplitWay) -> &'static str {
    match way {
        SplitWay::Right => "pane.split.right",
        SplitWay::Left => "pane.split.left",
        SplitWay::Down => "pane.split.down",
        SplitWay::Up => "pane.split.up",
        SplitWay::BestFit => "pane.split.best_fit",
    }
}

impl Workspace {
    /// The session on screen, as the target of a session action.
    fn session_on_screen(&self) -> Option<SessionTarget> {
        let model = self.model();
        let id = model.selected.as_ref()?;
        let session = model.sessions.iter().find(|s| &s.session_id == id)?;
        Some(SessionTarget {
            host: self.active,
            session: session.session_id.clone(),
            label: session.label.clone(),
        })
    }

    /// Ask before closing the session on screen, as its sidebar menu does.
    pub fn close_session_on_screen(&mut self, tx: &mut Tx) {
        if !self.model().connected {
            return;
        }
        if let Some(target) = self.session_on_screen() {
            self.perform(Intent::Session(target, SessionAction::Close), tx);
        }
    }

    /// Read every connected host's session list again, and the layout of the session each
    /// shows. Sessions follow the server's events without this; it is for when they seem not
    /// to have. Nothing is attached, created or closed.
    pub fn refresh_sessions(&mut self) {
        let connected: Vec<usize> = (0..self.hosts.len())
            .filter(|index| self.hosts[*index].model.connected)
            .collect();
        for index in &connected {
            self.send(*index, HostCommand::RefreshSessions);
        }
        match connected.as_slice() {
            [] => self.inform("No host is connected, so there are no sessions to read."),
            [index] => self.inform(format!(
                "Reading sessions again from {}",
                self.settings.hosts[*index].label
            )),
            hosts => self.inform(format!("Reading sessions again from {} hosts", hosts.len())),
        }
    }

    /// Another host's sessions are listed in the sidebar and could be folded away.
    fn other_hosts_expanded(&self) -> bool {
        self.settings
            .hosts
            .iter()
            .enumerate()
            .any(|(index, host)| index != self.active && !host.collapsed)
    }

    /// Fold every host in the sidebar but the one on screen. Each opens again from its
    /// header, or when one of its sessions is shown.
    pub fn collapse_other_hosts(&mut self) {
        if !self.other_hosts_expanded() {
            return;
        }
        let active = self.active;
        for (index, host) in self.settings.hosts.iter_mut().enumerate() {
            host.collapsed |= index != active;
        }
        self.save();
    }

    fn session_commands(&self) -> Vec<Command> {
        use Group::Session as G;
        let model = self.model();
        let host = &self.settings.hosts[self.active].label;
        let elsewhere = self.session_stop_count() > 1;
        vec![
            Command::new("session.switch", G, "Switch session…", Intent::Switcher)
                .keywords("change find go to"),
            Command::action(
                "session.new",
                G,
                format!("New session on {host}"),
                "session.new",
            )
            .enabled(model.connected)
            .keywords("create"),
            Command::new(
                "session.rename",
                G,
                "Rename session…",
                Intent::RenameSession,
            )
            .enabled(model.connected && model.view().is_some()),
            Command::action("session.next", G, "Next session", NEXT_SESSION).enabled(elsewhere),
            Command::action("session.previous", G, "Previous session", PREVIOUS_SESSION)
                .enabled(elsewhere),
            Command::action("history.back", G, "Back in history", actions::HISTORY_BACK)
                .detail("Mouse Back")
                .keywords("navigate"),
            Command::action(
                "history.forward",
                G,
                "Forward in history",
                actions::HISTORY_FORWARD,
            )
            .detail("Mouse Forward")
            .keywords("navigate"),
            Command::action("session.refresh", G, "Refresh sessions", REFRESH_SESSIONS)
                .enabled(self.hosts.iter().any(|h| h.model.connected))
                .keywords("reload read list"),
            Command::action("session.close", G, "Close session…", CLOSE_SESSION)
                .enabled(model.connected && self.session_on_screen().is_some())
                .keywords("kill end")
                .destructive(),
        ]
    }

    fn tab_commands_group(&self) -> Vec<Command> {
        use Group::Tab as G;
        let model = self.model();
        let in_session = model.connected && model.view().is_some();
        let several = in_session && model.view().is_some_and(|v| v.windows.len() > 1);
        let id = model.window().and_then(|w| w.window_id.clone());
        let mut rows = vec![
            Command::action("tab.new", G, "New tab", "client.tab.new")
                .enabled(in_session)
                .keywords("create"),
        ];
        if let Some(id) = &id {
            rows.push(Command::new(
                "tab.rename",
                G,
                "Rename tab…",
                Intent::RenameWindow(id.clone()),
            ));
        }
        rows.extend([
            Command::action("tab.next", G, "Next tab", "client.tab.next").enabled(several),
            Command::action("tab.previous", G, "Previous tab", "client.tab.previous")
                .enabled(several),
            Command::action(
                "tab.next_global",
                G,
                "Next tab across hosts",
                actions::NEXT_GLOBAL,
            ),
            Command::action(
                "tab.previous_global",
                G,
                "Previous tab across hosts",
                actions::PREVIOUS_GLOBAL,
            ),
        ]);
        if let Some(entries) = self.tab_commands() {
            rows.extend(
                [
                    "tab.mute",
                    "tab.move_left",
                    "tab.move_right",
                    "tab.close_others",
                ]
                .into_iter()
                .zip(entries)
                .map(|(id, entry)| {
                    let ends = entry.danger;
                    let row = Command::row(id, G, entry);
                    if ends { row.destructive() } else { row }
                }),
            );
        }
        if let Some(id) = id {
            rows.push(
                Command::new("tab.close", G, "Close tab…", Intent::CloseWindow(id)).destructive(),
            );
        }
        rows
    }

    /// The pane the list was opened for, else the focused one; none without a pane.
    fn pane_commands(&self) -> Vec<Command> {
        use Group::Pane as G;
        let Some(target) = self
            .panel
            .as_ref()
            .and_then(|p| p.target.clone())
            .or_else(|| self.focused_block().and_then(|b| self.target(&b)))
        else {
            return Vec::new();
        };
        let model = self.model();
        let zoomed = model
            .window()
            .is_some_and(|w| w.zoomed_block_id.as_ref() == Some(&target.block));
        // A row of the pane menu: it acts on `target`, and is available as it is there.
        let pane = |id, label: &'static str, action| {
            Command::new(id, G, label, Intent::Pane(target.clone(), action))
                .enabled(self.pane_verb_available(&target, action))
        };
        // The pane menu's groups in its order, with search and focus, which act on the
        // focused pane, beside the clipboard and the layout.
        let mut rows = vec![
            pane("pane.copy", "Copy selection", PaneAction::Copy).keywords("clipboard"),
            pane("pane.paste", "Paste", PaneAction::Paste).keywords("clipboard"),
            pane("pane.select_all", "Select all", PaneAction::SelectAll)
                .keywords("screen scrollback text"),
            Command::action("pane.search", G, "Search terminal…", actions::SEARCH)
                .keywords("find")
                .section(1),
            Command::action("pane.find_next", G, "Find next", find::NEXT)
                .enabled(self.can_find())
                .keywords("search match older")
                .section(1),
            Command::action("pane.find_previous", G, "Find previous", find::PREVIOUS)
                .enabled(self.can_find())
                .keywords("search match newer")
                .section(1),
        ];
        rows.extend(SplitWay::ALL.into_iter().map(|way| {
            pane(split_id(way), way.label(), PaneAction::Split(way))
                .keywords("new")
                .section(2)
        }));
        rows.push(
            pane(
                "pane.zoom",
                if zoomed { "Restore pane" } else { "Zoom pane" },
                PaneAction::Zoom,
            )
            .keywords("maximize unzoom")
            .section(2),
        );
        rows.extend(
            [
                "pane.focus.next",
                "pane.focus.previous",
                "pane.focus.left",
                "pane.focus.right",
                "pane.focus.up",
                "pane.focus.down",
            ]
            .into_iter()
            .zip(self.focus_commands())
            .map(|(id, entry)| Command::row(id, G, entry).section(3)),
        );
        rows.extend([
            pane("pane.rename", "Rename pane…", PaneAction::Rename).section(4),
            pane("pane.copy_id", "Copy pane ID", PaneAction::CopyId)
                .keywords("block clipboard")
                .section(4),
            pane("pane.download", "Download file…", PaneAction::Download)
                .keywords("save fetch get file transfer")
                .section(4),
            pane("pane.clear", "Clear terminal", PaneAction::Clear)
                .keywords("screen scrollback")
                .section(5),
            pane("pane.reset", "Reset terminal", PaneAction::Reset)
                .keywords("screen scrollback")
                .section(5),
            pane(
                "pane.reconnect",
                "Reconnect terminal",
                PaneAction::Reconnect,
            )
            .section(5),
            Command::action(
                "pane.close_focused",
                G,
                "Close pane or tab…",
                "client.close",
            )
            .keywords("focused")
            .destructive(),
            pane("pane.close", "Close pane…", PaneAction::Close).destructive(),
        ]);
        rows
    }

    fn host_commands(&self) -> Vec<Command> {
        use Group::Host as G;
        let host = self.active;
        let saved = &self.settings.hosts[host];
        let name = &saved.label;
        let mut rows = vec![
            Command::new("host.add", G, "Add host…", Intent::AddHost)
                .keywords("remote connect server"),
            Command::new(
                "host.edit",
                G,
                format!("Edit host {name}…"),
                Intent::EditHost(host),
            )
            .keywords("endpoint name"),
            Command::new(
                "host.shell",
                G,
                format!("Default shell for {name}…"),
                Intent::HostShell(host),
            )
            .detail(self.shell(host).unwrap_or_default()),
            Command::new(
                "host.remote_control",
                G,
                format!(
                    "Turn {} remote control for {name}",
                    if saved.remote_control { "off" } else { "on" }
                ),
                Intent::RemoteControl(host),
            ),
            Command::new("host.reconnect", G, "Reconnect host", Intent::Retry)
                .keywords("retry connection"),
        ];
        if self.can_start_ssh(host) {
            rows.push(
                Command::new(
                    "host.start_server",
                    G,
                    "Start personal server…",
                    Intent::StartSsh(host),
                )
                .detail(name.clone()),
            );
        }
        rows.extend([
            Command::action(
                "host.collapse_others",
                G,
                "Collapse other hosts",
                COLLAPSE_OTHER_HOSTS,
            )
            // It folds the sidebar's list, so there is nothing to see with the sidebar closed.
            .enabled(self.sidebar.is_open() && self.other_hosts_expanded())
            .keywords("sessions sidebar fold"),
            Command::new(
                "host.remove",
                G,
                format!("Remove host {name}…"),
                Intent::RemoveHost(host),
            )
            .enabled(self.settings.hosts.len() > 1)
            .keywords("forget delete")
            .destructive(),
        ]);
        rows
    }

    fn view_commands(&self) -> Vec<Command> {
        use Group::View as G;
        let sidebar_tabs = self.settings.vertical_tabs;
        let compact = self.settings.pane_density == PaneDensity::Compact;
        let zoom = (self.settings.ui_scale * 100.).round() as i32;
        let mut rows = vec![
            Command::action(
                "view.sidebar",
                G,
                if self.sidebar.is_open() {
                    "Hide sidebar"
                } else {
                    "Show sidebar"
                },
                "client.sidebar.toggle",
            )
            .keywords("toggle sessions"),
            Command::new(
                "view.tab_position",
                G,
                if sidebar_tabs {
                    "Show tabs in toolbar"
                } else {
                    "Show tabs in sidebar"
                },
                Intent::Command(Run::TabsInSidebar(!sidebar_tabs)),
            )
            .keywords("position vertical top"),
            Command::new(
                "view.pane_density",
                G,
                if compact {
                    "Use comfortable panes"
                } else {
                    "Use compact panes"
                },
                Intent::Command(Run::PaneDensity(if compact {
                    PaneDensity::Comfortable
                } else {
                    PaneDensity::Compact
                })),
            )
            .keywords("density style headers"),
            Command::action(
                "view.pane_headers",
                G,
                if self.header_reveal.pinned {
                    "Hide pane headers"
                } else {
                    "Reveal pane headers"
                },
                pane_header::TOGGLE,
            )
            .enabled(self.header_reveal.pinned || pane_header::can_reveal(self))
            .keywords("show title buttons"),
            Command::action(
                "view.terminal_theme",
                G,
                "Change terminal theme…",
                CHOOSE_THEME,
            )
            .keywords("colors colours appearance"),
            Command::new(
                "view.interface_theme",
                G,
                "Change interface theme…",
                Intent::ChooseChrome,
            )
            .keywords("window colors colours chrome appearance"),
            Command::action(
                "view.zoom.in",
                G,
                "Increase interface zoom",
                "client.zoom.in",
            )
            .enabled(zoom < 150)
            .keywords("enlarge bigger scale size"),
            Command::action(
                "view.zoom.out",
                G,
                "Decrease interface zoom",
                "client.zoom.out",
            )
            .enabled(zoom > 70)
            .keywords("shrink smaller scale size"),
            Command::action(
                "view.zoom.reset",
                G,
                "Reset interface zoom",
                "client.zoom.reset",
            )
            .enabled(zoom != 100)
            .keywords("scale size"),
        ];
        rows.extend(
            [
                "view.font.increase",
                "view.font.decrease",
                "view.font.reset",
            ]
            .into_iter()
            .zip(self.font_commands())
            .map(|(id, entry)| Command::row(id, G, entry).keywords("text")),
        );
        rows
    }

    fn app_commands(&self) -> Vec<Command> {
        use Group::App as G;
        let mut rows = vec![
            Command::action("app.settings", G, "Settings…", "client.settings")
                .keywords("preferences options"),
            Command::new("app.downloads", G, "Downloads", Intent::Downloads)
                .keywords("files transfers progress saved"),
            Command::new(
                "app.check_updates",
                G,
                "Check for updates",
                Intent::CheckUpdates,
            )
            .keywords("upgrade version release"),
        ];
        if let Some(update) = self.updates.available() {
            rows.push(
                Command::new(
                    "app.update",
                    G,
                    format!(
                        "Update Rex Linux ({})…",
                        rex_linux::format_version(&update.version)
                    ),
                    Intent::Updates,
                )
                .detail(self.updates.channel.id())
                .keywords("upgrade"),
            );
        }
        rows.extend([
            Command::action("app.about", G, "About Rex Linux", about::ABOUT)
                .keywords("version build license credits"),
            Command::row("app.help", G, help::command()).keywords("guide docs documentation"),
            Command::action("app.report_issue", G, "Report an issue", REPORT_ISSUE)
                .keywords("bug feedback problem"),
            Command::action("app.copy_version", G, "Copy version info", COPY_VERSION)
                .keywords("about build clipboard"),
            Command::action("app.copy_debug", G, "Copy debug info", about::COPY_DEBUG)
                .keywords("about diagnostics support clipboard"),
            Command::new("app.close", G, "Close app", Intent::Command(Run::CloseApp))
                .detail("Programs keep running")
                .keywords("quit exit window"),
        ]);
        rows
    }

    /// What the host on screen offers to run by name: the actions its Rex configuration
    /// defines and its block types' titled actions (their untitled methods are an API, not
    /// commands). One that needs arguments is left out, as is one `local` already offers by
    /// the same name. A block action needs a pane to act on.
    fn server_commands(&self, local: &[Command]) -> Vec<Command> {
        let Some(keymap) = self.keymap_snapshot() else {
            return Vec::new();
        };
        let in_pane = self.model().selected.is_some() && self.focused_block().is_some();
        keymap
            .reply
            .actions
            .iter()
            .filter_map(|action| {
                let block = match action.kind.as_deref()? {
                    "lua" => false,
                    "block" => true,
                    _ => return None,
                };
                let label = action.title.clone().filter(|t| !t.trim().is_empty())?;
                if needs_arguments(action.args.as_ref())
                    || local
                        .iter()
                        .any(|command| plain(&command.entry.label) == plain(&label))
                {
                    return None;
                }
                let mut command = Command::new(
                    format!("server:{}", action.name),
                    Group::Server,
                    label,
                    Intent::Command(Run::ServerAction(action.name.clone())),
                )
                .enabled(!block || in_pane);
                command.entry.keys = self.shortcut_chords(&action.name, &Value::Null);
                command.entry.detail = keycaps::spell(&command.entry.keys);
                command.keywords = Cow::Owned(
                    [
                        Some(&action.name),
                        action.category.as_ref(),
                        action.keywords.as_ref(),
                    ]
                    .into_iter()
                    .flatten()
                    .map(|words| words.to_lowercase())
                    .collect::<Vec<_>>()
                    .join(" "),
                );
                Some(command)
            })
            .collect()
    }

    /// Every command for the workspace as it is now, group by group, with each row's
    /// keyboard hint from the effective keymap.
    pub fn commands(&self) -> Vec<Command> {
        let mut commands = self.session_commands();
        commands.extend(self.tab_commands_group());
        commands.extend(self.pane_commands());
        commands.extend(self.host_commands());
        let after_host = commands.len();
        commands.extend(self.view_commands());
        commands.extend(self.app_commands());
        for command in &mut commands {
            self.shortcut_details(std::slice::from_mut(&mut command.entry));
        }
        let server = self.server_commands(&commands);
        commands.splice(after_host..after_host, server);
        commands
    }

    /// A group's heading, its name and, where it acts on one, what is on screen; then that
    /// subject alone, which the heading draws in its own case.
    fn group_heading(&self, group: Group) -> (String, Option<String>) {
        let model = self.model();
        let subject = match group {
            Group::Session => self.session_on_screen().map(|s| s.label),
            Group::Tab => model.window().map(|w| super::activity::tab_name(model, w)),
            Group::Pane => self
                .panel
                .as_ref()
                .and_then(|p| p.target.as_ref().map(|t| t.name.clone()))
                .or_else(|| self.focused_block().map(|b| self.pane_name(&b))),
            Group::Host | Group::Server => Some(self.settings.hosts[self.active].label.clone()),
            Group::View | Group::App => None,
        };
        match subject.filter(|s| !s.is_empty()) {
            Some(subject) => (format!("{} · {subject}", group.title()), Some(subject)),
            None => (group.title().to_owned(), None),
        }
    }

    /// The command rows of the list: under their group headings, or the matches of `query`
    /// as one list with the best first.
    pub(super) fn command_entries(&self, query: &str) -> Vec<Entry> {
        let commands = self.commands();
        let query = query.to_lowercase();
        if query.is_empty() {
            return commands
                .into_iter()
                .map(|mut command| {
                    (command.entry.group, command.entry.group_name) =
                        self.group_heading(command.group);
                    command.entry
                })
                .collect();
        }
        let mut matches: Vec<(u8, Entry)> = commands
            .into_iter()
            .filter_map(|mut command| {
                let rank = command.rank(&query)?;
                command.entry.group = "Commands".into();
                Some((rank, command.entry))
            })
            .collect();
        // Stable: equal matches keep the order of the groups.
        matches.sort_by_key(|(rank, _)| *rank);
        matches.into_iter().map(|(_, entry)| entry).collect()
    }
}
