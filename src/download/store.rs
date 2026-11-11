//! Where downloads land on this machine: the destination folder, the partial file written into
//! it, the rename to a final name that never replaces anything, and the record of partial files
//! that lets the next launch clean up after a killed app.
//!
//! Every filesystem operation after the folder is opened is relative to its file descriptor, so a
//! folder swapped for a symlink (or anything else) afterwards cannot redirect a write. The folder
//! itself is opened without following a symlink in its last component. Components above it are
//! resolved as the system resolves them: they are the user's own configuration.
use super::name;
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    ffi::{CStr, CString, OsStr},
    io,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
        unix::ffi::OsStrExt,
    },
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::io::AsyncWriteExt;

/// Ends the name of every partial file.
pub const PARTIAL_SUFFIX: &str = ".rex-part";
/// The most names tried for one file before giving up on finding a free one.
const MAX_NUMBERED: u32 = 10_000;
/// A UUID is 36 bytes; the partial name is `.` + name + `.` + uuid + `.rex-part`.
const PARTIAL_OVERHEAD: usize = 1 + 1 + 36 + PARTIAL_SUFFIX.len();

fn c_string(bytes: &[u8]) -> io::Result<CString> {
    CString::new(bytes).map_err(|_| io::Error::from_raw_os_error(libc::EINVAL))
}

fn check(result: libc::c_int) -> io::Result<libc::c_int> {
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(result)
    }
}

fn open_directory(path: &Path) -> io::Result<OwnedFd> {
    let c = c_string(path.as_os_str().as_bytes())?;
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let fd = check(unsafe { libc::open(c.as_ptr(), flags) })?;
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

fn unlink_at(dir: RawFd, name: &CStr) -> io::Result<()> {
    check(unsafe { libc::unlinkat(dir, name.as_ptr(), 0) }).map(drop)
}

struct Dir {
    fd: OwnedFd,
    path: PathBuf,
}

/// A folder opened for writing downloads. Cheap to clone; clones share the descriptor.
#[derive(Clone)]
pub struct Destination {
    dir: Arc<Dir>,
}

impl std::fmt::Debug for Destination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Destination").field(&self.dir.path).finish()
    }
}

impl Destination {
    /// Opens `path`, which must be an absolute path to an existing folder that is not a symbolic
    /// link. A link is refused, with the folder it points to in the message.
    pub fn open(path: &Path) -> Result<Self> {
        ensure!(
            path.is_absolute(),
            "The download folder must be an absolute path, not {}",
            path.display()
        );
        ensure!(
            !path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir)),
            "The download folder can't contain \"..\": {}",
            path.display()
        );
        // Components drops a trailing slash, which would make the system follow a final symlink.
        let clean: PathBuf = path.components().collect();
        match open_directory(&clean) {
            Ok(fd) => Ok(Self {
                dir: Arc::new(Dir { fd, path: clean }),
            }),
            Err(error) => {
                let shown = clean.display();
                if let Ok(meta) = std::fs::symlink_metadata(&clean)
                    && meta.file_type().is_symlink()
                {
                    return Err(match std::fs::canonicalize(&clean) {
                        Ok(real) => anyhow::anyhow!(
                            "{shown} is a symbolic link, and downloads are never saved through one. Use {} instead.",
                            real.display()
                        ),
                        Err(_) => anyhow::anyhow!(
                            "{shown} is a symbolic link, and downloads are never saved through one."
                        ),
                    });
                }
                Err(match error.kind() {
                    io::ErrorKind::NotFound => anyhow::anyhow!("{shown} doesn't exist."),
                    io::ErrorKind::PermissionDenied => {
                        anyhow::anyhow!("{shown} can't be opened: permission denied.")
                    }
                    _ if error.raw_os_error() == Some(libc::ENOTDIR) => {
                        anyhow::anyhow!("{shown} isn't a folder.")
                    }
                    _ => anyhow::Error::new(error).context(format!("{shown} can't be opened")),
                })
            }
        }
    }

    pub fn path(&self) -> &Path {
        &self.dir.path
    }

    /// Bytes available to this user on the folder's filesystem.
    #[allow(clippy::useless_conversion)] // the statvfs fields are u32 on 32-bit targets
    pub fn free_space(&self) -> io::Result<u64> {
        let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
        check(unsafe { libc::fstatvfs(self.dir.fd.as_raw_fd(), &mut stat) })?;
        Ok(u64::from(stat.f_bavail).saturating_mul(u64::from(stat.f_frsize)))
    }

    /// Creates the partial file for a download of `final_name` (already sanitized), recording
    /// it in `records` first. The file is exclusive, never follows a symlink, and is private to
    /// the user until it is finished.
    pub fn create_partial(&self, final_name: &str, records: &Records) -> Result<Partial> {
        ensure!(
            name::is_component(final_name),
            "{final_name:?} is not a file name"
        );
        let uuid = crate::upload::new_uuid()?;
        let stem = name::fit(final_name, name::MAX_NAME - PARTIAL_OVERHEAD);
        let partial = format!(".{stem}.{uuid}{PARTIAL_SUFFIX}");
        debug_assert!(partial.len() <= name::MAX_NAME);
        let c = c_string(partial.as_bytes())?;
        // A record that outlives a failed create is harmless; a file without one is not.
        if let Err(error) = records.add(&self.dir.path, &partial) {
            log::warn!("download record not written: {error:#}");
        }
        let flags =
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC;
        let fd =
            match check(unsafe { libc::openat(self.dir.fd.as_raw_fd(), c.as_ptr(), flags, 0o600) })
            {
                Ok(fd) => fd,
                Err(error) => {
                    records.remove(&self.dir.path, &partial);
                    return Err(error)
                        .with_context(|| format!("Can't save into {}", self.dir.path.display()));
                }
            };
        let file = std::fs::File::from(unsafe { OwnedFd::from_raw_fd(fd) });
        Ok(Partial {
            dir: self.dir.clone(),
            name: partial,
            file: Some(tokio::fs::File::from_std(file)),
            records: records.clone(),
            written: 0,
            done: false,
        })
    }
}

