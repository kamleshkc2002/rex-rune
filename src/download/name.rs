//! File names from a remote host are hostile input. The server sends the last element of the
//! path it resolved, verbatim: newlines, escapes, direction overrides, a leading dot or dash,
//! 255 bytes. Everything that shows or stores a name uses the result of [`sanitize`].

/// The longest file name Linux filesystems accept, in bytes.
pub const MAX_NAME: usize = 255;
/// What a name becomes when nothing usable is left of it.
pub const FALLBACK: &str = "download";
/// The longest suffix treated as a file extension, in bytes, dot included.
const MAX_EXTENSION: usize = 16;

/// A character that never survives into a name: control characters (C0, DEL, C1), the line and
/// paragraph separators, and Unicode format controls, which include every direction override and
/// isolate. Zero-width joiner and non-joiner are the exceptions: they are not direction controls
/// and real text (emoji sequences, Persian, Indic scripts) needs them.
fn dropped(c: char) -> bool {
    if c.is_control() {
        return true;
    }
    if matches!(c, '\u{200c}' | '\u{200d}') {
        return false;
    }
    matches!(c,
        '\u{00ad}'
        | '\u{0600}'..='\u{0605}'
        | '\u{061c}'
        | '\u{06dd}'
        | '\u{070f}'
        | '\u{08e2}'
        | '\u{180e}'
        | '\u{200b}'..='\u{200f}'
        | '\u{2028}'..='\u{202e}'
        | '\u{2060}'..='\u{206f}'
        | '\u{feff}'
        | '\u{fff9}'..='\u{fffb}'
        | '\u{110bd}'
        | '\u{110cd}'
        | '\u{13430}'..='\u{1343f}'
        | '\u{1bca0}'..='\u{1bca3}'
        | '\u{1d173}'..='\u{1d17a}'
        | '\u{e0001}'
        | '\u{e0020}'..='\u{e007f}'
    )
}

/// The name a downloaded file is saved under: the last path component of `raw` (both `/` and
/// `\` separate components), without control, direction and other format characters; a leading
/// dot becomes `_`, trailing dots and spaces go, an empty name, `.` or `..` becomes `download`,
/// and the whole is cut to 255 bytes keeping its extension and never splitting a character.
/// The result is always one ordinary path component: no separator, no NUL.
pub fn sanitize(raw: &str) -> String {
    let last = raw
        .rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or("");
    let mut name: String = last.chars().filter(|c| !dropped(*c)).collect();
    if matches!(name.as_str(), "" | "." | "..") {
        return FALLBACK.into();
    }
    if name.starts_with('.') {
        name.replace_range(..1, "_");
    }
    let end = name.trim_end_matches(['.', ' ']).len();
    name.truncate(end);
    if name.is_empty() {
        return FALLBACK.into();
    }
    fit(&name, MAX_NAME)
}

/// Splits a name into its stem and extension (dot included; empty when there is none). A
/// leading dot is not an extension, nor is a very long suffix; `.tar` stays with what follows it.
pub fn split_extension(name: &str) -> (&str, &str) {
    let Some(dot) = name.rfind('.').filter(|&i| i > 0) else {
        return (name, "");
    };
    let (mut stem, mut extension) = name.split_at(dot);
    if extension.len() == 1 || extension.len() > MAX_EXTENSION {
        return (name, "");
    }
    if let Some(tar) = stem.strip_suffix(".tar").map(str::len)
        && tar > 0
        && extension.len() + 4 <= MAX_EXTENSION
    {
        (stem, extension) = name.split_at(tar);
    }
    (stem, extension)
}

/// Cuts `name` to at most `max` bytes (at least 32), keeping its extension and never splitting
/// a UTF-8 character.
pub fn fit(name: &str, max: usize) -> String {
    debug_assert!(max >= 32);
    if name.len() <= max {
        return name.into();
    }
    let (stem, extension) = split_extension(name);
    let budget = max - extension.len();
    let mut cut = budget.min(stem.len());
    while !stem.is_char_boundary(cut) {
        cut -= 1;
    }
    let stem = stem[..cut].trim_end_matches(['.', ' ']);
    let stem = if stem.is_empty() { "_" } else { stem };
    format!("{stem}{extension}")
}

