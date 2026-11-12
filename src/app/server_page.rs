//! A view of the active host's existing identity cache. No requests, timers or server actions.
use super::{
    settings_kit::{self as kit, button, card, footnote, page},
    theme::*,
    widgets::Pill,
    workspace::Workspace,
};
use rex_rune::connection::server_info::Answer;
use rune::{El, col, text};

const MISSING: &str = "Not reported";

/// Preserve literal absolute Unix paths; strip credentials, query and fragment from URLs.
/// Never link reported addresses. Unknown listener syntax is not echoed unfiltered.
pub(super) fn address(raw: &str) -> Option<String> {
    if raw.chars().any(char::is_control) {
        return None;
    }
    // Transport treats everything after unix:// as a literal path, including ? and #.
    if let Some(path) = raw.strip_prefix("unix://") {
        return path.starts_with('/').then(|| raw.to_owned());
    }
    if let Ok(socket) = raw.parse::<std::net::SocketAddr>() {
        return Some(socket.to_string());
    }
    let mut url = url::Url::parse(raw).ok()?;
    match url.scheme() {
        "https" | "http" | "wss" | "ws" | "ssh" if url.host_str().is_some() => {
            url.set_password(None).ok()?;
            url.set_username("").ok()?;
        }
        _ => return None,
    }
    url.set_query(None);
    url.set_fragment(None);
    Some(url.to_string())
}

fn absent<T>(answer: &Answer<T>, connected: bool) -> &'static str {
    if connected && !answer.settled() {
        "Waiting for report"
    } else {
        MISSING
    }
}

/// Stacked values leave room for long addresses at 640 pt. Search indexes the actual label,
/// not the host's private values. The whole value remains selectable and in its tooltip.
fn fact(label: &str, value: impl Into<String>, control: Option<El>) -> El {
    let value = value.into();
    let words = col()
        .grow(1.)
        .shrink()
        .min_w(0.)
        .gap(2.)
        .child(text(label.to_owned()).font(ui(13.)).color(rgb(ink())))
        .child(
            kit::value(value.clone())
                .w_full()
                .truncate()
                .help_when_truncated(value),
        );
    let mut row = super::settings_nav::row(label, "", kit::row_frame(true)).child(words);
    if let Some(control) = control {
        row = row.child(control);
    }
    row
}

fn address_row(key: &str, label: &str, address: Option<String>, fallback: &str) -> El {
    let control = address.as_ref().map(|address| {
        let address = address.clone();
        button(key, "Copy", Pill::Neutral, move |s, tx| {
            tx.copy(address.clone());
            s.inform("Copied address to clipboard.");
        })
        .help(format!("Copy {}", label.to_lowercase()))
    });
    fact(label, address.unwrap_or_else(|| fallback.into()), control)
}

fn last_state(state: &str, connected: bool) -> String {
    if connected || state == MISSING {
        state.into()
    } else {
        format!("{state} (last reported)")
    }
}

