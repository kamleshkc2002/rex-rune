//! The installer's interface where there is nothing to install: every platform but Linux.
//!
//! Releases are published for Linux only, and only an install.sh installation there can
//! update itself (`installer.rs`). Elsewhere the client is built from source, so there is
//! never an interrupted update to recover, an old installation to move or a release to
//! install, and each function says so.
use super::CheckConfig;
use anyhow::{Result, bail};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum Progress {
    Verifying,
    Downloading { received: u64, total: u64 },
    Extracting,
    Installing,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Recovery {
    Unchanged,
    Restored(PathBuf),
    Superseded,
}

pub fn recover_current_install() -> Result<Recovery> {
    Ok(Recovery::Unchanged)
}

pub fn migrate_legacy_install() -> Result<Option<PathBuf>> {
    Ok(None)
}

pub fn install(
    _config: &CheckConfig,
    _expected: &str,
    _progress: impl FnMut(Progress),
) -> Result<PathBuf> {
    bail!(
        "Rex Rune updates itself only on Linux, from an install.sh installation; rebuild from source here"
    )
}
