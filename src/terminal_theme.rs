//! Terminal colour themes this client can declare: the built-in ones paired with the chrome
//! palettes, and Ghostty theme files from the user's own install.
//!
//! A theme reaches the terminal only through the server (`terminal.set_theme`), which applies
//! the owner's declaration and sends it back on the block's stream; the client never recolours
//! output itself. Ghostty's cursor-text and selection colours have no protocol field and are
//! drawn locally.
//!
//! A Ghostty theme file is `key = value` lines; `#` starts a comment and later lines win. The
//! keys read are `background`, `foreground`, `cursor-color`, `cursor-text`,
//! `selection-background` and `selection-foreground`, each `#rrggbb`, `rrggbb` or `#rgb`
//! (optionally quoted), and `palette = N=#rrggbb` with N from 0 to 255. Other keys, X11 colour
//! names and special values such as `cell-foreground` are ignored.
use crate::settings::{ChromeTheme, TerminalThemeChoice};
use anyhow::{Result, bail, ensure};
use rexrs::api::terminal::SetTheme;
use std::{
    collections::HashSet,
    ffi::OsString,
    path::{Path, PathBuf},
};

/// Midnight's background: the navy the Midnight chrome is drawn in.
pub const MIDNIGHT_BACKGROUND: u32 = 0x001528;

const GRUVBOX_DARK: [u32; 16] = [
    0x282828, 0xcc241d, 0x98971a, 0xd79921, 0x458588, 0xb16286, 0x689d6a, 0xa89984, 0x928374,
    0xfb4934, 0xb8bb26, 0xfabd2f, 0x83a598, 0xd3869b, 0x8ec07c, 0xebdbb2,
];

/// A theme's colours; a colour left out is Rex's built-in one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TerminalTheme {
    pub background: Option<u32>,
    pub foreground: Option<u32>,
    pub cursor: Option<u32>,
    pub cursor_text: Option<u32>,
    pub selection_background: Option<u32>,
    pub selection_foreground: Option<u32>,
    /// Changes to the 256-colour palette, by index.
    pub palette: Vec<(u8, u32)>,
    /// `dark` or `light` when the theme says so; otherwise the server judges its background.
    pub scheme: Option<&'static str>,
}

/// A terminal theme that ships with the client, paired with a chrome palette.
pub struct Builtin {
    /// Its id in the `terminal_theme` setting, after [`BUILTIN_PREFIX`].
    pub id: &'static str,
    pub name: &'static str,
    pub chrome: ChromeTheme,
    pub theme: fn() -> TerminalTheme,
}

/// Built-in theme ids carry this prefix; Ghostty theme names never contain a slash.
pub const BUILTIN_PREFIX: &str = "rex/";

pub const BUILTINS: &[Builtin] = &[
    Builtin {
        id: "midnight",
        name: "Midnight",
        chrome: ChromeTheme::Midnight,
        theme: TerminalTheme::midnight,
    },
    Builtin {
        id: "rose-pine",
        name: "Rosé Pine",
        chrome: ChromeTheme::RosePine,
        theme: TerminalTheme::rose_pine,
    },
    Builtin {
        id: "rose-pine-moon",
        name: "Rosé Pine Moon",
        chrome: ChromeTheme::RosePineMoon,
        theme: TerminalTheme::rose_pine_moon,
    },
    Builtin {
        id: "gruvbox-dark",
        name: "Gruvbox Dark",
        chrome: ChromeTheme::GruvboxDark,
        theme: TerminalTheme::gruvbox_dark,
    },
];

/// The built-in theme a `named:` id refers to, if it names one.
pub fn builtin(id: &str) -> Option<&'static Builtin> {
    let id = id.strip_prefix(BUILTIN_PREFIX)?;
    BUILTINS.iter().find(|b| b.id == id)
}

/// The built-in theme paired with `chrome`: its background is the chrome's ground.
pub fn paired(chrome: ChromeTheme) -> &'static Builtin {
    BUILTINS
        .iter()
        .find(|b| b.chrome == chrome)
        .expect("every chrome palette has a paired theme")
}

impl TerminalTheme {
    /// Midnight's own: the chrome's navy behind Rex's default text, dark.
    pub fn midnight() -> Self {
        Self {
            background: Some(MIDNIGHT_BACKGROUND),
            scheme: Some("dark"),
            ..Self::default()
        }
    }

