//! Popup and dialog state, and the action lists the switcher, Commands and menus show.
use super::splits::SplitWay;
use super::tabs::{TabAction, TabTarget};
use super::theme::{MONITOR, SERVER};
use super::workspace::Workspace;
use rex_linux::{
    settings::{ChromeTheme, TerminalThemeChoice},
    terminal_theme::{BUILTIN_PREFIX, BUILTINS, paired},
};
use rune::TextEdit;
use serde_json::Value;

#[derive(Clone, Debug)]
pub enum Intent {
    Select(usize, String),
    Tab(String),
    /// A named client action shared with the shortcut dispatcher.
    ClientAction(&'static str, Value),
    Switcher,
    /// Create a session on a host at once, named when a switcher query gave a name.
    CreateOn(usize, Option<String>),
    ChooseHost(String),
    RetryHost(usize),
    /// Review an opt-in, one-time personal server start.
    StartSsh(usize),
    /// The default shell form for a host.
    HostShell(usize),
    Pane(PaneTarget, PaneAction),
    Session(SessionTarget, SessionAction),
    /// A tab action on a named tab of any host and session.
    Window(TabTarget, TabAction),
    NewSession,
    NewWindow,
    Split(SplitWay),
    RenameSession,
    RenameWindow(String),
    CloseWindow(String),
    Zoom,
    Sidebar,
    Settings,
    Fonts,
    ClientLabel,
    RemoteControl(usize),
    /// The Add host list: the tailnet's machines, or an endpoint by hand.
    AddHost,
    /// Attach to the Rex server with built-in Tailscale on a listed machine at once.
    AddMachine(usize),
    /// The SSH form for a listed machine.
    MachineForm(usize),
    /// The endpoint form, from the Add host list.
    AddHostManually,
    EditHost(usize),
    RemoveHost(usize),
    Retry,
    /// The terminal theme picker, from Settings.
    ChooseTheme,
    /// Declare this terminal theme from now on, then return to Settings.
    Theme(TerminalThemeChoice),
    /// The chrome palette picker, from Settings.
    ChooseChrome,
    /// Draw the chrome in this palette from now on, then return to Settings.
    Chrome(ChromeTheme),
    /// Open Settings → Updates.
    Updates,
    /// Trigger an immediate release metadata check.
    CheckUpdates,
    /// Open Settings → About.
    About,
    /// Take this option of the open `Choice` list (a Settings `select_row`).
    Choose(usize),
    /// A Commands row that is not a named action.
    Command(super::commands::Run),
}
impl Intent {
    pub(super) fn shortcut(&self) -> Option<(&'static str, Value)> {
        use serde_json::json;
        Some(match self {
            Self::ClientAction(name, args) => (*name, args.clone()),
            Self::Switcher => ("session.switch", Value::Null),
            Self::NewSession => ("session.new", Value::Null),
            Self::NewWindow => ("client.tab.new", Value::Null),
            Self::RenameSession => ("session.rename", Value::Null),
            Self::RenameWindow(_) => ("client.tab.rename", Value::Null),
            Self::CloseWindow(_) => ("client.tab.close", Value::Null),
            Self::Window(_, tab) => match tab {
                TabAction::Rename => ("client.tab.rename", Value::Null),
                TabAction::Mute => ("client.tab.mute", Value::Null),
                TabAction::MoveLeft => ("client.tab.move", json!({"direction":"left"})),
                TabAction::MoveRight => ("client.tab.move", json!({"direction":"right"})),
                // A new tab in the target's session, which the shortcut's may not be.
                TabAction::New => return None,
                TabAction::CloseOthers => ("client.tab.close_others", Value::Null),
                TabAction::Close => ("client.tab.close", Value::Null),
            },
            Self::Sidebar => ("client.sidebar.toggle", Value::Null),
            Self::Settings => ("client.settings", Value::Null),
            Self::Pane(_, pane) => match pane {
                PaneAction::Split(way) => ("pane.split", json!({"direction":way.arg()})),
                PaneAction::Zoom => ("pane.zoom", Value::Null),
                PaneAction::Reconnect => ("pane.reconnect", Value::Null),
                PaneAction::Close => ("pane.close", Value::Null),
                PaneAction::Copy => ("pane.copy", Value::Null),
                PaneAction::Paste => ("pane.paste", Value::Null),
                PaneAction::SelectAll => ("pane.select_all", Value::Null),
                PaneAction::CopyId => ("pane.copy_id", Value::Null),
                PaneAction::Clear => ("pane.clear", Value::Null),
                PaneAction::Reset => ("pane.reset", Value::Null),
                _ => return None,
            },
            _ => return None,
        })
    }

