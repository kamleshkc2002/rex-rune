//! The font picker of Settings → Terminal: the families fontconfig knows, searched as you
//! type, as Monospaced or All fonts. It chooses the terminal font, where the current family
//! is checked, and adds fallback fonts, where the families already used are.
//!
//! The picker is a list panel (`PanelKind::Fonts`) so the list keys, Enter and Escape work as
//! in every other list, but it draws itself: a machine can have thousands of families, so its
//! rows are built for the part of the list on screen only (`El::virtual_rows`).
use super::{
    fonts::{self, Family},
    input::list_key,
    keycaps::keycap,
    panel::{Entry, Intent, PanelKind},
    popups, settings_kit, settings_view,
    theme::*,
    widgets::{Row, list_row},
    workspace::Workspace,
};
use rex_rune::{
    connection::Options,
    settings::{Settings, normalize_font_preferences},
};
use rune::{Cursor, El, GrowthSpec, Key, KeyEvent, ScrollSpec, Shadow, Tx, col, el, row, text};
use rune_kit::TextInput;

/// What a font chosen in the picker is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontPurpose {
    /// The terminal font (`Settings::font_family`).
    Terminal,
    /// One more of the fallback fonts (`Settings::font_fallbacks`).
    Fallback,
}

/// An open font picker: what it chooses, and whether it lists monospaced families only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontPick {
    pub purpose: FontPurpose,
    pub mono: bool,
}

/// What the font settings keep beside the saved preferences.
pub struct FontState {
    /// The families a picker lists: read from fontconfig the first time one is needed.
    pub catalog: Option<Vec<Family>>,
    /// The catalog's families the open picker lists, in order: those of its scope that
    /// match its search.
    shown: Vec<usize>,
    /// The fallback fonts `--font-fallback` gave this launch, which it uses in place of the
    /// saved list.
    pub launch_fallbacks: Option<Vec<String>>,
    /// The saved fallback list when the client started, to tell a pending change from the
    /// list in use.
    pub saved_fallbacks: Vec<String>,
}
impl FontState {
    pub fn new(options: &Options, settings: &Settings) -> Self {
        Self {
            catalog: None,
            shown: Vec::new(),
            launch_fallbacks: (!options.font_fallback.is_empty())
                .then(|| options.font_fallback.clone()),
            saved_fallbacks: settings.font_fallbacks.clone(),
        }
    }

    /// Whether `family` is installed (or bundled), once the catalog has been read.
    pub fn installed(&self, family: &str) -> Option<bool> {
        let lower = family.to_lowercase();
        if ["monospace", "sans-serif", "serif"].contains(&lower.as_str()) {
            return None;
        }
        self.catalog
            .as_ref()
            .map(|catalog| catalog.iter().any(|f| f.lower == lower))
    }
}

/// The families the headless tests list in place of this machine's: `fc-list` output, as
/// `fonts::parse_catalog` reads it.
#[cfg(test)]
pub const TEST_FAMILIES: &str = "Noto Sans\t\nHack Nerd Font Mono\t100\nNoto Color Emoji\t\n\
    JetBrainsMono Nerd Font\t100\nNoto Sans CJK JP\t\nDejaVu Sans Mono\t100\n\
    Symbols Nerd Font\t\nHack Nerd Font Mono\t100\n";

/// The picker's width, and the height of one of its rows with the gap under it.
const WIDTH: f32 = 320.;
const ROW: f32 = 29.;
/// At most this many rows show at once; the rest scroll.
const ROWS: f32 = 10.5;
const FIELD: f32 = 34.;
const SCOPE: f32 = 26.;
const SAMPLE: f32 = 52.;

impl Workspace {
    /// The families a picker can list, read from fontconfig on first use.
    pub fn font_catalog(&mut self) -> &[Family] {
        #[cfg(not(test))]
        let read = || fonts::installed().to_vec();
        // Tests list the same families on every machine.
        #[cfg(test)]
        let read = || fonts::parse_catalog(TEST_FAMILIES);
        self.fonts.catalog.get_or_insert_with(read)
    }

