//! How this client draws and copies in its terminals: the cursor's shape, the selection's
//! colours and corners, typography, and what selecting text copies. Preferences stay local;
//! changed cell metrics use the existing owner-only resize path.
use crate::terminal::CursorShape;
use serde::{Deserialize, Serialize};

/// The cursor a terminal shows until its program asks for a shape of its own. A value this
/// version doesn't know reads as Block.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorStyle {
    Bar,
    Underline,
    #[default]
    #[serde(other)]
    Block,
}
impl CursorStyle {
    /// In the order the control lists them.
    pub const ALL: [CursorStyle; 3] =
        [CursorStyle::Block, CursorStyle::Bar, CursorStyle::Underline];
    pub fn name(self) -> &'static str {
        match self {
            CursorStyle::Block => "Block",
            CursorStyle::Bar => "Bar",
            CursorStyle::Underline => "Underline",
        }
    }
    pub fn shape(self) -> CursorShape {
        match self {
            CursorStyle::Block => CursorShape::Block,
            CursorStyle::Bar => CursorShape::Bar,
            CursorStyle::Underline => CursorShape::Underline,
        }
    }
}

/// Where one of the selection's two colours comes from. A value this version doesn't know
/// reads as Theme.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionColor {
    /// Each selected cell's own text colour.
    CellForeground,
    /// Each selected cell's own background.
    CellBackground,
    /// The declared terminal theme's selection colour. Without one the fill is the chrome
    /// palette's and the text keeps its own colour.
    #[default]
    #[serde(other)]
    Theme,
}
impl SelectionColor {
    /// In the order the controls list them.
    pub const ALL: [SelectionColor; 3] = [
        SelectionColor::Theme,
        SelectionColor::CellForeground,
        SelectionColor::CellBackground,
    ];
    pub fn name(self) -> &'static str {
        match self {
            SelectionColor::Theme => "Theme",
            SelectionColor::CellForeground => "Cell foreground",
            SelectionColor::CellBackground => "Cell background",
        }
    }
}

/// The Terminal page's cursor, keyboard and selection settings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalConfig {
    pub font: super::typography::Typography,
    pub cursor_style: CursorStyle,
    /// Draw `cursor_style` whatever shape a program asks for.
    pub cursor_force: bool,
    /// Show the keys pressed so far of a multi-key shortcut in the toolbar.
    pub key_sequence_hints: bool,
    /// Text selected with the pointer goes to the clipboard when the button is released.
    pub copy_on_selection: bool,
    /// Text selected with the pointer also becomes the desktop's primary selection, and a
    /// middle-click pastes that selection unless the program is tracking the mouse.
    pub primary_selection: bool,
    pub selection_foreground: SelectionColor,
    pub selection_background: SelectionColor,
    pub selection_rounded: bool,
}
impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            font: Default::default(),
            cursor_style: CursorStyle::default(),
            cursor_force: false,
            key_sequence_hints: true,
            copy_on_selection: false,
            primary_selection: false,
            selection_foreground: SelectionColor::default(),
            selection_background: SelectionColor::default(),
            selection_rounded: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    #[test]
    fn a_file_from_before_these_settings_loads_with_their_defaults() {
        let old: Settings = serde_json::from_str(r#"{"font_size": 13.0}"#).unwrap();
        assert_eq!(old.terminal, TerminalConfig::default());
        let t = old.terminal;
        assert_eq!(t.cursor_style, CursorStyle::Block);
        assert!(!t.cursor_force && !t.copy_on_selection && !t.primary_selection);
        assert!(t.key_sequence_hints && !t.selection_rounded);
        assert_eq!(
            (t.selection_foreground, t.selection_background),
            (SelectionColor::Theme, SelectionColor::Theme)
        );
    }

    #[test]
    fn a_partial_group_and_values_from_a_newer_version_still_load() {
        let some: Settings = serde_json::from_str(
            r#"{"terminal": {"cursor_style": "bar", "selection_background": "cell_foreground",
                "selection_foreground": "sparkles", "copy_on_selection": true}}"#,
        )
        .unwrap();
        assert_eq!(some.terminal.cursor_style, CursorStyle::Bar);
        assert_eq!(
            some.terminal.selection_background,
            SelectionColor::CellForeground
        );
        assert_eq!(some.terminal.selection_foreground, SelectionColor::Theme);
        assert!(some.terminal.copy_on_selection);
        assert!(
            !some.terminal.selection_rounded,
            "old square selection stays unchanged"
        );
        let newer: Settings =
            serde_json::from_str(r#"{"terminal": {"cursor_style": "beam"}}"#).unwrap();
        assert_eq!(newer.terminal.cursor_style, CursorStyle::Block);
    }

    #[test]
    fn existing_title_and_font_keys_stay_top_level_and_unchanged() {
        let old = serde_json::json!({
            "generated_titles": true, "font_size": 17.,
            "font_family": "User Mono", "font_fallbacks": ["User Fallback"]
        });
        let mut settings: Settings = serde_json::from_value(old.clone()).unwrap();
        settings.terminal.cursor_style = CursorStyle::Underline;
        let saved = serde_json::to_value(&settings).unwrap();
        for key in [
            "generated_titles",
            "font_size",
            "font_family",
            "font_fallbacks",
        ] {
            assert_eq!(saved[key], old[key]);
            assert!(saved["terminal"].get(key).is_none());
        }
        assert_eq!(
            saved["terminal"]["font"],
            serde_json::to_value(super::super::typography::Typography::default()).unwrap()
        );
        assert_eq!(
            saved["terminal"],
            serde_json::json!({
                "cursor_style":"underline", "cursor_force":false, "key_sequence_hints":true,
                "copy_on_selection":false, "primary_selection":false,
                "selection_foreground":"theme", "selection_background":"theme", "selection_rounded":false,
                "font": super::super::typography::Typography::default()
            })
        );
    }

    #[test]
    fn every_choice_round_trips_through_a_saved_file() {
        let path =
            std::env::temp_dir().join(format!("rex-terminal-settings-{}.json", std::process::id()));
        for style in CursorStyle::ALL {
            for color in SelectionColor::ALL {
                let terminal = TerminalConfig {
                    cursor_style: style,
                    cursor_force: true,
                    key_sequence_hints: false,
                    copy_on_selection: true,
                    primary_selection: true,
                    selection_foreground: color,
                    selection_background: color,
                    selection_rounded: false,
                    font: Default::default(),
                };
                Settings {
                    terminal: terminal.clone(),
                    ..Settings::default()
                }
                .save(&path)
                .unwrap();
                assert_eq!(Settings::load(&path).unwrap().terminal, terminal);
            }
        }
        std::fs::remove_file(path).ok();
    }
}
