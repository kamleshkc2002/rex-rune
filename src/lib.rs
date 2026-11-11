/// Build provenance only. Runtime package markers/receipts distinguish official installs.
/// Unset (including ordinary source builds) deliberately means source, not tarball.
pub const BUILD_CHANNEL: &str = match option_env!("REX_LINUX_CHANNEL") {
    Some(channel) => channel,
    None => "source",
};

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
