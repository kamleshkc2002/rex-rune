//! Whether a finished download may be offered an "Open" action. Nothing here opens or runs
//! anything; it only answers the question, in one place.
//!
//! The rule is an allow list: **Open** is offered for a short list of document, text, image and
//! archive types whose final extension is on [`OPENABLE_EXTENSIONS`] and whose first bytes fit
//! the name ([`content_allows`]). Everything else, including every unknown type and every file
//! with no extension, gets only **Show in folder**.
use std::{io::Read, os::unix::fs::OpenOptionsExt, path::Path};

/// The final extensions that may be opened (compared in Unicode lowercase, so `x.PDF` is `pdf`;
/// the Kelvin sign in `.des\u{212a}top` is a `k`, as the system's glob matching treats it). For
/// `x.tar.gz` only `gz` counts. Nothing else is on the list: no markup (`svg`, `html`, `xml`),
/// PostScript, Office documents, audio, video, scripts or packages.
pub const OPENABLE_EXTENSIONS: &[&str] = &[
    "txt", "text", "log", "md", "markdown", "csv", "tsv", "json", "pdf", "png", "jpg", "jpeg",
    "gif", "webp", "zip", "tar", "gz", "tgz", "xz", "zst", "bz2", "7z",
];

/// How many bytes of a file's start decide [`content_allows`].
pub const SNIFF_BYTES: usize = 512;

/// Starts that make a file a program, script, web page, SVG or XML document or desktop launcher
/// whatever it is called, compared without case after an optional UTF-8 byte order mark and
/// leading white space.
const REFUSED_STARTS: &[&[u8]] = &[
    b"#!",
    b"%!ps",
    b"<!doctype",
    b"<html",
    b"<head",
    b"<body",
    b"<script",
    b"<svg",
    b"<?xml",
    b"<!--",
    b"<h1",
    b"<title",
    b"[desktop entry]",
    b"[kde desktop entry]",
    b"[desktop action",
];

/// The final extension of a file name, lowercased, or `None` for a name without one (a name
/// that only starts with a dot, such as `.txt`, has none either).
fn extension(name: &str) -> Option<String> {
    let dot = name.rfind('.').filter(|&i| i > 0)?;
    Some(name[dot + 1..].to_lowercase())
}

/// Whether the file's start is what the type's own name promises (`None`: a text type, which has
/// no magic bytes to require).
fn magic_matches(extension: &str, head: &[u8]) -> Option<bool> {
    let matches = match extension {
        "pdf" => head.starts_with(b"%PDF-"),
        "png" => head.starts_with(b"\x89PNG\r\n\x1a\n"),
        "jpg" | "jpeg" => head.starts_with(b"\xff\xd8\xff"),
        "gif" => head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a"),
        "webp" => head.starts_with(b"RIFF") && head.get(8..12) == Some(b"WEBP"),
        "zip" => head.starts_with(b"PK\x03\x04") || head.starts_with(b"PK\x05\x06"),
        "gz" => head.starts_with(b"\x1f\x8b"),
        "xz" => head.starts_with(b"\xfd7zXZ\0"),
        "zst" => head.starts_with(b"\x28\xb5\x2f\xfd"),
        "bz2" => head.starts_with(b"BZh"),
        "7z" => head.starts_with(b"7z\xbc\xaf\x27\x1c"),
        _ => return None,
    };
    Some(matches)
}

/// Whether the first bytes of a file with this (lowercase, allow-listed) extension are fine: not
/// a program, script, web page, SVG or XML document or desktop launcher under any name, and, for
/// the types that have magic bytes, the type's own.
pub fn content_allows(extension: &str, head: &[u8]) -> bool {
    let start = head
        .strip_prefix(b"\xef\xbb\xbf")
        .unwrap_or(head)
        .trim_ascii_start();
    let refused = head.starts_with(b"\x7fELF")
        || head.starts_with(b"MZ")
        || REFUSED_STARTS.iter().any(|marker| {
            start.len() >= marker.len() && start[..marker.len()].eq_ignore_ascii_case(marker)
        });
    !refused && magic_matches(extension, head).unwrap_or(true)
}