/// How a finished partial file takes its final name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// `renameat2` with `RENAME_NOREPLACE`, and `linkat` plus `unlinkat` on a filesystem that
    /// doesn't support the flag.
    Auto,
    /// Always `linkat` plus `unlinkat`.
    Link,
}

/// A download in progress, on disk. Deleted when dropped unless [`Partial::finish`] succeeded.
pub struct Partial {
    dir: Arc<Dir>,
    name: String,
    file: Option<tokio::fs::File>,
    records: Records,
    written: u64,
    done: bool,
}

impl Partial {
    pub fn file_name(&self) -> &str {
        &self.name
    }

    pub fn written(&self) -> u64 {
        self.written
    }

    pub async fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        let file = self.file.as_mut().ok_or(io::ErrorKind::BrokenPipe)?;
        file.write_all(bytes).await?;
        self.written += bytes.len() as u64;
        Ok(())
    }

    /// Writes everything to disk, sets the mode to 0644 less the umask (never executable), and
    /// moves the file to `final_name`, or `final_name (1)` and so on if it exists. Nothing is
    /// ever replaced. Returns the path it was saved at.
    pub async fn finish(self, final_name: &str) -> Result<PathBuf> {
        self.finish_with(final_name, Placement::Auto).await
    }

    pub async fn finish_with(mut self, final_name: &str, how: Placement) -> Result<PathBuf> {
        ensure!(
            name::is_component(final_name),
            "{final_name:?} is not a file name"
        );
        let mut file = self.file.take().context("The download file is closed")?;
        file.flush().await.context("Writing the file")?;
        file.sync_all().await.context("Writing the file to disk")?;
        let mode = 0o644 & !current_umask();
        check(unsafe { libc::fchmod(file.as_raw_fd(), mode) })
            .context("Setting file permissions")?;
        drop(file);
        let dir = self.dir.fd.as_raw_fd();
        let from = c_string(self.name.as_bytes())?;
        for n in 0..MAX_NUMBERED {
            let candidate = name::numbered(final_name, n);
            let to = c_string(candidate.as_bytes())?;
            match place(dir, &from, &to, how) {
                Ok(()) => {
                    self.done = true;
                    self.records.remove(&self.dir.path, &self.name);
                    return Ok(self.dir.path.join(candidate));
                }
                Err(error) if error.raw_os_error() == Some(libc::EEXIST) => {}
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("Can't save into {}", self.dir.path.display()));
                }
            }
        }
        bail!(
            "Too many files named {final_name} in {}",
            self.dir.path.display()
        )
    }

    /// Deletes the file now.
    pub fn discard(self) {}
}

