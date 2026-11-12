//! Local executable/theme discovery and bounded child lifetime. No shell or server text.
use super::{Health, Sound};
use std::{
    ffi::OsString,
    io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const LIMIT: Duration = Duration::from_secs(3);

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Invocation {
    pub program: PathBuf,
    pub args: Vec<OsString>,
}

pub(super) trait Files {
    fn executable(&self, path: &Path) -> bool;
    fn file(&self, path: &Path) -> bool;
}

/// XDG defaults and order, ignoring relative directories as required by the XDG spec.
fn data_dirs(
    home: Option<PathBuf>,
    data_home: Option<PathBuf>,
    dirs: Option<OsString>,
) -> Vec<PathBuf> {
    data_home
        .filter(|p| p.is_absolute())
        .or_else(|| {
            home.filter(|p| p.is_absolute())
                .map(|p| p.join(".local/share"))
        })
        .into_iter()
        .chain(
            std::env::split_paths(
                &dirs
                    .filter(|p| !p.is_empty())
                    .unwrap_or_else(|| "/usr/local/share:/usr/share".into()),
            )
            .filter(|p| p.is_absolute()),
        )
        .collect()
}

/// The directories of `PATH` a player may be run from. A relative entry (`.`, an empty
/// one, `bin`) means "wherever the app was started", which could be a folder holding
/// someone else's `pw-play`: those are never searched.
fn search_path(path: Option<OsString>) -> Vec<PathBuf> {
    std::env::split_paths(&path.unwrap_or_default())
        .filter(|p| p.is_absolute())
        .collect()
}

/// PATH is local launch configuration. Resolve once per attempt to an absolute path;
/// never use a shell, a title, status message, program name or remote environment.
pub(super) fn select(
    files: &impl Files,
    paths: &[PathBuf],
    data: &[PathBuf],
    sound: Sound,
) -> Option<Invocation> {
    for name in ["canberra-gtk-play", "pw-play", "paplay"] {
        let Some(program) = paths
            .iter()
            .map(|p| p.join(name))
            .find(|p| files.executable(p))
        else {
            continue;
        };
        let args = if name == "canberra-gtk-play" {
            vec!["-i".into(), sound.id().into()]
        } else {
            let file = data
                .iter()
                .map(|p| {
                    p.join("sounds/freedesktop/stereo")
                        .join(format!("{}.oga", sound.id()))
                })
                .find(|p| files.file(p))?;
            vec![file.into_os_string()]
        };
        return Some(Invocation { program, args });
    }
    None
}

pub(super) trait Running {
    fn poll(&mut self) -> io::Result<Option<bool>>;
    fn kill(&mut self);
    fn reap(&mut self);
}
pub(super) trait Player {
    type Child: Running;
    fn find(&mut self, sound: Sound) -> Option<Invocation>;
    fn spawn(&mut self, invocation: &Invocation) -> io::Result<Self::Child>;
    fn now(&self) -> Duration;
    fn pause(&mut self);
}

pub(super) fn attempt(
    player: &mut impl Player,
    sound: Option<Sound>,
    stopped: impl Fn() -> bool,
) -> Health {
    let Some(invocation) = player.find(sound.unwrap_or(Sound::Done)) else {
        return Health::Missing;
    };
    if sound.is_none() || stopped() {
        return Health::Available;
    }
    let Ok(mut child) = player.spawn(&invocation) else {
        return Health::Failed;
    };
    let start = player.now();
    let result = loop {
        if stopped() || player.now().saturating_sub(start) >= LIMIT {
            child.kill();
            break Health::TimedOut;
        }
        match child.poll() {
            Ok(Some(true)) => break Health::Available,
            Ok(Some(false)) => break Health::Failed,
            Err(_) => {
                child.kill();
                break Health::Failed;
            }
            Ok(None) => player.pause(),
        }
    };
    child.reap();
    result
}

pub(super) struct System;
impl Files for System {
    fn executable(&self, path: &Path) -> bool {
        path.metadata()
            .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    }
    fn file(&self, path: &Path) -> bool {
        path.is_file()
    }
}
impl Player for System {
    type Child = Child;
    fn find(&mut self, sound: Sound) -> Option<Invocation> {
        let paths = search_path(std::env::var_os("PATH"));
        let data = data_dirs(
            std::env::var_os("HOME").map(PathBuf::from),
            std::env::var_os("XDG_DATA_HOME").map(PathBuf::from),
            std::env::var_os("XDG_DATA_DIRS"),
        );
        select(self, &paths, &data, sound)
    }
    fn spawn(&mut self, invocation: &Invocation) -> io::Result<Child> {
        Command::new(&invocation.program)
            .args(&invocation.args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    }
    fn now(&self) -> Duration {
        static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        START.get_or_init(Instant::now).elapsed()
    }
    fn pause(&mut self) {
        std::thread::sleep(Duration::from_millis(20));
    }
}
impl Running for Child {
    fn poll(&mut self) -> io::Result<Option<bool>> {
        self.try_wait().map(|s| s.map(|s| s.success()))
    }
    fn kill(&mut self) {
        let _ = Child::kill(self);
    }
    fn reap(&mut self) {
        let _ = self.wait();
    }
}

#[cfg(test)]
pub(super) mod tests;
