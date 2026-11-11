//! Whether a finished download may be offered an "Open" action. Nothing here opens or runs
//! anything; it only answers the question, in one place.
use std::{io::Read, os::unix::fs::OpenOptionsExt, path::Path};

/// Extensions of files that can run or script something when opened, compared without case. An
/// entry with a dot inside matches a longer ending (`pkg.tar.zst`).
const RISKY_EXTENSIONS: &[&str] = &[
    // Launchers and scripts.
    "desktop",
    "directory",
    "sh",
    "bash",
    "zsh",
    "fish",
    "csh",
    "ksh",
    "command",
    "run",
    "py",
    "pyw",
    "pl",
    "rb",
    "php",
    "lua",
    "js",
    "mjs",
    "cjs",
    "vbs",
    "vbe",
    "wsf",
    "ps1",
    "psm1",
    "bat",
    "cmd",
    "reg",
    "hta",
    "jnlp",
    // Programs and installers.
    "jar",
    "appimage",
    "deb",
    "rpm",
    "pkg.tar.zst",
    "pkg.tar.xz",
    "pkg.tar.gz",
    "flatpak",
    "flatpakref",
    "flatpakrepo",
    "snap",
    "apk",
    "msi",
    "exe",
    "com",
    "scr",
    "dll",
    "so",
    "dmg",
    "pkg",
    "swf",
    "lnk",
    // Documents that run script in a browser.
    "html",
    "htm",
    "xhtml",
    "shtml",
    "svg",
    "svgz",
    "mht",
    "mhtml",
];

/// How many bytes of a file's start decide [`risky_content`].
pub const SNIFF_BYTES: usize = 4;

/// Whether the name alone rules out opening.
pub fn risky_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    RISKY_EXTENSIONS.iter().any(|extension| {
        lower
            .strip_suffix(extension)
            .is_some_and(|rest| rest.ends_with('.') && rest.len() > 1)
    })
}

/// Whether the first bytes of the file are an ELF program or a `#!` script.
pub fn risky_content(head: &[u8]) -> bool {
    head.starts_with(b"\x7fELF") || head.starts_with(b"#!")
}

/// Whether the app may offer to open this finished download: its name and its first bytes are
/// both unremarkable. A file that can't be read is not offered.
pub fn may_open(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if risky_name(name) {
        return false;
    }
    let Ok(mut file) = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
    else {
        return false;
    };
    let mut head = [0u8; SNIFF_BYTES];
    let mut filled = 0;
    while filled < head.len() {
        match file.read(&mut head[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(_) => return false,
        }
    }
    !risky_content(&head[..filled])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_deny_list_by_name() {
        for risky in [
            "launcher.desktop",
            "run.sh",
            "RUN.SH",
            "tool.py",
            "app.jar",
            "App-1.2.AppImage",
            "thing.appimage",
            "pkg.deb",
            "pkg.rpm",
            "foo-1.0-1-x86_64.pkg.tar.zst",
            "page.html",
            "page.HTM",
            "image.svg",
            "setup.exe",
            "x.bat",
            "x.ps1",
            "weird.name.sh",
            "a.tar.sh",
        ] {
            assert!(risky_name(risky), "{risky}");
        }
        for fine in [
            "notes.txt",
            "photo.png",
            "doc.pdf",
            "archive.tar.gz",
            "data.json",
            "build.log",
            "sh",
            ".sh",
            "html",
            "script.shx",
            "ashes",
            "x.pkg.tar.bz2",
            "README",
            "movie.mkv",
        ] {
            assert!(!risky_name(fine), "{fine}");
        }
    }

    #[test]
    fn the_deny_list_by_content() {
        assert!(risky_content(b"\x7fELF\x02\x01"));
        assert!(risky_content(b"#!/bin/sh\n"));
        assert!(risky_content(b"#!"));
        assert!(!risky_content(b"# comment\n"));
        assert!(!risky_content(b"PK\x03\x04"));
        assert!(!risky_content(b"\x7fEL"));
        assert!(!risky_content(b""));
    }

    #[test]
    fn may_open_checks_the_name_then_the_file() {
        let dir = std::env::temp_dir().join(format!("rex-open-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = |name: &str, bytes: &[u8]| {
            let path = dir.join(name);
            std::fs::write(&path, bytes).unwrap();
            path
        };
        assert!(may_open(&file("notes.txt", b"hello")));
        assert!(may_open(&file("empty", b"")));
        assert!(!may_open(&file("run.sh", b"echo")));
        assert!(!may_open(&file("innocent.txt", b"#!/bin/sh\nrm -rf ~\n")));
        assert!(!may_open(&file("program", b"\x7fELF\x02\x01\x01")));
        assert!(!may_open(&dir.join("missing.txt")));
        let link = dir.join("link.txt");
        std::os::unix::fs::symlink(dir.join("notes.txt"), &link).unwrap();
        assert!(!may_open(&link), "a symlink is never followed");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
