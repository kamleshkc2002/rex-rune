/// Build provenance only. Runtime package markers/receipts distinguish official installs.
/// Unset (including ordinary source builds) deliberately means source, not tarball.
pub const BUILD_CHANNEL: &str = match option_env!("REX_LINUX_CHANNEL") {
    Some(channel) => channel,
    None => "source",
};

/// A Rex Linux version as every page, notice, command and copied report prints it: the
/// bare number, without the `v` of a release tag.
pub fn format_version(version: &str) -> String {
    version.trim().trim_start_matches(['v', 'V']).to_owned()
}

pub mod connection;
pub mod download;
pub mod keymap;
pub mod layout;
pub mod links;
pub mod settings;
pub mod ssh;
pub mod tailnet;
pub mod terminal;
pub mod terminal_theme;
pub mod transport;
pub mod update;
pub mod upload;
pub mod wire;
