//! Who this client and its server are: the About page, and the version and debug text for a
//! bug report. The server's part is the active host's `server_info`, which its connection
//! reads once; this page sends no request.
use super::{
    settings_kit::{self, button, card, mono_value, page, setting, value},
    theme::*,
    widgets::{Pill, app_logo},
    workspace::Workspace,
};
use rex_linux::{connection::HostModel, format_version, settings::Host};
use rune::{Align, El, col, row, text};

pub const REPOSITORY: &str = "https://github.com/shuv1337/rex-rune";
/// Opens Settings → About.
pub const ABOUT: &str = "client.about";
/// Copies the version info and each host's kind and state, without names or addresses.
pub const COPY_DEBUG: &str = "client.debug.copy";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const COMMIT: &str = env!("REX_BUILD_COMMIT");
const BUILD_DATE: &str = env!("REX_BUILD_DATE");
/// What stands in for a fact the server didn't give.
const NOT_REPORTED: &str = "Not reported";

pub fn release_url() -> String {
    format!("{REPOSITORY}/releases/tag/v{VERSION}")
}

/// A connected host's server, as About and the copied reports name it.
struct Server<'a> {
    /// None when the server didn't say, or hasn't yet.
    version: Option<&'a str>,
    /// The version read has an answer, a refusal or a timeout: nothing more will come.
    settled: bool,
    api: u32,
    /// The oldest and newest revisions the server speaks.
    range: Option<(u32, u32)>,
}

impl<'a> Server<'a> {
    fn of(host: &'a HostModel) -> Option<Self> {
        (host.connected && host.protocol.api_revision != 0).then(|| Self {
            version: host.server_info.rex_version(),
            settled: host.server_info.build.settled(),
            api: host.protocol.api_revision,
            range: host.server_info.api_revisions,
        })
    }

    fn version(&self) -> &str {
        self.version.unwrap_or(NOT_REPORTED)
    }

    /// `13 (server supports 10–13)`, or the revision alone when the range is unknown.
    fn api(&self) -> String {
        match self.range {
            Some((oldest, newest)) => {
                format!("{} (server supports {oldest}–{newest})", self.api)
            }
            None => self.api.to_string(),
        }
    }
}

fn format_info(
    version: &str,
    commit: &str,
    date: &str,
    channel: &str,
    os: &str,
    server: Option<&Server>,
) -> String {
    let mut info = format!(
        "Rex Linux {}\nCommit: {commit}\nBuild date: {date}\nBuild channel: {channel}\nOS: {os}\n",
        format_version(version)
    );
    if let Some(server) = server {
        info.push_str(&format!(
            "Rex Server: {}\nServer API revision: {}\n",
            server.version(),
            server.api()
        ));
    }
    info
}

/// The client's and the active server's versions, for a bug report.
pub fn version_info(s: &Workspace) -> String {
    format_info(
        VERSION,
        COMMIT,
        BUILD_DATE,
        rex_linux::BUILD_CHANNEL,
        &format!("{} ({})", std::env::consts::OS, std::env::consts::ARCH),
        Server::of(s.model()).as_ref(),
    )
}

/// How a saved host is reached, without its address.
fn kind(endpoint: &str) -> &'static str {
    match endpoint.split_once("://").map(|(scheme, _)| scheme) {
        Some("unix") => "Unix socket",
        Some("ssh") => "SSH",
        Some("https" | "wss") => "HTTPS",
        Some("http" | "ws") => "HTTP",
        _ => "other endpoint",
    }
}

/// A host's connection state in a fixed word. Its status text is never copied: a failure
/// there can quote the address it failed to reach.
fn state(host: &HostModel) -> &'static str {
    if host.connected {
        "connected"
    } else if host.ssh_server_missing {
        "no server found over SSH"
    } else if host.status.starts_with("Connecting") {
        "connecting"
    } else if host.status.starts_with("Reconnecting") {
        "reconnecting"
    } else {
        "disconnected"
    }
}

