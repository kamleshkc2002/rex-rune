//! Cached Server facts, privacy, navigation and read-only behavior.
use super::pane_verb_tests::{Camera, framed_terminal, rect};
use super::*;
use crate::app::{panel::SettingsSection, server_page};
use rex_linux::connection::server_info::{Answer, Build, Process, ServerInfo};

fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
}

fn reported(version: &str) -> ServerInfo {
    ServerInfo {
        api_revisions: Some((10, 13)),
        build: Answer::Reported(Build {
            rex_version: Some(version.into()),
            platform: Some("linux_amd64".into()),
            commit: Some("abc1234".into()),
            dirty: Some(false),
            built_at: Some("2026-11-18T10:00:00-08:00".into()),
        }),
        process: Answer::Reported(Process {
            pid: Some(4242),
            instance_id: Some("private-instance".into()),
            host_id: Some("private-host-id".into()),
            started_at: Some("2026-11-18T11:00:00-08:00".into()),
            session_count: Some(7),
            listeners: vec![rexrs::api::server::Listener {
                endpoint: "unix:///run/rex/server.sock".into(),
                addresses: vec!["private-bound-address".into()],
                ..Default::default()
            }],
            tailscale: Some(rexrs::api::server::Tailscale {
                state: "running".into(),
                auth_url: Some("https://login.example/secret-auth".into()),
                error: Some("secret-error".into()),
                https: Some(rexrs::api::server::Http {
                    state: "available".into(),
                    url: Some("https://user:secret-password@build.tail1234.ts.net?token=secret-query#secret-fragment".into()),
                }),
                ..Default::default()
            }),
        }),
    }
}

fn fixture() -> Workspace {
    let mut ws = workspace();
    ws.settings.hosts[0].label = "Build machine".into();
    ws.settings.hosts[0].endpoint = "ssh://user@build.example/run/rex.sock".into();
    ws.hosts[0].model.connected = true;
    ws.hosts[0].model.protocol.api_revision = 13;
    ws.hosts[0].model.server_info = reported("0.1.0-preview.15");
    ws
}

// The workspace always retains a default host and cannot remove its last one. Exercise
// the page's defensive empty state in the real Settings shell without unrelated chrome
// (whose existing invariant requires a host).
fn page_view(s: &Workspace, cx: &mut rune::Cx<Workspace>) -> rune::El {
    if s.hosts.is_empty() {
        rune::el()
            .w_full()
            .h_full()
            .fill(crate::app::theme::rgb(crate::app::theme::ground()))
            .child(crate::app::settings_view::settings(s, s.viewport))
    } else {
        view(s, cx)
    }
}

fn prepare(ws: &mut Workspace, size: (f32, f32)) {
    ws.panel = Some(crate::app::panel::Panel::new(
        PanelKind::Settings,
        "Settings",
        &[],
    ));
    ws.viewport = [size.0, size.1];
    ws.open_section(SettingsSection::Server);
}

fn open(mut ws: Workspace, size: (f32, f32)) -> Runtime<Workspace> {
    prepare(&mut ws, size);
    let mut rt = Runtime::new(ws, page_view, size, Env::new(true, 1.));
    rt.on_key = Some(crate::app::input::on_key);
    settle(&mut rt);
    rt
}

fn page_text(rt: &Runtime<Workspace>) -> String {
    rt.elements()
        .iter()
        .filter(|e| !e.exiting && e.path.contains("settings-scroll:server"))
        .filter_map(|e| e.text.as_deref())
        .collect::<Vec<_>>()
        .join("\n")
}

fn shows(rt: &Runtime<Workspace>, value: &str) -> bool {
    rt.elements().iter().any(|e| {
        !e.exiting && e.path.contains("settings-scroll:server") && e.text.as_deref() == Some(value)
    })
}

fn tap(rt: &mut Runtime<Workspace>, key: &str) {
    rt.update(|_, tx| tx.scroll_center(key));
    settle(rt);
    click(rt, key);
    settle(rt);
}