    fn font_pick(&self) -> Option<FontPick> {
        match self.panel.as_ref()?.kind {
            PanelKind::Fonts(pick) => Some(pick),
            _ => None,
        }
    }

    /// Whether the picker marks `family`: the terminal font, or a fallback already added.
    fn font_marked(&self, purpose: FontPurpose, family: &str) -> bool {
        match purpose {
            FontPurpose::Terminal => self.settings.font_family.eq_ignore_ascii_case(family),
            FontPurpose::Fallback => self
                .settings
                .font_fallbacks
                .iter()
                .any(|f| f.eq_ignore_ascii_case(family)),
        }
    }

    /// Open the picker for `purpose`. The terminal font's opens on Monospaced, unless the
    /// saved family is installed and isn't one, so the font in use is always in the list it
    /// opens on; a fallback's opens on All fonts, since fallbacks are mostly symbol, emoji
    /// and other scripts' fonts.
    pub(super) fn open_fonts(&mut self, purpose: FontPurpose) {
        let saved = self.settings.font_family.to_lowercase();
        let proportional = self
            .font_catalog()
            .iter()
            .any(|f| f.lower == saved && !f.mono);
        let pick = FontPick {
            purpose,
            mono: purpose == FontPurpose::Terminal && !proportional,
        };
        let title = match purpose {
            FontPurpose::Terminal => "Terminal font",
            FontPurpose::Fallback => "Add fallback font",
        };
        self.open_panel(PanelKind::Fonts(pick), title, &[("", "Search fonts", "")]);
        self.list_fonts();
    }

    /// Work out what the open picker lists, and put its selection on the marked family, or
    /// on the first one it can choose.
    fn list_fonts(&mut self) {
        let Some(pick) = self.font_pick() else { return };
        let query = self.query().to_lowercase();
        let catalog = self.fonts.catalog.as_deref().unwrap_or_default();
        self.fonts.shown = catalog
            .iter()
            .enumerate()
            .filter(|(_, f)| (!pick.mono || f.mono) && f.lower.contains(&query))
            .map(|(index, _)| index)
            .collect();
        let entries = self.font_entries(pick);
        let selected = entries
            .iter()
            .position(|e| e.current)
            .filter(|_| pick.purpose == FontPurpose::Terminal)
            .or_else(|| entries.iter().position(|e| e.enabled))
            .unwrap_or(0);
        if let Some(panel) = &mut self.panel {
            panel.selected = selected;
            panel.error = None;
            panel.reveal = true;
        }
    }

    /// The picker's rows, as every list panel has them. A fallback's picker can't choose
    /// the terminal font or a family that is a fallback already, and says why.
    pub(super) fn font_entries(&self, pick: FontPick) -> Vec<Entry> {
        let catalog = self.fonts.catalog.as_deref().unwrap_or_default();
        self.fonts
            .shown
            .iter()
            .filter_map(|index| catalog.get(*index))
            .map(|family| {
                let mut entry = Entry::new(
                    family.name.clone(),
                    "",
                    "",
                    Intent::Font(family.name.clone()),
                );
                entry.current = self.font_marked(pick.purpose, &family.name);
                if pick.purpose == FontPurpose::Fallback {
                    if entry.current {
                        entry.detail = "Added".into();
                        entry.enabled = false;
                    } else if self.settings.font_family.eq_ignore_ascii_case(&family.name) {
                        entry.detail = "Terminal font".into();
                        entry.enabled = false;
                    }
                }
                entry
            })
            .collect()
    }

    /// List monospaced families only, or every family.
    pub fn set_font_scope(&mut self, mono: bool) {
        if let Some(panel) = &mut self.panel
            && let PanelKind::Fonts(pick) = &mut panel.kind
            && pick.mono != mono
        {
            pick.mono = mono;
            self.list_fonts();
        }
    }

    /// The picker's search changed.
    pub fn font_query_changed(&mut self) {
        self.list_fonts();
    }

