//! Build identity and bug-report text; no server request is made by this section.
use super::{
    settings_view::{divider, setting},
    theme::*,
    widgets::{Pill, pill},
    workspace::Workspace,
};
use rune::{El, Key, KeyEvent, Tx, col, text};

pub const REPOSITORY: &str = "https://github.com/shuv1337/rex-rune";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const COMMIT: &str = env!("REX_BUILD_COMMIT");
const BUILD_DATE: &str = env!("REX_BUILD_DATE");

pub fn release_url() -> String {
    format!("{REPOSITORY}/releases/tag/v{VERSION}")
}

fn format_info(
    version: &str,
    commit: &str,
    date: &str,
    os: &str,
    server_api: Option<u32>,
) -> String {
    let mut info = format!("Rex Linux {version}\nCommit: {commit}\nBuild date: {date}\nOS: {os}\n");
    if let Some(revision) = server_api {
        info.push_str(&format!("Server API revision: {revision}\n"));
    }
    info
}

fn server_api(s: &Workspace) -> Option<u32> {
    let host = &s.hosts[s.active].model;
    (host.connected && host.protocol.api_revision != 0).then_some(host.protocol.api_revision)
}

pub fn version_info(s: &Workspace) -> String {
    format_info(
        VERSION,
        COMMIT,
        BUILD_DATE,
        &format!("{} ({})", std::env::consts::OS, std::env::consts::ARCH),
        server_api(s),
    )
}

fn value(value: impl Into<String>) -> El {
    text(value.into())
        .font(ui(12.))
        .color(rgb(muted()))
        .selectable(true)
}

fn button(
    key: &str,
    label: &str,
    action: impl Fn(&mut Workspace, &mut Tx) + Clone + 'static,
) -> El {
    pill(key, label, Pill::Neutral, 30., action.clone()).on_key(
        move |s: &mut Workspace, event: &KeyEvent, tx: &mut Tx| {
            (event.key == Key::Enter).then(|| action(s, tx)).is_some()
        },
    )
}

pub fn section(s: &Workspace) -> El {
    let mut body = col()
        .gap(4.)
        .child(setting(
            "Rex Linux",
            "Native Linux client for Rex",
            value(VERSION),
        ))
        .child(setting("Commit", "", value(COMMIT)))
        .child(setting("Build date", "", value(BUILD_DATE)));
    if let Some(revision) = server_api(s) {
        body = body.child(
            setting(
                "Server API revision",
                "Active host · negotiated protocol",
                value(revision.to_string()),
            )
            .key("about-server"),
        );
    }
    body = body.child(divider());
    for (key, label, detail, url) in [
        (
            "about-repository",
            "Repository",
            "github.com/shuv1337/rex-rune",
            REPOSITORY.to_owned(),
        ),
        (
            "about-releases",
            "Release notes",
            "Changes in this version",
            release_url(),
        ),
        (
            "about-license",
            "License",
            "MIT",
            format!("{REPOSITORY}/blob/v{VERSION}/LICENSE"),
        ),
    ] {
        body = body.child(setting(
            label,
            detail,
            button(
                key,
                &format!("Open {}", label.to_lowercase()),
                move |s, _| s.open_link(url.clone()),
            ),
        ));
    }
    body.child(divider()).child(setting(
        "Bug reports",
        "Version details only; no host addresses or credentials.",
        button("about-copy", "Copy version info", |s, tx| {
            tx.copy(version_info(s));
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_includes_available_metadata_and_server() {
        assert_eq!(
            format_info(
                "0.5.0",
                "abc1234",
                "2026-11-10 PST",
                "linux (x86_64)",
                Some(12)
            ),
            "Rex Linux 0.5.0\nCommit: abc1234\nBuild date: 2026-11-10 PST\nOS: linux (x86_64)\nServer API revision: 12\n"
        );
    }

    #[test]
    fn source_archive_and_offline_report_are_honest() {
        assert_eq!(
            format_info("0.5.0", "unknown", "unknown", "linux", None),
            "Rex Linux 0.5.0\nCommit: unknown\nBuild date: unknown\nOS: linux\n"
        );
        assert_eq!(
            release_url(),
            format!("{REPOSITORY}/releases/tag/v{}", env!("CARGO_PKG_VERSION"))
        );
    }
}