pub fn section(s: &Workspace) -> El {
    let Some((host, saved)) = s.hosts.get(s.active).zip(s.settings.hosts.get(s.active)) else {
        return page([kit::section(
            "",
            [
                footnote("No active host. Add a host to see its reported server details.")
                    .key("server-empty"),
            ],
        )]);
    };
    let model = &host.model;
    let info = &model.server_info;
    let current = kit::section(
        "Active host",
        [card([
            fact("Configured host", saved.label.clone(), None),
            fact("Connection", super::about::state(model), None),
            address_row(
                "server-copy-endpoint",
                "Configured address",
                address(&saved.endpoint),
                "Address unavailable",
            ),
        ])],
    );
    let build = info.build.reported();
    let process = info.process.reported();
    let b_missing = absent(&info.build, model.connected);
    let p_missing = absent(&info.process, model.connected);
    let build_text = |value: Option<&str>| value.unwrap_or(b_missing).to_owned();
    let api = if !model.connected {
        "Not connected".into()
    } else if model.protocol.api_revision == 0 {
        MISSING.into()
    } else {
        model.protocol.api_revision.to_string()
    };
    let build = kit::section(
        "Server build",
        [card([
            fact(
                "Server version",
                build_text(build.and_then(|b| b.rex_version.as_deref())),
                None,
            ),
            fact("Negotiated API revision", api, None),
            fact(
                "Supported API revisions",
                info.api_revisions
                    .map_or_else(|| MISSING.into(), |(min, max)| format!("{min}–{max}")),
                None,
            ),
            fact(
                "Platform",
                build_text(build.and_then(|b| b.platform.as_deref())),
                None,
            ),
            fact(
                "Commit",
                build_text(build.and_then(|b| b.commit.as_deref())),
                None,
            ),
            fact(
                "Working tree",
                build
                    .and_then(|b| b.dirty)
                    .map_or(b_missing, |dirty| if dirty { "Modified" } else { "Clean" }),
                None,
            ),
            fact(
                "Built at",
                build_text(build.and_then(|b| b.built_at.as_deref())),
                None,
            ),
        ])],
    );
    let process_rows = kit::section(
        "Server process",
        [card([
            fact(
                "Process ID",
                process
                    .and_then(|p| p.pid)
                    .map_or_else(|| p_missing.into(), |pid| pid.to_string()),
                None,
            ),
            fact(
                "Started at",
                process
                    .and_then(|p| p.started_at.as_deref())
                    .unwrap_or(p_missing),
                None,
            ),
            fact(
                "Session count",
                process
                    .and_then(|p| p.session_count)
                    .map_or_else(|| p_missing.into(), |n| n.to_string()),
                None,
            ),
        ])],
    );
    let listeners = process.map(|p| &p.listeners);
    let mut listener_rows = vec![fact(
        "Listeners",
        listeners
            .filter(|l| !l.is_empty())
            .map_or_else(|| p_missing.into(), |l| format!("{} reported", l.len())),
        None,
    )];
    for (index, listener) in listeners.into_iter().flatten().enumerate() {
        listener_rows.push(fact(
            &format!("Listener {}", index + 1),
            address(&listener.endpoint).unwrap_or_else(|| "Endpoint format not shown".into()),
            None,
        ));
    }
    let listeners = kit::section(
        "Server listeners",
        [
            card(listener_rows),
            footnote(
                "Last reported endpoints, not a reachability check. Bound addresses and admission details are not shown.",
            ),
        ],
    );
    let tailscale = process.and_then(|p| p.tailscale.as_ref());
    let state = tailscale.map_or(p_missing, |t| match t.state.as_str() {
        "disabled" => "Disabled",
        "starting" => "Starting",
        "needs_login" => "Needs sign-in",
        "needs_machine_approval" => "Needs machine approval",
        "running" => "Running",
        _ => "Unknown (starting)",
    });
    let https = tailscale.and_then(|t| t.https.as_ref());
    let https_state = https.map_or(p_missing, |h| {
        if h.state == "available" {
            "Available"
        } else {
            "Pending"
        }
    });
    let remote = https
        .filter(|h| h.state == "available")
        .and_then(|h| h.url.as_deref())
        .and_then(address)
        .filter(|url| url.starts_with("https://"));
    let tailscale = kit::section(
        "Server built-in Tailscale",
        [
            card([
                fact(
                    "Built-in Tailscale",
                    last_state(state, model.connected),
                    None,
                ),
                fact(
                    "Tailscale HTTPS",
                    last_state(https_state, model.connected),
                    None,
                ),
                address_row(
                    "server-copy-tailnet",
                    "Tailscale address",
                    remote,
                    p_missing,
                ),
            ]),
            footnote(
                "Last reported by this Rex server, not this client's local Tailscale. Missing data does not mean disabled.",
            ),
        ],
    );
    page([
        current,
        footnote(if model.connected {
            "Last reported server facts, not live monitoring. Counts and states may have changed. URLs omit credentials, queries and fragments; Unix paths and timestamps are as reported."
        } else {
            "Disconnected: last reported server facts only. They do not mean the server is running now. URLs omit credentials, queries and fragments; Unix paths and timestamps are as reported."
        }).key("server-freshness"),
        build,
        process_rows,
        listeners,
        tailscale,
    ])
}