    /// Actions that open another panel instead of closing the current one, or close it
    /// themselves.
    pub fn opens_followup(&self) -> bool {
        matches!(
            self,
            Intent::Switcher
                | Intent::ClientAction(
                    "client.settings" | "session.switch" | super::about::ABOUT,
                    _
                )
                // Each asks or lists before anything happens.
                | Intent::ClientAction(
                    "client.close" | super::commands::CLOSE_SESSION | super::commands::CHOOSE_THEME,
                    _
                )
                | Intent::ChooseHost(_)
                | Intent::HostShell(_)
                | Intent::StartSsh(_)
                | Intent::RenameSession
                | Intent::RenameWindow(_)
                | Intent::CloseWindow(_)
                | Intent::Window(
                    _,
                    TabAction::Rename | TabAction::CloseOthers | TabAction::Close
                )
                | Intent::Settings
                | Intent::Updates
                | Intent::About
                | Intent::Fonts
                | Intent::ClientLabel
                | Intent::ChooseTheme
                | Intent::ChooseChrome
                | Intent::AddHost
                | Intent::AddMachine(_)
                | Intent::MachineForm(_)
                | Intent::AddHostManually
                | Intent::EditHost(_)
                | Intent::RemoveHost(_)
                | Intent::Pane(_, PaneAction::Rename | PaneAction::Close)
                | Intent::Session(_, SessionAction::Rename | SessionAction::Close)
        )
    }
}

/// The sidebar session a session menu, its rename form or its close confirm acts on: any
/// host's, not only the selected one, so every request names it explicitly.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionTarget {
    pub host: usize,
    pub session: String,
    pub label: String,
}

#[derive(Clone, Copy, Debug)]
pub enum SessionAction {
    Rename,
    MoveUp,
    MoveDown,
    Close,
}

#[derive(Clone, Debug)]
pub struct PaneTarget {
    pub host: usize,
    pub session: String,
    pub block: String,
    /// The server's label for the pane, which Rename starts from.
    pub label: String,
    /// What the pane is called on screen (`activity::pane_name`).
    pub name: String,
    pub tab: String,
}

#[derive(Clone, Copy, Debug)]
pub enum PaneAction {
    Split(SplitWay),
    Zoom,
    Rename,
    Close,
    Reconnect,
    /// The pane's selected text to the clipboard.
    Copy,
    /// The clipboard into the pane, as Ctrl Shift V does for the focused one.
    Paste,
    /// Select the pane's screen and scrollback.
    SelectAll,
    /// The pane's block ID to the clipboard, for `rex` commands that name a block.
    CopyId,
    /// `terminal.clear`: screen and scrollback, keeping the current line.
    Clear,
    /// `terminal.reset`: the terminal's starting state; the program keeps running.
    Reset,
}

#[derive(Clone, Debug)]
pub enum Form {
    Fonts,
    ClientLabel,
    /// A host's default shell for new terminals.
    HostShell(usize),
    /// `None`: the selected session (Commands); `Some`: a sidebar row's session.
    RenameSession(Option<SessionTarget>),
    RenameWindow(String),
    RenamePane(String),
    AddHost,
    /// A reachable endpoint and the server's listener facts.
    Discovered(String),
    /// A listed machine's name and SSH user, from the Add host list.
    AddSsh(usize),
    /// A host's name, endpoint and default shell, from Settings.
    EditHost(usize),
}

