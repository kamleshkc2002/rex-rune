//! Named workspace actions shared by keyboard, pointer and Commands dispatch.
//! The registry structure follows Andrea Zito (nivox), commit 8037d78.
use super::{panel::Intent, splits::SplitWay, workspace::Workspace};
use rex_linux::connection::Command;
use rune::Tx;
use serde_json::{Value, json};

pub const SEARCH: &str = "client.search";
pub const SELECT_SESSION: &str = "session.select";
pub const NEXT_GLOBAL: &str = "client.tab.next_global";
pub const PREVIOUS_GLOBAL: &str = "client.tab.previous_global";
pub const HISTORY_BACK: &str = "client.history.back";
pub const HISTORY_FORWARD: &str = "client.history.forward";
pub const DOWNLOAD: &str = "pane.download";

#[derive(Clone, Copy)]
enum Handler {
    Compatibility,
    Palette,
    Settings,
    Sidebar,
    Close,
    Search,
    NewTab,
    NextTab,
    PreviousTab,
    NextGlobal,
    PreviousGlobal,
    Back,
    Forward,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    SwitchSession,
    SelectSession,
    NewSession,
    Split,
    ZoomPane,
    FocusPane,
    PaneActions,
    Copy,
    Paste,
    Reconnect,
    Scroll,
    GotoTab,
    CloseTab,
    RenameTab,
    RenameSession,
    ClosePane,
    SendKey,
    Download,
    SelectAll,
    CopyPaneId,
    ClearTerminal,
    ResetTerminal,
    PaneHeaders,
    FindNext,
    FindPrevious,
    MoveTab,
    MuteTab,
    CloseOtherTabs,
    FocusNextPane,
    FocusPreviousPane,
    FontIncrease,
    FontDecrease,
    FontReset,
    Help,
    CloseSession,
    NextSession,
    PreviousSession,
    RefreshSessions,
    CollapseOtherHosts,
    ChooseTheme,
    CopyVersion,
    ReportIssue,
    About,
    CopyDebug,
}

pub struct Spec {
    pub name: &'static str,
    handler: Handler,
}

macro_rules! specs {
    ($($name:expr => $handler:ident),* $(,)?) => {
        pub static SPECS: &[Spec] = &[$(Spec {name: $name, handler: Handler::$handler}),*];
    };
}