#[test]
fn reported_facts_are_cached_not_live_and_sensitive_members_are_not_rendered() {
    let rt = open(fixture(), (1024., 700.));
    for value in [
        "Build machine",
        "connected",
        "ssh://build.example/run/rex.sock",
        "0.1.0-preview.15",
        "13",
        "10–13",
        "linux_amd64",
        "abc1234",
        "Clean",
        "2026-11-18T10:00:00-08:00",
        "4242",
        "2026-11-18T11:00:00-08:00",
        "7",
        "1 reported",
        "unix:///run/rex/server.sock",
        "Running",
        "Available",
        "https://build.tail1234.ts.net/",
    ] {
        assert!(shows(&rt, value), "missing {value}: {}", page_text(&rt));
    }
    let text = page_text(&rt);
    assert!(text.contains("Last reported server facts, not live monitoring"));
    assert!(text.contains("not this client's local Tailscale"));
    for secret in ["private-", "secret-", "user@", "token="] {
        assert!(!text.contains(secret), "{secret}: {text}");
        assert!(
            !rt.elements()
                .iter()
                .any(|e| e.path.contains("settings-scroll:server")
                    && e.help.as_deref().is_some_and(|help| help.contains(secret)))
        );
    }
}

#[test]
fn pending_partial_refused_and_unknown_fields_remain_distinct() {
    let mut ws = fixture();
    ws.hosts[0].model.server_info = ServerInfo::default();
    ws.hosts[0].model.protocol.api_revision = 0;
    let mut rt = open(ws, (1024., 700.));
    assert!(shows(&rt, "Waiting for report"));
    for label in ["Built-in Tailscale", "Tailscale HTTPS", "Tailscale address"] {
        assert!(page_text(&rt).contains(&format!("{label}\nWaiting for report")));
    }
    assert!(shows(&rt, "Not reported") && !shows(&rt, "Not connected"));
    assert!(!has(&rt, "server-copy-tailnet"));
    rt.update(|s, _| {
        s.hosts[0].model.server_info = ServerInfo {
            build: Answer::Unreported,
            process: Answer::Unreported,
            ..Default::default()
        };
    });
    settle(&mut rt);
    assert!(!shows(&rt, "Waiting for report"));
    assert!(shows(&rt, "Not reported"));
    assert!(!shows(&rt, "0") && !shows(&rt, "Disabled"));

    rt.update(|s, _| {
        s.hosts[0].model.protocol.api_revision = 10;
        s.hosts[0].model.server_info = ServerInfo {
            build: Answer::Reported(Build {
                rex_version: Some("old-server".into()),
                ..Default::default()
            }),
            process: Answer::Reported(Process::default()),
            ..Default::default()
        };
    });
    settle(&mut rt);
    assert!(shows(&rt, "10") && shows(&rt, "old-server"));
    assert!(!shows(&rt, "Waiting for report") && !shows(&rt, "0 reported"));

    rt.update(|s, _| {
        if let Answer::Reported(p) = &mut s.hosts[0].model.server_info.process {
            p.tailscale = Some(rexrs::api::server::Tailscale {
                state: "future-state-secret".into(),
                https: Some(rexrs::api::server::Http {
                    state: "future-state".into(),
                    url: Some("https://not-ready.example".into()),
                }),
                ..Default::default()
            });
        }
    });
    settle(&mut rt);
    assert!(shows(&rt, "Unknown (starting)") && shows(&rt, "Pending"));
    assert!(!has(&rt, "server-copy-tailnet"));
    assert!(!page_text(&rt).contains("future-state"));
    for (state, label) in [
        ("disabled", "Disabled"),
        ("starting", "Starting"),
        ("needs_login", "Needs sign-in"),
        ("needs_machine_approval", "Needs machine approval"),
    ] {
        rt.update(|s, _| {
            if let Answer::Reported(p) = &mut s.hosts[0].model.server_info.process {
                p.tailscale.as_mut().unwrap().state = state.into();
            }
        });
        settle(&mut rt);
        assert!(shows(&rt, label));
        assert!(!has(&rt, "server-copy-tailnet"));
    }
    rt.update(|s, _| {
        s.hosts[0].model.connected = false;
        s.hosts[0].model.server_info = ServerInfo::default();
    });
    settle(&mut rt);
    assert!(!shows(&rt, "Waiting for report"));
    assert!(shows(&rt, "Not reported"));
}