/// The options a Settings `select_row` lists in its popup, and what choosing one does.
#[derive(Clone, Debug)]
pub struct Choice {
    pub options: Vec<String>,
    pub current: Option<usize>,
    /// Called with the chosen index after the list has closed.
    pub apply: fn(&mut Workspace, usize, &mut rune::Tx),
}

/// The sections of Settings, in the order its sidebar lists them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsSection {
    #[default]
    Interface,
    Terminal,
    Notifications,
    Integrations,
    Hosts,
    Shortcuts,
    Updates,
    About,
}
impl SettingsSection {
    pub const ALL: [SettingsSection; 8] = [
        SettingsSection::Interface,
        SettingsSection::Terminal,
        SettingsSection::Notifications,
        SettingsSection::Integrations,
        SettingsSection::Hosts,
        SettingsSection::Shortcuts,
        SettingsSection::Updates,
        SettingsSection::About,
    ];
    pub fn title(self) -> &'static str {
        match self {
            SettingsSection::Interface => "Interface",
            SettingsSection::Terminal => "Terminal",
            SettingsSection::Notifications => "Notifications",
            SettingsSection::Integrations => "Integrations",
            SettingsSection::Hosts => "Remote Hosts",
            SettingsSection::Shortcuts => "Shortcuts",
            SettingsSection::Updates => "Updates",
            SettingsSection::About => "About",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            SettingsSection::Interface => "interface",
            SettingsSection::Terminal => "terminal",
            SettingsSection::Notifications => "notifications",
            SettingsSection::Integrations => "integrations",
            SettingsSection::Hosts => "hosts",
            SettingsSection::Shortcuts => "shortcuts",
            SettingsSection::Updates => "updates",
            SettingsSection::About => "about",
        }
    }
}

#[derive(Clone, Debug)]
pub enum PanelKind {
    /// `true`: the session switcher; `false`: Commands.
    Picker(bool),
    Hosts(String),
    Pane,
    Session,
    /// A sidebar host header's right-click menu.
    Host(usize),
    /// A tab's right-click menu; the tab is the panel's `tab`.
    Window,
    Form(Form),
    Settings,
    /// The terminal themes, filtered by the search field.
    Themes,
    /// The chrome palettes.
    Chromes,
    /// The values of one setting, from a Settings `select_row`.
    Choice(Choice),
    /// The Add host list of the tailnet's machines, filtered by the search field.
    Discover,
    /// Confirm forgetting a saved host; its server and programs are untouched.
    RemoveHost(usize),
    Confirm {
        method: String,
        payload: Value,
        description: String,
    },
    /// Captures the endpoint, not a mutable host index.
    StartSsh {
        endpoint: String,
    },
    /// The preview of a `rex skill` change, waiting for Keep or Apply.
    AgentChange {
        change: super::integrations::AgentChange,
        steps: Vec<super::integrations::Step>,
    },
}

pub struct Panel {
    pub kind: PanelKind,
    pub title: String,
    pub inputs: Vec<TextEdit>,
    /// (label, placeholder) for each input.
    pub fields: Vec<(String, String)>,
    pub selected: usize,
    /// Scroll the keyboard selection into view on the next commit.
    pub reveal: bool,
    pub error: Option<String>,
    pub target: Option<PaneTarget>,
    pub session: Option<SessionTarget>,
    /// The tab a tab menu, its rename form or its close confirmations act on.
    pub tab: Option<TabTarget>,
    pub back: Option<Box<Panel>>,
    pub anchor: Option<(f32, f32)>,
    /// The key of the control this list was opened from (a Settings `value_popup`): the list
    /// is placed against it and drawn over the panel in `back`.
    pub anchor_to: Option<String>,
    /// A switcher row being renamed in place, and its field.
    pub rename: Option<(SessionTarget, TextEdit)>,
}