    /// Take `family` from the open picker: the terminal font from now on, or one more
    /// fallback. Either is saved at once and used when the client is reopened; the picker
    /// closes, back to the Settings page it was opened from.
    pub(super) fn pick_font(&mut self, family: String, tx: &mut Tx) {
        let Some(pick) = self.font_pick() else { return };
        let mut fallbacks = self.settings.font_fallbacks.clone();
        let primary = match pick.purpose {
            FontPurpose::Terminal => family.clone(),
            FontPurpose::Fallback => {
                fallbacks.push(family.clone());
                self.settings.font_family.clone()
            }
        };
        // The same rules as a settings file: one family per name, a fallback once, and
        // never the terminal font again.
        let (primary, fallbacks) = match normalize_font_preferences(&primary, &fallbacks) {
            Ok(fonts) => fonts,
            Err(error) => {
                self.notify(format!("{error:#}"));
                return;
            }
        };
        let changed = primary != self.settings.font_family;
        self.settings.font_family = primary;
        self.settings.font_fallbacks = fallbacks;
        self.save();
        match self.settings_back() {
            Some(back) => self.panel = Some(back),
            None => self.close_panel(tx),
        }
        if changed && self.settings.font_family != self.launch_saved_font {
            self.inform(format!("{family} saved. Reopen the client to use it."));
        }
    }
}

/// Keys the picker's search field takes: Tab switches between Monospaced and All fonts,
/// Page Up and Page Down move a screenful, and the rest are a list's.
fn picker_key(s: &mut Workspace, ev: &KeyEvent, tx: &mut Tx) -> bool {
    match ev.key {
        Key::Tab => {
            if let Some(pick) = s.font_pick() {
                s.set_font_scope(!pick.mono);
            }
            true
        }
        Key::PageUp | Key::PageDown => {
            let step = if ev.key == Key::PageUp { -10 } else { 10 };
            let last = s.fonts.shown.len().saturating_sub(1) as isize;
            if let Some(panel) = &mut s.panel {
                panel.selected = (panel.selected as isize + step).clamp(0, last) as usize;
                panel.reveal = true;
            }
            tx.instant();
            true
        }
        _ => list_key(s, ev, tx),
    }
}

/// Monospaced | All fonts. Not a Tab stop: the search field keeps the keyboard, and Tab
/// switches between the two.
fn scope_switch(mono: bool, width: f32) -> El {
    let k = kit();
    let c = &k.c;
    let each = width / 2.;
    let hairline = |color| Shadow {
        spread: 0.5,
        color,
        ..Default::default()
    };
    let mut switch = row()
        .key("font-scope")
        .role("radiogroup")
        .label("Fonts listed")
        .help("Tab switches between Monospaced and All fonts")
        .w(width)
        .h(SCOPE)
        .mb(4.)
        .radius(k.radius.sm)
        .fill(c.layer_01)
        .box_shadow(hairline(c.border_base))
        .child(
            el().key("indicator")
                .absolute(if mono { 0. } else { each }, 0.)
                .size(each, SCOPE)
                .radius(k.radius.sm)
                .fill(c.bg_base)
                .box_shadow(hairline(c.border_strong))
                .motion(k.motion.control),
        );
    for (label, key, scope) in [
        ("Monospaced", "font-scope:mono", true),
        ("All fonts", "font-scope:all", false),
    ] {
        let on = scope == mono;
        let mut segment = row()
            .key(key)
            .role("radio")
            .label(label)
            .checked(on)
            .w(each)
            .h(SCOPE)
            .center()
            .radius(k.radius.sm)
            .tab_index(-1)
            .preserve_focus_on_press(true)
            .on_press(move |s: &mut Workspace| s.set_font_scope(scope))
            .child(
                text(label)
                    .font(if on { medium(12.5) } else { ui(12.5) })
                    .color(if on { c.text_base } else { c.text_muted }),
            );
        if !on {
            segment = segment
                .cursor(Cursor::Pointer)
                .hover(c.hover)
                .press(c.pressed);
        }
        switch = switch.child(segment);
    }
    switch
}