/// One host of the debug report: its number in place of its name, and no address.
fn host_line(index: usize, active: bool, saved: &Host, host: &HostModel) -> String {
    let mut facts = vec![kind(&saved.endpoint).to_owned(), state(host).to_owned()];
    if let Some(server) = Server::of(host) {
        facts.push(format!("API revision {}", server.api()));
        let mut build = Vec::new();
        if let Some(reported) = host.server_info.build.reported() {
            build.extend(reported.platform.clone());
            build.extend(reported.commit.as_ref().map(|c| format!("commit {c}")));
            if reported.dirty == Some(true) {
                build.push("modified".into());
            }
        }
        let mut version = match server.version {
            Some(version) => format!("Rex Server {version}"),
            None => "Rex Server version not reported".to_owned(),
        };
        if !build.is_empty() {
            version.push_str(&format!(" ({})", build.join(", ")));
        }
        facts.push(version);
        facts.push(match host.sessions.len() {
            1 => "1 session".into(),
            count => format!("{count} sessions"),
        });
        facts.push(
            if host.keymap.is_some() {
                "server keymap"
            } else {
                "local shortcuts"
            }
            .into(),
        );
    }
    facts.push(match host.connection_epoch {
        1 => "1 connection".into(),
        count => format!("{count} connections"),
    });
    if saved.remote_control {
        facts.push("remote control on".into());
    }
    format!(
        "Host {}{}: {}\n",
        index + 1,
        if active { " (active)" } else { "" },
        facts.join(", ")
    )
}

/// The display server this session says it runs on.
fn display_server(session_type: Option<&str>, wayland: bool, x11: bool) -> &'static str {
    match session_type {
        Some("wayland") => "Wayland",
        Some("x11") => "X11",
        _ if wayland => "Wayland",
        _ if x11 => "X11",
        _ => "unknown",
    }
}

/// [`version_info`], then what a report about this client's behaviour needs: how it was
/// installed, the desktop it runs on, its fonts and each saved host's kind and state. No host
/// name, address, session name, client id or credential is part of it.
pub fn debug_info(s: &Workspace) -> String {
    let env = |name: &str| std::env::var(name).ok().filter(|v| !v.trim().is_empty());
    let mut info = version_info(s);
    info.push_str(&format!(
        "\nInstall channel: {}\nAutomatic update checks: {}\n",
        s.updates.channel.label(),
        if s.updates.auto_check_enabled(s.settings.updates.check) {
            "on"
        } else {
            "off"
        }
    ));
    info.push_str(&format!(
        "Display server: {}\n",
        display_server(
            env("XDG_SESSION_TYPE").as_deref(),
            env("WAYLAND_DISPLAY").is_some(),
            env("DISPLAY").is_some(),
        )
    ));
    if let Some(desktop) = env("XDG_CURRENT_DESKTOP") {
        info.push_str(&format!("Desktop: {desktop}\n"));
    }
    info.push_str(&format!(
        "Interface: {}, {}% zoom\n",
        s.settings.chrome_theme.name(),
        (s.settings.ui_scale * 100.).round()
    ));
    let (family, fallbacks) = s.options.fonts(&s.settings);
    info.push_str(&format!(
        "Terminal font: {family} {} px, {}\n",
        s.settings.font_size,
        match fallbacks.len() {
            0 => "no fallbacks".to_owned(),
            1 => "1 fallback".to_owned(),
            count => format!("{count} fallbacks"),
        }
    ));
    info.push_str(&format!("Saved hosts: {}\n", s.hosts.len()));
    for (index, host) in s.hosts.iter().enumerate() {
        info.push_str(&host_line(
            index,
            index == s.active,
            &s.settings.hosts[index],
            &host.model,
        ));
    }
    info
}

impl Workspace {
    pub fn copy_version_info(&mut self, tx: &mut rune::Tx) {
        tx.copy(version_info(self));
        self.inform("Copied version info to clipboard.");
    }

    pub fn copy_debug_info(&mut self, tx: &mut rune::Tx) {
        tx.copy(debug_info(self));
        self.inform("Copied debug info to clipboard.");
    }
}