specs! {
    "client.palette.toggle" => Palette,
    "client.settings" => Settings,
    "client.sidebar.toggle" => Sidebar,
    "client.close" => Close,
    SEARCH => Search,
    "client.tab.new" => NewTab,
    "client.tab.next" => NextTab,
    "client.tab.previous" => PreviousTab,
    NEXT_GLOBAL => NextGlobal,
    PREVIOUS_GLOBAL => PreviousGlobal,
    HISTORY_BACK => Back,
    HISTORY_FORWARD => Forward,
    "client.zoom.in" => ZoomIn,
    "client.zoom.out" => ZoomOut,
    "client.zoom.reset" => ZoomReset,
    "session.switch" => SwitchSession,
    SELECT_SESSION => SelectSession,
    "session.new" => NewSession,
    "pane.split" => Split,
    "pane.zoom" => ZoomPane,
    "pane.focus" => FocusPane,
    "pane.actions" => PaneActions,
    "pane.copy" => Copy,
    "pane.paste" => Paste,
    "pane.reconnect" => Reconnect,
    "pane.scroll" => Scroll,
    "client.tab.goto" => GotoTab,
    "client.tab.close" => CloseTab,
    "client.tab.rename" => RenameTab,
    "session.rename" => RenameSession,
    "pane.close" => ClosePane,
    "pane.send_key" => SendKey,
    DOWNLOAD => Download,
    "pane.select_all" => SelectAll,
    "pane.copy_id" => CopyPaneId,
    "pane.clear" => ClearTerminal,
    "pane.reset" => ResetTerminal,
    super::pane_header::TOGGLE => PaneHeaders,
    super::find::NEXT => FindNext,
    super::find::PREVIOUS => FindPrevious,
    "client.tab.move" => MoveTab,
    "client.tab.mute" => MuteTab,
    "client.tab.close_others" => CloseOtherTabs,
    super::splits::FOCUS_NEXT => FocusNextPane,
    super::splits::FOCUS_PREVIOUS => FocusPreviousPane,
    super::font_size::INCREASE => FontIncrease,
    super::font_size::DECREASE => FontDecrease,
    super::font_size::RESET => FontReset,
    super::help::ACTION => Help,
    super::commands::CLOSE_SESSION => CloseSession,
    super::commands::NEXT_SESSION => NextSession,
    super::commands::PREVIOUS_SESSION => PreviousSession,
    super::commands::REFRESH_SESSIONS => RefreshSessions,
    super::commands::COLLAPSE_OTHER_HOSTS => CollapseOtherHosts,
    super::commands::CHOOSE_THEME => ChooseTheme,
    super::commands::COPY_VERSION => CopyVersion,
    super::commands::REPORT_ISSUE => ReportIssue,
    super::about::ABOUT => About,
    super::about::COPY_DEBUG => CopyDebug,
    "client.config.reload" => Compatibility,
    "client.config.copy_path" => Compatibility,
    "client.prompt.previous" => Compatibility,
    "client.prompt.next" => Compatibility,
    "pane.resize" => Compatibility,
    "pane.balance" => Compatibility,
    "pane.rename" => Compatibility,
    "pane.close_others" => Compatibility,
    "pane.move" => Compatibility,
    "pane.move.left" => Compatibility,
    "pane.move.right" => Compatibility,
    "pane.move.up" => Compatibility,
    "pane.move.down" => Compatibility,
    "pane.move_to_tab" => Compatibility,
    "pane.move_to_new_tab" => Compatibility,
    "pane.focus.left" => Compatibility,
    "pane.focus.right" => Compatibility,
    "pane.focus.up" => Compatibility,
    "pane.focus.down" => Compatibility,
    "pane.split.left" => Compatibility,
    "pane.split.right" => Compatibility,
    "pane.split.up" => Compatibility,
    "pane.split.down" => Compatibility,
    "pane.split.auto" => Compatibility,
    "client.find.open" => Compatibility,
    "client.find.next" => Compatibility,
    "client.find.previous" => Compatibility,
    "client.tab.move.backward" => Compatibility,
    "client.tab.move.forward" => Compatibility,
    "client.host.switch" => Compatibility,
    "client.host.add" => Compatibility,
    "client.host.remove" => Compatibility,
    "client.theme.change" => Compatibility,
    "client.mode.enter" => Compatibility,
    "client.mode.exit" => Compatibility,
    "client.mode.exit_all" => Compatibility,
    "client.scroll.page_up" => Compatibility,
    "client.scroll.page_down" => Compatibility,
}

pub fn find(name: &str) -> Option<&'static Spec> {
    SPECS.iter().find(|spec| spec.name == name)
}

/// Exhaustive by handler: new actions require an explicit remote decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Remote {
    View { navigation: bool },
    Never(&'static str),
}