#[test]
fn disconnected_reconnecting_and_new_connection_never_mix_reports() {
    let mut ws = fixture();
    ws.hosts[0].model.connected = false;
    ws.hosts[0].model.status = "Disconnected: password=secret-status".into();
    let mut rt = open(ws, (1024., 700.));
    assert!(shows(&rt, "disconnected"));
    assert!(shows(&rt, "Running (last reported)") && shows(&rt, "Available (last reported)"));
    assert!(!shows(&rt, "Running") && !shows(&rt, "Available"));
    assert!(shows(&rt, "Not connected") && shows(&rt, "0.1.0-preview.15"));
    assert!(page_text(&rt).contains("Disconnected: last reported"));
    assert!(!page_text(&rt).contains("secret-status"));
    let before = page_text(&rt);
    settle(&mut rt);
    assert_eq!(page_text(&rt), before, "no invented ticking uptime");

    rt.update(|s, _| s.hosts[0].model.status = "Reconnecting…".into());
    settle(&mut rt);
    assert!(shows(&rt, "reconnecting"));
    rt.update(|s, _| {
        // The existing welcome path replaces the cache on a new connection.
        let h = &mut s.hosts[0].model;
        h.connected = true;
        h.connection_epoch += 1;
        h.server_info = ServerInfo::default();
    });
    settle(&mut rt);
    assert!(shows(&rt, "Waiting for report"));
    assert!(!shows(&rt, "0.1.0-preview.15") && !shows(&rt, "4242"));
    rt.update(|s, _| s.hosts[0].model.server_info = reported("new-server"));
    settle(&mut rt);
    assert!(shows(&rt, "new-server"));
}

#[test]
fn active_hosts_with_same_server_ids_never_share_facts_or_copy_payloads() {
    let mut ws = workspace_hosts(2);
    for (i, name) in ["first", "second"].into_iter().enumerate() {
        ws.settings.hosts[i].label = name.into();
        ws.settings.hosts[i].endpoint =
            format!("https://user:password@{name}.example?token=secret");
        ws.hosts[i].model.connected = true;
        ws.hosts[i].model.protocol.api_revision = 13;
        ws.hosts[i].model.server_info = reported(name);
    }
    let mut rt = open(ws, (1024., 700.));
    let clipboard = rune::clipboard::memory();
    rt.use_clipboard(&clipboard);
    assert!(clipboard.text().is_none(), "opening does not copy");
    for (index, here, other) in [
        (0, "first", "second"),
        (1, "second", "first"),
        (0, "first", "second"),
    ] {
        rt.update(|s, _| s.active = index);
        settle(&mut rt);
        assert!(shows(&rt, here) && !shows(&rt, other));
        tap(&mut rt, "server-copy-endpoint");
        assert_eq!(
            clipboard.text().as_ref().map(|v| v.as_str()),
            Some(format!("https://{here}.example/").as_str())
        );
    }
    tap(&mut rt, "server-copy-tailnet");
    assert_eq!(
        clipboard.text().as_ref().map(|v| v.as_str()),
        Some("https://build.tail1234.ts.net/")
    );
}

#[test]
fn address_forms_strip_credentials_and_never_echo_unknown_syntax() {
    for (input, expected) in [
        (
            "https://u:p@example.org:443/path?token=secret#secret",
            Some("https://example.org/path"),
        ),
        (
            "wss://u:p@example.org:9000/ws?key=x",
            Some("wss://example.org:9000/ws"),
        ),
        (
            "ssh://u:p@box.example/run/rex.sock?password=x",
            Some("ssh://box.example/run/rex.sock"),
        ),
        (
            "unix:///run/rex.sock?token=x#x",
            Some("unix:///run/rex.sock?token=x#x"),
        ),
        (
            "unix:///tmp/rex?a/server.sock",
            Some("unix:///tmp/rex?a/server.sock"),
        ),
        (
            "unix:///tmp/rex#1/server.sock",
            Some("unix:///tmp/rex#1/server.sock"),
        ),
        (
            "unix:///home/me/My Sockets/réx.sock",
            Some("unix:///home/me/My Sockets/réx.sock"),
        ),
        ("unix://relative.sock", None),
        ("127.0.0.1:7297", Some("127.0.0.1:7297")),
        ("[::1]:7297", Some("[::1]:7297")),
        ("javascript:password", None),
        ("data:text/plain,secret", None),
        ("future://u:p@example?token=x", None),
        ("not an address: secret", None),
        ("https://example.org/\npassword", None),
    ] {
        assert_eq!(server_page::address(input).as_deref(), expected, "{input}");
    }
    let mut ws = fixture();
    ws.settings.hosts[0].endpoint = "future://secret-configured".into();
    if let Answer::Reported(p) = &mut ws.hosts[0].model.server_info.process {
        p.listeners[0].endpoint = "https://u:secret-listener@example.org?secret-query".into();
        for endpoint in ["tailnet://:7297", "localhost:7297"] {
            p.listeners.push(rexrs::api::server::Listener {
                endpoint: endpoint.into(),
                ..Default::default()
            });
        }
        p.tailscale.as_mut().unwrap().https.as_mut().unwrap().url =
            Some("javascript:secret-link".into());
    }
    let rt = open(ws, (1024., 700.));
    assert!(!has(&rt, "server-copy-endpoint") && !has(&rt, "server-copy-tailnet"));
    assert!(shows(&rt, "https://example.org/") && shows(&rt, "Address unavailable"));
    assert_eq!(
        page_text(&rt).matches("Endpoint format not shown").count(),
        2
    );
    assert!(!page_text(&rt).contains("secret"));

    // Copy a Unix endpoint exactly as transport interprets it, not as a URL.
    let literal = "unix:///home/me/My Sockets/réx?1#2.sock";
    let mut ws = fixture();
    ws.settings.hosts[0].endpoint = literal.into();
    let mut rt = open(ws, (640., 480.));
    let clipboard = rune::clipboard::memory();
    rt.use_clipboard(&clipboard);
    assert!(shows(&rt, literal));
    tap(&mut rt, "server-copy-endpoint");
    assert_eq!(clipboard.text().as_ref().map(|v| v.as_str()), Some(literal));
}

