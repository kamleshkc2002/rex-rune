//! Handing a link, a folder or a file to the desktop: `xdg-open` and the file manager's
//! D-Bus call on Linux, `open` on macOS.
use std::{
    ffi::OsStr,
    path::Path,
    process::{Command, Stdio},
};

/// The program that opens a link or a path with the user's own handler for it.
pub const OPENER: &str = if cfg!(target_os = "macos") {
    "/usr/bin/open"
} else {
    "xdg-open"
};

/// The opener's call for one target, which is its only argument, with nothing inherited.
pub fn command(target: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(OPENER);
    command
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// The opener's call for one absolute path. `None` for a relative path, which the opener
/// could read as an option or an URL.
pub fn path_command(target: &Path) -> Option<Command> {
    target.is_absolute().then(|| command(target))
}

/// Run a prepared opener call and wait for it. A failure is its text.
#[cfg_attr(test, allow(dead_code))]
pub fn run(mut command: Command) -> Result<(), String> {
    match command.status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(status.to_string()),
        Err(error) => Err(error.to_string()),
    }
}

/// Open the file manager on `path`'s folder with the file selected, or failing that on the
/// folder alone. `path` is absolute.
#[cfg_attr(test, allow(dead_code))]
pub fn reveal(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("not an absolute path".into());
    }
    // Finder's own reveal, or the file manager's own call: each selects the file.
    #[cfg(target_os = "macos")]
    let selected = {
        let mut reveal = command("-R");
        reveal.arg(path);
        run(reveal)
    };
    #[cfg(not(target_os = "macos"))]
    let selected = show_items(path);
    // Failing that, its folder.
    selected.or_else(|_| {
        let folder = path.parent().unwrap_or(Path::new("/"));
        run(path_command(folder).ok_or("not an absolute path")?)
    })
}

/// `org.freedesktop.FileManager1.ShowItems`: the file manager opens the folder with the file
/// selected.
#[cfg(not(target_os = "macos"))]
#[cfg_attr(test, allow(dead_code))]
fn show_items(path: &Path) -> Result<(), String> {
    let uri = url::Url::from_file_path(path).map_err(|()| "not an absolute path".to_owned())?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    runtime.block_on(async {
        const NAME: &str = "org.freedesktop.FileManager1";
        let call = async {
            let conn = zbus::Connection::session().await?;
            let proxy =
                zbus::Proxy::new(&conn, NAME, "/org/freedesktop/FileManager1", NAME).await?;
            proxy
                .call::<_, _, ()>("ShowItems", &(vec![uri.to_string()], ""))
                .await
        };
        match tokio::time::timeout(std::time::Duration::from_secs(5), call).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.to_string()),
            Err(_) => Err("the file manager did not answer".into()),
        }
    })
}