    /// Rosé Pine, as its Ghostty theme draws it.
    pub fn rose_pine() -> Self {
        Self::full(
            0x191724,
            [0xe0def4, 0xe0def4, 0x191724, 0x403d52, 0xe0def4],
            [
                0x26233a, 0xeb6f92, 0x31748f, 0xf6c177, 0x9ccfd8, 0xc4a7e7, 0xebbcba, 0xe0def4,
                0x6e6a86, 0xeb6f92, 0x31748f, 0xf6c177, 0x9ccfd8, 0xc4a7e7, 0xebbcba, 0xe0def4,
            ],
        )
    }

    /// Rosé Pine Moon, as its Ghostty theme draws it.
    pub fn rose_pine_moon() -> Self {
        Self::full(
            0x232136,
            [0xe0def4, 0xe0def4, 0x232136, 0x44415a, 0xe0def4],
            [
                0x393552, 0xeb6f92, 0x3e8fb0, 0xf6c177, 0x9ccfd8, 0xc4a7e7, 0xea9a97, 0xe0def4,
                0x6e6a86, 0xeb6f92, 0x3e8fb0, 0xf6c177, 0x9ccfd8, 0xc4a7e7, 0xea9a97, 0xe0def4,
            ],
        )
    }

    /// Gruvbox Dark's background, text, cursor and palette; selection stays the chrome's.
    pub fn gruvbox_dark() -> Self {
        Self {
            background: Some(0x282828),
            foreground: Some(0xebdbb2),
            cursor: Some(0xebdbb2),
            palette: (0..).zip(GRUVBOX_DARK).collect(),
            scheme: Some("dark"),
            ..Self::default()
        }
    }

    /// A dark theme with every colour: foreground, cursor, cursor text, selection background
    /// and selection foreground, then palette colours 0–15.
    fn full(
        background: u32,
        [fg, cursor, cursor_text, sel_bg, sel_fg]: [u32; 5],
        palette: [u32; 16],
    ) -> Self {
        Self {
            background: Some(background),
            foreground: Some(fg),
            cursor: Some(cursor),
            cursor_text: Some(cursor_text),
            selection_background: Some(sel_bg),
            selection_foreground: Some(sel_fg),
            palette: (0..).zip(palette).collect(),
            scheme: Some("dark"),
        }
    }

    /// A Ghostty theme file's colours, starting from none: a colour the file doesn't set
    /// stays Rex's built-in one.
    pub fn parse(name: &str, text: &str) -> Result<Self> {
        let mut theme = Self::default();
        let mut known = false;
        for line in text.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .unwrap_or(value);
            let slot = match key.trim() {
                "palette" => {
                    known = true;
                    if let Some((index, color)) = value.split_once('=')
                        && let (Ok(index), Some(color)) =
                            (index.trim().parse::<u8>(), parse_hex(color.trim()))
                    {
                        theme.palette.retain(|&(i, _)| i != index);
                        theme.palette.push((index, color));
                    }
                    continue;
                }
                "background" => &mut theme.background,
                "foreground" => &mut theme.foreground,
                "cursor-color" => &mut theme.cursor,
                "cursor-text" => &mut theme.cursor_text,
                "selection-background" => &mut theme.selection_background,
                "selection-foreground" => &mut theme.selection_foreground,
                _ => continue,
            };
            known = true;
            if let Some(color) = parse_hex(value) {
                *slot = Some(color);
            }
        }
        ensure!(known, "“{name}” is not a Ghostty theme");
        theme.palette.sort_unstable_by_key(|&(i, _)| i);
        Ok(theme)
    }

    /// The `terminal.set_theme` arguments. A theme with no colours declares Rex's built-in
    /// ones.
    pub fn args(&self) -> SetTheme {
        let mut set = SetTheme::new();
        if let Some(color) = self.background {
            set = set.background(hex(color));
        }
        if let Some(color) = self.foreground {
            set = set.foreground(hex(color));
        }
        if let Some(color) = self.cursor {
            set = set.cursor(hex(color));
        }
        if !self.palette.is_empty() {
            set = set.palette(
                self.palette
                    .iter()
                    .map(|&(i, color)| format!("{i}={}", hex(color))),
            );
        }
        if let Some(scheme) = self.scheme {
            set = set.scheme(scheme);
        }
        set
    }
}

