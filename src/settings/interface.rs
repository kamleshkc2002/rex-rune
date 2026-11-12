//! Local chrome preferences; defaults preserve the original toolbar and pane decoration.
use serde::{Deserialize, Serialize};

/// Unknown future modes safely keep the toolbar visible.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolbarBehavior {
    OnHover,
    Hidden,
    #[default]
    #[serde(other)]
    Always,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InterfaceConfig {
    pub toolbar: ToolbarBehavior,
    /// Share spare toolbar tab-track width; sidebar rows keep their existing layout.
    pub tabs_fill_available_space: bool,
    /// Maximum distinct cached program glyphs per tab (0–5); one preserves legacy behavior.
    pub tab_icon_count: u8,
    /// None keeps the legacy `pane_padding` on all four sides. With a value here,
    /// `pane_padding` is horizontal only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pane_padding_vertical: Option<f32>,
    /// None follows density: Comfortable cards are outlined, Compact panes are not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight_active_pane: Option<bool>,
    /// Local header name and enabled-button fading only; never terminal content.
    pub fade_inactive_pane_headers: bool,
    pub fade_pane_headers_in_inactive_window: bool,
}

impl Default for InterfaceConfig {
    fn default() -> Self {
        Self {
            toolbar: ToolbarBehavior::default(),
            tabs_fill_available_space: false,
            tab_icon_count: 1,
            pane_padding_vertical: None,
            highlight_active_pane: None,
            fade_inactive_pane_headers: false,
            fade_pane_headers_in_inactive_window: false,
        }
    }
}

impl InterfaceConfig {
    pub const MAX_TAB_ICONS: u8 = 5;

    pub(super) fn normalize(&mut self) {
        self.tab_icon_count = self.tab_icon_count.min(Self::MAX_TAB_ICONS);
        self.pane_padding_vertical = self
            .pane_padding_vertical
            .filter(|padding| padding.is_finite())
            .map(|padding| padding.clamp(0., 32.));
    }
}

impl super::Settings {
    /// Content padding as (horizontal, vertical), including old single-value files.
    pub fn padding(&self) -> (f32, f32) {
        (
            self.pane_padding,
            self.interface
                .pane_padding_vertical
                .unwrap_or(self.pane_padding),
        )
    }

    /// The UI edits one axis without moving the other. Old files still follow the
    /// single value until one of these independent controls is used.
    pub fn step_padding(&mut self, vertical: bool, step: f32) {
        let (horizontal, ends) = self.padding();
        if vertical {
            self.interface.pane_padding_vertical = Some((ends + step).clamp(0., 32.));
        } else {
            self.interface.pane_padding_vertical = Some(ends);
            self.pane_padding = (horizontal + step).clamp(0., 32.);
        }
    }

