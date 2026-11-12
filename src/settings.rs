pub mod interface;
pub mod recorded;
pub mod shortcuts;

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Host {
    pub label: String,
    pub endpoint: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub remote_control: bool,
    #[serde(default)]
    pub collapsed: bool,
    /// The shell new terminals on this host start (`zsh`, `/bin/zsh`); none uses the server
    /// user's login shell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
}

/// A tab this client keeps quiet: no activity word or progress bar on it and no desktop
/// notification from it. Local to this client; the server and other clients never see it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutedTab {
    /// The saved host's endpoint.
    pub endpoint: String,
    pub session: String,
    /// The Rex window id, unique on its server.
    pub window: String,
}

/// The window corner in-app notices stack in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoticePosition {
    #[default]
    TopRight,
    BottomRight,
}

/// The space and decorations around terminal panes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneDensity {
    Compact,
    #[default]
    Comfortable,
}

/// When a pane shows its header: its name, its program's activity and the Split, Zoom and
/// Close buttons. A value this version doesn't know reads as Auto.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneHeader {
    /// On every pane, a single one included, in either density.
    Always,
    /// Over the top of a pane while the pointer rests there; the terminal keeps its size.
    OnHover,
    Hidden,
    /// With the density: on the cards of a split or zoomed tab in Comfortable, never in
    /// Compact, and a single pane stays edge to edge.
    #[default]
    #[serde(other)]
    Auto,
}

/// What program activity shows in tabs and which changes become desktop notifications.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationsConfig {
    pub show_in_tabs: bool,
    pub system_notifications: bool,
    /// Notify when a background tab's programs move into each state.
    pub working: bool,
    pub blocked: bool,
    pub error: bool,
    pub done: bool,
    /// Pass on OSC 9 and OSC 777 notifications that programs send themselves.
    pub program_messages: bool,
}
impl Default for NotificationsConfig {
    fn default() -> Self {
        Self {
            show_in_tabs: true,
            system_notifications: true,
            working: false,
            blocked: true,
            error: true,
            done: true,
            program_messages: true,
        }
    }
}

/// Whether the client checks for signed release metadata in the background.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdatesConfig {
    pub check: bool,
}
impl Default for UpdatesConfig {
    fn default() -> Self {
        Self { check: true }
    }
}

/// Where downloaded files are saved.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadsConfig {
    /// The folder, absolute or starting with `~`, as the person wrote it. None is the XDG
    /// download folder (`xdg-user-dir DOWNLOAD`, else `~/Downloads`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
}

/// The palette the chrome is drawn in. A value this version doesn't know reads as Midnight.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChromeTheme {
    RosePine,
    RosePineMoon,
    GruvboxDark,
    #[default]
    #[serde(other)]
    Midnight,
}
impl ChromeTheme {
    pub const ALL: [ChromeTheme; 4] = [
        ChromeTheme::Midnight,
        ChromeTheme::RosePine,
        ChromeTheme::RosePineMoon,
        ChromeTheme::GruvboxDark,
    ];
    pub fn name(self) -> &'static str {
        match self {
            ChromeTheme::Midnight => "Midnight",
            ChromeTheme::RosePine => "Rosé Pine",
            ChromeTheme::RosePineMoon => "Rosé Pine Moon",
            ChromeTheme::GruvboxDark => "Gruvbox Dark",
        }
    }
}

