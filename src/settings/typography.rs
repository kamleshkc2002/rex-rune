//! How terminals set their text: the face ordinary text is drawn in, the row height, the
//! space between characters and whether bold and italic are drawn. All of it is local and
//! applies at once; the family and its fallbacks are `Settings::font_family` and
//! `Settings::font_fallbacks`, which apply when the client is reopened.
use serde::{Deserialize, Serialize};

/// The face of the terminal family that ordinary text is drawn in. A value this version
/// doesn't know reads as Regular.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FontStyle {
    Italic,
    Bold,
    BoldItalic,
    #[default]
    #[serde(other)]
    Regular,
}
impl FontStyle {
    /// In the order the control lists them.
    pub const ALL: [FontStyle; 4] = [
        FontStyle::Regular,
        FontStyle::Italic,
        FontStyle::Bold,
        FontStyle::BoldItalic,
    ];
    pub fn name(self) -> &'static str {
        match self {
            FontStyle::Regular => "Regular",
            FontStyle::Italic => "Italic",
            FontStyle::Bold => "Bold",
            FontStyle::BoldItalic => "Bold Italic",
        }
    }
    pub fn bold(self) -> bool {
        matches!(self, FontStyle::Bold | FontStyle::BoldItalic)
    }
    pub fn italic(self) -> bool {
        matches!(self, FontStyle::Italic | FontStyle::BoldItalic)
    }
}

/// Which of the bold and italic attributes programs set are drawn. One that isn't drawn
/// shows in the family's ordinary face. A value this version doesn't know reads as Both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoldItalic {
    BoldOnly,
    ItalicOnly,
    Neither,
    #[default]
    #[serde(other)]
    Both,
}
impl BoldItalic {
    /// In the order the control lists them.
    pub const ALL: [BoldItalic; 4] = [
        BoldItalic::Both,
        BoldItalic::BoldOnly,
        BoldItalic::ItalicOnly,
        BoldItalic::Neither,
    ];
    pub fn name(self) -> &'static str {
        match self {
            BoldItalic::Both => "Both",
            BoldItalic::BoldOnly => "Bold only",
            BoldItalic::ItalicOnly => "Italic only",
            BoldItalic::Neither => "Neither",
        }
    }
    pub fn bold(self) -> bool {
        matches!(self, BoldItalic::Both | BoldItalic::BoldOnly)
    }
    pub fn italic(self) -> bool {
        matches!(self, BoldItalic::Both | BoldItalic::ItalicOnly)
    }
}

/// What a character spacing is counted in. A value this version doesn't know reads as
/// points.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpacingUnit {
    /// Hundredths of the font's own character width.
    Percent,
    /// Points: a pixel at 100% zoom on an ordinary display.
    #[default]
    #[serde(other)]
    Px,
}
impl SpacingUnit {
    /// In the order the control lists them.
    pub const ALL: [SpacingUnit; 2] = [SpacingUnit::Px, SpacingUnit::Percent];
    /// As the Spacing unit list names it. Points are saved as `"px"`, the name the value
    /// had before it was called what it is.
    pub fn name(self) -> &'static str {
        match self {
            SpacingUnit::Px => "Points",
            SpacingUnit::Percent => "Percent",
        }
    }
    /// What follows a spacing counted in it: `+2 pt`, `+10%`.
    pub fn suffix(self) -> &'static str {
        match self {
            SpacingUnit::Px => " pt",
            SpacingUnit::Percent => "%",
        }
    }
    /// The least and the most spacing in this unit, and one step of its control.
    pub fn range(self) -> (i16, i16, i16) {
        match self {
            SpacingUnit::Px => (-3, 12, 1),
            SpacingUnit::Percent => (-30, 100, 5),
        }
    }
}

/// The least and the most a row's height may be adjusted, in percent, and one step.
pub const LINE_HEIGHT: (i16, i16, i16) = (-20, 100, 5);

/// The typography settings of Settings → Terminal and its Advanced font settings page.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Typography {
    pub style: FontStyle,
    /// Percent added to the row height the font size gives; 0 is that height.
    pub line_height: i16,
    /// Space added to (or, negative, taken from) every character's width, in `spacing_unit`.
    pub spacing: i16,
    pub spacing_unit: SpacingUnit,
    pub bold_italic: BoldItalic,
}
impl Typography {
    /// The row height adjustment, within its range whatever the file said.
    pub fn line_height(&self) -> i16 {
        self.line_height.clamp(LINE_HEIGHT.0, LINE_HEIGHT.1)
    }

    /// The character spacing, within its unit's range whatever the file said.
    pub fn spacing(&self) -> i16 {
        let (least, most, _) = self.spacing_unit.range();
        self.spacing.clamp(least, most)
    }

    /// One step of the row height's control, staying in range.
    pub fn step_line_height(&mut self, direction: i16) {
        self.line_height = (self.line_height() + direction.signum() * LINE_HEIGHT.2)
            .clamp(LINE_HEIGHT.0, LINE_HEIGHT.1);
    }

    /// One step of the character spacing's control, staying in its unit's range.
    pub fn step_spacing(&mut self, direction: i16) {
        let (least, most, step) = self.spacing_unit.range();
        self.spacing = (self.spacing() + direction.signum() * step).clamp(least, most);
    }