/// Whether the app may offer to open this finished download: a regular file (not reached through
/// a symlink) whose final extension is on [`OPENABLE_EXTENSIONS`] and whose first
/// [`SNIFF_BYTES`] bytes pass [`content_allows`]. A file that can't be read is not offered.
pub fn may_open(path: &Path) -> bool {
    let Some(extension) = path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(extension)
        .filter(|e| OPENABLE_EXTENSIONS.contains(&e.as_str()))
    else {
        return false;
    };
    let Ok(mut file) = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)
    else {
        return false;
    };
    if !file.metadata().is_ok_and(|m| m.is_file()) {
        return false;
    }
    let mut head = [0u8; SNIFF_BYTES];
    let mut filled = 0;
    while filled < head.len() {
        match file.read(&mut head[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(_) => return false,
        }
    }
    content_allows(&extension, &head[..filled])
}

#[cfg(test)]
mod tests {
    use super::*;

    const PDF: &[u8] = b"%PDF-1.7\n";
    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
    const ELF: &[u8] = b"\x7fELF\x02\x01\x01";

    /// A file name for each allowed type, with a start that fits it.
    fn allowed_cases() -> Vec<(String, Vec<u8>)> {
        let body = |extension: &str| -> Vec<u8> {
            match extension {
                "pdf" => PDF.to_vec(),
                "png" => PNG.to_vec(),
                "jpg" | "jpeg" => b"\xff\xd8\xff\xe0\0\x10JFIF".to_vec(),
                "gif" => b"GIF89a\x01\0".to_vec(),
                "webp" => b"RIFF\x24\0\0\0WEBPVP8 ".to_vec(),
                "zip" => b"PK\x03\x04\x14\0".to_vec(),
                "gz" => b"\x1f\x8b\x08\0".to_vec(),
                "xz" => b"\xfd7zXZ\0\0\x04".to_vec(),
                "zst" => b"\x28\xb5\x2f\xfd\x20".to_vec(),
                "bz2" => b"BZh91AY".to_vec(),
                "7z" => b"7z\xbc\xaf\x27\x1c\0\x04".to_vec(),
                _ => b"plain words, 1,2,3\n".to_vec(),
            }
        };
        OPENABLE_EXTENSIONS
            .iter()
            .map(|e| (format!("file.{e}"), body(e)))
            .collect()
    }

    #[test]
    fn the_allow_list_is_exactly_the_agreed_types() {
        assert_eq!(
            OPENABLE_EXTENSIONS,
            [
                "txt", "text", "log", "md", "markdown", "csv", "tsv", "json", "pdf", "png", "jpg",
                "jpeg", "gif", "webp", "zip", "tar", "gz", "tgz", "xz", "zst", "bz2", "7z"
            ]
        );
        // Lowercase, no dots: the extension is lowercased before it is looked up.
        for extension in OPENABLE_EXTENSIONS {
            assert_eq!(*extension, extension.to_lowercase());
            assert!(!extension.contains('.'));
        }
    }

    #[test]
    fn extension_is_the_final_one_in_unicode_lowercase() {
        for (name, want) in [
            ("notes.txt", Some("txt")),
            ("x.tar.gz", Some("gz")),
            ("x.PDF", Some("pdf")),
            ("x.des\u{212a}top", Some("desktop")),
            ("x.txt.", Some("")),
            ("x.txt ", Some("txt ")),
            ("a.b.c", Some("c")),
            ("README", None),
            (".txt", None),
            ("", None),
        ] {
            assert_eq!(extension(name).as_deref(), want, "{name:?}");
        }
    }

