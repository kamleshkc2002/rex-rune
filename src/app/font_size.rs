//! Terminal font size as actions: one point at a time within the 8–48 pt the setting
//! allows, or back to the default. Interface zoom is separate and keeps `Ctrl =/-/0`.
use super::{
    panel::{Entry, Intent},
    workspace::Workspace,
};
use rex_linux::settings::Settings;
use serde_json::Value;

pub const INCREASE: &str = "client.font.increase";
pub const DECREASE: &str = "client.font.decrease";
pub const RESET: &str = "client.font.reset";

const RANGE: (f32, f32) = (8., 48.);

fn default_size() -> f32 {
    Settings::default().font_size
}

/// The size a step leads to: one point up or down, or the default for a step of zero.
fn stepped(size: f32, step: i32) -> f32 {
    if step == 0 {
        default_size()
    } else {
        (size + step as f32).clamp(RANGE.0, RANGE.1)
    }
}

/// What the notice says about `size`, with the limit or default it has reached.
fn notice(size: f32) -> String {
    let suffix = if size >= RANGE.1 {
        " · maximum"
    } else if size <= RANGE.0 {
        " · minimum"
    } else if size == default_size() {
        " · default"
    } else {
        ""
    };
    format!("Terminal font {size} pt{suffix}")
}

impl Workspace {
    /// Step every terminal's font size and say the result in one notice, which replaces the
    /// last size notice. At a limit nothing changes and the notice says so.
    pub fn step_font_size(&mut self, step: i32) {
        let size = stepped(self.settings.font_size, step);
        if size != self.settings.font_size {
            self.change_font(size - self.settings.font_size);
        }
        self.size_notice(notice(self.settings.font_size));
    }

    /// Commands' rows for the terminal font size; a step that would change nothing is
    /// disabled.
    pub(super) fn font_commands(&self) -> Vec<Entry> {
        let size = self.settings.font_size;
        [
            ("Increase terminal font size", INCREASE, 1),
            ("Decrease terminal font size", DECREASE, -1),
            ("Reset terminal font size", RESET, 0),
        ]
        .into_iter()
        .map(|(label, action, step)| {
            let mut entry = Entry::new(
                label,
                "",
                "Workspace",
                Intent::ClientAction(action, Value::Null),
            );
            entry.enabled = stepped(size, step) != size;
            entry
        })
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_stay_in_range_and_zero_is_the_default() {
        assert_eq!(stepped(15., 1), 16.);
        assert_eq!(stepped(15., -1), 14.);
        assert_eq!(stepped(48., 1), 48.);
        assert_eq!(stepped(8., -1), 8.);
        assert_eq!(stepped(31., 0), default_size());
    }

    #[test]
    fn the_notice_names_limits_and_the_default() {
        assert_eq!(notice(16.), "Terminal font 16 pt");
        assert_eq!(notice(48.), "Terminal font 48 pt · maximum");
        assert_eq!(notice(8.), "Terminal font 8 pt · minimum");
        assert_eq!(
            notice(default_size()),
            format!("Terminal font {} pt · default", default_size())
        );
    }
}
