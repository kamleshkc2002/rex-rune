//! Whether a finished download may be offered an "Open" action. Nothing here opens or runs
//! anything; it only answers the question, in one place.
use std::{io::Read, os::unix::fs::OpenOptionsExt, path::Path};

/// Extensions of files that can run or script something when opened, compared without case (in
/// Unicode: `.des\u{212a}top` with a Kelvin sign is `.desktop` to the system's glob matching).
/// An entry with a dot inside matches a longer ending (`pkg.tar.zst`).
const RISKY_EXTENSIONS: &[&str] = &[
    // Launchers and scripts.
    "desktop",
    "kdelnk",
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
    "py2",
    "py3",
    "pyw",
    "pyi",
    "pyz",
    "pyzw",
    "pyc",
    "pyo",
    "pl",
    "pm",
    "perl",
    "rb",
    "php",
    "lua",
    "js",
    "mjs",
    "cjs",
    "vbs",
    "vbe",
    "jse",
    "wsf",
    "wsc",
    "sct",
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
    "xht",
    "xml",
    "xsl",
    "xslt",
    "shtml",
    "svg",
    "svgz",
    "mht",
    "mhtml",
];

/// How many bytes of a file's start decide [`risky_content`].
pub const SNIFF_BYTES: usize = 512;

/// Text that makes a file a web page, an SVG or XML document, a desktop launcher or a script
/// wherever the system's type detection (shared-mime-info) looks for it, whatever the file is
/// called. Compared without case, anywhere in the first [`SNIFF_BYTES`] bytes (the database looks
/// within the first 32 to 256 bytes, so this is a superset).
const RISKY_MARKERS: &[&[u8]] = &[
    // text/html, application/hta, application/xhtml+xml and image/svg+xml.
    b"<!doctype",
    b"<html",
    b"<head",
    b"<body",
    b"<title",
    b"<script",
    b"<svg",
    b"<?xml",
    b"<hta:application",
    b"//w3c//dtd xhtml ",
    b"xhtml1/dtd/xhtml1-strict.dtd",
    // application/x-desktop.
    b"[desktop entry]",
    b"[desktop action",
    b"[kde desktop entry]",
    // Script and program types that are denied by name: jnlp, wsc and sct, xslt, php, perl.
    b"<jnlp",
    b"<scriptlet",
    b"<xsl:stylesheet",
    b"<?php",
    b"use strict",
    b"use warnings",
    b"use diagnostics",
    b"use test::",
    b"\n=pod",
    b"\n=head1 name",
    b"\n=head1 description",
    b"begin {",
    // A shell archive (text/x-shellscript) has its marker at offset 10.
    b"# this is a shell archive",
];

/// What a file may start with, after a byte order mark and white space, to be taken for one of
/// these (the database wants them at offset 0): `<!--` and `<h1` make text/html, `# Config File`
/// and `# KDE Config File` make application/x-desktop, `eval "exec /...` is how Python, Perl,
/// Ruby and Lua scripts start without a `#!` line, and `#@~^` is an encoded VBScript or JScript.
const RISKY_STARTS: &[&[u8]] = &[
    b"<!--",
    b"<h1",
    b"# config file",
    b"# kde config file",
    b"eval \"exec /",
    b"#@~^",
];

/// First bytes of programs that have no ELF header: the old a.out forms (application/
/// x-executable), MZ (DOS and Windows programs), Java classes, Python byte code.
const RISKY_MAGIC: &[&[u8]] = &[
    b"MZ",
    b"\x1c\x52",
    b"\x10\x01",
    b"\x11\x01",
    b"\x83\x01",
    b"\xca\xfe\xba\xbe",
    b"\x99\x4e\x0d\x0a",
];

/// Whether the name alone rules out opening.
pub fn risky_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    RISKY_EXTENSIONS.iter().any(|extension| {
        lower
            .strip_suffix(extension)
            .is_some_and(|rest| rest.ends_with('.') && rest.len() > 1)
    })
}

/// Whether the first bytes of the file are an ELF program or a `#!` script, or hold the markers
/// of a web page, SVG or XML document or desktop launcher (a hostile host can give those any
/// name or none, and the browser would run their script).
pub fn risky_content(head: &[u8]) -> bool {
    let start = head
        .strip_prefix(b"\xef\xbb\xbf")
        .unwrap_or(head)
        .trim_ascii_start();
    head.starts_with(b"\x7fELF")
        || head.starts_with(b"#!")
        || RISKY_MAGIC.iter().any(|magic| head.starts_with(magic))
        || RISKY_STARTS.iter().any(|marker| {
            start.len() >= marker.len() && start[..marker.len()].eq_ignore_ascii_case(marker)
        })
        || RISKY_MARKERS.iter().any(|marker| {
            head.windows(marker.len())
                .any(|window| window.eq_ignore_ascii_case(marker))
        })
}