#[test]
fn no_hosts_has_no_fabricated_facts_or_copy_controls() {
    let mut ws = fixture();
    ws.hosts.clear();
    ws.settings.hosts.clear();
    let rt = open(ws, (640., 480.));
    assert!(has(&rt, "server-empty"));
    assert!(!has(&rt, "server-copy-endpoint") && !has(&rt, "server-copy-tailnet"));
    assert!(!shows(&rt, "connected") && !shows(&rt, "0"));
}

#[test]
fn settings_search_history_keyboard_and_copy_are_local_even_over_a_live_pane() {
    let _families = rune::text::with_families("Inter", "Inter");
    for size in [(1024., 700.), (640., 480.)] {
        let (mut ws, host) = tabbed_workspace("single");
        ws.hosts[0].model.server_info = reported("read-only");
        let (sender, terminal) = flume::unbounded();
        framed_terminal(&mut ws, "block:a", &sender, b"unchanged terminal");
        let mut rt = runtime_at(ws, size);
        settle(&mut rt);
        host.drain().for_each(drop);
        terminal.drain().for_each(drop);
        let claimed = rt.state.claimed.clone();
        let frame = rt.state.terminals[&rt.state.term_key("block:a")]
            .frame
            .clone();
        let clipboard = rune::clipboard::memory();
        rt.use_clipboard(&clipboard);
        rt.update(|s, tx| s.perform(Intent::Settings, tx));
        settle(&mut rt);
        click(&mut rt, "settings-section:server");
        settle(&mut rt);
        assert_eq!(rt.state.settings_page(), SettingsSection::Server);
        assert!(focused(&rt).is_some_and(|f| f.contains("settings-section:server")));
        for label in [
            "Configured address",
            "Server version",
            "Started at",
            "Session count",
            "Listeners",
            "Built-in Tailscale",
            "Tailscale address",
        ] {
            click(&mut rt, "settings-search");
            rt.update(|s, _| {
                s.settings_nav.search = rune::TextEdit::new(label);
                s.settings_search_changed();
            });
            let hits = rt.state.settings_nav.hits();
            assert_eq!(hits[0].label, label);
            assert_eq!(hits[0].page, SettingsSection::Server);
            press(&mut rt, Key::Enter);
            settle(&mut rt);
            let found = rect(&rt, "settings-found");
            let scroll = rect(&rt, "settings-scroll:server");
            assert!(
                found[1] >= scroll[1] - 1. && found[1] + found[3] <= scroll[1] + scroll[3] + 1.
            );
        }
        press(&mut rt, Key::Escape); // Clear the search, returning the section list.
        let offset = rt.state.settings_nav.offset().get();
        assert!(offset > 0.);
        click(&mut rt, "settings-section:terminal");
        rt.key(KeyEvent {
            key: Key::Left,
            mods: Mods {
                alt: true,
                ..Default::default()
            },
        });
        settle(&mut rt);
        assert_eq!(rt.state.settings_page(), SettingsSection::Server);
        assert!((rt.state.settings_nav.offset().get() - offset).abs() < 1.);
        rt.key(KeyEvent {
            key: Key::Right,
            mods: Mods {
                alt: true,
                ..Default::default()
            },
        });
        settle(&mut rt);
        assert_eq!(rt.state.settings_page(), SettingsSection::Terminal);
        click(&mut rt, "settings-back");
        settle(&mut rt);
        rt.update(|_, tx| {
            tx.scroll_center("server-copy-tailnet");
            tx.focus("server-copy-tailnet", true);
        });
        settle(&mut rt);
        press(&mut rt, Key::Enter);
        assert_eq!(
            clipboard.text().as_ref().map(|v| v.as_str()),
            Some("https://build.tail1234.ts.net/")
        );
        press(&mut rt, Key::Tab);
        press(&mut rt, Key::Escape);
        settle(&mut rt);
        assert!(rt.state.panel.is_none());
        assert!(
            host.is_empty() && terminal.is_empty(),
            "no requests, mutations, PTY input or resizes"
        );
        assert_eq!(rt.state.claimed, claimed);
        assert!(std::sync::Arc::ptr_eq(
            frame.as_ref().unwrap(),
            rt.state.terminals[&rt.state.term_key("block:a")]
                .frame
                .as_ref()
                .unwrap()
        ));
    }
}