    #[test]
    fn the_content_check_refuses_programs_and_markup_and_wants_magic() {
        // A text type passes unless a refused start matches.
        for fine in [
            &b""[..],
            b"Dear Sir,\nthe invoice is attached.\n",
            b"# comment\n",
            b"a < b and b > c",
            b"{\"html\": true}",
            b"id,name\n1,x\n",
            b"M",
            b"MY NOTES",
            b"\xef\xbb\xbfhello",
            // A refused marker later in the text, or only inside a sentence, is not a start.
            b"Notes\n<!-- a comment later in a note -->",
            b"a heading <h1> inside a sentence",
            b"%PDF-1.7 mentioned in text",
        ] {
            assert!(
                content_allows("txt", fine),
                "{}",
                String::from_utf8_lossy(fine)
            );
        }
        for refused in [
            ELF,
            b"#!/bin/sh\n",
            b"#!",
            b"\n  #!/bin/sh",
            b"\xef\xbb\xbf#!/bin/sh\n",
            b"MZ\x90\x00",
            // The match is on the prefix, so plain text that starts with these letters loses Open.
            b"MZ notes for Tuesday",
            b"<header>",
            b"<h10>",
            b"%!PS-Adobe-3.0 EPSF-3.0\n",
            b"<!DOCTYPE html><script>alert(1)</script>",
            b"<html><body>x",
            b"\n\n  \t<HTML>",
            b"\xef\xbb\xbf<!doctype html>",
            b"<head>",
            b"<BODY>",
            b"<Script>x()</Script>",
            b"<svg onload=\"x()\"></svg>",
            b"<?xml version=\"1.0\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\">",
            b"<!-- saved page -->\n<p>hi</p>",
            b"\xef\xbb\xbf  <!-- x",
            b"<H1>Index</H1>",
            b"<TITLE>Report</TITLE>",
            b"[Desktop Entry]\nExec=sh\n",
            b"[KDE Desktop Entry]\n",
            b"[Desktop Action Gallery]\n",
        ] {
            for extension in ["txt", "md", "json", "csv", "log"] {
                assert!(
                    !content_allows(extension, refused),
                    "{extension}: {}",
                    String::from_utf8_lossy(refused)
                );
            }
        }
        // A type with magic bytes needs them.
        for (extension, magic) in [
            ("pdf", PDF),
            ("png", PNG),
            ("jpg", b"\xff\xd8\xff\xdb"),
            ("jpeg", b"\xff\xd8\xff\xe1"),
            ("gif", b"GIF87a"),
            ("gif", b"GIF89a"),
            ("webp", b"RIFF\0\0\0\0WEBP"),
            ("zip", b"PK\x03\x04"),
            ("zip", b"PK\x05\x06\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"),
            ("gz", b"\x1f\x8b"),
            ("xz", b"\xfd7zXZ\0"),
            ("zst", b"\x28\xb5\x2f\xfd"),
            ("bz2", b"BZh"),
            ("7z", b"7z\xbc\xaf\x27\x1c"),
        ] {
            assert!(content_allows(extension, magic), "{extension}");
            // The same bytes under another name's rule, and text, are refused.
            assert!(!content_allows(extension, b"just some text"), "{extension}");
            assert!(!content_allows(extension, b""), "{extension}");
            assert!(!content_allows(extension, &magic[..1]), "{extension}");
            assert!(!content_allows(extension, ELF), "{extension}");
            assert!(!content_allows(extension, b"<html>"), "{extension}");
        }
        assert!(!content_allows("png", PDF));
        assert!(!content_allows("pdf", PNG));
        assert!(!content_allows("webp", b"RIFF\0\0\0\0WAVE"));
        assert!(!content_allows("zip", b"PK\x01\x02"));
        // Tar has no magic bytes at the start: only the refused starts apply.
        assert!(content_allows("tar", b"anything.txt\0\0"));
        assert!(!content_allows("tar", ELF));
    }

