//! Bundled chrome fonts, a fontconfig-resolved terminal family and the user's ordered
//! fallbacks; and the families fontconfig knows, for the font pickers in Settings.
use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
    process::Command,
    sync::OnceLock,
};

pub struct Fonts {
    pub files: &'static [&'static [u8]],
    /// System font files Rune registers by path, without copying them into heap buffers.
    pub paths: &'static [PathBuf],
    pub families: (&'static str, &'static str),
    /// The resolved primary family, for Settings; the grid uses the mono slot's full stack.
    pub terminal: Option<&'static str>,
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn pattern(family: &str) -> String {
    family
        .replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace(',', "\\,")
}

fn css_family(family: &str) -> String {
    format!("'{}'", family.replace('\\', "\\\\").replace('\'', "\\'"))
}

fn bundled(family: &str) -> Option<&'static str> {
    [
        rune_theme_opencode::FAMILIES.0,
        rune_theme_opencode::FAMILIES.1,
    ]
    .into_iter()
    .find(|name| name.eq_ignore_ascii_case(family))
}

fn add_name(names: &mut Vec<String>, name: &str) {
    if !names
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(name))
    {
        names.push(name.to_owned());
    }
}

/// The available faces fontconfig resolves for one requested family, in four styles, without
/// accepting an unrelated substitute. Rune synthesizes a style the family does not provide.
fn resolve_family(family: &str) -> Option<(String, Vec<PathBuf>)> {
    let generic = ["monospace", "sans-serif", "serif"]
        .iter()
        .any(|name| family.eq_ignore_ascii_case(name));
    let mut name = None;
    let mut files = Vec::new();
    for style in ["", ":bold", ":italic", ":bold:italic"] {
        let out = Command::new("fc-match")
            .args([
                "-f",
                "%{family[0]}\n%{file}",
                "--",
                &format!("{}{style}", pattern(family)),
            ])
            .output()
            .ok()?;
        if !out.status.success() {
            continue;
        }
        let out = String::from_utf8_lossy(&out.stdout);
        let Some((found, file)) = out.split_once('\n') else {
            continue;
        };
        if !generic && !found.eq_ignore_ascii_case(family) {
            continue;
        }
        let file = PathBuf::from(file.trim());
        if file.is_file() && !files.contains(&file) {
            name.get_or_insert_with(|| found.to_owned());
            files.push(file);
        }
    }
    Some((name?, files))
}

/// A font family a picker lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Family {
    pub name: String,
    /// The name in lower case, which a search is matched against.
    pub lower: String,
    /// Every character is one width (fontconfig's `mono` and `charcell` spacing) or one of
    /// two (`dual`, as CJK terminal fonts are): what a terminal's grid needs.
    pub mono: bool,
}
impl Family {
    pub fn new(name: &str, mono: bool) -> Self {
        Self {
            name: name.to_owned(),
            lower: name.to_lowercase(),
            mono,
        }
    }
}

/// What `fc-list` is asked to print for each face: its first family name (the one
/// `resolve_family` accepts) and its spacing, tab-separated.
const CATALOG_FORMAT: &str = "%{family[0]}\t%{spacing}\n";

/// The families in `fc-list`'s output in [`CATALOG_FORMAT`], and the two bundled ones, in
/// alphabetical order whatever their case. A family is monospaced only when all listed faces
/// are fixed-pitch; mixed families stay in All fonts.
/// Names the settings can't hold (with a comma or a control character) are left out.
pub fn parse_catalog(listing: &str) -> Vec<Family> {
    // Keyed by the name in lower case, so "Hack" and "hack" from two files are one family.
    let mut families: BTreeMap<String, (String, bool)> = BTreeMap::new();
    let mut add = |name: &str, mono: bool| {
        let Ok(name) = rex_linux::settings::parse_font_family(name) else {
            return;
        };
        let entry = families.entry(name.to_lowercase()).or_insert((name, true));
        entry.1 &= mono;
    };
    for line in listing.lines() {
        let (name, spacing) = line.split_once('\t').unwrap_or((line, ""));
        // FC_DUAL, FC_MONO and FC_CHARCELL; a proportional face has no spacing at all.
        let mono = matches!(spacing.trim().parse::<u32>(), Ok(90 | 100 | 110));
        add(name, mono);
    }
    add(rune_theme_opencode::FAMILIES.0, false);
    add(rune_theme_opencode::FAMILIES.1, true);
    families
        .into_values()
        .map(|(name, mono)| Family::new(&name, mono))
        .collect()
}

/// The families installed on this machine and the bundled ones, read from fontconfig once
/// per run. Without `fc-list` only the bundled families are listed.
pub fn installed() -> &'static [Family] {
    static CATALOG: OnceLock<Vec<Family>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let listing = Command::new("fc-list")
            .args(["--format", CATALOG_FORMAT])
            .output()
            .ok()
            .filter(|out| out.status.success())
            .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
            .unwrap_or_default();
        parse_catalog(&listing)
    })
}

