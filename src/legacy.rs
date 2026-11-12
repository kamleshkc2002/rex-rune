//! What remains of `rex-linux`, the name this client had through 0.7.x: the name it still
//! answers to, and the one-time move of that name's settings and state. Installations are
//! moved by [`crate::update::installer::migrate_legacy_install`]. `docs/releases.md` says
//! when all of this can go.
use std::path::{Path, PathBuf};

/// The former binary, package, settings folder and release-metadata name.
pub const NAME: &str = "rex-linux";
/// The name every one of those has now.
pub const CURRENT: &str = "rex-rune";

/// Whether this process was started through the old name, by the compatibility link or by
/// the 0.7.x updater checking the binary it is about to install.
pub fn invoked_as_legacy() -> bool {
    std::env::args_os()
        .next()
        .is_some_and(|program| Path::new(&program).file_name().is_some_and(|n| n == NAME))
}

/// Move the old name's settings and state folders to the new name, once. A folder is moved
/// only while the new one does not exist, so nothing is ever merged or overwritten, and a
/// failure leaves both names as they were. Returns what was moved, for the caller to report.
pub fn migrate_user_dirs() -> Vec<(PathBuf, PathBuf)> {
    crate::platform::dirs::legacy_bases()
        .iter()
        .filter_map(|root| migrate_dir(root))
        .collect()
}

fn migrate_dir(root: &Path) -> Option<(PathBuf, PathBuf)> {
    let (old, new) = (root.join(NAME), root.join(CURRENT));
    // A link is the user's own arrangement; leave it for them.
    let is_plain_dir = std::fs::symlink_metadata(&old).is_ok_and(|meta| meta.is_dir());
    if !is_plain_dir || std::fs::symlink_metadata(&new).is_ok() {
        return None;
    }
    match std::fs::rename(&old, &new) {
        Ok(()) => Some((old, new)),
        Err(error) => {
            eprintln!(
                "Could not move {} to {}: {error}. Rex Rune starts with the settings in the new folder.",
                old.display(),
                new.display()
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_moves_once_and_never_onto_an_existing_one() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        assert_eq!(migrate_dir(root), None);

        std::fs::create_dir(root.join(NAME)).unwrap();
        std::fs::write(root.join(NAME).join("workspace.json"), b"old").unwrap();
        assert_eq!(
            migrate_dir(root),
            Some((root.join(NAME), root.join(CURRENT)))
        );
        assert!(!root.join(NAME).exists());
        assert_eq!(
            std::fs::read(root.join(CURRENT).join("workspace.json")).unwrap(),
            b"old"
        );

        // An old client run afterwards makes its folder again: the new one wins untouched.
        std::fs::create_dir(root.join(NAME)).unwrap();
        std::fs::write(root.join(NAME).join("workspace.json"), b"older").unwrap();
        assert_eq!(migrate_dir(root), None);
        assert_eq!(
            std::fs::read(root.join(CURRENT).join("workspace.json")).unwrap(),
            b"old"
        );
        assert!(root.join(NAME).join("workspace.json").exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_old_folder_is_left_alone() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir(root.join("elsewhere")).unwrap();
        std::os::unix::fs::symlink(root.join("elsewhere"), root.join(NAME)).unwrap();
        assert_eq!(migrate_dir(root), None);
        assert!(!root.join(CURRENT).exists());
    }
}
