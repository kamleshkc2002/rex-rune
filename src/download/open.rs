//! Whether a finished download may be offered an "Open" action, and the one call that opens it.
//! Nothing here runs on its own; a press on a finished row is the only caller of [`xdg_open`].
//!
//! **Open** is offered for a short list of document, text, image and archive types, and only
//! when three things agree:
//!
//! 1. the name: its final extension is on [`OPENABLE_EXTENSIONS`] (ASCII, any case), and a
//!    compressed type's name has no inner extension other than `tar` ([`name_kind`]);
//! 2. the first bytes: they fit the name and are not a program, page, SVG, launcher or jar
//!    ([`content_allows`]; the text types have no magic, so there they only rule things out);
//! 3. the desktop's own answer to "what type is this file": `xdg-mime query filetype`, run the
//!    way `xdg-open` runs it, is one of the types allowed for that extension ([`type_allowed`]).
//!
//! The third is what decides the program, because `xdg-open` picks the handler from that same
//! answer, which on some desktops comes from the name and on others from the content alone.
//! It fails closed: no `xdg-mime`, no answer in [`TYPE_TIMEOUT`], a failure or an unlisted type
//! means no Open. Everything else gets only **Show in folder**.
use std::{
    ffi::OsString,
    io::Read,
    os::unix::fs::OpenOptionsExt,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// The final extensions that may be opened (ASCII only, compared without case, so `x.PDF` is
/// `pdf`; a non-ASCII extension never matches, whatever Unicode folds it to). For a compressed
/// type the name may only be `x.<ext>` or `x.tar.<ext>`. Nothing else is on the list: no markup
/// (`svg`, `html`, `xml`), PostScript, Office documents, audio, video, scripts or packages, and
/// not `text`, which no file-type pattern matches, so the desktop would decide by content.
pub const OPENABLE_EXTENSIONS: &[&str] = &[
    "txt", "log", "md", "markdown", "csv", "tsv", "json", "pdf", "png", "jpg", "jpeg", "gif",
    "webp", "zip", "tar", "gz", "tgz", "xz", "zst", "bz2", "7z",
];

/// How many bytes of a file's start are read for [`content_allows`].
pub const SNIFF_BYTES: usize = 4096;

/// How many of those a text type must show as UTF-8 without a NUL.
const TEXT_BYTES: usize = 512;

/// How long `xdg-mime` may take to name a file's type.
pub const TYPE_TIMEOUT: Duration = Duration::from_secs(2);

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

/// An allowed name: its extension from [`OPENABLE_EXTENSIONS`], and whether a compressed type's
/// name says `x.tar.<ext>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kind {
    pub extension: &'static str,
    pub tar: bool,
}

/// The allowed kind of a file name, or `None`. The final extension must be on the list, in ASCII
/// (any case). For `gz`, `xz`, `zst` and `bz2`, what comes before it may have no dot (`x.gz`) or
/// end in `.tar` (`x.tar.gz`); `x.ps.gz`, `x.svg.gz` and `x.img.xz` are other types to the
/// desktop and get no Open. A name that only starts with a dot (`.txt`) has no extension.
pub fn name_kind(name: &str) -> Option<Kind> {
    let dot = name.rfind('.').filter(|&i| i > 0)?;
    let last = &name[dot + 1..];
    let extension = *OPENABLE_EXTENSIONS
        .iter()
        .find(|e| e.eq_ignore_ascii_case(last))?;
    let mut tar = false;
    if matches!(extension, "gz" | "xz" | "zst" | "bz2") {
        let stem = &name[..dot];
        if let Some(inner) = stem.rfind('.') {
            if !stem[inner + 1..].eq_ignore_ascii_case("tar") {
                return None;
            }
            tar = true;
        }
    }
    Some(Kind { extension, tar })
}

/// The types with no magic bytes, which the content check can only rule things out for.
fn is_text(extension: &str) -> bool {
    matches!(
        extension,
        "txt" | "log" | "md" | "markdown" | "csv" | "tsv" | "json"
    )
}