impl Drop for Partial {
    fn drop(&mut self) {
        if self.done {
            return;
        }
        self.file = None;
        if let Ok(name) = c_string(self.name.as_bytes()) {
            let _ = unlink_at(self.dir.fd.as_raw_fd(), &name);
        }
        self.records.remove(&self.dir.path, &self.name);
    }
}

fn place(dir: RawFd, from: &CStr, to: &CStr, how: Placement) -> io::Result<()> {
    const RENAME_NOREPLACE: libc::c_uint = 1;
    if how == Placement::Auto {
        let result = unsafe {
            libc::syscall(
                libc::SYS_renameat2,
                dir,
                from.as_ptr(),
                dir,
                to.as_ptr(),
                RENAME_NOREPLACE,
            )
        };
        if result == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        // The filesystem, or an old kernel, doesn't know the flag.
        if !matches!(
            error.raw_os_error(),
            Some(libc::EINVAL | libc::ENOSYS | libc::EOPNOTSUPP)
        ) {
            return Err(error);
        }
    }
    // linkat fails if the name exists and never replaces it.
    check(unsafe { libc::linkat(dir, from.as_ptr(), dir, to.as_ptr(), 0) })?;
    // The file is saved; a partial name that can't be removed is left to the next launch.
    let _ = unlink_at(dir, from);
    Ok(())
}

/// The process umask, read without changing it (`umask(2)` can only set it).
fn current_umask() -> u32 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            let line = status.lines().find_map(|l| l.strip_prefix("Umask:"))?;
            u32::from_str_radix(line.trim(), 8).ok()
        })
        .unwrap_or(0o022)
        & 0o777
}

// -- Folder choice --------------------------------------------------------------------------

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
}

fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home_dir().join(".config"))
}

/// The XDG download folder, as `xdg-user-dir DOWNLOAD` reports it from `user-dirs.dirs`, else
/// `~/Downloads`.
pub fn default_directory() -> PathBuf {
    default_directory_in(&home_dir(), &config_home())
}

pub fn default_directory_in(home: &Path, config_home: &Path) -> PathBuf {
    let fallback = home.join("Downloads");
    let Ok(text) = std::fs::read_to_string(config_home.join("user-dirs.dirs")) else {
        return fallback;
    };
    let value = text.lines().find_map(|line| {
        let value = line.trim().strip_prefix("XDG_DOWNLOAD_DIR=")?;
        Some(value.trim().trim_matches('"').to_owned())
    });
    let Some(value) = value else {
        return fallback;
    };
    let path = match value.strip_prefix("$HOME") {
        Some(rest) => home.join(rest.trim_start_matches('/')),
        None => PathBuf::from(&value),
    };
    // xdg-user-dirs points a disabled directory at $HOME itself.
    if !path.is_absolute() || path == home || path.components().count() == 1 {
        return fallback;
    }
    path.components().collect()
}

/// A folder as a person writes it in a setting: absolute, or starting with `~`.
pub fn expand_home(text: &str) -> PathBuf {
    match text.strip_prefix('~') {
        Some("") => home_dir(),
        Some(rest) if rest.starts_with('/') => home_dir().join(rest.trim_start_matches('/')),
        _ => PathBuf::from(text),
    }
}

// -- Records ---------------------------------------------------------------------------------

/// One partial file the app has created and not finished or deleted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub dir: PathBuf,
    pub name: String,
    /// The process that owns it, so a second running app's files are not taken for leftovers.
    pub pid: u32,
    pub started: u64,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct RecordFile {
    partials: Vec<Record>,
}

/// `$XDG_STATE_HOME/rex-linux/downloads.json`: the partial files this app has made. Only these
/// are ever deleted after a crash, and only if they still look like a partial file.
#[derive(Clone, Debug)]
pub struct Records {
    path: PathBuf,
}