/// The terminal theme this client declares to the server, saved as `follow_server`,
/// `builtin_default`, `follow_chrome` or `named:<id>`. An id is a Ghostty theme's file name,
/// or a built-in theme's after `rex/`. A value this version doesn't know reads as Follow
/// server.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum TerminalThemeChoice {
    /// Declare nothing: panes show whatever colours the server gives them.
    #[default]
    FollowServer,
    /// Declare Rex's built-in colours.
    RexDefault,
    /// Declare the built-in theme paired with the chrome palette, whose background is the
    /// chrome's ground.
    FollowChrome,
    Named(String),
}
impl From<String> for TerminalThemeChoice {
    fn from(value: String) -> Self {
        match value.as_str() {
            "builtin_default" => Self::RexDefault,
            "follow_chrome" => Self::FollowChrome,
            _ => match value.strip_prefix("named:") {
                Some(id) if !id.is_empty() => Self::Named(id.to_owned()),
                _ => Self::FollowServer,
            },
        }
    }
}
impl From<TerminalThemeChoice> for String {
    fn from(choice: TerminalThemeChoice) -> Self {
        match choice {
            TerminalThemeChoice::FollowServer => "follow_server".into(),
            TerminalThemeChoice::RexDefault => "builtin_default".into(),
            TerminalThemeChoice::FollowChrome => "follow_chrome".into(),
            TerminalThemeChoice::Named(id) => format!("named:{id}"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub hosts: Vec<Host>,
    /// None follows the current hostname rather than persisting a snapshot of it.
    pub client_label: Option<String>,
    pub last_host: Option<String>,
    pub sessions: HashMap<String, String>,
    pub sidebar_width: f32,
    /// The sidebar was open when the app last saved, so it opens again at launch.
    pub sidebar_open: bool,
    /// The last non-maximized window size in window points, independent of interface zoom.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_size: Option<[f32; 2]>,
    pub interface: interface::InterfaceConfig,
    pub vertical_tabs: bool,
    pub pane_density: PaneDensity,
    pub pane_header: PaneHeader,
    pub pane_padding: f32,
    pub font_size: f32,
    pub font_family: String,
    pub font_fallbacks: Vec<String>,
    pub ui_scale: f32,
    pub notice_position: NoticePosition,
    pub notifications: NotificationsConfig,
    pub updates: UpdatesConfig,
    pub downloads: DownloadsConfig,
    /// Passed to `rex skill setup` as `--session-naming`, always explicitly.
    pub agent_session_naming: bool,
    pub chrome_theme: ChromeTheme,
    pub terminal_theme: TerminalThemeChoice,
    /// The user picked a terminal theme themselves at least once, so choosing a chrome
    /// palette leaves it alone. Until then a palette brings its paired theme (Follow chrome).
    pub terminal_theme_chosen: bool,
    /// This client has declared a terminal theme to some server at least once. Rex keeps a
    /// client's last declaration per block across launches, so Follow server then resets
    /// eligible blocks to Rex's colours. Never cleared automatically.
    pub terminal_theme_declared: bool,
    /// Prefer foreground program and directory over shell-set titles. Display only.
    pub generated_titles: bool,
    /// Tabs muted from their menu, until unmuted or closed.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub muted_tabs: Vec<MutedTab>,
    /// Shortcuts recorded in Settings → Shortcuts: a keymap layer above `shortcut_file`.
    #[serde(skip_serializing_if = "recorded::Recorded::is_empty")]
    pub recorded_shortcuts: recorded::Recorded,
    /// Read-only sibling file, never serialized into workspace.json.
    #[serde(skip)]
    pub shortcut_file: shortcuts::Shortcuts,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            hosts: vec![Host {
                label: std::fs::read_to_string("/etc/hostname")
                    .unwrap_or_else(|_| "Local".into())
                    .trim()
                    .to_owned(),
                endpoint: crate::transport::local_endpoint(),
                remote_control: false,
                collapsed: false,
                shell: None,
            }],
            client_label: None,
            last_host: None,
            sessions: HashMap::new(),
            sidebar_width: 236.,
            sidebar_open: false,
            window_size: None,
            interface: interface::InterfaceConfig::default(),
            vertical_tabs: false,
            pane_density: PaneDensity::default(),
            pane_header: PaneHeader::default(),
            pane_padding: 8.,
            font_size: 15.,
            font_family: "JetBrainsMono Nerd Font".into(),
            font_fallbacks: Vec::new(),
            ui_scale: 1.,
            notice_position: NoticePosition::default(),
            notifications: NotificationsConfig::default(),
            updates: UpdatesConfig::default(),
            downloads: DownloadsConfig::default(),
            agent_session_naming: false,
            chrome_theme: ChromeTheme::default(),
            terminal_theme: TerminalThemeChoice::default(),
            terminal_theme_chosen: false,
            terminal_theme_declared: false,
            generated_titles: false,
            muted_tabs: Vec::new(),
            recorded_shortcuts: recorded::Recorded::default(),
            shortcut_file: shortcuts::Shortcuts::default(),
        }
    }
}
impl Settings {
    pub fn client_label(&self) -> String {
        self.client_label
            .as_deref()
            .map(str::trim)
            .filter(|label| !label.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(hostname)
    }

    fn config_dir() -> PathBuf {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
            })
    }
    pub fn path() -> PathBuf {
        Self::config_dir().join("rex-linux/workspace.json")
    }
    pub fn load_default() -> Result<(Self, PathBuf)> {
        let path = Self::path();
        let settings = Self::load(&path)?;
        Ok((settings, path))
    }
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self {
                shortcut_file: shortcuts::Shortcuts::load(path),
                ..Self::default()
            });
        }
        let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)
            .with_context(|| format!("Unable to read {}", path.display()))?;
        let mut settings: Self = serde_json::from_value(value.clone())
            .with_context(|| format!("Unable to read {}", path.display()))?;
        for host in &settings.hosts {
            validate_host(&host.label, &host.endpoint)?;
        }
        ensure!(
            !settings.hosts.is_empty(),
            "workspace needs at least one host"
        );
        (settings.font_family, settings.font_fallbacks) =
            normalize_font_preferences(&settings.font_family, &settings.font_fallbacks)?;
        settings.font_size = settings.font_size.clamp(8., 48.);
        settings.pane_padding = settings.pane_padding.clamp(0., 32.);
        settings.sidebar_width = settings.sidebar_width.clamp(200., 360.);
        settings.ui_scale = normalize_ui_scale(settings.ui_scale);
        // Import before the first save loses nivox's unknown keybindings field.
        let imported = shortcuts::import_nivox(&value, &shortcuts::Shortcuts::path(path));
        settings.shortcut_file = shortcuts::Shortcuts::load(path);
        match imported {
            Ok(count) => settings.shortcut_file.imported = count,
            Err(error) => settings
                .shortcut_file
                .problem(format!("Unable to import nivox shortcuts: {error}")),
        }
        Ok(settings)
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(temporary, path)?;
        Ok(())
    }
}

