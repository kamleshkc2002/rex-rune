//! Transactional install.sh-channel updater. Never runs an archive's installer script.
//! All replacements and backups are on the installation filesystem. An interrupted
//! transaction is rolled back on the next launch, before channel/receipt detection.
use super::*;
use std::{
    collections::HashSet,
    io::{self, Write},
    os::unix::fs::{MetadataExt, PermissionsExt},
    process::{Command, Stdio},
    time::Instant,
};

const MAX_ARCHIVE: u64 = 256 * 1024 * 1024;
const MAX_UNPACKED: u64 = 1024 * 1024 * 1024;
const DESKTOP: &str = "dev.shuv.rex-linux.desktop";
const RECOVERY_WAIT: Duration = Duration::from_secs(2);

// Order is also the journal's stable file order. Keep destinations and modes together.
const INSTALL_FILES: [(&str, u32); 6] = [
    ("icons/hicolor/32x32/apps/dev.shuv.rex-linux.png", 0o644),
    ("icons/hicolor/128x128/apps/dev.shuv.rex-linux.png", 0o644),
    ("icons/hicolor/256x256/apps/dev.shuv.rex-linux.png", 0o644),
    ("applications/dev.shuv.rex-linux.desktop", 0o644),
    ("", 0o755), // binary, outside the data directory
    ("rex-linux/install.json", 0o600),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Identity {
    dev: u64,
    ino: u64,
    size: u64,
    mtime: i64,
    mtime_nsec: i64,
}

fn identity(path: &Path) -> io::Result<Identity> {
    let meta = fs::symlink_metadata(path)?;
    Ok(Identity {
        dev: meta.dev(),
        ino: meta.ino(),
        size: meta.size(),
        mtime: meta.mtime(),
        mtime_nsec: meta.mtime_nsec(),
    })
}

#[derive(Clone, Debug)]
pub enum Progress {
    Verifying,
    Downloading { received: u64, total: u64 },
    Extracting,
    Installing,
}

#[derive(Serialize, Deserialize)]
struct Journal {
    binary: PathBuf,
    data: PathBuf,
    existed: Vec<bool>,
    #[serde(default)]
    new_inodes: Vec<Identity>,
}

#[derive(Clone, Debug)]
struct InstallLayout {
    local: PathBuf,
    binary: PathBuf,
    data: PathBuf,
}

impl InstallLayout {
    fn current() -> Result<Self> {
        let home = PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?);
        let bin = std::env::var_os("XDG_BIN_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/bin"));
        Ok(Self {
            local: home.join(".local"),
            binary: bin.join("rex-linux"),
            data: xdg_data_dir(),
        })
    }

    fn receipt(&self) -> PathBuf {
        self.data.join("rex-linux/install.json")
    }

    fn transaction(&self) -> PathBuf {
        self.binary.with_file_name(".rex-update-transaction")
    }

    fn destinations(&self) -> Vec<PathBuf> {
        INSTALL_FILES
            .iter()
            .map(|(path, _)| {
                if path.is_empty() {
                    self.binary.clone()
                } else {
                    self.data.join(path)
                }
            })
            .collect()
    }

    fn validate_paths(&self) -> Result<()> {
        ensure!(
            unsafe { libc::geteuid() } != 0,
            "self-update never runs as root"
        );
        let local = fs::canonicalize(&self.local).context("missing ~/.local installation")?;
        let device = fs::metadata(&local)?.dev();
        for path in self.destinations() {
            ensure!(path.is_absolute(), "install paths must be absolute");
            let parent = fs::canonicalize(path.parent().context("missing install parent")?)
                .with_context(|| format!("missing install directory for {}", path.display()))?;
            ensure!(
                parent.starts_with(&local) && fs::metadata(&parent)?.dev() == device,
                "self-update requires all install files on one filesystem under ~/.local"
            );
            if let Ok(meta) = fs::symlink_metadata(&path) {
                ensure!(
                    meta.is_file() && !meta.file_type().is_symlink(),
                    "refusing non-regular install file {}",
                    path.display()
                );
                ensure!(
                    meta.uid() == unsafe { libc::geteuid() } && meta.mode() & 0o200 != 0,
                    "install file is not user-writable: {}",
                    path.display()
                );
            }
            // Creating a private file tests actual directory access (including ACLs).
            tempfile::tempfile_in(&parent).with_context(|| {
                format!("install directory is not writable: {}", parent.display())
            })?;
        }
        Ok(())
    }

    fn validate_receipt(&self, current: &str) -> Result<()> {
        ensure!(
            verify_install_receipt(&self.receipt(), &self.binary),
            "install receipt does not verify against this binary; repair with install.sh before updating"
        );
        let receipt: InstallReceipt = serde_json::from_slice(&fs::read(self.receipt())?)?;
        ensure!(
            receipt.version == current,
            "installed version differs from this running app; restart before updating"
        );
        Ok(())
    }

    fn lock(&self) -> Result<FileLock> {
        FileLock::try_acquire(&self.binary.with_file_name(".rex-update.lock"))
            .context("another update is running, or the install directory is not writable")
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Recovery {
    Unchanged,
    Restored(PathBuf),
    Superseded,
}

/// Recover only this user's running install, never package-managed or source builds.
/// Returns the restored binary when rollback requires re-exec before UI startup.
pub fn recover_current_install() -> Result<Recovery> {
    let Ok(mut layout) = InstallLayout::current() else {
        return Ok(Recovery::Unchanged);
    };
    // Desktop launches and terminal launches can have different XDG settings.
    // Find the journal beside the actual executable, and restore its saved data path.
    layout.binary = std::env::current_exe()?;
    recover_install_at(&layout, RECOVERY_WAIT, unsafe { libc::geteuid() } == 0)
}

fn recover_install_at(
    layout: &InstallLayout,
    timeout: Duration,
    is_root: bool,
) -> Result<Recovery> {
    // Source/package builds have nothing to recover or sweep.
    let under_local =
        fs::canonicalize(&layout.local).is_ok_and(|local| layout.binary.starts_with(local));
    let journal_exists = layout.transaction().exists();
    let lock_path = layout.binary.with_file_name(".rex-update.lock");
    if !journal_exists {
        // Housekeeping must never delay or warn on an ordinary launch. An active
        // updater owns its staging directories; root must not sweep user files.
        if under_local
            && !is_root
            && let Some(_lock) = FileLock::try_acquire(&lock_path)
        {
            let _ = sweep_staging(layout);
        }
        return Ok(Recovery::Unchanged);
    }
    ensure!(!is_root, "update recovery never runs as root");
    ensure!(
        under_local,
        "update recovery requires a user installation under ~/.local"
    );
    let _lock = FileLock::acquire_timeout(&lock_path, timeout)
        .context("update recovery timed out waiting for another update, or install directory is not writable")?;
    // Recheck the journal under the lock: another process may have completed it.
    let recovery = recover(layout)?;
    let _ = sweep_staging(layout);
    Ok(recovery)
}

/// Download and install a freshly re-verified manifest, never trusting cached UI metadata.
/// The expected version binds the operation to the release shown when the user clicked.
pub fn install(
    config: &CheckConfig,
    expected: &str,
    mut progress: impl FnMut(Progress),
) -> Result<PathBuf> {
    let mut layout = InstallLayout::current()?;
    // Resolve a symlinked HOME/bin directory, but retain the final component so
    // validate_paths still rejects a symlink in place of the installed file.
    layout.binary = fs::canonicalize(layout.binary.parent().unwrap())?.join("rex-linux");
    ensure!(
        InstallChannel::detect() == InstallChannel::Tarball,
        "self-update requires a verified install.sh receipt (not a package or source build)"
    );
    ensure!(
        std::env::current_exe()? == fs::canonicalize(&layout.binary)?,
        "running binary is not the install.sh binary at {}",
        layout.binary.display()
    );
    install_with(config, expected, &layout, &mut progress, download)?;
    Ok(layout.binary)
}

fn install_with(
    config: &CheckConfig,
    expected: &str,
    layout: &InstallLayout,
    progress: &mut impl FnMut(Progress),
    fetch: impl FnOnce(&TarballArtifact, &Path, &mut dyn FnMut(Progress)) -> Result<()>,
) -> Result<()> {
    layout.validate_paths()?;
    let _lock = layout.lock()?;
    recover(layout)?;
    layout.validate_receipt(&config.current_version)?;
    progress(Progress::Verifying);
    let manifest = fetch_verified_manifest(config)?;
    ensure!(
        manifest.version == expected,
        "release changed since the check; use Check now and try again"
    );
    ensure!(
        StableVersion::parse(&manifest.version)? > StableVersion::parse(&config.current_version)?,
        "refusing an equal version or downgrade"
    );
    let state_lock = lock_update_state(&config.state_path)?;
    let mut state = UpdateState::load(&config.state_path);
    ensure!(
        matches!(
            apply_verified_manifest(
                &mut state,
                &config.current_version,
                &manifest,
                now_epoch_secs()
            )?,
            CheckOutcome::UpdateAvailable { .. }
        ),
        "manual upgrade required by this release's update_from floor"
    );
    state.save(&config.state_path)?;
    drop(state_lock);
    let target = format!("{}-linux-gnu", std::env::consts::ARCH);
    let release_target = manifest
        .targets
        .get(&target)
        .context("release does not support this architecture")?;
    check_glibc(&release_target.min_glibc)?;
    let artifact = &release_target.tarball;
    ensure!(
        artifact.size <= MAX_ARCHIVE,
        "tarball exceeds 256 MiB limit"
    );
    let stage = tempfile::Builder::new()
        .prefix(".rex-update-")
        .tempdir_in(layout.binary.parent().unwrap())?;
    let archive = stage.path().join("archive.tar.gz");
    fetch(artifact, &archive, progress)?;
    progress(Progress::Verifying);
    verify_archive(&archive, artifact)?;
    progress(Progress::Extracting);
    let unpack = stage.path().join("unpack");
    fs::create_dir(&unpack)?;
    let root = format!("rex-linux-{}-{target}", manifest.version);
    extract(&archive, &unpack, &root)?;
    prepare(layout, stage.path(), &unpack.join(root), &manifest.version)?;
    // Recheck immediately before changing the installation (another installer may have run).
    layout.validate_receipt(&config.current_version)?;
    progress(Progress::Installing);
    commit(layout, stage, |_| Ok(()))?;
    refresh_caches(layout);
    Ok(())
}

fn check_glibc(required: &str) -> Result<()> {
    let current = unsafe { std::ffi::CStr::from_ptr(libc::gnu_get_libc_version()) }.to_str()?;
    let parse = |version: &str| -> Result<(u32, u32)> {
        let (major, minor) = version.split_once('.').context("invalid glibc version")?;
        Ok((major.parse()?, minor.parse()?))
    };
    ensure!(
        parse(current)? >= parse(required)?,
        "this release requires glibc {required} or newer; this system has {current}. Upgrade your operating system before updating Rex"
    );
    Ok(())
}

fn download_error(error: impl Into<anyhow::Error>) -> anyhow::Error {
    let error = error.into();
    let timed_out = error.chain().any(|cause| {
        cause
            .downcast_ref::<ureq::Error>()
            .is_some_and(|e| matches!(e, ureq::Error::Timeout(_)))
            || cause.downcast_ref::<io::Error>().is_some_and(|e| {
                e.kind() == io::ErrorKind::TimedOut
                    || e.get_ref()
                        .and_then(|inner| inner.downcast_ref::<ureq::Error>())
                        .is_some_and(|inner| matches!(inner, ureq::Error::Timeout(_)))
            })
    });
    error.context(if timed_out {
        "tarball download timed out"
    } else {
        "tarball download failed"
    })
}

// Capture into a file rather than a pipe: a noisy child cannot block before the deadline.
fn binary_version(binary: &Path, timeout: Duration) -> Result<std::process::Output> {
    let mut stdout = tempfile::tempfile()?;
    let mut child = Command::new(binary)
        .arg("--version")
        .stdout(stdout.try_clone()?)
        .stderr(Stdio::null())
        .spawn()
        .context("new binary cannot run")?;
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("new binary --version timed out");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    use std::io::{Seek, SeekFrom};
    stdout.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    stdout.take(4096).read_to_end(&mut bytes)?;
    Ok(std::process::Output {
        status,
        stdout: bytes,
        stderr: Vec::new(),
    })
}

fn download(
    artifact: &TarballArtifact,
    path: &Path,
    progress: &mut dyn FnMut(Progress),
) -> Result<()> {
    let url = validate_fetch_url(&artifact.url, false)?;
    let _ = rustls::crypto::ring::default_provider().install_default();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .https_only(true)
        .max_redirects(5)
        .timeout_global(Some(Duration::from_secs(3600)))
        .timeout_per_read(Some(Duration::from_secs(30)))
        .timeout_connect(Some(Duration::from_secs(20)))
        .user_agent("rex-linux")
        .build()
        .into();
    let mut response = agent.get(url.as_str()).call().map_err(download_error)?;
    ensure!(
        response.status().as_u16() == 200,
        "unexpected tarball HTTP status"
    );
    copy_download(
        response.body_mut().as_reader(),
        path,
        artifact.size,
        progress,
    )
}

fn copy_download(
    mut reader: impl Read,
    path: &Path,
    size: u64,
    progress: &mut dyn FnMut(Progress),
) -> Result<()> {
    ensure!(size > 0 && size <= MAX_ARCHIVE, "invalid tarball size");
    let mut file = File::create(path)?;
    let mut received = 0;
    let mut last_percent = 0;
    let mut buf = [0; 64 * 1024];
    progress(Progress::Downloading {
        received,
        total: size,
    });
    loop {
        // Read at most one byte past the signed bound, solely to detect excess.
        let remaining = (size - received + 1).min(buf.len() as u64) as usize;
        let n = reader.read(&mut buf[..remaining]).map_err(download_error)?;
        if n == 0 {
            break;
        }
        received += n as u64;
        ensure!(received <= size, "download exceeds signed size");
        file.write_all(&buf[..n])?;
        let percent = received * 100 / size;
        if percent != last_percent {
            last_percent = percent;
            progress(Progress::Downloading {
                received,
                total: size,
            });
        }
    }
    ensure!(received == size, "download is shorter than signed size");
    file.sync_all()?;
    Ok(())
}

fn hash_file(path: &Path) -> Result<String> {
    let mut hash = Sha256::new();
    io::copy(&mut File::open(path)?, &mut hash)?;
    Ok(format!("{:x}", hash.finalize()))
}

fn verify_archive(path: &Path, artifact: &TarballArtifact) -> Result<()> {
    ensure!(
        fs::metadata(path)?.len() == artifact.size,
        "tarball size differs from signed manifest"
    );
    ensure!(
        hash_file(path)? == artifact.sha256,
        "tarball SHA-256 differs from signed manifest"
    );
    Ok(())
}

fn safe_path(bytes: &[u8], root: &str) -> Result<PathBuf> {
    let text = std::str::from_utf8(bytes).context("non-UTF-8 tar path")?;
    let text = text.trim_end_matches('/');
    ensure!(
        !text.starts_with('/') && text.split('/').all(|p| !matches!(p, "" | "." | "..")),
        "unsafe tar path"
    );
    ensure!(
        text == root || text.starts_with(&format!("{root}/")),
        "tar path outside package root"
    );
    Ok(PathBuf::from(text))
}

fn extract(archive: &Path, destination: &Path, root: &str) -> Result<()> {
    let gzip = flate2::read::GzDecoder::new(File::open(archive)?);
    let mut tar = tar::Archive::new(gzip.take(MAX_UNPACKED + 1));
    let mut total = 0u64;
    let mut seen = HashSet::new();
    for (index, entry) in tar.entries()?.enumerate() {
        ensure!(index < 20_000, "too many tar entries");
        let mut entry = entry?;
        let relative = safe_path(&entry.path_bytes(), root)?;
        ensure!(seen.insert(relative.clone()), "duplicate tar path");
        let kind = entry.header().entry_type();
        ensure!(
            kind.is_file() || kind.is_dir(),
            "tar links and special files are forbidden"
        );
        total = total
            .checked_add(entry.size())
            .context("tar size overflow")?;
        ensure!(total <= MAX_UNPACKED, "unpacked tar exceeds 1 GiB");
        let path = destination.join(relative);
        if kind.is_dir() {
            fs::create_dir_all(&path)?;
        } else {
            fs::create_dir_all(path.parent().unwrap())?;
            let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
            io::copy(&mut entry, &mut file)?;
        }
    }
    Ok(())
}

fn prepare(layout: &InstallLayout, stage: &Path, package: &Path, version: &str) -> Result<()> {
    let binary = package.join("bin/rex-linux");
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o755))?;
    // No package scripts are executed. Check the verified binary can run before replacing anything.
    let output = binary_version(&binary, Duration::from_secs(10))?;
    ensure!(
        output.status.success()
            && String::from_utf8_lossy(&output.stdout).trim() == format!("rex-linux {version}"),
        "new binary cannot run or reports the wrong version"
    );
    let mut sources: Vec<_> = [32, 128, 256]
        .map(|size| {
            package.join(format!(
                "resources/icons/{size}x{size}/apps/dev.shuv.rex-linux.png"
            ))
        })
        .into();
    let desktop_dir = stage.join("desktop");
    fs::create_dir(&desktop_dir)?;
    let path = layout
        .binary
        .to_str()
        .context("install path is not UTF-8")?;
    ensure!(
        !path.contains(['\n', '\r']),
        "desktop launcher path contains a newline"
    );
    // Same quoting and desktop-file-utils transformation as scripts/install-desktop.sh.
    let quoted = format!(
        "\"{}\"",
        path.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('`', "\\`")
            .replace('$', "\\$")
    );
    let status = Command::new("desktop-file-install")
        .arg(format!("--dir={}", desktop_dir.display()))
        .args([
            "--set-key=Exec",
            &format!("--set-value={quoted}"),
            "--set-key=TryExec",
            &format!("--set-value={path}"),
        ])
        .arg(package.join("resources").join(DESKTOP))
        .status()
        .context("desktop-file-install is required")?;
    ensure!(status.success(), "desktop-file-install failed");
    let desktop = desktop_dir.join(DESKTOP);
    ensure!(
        Command::new("desktop-file-validate")
            .arg(&desktop)
            .status()?
            .success(),
        "desktop launcher validation failed"
    );
    sources.push(desktop);
    sources.push(binary.clone());
    let receipt = stage.join("receipt.json");
    fs::write(
        &receipt,
        serde_json::to_vec(&InstallReceipt {
            schema: 1,
            path: fs::canonicalize(&layout.binary)?
                .to_string_lossy()
                .into_owned(),
            version: version.into(),
            sha256: hash_file(&binary)?,
        })?,
    )?;
    sources.push(receipt);
    for (i, source) in sources.iter().enumerate() {
        let path = stage.join(format!("new-{i}"));
        fs::copy(source, &path)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(INSTALL_FILES[i].1))?;
        File::open(path)?.sync_all()?;
    }
    Ok(())
}

