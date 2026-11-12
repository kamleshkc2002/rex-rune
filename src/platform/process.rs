//! Other processes of this user, named so that a recycled process id is not mistaken for
//! the process that had it before.
//!
//! Two copies of the client share a runtime folder and a downloads record. Each removes
//! what a dead copy left behind, so "is that process still running" has to be answered
//! correctly: a wrong "no" deletes a running copy's tunnel sockets and partial downloads.

/// Whether a process with this id exists. One owned by another user counts.
pub fn exists(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return false;
    };
    if pid <= 0 {
        return false;
    }
    // Signal 0 sends nothing: it only reports whether the process could be signalled.
    let result = unsafe { libc::kill(pid, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// When the process started, in a unit that is only ever compared with itself on this
/// machine: with its id, that names one process for good. `None` when the process is gone
/// or the platform cannot say, which callers treat as "not the same process".
pub fn start_time(pid: u32) -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        // The command name is in parentheses and may hold anything; the fields follow its
        // last one. The start time, in clock ticks since boot, is the 22nd field.
        let rest = &stat[stat.rfind(')')? + 1..];
        rest.split_whitespace().nth(19)?.parse().ok()
    }
    #[cfg(target_os = "macos")]
    {
        let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
        let size = libc::c_int::try_from(std::mem::size_of::<libc::proc_bsdinfo>()).ok()?;
        let written = unsafe {
            libc::proc_pidinfo(
                libc::c_int::try_from(pid).ok()?,
                libc::PROC_PIDTBSDINFO,
                0,
                (&raw mut info).cast(),
                size,
            )
        };
        // Microseconds since the epoch, as the kernel recorded them when the process was made.
        (written == size).then(|| info.pbi_start_tvsec * 1_000_000 + info.pbi_start_tvusec)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = pid;
        None
    }
}

/// This user's numeric id, for naming a folder that is theirs alone.
pub fn user_id() -> String {
    unsafe { libc::getuid() }.to_string()
}

/// The process umask. Linux reports it without changing it. Elsewhere `umask(2)` can only
/// set it, so it is set and put back once and remembered: call this at the start of `main`,
/// before another thread could create a file in that instant.
pub fn umask() -> u32 {
    static UMASK: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *UMASK.get_or_init(|| {
        let reported = if cfg!(target_os = "linux") {
            std::fs::read_to_string("/proc/self/status")
                .ok()
                .and_then(|status| {
                    let line = status.lines().find_map(|l| l.strip_prefix("Umask:"))?;
                    u32::from_str_radix(line.trim(), 8).ok()
                })
        } else {
            None
        };
        let mask = reported.unwrap_or_else(|| {
            let mask = unsafe { libc::umask(0o077) };
            unsafe { libc::umask(mask) };
            // `mode_t` is 16 bits on macOS and 32 on Linux.
            u64::from(mask) as u32
        });
        mask & 0o777
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_process_exists_and_keeps_one_start_time() {
        let pid = std::process::id();
        assert!(exists(pid));
        if cfg!(any(target_os = "linux", target_os = "macos")) {
            let started = start_time(pid).expect("this process has a start time");
            assert_eq!(start_time(pid), Some(started));
        }
    }

    #[test]
    fn a_finished_process_is_gone_and_a_child_started_later() {
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .unwrap();
        let pid = child.id();
        assert!(exists(pid));
        if cfg!(any(target_os = "linux", target_os = "macos")) {
            let (ours, theirs) = (start_time(std::process::id()), start_time(pid));
            assert!(theirs.is_some());
            assert!(
                theirs >= ours,
                "a child cannot have started before its parent"
            );
        }
        child.kill().unwrap();
        child.wait().unwrap();
        assert!(!exists(pid), "a reaped child is gone");
        assert_eq!(start_time(pid), None);
        assert!(!exists(0) && !exists(u32::MAX));
    }

    #[test]
    fn the_user_id_is_a_number() {
        assert!(user_id().parse::<u32>().is_ok());
        // The real mask, the same at every reading: a new file gets exactly its complement.
        let mask = umask();
        assert_eq!(umask(), mask);
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let temp = tempfile::tempdir().unwrap();
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o666)
            .open(temp.path().join("probe"))
            .unwrap();
        let mode = file.metadata().unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o666 & !mask);
    }
}