pub fn hostname() -> String {
    hostname_label(&gethostname::gethostname().to_string_lossy())
}

fn hostname_label(name: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        "Rex Linux".into()
    } else {
        name.to_owned()
    }
}

pub fn parse_font_family(family: &str) -> std::result::Result<String, String> {
    let family = family.trim();
    if family.is_empty() || family.chars().any(|c| c.is_control() || c == ',') {
        return Err("Enter one font family name, without commas or control characters.".into());
    }
    Ok(family.to_owned())
}

pub fn normalize_font_preferences(
    family: &str,
    fallbacks: &[String],
) -> Result<(String, Vec<String>)> {
    let family = parse_font_family(family).map_err(anyhow::Error::msg)?;
    let mut names: Vec<String> = Vec::new();
    for fallback in fallbacks {
        let fallback = parse_font_family(fallback).map_err(anyhow::Error::msg)?;
        if !fallback.eq_ignore_ascii_case(&family)
            && !names
                .iter()
                .any(|name| name.eq_ignore_ascii_case(&fallback))
        {
            names.push(fallback);
        }
    }
    Ok((family, names))
}

pub fn normalize_ui_scale(scale: f32) -> f32 {
    if scale.is_finite() {
        (scale * 10.).round().clamp(7., 15.) / 10.
    } else {
        1.
    }
}
pub fn validate_host(label: &str, endpoint: &str) -> Result<()> {
    ensure!(!label.trim().is_empty(), "Enter a host name");
    if let Some(path) = endpoint.strip_prefix("unix://") {
        ensure!(path.starts_with('/'), "Unix socket paths must be absolute");
    } else if crate::ssh::Target::parse(endpoint)?.is_none() {
        let url =
            url::Url::parse(endpoint).context("Use an https://, ssh:// or unix:// endpoint")?;
        ensure!(
            matches!(url.scheme(), "https" | "http" | "ws" | "wss") && url.host_str().is_some(),
            "Use an https://, ssh:// or unix:// endpoint"
        );
    }
    Ok(())
}