impl Spec {
    pub fn remote(&self) -> Remote {
        use Handler::*;
        match self.handler {
            Compatibility => Remote::Never(
                "Compatibility actions are available only to the person at this computer.",
            ),
            NextTab | PreviousTab | GotoTab | FocusPane | SelectSession => {
                Remote::View { navigation: true }
            }
            Sidebar | ZoomPane | Scroll => Remote::View { navigation: false },
            Paste => Remote::Never(
                "Use rex send instead; remote actions cannot read this computer's clipboard.",
            ),
            Copy => Remote::Never(
                "Use rex capture instead; remote actions cannot change this computer's clipboard.",
            ),
            SendKey => Remote::Never("Use rex send-key instead."),
            NewSession => Remote::Never("Use rex new instead."),
            NewTab => Remote::Never("Use rex window new instead."),
            Split => Remote::Never("Use rex split instead."),
            Close | ClosePane => Remote::Never("Use rex block close instead."),
            CloseTab | CloseOtherTabs => Remote::Never("Use rex window close instead."),
            MoveTab => Remote::Never("Use rex window move instead."),
            MuteTab => Remote::Never(
                "Only the person at this computer can mute this client's notifications.",
            ),
            RenameTab => Remote::Never("Use rex window rename instead."),
            RenameSession => Remote::Never("Use rex session rename instead."),
            Download => Remote::Never(
                "Only the person at this computer can choose a file to download; nothing a host sends can start one.",
            ),
            Palette | Settings | Search | SwitchSession | PaneActions => {
                Remote::Never("Only the person at this computer can open dialogs and popups.")
            }
            ZoomIn | ZoomOut | ZoomReset => Remote::Never(
                "Only the person at this computer can change saved interface preferences.",
            ),
            NextGlobal | PreviousGlobal | Back | Forward => {
                Remote::Never("Use rex do session.select on the target host instead.")
            }
            Reconnect => Remote::Never("Only the person at this computer can reconnect terminals."),
            SelectAll => {
                Remote::Never("Only the person at this computer can select terminal text.")
            }
            CopyPaneId => Remote::Never("Remote actions cannot change this computer's clipboard."),
            ClearTerminal | ResetTerminal => Remote::Never(
                "Only the person at this computer can clear or reset terminals from here.",
            ),
            PaneHeaders => {
                Remote::Never("Only the person at this computer can show or hide pane headers.")
            }
            FindNext | FindPrevious => {
                Remote::Never("Only the person at this computer can search a terminal.")
            }
            FocusNextPane | FocusPreviousPane => {
                Remote::Never("Use pane.focus with a direction instead.")
            }
            FontIncrease | FontDecrease | FontReset => Remote::Never(
                "Only the person at this computer can change saved terminal preferences.",
            ),
            Help | ReportIssue => {
                Remote::Never("Only the person at this computer can open its browser.")
            }
            CloseSession => Remote::Never("Use rex session kill instead."),
            NextSession | PreviousSession => {
                Remote::Never("Use rex do session.select on the target host instead.")
            }
            RefreshSessions => Remote::Never("Use rex session ls instead."),
            CollapseOtherHosts => Remote::Never(
                "Only the person at this computer can change saved interface preferences.",
            ),
            ChooseTheme => {
                Remote::Never("Only the person at this computer can open dialogs and popups.")
            }
            CopyVersion | CopyDebug => {
                Remote::Never("Remote actions cannot change this computer's clipboard.")
            }
            About => Remote::Never("Only the person at this computer can open dialogs and popups."),
        }
    }
    /// An action only this person's own key or click may run: a returned client action must
    /// not start it.
    pub fn starts_download(&self) -> bool {
        matches!(self.handler, Handler::Download)
    }
    pub fn global(&self) -> bool {
        matches!(
            self.handler,
            Handler::ZoomIn | Handler::ZoomOut | Handler::ZoomReset
        )
    }
    pub fn repeats(&self) -> bool {
        matches!(
            self.name,
            "pane.resize"
                | "client.scroll.page_up"
                | "client.scroll.page_down"
                | "client.find.next"
                | "client.find.previous"
                | "pane.focus.left"
                | "pane.focus.right"
                | "pane.focus.up"
                | "pane.focus.down"
        ) || self.global()
            || matches!(
                self.handler,
                Handler::FocusPane
                    | Handler::NextTab
                    | Handler::PreviousTab
                    | Handler::Scroll
                    | Handler::NextGlobal
                    | Handler::PreviousGlobal
                    | Handler::FindNext
                    | Handler::FindPrevious
                    | Handler::FocusNextPane
                    | Handler::FocusPreviousPane
                    | Handler::FontIncrease
                    | Handler::FontDecrease
            )
    }