fn sync_dir(path: &Path) -> Result<()> {
    File::open(path)?
        .sync_all()
        .context("sync install directory")
}

fn commit(
    layout: &InstallLayout,
    stage: tempfile::TempDir,
    before_replace: impl Fn(usize) -> Result<()>,
) -> Result<()> {
    let destinations = layout.destinations();
    let mut existed = Vec::new();
    for (i, dest) in destinations.iter().enumerate() {
        let exists = dest.try_exists()?;
        existed.push(exists);
        if exists {
            fs::hard_link(dest, stage.path().join(format!("old-{i}")))?;
        }
    }
    let journal = stage.path().join("journal.json");
    fs::write(
        &journal,
        serde_json::to_vec(&Journal {
            binary: layout.binary.clone(),
            data: layout.data.clone(),
            existed,
            new_inodes: (0..destinations.len())
                .map(|i| identity(&stage.path().join(format!("new-{i}"))))
                .collect::<io::Result<_>>()?,
        })?,
    )?;
    File::open(journal)?.sync_all()?;
    sync_dir(stage.path())?;
    fs::rename(stage.path(), layout.transaction())?;
    let result = (|| -> Result<()> {
        sync_dir(layout.binary.parent().unwrap())?;
        for (i, dest) in destinations.iter().enumerate() {
            before_replace(i)?;
            fs::rename(layout.transaction().join(format!("new-{i}")), dest)?;
            sync_dir(dest.parent().unwrap())?;
        }
        File::create(layout.transaction().join("committed"))?.sync_all()?;
        sync_dir(&layout.transaction())?;
        Ok(())
    })();
    if let Err(error) = result {
        // Retain the backups/journal if rollback itself fails, for next-launch recovery.
        (|| -> Result<()> {
            // Remove a failed commit marker durably before starting rollback.
            let marker = layout.transaction().join("committed");
            if marker.exists() {
                fs::remove_file(marker)?;
                sync_dir(&layout.transaction())?;
            }
            recover(layout).map(|_| ())
        })()
        .with_context(|| {
            format!(
                "{error:#}; rollback needs recovery at {}",
                layout.transaction().display()
            )
        })?;
        return Err(error.context("previous installation restored"));
    }
    // Once committed, cleanup failure is not an installation failure. Next launch retries it.
    let _ = cleanup_transaction(layout);
    Ok(())
}

