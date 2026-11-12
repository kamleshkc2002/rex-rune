//! Live check of downloads against a disposable Rex server (API revision 13): creates one
//! session there, downloads real files through the same host connection the app uses, and
//! compares their SHA-256 sums. Never point it at a server whose sessions matter.
//!
//! ```text
//! rex-download-probe <endpoint> <work-dir> [server-pid]
//! ```
//!
//! `work-dir` receives the source files, the destination folders and the records file. With a
//! `server-pid` the probe also checks that the server closes a cancelled file. Exit 0 when every
//! check passed.
use anyhow::{Context, Result, bail, ensure};
use rex_rune::{
    connection::{HostCommand, HostConnection, HostModel, downloads::Request},
    download::{Config, Failure, Limits, Reason, Records, State, next_id},
};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

const MIB: u64 = 1024 * 1024;

fn main() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(run())
}

fn sha256(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 1 << 20];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// A file of `len` pseudo-random bytes.
fn random_file(path: &Path, len: u64) -> Result<()> {
    let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut state = 0x9e37_79b9_7f4a_7c15u64 ^ len;
    let mut left = len;
    let mut block = [0u8; 8192];
    while left > 0 {
        for chunk in block.chunks_mut(8) {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            chunk.copy_from_slice(&state.to_le_bytes());
        }
        let n = left.min(block.len() as u64) as usize;
        out.write_all(&block[..n])?;
        left -= n as u64;
    }
    out.flush()?;
    Ok(())
}

struct Probe {
    host: HostConnection,
    session: String,
    block: String,
    work: PathBuf,
    server_pid: Option<u32>,
    failed: Vec<String>,
}

impl Probe {
    async fn wait<T>(&self, what: &str, mut get: impl FnMut(&HostModel) -> Option<T>) -> Result<T> {
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            if let Some(found) = get(&self.host.model.lock().unwrap()) {
                return Ok(found);
            }
            ensure!(Instant::now() < deadline, "timed out waiting for {what}");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    fn start(&self, path: &Path, dest: &Path) -> u64 {
        self.start_with(path, dest, |_| {})
    }

    fn start_with(&self, path: &Path, dest: &Path, configure: impl FnOnce(&mut Config)) -> u64 {
        let id = next_id();
        let mut request =
            Request::new(id, &self.session, &self.block, path.to_str().unwrap(), dest);
        request.config = Config {
            records: Records::at(self.work.join("state/downloads.json")),
            ..Config::default()
        };
        configure(&mut request.config);
        self.host
            .commands
            .send(HostCommand::Download(Box::new(request)))
            .unwrap();
        id
    }

    async fn finish(&self, id: u64) -> Result<State> {
        self.wait("the transfer to end", |m| {
            m.downloads
                .iter()
                .find(|t| t.id == id)
                .filter(|t| t.state.is_final())
                .map(|t| t.state.clone())
        })
        .await
    }

    fn report(&mut self, name: &str, result: Result<String>) {
        match result {
            Ok(detail) => println!("PASS {name}: {detail}"),
            Err(error) => {
                println!("FAIL {name}: {error:#}");
                self.failed.push(name.into());
            }
        }
    }

    /// Downloads `source` into a fresh folder and checks the saved file matches it.
    async fn fetch(&self, label: &str, source: &Path, expect_name: &str) -> Result<String> {
        let dest = self.work.join(format!("dest-{label}"));
        std::fs::create_dir_all(&dest)?;
        let started = Instant::now();
        let id = self.start(source, &dest);
        let State::Saved { path, bytes, .. } = self.finish(id).await? else {
            bail!("did not save: {:?}", self.state(id));
        };
        let elapsed = started.elapsed();
        ensure!(
            path == dest.join(expect_name),
            "saved as {path:?}, expected {expect_name:?}"
        );
        let (want, got) = (sha256(source)?, sha256(&path)?);
        ensure!(want == got, "sha256 differs: {want} vs {got}");
        ensure!(
            bytes == std::fs::metadata(source)?.len(),
            "reported {bytes} bytes"
        );
        let mode =
            std::os::unix::fs::PermissionsExt::mode(&std::fs::metadata(&path)?.permissions());
        ensure!(mode & 0o111 == 0, "saved with an executable bit: {mode:o}");
        let left: Vec<_> = std::fs::read_dir(&dest)?
            .map(|e| e.unwrap().file_name())
            .collect();
        ensure!(left.len() == 1, "extra files: {left:?}");
        let rate = bytes as f64 / MIB as f64 / elapsed.as_secs_f64().max(1e-6);
        Ok(format!(
            "{bytes} bytes, sha256 {}, {rate:.0} MiB/s",
            &got[..16]
        ))
    }

    fn state(&self, id: u64) -> Option<State> {
        self.host
            .model
            .lock()
            .unwrap()
            .downloads
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.state.clone())
    }