    /// Count the spacing in `unit` from now on. The number is kept where the new unit
    /// allows it, so a change of unit made by mistake is undone by changing it back.
    pub fn set_spacing_unit(&mut self, unit: SpacingUnit) {
        self.spacing_unit = unit;
        self.spacing = self.spacing();
    }

    /// The spacing in points for a font whose characters are `advance` points wide.
    pub fn spacing_points(&self, advance: f32) -> f32 {
        match self.spacing_unit {
            SpacingUnit::Px => f32::from(self.spacing()),
            SpacingUnit::Percent => advance * f32::from(self.spacing()) / 100.,
        }
    }

    /// What multiplies the row height the font size gives.
    pub fn line_scale(&self) -> f32 {
        1. + f32::from(self.line_height()) / 100.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    #[test]
    fn a_file_from_before_these_settings_loads_with_their_defaults() {
        let old: Settings =
            serde_json::from_str(r#"{"font_family": "Hack", "terminal": {"cursor_style": "bar"}}"#)
                .unwrap();
        let font = old.terminal.font;
        assert_eq!(font, Typography::default());
        assert_eq!(font.style, FontStyle::Regular);
        assert_eq!((font.line_height, font.spacing), (0, 0));
        assert_eq!(font.spacing_unit, SpacingUnit::Px);
        assert_eq!(font.bold_italic, BoldItalic::Both);
        assert_eq!(font.line_scale(), 1.);
        assert_eq!(font.spacing_points(9.), 0.);
    }

    #[test]
    fn a_partial_font_group_preserves_cursor_selection_and_top_level_keys() {
        let settings: Settings = serde_json::from_str(
            r#"{
            "font_family":"Joe's Mono", "font_fallbacks":["Fallback One","Fallback Two"],
            "font_size":17, "generated_titles":true,
            "terminal":{"font":{"line_height":20}, "cursor_style":"bar",
                "cursor_force":true, "primary_selection":true, "copy_on_selection":true}
        }"#,
        )
        .unwrap();
        let font = &settings.terminal.font;
        assert_eq!(font.line_height, 20);
        assert_eq!(font.style, FontStyle::Regular);
        assert_eq!(font.bold_italic, BoldItalic::Both);
        assert_eq!(font.spacing, 0);
        assert!(
            settings.terminal.cursor_force
                && settings.terminal.primary_selection
                && settings.terminal.copy_on_selection
        );
        let value = serde_json::to_value(settings).unwrap();
        assert_eq!(value["font_family"], "Joe's Mono");
        assert_eq!(
            value["font_fallbacks"],
            serde_json::json!(["Fallback One", "Fallback Two"])
        );
        assert_eq!(value["font_size"], 17.);
        assert_eq!(value["generated_titles"], true);
        assert!(value["terminal"].get("font_family").is_none());
    }

    #[test]
    fn unknown_values_and_numbers_out_of_range_still_load() {
        let newer: Settings = serde_json::from_str(
            r#"{"terminal": {"font": {"style": "condensed", "bold_italic": "underline",
                "spacing_unit": "em", "line_height": 900, "spacing": -40}}}"#,
        )
        .unwrap();
        let font = newer.terminal.font;
        assert_eq!(font.style, FontStyle::Regular);
        assert_eq!(font.bold_italic, BoldItalic::Both);
        assert_eq!(font.spacing_unit, SpacingUnit::Px);
        // The file's numbers are kept as written and read within range.
        assert_eq!((font.line_height(), font.spacing()), (100, -3));
        assert_eq!(font.line_scale(), 2.);
    }

    #[test]
    fn steps_stay_in_range_and_a_unit_change_keeps_the_number_it_can() {
        let mut font = Typography::default();
        for _ in 0..4 {
            font.step_line_height(1);
        }
        assert_eq!(font.line_height, 20);
        for _ in 0..40 {
            font.step_line_height(1);
        }
        assert_eq!(font.line_height, 100);
        for _ in 0..40 {
            font.step_line_height(-1);
        }
        assert_eq!(font.line_height, -20);

        font.step_spacing(1);
        font.step_spacing(1);
        assert_eq!((font.spacing, font.spacing_unit), (2, SpacingUnit::Px));
        assert_eq!(font.spacing_points(9.), 2.);
        font.set_spacing_unit(SpacingUnit::Percent);
        assert_eq!(font.spacing, 2);
        font.step_spacing(1);
        assert_eq!(font.spacing, 7);
        assert!((font.spacing_points(10.) - 0.7).abs() < 1e-6);
        font.spacing = 60;
        font.set_spacing_unit(SpacingUnit::Px);
        assert_eq!(font.spacing, 12, "the most the unit allows");
        for _ in 0..40 {
            font.step_spacing(-1);
        }
        assert_eq!(font.spacing, -3);
    }

    #[test]
    fn every_choice_round_trips_through_a_saved_file() {
        let path = std::env::temp_dir().join(format!("rex-typography-{}.json", std::process::id()));
        for style in FontStyle::ALL {
            for bold_italic in BoldItalic::ALL {
                for spacing_unit in SpacingUnit::ALL {
                    let font = Typography {
                        style,
                        line_height: 20,
                        spacing: -2,
                        spacing_unit,
                        bold_italic,
                    };
                    let mut settings = Settings::default();
                    settings.terminal.font = font.clone();
                    settings.save(&path).unwrap();
                    assert_eq!(Settings::load(&path).unwrap().terminal.font, font);
                }
            }
        }
        std::fs::remove_file(path).ok();
    }
}