/// The `n`th alternative to `name` for when it exists: `name (1).ext`, `name (2).ext`, cut to
/// fit. `n` 0 is the name itself.
pub fn numbered(name: &str, n: u32) -> String {
    if n == 0 {
        return name.into();
    }
    let (stem, extension) = split_extension(name);
    let suffix = format!(" ({n})");
    let budget = MAX_NAME - suffix.len() - extension.len();
    let mut cut = budget.min(stem.len());
    while !stem.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}{suffix}{extension}", &stem[..cut])
}

/// Whether `name` is one ordinary path component Linux accepts.
pub fn is_component(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NAME
        && name != "."
        && name != ".."
        && !name.contains(['/', '\0'])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_table() {
        for (raw, expected) in [
            ("hello.txt", "hello.txt"),
            ("a b (1).txt", "a b (1).txt"),
            ("  spaced out ", "  spaced out"),
            ("café 界.txt", "café 界.txt"),
            // Traversal and absolute paths: only the last component.
            ("../../etc/passwd", "passwd"),
            ("/etc/shadow", "shadow"),
            ("dir/sub/file.bin", "file.bin"),
            ("a/b/", "b"),
            ("a//b///", "b"),
            ("..\\..\\windows\\system32\\x.dll", "x.dll"),
            ("C:\\Users\\me\\report.pdf", "report.pdf"),
            ("mixed/dir\\file.txt", "file.txt"),
            ("back\\", "back"),
            // Nothing left, or only dots.
            ("", "download"),
            (".", "download"),
            ("..", "download"),
            ("/", "download"),
            ("\\", "download"),
            ("a/..", "download"),
            ("a/.", "download"),
            ("\n\t\u{7f}", "download"),
            ("   ", "download"),
            ("...", "_"),
            (". ", "_"),
            // A leading dot is never a hidden file.
            (".bashrc", "_bashrc"),
            ("..hidden", "_.hidden"),
            (".ssh/authorized_keys", "authorized_keys"),
            // Trailing dots and spaces.
            ("report.", "report"),
            ("report. . ", "report"),
            ("x.txt ", "x.txt"),
            // A leading dash is left alone: nothing here runs a shell.
            ("-rf", "-rf"),
            ("--help.txt", "--help.txt"),
            // Control characters: C0, DEL, C1, NUL, ESC.
            ("line1\nline2.txt", "line1line2.txt"),
            ("tab\there", "tabhere"),
            ("esc\u{1b}[31mred", "esc[31mred"),
            ("nul\0byte", "nulbyte"),
            ("del\u{7f}ete", "delete"),
            ("c1\u{85}\u{9b}x", "c1x"),
            ("bell\u{7}.txt", "bell.txt"),
            // Direction overrides, isolates, marks and other format characters.
            ("photo\u{202e}gpj.exe", "photogpj.exe"),
            ("a\u{202a}b\u{202b}c\u{202c}d\u{202d}e", "abcde"),
            ("x\u{2066}y\u{2067}z\u{2068}w\u{2069}", "xyzw"),
            ("m\u{200e}a\u{200f}r\u{061c}k", "mark"),
            ("zero\u{200b}width\u{feff}\u{2060}", "zerowidth"),
            ("soft\u{ad}hyphen", "softhyphen"),
            ("line\u{2028}sep\u{2029}", "linesep"),
            ("tag\u{e0041}\u{e007f}s", "tags"),
            // Joiners stay: emoji sequences and several scripts need them.
            ("👨\u{200d}👩.txt", "👨\u{200d}👩.txt"),
            ("می\u{200c}خواهم", "می\u{200c}خواهم"),
        ] {
            assert_eq!(sanitize(raw), expected, "{raw:?}");
        }
    }

    #[test]
    fn every_result_is_one_ordinary_component() {
        let pieces = [
            "a", ".", "..", "/", "\\", "\0", "\n", " ", "é", "界", "\u{202e}", "\u{200d}",
            "\u{7f}", "x.y", ".tar", ".gz", "-", "\u{85}", "😀", "\u{feff}",
        ];
        // Every combination of up to four pieces.
        let mut stack = vec![String::new()];
        for _ in 0..4 {
            let mut next = Vec::new();
            for base in &stack {
                for piece in pieces {
                    let candidate = format!("{base}{piece}");
                    let name = sanitize(&candidate);
                    assert!(is_component(&name), "{candidate:?} -> {name:?}");
                    assert!(!name.starts_with('.'), "{candidate:?} -> {name:?}");
                    assert!(!name.ends_with(['.', ' ']), "{candidate:?} -> {name:?}");
                    assert_eq!(sanitize(&name), name, "idempotent: {candidate:?}");
                    next.push(candidate);
                }
            }
            stack = next;
        }
    }

    #[test]
    fn long_names_keep_their_extension_and_whole_characters() {
        let long = format!("{}.txt", "a".repeat(400));
        let name = sanitize(&long);
        assert_eq!(name.len(), 255);
        assert!(name.ends_with(".txt"));
        assert!(name.starts_with("aaaa"));

        // Exactly 255 bytes is untouched.
        let exact = format!("{}.bin", "b".repeat(251));
        assert_eq!(sanitize(&exact), exact);

        // The cut falls inside a three-byte character: it moves back, never splits.
        let wide = format!("{}.md", "界".repeat(100));
        let name = sanitize(&wide);
        assert!(name.len() <= 255 && name.ends_with(".md"));
        assert_eq!(name.chars().filter(|c| *c == '界').count(), 84, "{name}");
        // A four-byte character at the cut.
        let emoji = format!("{}.md", "😀".repeat(80));
        let name = sanitize(&emoji);
        assert!(name.len() <= 255 && name.ends_with(".md"));
        assert!(name.chars().filter(|c| *c == '😀').count() == 63, "{name}");

        // No extension: cut from the end.
        let plain = "z".repeat(1000);
        assert_eq!(sanitize(&plain).len(), 255);
        // A "extension" too long to keep is not one.
        let odd = format!("a.{}", "e".repeat(300));
        let name = sanitize(&odd);
        assert_eq!(name.len(), 255);
        // Dots left at the cut are trimmed.
        let dotted = format!("{}.. .txt", "q".repeat(250));
        assert!(sanitize(&dotted).ends_with(".txt"));
        assert!(!sanitize(&dotted).contains(". ."));
        // Compound extensions survive.
        let tar = format!("{}.tar.gz", "t".repeat(300));
        let name = sanitize(&tar);
        assert!(name.ends_with(".tar.gz") && name.len() == 255, "{name}");
    }

    #[test]
    fn numbered_names_keep_the_extension_and_the_limit() {
        assert_eq!(numbered("a.txt", 0), "a.txt");
        assert_eq!(numbered("a.txt", 1), "a (1).txt");
        assert_eq!(numbered("a.txt", 12), "a (12).txt");
        assert_eq!(numbered("noext", 2), "noext (2)");
        assert_eq!(numbered("x.tar.gz", 1), "x (1).tar.gz");
        assert_eq!(numbered(".hidden", 1), ".hidden (1)");
        let long = sanitize(&format!("{}.txt", "w".repeat(400)));
        for n in [1, 9, 10, 999, 12345] {
            let name = numbered(&long, n);
            assert!(name.len() <= 255, "{n}: {}", name.len());
            assert!(name.ends_with(&format!(" ({n}).txt")));
        }
        let wide = sanitize(&format!("{}.md", "界".repeat(100)));
        assert!(numbered(&wide, 77).len() <= 255);
    }

    #[test]
    fn extensions() {
        assert_eq!(split_extension("a.txt"), ("a", ".txt"));
        assert_eq!(split_extension("a.tar.gz"), ("a", ".tar.gz"));
        assert_eq!(split_extension("a.b.c"), ("a.b", ".c"));
        assert_eq!(split_extension(".bashrc"), (".bashrc", ""));
        assert_eq!(split_extension("plain"), ("plain", ""));
        assert_eq!(split_extension("trailing."), ("trailing.", ""));
        assert_eq!(split_extension(".tar.gz"), (".tar", ".gz"));
    }
}