/// The identity block: icon, name, version, how it was installed and the active server's
/// version, over the two copy actions.
fn header(s: &Workspace) -> El {
    let line = |words: String| {
        text(words)
            .font(ui(12.))
            .color(rgb(muted()))
            .align(Align::Center)
            .selectable(true)
    };
    let mut lines = col()
        .items_center()
        .gap(3.)
        .child(
            text(format!("Version {}", format_version(VERSION)))
                .key("about-version")
                .font(ui(12.5))
                .color(rgb(ink()))
                .selectable(true),
        )
        .child(line(s.updates.channel.label()).key("about-channel"));
    // The server line appears once the server has answered, so it never reads as missing
    // while its answer is on the way.
    if let Some(server) = Server::of(s.model()).filter(|server| server.settled) {
        lines = lines.child(
            line(match server.version {
                Some(version) => format!("Rex Server {version}"),
                None => "Rex Server version not reported".to_owned(),
            })
            .key("about-server-version"),
        );
    }
    col()
        .key("about-header")
        .w_full()
        .items_center()
        .pt(4.)
        .gap(12.)
        .child(app_logo(s, 64.).key("about-icon"))
        .child(
            col()
                .items_center()
                .gap(5.)
                .child(
                    text("Rex Linux")
                        .role("heading")
                        .font(medium(15.))
                        .color(rgb(ink())),
                )
                .child(lines),
        )
        .child(
            row()
                .gap(8.)
                .center_y()
                .child(button(
                    "about-copy",
                    "Copy version info",
                    Pill::Neutral,
                    |s, tx| s.copy_version_info(tx),
                ))
                .child(button(
                    "about-copy-debug",
                    "Copy debug info",
                    Pill::Neutral,
                    |s, tx| s.copy_debug_info(tx),
                )),
        )
        .child(
            text("For bug reports. Neither copy has host names, addresses or credentials.")
                .font(ui(11.5))
                .color(rgb(muted()))
                .align(Align::Center)
                .wrap(),
        )
}

/// The commit this binary was built from and the day it was built, on one row. A source
/// archive without Git knows neither and says so once.
fn build_row(commit: &str, date: &str) -> El {
    let facts = if commit == date {
        row().child(value(commit))
    } else {
        row()
            .gap(7.)
            .center_y()
            .child(mono_value(commit))
            .child(value("·"))
            .child(value(date))
    };
    setting("Build", "", facts.key("about-build"))
}