/// Whether the first bytes are a zip's own: the first local file header is not a jar's (its name
/// is not `META-INF/...` and its extra fields have no `0xCAFE`), which is what makes `file` and
/// others call a zip `application/java-archive` and run it with `java -jar`. A header the read
/// does not cover whole is refused.
fn zip_allows(head: &[u8]) -> bool {
    if head.starts_with(b"PK\x05\x06") {
        return true; // an empty archive: no entries at all
    }
    let field = |at: usize| {
        head.get(at..at + 2)
            .map(|b| usize::from(b[0]) | usize::from(b[1]) << 8)
    };
    let (Some(name_len), Some(extra_len)) = (field(26), field(28)) else {
        return false;
    };
    let (Some(name), Some(mut extra)) = (
        head.get(30..30 + name_len),
        head.get(30 + name_len..30 + name_len + extra_len),
    ) else {
        return false;
    };
    if name.len() >= 9 && name[..9].eq_ignore_ascii_case(b"META-INF/") {
        return false;
    }
    // Extra fields: a two-byte id and a two-byte length each.
    while extra.len() >= 4 {
        let (id, len) = (
            u16::from_le_bytes([extra[0], extra[1]]),
            usize::from(extra[2]) | usize::from(extra[3]) << 8,
        );
        if id == 0xcafe {
            return false;
        }
        extra = extra.get(4 + len..).unwrap_or_default();
    }
    true
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
        "zip" => {
            (head.starts_with(b"PK\x03\x04") || head.starts_with(b"PK\x05\x06")) && zip_allows(head)
        }
        "tar" => head.get(257..262) == Some(b"ustar"),
        "gz" | "tgz" => head.starts_with(b"\x1f\x8b"),
        "xz" => head.starts_with(b"\xfd7zXZ\0"),
        "zst" => head.starts_with(b"\x28\xb5\x2f\xfd"),
        "bz2" => head.starts_with(b"BZh"),
        "7z" => head.starts_with(b"7z\xbc\xaf\x27\x1c"),
        _ => return None,
    };
    Some(matches)
}

/// Whether a text file's start is plain text: UTF-8 with no NUL in the first [`TEXT_BYTES`] (a
/// character cut by that limit is fine), and no `<script` in any of `head`, in any case, because
/// the desktop types such a file as a web page.
fn text_allows(head: &[u8]) -> bool {
    let limit = &head[..head.len().min(TEXT_BYTES)];
    let utf8 = match std::str::from_utf8(limit) {
        Ok(_) => true,
        // Cut mid-character by the limit, not wrong before it.
        Err(e) => e.error_len().is_none() && head.len() > TEXT_BYTES,
    };
    utf8 && !limit.contains(&0) && !head.windows(7).any(|w| w.eq_ignore_ascii_case(b"<script"))
}

/// Whether the first bytes of a file with this (allow-listed) extension are fine: not a program,
/// script, web page, SVG or XML document or desktop launcher under any name, the text types
/// plain UTF-8 text, and for the types with magic bytes, the type's own (a zip also not a jar, a
/// tar with `ustar` at 257).
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
    !refused
        && match magic_matches(extension, head) {
            Some(fits) => fits,
            None => !is_text(extension) || text_allows(head),
        }
}

/// Whether the desktop may call a file of this kind by `mime` (what `xdg-mime query filetype`
/// printed) and still have `xdg-open` run an ordinary viewer for it. Each extension has its own
/// type and the documented aliases of it (shared-mime-info's `aliases`, and the names `file`
/// prints); a `.tar.gz`, `.tar.xz`, `.tar.zst` and `.tar.bz2` also its compressed-tar type.
/// Anything else, a web page or a Java archive or a package among them, is refused.
pub fn type_allowed(kind: Kind, mime: &str) -> bool {
    let mime = mime.to_ascii_lowercase();
    let mime = mime.as_str();
    let tar = kind.tar;
    match kind.extension {
        "txt" => mime == "text/plain",
        "log" => matches!(mime, "text/plain" | "text/x-log"),
        "md" | "markdown" => matches!(mime, "text/markdown" | "text/x-markdown" | "text/plain"),
        "csv" => matches!(
            mime,
            "text/csv" | "text/x-csv" | "text/x-comma-separated-values" | "text/plain"
        ),
        "tsv" => matches!(mime, "text/tab-separated-values" | "text/plain"),
        "json" => matches!(mime, "application/json" | "text/plain"),
        "pdf" => matches!(
            mime,
            "application/pdf"
                | "application/x-pdf"
                | "application/acrobat"
                | "application/nappdf"
                | "image/pdf"
        ),
        "png" => mime == "image/png",
        "jpg" | "jpeg" => matches!(mime, "image/jpeg" | "image/pjpeg"),
        "gif" => mime == "image/gif",
        "webp" => mime == "image/webp",
        "zip" => matches!(
            mime,
            "application/zip" | "application/x-zip" | "application/x-zip-compressed"
        ),
        "tar" => matches!(mime, "application/x-tar" | "application/x-gtar"),
        "gz" => {
            matches!(mime, "application/gzip" | "application/x-gzip")
                || tar && mime == "application/x-compressed-tar"
        }
        "tgz" => matches!(
            mime,
            "application/x-compressed-tar" | "application/gzip" | "application/x-gzip"
        ),
        "xz" => mime == "application/x-xz" || tar && mime == "application/x-xz-compressed-tar",
        "zst" => {
            matches!(mime, "application/zstd" | "application/x-zstd")
                || tar && mime == "application/x-zstd-compressed-tar"
        }
        "bz2" => {
            matches!(
                mime,
                "application/x-bzip2" | "application/bzip2" | "application/x-bzip"
            ) || tar
                && matches!(
                    mime,
                    "application/x-bzip2-compressed-tar" | "application/x-bzip-compressed-tar"
                )
        }
        "7z" => mime == "application/x-7z-compressed",
        _ => false,
    }
}