    pub fn title(&self) -> &'static str {
        match self.handler {
            Handler::Compatibility => super::compatibility::title(self.name),
            Handler::Palette => "Commands",
            Handler::Settings => "Settings",
            Handler::Sidebar => "Toggle sidebar",
            Handler::Close => "Close pane or tab",
            Handler::Search => "Search terminal",
            Handler::NewTab => "New tab",
            Handler::NextTab => "Next tab",
            Handler::PreviousTab => "Previous tab",
            Handler::NextGlobal => "Next tab across hosts",
            Handler::PreviousGlobal => "Previous tab across hosts",
            Handler::Back => "Back in history",
            Handler::Forward => "Forward in history",
            Handler::ZoomIn => "Increase interface zoom",
            Handler::ZoomOut => "Decrease interface zoom",
            Handler::ZoomReset => "Reset interface zoom",
            Handler::SwitchSession => "Switch session",
            Handler::SelectSession => "Show session",
            Handler::NewSession => "New session",
            Handler::Split => "Split pane",
            Handler::ZoomPane => "Zoom pane",
            Handler::FocusPane => "Focus pane",
            Handler::PaneActions => "Pane actions",
            Handler::Copy => "Copy selection",
            Handler::Paste => "Paste",
            Handler::Reconnect => "Reconnect terminal",
            Handler::Scroll => "Scroll terminal",
            Handler::GotoTab => "Go to tab",
            Handler::CloseTab => "Close tab",
            Handler::RenameTab => "Rename tab",
            Handler::RenameSession => "Rename session",
            Handler::ClosePane => "Close pane",
            Handler::SendKey => "Send key",
            Handler::Download => "Download file",
            Handler::SelectAll => "Select all",
            Handler::CopyPaneId => "Copy pane ID",
            Handler::ClearTerminal => "Clear terminal",
            Handler::ResetTerminal => "Reset terminal",
            Handler::PaneHeaders => "Reveal pane headers",
            Handler::FindNext => "Find next",
            Handler::FindPrevious => "Find previous",
            Handler::MoveTab => "Move tab",
            Handler::MuteTab => "Mute or unmute tab",
            Handler::CloseOtherTabs => "Close other tabs",
            Handler::FocusNextPane => "Focus next pane",
            Handler::FocusPreviousPane => "Focus previous pane",
            Handler::FontIncrease => "Increase terminal font size",
            Handler::FontDecrease => "Decrease terminal font size",
            Handler::FontReset => "Reset terminal font size",
            Handler::Help => "Rex help",
            Handler::CloseSession => "Close session",
            Handler::NextSession => "Next session",
            Handler::PreviousSession => "Previous session",
            Handler::RefreshSessions => "Refresh sessions",
            Handler::CollapseOtherHosts => "Collapse other hosts",
            Handler::ChooseTheme => "Change terminal theme",
            Handler::CopyVersion => "Copy version info",
            Handler::ReportIssue => "Report an issue",
            Handler::About => "About Rex Linux",
            Handler::CopyDebug => "Copy debug info",
        }
    }
    pub fn category(&self) -> &'static str {
        match self.handler {
            Handler::Compatibility => super::compatibility::category(self.name),
            Handler::Palette
            | Handler::Settings
            | Handler::Sidebar
            | Handler::ZoomIn
            | Handler::ZoomOut
            | Handler::ZoomReset
            | Handler::PaneHeaders
            | Handler::Help
            | Handler::CollapseOtherHosts
            | Handler::ChooseTheme
            | Handler::CopyVersion
            | Handler::ReportIssue
            | Handler::About
            | Handler::CopyDebug => "Interface",
            Handler::NextTab
            | Handler::PreviousTab
            | Handler::NextGlobal
            | Handler::PreviousGlobal
            | Handler::Back
            | Handler::Forward
            | Handler::SwitchSession
            | Handler::SelectSession
            | Handler::GotoTab
            | Handler::NextSession
            | Handler::PreviousSession
            | Handler::RefreshSessions => "Navigate",
            Handler::NewTab
            | Handler::NewSession
            | Handler::Close
            | Handler::CloseTab
            | Handler::RenameTab
            | Handler::RenameSession
            | Handler::ClosePane
            | Handler::MoveTab
            | Handler::MuteTab
            | Handler::CloseOtherTabs
            | Handler::CloseSession => "Create and close",
            Handler::Split
            | Handler::ZoomPane
            | Handler::FocusPane
            | Handler::PaneActions
            | Handler::Download
            | Handler::Reconnect
            | Handler::CopyPaneId
            | Handler::ClearTerminal
            | Handler::ResetTerminal
            | Handler::FocusNextPane
            | Handler::FocusPreviousPane => "Panes",
            Handler::Copy
            | Handler::Paste
            | Handler::SelectAll
            | Handler::Scroll
            | Handler::SendKey
            | Handler::Search
            | Handler::FindNext
            | Handler::FindPrevious
            | Handler::FontIncrease
            | Handler::FontDecrease
            | Handler::FontReset => "Text and view",
        }
    }
    pub(super) fn schema(&self) -> Option<Value> {
        if matches!(self.handler, Handler::Compatibility) {
            return super::compatibility::schema(self.name);
        }
        if matches!(self.handler, Handler::SelectSession) {
            return Some(
                json!({"type":"object","properties":{"session_id":{"type":"string","minLength":1},"window_id":{"type":"string","minLength":1}},"required":["session_id"],"additionalProperties":false}),
            );
        }
        let (key, schema) = match self.handler {
            Handler::Split => (
                "direction",
                json!({"type":"string","enum":SplitWay::ALL.map(SplitWay::arg)}),
            ),
            Handler::FocusPane => (
                "direction",
                json!({"type":"string","enum":["left","right","up","down"]}),
            ),
            Handler::Scroll => (
                "lines",
                json!({"type":"integer","minimum":i32::MIN,"maximum":i32::MAX}),
            ),
            Handler::GotoTab => ("index", json!({"type":"integer","minimum":-1})),
            Handler::SendKey => ("key", json!({"type":"string"})),
            Handler::MoveTab => (
                "direction",
                json!({"type":"string","enum":["left","right","backward","forward"]}),
            ),
            _ => return None,
        };
        Some(
            json!({"type":"object","properties":{key:schema},"required":[key],"additionalProperties":false}),
        )
    }

    /// Validate before dispatch: invalid arguments must not partially perform an action.
    pub fn accepts(&self, args: &Value) -> bool {
        if matches!(self.handler, Handler::Compatibility) {
            return super::compatibility::accepts(self.name, args);
        }
        let Some(object) = args.as_object() else {
            return args.is_null()
                && !matches!(
                    self.handler,
                    Handler::SelectSession
                        | Handler::Split
                        | Handler::FocusPane
                        | Handler::Scroll
                        | Handler::GotoTab
                        | Handler::SendKey
                        | Handler::MoveTab
                );
        };
        match self.handler {
            Handler::SelectSession => {
                object.keys().all(|k| k == "session_id" || k == "window_id")
                    && args["session_id"].as_str().is_some_and(|id| !id.is_empty())
                    && object
                        .get("window_id")
                        .is_none_or(|id| id.as_str().is_some_and(|id| !id.is_empty()))
            }
            Handler::Split => object.len() == 1 && split_way(args).is_some(),
            Handler::FocusPane => {
                object.len() == 1
                    && matches!(
                        args["direction"].as_str(),
                        Some("left" | "right" | "up" | "down")
                    )
            }
            Handler::Scroll => {
                object.len() == 1
                    && args["lines"]
                        .as_i64()
                        .is_some_and(|n| i32::try_from(n).is_ok())
            }
            Handler::GotoTab => {
                object.len() == 1
                    && args["index"]
                        .as_i64()
                        .is_some_and(|n| n == -1 || n > 0 && usize::try_from(n).is_ok())
            }
            Handler::MoveTab => {
                object.len() == 1
                    && matches!(
                        args["direction"].as_str(),
                        Some("left" | "right" | "backward" | "forward")
                    )
            }
            Handler::SendKey => {
                object.len() == 1
                    && args["key"]
                        .as_str()
                        .is_some_and(|key| super::keys::synthesize(key).is_some())
            }
            _ => object.is_empty(),
        }
    }
}