impl Panel {
    pub fn new(kind: PanelKind, title: &str, fields: &[(&str, &str, &str)]) -> Self {
        let mut inputs: Vec<TextEdit> = fields
            .iter()
            .map(|(_, _, text)| TextEdit::new(*text))
            .collect();
        if let Some(first) = inputs.first_mut() {
            first.focused = true;
        }
        Self {
            kind,
            title: title.into(),
            inputs,
            fields: fields
                .iter()
                .map(|(l, p, _)| ((*l).into(), (*p).into()))
                .collect(),
            selected: 0,
            reveal: false,
            error: None,
            target: None,
            session: None,
            tab: None,
            back: None,
            anchor: None,
            anchor_to: None,
            rename: None,
        }
    }
    pub fn is_list(&self) -> bool {
        matches!(
            self.kind,
            PanelKind::Picker(_)
                | PanelKind::Hosts(_)
                | PanelKind::Pane
                | PanelKind::Session
                | PanelKind::Host(_)
                | PanelKind::Window
                | PanelKind::Themes
                | PanelKind::Chromes
                | PanelKind::Choice(_)
                | PanelKind::Discover
        )
    }
}

pub struct Entry {
    pub label: String,
    pub detail: String,
    pub action: Intent,
    pub group: String,
    pub current: bool,
    pub followup: bool,
    pub enabled: bool,
    pub footer: bool,
    /// The group of a menu the row belongs to. A hairline separates one group from the next;
    /// it is drawn by the popup, so a group that is filtered away leaves no stray line.
    pub section: u8,
    /// A destructive row (Close…, Remove…): drawn in `DANGER`, last, after a separator.
    pub danger: bool,
    /// The shortcut `detail` spells, as keycaps (`shortcut_details` fills both).
    pub keys: super::keycaps::Chords,
    /// Drawn before the group header: the host's kind in the switcher.
    pub group_icon: Option<&'static rune::Icon>,
    /// A create button at the right of the group header, with its help text.
    pub group_create: Option<(Intent, String)>,
}
impl Entry {
    pub fn new(
        label: impl Into<String>,
        detail: impl Into<String>,
        group: impl Into<String>,
        action: Intent,
    ) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
            group: group.into(),
            followup: action.opens_followup(),
            action,
            current: false,
            enabled: true,
            footer: false,
            section: 0,
            danger: false,
            keys: Vec::new(),
            group_icon: None,
            group_create: None,
        }
    }

    /// Puts the row in a menu's group `section`.
    pub fn in_section(mut self, section: u8) -> Self {
        self.section = section;
        self
    }

    /// Marks the row destructive.
    pub fn destructive(mut self) -> Self {
        self.danger = true;
        self
    }
}

impl Workspace {
    /// A sidebar session's menu, in groups: Switch (unless current); Rename, Move up and down
    /// within its host; Close, last and destructive. An offline host keeps Switch and offers
    /// Retry and Edit host… before Close, with the rest unavailable.
    fn session_entries(&self, target: &SessionTarget) -> Vec<Entry> {
        let host = &self.hosts[target.host].model;
        let current = self.active == target.host && host.selected.as_ref() == Some(&target.session);
        let position = host
            .sessions
            .iter()
            .position(|s| s.session_id == target.session);
        let mut entries = Vec::new();
        if !current {
            entries.push(Entry::new(
                "Switch to session",
                "",
                "",
                Intent::Select(target.host, target.session.clone()),
            ));
        }
        for (label, action, enabled) in [
            ("Rename session…", SessionAction::Rename, true),
            (
                "Move up",
                SessionAction::MoveUp,
                position.is_some_and(|p| p > 0),
            ),
            (
                "Move down",
                SessionAction::MoveDown,
                position.is_some_and(|p| p + 1 < host.sessions.len()),
            ),
        ] {
            let mut entry =
                Entry::new(label, "", "", Intent::Session(target.clone(), action)).in_section(1);
            entry.enabled = enabled && host.connected;
            entries.push(entry);
        }
        if !host.connected {
            entries.push(
                Entry::new("Retry connection", "", "", Intent::RetryHost(target.host))
                    .in_section(2),
            );
            entries.push(
                Entry::new("Edit host…", "", "", Intent::EditHost(target.host)).in_section(2),
            );
        }
        let mut close = Entry::new(
            "Close session…",
            "",
            "",
            Intent::Session(target.clone(), SessionAction::Close),
        )
        .in_section(3)
        .destructive();
        close.enabled = host.connected;
        entries.push(close);
        entries
    }