/// The name and the start of the file: a regular file (not reached through a symlink) whose
/// name is allowed and whose first [`SNIFF_BYTES`] bytes pass [`content_allows`]. Pure file
/// reads, quick enough for the UI thread; this decides whether a row is a candidate for Open.
/// A file that can't be read is not offered.
pub fn may_open(path: &Path) -> bool {
    kind_if_candidate(path).is_some()
}

fn kind_if_candidate(path: &Path) -> Option<Kind> {
    let kind = path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(name_kind)?;
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)
        .ok()?;
    if !file.metadata().is_ok_and(|m| m.is_file()) {
        return None;
    }
    let mut head = [0u8; SNIFF_BYTES];
    let mut filled = 0;
    while filled < head.len() {
        match file.read(&mut head[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(_) => return None,
        }
    }
    content_allows(kind.extension, &head[..filled]).then_some(kind)
}

/// What names a file's type: [`desktop_type`], or a stand-in in tests.
pub type TypeQuery = fn(&Path) -> Option<String>;

/// Everything: [`may_open`], and the type `query` reports is allowed for the name.
pub fn may_open_with(path: &Path, query: TypeQuery) -> bool {
    kind_if_candidate(path)
        .is_some_and(|kind| query(path).is_some_and(|mime| type_allowed(kind, &mime)))
}

/// [`may_open_with`] the desktop's own answer. Runs a program, so not on the UI thread.
pub fn may_open_on_desktop(path: &Path) -> bool {
    may_open_with(path, desktop_type)
}

/// The type the desktop gives a file: `xdg-mime query filetype <path>`, which is what `xdg-open`
/// asks to choose its handler, run in this process's own environment so both see the same
/// desktop. `None` when there is no `xdg-mime`, it fails, says nothing or more than one word, or
/// takes longer than [`TYPE_TIMEOUT`]. `path` must be absolute.
pub fn desktop_type(path: &Path) -> Option<String> {
    type_from(Command::new("xdg-mime"), path, TYPE_TIMEOUT)
}

/// [`desktop_type`] with the program and the time it gets given, so tests can stand in for
/// `xdg-mime`. `command` is run with `query filetype <path>` added.
fn type_from(mut command: Command, path: &Path, timeout: Duration) -> Option<String> {
    if !path.is_absolute() {
        return None;
    }
    let mut child = command
        .args(["query", "filetype"])
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() < timeout => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    let mut out = String::new();
    child
        .stdout
        .take()?
        .take(1024)
        .read_to_string(&mut out)
        .ok()?;
    // `xdg-open` drops anything after a semicolon from the answer, so does this.
    let answer = out.trim().split(';').next()?.trim();
    (status.success() && !answer.is_empty() && !answer.contains(char::is_whitespace))
        .then(|| answer.to_owned())
}

/// The `xdg-open` call for one absolute path, which is its only argument. `None` for a relative
/// path, which `xdg-open` could read as an option or an URL.
pub fn xdg_open_command(target: &Path) -> Option<Command> {
    if !target.is_absolute() {
        return None;
    }
    let mut command = Command::new("xdg-open");
    command
        .arg::<OsString>(target.into())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    Some(command)
}

/// Hand an absolute path to the desktop's opener and wait for it. A failure is its text.
pub fn xdg_open(target: &Path) -> Result<(), String> {
    let mut command = xdg_open_command(target).ok_or("not an absolute path")?;
    match command.status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(status.to_string()),
        Err(error) => Err(error.to_string()),
    }
}

