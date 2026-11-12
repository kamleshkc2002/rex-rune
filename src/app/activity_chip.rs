//! A quiet entry point into Activity, not a read receipt or a server inventory.
use super::{
    theme::{danger, warning},
    workspace::Workspace,
};
use rex_rune::connection::activity::ProgramState;

/// Toolbar width below which the chip is not drawn: at 640 pt it would take half the tab
/// track.
const MIN_WIDTH: f32 = 720.;

pub struct Summary {
    pub count: usize,
    pub needs_you: bool,
}

impl Summary {
    pub fn label(&self) -> String {
        match (self.needs_you, self.count) {
            (true, 1) => "1 needs you".into(),
            (true, n) => format!("{n} need you"),
            (false, 1) => "1 error".into(),
            (false, n) => format!("{n} errors"),
        }
    }

    pub fn tone(&self) -> u32 {
        if self.needs_you { warning() } else { danger() }
    }
}

impl Workspace {
    /// The chip as the toolbar draws it. In a narrow window it waits for the briefer chips,
    /// a key sequence in progress and bytes arriving, so the tab strip keeps its room and
    /// the download chip behaves exactly as it does without this one. Near the minimum
    /// window width it would leave the tab strip a sliver, so it is not drawn at all.
    pub fn activity_chip_shown(&self) -> Option<Summary> {
        // The toolbar's own width: interface zoom lays it out narrower than the window.
        let width = self.viewport[0] / self.ui_scale();
        if width < MIN_WIDTH {
            return None;
        }
        let narrow = width < super::downloads_view::CHIP_FULL_WIDTH;
        if narrow
            && (self.bindings.sequence.depth > 0
                || super::downloads_view::chip_label(self).is_some())
        {
            return None;
        }
        self.activity_chip()
    }

    /// What the chip would say: tabs other than the one on screen that wait on the person,
    /// or failing that have errored.
    pub fn activity_chip(&self) -> Option<Summary> {
        if !self.settings.notifications.show_in_tabs {
            return None;
        }
        let (mut waiting, mut errors) = (0, 0);
        for (host, session, window, activity) in self.activity_tabs() {
            // On screen, even when the native window is unfocused. This is not tab_seen:
            // the chip never acknowledges a tab or closes a desktop notification.
            if self.tab_in_front(host, session, window) {
                continue;
            }
            match activity.state {
                ProgramState::Blocked => waiting += 1,
                ProgramState::Error => errors += 1,
                _ => {}
            }
        }
        if waiting > 0 {
            Some(Summary {
                count: waiting,
                needs_you: true,
            })
        } else if errors > 0 {
            Some(Summary {
                count: errors,
                needs_you: false,
            })
        } else {
            None
        }
    }
}