impl Default for Records {
    fn default() -> Self {
        Self::at(crate::update::xdg_state_dir().join("rex-linux/downloads.json"))
    }
}

/// The start time of a process in clock ticks, which with its id names it for good.
fn process_start(pid: u32) -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The command name is in parentheses and may hold anything; the fields follow its last one.
    let rest = &stat[stat.rfind(')')? + 1..];
    rest.split_whitespace().nth(19)?.parse().ok()
}

impl Records {
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn load(&self) -> RecordFile {
        std::fs::read(&self.path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    fn save(&self, file: &RecordFile) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = self
            .path
            .with_extension(format!("json.tmp.{}", std::process::id()));
        std::fs::write(&temporary, serde_json::to_vec_pretty(file)?)?;
        std::fs::rename(temporary, &self.path)?;
        Ok(())
    }

    pub fn add(&self, dir: &Path, name: &str) -> Result<()> {
        let mut file = self.load();
        let pid = std::process::id();
        file.partials.push(Record {
            dir: dir.into(),
            name: name.into(),
            pid,
            started: process_start(pid).unwrap_or(0),
        });
        self.save(&file)
    }

    pub fn remove(&self, dir: &Path, name: &str) {
        let mut file = self.load();
        let before = file.partials.len();
        file.partials.retain(|r| !(r.dir == dir && r.name == name));
        if file.partials.len() != before
            && let Err(error) = self.save(&file)
        {
            log::warn!("download record not updated: {error:#}");
        }
    }

    pub fn list(&self) -> Vec<Record> {
        self.load().partials
    }

    /// At launch: deletes the partial files of apps that no longer run, and forgets them.
    /// Only recorded files are looked at; the folder is never listed. A record can delete only
    /// a regular file, directly inside the recorded folder, whose name has the form of a partial
    /// file; nothing is followed through a symlink. Returns what was deleted.
    pub fn clean_up_leftovers(&self) -> Vec<PathBuf> {
        let file = self.load();
        if file.partials.is_empty() {
            return Vec::new();
        }
        let mut keep = Vec::new();
        let mut deleted = Vec::new();
        for record in file.partials {
            let running = record.started != 0 && process_start(record.pid) == Some(record.started);
            if running {
                keep.push(record);
            } else if let Some(path) = delete_leftover(&record) {
                deleted.push(path);
            }
        }
        if let Err(error) = self.save(&RecordFile { partials: keep }) {
            log::warn!("download records not updated: {error:#}");
        }
        deleted
    }
}

/// Whether `name` is `.<name>.<uuid>.rex-part`, a single path component.
pub fn is_partial_name(name: &str) -> bool {
    let Some(inner) = name
        .strip_prefix('.')
        .and_then(|n| n.strip_suffix(PARTIAL_SUFFIX))
    else {
        return false;
    };
    if name.len() > name::MAX_NAME || name.contains(['/', '\0']) || inner.len() < 38 {
        return false;
    }
    let (stem, uuid) = inner.split_at(inner.len() - 36);
    let Some(stem) = stem.strip_suffix('.') else {
        return false;
    };
    let groups: Vec<&str> = uuid.split('-').collect();
    !stem.is_empty()
        && groups.iter().map(|g| g.len()).eq([8, 4, 4, 4, 12])
        && uuid
            .bytes()
            .all(|b| b == b'-' || b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn delete_leftover(record: &Record) -> Option<PathBuf> {
    if !record.dir.is_absolute()
        || record
            .dir
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        || !is_partial_name(&record.name)
    {
        return None;
    }
    let clean: PathBuf = record.dir.components().collect();
    let dir = open_directory(&clean).ok()?;
    let name = c_string(record.name.as_bytes()).ok()?;
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    check(unsafe {
        libc::fstatat(
            dir.as_raw_fd(),
            name.as_ptr(),
            &mut stat,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    })
    .ok()?;
    if stat.st_mode & libc::S_IFMT != libc::S_IFREG || stat.st_uid != unsafe { libc::geteuid() } {
        return None;
    }
    unlink_at(dir.as_raw_fd(), &name).ok()?;
    Some(clean.join(OsStr::from_bytes(record.name.as_bytes())))
}