    fn open_files(&self, name: &str) -> Option<usize> {
        let pid = self.server_pid?;
        let mut count = 0;
        for entry in std::fs::read_dir(format!("/proc/{pid}/fd")).ok()? {
            if let Ok(target) = std::fs::read_link(entry.ok()?.path())
                && target.to_string_lossy().contains(name)
            {
                count += 1;
            }
        }
        Some(count)
    }
}

async fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let endpoint = args
        .next()
        .context("usage: rex-download-probe <endpoint> <work-dir> [server-pid]")?;
    let work = PathBuf::from(args.next().context("work directory")?);
    let server_pid = args.next().map(|p| p.parse()).transpose()?;
    ensure!(
        endpoint.starts_with("unix://") && !endpoint.contains("/run/user/"),
        "refusing {endpoint}: pass the socket of a disposable server you started"
    );
    let work = std::fs::canonicalize({
        std::fs::create_dir_all(&work)?;
        work
    })?;
    let source = work.join("src");
    std::fs::create_dir_all(&source)?;

    let host = HostConnection::spawn(
        endpoint,
        None,
        Some(("rex-download-probe".into(), None)),
        None,
        false,
        None,
    );
    let started = Instant::now();
    let (session, block) = loop {
        if let Some(found) = {
            let m = host.model.lock().unwrap();
            m.selected
                .clone()
                .zip(m.focused().map(str::to_owned))
                .filter(|_| m.connected)
        } {
            break found;
        }
        ensure!(
            started.elapsed() < Duration::from_secs(15),
            "no terminal on the server"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    let revision = host.model.lock().unwrap().protocol.api_revision;
    println!("connected, API revision {revision}, session {session}");
    let mut probe = Probe {
        host,
        session,
        block,
        work: work.clone(),
        server_pid,
        failed: Vec::new(),
    };

    if revision < 13 {
        // An older server: the request is never sent, and the transfer says why.
        std::fs::write(source.join("small.txt"), "hello, download\n")?;
        let dest = work.join("dest-old");
        std::fs::create_dir_all(&dest)?;
        let id = probe.start(&source.join("small.txt"), &dest);
        let state = probe.finish(id).await?;
        ensure!(state == State::Failed(Failure::ServerTooOld), "{state:?}");
        println!(
            "PASS old server (revision {revision}): {}",
            Failure::ServerTooOld
        );
        return Ok(());
    }

    // Fixtures.
    std::fs::write(source.join("small.txt"), "hello, download\n")?;
    std::fs::write(source.join("empty.bin"), "")?;
    std::fs::write(
        source.join("name with spaces & ünïcode 界 🚀.txt"),
        "unicode name\n",
    )?;
    std::fs::write(
        source.join("two\nlines\t.txt"),
        "control characters in the name\n",
    )?;
    std::fs::write(source.join("-leading-dash.txt"), "dash\n")?;
    std::fs::write(source.join(".hidden"), "dot\n")?;
    random_file(&source.join("big-64MiB.bin"), 64 * MIB)?;
    {
        // A sparse 1.5 GiB file: its hint needs the confirmation, and it can be read fast.
        let file = std::fs::File::create(source.join("sparse-1.5GiB.img"))?;
        file.set_len(1536 * MIB)?;
    }

    for (label, file, saved) in [
        ("small", "small.txt", "small.txt"),
        ("empty", "empty.bin", "empty.bin"),
        (
            "unicode",
            "name with spaces & ünïcode 界 🚀.txt",
            "name with spaces & ünïcode 界 🚀.txt",
        ),
        ("newline", "two\nlines\t.txt", "twolines.txt"),
        ("dash", "-leading-dash.txt", "-leading-dash.txt"),
        ("dot", ".hidden", "_hidden"),
        ("big", "big-64MiB.bin", "big-64MiB.bin"),
    ] {
        let result = probe.fetch(label, &source.join(file), saved).await;
        probe.report(label, result);
    }

    // The same file twice: the second never replaces the first.
    let result = async {
        let dest = probe.work.join("dest-twice");
        std::fs::create_dir_all(&dest)?;
        for _ in 0..2 {
            let id = probe.start(&source.join("small.txt"), &dest);
            let State::Saved { .. } = probe.finish(id).await? else {
                bail!("not saved")
            };
        }
        let mut names: Vec<_> = std::fs::read_dir(&dest)?
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        ensure!(names == ["small (1).txt", "small.txt"], "{names:?}");
        Ok(format!("{names:?}"))
    }
    .await;
    probe.report("same file twice", result);

    // Missing file and a folder: refused in Rex's words, before any bytes.
    for (label, path, wanted) in [
        ("missing file", source.join("does-not-exist"), "not found"),
        ("a folder", source.clone(), "not a regular file"),
    ] {
        let result = async {
            let dest = probe.work.join("dest-refused");
            std::fs::create_dir_all(&dest)?;
            let id = probe.start(&path, &dest);
            let State::Failed(Failure::Server(message)) = probe.finish(id).await? else {
                bail!("not refused by Rex: {:?}", probe.state(id));
            };
            ensure!(message.contains(wanted), "{message}");
            ensure!(
                std::fs::read_dir(&dest)?.next().is_none(),
                "something was written"
            );
            Ok(message)
        }
        .await;
        probe.report(label, result);
    }

    // A symlinked destination is refused locally.
    let result = async {
        let real = probe.work.join("dest-real");
        std::fs::create_dir_all(&real)?;
        let link = probe.work.join("dest-link");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(&real, &link)?;
        let id = probe.start(&source.join("small.txt"), &link);
        let State::Failed(Failure::Destination(message)) = probe.finish(id).await? else {
            bail!("not refused: {:?}", probe.state(id));
        };
        ensure!(
            std::fs::read_dir(&real)?.next().is_none(),
            "something was written"
        );
        Ok(message)
    }
    .await;
    probe.report("symlinked destination", result);

    // Cancel halfway through a large file: confirm first (it is over 1 GiB), then cancel.
    let result = async {
        let dest = probe.work.join("dest-cancel");
        std::fs::create_dir_all(&dest)?;
        let id = probe.start(&source.join("sparse-1.5GiB.img"), &dest);
        probe
            .wait("the confirmation", |m| {
                m.downloads.iter().find(|t| t.id == id).filter(|t| matches!(t.state, State::AwaitingConfirmation(_))).map(|_| ())
            })
            .await?;
        probe.host.commands.send(HostCommand::DownloadConfirm(id)).unwrap();
        probe
            .wait("bytes", |m| {
                m.downloads
                    .iter()
                    .find(|t| t.id == id)
                    .filter(|t| matches!(t.state, State::Receiving { received, .. } if received >= 8 * MIB))
                    .map(|_| ())
            })
            .await?;
        probe.host.commands.send(HostCommand::DownloadCancel(id)).unwrap();
        let state = probe.finish(id).await?;
        ensure!(state == State::Cancelled, "{state:?}");
        let left: Vec<_> = std::fs::read_dir(&dest)?.map(|e| e.unwrap().file_name()).collect();
        ensure!(left.is_empty(), "left behind: {left:?}");
        let mut detail = "cancelled, partial file deleted".to_owned();
        if probe.server_pid.is_some() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let open = probe.open_files("sparse-1.5GiB.img").unwrap_or(usize::MAX);
            ensure!(open == 0, "the server still has the file open ({open})");
            detail.push_str(", server closed the file");
        }
        ensure!(Records::at(probe.work.join("state/downloads.json")).list().is_empty(), "record left");
        Ok(detail)
    }
    .await;
    probe.report("cancel halfway", result);

    // Cancelling at the question must free Rex's reservation at once: a terminal allows 16
    // reserved files, so 17 cancels in a row and then a normal download only work if each
    // cancel gave its slot back.
    let result = async {
        let dest = probe.work.join("dest-questions");
        std::fs::create_dir_all(&dest)?;
        for round in 1..=17 {
            let id = probe.start(&source.join("sparse-1.5GiB.img"), &dest);
            probe
                .wait("the confirmation", |m| {
                    m.downloads
                        .iter()
                        .find(|t| t.id == id)
                        .filter(|t| matches!(t.state, State::AwaitingConfirmation(_)))
                        .map(|_| ())
                })
                .await?;
            probe
                .host
                .commands
                .send(HostCommand::DownloadCancel(id))
                .unwrap();
            let state = probe.finish(id).await?;
            ensure!(state == State::Cancelled, "round {round}: {state:?}");
        }
        let mut detail = "17 questions cancelled".to_owned();
        if probe.server_pid.is_some() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let open = probe.open_files("sparse-1.5GiB.img").unwrap_or(usize::MAX);
            ensure!(
                open == 0,
                "the server still has the file open ({open} descriptors)"
            );
            detail.push_str(", server holds no descriptor");
        }
        let after = probe
            .fetch("after-questions", &source.join("small.txt"), "small.txt")
            .await?;
        detail.push_str(&format!(", then: {after}"));
        ensure!(
            std::fs::read_dir(&dest)?.next().is_none(),
            "something was written"
        );
        Ok(detail)
    }
    .await;
    probe.report("cancel at the question, 17 times", result);

    // Free space is injected (the probe cannot fill a disk): an 8 MiB reserve and a 1 MiB floor,
    // and a destination that reports `looks` in turn (when the transfer is admitted, when the
    // question is answered, then while it runs); the last one repeats.
    let low_space = |looks: &'static [u64]| {
        move |config: &mut Config| {
            config.limits = Limits {
                reserve: 8 * MIB,
                low_floor: MIB,
                ..Limits::default()
            };
            let seen = AtomicU64::new(0);
            config.space = Some(Arc::new(move || -> std::io::Result<u64> {
                let look = seen.fetch_add(1, Ordering::Relaxed) as usize;
                Ok(looks[look.min(looks.len() - 1)])
            }));
        }
    };

    // The 64 MiB file does not fit in 32 MiB free: refused at once, no question, and the
    // reservation is freed on the server.
    let result = async {
        let dest = probe.work.join("dest-low-refused");
        std::fs::create_dir_all(&dest)?;
        let id = probe.start_with(&source.join("big-64MiB.bin"), &dest, low_space(&[32 * MIB]));
        let state = probe.finish(id).await?;
        ensure!(
            state == State::Failed(Failure::LowSpace { free: 32 * MIB }),
            "{state:?}"
        );
        ensure!(
            std::fs::read_dir(&dest)?.next().is_none(),
            "something was written"
        );
        let mut detail = "refused without a question".to_owned();
        if probe.server_pid.is_some() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let open = probe.open_files("big-64MiB.bin").unwrap_or(usize::MAX);
            ensure!(open == 0, "the server still has the file open ({open})");
            detail.push_str(", server holds no descriptor");
        }
        Ok(detail)
    }
    .await;
    probe.report("low space: bigger than the free space", result);

    // 65 MiB free, an 8 MiB reserve and a 1 MiB floor: it fits but eats the reserve. It is asked
    // about, and after a yes it is saved whole although the destination then reports less than
    // the reserve (2 MiB free, over the floor).
    let result = async {
        let dest = probe.work.join("dest-low-yes");
        std::fs::create_dir_all(&dest)?;
        let id = probe.start_with(
            &source.join("big-64MiB.bin"),
            &dest,
            low_space(&[65 * MIB, 65 * MIB, 2 * MIB]),
        );
        let State::AwaitingConfirmation(question) = probe
            .wait("the question", |m| {
                m.downloads
                    .iter()
                    .find(|t| t.id == id)
                    .filter(|t| matches!(t.state, State::AwaitingConfirmation(_)))
                    .map(|t| t.state.clone())
            })
            .await?
        else {
            unreachable!()
        };
        ensure!(
            question.reason == Reason::LowSpace
                && question.hint == 64 * MIB
                && question.free == Some(65 * MIB),
            "{question:?}"
        );
        probe
            .host
            .commands
            .send(HostCommand::DownloadConfirm(id))
            .unwrap();
        let State::Saved { path, bytes, .. } = probe.finish(id).await? else {
            bail!("not saved: {:?}", probe.state(id));
        };
        let (want, got) = (sha256(&source.join("big-64MiB.bin"))?, sha256(&path)?);
        ensure!(
            want == got && bytes == 64 * MIB,
            "{bytes} bytes, sha256 {got}"
        );
        Ok(format!(
            "asked (LowSpace, 64 MiB of 65 MiB free), yes, saved {bytes} bytes intact"
        ))
    }
    .await;
    probe.report("low space: a yes uses the reserve", result);

    // 64.5 MiB free: the 64 MiB file fits, but not with the 1 MiB floor left over, so even a yes
    // could not help: refused at once, no question.
    let result = async {
        let dest = probe.work.join("dest-low-floor");
        std::fs::create_dir_all(&dest)?;
        let free = 64 * MIB + MIB / 2;
        let id = probe.start_with(
            &source.join("big-64MiB.bin"),
            &dest,
            low_space(&[64 * MIB + MIB / 2]),
        );
        let state = probe.finish(id).await?;
        ensure!(
            state == State::Failed(Failure::LowSpace { free }),
            "{state:?}"
        );
        ensure!(
            std::fs::read_dir(&dest)?.next().is_none(),
            "something was written"
        );
        Ok("refused: no room for the floor".to_owned())
    }
    .await;
    probe.report("low space: the floor must fit too", result);

    // Asked with 65 MiB free; by the yes the disk is down to 40 MiB: the yes is checked again, the
    // transfer fails and the reservation is freed.
    let result = async {
        let dest = probe.work.join("dest-low-stale");
        std::fs::create_dir_all(&dest)?;
        let id = probe.start_with(
            &source.join("big-64MiB.bin"),
            &dest,
            low_space(&[65 * MIB, 40 * MIB]),
        );
        probe
            .wait("the question", |m| {
                m.downloads
                    .iter()
                    .find(|t| t.id == id)
                    .filter(|t| matches!(t.state, State::AwaitingConfirmation(_)))
                    .map(|_| ())
            })
            .await?;
        probe
            .host
            .commands
            .send(HostCommand::DownloadConfirm(id))
            .unwrap();
        let state = probe.finish(id).await?;
        ensure!(
            state == State::Failed(Failure::LowSpace { free: 40 * MIB }),
            "{state:?}"
        );
        ensure!(
            std::fs::read_dir(&dest)?.next().is_none(),
            "something was written"
        );
        let mut detail = "the stale yes was refused".to_owned();
        if probe.server_pid.is_some() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let open = probe.open_files("big-64MiB.bin").unwrap_or(usize::MAX);
            ensure!(open == 0, "the server still has the file open ({open})");
            detail.push_str(", server holds no descriptor");
        }
        Ok(detail)
    }
    .await;
    probe.report("low space: a yes that waited is checked again", result);

    // A file whose reported size is wrong: Rex ends the stream at its end, so it is saved and
    // says it was short (sysfs reports 4096 for a few bytes).
    let sysfs = Path::new("/sys/class/net/lo/mtu");
    if sysfs.exists() {
        let result = async {
            let dest = probe.work.join("dest-short");
            std::fs::create_dir_all(&dest)?;
            let id = probe.start(sysfs, &dest);
            let State::Saved {
                path,
                bytes,
                short_of,
            } = probe.finish(id).await?
            else {
                bail!("not saved: {:?}", probe.state(id));
            };
            ensure!(
                std::fs::read(&path)? == std::fs::read(sysfs)?,
                "content differs"
            );
            ensure!(
                short_of.is_some_and(|hint| hint > bytes),
                "short_of {short_of:?}, {bytes} bytes"
            );
            Ok(format!("{bytes} bytes saved, Rex reported {short_of:?}"))
        }
        .await;
        probe.report("file shorter than its size", result);
    }

    if probe.failed.is_empty() {
        println!("all checks passed");
        Ok(())
    } else {
        bail!("failed: {}", probe.failed.join(", "))
    }
}