    /// A sidebar host header's menu, in groups: New session; Retry while offline and Start
    /// personal server… when it applies; Default shell… and Edit host…; Remove host…, last and
    /// destructive (not for the only host).
    fn host_entries(&self, host: usize) -> Vec<Entry> {
        let connected = self.hosts[host].model.connected;
        let mut create = Entry::new("New session", "", "", Intent::CreateOn(host, None));
        create.enabled = connected;
        let mut entries = vec![create];
        if !connected {
            entries.push(
                Entry::new("Retry connection", "", "", Intent::RetryHost(host)).in_section(1),
            );
        }
        if self.can_start_ssh(host) {
            entries.push(
                Entry::new(
                    "Start personal server…",
                    "Review first",
                    "",
                    Intent::StartSsh(host),
                )
                .in_section(1),
            );
        }
        entries.push(
            Entry::new(
                "Default shell…",
                self.shell(host).unwrap_or_default(),
                "",
                Intent::HostShell(host),
            )
            .in_section(2),
        );
        entries.push(Entry::new("Edit host…", "", "", Intent::EditHost(host)).in_section(2));
        let mut remove = Entry::new("Remove host…", "", "", Intent::RemoveHost(host))
            .in_section(3)
            .destructive();
        remove.enabled = self.settings.hosts.len() > 1;
        entries.push(remove);
        entries
    }