/// False means unknown action or invalid args, not that an offline action failed.
pub fn run(s: &mut Workspace, name: &str, args: &Value, tx: &mut Tx) -> bool {
    let Some(spec) = find(name).filter(|spec| spec.accepts(args)) else {
        return false;
    };
    match spec.handler {
        Handler::Compatibility => super::compatibility::run(s, name, args, tx),
        Handler::Palette => s.picker(false),
        Handler::Settings => s.perform(Intent::Settings, tx),
        Handler::Sidebar => s.perform(Intent::Sidebar, tx),
        Handler::PaneHeaders => s.toggle_pane_headers(),
        Handler::Close => s.close_focused(tx),
        Handler::Search => s.open_search(),
        Handler::FindNext => s.find_step(true),
        Handler::FindPrevious => s.find_step(false),
        Handler::NewTab => s.perform(Intent::NewWindow, tx),
        Handler::NextTab => s.request("session.focus_next_window", json!({})),
        Handler::PreviousTab => s.request("session.focus_previous_window", json!({})),
        Handler::NextGlobal => s.cycle_tab(1, tx),
        Handler::PreviousGlobal => s.cycle_tab(-1, tx),
        Handler::Back => s.navigate_history(-1, tx),
        Handler::Forward => s.navigate_history(1, tx),
        Handler::ZoomIn => s.change_ui_scale(1, tx),
        Handler::ZoomOut => s.change_ui_scale(-1, tx),
        Handler::ZoomReset => s.change_ui_scale(0, tx),
        Handler::SwitchSession => s.picker(true),
        Handler::SelectSession => {
            let visit = s.session_visit(s.active, args);
            match visit {
                Ok(visit) => s.navigate_to(visit, tx),
                Err(error) => s.inform(format!("Session selection failed: {error:?}")),
            }
        }
        Handler::NewSession => s.perform(Intent::NewSession, tx),
        Handler::Split => {
            if let Some(way) = split_way(args) {
                s.perform(Intent::Split(way), tx);
            }
        }
        Handler::ZoomPane => s.perform(Intent::Zoom, tx),
        Handler::FocusPane => s.request("session.focus_direction", args.clone()),
        Handler::PaneActions => {
            if let Some(block) = s.focused_block() {
                s.pane_menu(block, None);
            }
        }
        Handler::Copy => {
            if let Some(text) = s.active_terminal().and_then(|t| t.copy_text()) {
                tx.copy(text);
            }
        }
        Handler::Paste => s.paste_clipboard(tx),
        Handler::Reconnect => {
            if let Some(term) = s.active_terminal_mut() {
                term.send(Command::Reconnect);
            }
        }
        Handler::Scroll => {
            if let Some(term) = s.active_terminal_mut() {
                term.send(Command::Scroll(args["lines"].as_i64().unwrap() as isize));
            }
        }
        Handler::GotoTab => {
            let index = tab_index(args, s.model().view().map_or(0, |v| v.windows.len()));
            if let Some(id) = s
                .model()
                .view()
                .and_then(|v| v.windows.get(index?))
                .and_then(|w| w.window_id.clone())
            {
                s.perform(Intent::Tab(id), tx);
            }
        }
        Handler::CloseTab | Handler::RenameTab => {
            if let Some(id) = s.model().window().and_then(|w| w.window_id.clone()) {
                s.perform(
                    if matches!(spec.handler, Handler::CloseTab) {
                        Intent::CloseWindow(id)
                    } else {
                        Intent::RenameWindow(id)
                    },
                    tx,
                );
            }
        }
        Handler::RenameSession => s.perform(Intent::RenameSession, tx),
        Handler::ClosePane => {
            if let Some(target) = s.focused_block().and_then(|b| s.target(&b)) {
                s.perform(Intent::Pane(target, super::panel::PaneAction::Close), tx);
            }
        }
        Handler::Download => {
            if let Some(target) = s.focused_block().and_then(|b| s.target(&b)) {
                s.perform(Intent::Pane(target, super::panel::PaneAction::Download), tx);
            }
        }
        Handler::SelectAll
        | Handler::CopyPaneId
        | Handler::ClearTerminal
        | Handler::ResetTerminal => {
            use super::panel::PaneAction;
            let verb = match spec.handler {
                Handler::SelectAll => PaneAction::SelectAll,
                Handler::CopyPaneId => PaneAction::CopyId,
                Handler::ClearTerminal => PaneAction::Clear,
                _ => PaneAction::Reset,
            };
            if let Some(target) = s.focused_block().and_then(|b| s.target(&b)) {
                s.perform(Intent::Pane(target, verb), tx);
            }
        }
        Handler::MoveTab | Handler::MuteTab | Handler::CloseOtherTabs => {
            use super::tabs::TabAction;
            let action = match spec.handler {
                Handler::MuteTab => TabAction::Mute,
                Handler::CloseOtherTabs => TabAction::CloseOthers,
                _ if matches!(args["direction"].as_str(), Some("left" | "backward")) => {
                    TabAction::MoveLeft
                }
                _ => TabAction::MoveRight,
            };
            if let Some(target) = s.active_tab() {
                s.perform(Intent::Window(target, action), tx);
            }
        }
        Handler::FocusNextPane => s.cycle_pane(true),
        Handler::FocusPreviousPane => s.cycle_pane(false),
        Handler::FontIncrease => s.step_font_size(1),
        Handler::FontDecrease => s.step_font_size(-1),
        Handler::FontReset => s.step_font_size(0),
        Handler::Help => s.open_link(super::help::url()),
        Handler::CloseSession => s.close_session_on_screen(tx),
        Handler::NextSession => s.cycle_session(1, tx),
        Handler::PreviousSession => s.cycle_session(-1, tx),
        Handler::RefreshSessions => s.refresh_sessions(),
        Handler::CollapseOtherHosts => s.collapse_other_hosts(),
        Handler::ChooseTheme => s.perform(Intent::ChooseTheme, tx),
        Handler::CopyVersion => s.copy_version_info(tx),
        Handler::CopyDebug => s.copy_debug_info(tx),
        Handler::About => s.perform(Intent::About, tx),
        Handler::ReportIssue => s.open_link(super::commands::issue_url()),
        Handler::SendKey => {
            if let Some(payloads) = args["key"].as_str().and_then(super::keys::synthesize)
                && let Some(term) = s.active_terminal_mut()
            {
                for payload in payloads {
                    term.send(Command::SendKey(payload));
                }
            }
        }
    }
    true
}

