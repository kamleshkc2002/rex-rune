//! About, the copied version and debug reports, and their Commands rows.
use super::*;
use crate::app::{about, panel::SettingsSection};
use rex_rune::connection::server_info::{Answer, Build, Process, ServerInfo};

fn shows(rt: &Runtime<Workspace>, words: &str) -> bool {
    rt.elements()
        .iter()
        .any(|e| !e.exiting && e.text.as_deref() == Some(words))
}

fn about(ws: Workspace) -> Runtime<Workspace> {
    let mut ws = ws;
    ws.perform(Intent::About, &mut rune::Tx::default());
    let mut rt = runtime_at(ws, (1440., 1000.));
    settle(&mut rt);
    rt
}

fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
}

fn rect(rt: &Runtime<Workspace>, key: &str) -> [f32; 4] {
    rt.elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("missing {key}"))
        .rect
}

fn build(version: &str) -> Answer<Build> {
    Answer::Reported(Build {
        rex_version: Some(version.into()),
        platform: Some("linux_amd64".into()),
        ..Build::default()
    })
}

#[test]
fn about_names_the_active_server_once_it_answers_and_says_what_it_did_not_report() {
    let mut ws = workspace_hosts(2);
    ws.hosts[0].model.connected = true;
    ws.hosts[0].model.protocol.api_revision = 13;
    ws.hosts[0].model.server_info.api_revisions = Some((10, 13));
    let mut rt = about(ws);
    assert_eq!(rt.state.settings_section, SettingsSection::About);
    assert!(shows(&rt, "Rex Rune"));
    assert!(shows(
        &rt,
        &format!("Version {}", env!("CARGO_PKG_VERSION"))
    ));
    // Connected, the version read still on its way: the revision row, no server line yet.
    assert!(has(&rt, "about-server"));
    assert!(shows(
        &rt,
        "Negotiated with the active host, which supports 10–13."
    ));
    assert!(!has(&rt, "about-server-version"));
    assert!(
        about::version_info(&rt.state).ends_with(
            "Rex Server: Not reported\nServer API revision: 13 (server supports 10–13)\n"
        )
    );

    rt.update(|s, _| s.hosts[0].model.server_info.build = build("0.1.0-preview.15"));
    settle(&mut rt);
    assert!(shows(&rt, "Rex Server 0.1.0-preview.15"));
    assert!(about::version_info(&rt.state).ends_with(
        "Rex Server: 0.1.0-preview.15\nServer API revision: 13 (server supports 10–13)\n"
    ));

    // An older server: it refused the read and its welcome carried no range.
    rt.update(|s, _| {
        s.hosts[0].model.server_info = ServerInfo {
            build: Answer::Unreported,
            process: Answer::Unreported,
            ..ServerInfo::default()
        }
    });
    settle(&mut rt);
    assert!(shows(&rt, "Rex Server version not reported"));
    assert!(shows(
        &rt,
        "Negotiated with the active host, which did not report its range."
    ));
    assert!(
        about::version_info(&rt.state)
            .ends_with("Rex Server: Not reported\nServer API revision: 13\n")
    );

    // The other host is not connected: About says nothing about a server.
    rt.update(|s, _| s.active = 1);
    settle(&mut rt);
    assert!(!has(&rt, "about-server-version") && !has(&rt, "about-server"));
    assert!(!about::version_info(&rt.state).contains("Server"));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_about_header_is_centred_over_the_cards() {
    let mut ws = workspace();
    ws.hosts[0].model.connected = true;
    ws.hosts[0].model.protocol.api_revision = 13;
    ws.hosts[0].model.server_info.build = build("0.1.0-preview.15");
    let rt = about(ws);
    let header = rect(&rt, "about-header");
    let middle = header[0] + header[2] / 2.;
    let icon = rect(&rt, "about-icon");
    assert_eq!((icon[2], icon[3]), (64., 64.));
    for key in [
        "about-icon",
        "about-version",
        "about-channel",
        "about-server-version",
    ] {
        let r = rect(&rt, key);
        assert!(
            (r[0] + r[2] / 2. - middle).abs() < 1.,
            "{key} is centred: {r:?} in {header:?}"
        );
    }
    let (copy, debug) = (rect(&rt, "about-copy"), rect(&rt, "about-copy-debug"));
    assert!(
        ((copy[0] + debug[0] + debug[2]) / 2. - middle).abs() < 1.,
        "the two buttons are centred as a pair"
    );
    assert_eq!(copy[1], debug[1]);
    let scroll = rect(&rt, "settings-scroll:about");
    assert!(header[0] >= scroll[0] && header[0] + header[2] <= scroll[0] + scroll[2]);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn debug_info_names_no_host_address_session_or_client() {
    let mut ws = workspace_hosts(3);
    for (index, (label, endpoint)) in [
        ("laptop", "unix:///run/user/1000/rex/server.sock"),
        ("buildbox", "https://buildbox.tail1234.ts.net"),
        ("vault", "ssh://admin@vault.example/run/rex.sock"),
    ]
    .into_iter()
    .enumerate()
    {
        ws.settings.hosts[index].label = label.into();
        ws.settings.hosts[index].endpoint = endpoint.into();
    }
    ws.settings.hosts[1].remote_control = true;
    let local = &mut ws.hosts[0].model;
    local.connected = true;
    local.status = "Connected".into();
    local.connection_epoch = 2;
    local.client_id = "client:abc123".into();
    local.protocol.api_revision = 13;
    local.sessions = vec![session("payroll")];
    local.server_info = ServerInfo {
        api_revisions: Some((10, 13)),
        build: build("0.1.0-preview.15"),
        process: Answer::Reported(Process {
            pid: Some(4242),
            instance_id: Some("instance:xyz".into()),
            host_id: Some("hostid:xyz".into()),
            listeners: vec![rexrs::api::server::Listener {
                endpoint: "unix:///run/user/1000/rex/server.sock".into(),
                transport: "unix".into(),
                ..Default::default()
            }],
            tailscale: Some(rexrs::api::server::Tailscale {
                state: "running".into(),
                dns_name: Some("laptop.tail1234.ts.net".into()),
                ..Default::default()
            }),
            ..Process::default()
        }),
    };
    let remote = &mut ws.hosts[1].model;
    remote.connection_epoch = 1;
    remote.status =
        "Disconnected · error connecting to https://buildbox.tail1234.ts.net: refused".into();
    remote.notice = Some("buildbox.tail1234.ts.net closed the connection".into());
    ws.hosts[2].model.ssh_server_missing = true;
    ws.hosts[2].model.status = "No Rex server on admin@vault.example".into();

    let text = about::debug_info(&ws);
    assert!(text.starts_with(&about::version_info(&ws)));
    for line in [
        "Saved hosts: 3\n",
        "Host 1 (active): Unix socket, connected, API revision 13 (server supports 10–13), Rex Server 0.1.0-preview.15 (linux_amd64), 1 session, local shortcuts, 2 connections\n",
        "Host 2: HTTPS, disconnected, 1 connection, remote control on\n",
        "Host 3: SSH, no server found over SSH, 0 connections\n",
        "Install channel: ",
        "Automatic update checks: ",
        "Display server: ",
        "Terminal font: ",
    ] {
        assert!(text.contains(line), "missing {line:?} in:\n{text}");
    }
    for secret in [
        "://",
        "laptop",
        "buildbox",
        "vault",
        "tail1234",
        "admin@",
        "server.sock",
        "rex.sock",
        "/run/",
        "payroll",
        "client:",
        "instance:",
        "hostid:",
        "4242",
        "refused",
    ] {
        assert!(!text.contains(secret), "{secret:?} leaked into:\n{text}");
    }
    // The version report is a part of it, and holds none of them either.
    assert!(!about::version_info(&ws).contains("://"));
    std::fs::remove_file(&ws.settings_path).ok();
}

#[test]
fn commands_open_about_and_copy_both_reports() {
    let mut ws = workspace();
    ws.hosts[0].model.connected = true;
    ws.hosts[0].model.protocol.api_revision = 13;
    ws.hosts[0].model.server_info.build = build("0.1.0-preview.15");
    let mut rt = runtime(ws);
    let clipboard = rune::clipboard::memory();
    rt.use_clipboard(&clipboard);
    let run = |rt: &mut Runtime<Workspace>, query: &str| {
        rt.update(|s, tx| {
            crate::app::actions::run(s, "client.palette.toggle", &serde_json::Value::Null, tx);
        });
        rt.frame(1. / 60.);
        let labels: Vec<_> = rt.state.entries().into_iter().map(|e| e.label).collect();
        for row in ["About Rex Rune", "Copy version info", "Copy debug info"] {
            assert!(labels.iter().any(|l| l == row), "{row} is a Commands row");
        }
        for c in query.chars() {
            rt.key(KeyEvent {
                key: Key::Text(c.to_string()),
                mods: Mods::default(),
            });
        }
        press(rt, Key::Enter);
    };

    run(&mut rt, "copy version");
    assert!(rt.state.panel.is_none(), "copying closes Commands");
    let copied = clipboard.text().expect("version info was copied");
    assert_eq!(copied.as_str(), about::version_info(&rt.state));
    assert!(copied.contains("Rex Server: 0.1.0-preview.15\n"));
    assert_eq!(
        rt.state.notices.last().map(|n| (n.text.as_str(), n.sticky)),
        Some(("Copied version info to clipboard.", false))
    );

    run(&mut rt, "copy debug");
    let copied = clipboard.text().expect("debug info was copied");
    assert_eq!(copied.as_str(), about::debug_info(&rt.state));
    assert!(copied.contains("Host 1 (active): Unix socket, connected"));
    assert_eq!(
        rt.state.notices.last().map(|n| n.text.as_str()),
        Some("Copied debug info to clipboard.")
    );

    run(&mut rt, "about rex");
    assert!(matches!(
        rt.state.panel.as_ref().map(|p| &p.kind),
        Some(PanelKind::Settings)
    ));
    assert_eq!(rt.state.settings_section, SettingsSection::About);
    assert!(has(&rt, "about-header"));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn every_page_prints_the_client_version_one_way() {
    let version = rex_rune::format_version(env!("CARGO_PKG_VERSION"));
    assert!(!version.starts_with('v'));
    let mut ws = workspace();
    ws.perform(Intent::Updates, &mut rune::Tx::default());
    let mut rt = runtime_at(ws, (1440., 1000.));
    settle(&mut rt);
    let installed = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("updates-current-version"))
        .expect("the installed version");
    assert_eq!(installed.text.as_deref(), Some(version.as_str()));
    rt.update(|s, tx| s.perform(Intent::About, tx));
    settle(&mut rt);
    assert!(shows(&rt, &format!("Version {version}")));
    assert!(about::version_info(&rt.state).starts_with(&format!("Rex Rune {version}\n")));
    std::fs::remove_file(&rt.state.settings_path).ok();
}