/// Whether a colour reads as light: perceived brightness above half.
pub fn is_light(color: u32) -> bool {
    let (r, g, b) = ((color >> 16) & 0xff, (color >> 8) & 0xff, color & 0xff);
    (299 * r + 587 * g + 114 * b) / 1000 > 128
}

/// Where Ghostty themes are found, in Ghostty's order: the user's own, then Ghostty's.
pub struct ThemeDirs {
    pub user: Option<PathBuf>,
    pub system: Vec<PathBuf>,
}

impl ThemeDirs {
    /// `$XDG_CONFIG_HOME/ghostty/themes` (default `~/.config`), then
    /// `$GHOSTTY_RESOURCES_DIR/themes`, then `ghostty/themes` under each `$XDG_DATA_DIRS`.
    pub fn from_env() -> Self {
        Self::from_vars(|key| std::env::var_os(key))
    }

    fn from_vars(var: impl Fn(&str) -> Option<OsString>) -> Self {
        let var = |key: &str| var(key).filter(|v| !v.is_empty());
        let user = var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| var("HOME").map(|home| PathBuf::from(home).join(".config")))
            .map(|config| config.join("ghostty/themes"));
        let mut system: Vec<PathBuf> = var("GHOSTTY_RESOURCES_DIR")
            .map(|dir| PathBuf::from(dir).join("themes"))
            .into_iter()
            .collect();
        let data = var("XDG_DATA_DIRS").unwrap_or_else(|| "/usr/local/share:/usr/share".into());
        system.extend(
            std::env::split_paths(&data)
                .filter(|dir| dir.is_absolute())
                .map(|dir| dir.join("ghostty/themes")),
        );
        Self { user, system }
    }

    fn all(&self) -> impl Iterator<Item = (&Path, bool)> {
        let user = self.user.iter().map(|dir| (dir.as_path(), true));
        user.chain(self.system.iter().map(|dir| (dir.as_path(), false)))
    }

    /// Every readable theme, a name found twice taking the first directory's file, sorted
    /// by name regardless of case.
    pub fn list(&self) -> Vec<ThemeInfo> {
        let mut seen = HashSet::new();
        let mut themes = Vec::new();
        for (dir, custom) in self.all() {
            let Ok(entries) = std::fs::read_dir(dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                    continue;
                };
                if !valid_name(&name) || seen.contains(&name) {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(entry.path()) else {
                    continue;
                };
                if let Ok(theme) = TerminalTheme::parse(&name, &text) {
                    seen.insert(name.clone());
                    themes.push(ThemeInfo {
                        light: theme.background.is_some_and(is_light),
                        name,
                        custom,
                    });
                }
            }
        }
        themes.sort_by_cached_key(|t| t.name.to_lowercase());
        themes
    }

    /// Ghostty's theme by name, from the first directory that has it.
    pub fn load(&self, name: &str) -> Result<TerminalTheme> {
        ensure!(valid_name(name), "“{name}” is not a theme name");
        for (dir, _) in self.all() {
            if let Ok(text) = std::fs::read_to_string(dir.join(name)) {
                return TerminalTheme::parse(name, &text);
            }
        }
        bail!("Theme “{name}” not found")
    }

    /// The theme a setting declares under `chrome`: none for Follow server, no colours for
    /// Rex's defaults.
    pub fn resolve(
        &self,
        choice: &TerminalThemeChoice,
        chrome: ChromeTheme,
    ) -> Result<Option<TerminalTheme>> {
        Ok(match choice {
            TerminalThemeChoice::FollowServer => None,
            TerminalThemeChoice::RexDefault => Some(TerminalTheme::default()),
            TerminalThemeChoice::FollowChrome => Some((paired(chrome).theme)()),
            TerminalThemeChoice::Named(id) => Some(match builtin(id) {
                Some(builtin) => (builtin.theme)(),
                None => self.load(id)?,
            }),
        })
    }
}

/// One theme the picker offers.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeInfo {
    pub name: String,
    pub light: bool,
    /// Found in the user's own theme directory.
    pub custom: bool,
}

