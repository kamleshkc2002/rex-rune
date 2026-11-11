//! Rex help: this version's guide, opened in the desktop's browser.
use super::{
    about::REPOSITORY,
    panel::{Entry, Intent},
};
use serde_json::Value;

pub const ACTION: &str = "client.help";

/// The README of the release this build is, so the guide matches what is installed.
pub fn url() -> String {
    format!("{REPOSITORY}/blob/v{}/README.md", env!("CARGO_PKG_VERSION"))
}

/// Commands' row.
pub fn command() -> Entry {
    Entry::new(
        "Rex help",
        "",
        "",
        Intent::ClientAction(ACTION, Value::Null),
    )
}