fn split_way(args: &Value) -> Option<SplitWay> {
    args["direction"].as_str().and_then(SplitWay::parse)
}

pub fn tab_index(args: &Value, count: usize) -> Option<usize> {
    match args["index"].as_i64()? {
        -1 => count.checked_sub(1),
        n if n > 0 => usize::try_from(n - 1).ok().filter(|n| *n < count),
        _ => None,
    }
}

#[derive(Clone, Copy)]
pub enum Kind {
    Local(&'static Spec),
    Forward,
    Server { repeats: bool },
    Skip,
}
pub fn classify(
    binding: &rexrs::api::client::Binding,
    server_actions: &[rexrs::api::client::InspectResponseAction],
) -> Kind {
    if binding.action == "client.key.forward" {
        return Kind::Forward;
    }
    if let Some(spec) = find(&binding.action) {
        return if spec.accepts(binding.args.as_ref().unwrap_or(&Value::Null)) {
            Kind::Local(spec)
        } else {
            Kind::Skip
        };
    }
    match server_actions
        .iter()
        .find(|a| a.name == binding.action && matches!(a.kind.as_deref(), Some("lua" | "block")))
    {
        Some(action) => Kind::Server {
            repeats: action.repeats.unwrap_or(false),
        },
        None => Kind::Skip,
    }
}

/// Bindings that can actually win, including prefix-vs-key conflicts. Parsed chords,
/// not display strings, decide equality. Mode bindings compete only within their mode.
pub fn effective<'a>(
    map: &'a rex_linux::keymap::Resolved,
    server: &[rexrs::api::client::InspectResponseAction],
) -> Vec<&'a rex_linux::keymap::ResolvedBinding> {
    let mut result: Vec<&rex_linux::keymap::ResolvedBinding> = Vec::new();
    for candidate in &map.bindings {
        let Some(chords) = candidate.chords.as_ref() else {
            continue;
        };
        if matches!(classify(&candidate.binding, server), Kind::Skip) {
            continue;
        }
        if result.iter().any(|higher| {
            if higher.binding.mode != candidate.binding.mode {
                return false;
            }
            let keys = higher.chords.as_ref().unwrap();
            keys.starts_with(chords) || chords.starts_with(keys)
        }) {
            continue;
        }
        result.push(candidate);
    }
    result
}