/// A file name in a theme directory: never a path that leaves it.
fn valid_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\'])
}

fn parse_hex(value: &str) -> Option<u32> {
    let digits = value.strip_prefix('#').unwrap_or(value);
    if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    match digits.len() {
        6 => u32::from_str_radix(digits, 16).ok(),
        3 => {
            let short = u32::from_str_radix(digits, 16).ok()?;
            let (r, g, b) = ((short >> 8) & 0xf, (short >> 4) & 0xf, short & 0xf);
            Some(((r * 0x11) << 16) | ((g * 0x11) << 8) | (b * 0x11))
        }
        _ => None,
    }
}

fn hex(color: u32) -> String {
    format!("#{color:06x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const MOCHA: &str = "\
palette = 0=#45475a
palette = 1=#f38ba8
palette = 15=#a6adc8
background = #1e1e2e
foreground = #cdd6f4
cursor-color = #f5e0dc
cursor-text = #1e1e2e
selection-background = #f5e0dc
selection-foreground = #1e1e2e
";

    fn args(theme: &TerminalTheme) -> serde_json::Value {
        serde_json::to_value(theme.args()).unwrap()
    }

    #[test]
    fn midnight_declares_the_navy_and_dark() {
        assert_eq!(
            args(&TerminalTheme::midnight()),
            json!({"background":"#001528","scheme":"dark"})
        );
        assert_eq!(
            args(&TerminalTheme::default()),
            json!({}),
            "Rex's own colours"
        );
    }

    #[test]
    fn a_ghostty_theme_becomes_set_theme_arguments() {
        let theme = TerminalTheme::parse("Catppuccin Mocha", MOCHA).unwrap();
        assert_eq!(
            args(&theme),
            json!({
                "background":"#1e1e2e",
                "foreground":"#cdd6f4",
                "cursor":"#f5e0dc",
                "palette":["0=#45475a","1=#f38ba8","15=#a6adc8"],
            })
        );
        assert_eq!(theme.cursor_text, Some(0x1e1e2e));
        assert_eq!(
            (theme.selection_background, theme.selection_foreground),
            (Some(0xf5e0dc), Some(0x1e1e2e))
        );
    }

    #[test]
    fn ghostty_themes_never_start_from_a_builtin_theme() {
        let plain = TerminalTheme::parse("plain", "palette = 2=#00ff00\n").unwrap();
        assert_eq!(plain.background, None);
        assert_eq!(plain.scheme, None);
        assert_eq!(args(&plain), json!({"palette":["2=#00ff00"]}));
    }

    #[test]
    fn parsing_follows_ghostty_config_rules() {
        let theme = TerminalTheme::parse(
            "t",
            "# a comment\nbackground= #171219\nforeground = \"eee4ff\"\nfont-size = 13\n\
             palette = 3=#abc\npalette = 3=#010203\npalette = 300=#ffffff\npalette = 4=red\n\
             cursor-color = cell-foreground\n",
        )
        .unwrap();
        assert_eq!(theme.background, Some(0x171219));
        assert_eq!(theme.foreground, Some(0xeee4ff));
        assert_eq!(theme.palette, [(3, 0x010203)], "later lines win");
        assert_eq!(theme.cursor, None, "unreadable colours keep the default");
        assert!(TerminalTheme::parse("x", "font-size = 13\n").is_err());
        assert_eq!(parse_hex("#abc"), Some(0xaabbcc));
    }

    #[test]
    fn a_users_theme_overrides_ghosttys_and_names_never_leave_the_directory() {
        let root = std::env::temp_dir().join(format!("rex-rune-themes-{}", std::process::id()));
        let (user, system) = (root.join("user"), root.join("system"));
        std::fs::create_dir_all(&user).unwrap();
        std::fs::create_dir_all(&system).unwrap();
        std::fs::write(user.join("Shared"), "background = #ffffff\n").unwrap();
        std::fs::write(system.join("Shared"), "background = #000000\n").unwrap();
        std::fs::write(system.join("only system"), "background = #101010\n").unwrap();
        std::fs::write(system.join("broken"), "font-size = 13\n").unwrap();
        std::fs::write(root.join("outside"), "background = #123456\n").unwrap();
        let dirs = ThemeDirs {
            user: Some(user.clone()),
            system: vec![system.clone(), root.join("missing")],
        };

        assert_eq!(dirs.load("Shared").unwrap().background, Some(0xffffff));
        assert_eq!(
            dirs.list(),
            [
                ThemeInfo {
                    name: "only system".into(),
                    light: false,
                    custom: false
                },
                ThemeInfo {
                    name: "Shared".into(),
                    light: true,
                    custom: true
                },
            ]
        );
        assert!(dirs.load("../outside").is_err());
        assert!(dirs.load("missing").is_err());
        let named = |id: &str| TerminalThemeChoice::Named(id.into());
        let dirs = |choice: &TerminalThemeChoice| dirs.resolve(choice, ChromeTheme::Midnight);
        assert_eq!(
            dirs(&named("rex/midnight")).unwrap(),
            Some(TerminalTheme::midnight())
        );
        assert!(
            dirs(&named("rex/missing")).is_err(),
            "not a built-in, and not a file name either"
        );
        assert_eq!(
            dirs(&named("Shared")).unwrap().unwrap().background,
            Some(0xffffff)
        );
        assert_eq!(dirs(&TerminalThemeChoice::FollowServer).unwrap(), None);
        assert_eq!(
            dirs(&TerminalThemeChoice::RexDefault).unwrap(),
            Some(TerminalTheme::default())
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn follow_chrome_declares_the_palettes_paired_theme() {
        let dirs = ThemeDirs {
            user: None,
            system: vec![],
        };
        for chrome in ChromeTheme::ALL {
            let builtin = paired(chrome);
            assert_eq!(builtin.name, chrome.name());
            let theme = dirs
                .resolve(&TerminalThemeChoice::FollowChrome, chrome)
                .unwrap()
                .unwrap();
            assert_eq!(theme, (builtin.theme)());
            assert_eq!(
                dirs.resolve(
                    &TerminalThemeChoice::Named(format!("{BUILTIN_PREFIX}{}", builtin.id)),
                    ChromeTheme::Midnight
                )
                .unwrap(),
                Some(theme.clone()),
                "a paired theme can be named under any chrome"
            );
            assert_eq!(args(&theme)["scheme"], "dark");
        }
        assert_eq!(
            args(&TerminalTheme::rose_pine()),
            json!({
                "background":"#191724",
                "foreground":"#e0def4",
                "cursor":"#e0def4",
                "palette":[
                    "0=#26233a","1=#eb6f92","2=#31748f","3=#f6c177","4=#9ccfd8","5=#c4a7e7",
                    "6=#ebbcba","7=#e0def4","8=#6e6a86","9=#eb6f92","10=#31748f","11=#f6c177",
                    "12=#9ccfd8","13=#c4a7e7","14=#ebbcba","15=#e0def4",
                ],
                "scheme":"dark",
            })
        );
        let gruvbox = TerminalTheme::gruvbox_dark();
        assert_eq!(
            (
                gruvbox.background,
                gruvbox.foreground,
                gruvbox.palette.len()
            ),
            (Some(0x282828), Some(0xebdbb2), 16)
        );
        assert_eq!(gruvbox.selection_background, None, "the chrome's selection");
    }

    #[test]
    fn the_user_directory_follows_xdg_config_home() {
        let env = |config: &'static str| {
            move |key: &str| {
                Some(OsString::from(match key {
                    "XDG_CONFIG_HOME" => config,
                    "HOME" => "/home/someone",
                    "GHOSTTY_RESOURCES_DIR" => "/opt/ghostty",
                    "XDG_DATA_DIRS" => "/usr/share:relative",
                    _ => return None,
                }))
            }
        };
        let dirs = ThemeDirs::from_vars(env("/tmp/xdg"));
        assert_eq!(dirs.user, Some(PathBuf::from("/tmp/xdg/ghostty/themes")));
        assert_eq!(
            dirs.system,
            [
                PathBuf::from("/opt/ghostty/themes"),
                PathBuf::from("/usr/share/ghostty/themes")
            ]
        );
        assert_eq!(
            ThemeDirs::from_vars(env("")).user,
            Some(PathBuf::from("/home/someone/.config/ghostty/themes")),
            "an empty XDG_CONFIG_HOME means ~/.config"
        );
    }
}
