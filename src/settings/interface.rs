//! Local toolbar visibility; defaults preserve the original working row.
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct InterfaceConfig {
    pub toolbar: ToolbarBehavior,
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