/// What a binding runs, as Commands and the menus name it: an action that takes a direction
/// or a place reads as their row for it (`Split down`, `Focus pane left`, `Move tab right`).
pub fn binding_title(binding: &rexrs::api::client::Binding) -> String {
    let title = find(&binding.action)
        .map(|s| s.title())
        .unwrap_or(&binding.action);
    let Some(args) = &binding.args else {
        return title.into();
    };
    // `best_fit` reads as two words.
    let direction = || {
        args["direction"]
            .as_str()
            .unwrap_or_default()
            .replace('_', " ")
    };
    match binding.action.as_str() {
        "pane.split" => format!("Split {}", direction()),
        "pane.focus" => format!("Focus pane {}", direction()),
        "client.tab.move" => format!("Move tab {}", direction()),
        "pane.scroll" => {
            let lines = args["lines"].as_i64().unwrap_or_default();
            let direction = if lines < 0 { "up" } else { "down" };
            format!("Scroll {direction} · {} lines", lines.unsigned_abs())
        }
        "client.tab.goto" => format!("Go to tab {}", args["index"]),
        "pane.send_key" => format!("{title} · {}", args["key"].as_str().unwrap_or_default()),
        _ => title.into(),
    }
}

pub fn declarations() -> Vec<rexrs::api::client::InspectResponseAction> {
    SPECS
        .iter()
        .map(|s| {
            let mut action = rexrs::api::client::InspectResponseAction::new(s.name)
                .kind("client")
                .title(s.title())
                .category(s.category())
                .keywords(s.title())
                .repeats(s.repeats());
            action.args = s.schema();
            if let Remote::Never(message) = s.remote() {
                action.description = Some(format!(
                    "{}Not available remotely. {message}",
                    super::compatibility::unsupported(s.name)
                        .map(|reason| format!("{reason} "))
                        .unwrap_or_default()
                ));
            }
            action
        })
        .collect()
}