/// A name for a host from its endpoint alone: the machine for a URL or SSH target, the socket
/// path otherwise.
pub fn endpoint_label(endpoint: &str) -> String {
    if let Ok(Some(target)) = crate::ssh::Target::parse(endpoint) {
        return target.host;
    }
    if !endpoint.starts_with("unix://")
        && let Ok(url) = url::Url::parse(endpoint)
        && let Some(host) = url.host_str()
    {
        return host.to_owned();
    }
    endpoint.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_padding_defaults_clamps_and_round_trips_zero() {
        let old: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(old.pane_padding, 8.);
        let path =
            std::env::temp_dir().join(format!("rex-padding-settings-{}.json", std::process::id()));
        for (padding, expected) in [(-1., 0.), (0., 0.), (12., 12.), (100., 32.)] {
            Settings {
                pane_padding: padding,
                ..Settings::default()
            }
            .save(&path)
            .unwrap();
            assert_eq!(Settings::load(&path).unwrap().pane_padding, expected);
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn font_preferences_survive_a_save_and_reload() {
        let json = serde_json::json!({
            "font_family": "TX02 Nerd Font Mono",
            "font_fallbacks": ["Noto Sans CJK JP", "Noto Color Emoji"]
        });
        let settings: Settings = serde_json::from_value(json.clone()).unwrap();
        let path =
            std::env::temp_dir().join(format!("rex-font-settings-{}.json", std::process::id()));
        settings.save(&path).unwrap();
        let saved = serde_json::to_value(Settings::load(&path).unwrap()).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(saved["font_family"], json["font_family"]);
        assert_eq!(saved["font_fallbacks"], json["font_fallbacks"]);
    }

    #[test]
    fn font_preferences_default_normalize_and_reject_invalid_names() {
        let old: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(old.font_family, "JetBrainsMono Nerd Font");
        assert!(old.font_fallbacks.is_empty());
        let (family, fallbacks) = normalize_font_preferences(
            "  IBM Plex Mono  ",
            &[
                "Noto Sans CJK JP".into(),
                " noto sans cjk jp ".into(),
                "IBM Plex Mono".into(),
            ],
        )
        .unwrap();
        assert_eq!(family, "IBM Plex Mono");
        assert_eq!(fallbacks, ["Noto Sans CJK JP"]);
        for invalid in ["", "   ", "bad\nname", "two,families"] {
            assert!(
                normalize_font_preferences(invalid, &[]).is_err(),
                "{invalid:?}"
            );
            assert!(normalize_font_preferences("IBM Plex Mono", &[invalid.into()]).is_err());
        }
        assert!(normalize_font_preferences("IBM Plex Mono", &[]).is_ok());
    }

    #[test]
    fn client_label_follows_hostname_unless_explicit_and_loads_existing_strings() {
        for json in [r#"{}"#, r#"{"client_label":null}"#] {
            let settings: Settings = serde_json::from_str(json).unwrap();
            assert!(settings.client_label.is_none());
            assert_eq!(settings.client_label(), hostname());
            let saved = serde_json::to_value(&settings).unwrap();
            assert!(
                saved["client_label"].is_null(),
                "saving must not freeze the hostname"
            );
        }
        let settings: Settings = serde_json::from_str(r#"{"client_label":"My laptop"}"#).unwrap();
        assert_eq!(settings.client_label.as_deref(), Some("My laptop"));
        assert_eq!(settings.client_label(), "My laptop");
        let saved = serde_json::to_value(&settings).unwrap();
        assert_eq!(saved["client_label"], "My laptop");
        // Old files with an empty label still load and get a usable default.
        let empty: Settings = serde_json::from_str(r#"{"client_label":""}"#).unwrap();
        assert_eq!(empty.client_label(), hostname());
        assert_eq!(
            hostname(),
            hostname_label(&gethostname::gethostname().to_string_lossy())
        );
        assert_eq!(hostname_label("  workstation\n"), "workstation");
        assert_eq!(hostname_label(""), "Rex Linux");
        assert_eq!(hostname_label(" \n "), "Rex Linux");
    }

    #[test]
    fn older_files_load_and_the_notice_corner_round_trips() {
        let old: Settings = serde_json::from_str(
            r#"{"hosts":[{"label":"a","endpoint":"unix:///run/rex.sock"}],"font_size":14}"#,
        )
        .unwrap();
        assert!(!old.hosts[0].remote_control);
        assert!(old.client_label.is_none());
        assert_eq!(old.client_label(), hostname());
        assert!(
            serde_json::to_value(&old).unwrap()["hosts"][0]
                .get("remote_control")
                .is_none()
        );
        assert_eq!(old.notice_position, NoticePosition::TopRight);
        assert_eq!(old.notifications, NotificationsConfig::default());
        let partial: Settings =
            serde_json::from_str(r#"{"notifications":{"done":false}}"#).unwrap();
        assert!(!partial.notifications.done && partial.notifications.blocked);
        let mut s = old;
        s.notice_position = NoticePosition::BottomRight;
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["notice_position"], "bottom_right");
        let back: Settings = serde_json::from_value(json).unwrap();
        assert_eq!(back.notice_position, NoticePosition::BottomRight);
    }

    #[test]
    fn generated_titles_default_off_for_old_files_and_round_trip() {
        let old: Settings =
            serde_json::from_str(r#"{"font_size":17,"vertical_tabs":true}"#).unwrap();
        assert!(!old.generated_titles);
        assert_eq!(old.font_size, 17.);
        assert!(old.vertical_tabs);
        for generated_titles in [true, false] {
            let settings = Settings {
                generated_titles,
                ..Settings::default()
            };
            let saved = serde_json::to_value(&settings).unwrap();
            assert_eq!(saved["generated_titles"], generated_titles);
            let back: Settings = serde_json::from_value(saved).unwrap();
            assert_eq!(back.generated_titles, generated_titles);
        }
    }

    #[test]
    fn the_terminal_theme_round_trips_and_unknown_values_follow_the_server() {
        let old: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(old.terminal_theme, TerminalThemeChoice::FollowServer);
        assert!(!old.terminal_theme_declared && !old.terminal_theme_chosen);
        for (choice, saved) in [
            (TerminalThemeChoice::FollowServer, "follow_server"),
            (TerminalThemeChoice::RexDefault, "builtin_default"),
            (TerminalThemeChoice::FollowChrome, "follow_chrome"),
            (
                TerminalThemeChoice::Named("rex/midnight".into()),
                "named:rex/midnight",
            ),
            (
                TerminalThemeChoice::Named("Catppuccin Mocha".into()),
                "named:Catppuccin Mocha",
            ),
        ] {
            let s = Settings {
                terminal_theme: choice.clone(),
                ..Settings::default()
            };
            let json = serde_json::to_value(&s).unwrap();
            assert_eq!(json["terminal_theme"], saved);
            let back: Settings = serde_json::from_value(json).unwrap();
            assert_eq!(back.terminal_theme, choice);
        }
        for unknown in ["follow_wallpaper", "named:", "Catppuccin Mocha"] {
            let s: Settings =
                serde_json::from_value(serde_json::json!({ "terminal_theme": unknown })).unwrap();
            assert_eq!(
                s.terminal_theme,
                TerminalThemeChoice::FollowServer,
                "{unknown}"
            );
        }
    }

    #[test]
    fn pane_density_defaults_to_comfortable_and_round_trips() {
        let old: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(old.pane_density, PaneDensity::Comfortable);
        for (density, saved) in [
            (PaneDensity::Compact, "compact"),
            (PaneDensity::Comfortable, "comfortable"),
        ] {
            let settings = Settings {
                pane_density: density,
                ..Settings::default()
            };
            let json = serde_json::to_value(&settings).unwrap();
            assert_eq!(json["pane_density"], saved);
            let back: Settings = serde_json::from_value(json).unwrap();
            assert_eq!(back.pane_density, density);
        }
    }

    #[test]
    fn the_downloads_directory_defaults_to_none_and_round_trips() {
        let old: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(old.downloads.directory, None);
        assert!(
            serde_json::to_value(&old).unwrap()["downloads"]
                .get("directory")
                .is_none(),
            "an unset folder is not written"
        );
        let path = std::env::temp_dir().join(format!(
            "rex-downloads-settings-{}.json",
            std::process::id()
        ));
        for directory in [Some("~/Downloads/rex"), Some("/srv/files"), None] {
            let mut settings = Settings::default();
            settings.downloads.directory = directory.map(str::to_owned);
            settings.save(&path).unwrap();
            let saved = std::fs::read_to_string(&path).unwrap();
            assert_eq!(saved.contains("\"directory\""), directory.is_some());
            assert_eq!(
                Settings::load(&path)
                    .unwrap()
                    .downloads
                    .directory
                    .as_deref(),
                directory
            );
        }
        std::fs::write(&path, r#"{"downloads":{"directory":"/a/b"}}"#).unwrap();
        assert_eq!(
            Settings::load(&path)
                .unwrap()
                .downloads
                .directory
                .as_deref(),
            Some("/a/b"),
            "the key is downloads.directory"
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn pane_header_defaults_to_auto_round_trips_and_reads_an_unknown_value_as_auto() {
        let old: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(old.pane_header, PaneHeader::Auto);
        for (header, saved) in [
            (PaneHeader::Auto, "auto"),
            (PaneHeader::Always, "always"),
            (PaneHeader::OnHover, "on_hover"),
            (PaneHeader::Hidden, "hidden"),
        ] {
            let settings = Settings {
                pane_header: header,
                ..Settings::default()
            };
            let json = serde_json::to_value(&settings).unwrap();
            assert_eq!(json["pane_header"], saved);
            let back: Settings = serde_json::from_value(json).unwrap();
            assert_eq!(back.pane_header, header);
        }
        let newer: Settings =
            serde_json::from_value(serde_json::json!({ "pane_header": "on_focus" })).unwrap();
        assert_eq!(newer.pane_header, PaneHeader::Auto);
    }

    #[test]
    fn updates_check_defaults_to_true_and_round_trips() {
        let old: Settings = serde_json::from_str("{}").unwrap();
        assert!(old.updates.check);
        for check in [false, true] {
            let settings = Settings {
                updates: UpdatesConfig { check },
                ..Settings::default()
            };
            let json = serde_json::to_value(&settings).unwrap();
            assert_eq!(json["updates"]["check"], check);
            let back: Settings = serde_json::from_value(json).unwrap();
            assert_eq!(back.updates.check, check);
        }
    }

    #[test]
    fn the_chrome_theme_round_trips_and_unknown_values_are_midnight() {
        let old: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(old.chrome_theme, ChromeTheme::Midnight);
        for (chrome, saved) in [
            (ChromeTheme::Midnight, "midnight"),
            (ChromeTheme::RosePine, "rose_pine"),
            (ChromeTheme::RosePineMoon, "rose_pine_moon"),
            (ChromeTheme::GruvboxDark, "gruvbox_dark"),
        ] {
            let s = Settings {
                chrome_theme: chrome,
                terminal_theme_chosen: true,
                ..Settings::default()
            };
            let json = serde_json::to_value(&s).unwrap();
            assert_eq!(json["chrome_theme"], saved);
            let back: Settings = serde_json::from_value(json).unwrap();
            assert_eq!(back.chrome_theme, chrome);
            assert!(back.terminal_theme_chosen);
        }
        let s: Settings =
            serde_json::from_value(serde_json::json!({ "chrome_theme": "solarized_light" }))
                .unwrap();
        assert_eq!(s.chrome_theme, ChromeTheme::Midnight);
    }

    #[test]
    fn hosts_take_https_ssh_and_unix_endpoints() {
        for ok in [
            "https://box.tail1234.ts.net",
            "http://box:8080",
            "ssh://me@box.tail1234.ts.net",
            "ssh://box:2222",
            "ssh://ops@box/run/rex/server.sock",
            "unix:///run/user/1000/rex/server.sock",
        ] {
            validate_host("box", ok).unwrap_or_else(|e| panic!("{ok}: {e}"));
        }
        for (bad, words) in [
            ("ftp://box", "https://, ssh:// or unix://"),
            ("box", "https://, ssh:// or unix://"),
            ("ssh://", "ssh://user@host"),
            ("ssh://box/", "absolute socket path"),
            ("ssh://box/a b.sock", "ssh://user@host"),
            ("unix://relative.sock", "absolute"),
        ] {
            let error = validate_host("box", bad).unwrap_err().to_string();
            assert!(error.contains(words), "{bad}: {error}");
        }
        assert!(validate_host("  ", "https://box").is_err());
        assert_eq!(
            endpoint_label("ssh://me@box.tail1234.ts.net"),
            "box.tail1234.ts.net"
        );
        assert_eq!(
            endpoint_label("https://box.tail1234.ts.net:8443"),
            "box.tail1234.ts.net"
        );
        assert_eq!(
            endpoint_label("unix:///tmp/rex.sock"),
            "unix:///tmp/rex.sock"
        );
    }
}