/// Row `index` of the open picker. Built only while it is on screen or near it.
fn font_row(s: &Workspace, index: usize) -> El {
    let (Some(panel), Some(pick)) = (s.panel.as_ref(), s.font_pick()) else {
        return el();
    };
    let catalog = s.fonts.catalog.as_deref().unwrap_or_default();
    let Some(family) = s.fonts.shown.get(index).and_then(|i| catalog.get(*i)) else {
        return el();
    };
    let marked = s.font_marked(pick.purpose, &family.name);
    let primary = s.settings.font_family.eq_ignore_ascii_case(&family.name);
    let (detail, enabled) = match pick.purpose {
        FontPurpose::Terminal => ("", true),
        FontPurpose::Fallback if marked => ("Added", false),
        FontPurpose::Fallback if primary => ("Terminal font", false),
        FontPurpose::Fallback => ("", true),
    };
    let name = family.name.clone();
    // The gap between rows is the row's own, so every row is the height the list counts on.
    el().w_full().h(ROW).child(list_row(
        Row {
            key: format!("font:{index}"),
            surface: "fonts",
            activity: None,
            label: &family.name,
            detail,
            tone: None,
            keys: &[],
            height: ROW - 1.,
            check: Some(marked),
            followup: false,
            enabled,
            danger: false,
            selected: index == panel.selected && enabled,
            actions: None,
            columns: None,
        },
        index,
        move |s: &mut Workspace, tx: &mut Tx| s.perform(Intent::Font(name.clone()), tx),
    ))
}