/// Resolve once per file generation, including argument diagnostics. Invalid bindings
/// are skipped at match time, so a lower valid action can still be selected.
pub fn resolve(overrides: &rex_linux::keymap::Overrides) -> rex_linux::keymap::Resolved {
    let actions = declarations();
    let mut map = rex_linux::keymap::resolve(&[rex_linux::keymap::defaults()], overrides, &actions);
    diagnose_compatibility(&mut map);
    for binding in &map.bindings {
        let binding = &binding.binding;
        if find(&binding.action)
            .is_some_and(|spec| !spec.accepts(binding.args.as_ref().unwrap_or(&Value::Null)))
        {
            map.diagnostics.push(rexrs::api::client::Diagnostic {
                severity: "warning".into(),
                source: binding.source.clone(),
                message: format!("invalid arguments for {:?}", binding.action),
                action: Some(binding.action.clone()),
                key: Some(binding.keys.join(" ")),
                mode: None,
            });
        }
    }
    map
}

/// Client-specific limitations must also appear for bindings returned by a server.
pub fn diagnose_compatibility(map: &mut rex_linux::keymap::Resolved) {
    for row in &map.bindings {
        if let Some(message) = super::compatibility::unsupported(&row.binding.action)
            && !map.diagnostics.iter().any(|d| {
                d.message == message
                    && d.action.as_ref() == Some(&row.binding.action)
                    && d.source == row.binding.source
                    && d.key.as_deref() == Some(&row.binding.keys.join(" "))
            })
        {
            map.diagnostics.push(rexrs::api::client::Diagnostic {
                severity: "warning".into(),
                source: row.binding.source.clone(),
                message: message.into(),
                action: Some(row.binding.action.clone()),
                key: Some(row.binding.keys.join(" ")),
                mode: row.binding.mode.clone(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_unique_and_covers_every_default() {
        let names: std::collections::HashSet<_> = SPECS.iter().map(|s| s.name).collect();
        assert_eq!(names.len(), SPECS.len());
        for binding in rex_linux::keymap::defaults().bindings {
            let spec = find(&binding.action).expect("default action is registered");
            assert!(
                spec.accepts(binding.args.as_ref().unwrap_or(&Value::Null)),
                "{}",
                binding.action
            );
        }
    }

    #[test]
    fn arguments_are_checked_before_dispatch() {
        for name in ["pane.split", "pane.focus", "pane.scroll"] {
            let spec = find(name).unwrap();
            assert!(!spec.accepts(&Value::Null));
            assert!(!spec.accepts(&json!({})));
        }
        assert!(
            !find("pane.split")
                .unwrap()
                .accepts(&json!({"direction":"sideways"}))
        );
        assert!(!find("pane.scroll").unwrap().accepts(&json!({"lines":1.5})));
        assert!(
            !find("pane.scroll")
                .unwrap()
                .accepts(&json!({"lines":i64::MAX}))
        );
        assert!(!find(SEARCH).unwrap().accepts(&json!({"typo":true})));
        assert!(find(SEARCH).unwrap().accepts(&Value::Null));
        assert!(find(SEARCH).unwrap().accepts(&json!({})));
        assert!(find("client.nothing").is_none());
    }
}
