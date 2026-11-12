//! The per-user folders this client reads and writes.
//!
//! | Folder | Linux and other Unix | macOS |
//! | --- | --- | --- |
//! | settings | `$XDG_CONFIG_HOME/rex-rune` (`~/.config/rex-rune`) | `~/Library/Application Support/rex-rune` |
//! | state | `$XDG_STATE_HOME/rex-rune` (`~/.local/state/rex-rune`) | `~/Library/Application Support/rex-rune` |
//! | runtime | `$XDG_RUNTIME_DIR/rex-rune`, else `<tmp>/rex-rune-<uid>` | `<tmp>/rex-rune-<uid>` |
//!
//! Settings are `workspace.json` and `shortcuts.json`; state is `update.json` and
//! `downloads.json`; runtime holds the sockets of SSH tunnels. `<tmp>` on macOS is the
//! user's own temporary folder, whatever `TMPDIR` says. A relative `XDG_CONFIG_HOME` is
//! used as it is given and a relative `XDG_STATE_HOME` is ignored, as before these moved
//! here.
use std::path::{Path, PathBuf};

/// The folder name this client uses under each base folder.
pub const APP: &str = "rex-rune";

/// The user's home folder, or an empty path when `HOME` is unset.
pub fn home() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
}

/// Where the settings files live.
pub fn config() -> PathBuf {
    config_base().join(APP)
}

/// Where update and download records live.
pub fn state() -> PathBuf {
    state_base().join(APP)
}

/// Where this user's SSH tunnel sockets live, one folder per process.
pub fn runtime() -> PathBuf {
    match xdg_runtime() {
        Some(runtime) => runtime.join(APP),
        None => user_temp().join(format!("{APP}-{}", super::process::user_id())),
    }
}

/// A temporary folder for this user. macOS gives each user one of their own; the system
/// is asked for it, because `TMPDIR` can be missing (under `env -i`, `sudo -u`, some
/// launchd jobs), and then `std::env::temp_dir` is the `/tmp` everyone shares.
fn user_temp() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let mut buffer = vec![0u8; libc::PATH_MAX as usize];
        let written = unsafe {
            libc::confstr(
                libc::_CS_DARWIN_USER_TEMP_DIR,
                buffer.as_mut_ptr().cast(),
                buffer.len(),
            )
        };
        if written > 1 && written <= buffer.len() {
            use std::os::unix::ffi::OsStringExt;
            buffer.truncate(written - 1);
            return PathBuf::from(std::ffi::OsString::from_vec(buffer));
        }
    }
    std::env::temp_dir()
}

/// Make `path` a folder only this user can enter, creating it if need be, and refuse one
/// that is someone else's or that others can get into. A socket in a folder another user
/// made or can write to could be swapped for theirs.
pub fn private(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true).mode(0o700);
    builder.create(path)?;
    let refuse = |why: &str| {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("{} {why}", path.display()),
        ))
    };
    let meta = std::fs::symlink_metadata(path)?;
    if !meta.is_dir() {
        return refuse("is not a folder");
    }
    if meta.uid() != unsafe { libc::getuid() } {
        return refuse("belongs to another user");
    }
    if meta.mode() & 0o077 != 0 {
        // Ours, but open: close it. Whatever is inside is checked the same way in turn.
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// The folders that held `rex-linux` folders through 0.7.x, for the one-time move in
/// [`crate::legacy`]. Empty where that client never ran.
pub fn legacy_bases() -> Vec<PathBuf> {
    if cfg!(target_os = "linux") {
        vec![config_base(), state_base()]
    } else {
        Vec::new()
    }
}

fn application_support() -> PathBuf {
    home().join("Library/Application Support")
}

fn config_base() -> PathBuf {
    if cfg!(target_os = "macos") {
        return application_support();
    }
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".config"))
}

fn state_base() -> PathBuf {
    if cfg!(target_os = "macos") {
        return application_support();
    }
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home().join(".local/state"))
}

/// `$XDG_RUNTIME_DIR` where sessions have one: not on macOS, which sets none.
pub(super) fn xdg_runtime() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        return None;
    }
    std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|runtime| !runtime.is_empty())
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_and_state_are_named_for_the_client_under_the_platforms_folder() {
        assert!(config().ends_with(APP) && state().ends_with(APP));
        if cfg!(target_os = "macos") {
            let support = home().join("Library/Application Support").join(APP);
            assert_eq!(config(), support);
            assert_eq!(state(), support);
            assert!(legacy_bases().is_empty(), "rex-linux never ran on macOS");
        }
        if cfg!(target_os = "linux") {
            assert_eq!(legacy_bases().len(), 2);
        }
        // The runtime folder is this user's alone: named for them unless the session gives one.
        let runtime = runtime();
        let name = runtime.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name == APP || name == format!("{APP}-{}", crate::platform::process::user_id()));
        if cfg!(target_os = "macos") {
            assert!(
                runtime.starts_with("/var/folders") || runtime.starts_with("/private/var/folders"),
                "the user's own temporary folder, not the shared /tmp: {}",
                runtime.display()
            );
        }
    }

    #[test]
    fn a_private_folder_is_made_closed_and_an_open_or_foreign_one_is_not_trusted() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        let made = temp.path().join("a/b");
        private(&made).unwrap();
        assert_eq!(mode(&made), 0o700);
        // One of ours that was left open is closed.
        std::fs::set_permissions(&made, std::fs::Permissions::from_mode(0o777)).unwrap();
        private(&made).unwrap();
        assert_eq!(mode(&made), 0o700);
        // A file, or a link to a folder, in its place is refused.
        let file = temp.path().join("file");
        std::fs::write(&file, b"").unwrap();
        assert!(private(&file).is_err());
        let link = temp.path().join("link");
        std::os::unix::fs::symlink(&made, &link).unwrap();
        assert!(private(&link).is_err());
        // So is a folder that belongs to someone else (root's, here).
        if unsafe { libc::getuid() } != 0 {
            assert!(private(Path::new("/")).is_err());
        }
    }
}