// Retire the whole journal atomically before deleting any backup/marker. A kill
// during recursive deletion must never expose a half-deleted recovery journal.
fn cleanup_transaction(layout: &InstallLayout) -> Result<()> {
    let parent = layout.binary.parent().unwrap();
    let cleanup = tempfile::Builder::new()
        .prefix(".rex-update-cleanup-")
        .tempdir_in(parent)?;
    fs::rename(layout.transaction(), cleanup.path())?;
    let path = cleanup.keep();
    sync_dir(parent)?;
    let _ = fs::remove_dir_all(path);
    Ok(())
}

// Called only while holding the installation lock. Active stages use the same lock.
fn sweep_staging(layout: &InstallLayout) -> Result<()> {
    for entry in fs::read_dir(layout.binary.parent().unwrap())? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(".rex-update-")
            && name != ".rex-update-transaction"
            && entry.file_type()?.is_dir()
        {
            fs::remove_dir_all(entry.path())?;
        }
    }
    Ok(())
}

/// Report whether rollback restored the binary or a later install superseded it.
fn recover(layout: &InstallLayout) -> Result<Recovery> {
    recover_with_cleanup(layout, cleanup_transaction)
}

fn recover_with_cleanup(
    layout: &InstallLayout,
    cleanup: impl FnOnce(&InstallLayout) -> Result<()>,
) -> Result<Recovery> {
    let tx = layout.transaction();
    if !tx.exists() {
        return Ok(Recovery::Unchanged);
    }
    let mut restored_binary = false;
    if !tx.join("committed").exists() {
        let journal: Journal = serde_json::from_slice(&fs::read(tx.join("journal.json"))?)?;
        ensure!(
            journal.binary == layout.binary,
            "recovery journal belongs to a different binary"
        );
        let original = InstallLayout {
            data: journal.data,
            ..layout.clone()
        };
        let existed = journal.existed;
        let destinations = original.destinations();
        ensure!(
            existed.len() == destinations.len() && journal.new_inodes.len() == destinations.len(),
            "invalid or legacy update recovery journal; see docs/self-updates.md for manual recovery"
        );
        // Classify EVERY destination before touching any. A later installer/repair
        // supersedes the whole transaction, including files that were absent before.
        let mut replaced = Vec::new();
        for (i, dest) in destinations.iter().enumerate() {
            let current = match identity(dest) {
                Ok(id) => Some(id),
                Err(e) if e.kind() == io::ErrorKind::NotFound => None,
                Err(e) => return Err(e.into()),
            };
            let old = if existed[i] {
                Some(identity(&tx.join(format!("old-{i}")))?)
            } else {
                None
            };
            if current == old {
                replaced.push(false);
            } else if current == Some(journal.new_inodes[i]) {
                replaced.push(true);
            } else {
                cleanup(layout)?;
                return Ok(Recovery::Superseded);
            }
        }
        original.validate_paths()?;
        let mut failures = Vec::new();
        for (i, dest) in destinations.iter().enumerate().rev() {
            let restore_result = (|| -> Result<()> {
                if !replaced[i] {
                    return Ok(());
                }
                if existed[i] {
                    let backup = tx.join(format!("old-{i}"));
                    let restore = tx.join(format!("restore-{i}"));
                    if restore.exists() {
                        fs::remove_file(&restore)?;
                    }
                    fs::hard_link(backup, &restore)?;
                    fs::rename(restore, dest)?;
                } else {
                    fs::remove_file(dest)?;
                }
                if dest == &layout.binary {
                    restored_binary = true;
                }
                sync_dir(dest.parent().unwrap())
            })();
            if let Err(error) = restore_result {
                failures.push(format!("{}: {error:#}", dest.display()));
            }
        }
        ensure!(
            failures.is_empty(),
            "could not restore all install files: {}",
            failures.join("; ")
        );
        // Mark completed rollback before cleanup, making interrupted cleanup safe too.
        File::create(tx.join("committed"))?.sync_all()?;
        sync_dir(&tx)?;
    }
    // A completed rollback still requires re-exec even if retiring the journal
    // fails. Its durable marker makes a later cleanup safe.
    let _ = cleanup(layout);
    Ok(if restored_binary {
        Recovery::Restored(layout.binary.clone())
    } else {
        Recovery::Unchanged
    })
}

fn refresh_caches(layout: &InstallLayout) {
    // Derived caches are best effort, as they are not part of the installed payload.
    let _ = Command::new("update-desktop-database")
        .arg(layout.data.join("applications"))
        .status();
    let _ = Command::new("gtk-update-icon-cache")
        .args(["-f", "-t"])
        .arg(layout.data.join("icons/hicolor"))
        .status();
}

#[cfg(test)]
mod tests;