/// Whether the app may offer to open this finished download: its name and its first
/// [`SNIFF_BYTES`] bytes are both unremarkable. A file that can't be read is not offered.
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
            "x.kdelnk",
            "t.py3",
            "t.PY2",
            "stubs.pyi",
            "Mod.pm",
            "x.perl",
            "x.jse",
            "x.sct",
            "x.wsc",
            "weird.name.sh",
            "a.tar.sh",
            // Added after review: XML, XSLT, zipapps and byte code.
            "page.xht",
            "data.xml",
            "POM.XML",
            "style.xsl",
            "style.xslt",
            "app.pyz",
            "app.pyzw",
            "mod.pyc",
            "mod.pyo",
            // Compared in Unicode lower case: the Kelvin sign is a K to the system's globs.
            "x.des\u{212A}top",
            "x.DES\u{212A}TOP",
            "x.\u{212A}delnk",
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
        // Markup under any name: whitespace, a byte order mark or other text in front, any case.
        for markup in [
            &b"<!DOCTYPE html><script>alert(1)</script>"[..],
            b"<html><body>x",
            b"\n\n  \t<HTML>",
            b"\xef\xbb\xbf<!doctype html>",
            b"<?xml version=\"1.0\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\">",
            b"<svg onload=\"x()\"></svg>",
            b"some text first\n<Script>x()</Script>",
            b"[Desktop Entry]\nExec=sh\n",
            b"# comment\n[Desktop Entry]\nType=Application",
            b"[KDE Desktop Entry]\n",
        ] {
            assert!(risky_content(markup), "{}", String::from_utf8_lossy(markup));
        }
        // What shared-mime-info also takes for a web page, launcher or script, found by reading
        // its database (text/html, xhtml, svg, x-desktop, x-shellscript, x-executable and the
        // script types denied by name).
        for markup in [
            // text/html: a comment or a heading at the start, a title in the first 256 bytes.
            &b"<!-- saved page -->\n<p>hi</p>"[..],
            b"<!--",
            b"\xef\xbb\xbf  <!-- x",
            b"<H1>Index</H1>",
            b"<h1",
            b"Some text\n<TITLE>Report</TITLE>",
            b"<hta:application id=x>",
            // xhtml and svg by their document types.
            b"x //W3C//DTD XHTML 1.0 Strict//EN",
            b"x http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd",
            b"<!-- Created with Inkscape -->",
            // x-desktop and the KDE configuration files of the same type.
            b"# Config File\nExec=x",
            b"# KDE Config File\n[Desktop Entry]",
            b"  \n# kde config file",
            // Other text that runs: jnlp, scriptlets, stylesheets, php, perl, a shell archive.
            b"<jnlp spec=\"1.0\">",
            b"<scriptlet>",
            b"<xsl:stylesheet version=\"1.0\">",
            b"<?php echo 1;",
            b"use strict;\nuse warnings;\n",
            b"use Test::More;",
            b"x\n=pod\n",
            b"x\n=head1 NAME\n",
            b"BEGIN { exit }",
            b"#!/bin/sh\n",
            b"# This is a shell archive (produced by GNU sharutils)",
            b"\n# This is a shell archive",
            // Scripts that start with an eval line, and encoded script.
            b"eval \"exec /usr/bin/perl -S $0 $*\"\n",
            b"eval \"exec /usr/bin/python3 $0\"",
            b"#@~^AAAA",
            // Programs without an ELF header: a.out, DOS and Windows, Java, Python byte code.
            b"MZ\x90\x00",
            b"\x1c\x52abc",
            b"\x10\x01abc",
            b"\x11\x01abc",
            b"\x83\x01abc",
            b"\xca\xfe\xba\xbe\x00\x00",
            b"\x99\x4e\x0d\x0a\x00",
        ] {
            assert!(risky_content(markup), "{}", String::from_utf8_lossy(markup));
        }
        // Their look-alikes that are ordinary files. The markers that the database wants at the
        // start of a file only count there.
        for fine in [
            &b"Notes\n<!-- a comment later in a note -->"[..],
            b"a heading <h1> inside a sentence",
            b"# configuration",
            b"% a comment\neval \"exec /x\" is how it was done",
            b"M",
            b"MY NOTES",
            b"\xff\xd8\xff\xe0JFIF",
            b"PK\x03\x04",
            b"{\"title\": 1}",
        ] {
            assert!(!risky_content(fine), "{}", String::from_utf8_lossy(fine));
        }
        // Plain text, data and documents are fine.
        for fine in [
            &b"Dear Sir,\nthe invoice is attached.\n"[..],
            b"%PDF-1.7\n",
            b"\x89PNG\r\n\x1a\n",
            b"a < b and b > c",
            b"{\"html\": true}",
        ] {
            assert!(!risky_content(fine), "{}", String::from_utf8_lossy(fine));
        }
        // `may_open` reads only SNIFF_BYTES; markup beyond them is not seen.
        let mut late = vec![b'a'; SNIFF_BYTES];
        late.extend_from_slice(b"<html>");
        assert!(!risky_content(&late[..SNIFF_BYTES]));
        assert!(risky_content(&late[6..SNIFF_BYTES + 6]));
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
        // Markup is refused whatever the name: none, an unknown extension, a numbered copy.
        for name in ["invoice", "report.dat", "logo.dat", "invoice (1)", "lead"] {
            for body in [
                &b"<!DOCTYPE html><script>alert(1)</script>"[..],
                b"  \n<html>",
                b"<?xml version=\"1.0\"?><svg>",
            ] {
                assert!(!may_open(&file(name, body)), "{name}");
            }
        }
        // A comment first and the script 600 bytes later, beyond what is read: the comment
        // alone makes it a web page to the system.
        let mut late = b"<!--".to_vec();
        late.extend_from_slice(&[b' '; 600]);
        late.extend_from_slice(b"<script>alert(1)</script>");
        assert!(!may_open(&file("report.dat", &late)));
        assert!(!may_open(&file("report (1).dat", b"# KDE Config File\n")));
        assert!(!may_open(&file("sheet.dat", b"<xsl:stylesheet>")));
        assert!(!may_open(&file("style.xsl", b"plain")));
        assert!(!may_open(&file("launcher.des\u{212a}top", b"plain")));
        assert!(may_open(&file("report.txt", &[b' '; 600])));
        assert!(!may_open(&dir.join("missing.txt")));
        let link = dir.join("link.txt");
        std::os::unix::fs::symlink(dir.join("notes.txt"), &link).unwrap();
        assert!(!may_open(&link), "a symlink is never followed");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
