//! Monochrome cached program icons. No assets, fetching, or theme-dependent policy.
use super::{activity::program_icon, theme::*, widgets::glyph, workspace::Workspace};
use rex_linux::settings::interface::InterfaceConfig;
use rune::{El, Icon, el, row, text};

pub struct TabIcon {
    pub icon: &'static Icon,
    pub program: Option<String>,
}

/// Deduplicate resolved glyphs (vim/nvim, shells/unknowns), not process names.
pub fn icons_of(programs: &[Option<String>], count: u8) -> Vec<TabIcon> {
    let mut icons: Vec<TabIcon> = Vec::new();
    let count = usize::from(count.min(InterfaceConfig::MAX_TAB_ICONS));
    for program in programs {
        if icons.len() == count {
            break;
        }
        let icon = program_icon(program.as_deref());
        if icons.iter().all(|other| other.icon.name != icon.name) {
            icons.push(TabIcon {
                icon,
                program: program.clone(),
            });
        }
    }
    if count > 0 && icons.is_empty() {
        icons.push(TabIcon {
            icon: program_icon(None),
            program: None,
        });
    }
    icons
}

/// Includes the gap before the label; zero icons leave no child and no gap.
pub fn leading_width(count: usize) -> f32 {
    count as f32 * (15. + 8.)
}

/// Append directly to the existing 8pt-gap row: one keeps the original toolbar tree
/// and geometry, and zero has no spacer. Preview and both tab surfaces share this.
pub fn append(mut parent: El, key: &str, icons: &[TabIcon], active: bool) -> El {
    for (index, icon) in icons.iter().enumerate() {
        let mut cell = el()
            .key(if index == 0 {
                key.to_owned()
            } else {
                format!("{key}:{index}")
            })
            .child(glyph(
                icon.icon,
                15.,
                rgb(if active { ink() } else { muted() }),
            ));
        if let Some(program) = &icon.program {
            cell = cell.help(program.clone());
        }
        parent = parent.child(cell);
    }
    parent
}

impl Workspace {
    pub fn step_tab_icons(&mut self, step: i8) {
        self.settings.interface.tab_icon_count = self
            .settings
            .interface
            .tab_icon_count
            .saturating_add_signed(step)
            .min(InterfaceConfig::MAX_TAB_ICONS);
        self.reveal_tab_again = true;
        self.save();
    }
}

/// A deliberately inert sample, not a fabricated server tab. Uses the real glyph
/// resolver and row layout, including the zero-icon case.
pub fn preview(s: &Workspace) -> El {
    let programs = ["zsh", "claude", "nvim", "git", "cargo"].map(|p| Some(p.to_owned()));
    let icons = icons_of(&programs, s.settings.interface.tab_icon_count);
    let pill = append(
        row()
            .h(28.)
            .px(14.)
            .gap(8.)
            .center_y()
            .radius(8.)
            .continuous_corners()
            .fill(white(0.11)),
        "tab-icons-preview-icon",
        &icons,
        true,
    )
    .child(text("Sample tab").font(medium(12.5)).color(rgb(ink())));
    row()
        .key("tab-icons-preview")
        .role("img")
        .label("Sample tab with the chosen icon count")
        .padding(12.)
        .inert()
        .child(pill)
}