/// The click: all three checks again, now, and only if they pass, `xdg-open`. `Ok(false)`: not
/// opened because the file is not one that may be. Runs programs, so not on the UI thread.
pub fn open_file(path: &Path) -> Result<bool, String> {
    if !may_open_on_desktop(path) {
        return Ok(false);
    }
    xdg_open(path).map(|()| true)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PDF: &[u8] = b"%PDF-1.7\n";
    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
    const ELF: &[u8] = b"\x7fELF\x02\x01\x01";

    /// A zip's first local file header with this entry name and these extra-field bytes.
    fn zip_head(name: &[u8], extra: &[u8]) -> Vec<u8> {
        let mut head = [b"PK\x03\x04\x14\0".as_slice(), &[0u8; 24]].concat();
        head[26..28].copy_from_slice(&(name.len() as u16).to_le_bytes());
        head[28..30].copy_from_slice(&(extra.len() as u16).to_le_bytes());
        head.extend_from_slice(name);
        head.extend_from_slice(extra);
        head
    }

    /// A tar's first header block: a name, and `ustar` at offset 257.
    fn tar_head() -> Vec<u8> {
        let mut head = vec![0u8; 512];
        head[..5].copy_from_slice(b"a.txt");
        head[257..262].copy_from_slice(b"ustar");
        head
    }

    /// Start bytes that fit each allowed type.
    fn body(extension: &str) -> Vec<u8> {
        match extension {
            "pdf" => PDF.to_vec(),
            "png" => PNG.to_vec(),
            "jpg" | "jpeg" => b"\xff\xd8\xff\xe0\0\x10JFIF".to_vec(),
            "gif" => b"GIF89a\x01\0".to_vec(),
            "webp" => b"RIFF\x24\0\0\0WEBPVP8 ".to_vec(),
            "zip" => zip_head(b"file.txt", b""),
            "tar" => tar_head(),
            "gz" | "tgz" => b"\x1f\x8b\x08\0".to_vec(),
            "xz" => b"\xfd7zXZ\0\0\x04".to_vec(),
            "zst" => b"\x28\xb5\x2f\xfd\x20".to_vec(),
            "bz2" => b"BZh91AY".to_vec(),
            "7z" => b"7z\xbc\xaf\x27\x1c\0\x04".to_vec(),
            _ => b"plain words, 1,2,3\n".to_vec(),
        }
    }

    /// A file name for each allowed type, with a start that fits it.
    fn allowed_cases() -> Vec<(String, Vec<u8>)> {
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
                "txt", "log", "md", "markdown", "csv", "tsv", "json", "pdf", "png", "jpg", "jpeg",
                "gif", "webp", "zip", "tar", "gz", "tgz", "xz", "zst", "bz2", "7z"
            ]
        );
        // `text` matches no file-type pattern, so the desktop would decide by content alone.
        assert!(!OPENABLE_EXTENSIONS.contains(&"text"));
        // Lowercase ASCII, no dots: names are matched to these without case.
        for extension in OPENABLE_EXTENSIONS {
            assert!(extension.is_ascii());
            assert_eq!(*extension, extension.to_ascii_lowercase());
            assert!(!extension.contains('.'));
        }
    }

    #[test]
    fn the_name_decides_the_kind_in_ascii_and_a_compressed_name_has_no_inner_extension_but_tar() {
        let k = |extension, tar| Some(Kind { extension, tar });
        for (name, want) in [
            ("notes.txt", k("txt", false)),
            ("x.PDF", k("pdf", false)),
            ("Report.Txt", k("txt", false)),
            ("x.ZIP", k("zip", false)),
            ("a.b.c.md", k("md", false)),
            ("my notes (1).txt", k("txt", false)),
            ("x.tar", k("tar", false)),
            ("x.tgz", k("tgz", false)),
            // Compressed types: no inner extension, or `tar`.
            ("x.gz", k("gz", false)),
            ("x.tar.gz", k("gz", true)),
            ("x.TAR.GZ", k("gz", true)),
            ("x.xz", k("xz", false)),
            ("x.tar.xz", k("xz", true)),
            ("foo.tar.zst", k("zst", true)),
            ("x.tar.bz2", k("bz2", true)),
            // Typed `application/x-uml` by the desktop, which the type check then refuses.
            ("x.xmi.tar.bz2", k("bz2", true)),
            (".tar.gz", k("gz", true)),
            ("x.ps.gz", None),
            ("x.eps.gz", None),
            ("x.ps.bz2", None),
            ("x.eps.bz2", None),
            ("x.svg.gz", None),
            ("x.img.xz", None),
            ("x.raw-disk-image.xz", None),
            ("x.dvi.gz", None),
            ("x.xcf.gz", None),
            ("backup.2024.gz", None),
            ("a.tar.b.gz", None),
            ("x.tartar.gz", None),
            // Never on the list.
            ("x.text", None),
            ("x.TEXT", None),
            ("x.des\u{212a}top", None),
            ("x.mar\u{212a}down", None),
            ("x.MAR\u{212a}DOWN", None),
            ("x.p\u{0434}f", None),
            ("x.txt.", None),
            ("x.txt ", None),
            ("x.txt\u{a0}", None),
            ("x.", None),
            ("README", None),
            (".txt", None),
            (".gz", None),
            ("", None),
        ] {
            assert_eq!(name_kind(name), want, "{name:?}");
        }
    }

    #[test]
    fn a_text_type_must_be_plain_utf8_with_no_nul_and_no_script_anywhere_it_reads() {
        for fine in [
            &b""[..],
            b"Dear Sir,\nthe invoice is attached.\n",
            "caf\u{e9} \u{30d5}\u{30a1}\u{30a4}\u{30eb}\n".as_bytes(),
            b"\xef\xbb\xbfhello",
            b"a < b and b > c",
            b"<scrap> is not <scrip",
        ] {
            for extension in ["txt", "log", "md", "markdown", "csv", "tsv", "json"] {
                assert!(
                    content_allows(extension, fine),
                    "{extension}: {}",
                    String::from_utf8_lossy(fine)
                );
            }
        }
        for refused in [
            &b"hello\0world"[..],
            b"\0",
            b"caf\xe9 au lait",
            b"\xff\xfe\x00h\x00i",
            b"\xc3",
            b"ok\n<script>alert(1)</script>",
            b"intro\n<SCRIPT src=x>",
            b"a <ScRiPt",
            b"<p><script>x",
        ] {
            for extension in ["txt", "log", "md", "markdown", "csv", "tsv", "json"] {
                assert!(
                    !content_allows(extension, refused),
                    "{extension}: {}",
                    String::from_utf8_lossy(refused)
                );
            }
        }
        // A script anywhere that is read counts, not only in the first 512 bytes.
        let late = [b"x".repeat(SNIFF_BYTES - 7).as_slice(), b"<script"].concat();
        assert!(!content_allows("txt", &late));
        // A character cut by the 512-byte limit is fine; one cut by the end of the file is not,
        // and a bad byte before the limit is not.
        let cut = [b"x".repeat(TEXT_BYTES - 1).as_slice(), "\u{e9}".as_bytes()].concat();
        assert!(content_allows("txt", &cut), "an \u{e9} across the limit");
        let ended = &cut[..TEXT_BYTES];
        assert!(
            !content_allows("txt", ended),
            "half a character at the end of the file"
        );
        let bad = [b"x".repeat(TEXT_BYTES + 10).as_slice(), b"\xff"].concat();
        assert!(
            content_allows("txt", &bad),
            "bytes after the limit are not looked at"
        );
        let early = [b"\xff", b"x".repeat(TEXT_BYTES + 10).as_slice()].concat();
        assert!(!content_allows("txt", &early));
        let nul_late = [b"x".repeat(TEXT_BYTES).as_slice(), b"\0"].concat();
        assert!(
            content_allows("txt", &nul_late),
            "a NUL after the limit is not looked at"
        );
    }

    #[test]
    fn a_zip_is_not_a_jar() {
        assert!(content_allows("zip", &zip_head(b"file.txt", b"")));
        assert!(content_allows("zip", &zip_head(b"dir/META-INF/x", b"")));
        assert!(content_allows("zip", &zip_head(b"META-INF", b"")));
        assert!(content_allows(
            "zip",
            &zip_head(b"a", b"\x55\x54\x05\0\x01\0\0\0\0")
        ));
        assert!(content_allows(
            "zip",
            b"PK\x05\x06\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"
        ));
        // META-INF/ first, in any case, whatever follows it.
        for name in [
            &b"META-INF/"[..],
            b"META-INF/MANIFEST.MF",
            b"meta-inf/x",
            b"Meta-Inf/",
        ] {
            assert!(!content_allows("zip", &zip_head(name, b"")), "{name:?}");
        }
        // The 0xCAFE extra field, first or after another, with or without data.
        for extra in [
            &b"\xfe\xca\0\0"[..],
            b"\xfe\xca\x02\0ab",
            b"\x55\x54\x01\0x\xfe\xca\0\0",
            b"\x55\x54\x03\0xyz\xfe\xca\x01\0z",
        ] {
            assert!(
                !content_allows("zip", &zip_head(b"a.txt", extra)),
                "{extra:?}"
            );
        }
        // A header the bytes do not cover whole is not trusted.
        let mut cut = zip_head(b"a.txt", b"\xfe\xca\0\0");
        cut.truncate(cut.len() - 3);
        assert!(!content_allows("zip", &cut));
        assert!(!content_allows("zip", b"PK\x03\x04\x14\0"));
        // Zip bytes under a name that is not a zip's get no help from this rule, but are not
        // allowed as another type either.
        let jar = zip_head(b"META-INF/", b"\xfe\xca\0\0");
        for extension in ["txt", "tar", "tgz", "gz", "pdf", "png"] {
            assert!(
                !content_allows(extension, &jar),
                "{extension}: zip bytes are not that type"
            );
        }
        // The same jar as a text file is not UTF-8 text without trouble, but a plain-looking
        // one is caught by the type check instead, which the tests of `may_open_with` cover.
    }

    #[test]
    fn tar_needs_ustar_at_257_and_tgz_the_gzip_magic() {
        assert!(content_allows("tar", &tar_head()));
        assert!(!content_allows("tar", b"anything.txt\0\0"));
        assert!(!content_allows("tar", &tar_head()[..261]));
        assert!(!content_allows("tar", ELF));
        let mut shifted = tar_head();
        shifted.rotate_right(1);
        assert!(!content_allows("tar", &shifted));
        assert!(content_allows("tgz", b"\x1f\x8b\x08"));
        assert!(!content_allows("tgz", b"just some bytes"));
        assert!(!content_allows("tgz", b""));
        assert!(!content_allows("tgz", ELF));
    }

    #[test]
    fn the_content_check_refuses_programs_and_markup_and_wants_magic() {
        for fine in [
            &b"# comment\n"[..],
            b"{\"html\": true}",
            b"id,name\n1,x\n",
            b"M",
            b"MY NOTES",
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
            ("gz", b"\x1f\x8b"),
            ("xz", b"\xfd7zXZ\0"),
            ("zst", b"\x28\xb5\x2f\xfd"),
            ("bz2", b"BZh"),
            ("7z", b"7z\xbc\xaf\x27\x1c"),
        ] {
            assert!(content_allows(extension, magic), "{extension}");
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
    }

    #[test]
    fn the_type_table_allows_each_extension_its_own_types_and_nothing_else() {
        let all = [
            "text/plain",
            "text/x-log",
            "text/markdown",
            "text/x-markdown",
            "text/csv",
            "text/x-csv",
            "text/x-comma-separated-values",
            "text/tab-separated-values",
            "application/json",
            "application/pdf",
            "application/x-pdf",
            "image/png",
            "image/jpeg",
            "image/pjpeg",
            "image/gif",
            "image/webp",
            "application/zip",
            "application/x-zip-compressed",
            "application/x-tar",
            "application/gzip",
            "application/x-gzip",
            "application/x-compressed-tar",
            "application/x-xz",
            "application/x-xz-compressed-tar",
            "application/zstd",
            "application/x-zstd",
            "application/x-zstd-compressed-tar",
            "application/x-bzip2",
            "application/x-bzip2-compressed-tar",
            "application/x-7z-compressed",
            // Things a hostile file can be typed as.
            "text/html",
            "application/xhtml+xml",
            "image/svg+xml",
            "image/svg+xml-compressed",
            "application/java-archive",
            "application/x-java-archive",
            "application/vnd.debian.binary-package",
            "application/x-rpm",
            "application/x-ole-storage",
            "application/vnd.flatpak.ref",
            "application/postscript",
            "application/x-gzpostscript",
            "image/x-eps",
            "application/x-raw-disk-image-xz-compressed",
            "application/x-shellscript",
            "application/x-executable",
            "application/x-desktop",
            "application/octet-stream",
            "application/x-zip-compressed-fb2",
            "application/epub+zip",
            "",
            "text",
            "text/plain text/html",
        ];
        let k = |extension, tar| Kind { extension, tar };
        let table: &[(Kind, &[&str])] = &[
            (k("txt", false), &["text/plain"]),
            (k("log", false), &["text/plain", "text/x-log"]),
            (
                k("md", false),
                &["text/markdown", "text/x-markdown", "text/plain"],
            ),
            (
                k("markdown", false),
                &["text/markdown", "text/x-markdown", "text/plain"],
            ),
            (
                k("csv", false),
                &[
                    "text/csv",
                    "text/x-csv",
                    "text/x-comma-separated-values",
                    "text/plain",
                ],
            ),
            (
                k("tsv", false),
                &["text/tab-separated-values", "text/plain"],
            ),
            (k("json", false), &["application/json", "text/plain"]),
            (k("pdf", false), &["application/pdf", "application/x-pdf"]),
            (k("png", false), &["image/png"]),
            (k("jpg", false), &["image/jpeg", "image/pjpeg"]),
            (k("jpeg", false), &["image/jpeg", "image/pjpeg"]),
            (k("gif", false), &["image/gif"]),
            (k("webp", false), &["image/webp"]),
            (
                k("zip", false),
                &["application/zip", "application/x-zip-compressed"],
            ),
            (k("tar", false), &["application/x-tar"]),
            (k("gz", false), &["application/gzip", "application/x-gzip"]),
            (
                k("gz", true),
                &[
                    "application/gzip",
                    "application/x-gzip",
                    "application/x-compressed-tar",
                ],
            ),
            (
                k("tgz", false),
                &[
                    "application/x-compressed-tar",
                    "application/gzip",
                    "application/x-gzip",
                ],
            ),
            (k("xz", false), &["application/x-xz"]),
            (
                k("xz", true),
                &["application/x-xz", "application/x-xz-compressed-tar"],
            ),
            (k("zst", false), &["application/zstd", "application/x-zstd"]),
            (
                k("zst", true),
                &[
                    "application/zstd",
                    "application/x-zstd",
                    "application/x-zstd-compressed-tar",
                ],
            ),
            (k("bz2", false), &["application/x-bzip2"]),
            (
                k("bz2", true),
                &["application/x-bzip2", "application/x-bzip2-compressed-tar"],
            ),
            (k("7z", false), &["application/x-7z-compressed"]),
        ];
        // Every allowed extension is in the table.
        for extension in OPENABLE_EXTENSIONS {
            assert!(
                table.iter().any(|(k, _)| k.extension == *extension),
                "{extension}"
            );
        }
        for (kind, allowed) in table {
            for mime in all {
                assert_eq!(
                    type_allowed(*kind, mime),
                    allowed.contains(&mime),
                    "{kind:?} {mime:?}"
                );
            }
        }
        // The answer is compared without case, and the aliases the table lists are the same type.
        assert!(type_allowed(k("txt", false), "Text/Plain"));
        assert!(type_allowed(k("pdf", false), "application/acrobat"));
        assert!(type_allowed(
            k("bz2", true),
            "application/x-bzip-compressed-tar"
        ));
        // A name this module never allows has no types.
        assert!(!type_allowed(k("svg", false), "image/svg+xml"));
        assert!(!type_allowed(k("text", false), "text/plain"));
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
            ("x.tar.zst", b"\x28\xb5\x2f\xfd\x20"),
            ("backup.tar", &tar_head()),
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
            "x.text",
            "deb.text",
            // Non-ASCII, so never the allowed name its Unicode lowercase would be.
            "x.des\u{212a}top",
            "x.mar\u{212a}down",
            // As the sanitizer never leaves them, but may_open still refuses them.
            "x.txt.",
            "x.txt ",
            "x.txt\u{a0}",
            // Another type to the desktop than the last extension says.
            "x.ps.gz",
            "x.eps.bz2",
            "x.svg.gz",
            "x.img.xz",
        ] {
            assert!(!may_open(&file(name, b"plain words")), "{name}");
            assert!(!may_open(&file(name, PDF)), "{name}");
            assert!(!may_open(&file(name, b"\x1f\x8b\x08\0")), "{name}");
        }
        // A zstd-compressed tar is a `.tar.zst` whatever comes before that; with its own magic
        // it is a candidate, without it it is not.
        assert!(may_open(&file("foo.pkg.tar.zst", b"\x28\xb5\x2f\xfd\x20")));
        assert!(!may_open(&file("foo.pkg.tar.zst", b"just some text")));
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
        assert!(!may_open(&file("x.txt", b"hello\0")));
        assert!(!may_open(&file("x.tar", b"anything.txt\0\0")));
        assert!(!may_open(&file("x.tgz", b"just some bytes")));
        // The jar: a valid zip with META-INF/ first and the 0xCAFE extra field is not a
        // document under any of the allowed names.
        let jar = zip_head(b"META-INF/", b"\xfe\xca\0\0");
        for name in [
            "report.zip",
            "notes.txt",
            "x.tar",
            "x.tgz",
            "x.ZIP",
            "x.log",
            "x.gz",
        ] {
            assert!(!may_open(&file(name, &jar)), "{name}");
        }
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

    #[test]
    fn the_desktops_type_must_be_one_the_table_allows_for_the_name() {
        let dir = std::env::temp_dir().join(format!("rex-open-type-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = |name: &str, bytes: &[u8]| {
            let path = dir.join(name);
            std::fs::write(&path, bytes).unwrap();
            path
        };
        let txt = file("n.txt", b"plain words");
        let zip = file("n.zip", &body("zip"));
        let targz = file("n.tar.gz", &body("gz"));
        let gz = file("n.gz", &body("gz"));
        assert!(may_open_with(&txt, |_| Some("text/plain".into())));
        assert!(may_open_with(&txt, |_| Some("Text/Plain".into())));
        assert!(may_open_with(&zip, |_| Some(
            "application/x-zip-compressed".into()
        )));
        assert!(may_open_with(&targz, |_| Some(
            "application/x-compressed-tar".into()
        )));
        assert!(may_open_with(&targz, |_| Some("application/gzip".into())));
        // The compressed-tar type is only for a name that says tar.
        assert!(!may_open_with(&gz, |_| Some(
            "application/x-compressed-tar".into()
        )));
        // Fail closed: no answer, or one that is not allowed, or one that is a different type.
        assert!(!may_open_with(&txt, |_| None));
        assert!(!may_open_with(&txt, |_| Some(String::new())));
        assert!(!may_open_with(&txt, |_| Some("text/html".into())));
        assert!(!may_open_with(&txt, |_| Some(
            "application/x-shellscript".into()
        )));
        assert!(!may_open_with(&txt, |_| Some(
            "application/octet-stream".into()
        )));
        assert!(!may_open_with(&txt, |_| Some("text".into())));
        assert!(!may_open_with(&txt, |_| Some("application/zip".into())));
        assert!(!may_open_with(&txt, |_| Some(
            "application/java-archive".into()
        )));
        assert!(!may_open_with(&zip, |_| Some(
            "application/java-archive".into()
        )));
        // The cheap check still decides first: the right type for a name does not rescue a
        // file whose bytes are refused, and the query is not even asked.
        let script = file("s.txt", b"#!/bin/sh\nid\n");
        assert!(!may_open_with(&script, |_| Some("text/plain".into())));
        let jar = file("report.zip", &zip_head(b"META-INF/", b"\xfe\xca\0\0"));
        assert!(!may_open_with(&jar, |_| Some("application/zip".into())));
        let page = file("p.txt", b"<p><script>alert(1)</script></p>");
        assert!(!may_open_with(&page, |_| Some("text/plain".into())));
        // A name that is not on the list is not even asked about.
        let svg = file("x.svg", b"plain words");
        assert!(!may_open_with(&svg, |_| panic!(
            "asked about an unlisted name"
        )));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A shell command standing in for `xdg-mime`; `query filetype <path>` become its $1 to $3.
    fn stand_in(script: &str) -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", script, "xdg-mime"]);
        command
    }

    #[test]
    fn xdg_mime_is_run_with_the_absolute_path_and_every_odd_answer_is_no_type() {
        let path = Path::new("/tmp/some dir/-n (1).txt");
        let ask = |script: &str| type_from(stand_in(script), path, Duration::from_secs(5));
        assert_eq!(ask("echo text/plain").as_deref(), Some("text/plain"));
        assert_eq!(
            ask("echo 'text/plain; charset=utf-8'").as_deref(),
            Some("text/plain"),
            "what follows a semicolon is dropped, as xdg-open drops it"
        );
        // The arguments are `query`, `filetype` and then the path whole.
        assert_eq!(
            ask("printf '%s|%s|%s|%s' \"$#\" \"$1\" \"$2\" \"$3\" | tr ' ' _").as_deref(),
            Some("3|query|filetype|/tmp/some_dir/-n_(1).txt")
        );
        // Nothing, a failure, more than one word or line.
        assert_eq!(ask("exit 0"), None);
        assert_eq!(ask("echo"), None);
        assert_eq!(ask("echo text/plain; exit 1"), None);
        assert_eq!(ask("echo text/plain; echo text/html"), None);
        assert_eq!(ask("echo text/plain text/html"), None);
        assert_eq!(ask("echo ';'"), None);
        // No such program.
        assert_eq!(
            type_from(
                Command::new("/nonexistent/xdg-mime"),
                path,
                Duration::from_secs(5)
            ),
            None
        );
        // A relative path is never passed on.
        assert_eq!(
            type_from(
                stand_in("echo text/plain"),
                Path::new("n.txt"),
                Duration::from_secs(5)
            ),
            None
        );
        // Too slow: gives up at the timeout, and says nothing even though it would answer.
        let started = Instant::now();
        let slow = type_from(
            stand_in("sleep 5; echo text/plain"),
            path,
            Duration::from_millis(200),
        );
        assert_eq!(slow, None);
        assert!(
            started.elapsed() < Duration::from_secs(4),
            "{:?}",
            started.elapsed()
        );
        assert!(TYPE_TIMEOUT <= Duration::from_secs(3));
    }

    #[test]
    fn xdg_open_gets_one_argument_the_absolute_path() {
        for target in [
            "/home/u/Downloads/notes.txt",
            "/home/u/Downloads/-rf %41 #x?.txt",
            "/home/u/Downloads/two  spaces & 100%.txt",
            "/home/u/Downloads/\u{fc}n\u{ef} \u{30d5}.txt",
            "/",
        ] {
            let command = xdg_open_command(Path::new(target)).unwrap();
            assert_eq!(command.get_program(), "xdg-open");
            let args: Vec<_> = command.get_args().collect();
            assert_eq!(args, [std::ffi::OsStr::new(target)], "{target}");
            assert_eq!(
                command.get_envs().count(),
                0,
                "the environment is inherited as is"
            );
        }
        // A relative path could be read as an option or an URL.
        for target in ["notes.txt", "-n", "--help", "./a", "file:///etc/passwd", ""] {
            assert!(xdg_open_command(Path::new(target)).is_none(), "{target}");
        }
        assert!(xdg_open(Path::new("-n")).is_err());
    }
}