/// The picker, against the control it was opened from and over the Settings page, which
/// stays where it was and takes no input until the picker closes.
pub fn popup(s: &Workspace, viewport: [f32; 2]) -> El {
    let k = kit();
    let panel = s.panel.as_ref().expect("font picker");
    let PanelKind::Fonts(pick) = panel.kind else {
        return popups::overlay();
    };
    let [vw, vh] = viewport;
    let width = WIDTH.min(vw - 16.);
    let inner = width - 10.;
    // Where the window shows the control now, however far its page has scrolled.
    let control = panel
        .anchor_to
        .as_deref()
        .and_then(rune::memo::window_layout);
    let head = FIELD + 3. + SCOPE + 4. + 12. + SAMPLE;
    // Over Settings the picker stays under the dialog's header, so the page's title and
    // Close are never behind it.
    let ceiling = settings_kit::under(panel)
        .and_then(|_| rune::memo::window_layout("settings-header"))
        .map_or(8., |[_, y, _, h]| y + h);
    let room = control.map(|[_, y, _, h]| {
        (
            vh - (y + h + popups::CONTROL_GAP) - 8.,
            y - popups::CONTROL_GAP - ceiling,
        )
    });
    let count = s.fonts.shown.len();
    let content = count as f32 * ROW;
    // Under its control wherever the whole list or six rows of it fit there, as every other
    // list opens; over it only where that side has more room. The list scrolls, so it is
    // as tall as the side it is on allows and no taller.
    let wanted = content.clamp(48., ROWS * ROW).min(6. * ROW);
    let over = room.is_some_and(|(below, above)| below - head < wanted && above > below);
    let list_max = match room {
        Some((_, above)) if over => above - head,
        Some((below, _)) => below - head,
        None => vh - 76. - head - 16.,
    }
    .clamp(ROW, ROWS * ROW);
    let view_h = content.min(list_max);
    let overflow = content > list_max + 1.;

    let mut surface = glass(
        col()
            .key("menu-surface")
            .role("menu")
            .label(panel.title.clone())
            .w(width)
            .padding(5.)
            .gap(1.)
            .preserve_focus_on_press(true),
        12.,
        true,
    )
    .clip_rounded(12.)
    .transition(pop_in())
    .child(
        row()
            .w_full()
            .h(FIELD)
            .mb(3.)
            .px(2.)
            .center_y()
            .radius(8.)
            .continuous_corners()
            .fill(white(0.045))
            .child(
                TextInput::new(&panel.inputs[0])
                    .placeholder("Search fonts")
                    .active_descendant(Some(format!("entry:{}", panel.selected)))
                    .leading_icon(&SEARCH)
                    .leading_icon_size(14.)
                    .text_style(ui(13.))
                    .height(FIELD - 2.)
                    .plain()
                    .trailing(keycap("Esc", false))
                    .on_edit(|s: &mut Workspace| s.edit_input(0))
                    .on_change(|s: &mut Workspace, _: &mut Tx| s.font_query_changed())
                    .on_key(picker_key)
                    .width(None)
                    .build(k)
                    .key("popup-search")
                    .w_full(),
            ),
    )
    .child(scope_switch(pick.mono, inner));

    if count == 0 {
        let none = match (pick.mono, s.query().is_empty()) {
            (true, false) => "No monospaced fonts match. Tab lists all fonts.",
            (true, true) => "No monospaced fonts are installed. Tab lists all fonts.",
            (false, _) => "No fonts match.",
        };
        surface = surface.child(
            col().w_full().px(12.).py(14.).child(
                text(none)
                    .key("font-none")
                    .font(ui(13.))
                    .color(rgb(muted()))
                    .wrap(),
            ),
        );
    } else {
        let mut scroller = col()
            .key("results-scroll")
            .w_full()
            .h(view_h)
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .scroll_edge_fade(10.)
            .child(
                col()
                    .key("results")
                    .w_full()
                    .pr(4.)
                    .growth(GrowthSpec::default())
                    .virtual_rows(
                        count,
                        ROW,
                        |_: &Workspace, index| format!("entry:{index}").into(),
                        font_row,
                    ),
            );
        let mut results = el().w_full();
        if overflow {
            scroller =
                scroller.on_scroll_offset(s.switcher_fx.watch(inner, view_h, |_, _| String::new()));
            results = results.child(scroller).child(s.switcher_fx.thumb_el());
        } else {
            results = results.child(scroller);
        }
        surface = surface.child(results);
    }

    if let Some(family) = s
        .fonts
        .shown
        .get(panel.selected)
        .and_then(|index| s.fonts.catalog.as_ref()?.get(*index))
    {
        // Only the highlighted family is shaped, not every installed file or list row.
        // Rune resolves Named families lazily through the platform font collection.
        surface = surface.child(
            col()
                .key("font-sample")
                .w_full()
                .h(SAMPLE)
                .px(12.)
                .py(5.)
                .clip_rounded(0.)
                .child(
                    text("Requested face · unavailable glyphs may fall back")
                        .font(ui(10.))
                        .color(rgb(muted()))
                        .truncate(),
                )
                .child(
                    text(format!("{}  Aa 0O 1il", family.name))
                        .key("font-family-sample")
                        .font(rune::TextStyle::new(16., 400.).family(fonts::named(&family.name)))
                        .color(rgb(ink()))
                        .no_wrap(),
                ),
        );
    } else {
        surface = surface.child(el().h(SAMPLE));
    }

    let mut layer = match settings_kit::under(panel) {
        Some(_) => settings_view::settings_under(s, viewport),
        None => popups::overlay(),
    }
    .child(popups::dismiss_layer());
    // The page under the picker is inert, so its control lost focus when the picker opened.
    // The picker holds the keyboard as a modal, and this empty marker, removed with it,
    // hands focus back to the control.
    if let Some(key) = panel.anchor_to.as_ref().filter(|_| control.is_some()) {
        surface = surface.modal(true, |s: &mut Workspace| s.cancel_panel_quiet());
        layer = layer.child(
            el().key("list-focus-return")
                .absolute(0., 0.)
                .size(0., 0.)
                .focus_return(key.clone()),
        );
    }
    let estimated = head + view_h.max(48.);
    let Some([cx, cy, cw, ch]) = control else {
        let x = ((vw - width) / 2.).max(8.);
        return layer.child(surface.absolute(x, 76_f32.min((vh - estimated - 8.).max(8.))));
    };
    // Its trailing edge on the control's; pinned by the edge that touches the control, so
    // the gap holds whatever the list's height turns out to be.
    let x = (cx + cw - width).clamp(8., (vw - width - 8.).max(8.));
    layer.child(if over {
        surface.pin(Some(x), None, None, Some(vh - (cy - popups::CONTROL_GAP)))
    } else {
        surface.absolute(x, cy + ch + popups::CONTROL_GAP)
    })
}
