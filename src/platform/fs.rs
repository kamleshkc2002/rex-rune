//! File operations with no portable form.
use std::{ffi::CStr, io, os::fd::RawFd};

/// Rename `from` to `to`, both names in the open directory `dir`, in one step and only if
/// `to` does not exist. `EEXIST` when it does.
///
/// Linux has `renameat2(RENAME_NOREPLACE)` and macOS `renameatx_np(RENAME_EXCL)`. Where
/// neither exists, or the filesystem does not support the flag, the error satisfies
/// [`no_replace_unsupported`] and the caller links and unlinks instead.
pub fn rename_no_replace(dir: RawFd, from: &CStr, to: &CStr) -> io::Result<()> {
    #[cfg(target_os = "linux")]
    let result = {
        const RENAME_NOREPLACE: libc::c_uint = 1;
        unsafe {
            libc::syscall(
                libc::SYS_renameat2,
                dir,
                from.as_ptr(),
                dir,
                to.as_ptr(),
                RENAME_NOREPLACE,
            )
        }
    };
    #[cfg(target_os = "macos")]
    let result = libc::c_long::from(unsafe {
        libc::renameatx_np(dir, from.as_ptr(), dir, to.as_ptr(), libc::RENAME_EXCL)
    });
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    let result = {
        let _ = (dir, from, to);
        return Err(io::Error::from_raw_os_error(libc::ENOSYS));
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Whether an error from [`rename_no_replace`] means the system, an old kernel or the
/// filesystem does not know the operation, as opposed to the rename having failed.
pub fn no_replace_unsupported(error: &io::Error) -> bool {
    // ENOTSUP and EOPNOTSUPP are one number on Linux and two on macOS.
    error.raw_os_error().is_some_and(|code| {
        [libc::EINVAL, libc::ENOSYS, libc::EOPNOTSUPP, libc::ENOTSUP].contains(&code)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{ffi::CString, os::fd::AsRawFd};

    #[test]
    fn a_rename_never_replaces_and_says_when_it_cannot_be_done() {
        let temp = tempfile::tempdir().unwrap();
        let dir = std::fs::File::open(temp.path()).unwrap();
        let name = |name: &str| CString::new(name).unwrap();
        std::fs::write(temp.path().join("a"), b"a").unwrap();
        std::fs::write(temp.path().join("b"), b"b").unwrap();
        match rename_no_replace(dir.as_raw_fd(), &name("a"), &name("b")) {
            Err(error) if no_replace_unsupported(&error) => {
                // Linux and macOS have the call, on every filesystem tests run on.
                if cfg!(any(target_os = "linux", target_os = "macos")) {
                    panic!("{error}");
                }
                return;
            }
            Err(error) => assert_eq!(error.raw_os_error(), Some(libc::EEXIST)),
            Ok(()) => panic!("replaced an existing file"),
        }
        assert_eq!(std::fs::read(temp.path().join("b")).unwrap(), b"b");
        rename_no_replace(dir.as_raw_fd(), &name("a"), &name("c")).unwrap();
        assert!(!temp.path().join("a").exists());
        assert_eq!(std::fs::read(temp.path().join("c")).unwrap(), b"a");
    }
}