    #[test]
    fn may_open_allows_the_listed_types_and_nothing_else() {
        let dir = std::env::temp_dir().join(format!("rex-open-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = |name: &str, bytes: &[u8]| {
            let path = dir.join(name);
            std::fs::write(&path, bytes).unwrap();
            path
        };
        for (name, bytes) in allowed_cases() {
            assert!(may_open(&file(&name, &bytes)), "{name}");
        }
        for (name, bytes) in [
            ("notes.txt", &b"hello"[..]),
            ("empty.txt", b""),
            ("x.PDF", PDF),
            ("Report.Txt", b"words"),
            ("x.tar.gz", b"\x1f\x8b\x08\0"),
            ("backup.tar", b"name\0\0\0"),
            ("a.b.c.md", b"# Title\n"),
            ("report.txt", &[b' '; 600]),
            // A refused start beyond what is read is not seen.
            (
                "long.txt",
                &[b"x".repeat(SNIFF_BYTES).as_slice(), b"<html>"].concat(),
            ),
            ("bom.txt", b"\xef\xbb\xbfhello"),
            // A name that the sanitizer leaves with a space inside.
            ("my notes (1).txt", b"hello"),
        ] {
            assert!(may_open(&file(name, bytes)), "{name}");
        }
        // Not on the list, whatever the bytes: Show in folder only.
        for name in [
            "invoice.eps",
            "x.rdp",
            "x.remmina",
            "x.svg",
            "x.svgz",
            "x.html",
            "x.htm",
            "x.xml",
            "x.desktop",
            "x.sh",
            "run.sh",
            "tool.py",
            "app.jar",
            "App.AppImage",
            "pkg.deb",
            "foo.pkg.tar.zst",
            "setup.exe",
            "x.url",
            "x.docx",
            "x.odt",
            "x.ps",
            "song.mp3",
            "movie.mkv",
            "x.lnk",
            "README",
            "empty",
            "invoice (1)",
            ".txt",
            "x.",
            "x.dat",
            // Compared in Unicode lowercase: the Kelvin sign is a k, so this is .desktop.
            "x.des\u{212a}top",
            "x.DES\u{212a}TOP",
            // As the sanitizer never leaves them, but may_open still refuses them.
            "x.txt.",
            "x.txt ",
            "x.txt\u{a0}",
        ] {
            assert!(!may_open(&file(name, b"plain words")), "{name}");
            assert!(!may_open(&file(name, PDF)), "{name}");
        }
        // The name says one thing and the file another.
        assert!(!may_open(&file(
            "x.pdf",
            b"<!DOCTYPE html><script>alert(1)</script>"
        )));
        assert!(!may_open(&file("x.pdf", b"just text")));
        assert!(!may_open(&file("x.png", ELF)));
        assert!(!may_open(&file("x.png", PDF)));
        assert!(!may_open(&file("x.zip", b"")));
        assert!(!may_open(&file("x.txt", b"#!/bin/sh\nrm -rf ~\n")));
        assert!(!may_open(&file("x.md", b"<!-- note -->\n# Title\n")));
        assert!(!may_open(&file("x.md", b"<h1>Title</h1>")));
        assert!(!may_open(&file("x.json", b"<?xml version=\"1.0\"?>")));
        assert!(!may_open(&file("x.csv", b"[Desktop Entry]\nExec=x")));
        assert!(!may_open(&file("x.log", ELF)));
        assert!(!may_open(&file("x.txt", b"MZ\x90\0")));
        assert!(!may_open(&file("x.txt", b"%!PS-Adobe-3.0\n")));
        // Missing, a directory with an allowed name, and a symlink are not opened.
        assert!(!may_open(&dir.join("missing.txt")));
        let folder = dir.join("folder.txt");
        std::fs::create_dir(&folder).unwrap();
        assert!(!may_open(&folder), "a directory is not a regular file");
        // Not a regular file either: a FIFO that already holds more text than is read.
        let fifo = dir.join("pipe.txt");
        let c_path = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);
        let mut writer = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&fifo)
            .unwrap();
        std::io::Write::write_all(&mut writer, &[b'a'; 600]).unwrap();
        assert!(!may_open(&fifo), "a FIFO is not a regular file");
        let link = dir.join("link.txt");
        std::os::unix::fs::symlink(dir.join("notes.txt"), &link).unwrap();
        assert!(!may_open(&link), "a symlink is never followed");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