#[test]
fn long_values_fit_both_page_widths_and_keep_safe_tooltips() {
    let _families = rune::text::with_families("Inter", "Inter");
    for size in [(1024., 700.), (640., 480.)] {
        let mut ws = fixture();
        ws.settings.hosts[0].label = "Very long host ".repeat(30);
        ws.settings.hosts[0].endpoint = format!(
            "https://box.example/{}?password=secret",
            "folder/".repeat(40)
        );
        let rt = open(ws, size);
        let scroll = rect(&rt, "settings-scroll:server");
        for e in rt.elements().iter().filter(|e| {
            !e.exiting
                && e.path.contains("settings-scroll:server")
                && matches!(e.role, "text" | "button")
        }) {
            assert!(
                e.rect[0] >= scroll[0] - 0.5
                    && e.rect[0] + e.rect[2] <= scroll[0] + scroll[2] + 0.5,
                "{:?} outside {scroll:?}",
                e.rect
            );
            assert!(
                !e.help
                    .as_deref()
                    .is_some_and(|h| h.contains("password") || h.contains("secret"))
            );
        }
    }
}

fn draw(case: &str) {
    let Some(mut camera) = Camera::new() else {
        return;
    };
    let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
    let _families = rune::text::with_families("Inter", "IBM Plex Mono");
    let mut ws = fixture();
    match case {
        "older" => {
            ws.hosts[0].model.protocol.api_revision = 10;
            ws.hosts[0].model.server_info = ServerInfo {
                build: Answer::Unreported,
                process: Answer::Reported(Process::default()),
                ..Default::default()
            };
        }
        "disconnected" => {
            ws.hosts[0].model.connected = false;
            ws.hosts[0].model.status = "Disconnected".into();
        }
        "nohost" => {
            ws.hosts.clear();
            ws.settings.hosts.clear();
        }
        _ => {}
    }
    let size = if std::env::var_os("REX_M33_MINIMUM").is_some() {
        (640., 480.)
    } else {
        (1024., 700.)
    };
    prepare(&mut ws, size);
    let mut rt = Runtime::with_text(
        ws,
        page_view,
        size,
        Env::new(true, 1.),
        rune::text::TextSystem::new(fonts.files, fonts.families),
    );
    rt.on_key = Some(crate::app::input::on_key);
    settle(&mut rt);
    camera.snap(&mut rt, &format!("{case}-top"));
    if case != "nohost" {
        let scroll = rect(&rt, "settings-scroll:server");
        rt.wheel(scroll[0] + 20., scroll[1] + 30., -400.);
        settle(&mut rt);
        camera.snap(&mut rt, &format!("{case}-build"));
        rt.wheel(scroll[0] + 20., scroll[1] + 30., -4000.);
        settle(&mut rt);
        camera.snap(&mut rt, &format!("{case}-bottom"));
    }
}

#[test]
fn server_connected_draw() {
    draw("connected");
}
#[test]
fn server_older_draw() {
    draw("older");
}
#[test]
fn server_disconnected_draw() {
    draw("disconnected");
}
#[test]
fn server_nohost_draw() {
    draw("nohost");
}