pub fn section(s: &Workspace) -> El {
    let mut build = vec![build_row(COMMIT, BUILD_DATE)];
    if let Some(server) = Server::of(s.model()) {
        build.push(
            setting(
                "Server API revision",
                &match server.range {
                    Some((oldest, newest)) => {
                        format!(
                            "Negotiated with the active host, which supports {oldest}–{newest}."
                        )
                    }
                    None => "Negotiated with the active host, which did not report its range."
                        .to_owned(),
                },
                value(server.api.to_string()),
            )
            .key("about-server"),
        );
    }
    // Finish the metadata section before building Project rows so Settings search
    // associates each row with the section it is actually drawn under.
    let build = settings_kit::section("", [card(build)]);
    let links = [
        (
            "about-repository",
            "Repository",
            "github.com/shuv1337/rex-rune",
            "Open repository",
            REPOSITORY.to_owned(),
        ),
        (
            "about-releases",
            "Release notes",
            "Changes in this version",
            "Open release notes",
            release_url(),
        ),
        (
            "about-license",
            "License",
            "MIT",
            "Open license",
            format!("{REPOSITORY}/blob/v{VERSION}/LICENSE"),
        ),
        (
            "about-issues",
            "Issues",
            "Report a problem or ask for a feature",
            "Report an issue",
            format!("{REPOSITORY}/issues"),
        ),
    ]
    .map(|(key, label, detail, action, url)| {
        setting(
            label,
            detail,
            button(key, action, Pill::Neutral, move |s, _| {
                s.open_link(url.clone())
            }),
        )
    });
    page([
        header(s),
        build,
        settings_kit::section("Project", [card(links)]),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use rex_linux::connection::server_info::{Answer, Build, ServerInfo};

    fn server<'a>(version: Option<&'a str>, range: Option<(u32, u32)>) -> Server<'a> {
        Server {
            version,
            settled: true,
            api: 12,
            range,
        }
    }

    #[test]
    fn report_includes_available_metadata_and_server() {
        assert_eq!(
            format_info(
                "0.5.0",
                "abc1234",
                "2026-11-10 PST",
                "official",
                "linux (x86_64)",
                Some(&server(Some("0.1.0-preview.15"), Some((10, 13))))
            ),
            "Rex Linux 0.5.0\nCommit: abc1234\nBuild date: 2026-11-10 PST\nBuild channel: official\nOS: linux (x86_64)\nRex Server: 0.1.0-preview.15\nServer API revision: 12 (server supports 10–13)\n"
        );
    }

    #[test]
    fn an_older_server_is_named_without_what_it_did_not_report() {
        assert_eq!(
            format_info(
                "0.5.0",
                "a",
                "b",
                "source",
                "linux",
                Some(&server(None, None))
            ),
            "Rex Linux 0.5.0\nCommit: a\nBuild date: b\nBuild channel: source\nOS: linux\nRex Server: Not reported\nServer API revision: 12\n"
        );
    }

    #[test]
    fn source_archive_and_offline_report_are_honest() {
        assert_eq!(
            format_info("0.5.0", "unknown", "unknown", "source", "linux", None),
            "Rex Linux 0.5.0\nCommit: unknown\nBuild date: unknown\nBuild channel: source\nOS: linux\n"
        );
        // Plain cargo builds default to source; release CI supplies its own marker.
        assert_eq!(
            rex_linux::BUILD_CHANNEL,
            option_env!("REX_LINUX_CHANNEL").unwrap_or("source")
        );
        assert_eq!(
            release_url(),
            format!("{REPOSITORY}/releases/tag/v{}", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn one_version_format_without_the_tag_prefix() {
        assert_eq!(format_version("0.6.0"), "0.6.0");
        assert_eq!(format_version("v0.6.0"), "0.6.0");
        assert_eq!(format_version(" V1.2.3-rc.1 "), "1.2.3-rc.1");
        assert!(format_info("v0.6.0", "a", "b", "c", "d", None).starts_with("Rex Linux 0.6.0\n"));
    }

    #[test]
    fn a_host_line_has_its_kind_and_state_and_never_its_name_or_address() {
        let saved = |endpoint: &str| Host {
            label: "buildbox".into(),
            endpoint: endpoint.into(),
            remote_control: false,
            collapsed: false,
            shell: None,
        };
        let mut host = HostModel {
            connected: true,
            status: "Connected".into(),
            client_id: "client:secret".into(),
            connection_epoch: 1,
            ..HostModel::default()
        };
        host.protocol.api_revision = 13;
        host.server_info = ServerInfo {
            api_revisions: Some((10, 13)),
            build: Answer::Reported(Build {
                rex_version: Some("0.1.0-preview.15".into()),
                platform: Some("linux_amd64".into()),
                commit: Some("9a754d".into()),
                dirty: Some(true),
                ..Build::default()
            }),
            ..ServerInfo::default()
        };
        assert_eq!(
            host_line(0, true, &saved("https://buildbox.tail1234.ts.net"), &host),
            "Host 1 (active): HTTPS, connected, API revision 13 (server supports 10–13), Rex Server 0.1.0-preview.15 (linux_amd64, commit 9a754d, modified), 0 sessions, local shortcuts, 1 connection\n"
        );
        // A server that answered neither read, then a failure whose text quotes the address.
        host.server_info = ServerInfo {
            build: Answer::Unreported,
            process: Answer::Unreported,
            ..ServerInfo::default()
        };
        assert_eq!(
            host_line(1, false, &saved("unix:///run/user/1000/rex.sock"), &host),
            "Host 2: Unix socket, connected, API revision 13, Rex Server version not reported, 0 sessions, local shortcuts, 1 connection\n"
        );
        host.connected = false;
        host.connection_epoch = 3;
        host.status = "Disconnected · could not reach ssh://admin@buildbox.example".into();
        let mut remote = saved("ssh://admin@buildbox.example");
        remote.remote_control = true;
        assert_eq!(
            host_line(2, false, &remote, &host),
            "Host 3: SSH, disconnected, 3 connections, remote control on\n"
        );
        for (status, word) in [
            ("Connecting…", "connecting"),
            ("Connecting over SSH…", "connecting"),
            ("Reconnecting…", "reconnecting"),
            ("tls handshake with 100.64.0.1 failed", "disconnected"),
        ] {
            host.status = status.into();
            assert_eq!(state(&host), word);
        }
        host.ssh_server_missing = true;
        assert_eq!(state(&host), "no server found over SSH");
        assert_eq!(kind("buildbox:7297"), "other endpoint");
        assert_eq!(kind("http://127.0.0.1:7297"), "HTTP");
    }

    #[test]
    fn the_display_server_follows_the_session_type_then_the_sockets() {
        assert_eq!(display_server(Some("wayland"), false, true), "Wayland");
        assert_eq!(display_server(Some("x11"), true, true), "X11");
        assert_eq!(display_server(Some("tty"), true, true), "Wayland");
        assert_eq!(display_server(None, false, true), "X11");
        assert_eq!(display_server(None, false, false), "unknown");
    }
}