/// `family` for a text style ([`rune::Family::Named`]), which loads an installed family the
/// first time it is drawn: a name can be shown in its own face. A family name has no comma,
/// so Rune reads it as one literal name. Each name is kept for the life of the process, once.
pub fn named(family: &str) -> rune::Family {
    thread_local! {
        static NAMES: std::cell::RefCell<std::collections::HashMap<String, &'static str>> =
            Default::default();
    }
    NAMES.with_borrow_mut(|names| {
        rune::Family::Named(
            names
                .entry(family.to_owned())
                .or_insert_with(|| leak(family.to_owned())),
        )
    })
}

pub fn load(terminal_family: &str, fallbacks: &[String]) -> Fonts {
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    let mut names = Vec::new();
    let mut terminal = None;
    for (index, family) in std::iter::once(terminal_family)
        .chain(fallbacks.iter().map(String::as_str))
        .enumerate()
    {
        let resolved = bundled(family).map(str::to_owned).or_else(|| {
            let (name, files) = resolve_family(family)?;
            for file in files {
                if seen.insert(file.clone()) {
                    paths.push(file);
                }
            }
            Some(name)
        });
        match resolved {
            Some(name) => {
                if index == 0 {
                    terminal = Some(leak(name.clone()));
                }
                add_name(&mut names, &name);
            }
            None => {
                if index == 0 {
                    add_name(&mut names, "IBM Plex Mono");
                }
                eprintln!(
                    "Terminal font {family:?} not found; {}.",
                    if index == 0 {
                        "using IBM Plex Mono"
                    } else {
                        "skipping this fallback"
                    }
                );
            }
        }
    }
    add_name(&mut names, "IBM Plex Mono");
    let mono = names
        .iter()
        .map(|name| css_family(name))
        .collect::<Vec<_>>()
        .join(", ");
    Fonts {
        files: rune_theme_opencode::FONTS,
        paths: Box::leak(paths.into_boxed_slice()),
        families: (rune_theme_opencode::FAMILIES.0, leak(mono)),
        terminal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_stack_keeps_chrome_separate_and_preserves_fallback_order() {
        let fonts = load("IBM Plex Mono", &["Inter".into(), "ibm plex mono".into()]);
        assert_eq!(fonts.families.0, rune_theme_opencode::FAMILIES.0);
        assert_eq!(fonts.families.1, "'IBM Plex Mono', 'Inter'");
        assert_eq!(fonts.terminal, Some("IBM Plex Mono"));
        assert_eq!(fonts.files, rune_theme_opencode::FONTS);
        assert!(fonts.paths.is_empty(), "bundled faces are not re-read");
    }

    #[test]
    fn the_catalog_merges_faces_sorts_names_and_marks_fixed_pitch_families() {
        let listing = "Zed Mono\t100\nNoto Sans\t\nhack\t\nHack\t100\nNoto Sans\t\n\
                       Sarasa Term\t90\nCell Font\t110\nPropo\t0\nOdd, Name\t100\n\
                       Fira Code\t\nFira Code\t100\n";
        let catalog = parse_catalog(listing);
        let names: Vec<(&str, bool)> = catalog.iter().map(|f| (f.name.as_str(), f.mono)).collect();
        assert_eq!(
            names,
            [
                ("Cell Font", true),
                ("Fira Code", false),
                // One family for both spellings, excluded because one face is proportional.
                ("hack", false),
                ("IBM Plex Mono", true),
                ("Inter", false),
                ("Noto Sans", false),
                ("Propo", false),
                ("Sarasa Term", true),
                ("Zed Mono", true),
            ]
        );
        assert!(catalog.iter().all(|f| f.lower == f.name.to_lowercase()));
        // With no fontconfig at all the bundled families are still there to choose.
        let bundled: Vec<String> = parse_catalog("").into_iter().map(|f| f.name).collect();
        assert_eq!(bundled, ["IBM Plex Mono", "Inter"]);
    }

    #[test]
    fn the_installed_families_always_include_the_bundled_ones_in_order() {
        // Whatever this machine has, or with no fontconfig at all.
        let catalog = installed();
        assert!(catalog.iter().any(|f| f.name == "IBM Plex Mono" && f.mono));
        assert!(catalog.iter().any(|f| f.name == "Inter"));
        assert!(catalog.windows(2).all(|pair| pair[0].lower < pair[1].lower));
        assert!(std::ptr::eq(catalog, installed()), "read once");
    }

    #[test]
    fn missing_fontconfig_still_lists_bundled_families() {
        const CHILD: &str = "REX_TEST_NO_FONTCONFIG";
        if std::env::var_os(CHILD).is_some() {
            let names: Vec<_> = installed().iter().map(|f| f.name.as_str()).collect();
            assert_eq!(names, ["IBM Plex Mono", "Inter"]);
            assert!(std::ptr::eq(installed(), installed()));
            return;
        }
        // A separate process keeps PATH and the OnceLock isolated from parallel tests.
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "app::fonts::tests::missing_fontconfig_still_lists_bundled_families",
                "--test-threads=1",
            ])
            .env(CHILD, "1")
            .env("PATH", "")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }

    #[test]
    fn a_family_name_is_interned_once_as_a_literal_name() {
        let (a, b) = (named("Joe's Mono"), named("Joe's Mono"));
        let rune::Family::Named(first) = a else {
            panic!("a named family")
        };
        let rune::Family::Named(second) = b else {
            panic!("a named family")
        };
        assert_eq!(first, "Joe's Mono");
        assert!(std::ptr::eq(first, second), "kept once");
    }

    #[test]
    fn named_family_samples_use_the_requested_bundled_face_not_chrome_fallback() {
        let _families = rune::text::with_families("Inter", "Inter");
        let mut ts =
            rune::text::TextSystem::new(rune_theme_opencode::FONTS, rune_theme_opencode::FAMILIES);
        let sample = rune::text::Run::new("Aa 0O 1il");
        let plex = ts.run_layout(
            &sample,
            rune::TextStyle::new(16., 400.).family(named("IBM Plex Mono")),
            1.,
        );
        let chrome = ts.run_layout(
            &sample,
            rune::TextStyle::new(16., 400.).family(named("Inter")),
            1.,
        );
        let bytes = |layout: &rune::text::RunLayout| {
            layout
                .layout
                .lines()
                .next()
                .unwrap()
                .runs()
                .next()
                .unwrap()
                .font()
                .data
                .as_ref()
                .to_vec()
        };
        assert_ne!(
            bytes(&plex),
            bytes(&chrome),
            "a named sample is not silently the chrome face"
        );
        assert_eq!(
            bytes(&plex),
            include_bytes!("../../vendor/rune/crates/rune/assets/IBMPlexMono-Regular.ttf")
        );
    }

    #[test]
    fn fontconfig_and_css_names_are_escaped_as_patterns_not_syntax() {
        assert_eq!(pattern(r"A:B,C\D"), r"A\:B\,C\\D");
        assert_eq!(css_family("Joe's Mono"), "'Joe\\'s Mono'");
    }

    #[test]
    fn native_font_paths_shape_from_the_file_without_an_embedded_copy() {
        let _families = rune::text::with_families("Inter", "Inter");
        const PLEX: &[u8] =
            include_bytes!("../../vendor/rune/crates/rune/assets/IBMPlexMono-Regular.ttf");
        let path = std::env::temp_dir().join(format!(
            "rex-font-path-{}-{}.ttf",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, PLEX).unwrap();
        let style = rune::TextStyle::new(13., 400.).mono();
        let mut without_path =
            rune::text::TextSystem::new_with_paths(&[], &[], ("Inter", "IBM Plex Mono"));
        let fallback_width = without_path
            .layout_rich("MMMM", &[], style, 1., None)
            .full_width();
        let mut with_path = rune::text::TextSystem::new_with_paths(
            &[],
            std::slice::from_ref(&path),
            ("Inter", "IBM Plex Mono"),
        );
        let plex_width = with_path
            .layout_rich("MMMM", &[], style, 1., None)
            .full_width();
        std::fs::remove_file(path).unwrap();
        assert!(
            (plex_width - fallback_width).abs() > 0.5,
            "path font was not selected: {plex_width} vs {fallback_width}"
        );
    }

    #[test]
    #[ignore = "requires fontconfig and a CJK font; exercised by the Nix package check"]
    fn fontconfig_cjk_fallback_has_real_glyphs_and_keeps_ascii_metrics() {
        let _families = rune::text::with_families("Inter", "Inter");
        let fonts = load("IBM Plex Mono", &[]);
        let mut ts =
            rune::text::TextSystem::new_with_paths(fonts.files, fonts.paths, fonts.families);
        let style = rune::TextStyle::new(15., 400.).mono();
        let text = rune::text::Run::new("世界 日本語 한글");
        let shaped = ts.run_layout(&text, style, 1.);
        let mut count = 0;
        for line in shaped.layout.lines() {
            for run in line.runs() {
                for cluster in run.visual_clusters() {
                    for glyph in cluster.glyphs() {
                        assert_ne!(
                            glyph.id,
                            0,
                            "missing CJK glyph in {:?}",
                            cluster.text_range()
                        );
                        count += 1;
                    }
                }
            }
        }
        assert!(count >= 9);
        let mut bundled =
            rune::text::TextSystem::new(rune_theme_opencode::FONTS, rune_theme_opencode::FAMILIES);
        assert_eq!(
            ts.measure("MMMM 1234", style, 1.),
            bundled.measure("MMMM 1234", style, 1.)
        );
    }
}