    pub fn highlights_active_pane(&self) -> bool {
        self.interface
            .highlight_active_pane
            .unwrap_or(self.pane_density == super::PaneDensity::Comfortable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use serde_json::json;

    #[test]
    fn old_and_unknown_toolbar_settings_keep_the_original_row() {
        for value in [
            json!({}),
            json!({"interface": {}}),
            json!({"interface": {"toolbar": "left_side"}}),
        ] {
            let settings: Settings = serde_json::from_value(value).unwrap();
            assert_eq!(settings.interface.toolbar, ToolbarBehavior::Always);
        }
    }

    #[test]
    fn pane_defaults_legacy_files_roundtrip_and_independent_steps() {
        for value in [
            json!({}),
            json!({"pane_padding": 12, "pane_density": "compact"}),
        ] {
            let mut settings: Settings = serde_json::from_value(value).unwrap();
            assert_eq!(settings.interface, InterfaceConfig::default());
            assert_eq!(
                settings.padding(),
                (settings.pane_padding, settings.pane_padding)
            );
            assert_eq!(
                settings.highlights_active_pane(),
                settings.pane_density == crate::settings::PaneDensity::Comfortable
            );
            let encoded = serde_json::to_value(&settings).unwrap();
            assert!(encoded["interface"].get("pane_padding_vertical").is_none());
            assert!(encoded["interface"].get("highlight_active_pane").is_none());
            settings.pane_padding = 10.;
            assert_eq!(
                settings.padding(),
                (10., 10.),
                "legacy changes still move both axes"
            );
            settings.step_padding(false, 1.);
            assert_eq!(settings.padding(), (11., 10.), "UI pins the other axis");
            settings.step_padding(true, -1.);
            assert_eq!(settings.padding(), (11., 9.));
            for choice in [None, Some(false), Some(true)] {
                settings.interface.highlight_active_pane = choice;
                let back: Settings =
                    serde_json::from_value(serde_json::to_value(&settings).unwrap()).unwrap();
                assert_eq!(back.interface, settings.interface);
                assert_eq!(back.padding(), (11., 9.));
            }
        }
    }

    #[test]
    fn padding_file_bounds_and_steps_keep_zero_and_thirty_two() {
        let path = std::env::temp_dir().join(format!("rex-m27-bounds-{}.json", std::process::id()));
        for (value, expected) in [(-5., 0.), (0., 0.), (4.5, 4.5), (32., 32.), (55., 32.)] {
            let mut settings = Settings {
                pane_padding: value,
                ..Settings::default()
            };
            settings.interface.pane_padding_vertical = Some(value);
            settings.save(&path).unwrap();
            let mut back = Settings::load(&path).unwrap();
            assert_eq!(back.padding(), (expected, expected));
            back.step_padding(false, -100.);
            back.step_padding(true, 100.);
            assert_eq!(back.padding(), (0., 32.));
            back.step_padding(false, -1.);
            back.step_padding(true, 1.);
            assert_eq!(back.padding(), (0., 32.));
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn header_fading_defaults_serde_and_save_load() {
        for value in [
            json!({}),
            json!({"interface": {}}),
            json!({
                "pane_padding": 12, "interface": {"highlight_active_pane": true}
            }),
        ] {
            let settings: Settings = serde_json::from_value(value).unwrap();
            assert!(!settings.interface.fade_inactive_pane_headers);
            assert!(!settings.interface.fade_pane_headers_in_inactive_window);
        }
        let path =
            std::env::temp_dir().join(format!("rex-header-fade-{}.json", std::process::id()));
        for pane in [false, true] {
            for window in [false, true] {
                let mut settings = Settings::default();
                settings.interface.fade_inactive_pane_headers = pane;
                settings.interface.fade_pane_headers_in_inactive_window = window;
                let encoded = serde_json::to_value(&settings).unwrap();
                assert_eq!(encoded["interface"]["fade_inactive_pane_headers"], pane);
                assert_eq!(
                    encoded["interface"]["fade_pane_headers_in_inactive_window"],
                    window
                );
                let back: Settings = serde_json::from_value(encoded).unwrap();
                assert_eq!(back.interface, settings.interface);
                settings.save(&path).unwrap();
                assert_eq!(Settings::load(&path).unwrap().interface, settings.interface);
            }
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn tab_fill_defaults_serde_and_disk_roundtrip() {
        for value in [
            json!({}),
            json!({"interface": {}}),
            json!({"vertical_tabs": true}),
        ] {
            let settings: Settings = serde_json::from_value(value).unwrap();
            assert!(!settings.interface.tabs_fill_available_space);
        }
        let path = std::env::temp_dir().join(format!("rex-tab-fill-{}.json", std::process::id()));
        for fill in [true, false] {
            let mut settings = Settings::default();
            settings.interface.tabs_fill_available_space = fill;
            settings.vertical_tabs = true;
            let encoded = serde_json::to_value(&settings).unwrap();
            assert_eq!(encoded["interface"]["tabs_fill_available_space"], fill);
            let back: Settings = serde_json::from_value(encoded).unwrap();
            assert_eq!(back.interface, settings.interface);
            settings.save(&path).unwrap();
            let back = Settings::load(&path).unwrap();
            assert_eq!(back.interface, settings.interface);
            assert!(back.vertical_tabs);
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn tab_icon_defaults_partial_files_bounds_and_disk_roundtrip() {
        for value in [
            json!({}),
            json!({"interface": {}}),
            json!({"interface": {"tabs_fill_available_space": true}}),
        ] {
            let settings: Settings = serde_json::from_value(value).unwrap();
            assert_eq!(settings.interface.tab_icon_count, 1);
        }
        let path = std::env::temp_dir().join(format!("rex-tab-icons-{}.json", std::process::id()));
        for count in [0, 1, 2, 3, 4, 5, 6, 255] {
            let mut settings = Settings::default();
            settings.interface.tab_icon_count = count;
            settings.interface.tabs_fill_available_space = true;
            settings.save(&path).unwrap();
            let back = Settings::load(&path).unwrap();
            assert_eq!(back.interface.tab_icon_count, count.min(5));
            assert!(back.interface.tabs_fill_available_space);
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn toolbar_modes_persist() {
        let path = std::env::temp_dir().join(format!(
            "rex-toolbar-persistence-{}.json",
            std::process::id()
        ));
        for (toolbar, word) in [
            (ToolbarBehavior::Always, "always"),
            (ToolbarBehavior::OnHover, "on_hover"),
            (ToolbarBehavior::Hidden, "hidden"),
        ] {
            let mut settings = Settings::default();
            settings.interface.toolbar = toolbar;
            settings.save(&path).unwrap();
            assert_eq!(Settings::load(&path).unwrap().interface.toolbar, toolbar);
            assert_eq!(
                serde_json::to_value(&settings).unwrap()["interface"]["toolbar"],
                word
            );
        }
        std::fs::remove_file(path).unwrap();
    }
}
