//! Settings → Terminal → Titles: whether a tab or pane without a name of its own is called by
//! its foreground program and that program's directory (`vim - ~/rex`) in place of a title
//! its shell set. What is generated, and which titles it replaces, is decided in
//! `rex_rune::connection::titles`; every place that names a tab or pane passes the
//! preference to `activity::tab_name` or `activity::pane_name`.
use super::{
    settings_kit::{card, footnote, section, setting},
    theme::kit,
    workspace::Workspace,
};
use rune::{El, Tx};
use rune_kit::Switch;

/// The Titles section of the Terminal page.
pub fn titles(s: &Workspace) -> El {
    section(
        "Titles",
        [
            card([setting(
                "Prefer generated titles",
                "",
                Switch::new(s.generated_titles())
                    .label("Prefer generated titles")
                    .hide_label()
                    .on_toggle_tx(|s: &mut Workspace, tx: &mut Tx| {
                        s.prefer_generated_titles(!s.generated_titles(), tx)
                    })
                    .build(kit())
                    .key("generated-titles"),
            )]),
            footnote(
                "Show the running program and its directory, such as vim - ~/rex, instead of titles that shells set. Titles that programs set for themselves, and names you give tabs and panes, still appear.",
            ),
        ],
    )
}

impl Workspace {
    /// Whether tabs and panes without a name of their own show a generated title.
    pub fn generated_titles(&self) -> bool {
        self.settings.generated_titles
    }

    /// Turn generated titles on or off, saved at once. Tabs and panes are named as they are
    /// drawn; the window's title is kept, so it is named again here.
    pub fn prefer_generated_titles(&mut self, on: bool, tx: &mut Tx) {
        self.settings.generated_titles = on;
        self.save();
        self.update_title(tx);
    }
}