    /// This machine (a Unix socket) or a remote server.
    pub fn host_icon(&self, index: usize) -> &'static rune::Icon {
        if self.settings.hosts[index].endpoint.starts_with("unix://") {
            &MONITOR
        } else {
            &SERVER
        }
    }

    /// The server's own colours, the built-in themes, then the dark and the light Ghostty
    /// themes, each kept when its name contains `lower`.
    fn theme_entries(&self, lower: &str) -> Vec<Entry> {
        let current = &self.settings.terminal_theme;
        let mut entries = Vec::new();
        let mut add = |label: &str, detail: &str, group: &str, choice: TerminalThemeChoice| {
            if label.to_lowercase().contains(lower) {
                let mut e = Entry::new(label, detail, group, Intent::Theme(choice.clone()));
                e.current = *current == choice;
                entries.push(e);
            }
        };
        add(
            "Follow server",
            if self.settings.terminal_theme_declared {
                "Resets to Rex's colours"
            } else {
                "Declares nothing"
            },
            "Server",
            TerminalThemeChoice::FollowServer,
        );
        add(
            "Rex default colours",
            "",
            "Server",
            TerminalThemeChoice::RexDefault,
        );
        add(
            "Follow chrome",
            paired(self.settings.chrome_theme).name,
            "Built in",
            TerminalThemeChoice::FollowChrome,
        );
        for builtin in BUILTINS {
            let id = format!("{BUILTIN_PREFIX}{}", builtin.id);
            add(builtin.name, "", "Built in", TerminalThemeChoice::Named(id));
        }
        for (group, light) in [("Dark", false), ("Light", true)] {
            for theme in self.themes.iter().filter(|t| t.light == light) {
                add(
                    &theme.name,
                    if theme.custom { "Custom" } else { "" },
                    group,
                    TerminalThemeChoice::Named(theme.name.clone()),
                );
            }
        }
        entries
    }

    pub fn entries(&self) -> Vec<Entry> {
        let Some(panel) = &self.panel else {
            return vec![];
        };
        let query = self.query();
        let lower = query.to_lowercase();
        if matches!(panel.kind, PanelKind::Themes) {
            return self.theme_entries(&lower);
        }
        if matches!(panel.kind, PanelKind::Discover) {
            return self.discovery_entries(&lower);
        }
        if matches!(panel.kind, PanelKind::Chromes) {
            return ChromeTheme::ALL
                .into_iter()
                .map(|chrome| {
                    let mut e = Entry::new(chrome.name(), "", "", Intent::Chrome(chrome));
                    e.current = chrome == self.settings.chrome_theme;
                    e
                })
                .collect();
        }
        if let PanelKind::Choice(choice) = &panel.kind {
            return choice
                .options
                .iter()
                .enumerate()
                .map(|(index, option)| {
                    let mut e = Entry::new(option.clone(), "", "", Intent::Choose(index));
                    e.current = choice.current == Some(index);
                    e
                })
                .collect();
        }
        if let PanelKind::Hosts(name) = &panel.kind {
            return self
                .hosts
                .iter()
                .enumerate()
                .map(|(i, h)| {
                    let mut e = Entry::new(
                        self.settings.hosts[i].label.clone(),
                        if h.model.connected {
                            ""
                        } else {
                            "Offline — retry in session menu"
                        },
                        "",
                        Intent::CreateOn(i, (!name.is_empty()).then(|| name.clone())),
                    );
                    e.enabled = h.model.connected;
                    e
                })
                .collect();
        }
        if let (PanelKind::Session, Some(target)) = (&panel.kind, &panel.session) {
            return self.session_entries(target);
        }
        if let PanelKind::Host(host) = panel.kind {
            return self.host_entries(host);
        }
        if let (PanelKind::Window, Some(target)) = (&panel.kind, &panel.tab) {
            let mut entries = self.tab_entries(target);
            // The shortcuts act on the session on screen; another session's tab shows none.
            if self.tab_on_screen(target) {
                self.shortcut_details(&mut entries);
            }
            return entries;
        }
        let sessions_only = matches!(panel.kind, PanelKind::Picker(true));
        let pane_only = matches!(panel.kind, PanelKind::Pane);
        if !matches!(panel.kind, PanelKind::Picker(_) | PanelKind::Pane) {
            return vec![];
        }
        let mut entries = Vec::new();
        let model = self.model();
        let host_name = &self.settings.hosts[self.active].label;
        // A pane's own menu, in groups: clipboard, layout (the four sides and Zoom), the pane
        // itself, its terminal, then Close, last and destructive. Commands lists the same
        // actions and more in its Pane group (`commands.rs`).
        if pane_only
            && let Some(target) = panel
                .target
                .clone()
                .or_else(|| self.focused_block().and_then(|b| self.target(&b)))
        {
            let zoomed = model
                .window()
                .is_some_and(|w| w.zoomed_block_id.as_ref() == Some(&target.block));
            let mut rows = vec![
                ("Copy", 0, PaneAction::Copy),
                ("Paste", 0, PaneAction::Paste),
                ("Select all", 0, PaneAction::SelectAll),
            ];
            // The split that picks a side is in Commands only.
            rows.extend(
                SplitWay::ALL
                    .into_iter()
                    .filter(|way| *way != SplitWay::BestFit)
                    .map(|way| (way.label(), 1, PaneAction::Split(way))),
            );
            rows.extend([
                (
                    if zoomed { "Restore pane" } else { "Zoom pane" },
                    1,
                    PaneAction::Zoom,
                ),
                ("Rename pane…", 2, PaneAction::Rename),
                ("Copy pane ID", 2, PaneAction::CopyId),
                ("Clear terminal", 3, PaneAction::Clear),
                ("Reset terminal", 3, PaneAction::Reset),
                ("Reconnect terminal", 3, PaneAction::Reconnect),
                ("Close pane…", 4, PaneAction::Close),
            ]);
            for (label, section, action) in rows {
                let mut entry = Entry::new(label, "", "", Intent::Pane(target.clone(), action))
                    .in_section(section);
                entry.danger = matches!(action, PaneAction::Close);
                entry.enabled = self.pane_verb_available(&target, action);
                entries.push(entry);
            }
        }
        // A typed switcher query names a new session wherever it is created.
        let named = (!query.is_empty()).then(|| query.clone());
        if !pane_only && (sessions_only || !query.is_empty()) {
            for (index, host) in self.hosts.iter().enumerate() {
                let label = &self.settings.hosts[index].label;
                let create = (sessions_only && host.model.connected).then(|| {
                    (
                        Intent::CreateOn(index, named.clone()),
                        format!("New session on {label}"),
                    )
                });
                let group = if !sessions_only {
                    "Sessions · all hosts".into()
                } else if host.model.connected {
                    label.clone()
                } else {
                    format!("{label} · Offline")
                };
                for session in &host.model.sessions {
                    let detail = if sessions_only {
                        String::new()
                    } else {
                        format!("{label} · Session")
                    };
                    let mut e = Entry::new(
                        &session.label,
                        detail,
                        &group,
                        Intent::Select(index, session.session_id.clone()),
                    );
                    if sessions_only {
                        e.group_icon = Some(self.host_icon(index));
                        e.group_create = create.clone();
                    }
                    e.current = self.active == index
                        && host.model.selected.as_ref() == Some(&session.session_id);
                    if format!("{} {label}", e.label)
                        .to_lowercase()
                        .contains(&lower)
                    {
                        entries.push(e);
                    }
                }
                if sessions_only
                    && (query.is_empty() || label.to_lowercase().contains(&lower))
                    && (!host.model.connected || host.model.sessions.is_empty())
                {
                    let mut e = if host.model.connected {
                        Entry::new(
                            match &named {
                                Some(name) => format!("New session \"{name}\""),
                                None => "New session".into(),
                            },
                            "",
                            group,
                            Intent::CreateOn(index, named.clone()),
                        )
                    } else {
                        Entry::new("Retry connection", "", group, Intent::RetryHost(index))
                    };
                    e.group_icon = Some(self.host_icon(index));
                    e.group_create = create;
                    entries.push(e);
                }
            }
        }
        if !sessions_only
            && !pane_only
            && !query.is_empty()
            && let Some(view) = model.view()
        {
            for tab in &view.windows {
                if let Some(id) = &tab.window_id {
                    let mut e = Entry::new(
                        super::activity::tab_name(model, tab),
                        "Tab",
                        format!(
                            "Tabs · {host_name} / {}",
                            view.label.as_deref().unwrap_or("Session")
                        ),
                        Intent::Tab(id.clone()),
                    );
                    e.current = view.active_window_id.as_ref() == Some(id);
                    entries.push(e);
                }
            }
        }
        self.shortcut_details(&mut entries);
        if !sessions_only {
            entries.retain(|e| {
                format!("{} {} {}", e.label, e.detail, e.group)
                    .to_lowercase()
                    .contains(&lower)
            });
        }
        // Commands: its rows first, then the sessions and tabs a search found.
        if !sessions_only && !pane_only {
            let mut commands = self.command_entries(&query);
            commands.append(&mut entries);
            entries = commands;
        }
        if sessions_only {
            let (create, elsewhere, key) = match &named {
                Some(name) => (
                    format!("New session \"{name}\" on {host_name}"),
                    format!("Create \"{name}\" on another host…"),
                    "",
                ),
                None => (
                    format!("New session on {host_name}"),
                    "Create on another host…".into(),
                    "",
                ),
            };
            for (label, key, action) in [
                (create, key, Intent::CreateOn(self.active, named)),
                (elsewhere, "", Intent::ChooseHost(query.clone())),
                ("Add host…".into(), "", Intent::AddHost),
            ] {
                let mut e = Entry::new(label, key, "", action);
                e.footer = true;
                if matches!(e.action, Intent::CreateOn(..)) {
                    e.enabled = model.connected;
                }
                entries.push(e);
            }
        }
        self.shortcut_details(&mut entries);
        entries
    }

    pub(super) fn shortcut_details(&self, entries: &mut [Entry]) {
        for entry in entries {
            let shortcut = match &entry.action {
                Intent::CreateOn(host, None) if *host == self.active => {
                    Some(("session.new", Value::Null))
                }
                action => action.shortcut(),
            };
            if let Some((name, args)) = shortcut {
                let keys = self.shortcut_chords(name, &args);
                // A pointer hint (Mouse Back) stays until the action is given a key.
                if keys.is_empty() && entry.detail.starts_with("Mouse ") {
                    continue;
                }
                entry.detail = super::keycaps::spell(&keys);
                entry.keys = keys;
            }
        }
    }
}
