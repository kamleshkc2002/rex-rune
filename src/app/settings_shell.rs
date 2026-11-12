//! The frame around a Settings page: the sidebar (Search Settings over the section list, or
//! over what a search found) and the header (Back, Forward, the title and Close). The state
//! behind both is `settings_nav`.
use super::{
    keycaps::{keycap, spell},
    panel::SettingsSection,
    settings_kit::{INSET, on_return, search_box},
    settings_nav::{HEADER, PAGE_INSET},
    theme::*,
    widgets::{glyph, highlight},
    workspace::Workspace,
};
use rune::{Cursor, El, Icon, Key, KeyEvent, ScrollSpec, TextEdit, Tx, col, el, row, text};
use rune_kit::{IconButton, TextInput};

/// Keys the search field takes: the arrows walk the hits and Enter opens the selected one;
/// Escape ends a search (and closes Settings when there is none). With nothing typed, Down
/// moves to the section list.
fn search_key(s: &mut Workspace, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if s.settings_key(ev, tx) || super::input::global_key(s, ev, tx) {
        return true;
    }
    let searching = !s.settings_nav.query().is_empty();
    match ev.key {
        Key::Down | Key::Up if searching => {
            let direction = if ev.key == Key::Down { 1 } else { -1 };
            if let Some(lit) = s.settings_nav.step_selected(direction) {
                tx.scroll_into_view(format!("settings-hit:{lit}"));
                tx.instant();
            }
        }
        Key::Down => tx.focus(
            format!("settings-section:{}", s.settings_section.key()),
            true,
        ),
        Key::Enter if searching => s.open_settings_hit(s.settings_nav.selected(), tx),
        Key::Escape if !s.settings_nav.search.text.is_empty() => s.clear_settings_search(),
        _ => return false,
    }
    true
}

/// A letter typed while the section list has focus starts a search with it.
fn type_ahead(s: &mut Workspace, ev: &KeyEvent, _: &mut Tx) -> bool {
    let Key::Text(typed) = &ev.key else {
        return false;
    };
    if ev.mods.ctrl || ev.mods.alt || ev.mods.cmd || typed.trim().is_empty() {
        return false;
    }
    s.settings_nav.search = TextEdit::new(typed.clone());
    s.settings_nav.search.focused = true;
    s.settings_search_changed();
    true
}

macro_rules! icon {
    ($name:ident, $id:literal, $($el:literal),+) => {
        static $name: Icon = Icon {
            name: $id,
            view: 20.0,
            body: concat!($("<", $el, r#" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#),+),
        };
    };
}
// The window: its toolbar across the top and the sidebar under it.
icon!(
    INTERFACE,
    "rex-settings-interface",
    r#"rect x="3" y="4" width="14" height="12" rx="3""#,
    r#"path d="M3 8h14""#,
    r#"path d="M8 8v8""#
);
// A keyboard: three keys over the space bar.
icon!(
    SHORTCUTS,
    "rex-settings-shortcuts",
    r#"rect x="2.5" y="5.5" width="15" height="9" rx="2""#,
    r#"path d="M6 8.75h.01""#,
    r#"path d="M10 8.75h.01""#,
    r#"path d="M14 8.75h.01""#,
    r#"path d="M7 11.5h6""#
);
// A plug.
icon!(
    INTEGRATIONS,
    "rex-settings-integrations",
    r#"path d="M7.5 3.5V7""#,
    r#"path d="M12.5 3.5V7""#,
    r#"path d="M5.5 7h9v2.5a4.5 4.5 0 0 1-9 0Z""#,
    r#"path d="M10 14v2.5""#
);
icon!(
    NOTIFICATIONS,
    "rex-settings-notifications",
    r#"path d="M14 8.5a4 4 0 0 0-8 0c0 2.6-.9 3.9-1.75 4.75h11.5C14.9 12.4 14 11.1 14 8.5Z""#,
    r#"path d="M8.75 16h2.5""#
);
// An arrow down onto a line.
icon!(
    UPDATES,
    "rex-settings-updates",
    r#"path d="M10 3.5V12""#,
    r#"path d="m6.5 8.5 3.5 3.5 3.5-3.5""#,
    r#"path d="M4.5 15.5h11""#
);
icon!(
    ABOUT,
    "rex-settings-about",
    r#"circle cx="10" cy="10" r="6.5""#,
    r#"path d="M10 9.5v3.75""#,
    r#"path d="M10 6.75h.01""#
);

/// The glyph on a section's tile. Terminal and Remote Hosts use the glyphs the tab strip and
/// the session sidebar use for a terminal and a server.
fn section_icon(section: SettingsSection) -> &'static Icon {
    match section {
        SettingsSection::Interface => &INTERFACE,
        SettingsSection::Terminal => &TERMINAL,
        SettingsSection::Shortcuts => &SHORTCUTS,
        SettingsSection::Integrations => &INTEGRATIONS,
        SettingsSection::Notifications => &NOTIFICATIONS,
        SettingsSection::Server | SettingsSection::Hosts => &SERVER,
        SettingsSection::Updates => &UPDATES,
        SettingsSection::About => &ABOUT,
    }
}

/// A section's glyph on its tile: a 22 pt rounded square, lit in `ACCENT` for the section on
/// screen.
fn tile(section: SettingsSection, on: bool) -> El {
    let (fill, ink_color) = if on {
        (rgb(accent()).alpha(SELECTION_FILL), rgb(accent()))
    } else {
        (white(0.07), rgb(muted()))
    };
    el().size(22., 22.)
        .radius(6.)
        .continuous_corners()
        .fill(fill)
        .center()
        .child(glyph(section_icon(section), 20., ink_color))
}

/// The first section of each sidebar group after the first: the window and its colours;
/// what runs in it and reaches it; the app itself.
fn starts_group(section: SettingsSection) -> bool {
    matches!(
        section,
        SettingsSection::Interface | SettingsSection::Terminal | SettingsSection::Updates
    )
}

fn search_field(s: &Workspace) -> El {
    let nav = &s.settings_nav;
    let searching = !nav.query().is_empty();
    let mut input = TextInput::new(&nav.search)
        .placeholder("Search Settings")
        .label("Search Settings")
        .leading_icon(&SEARCH)
        .leading_icon_size(14.)
        .text_style(ui(13.))
        .height(30.)
        .plain()
        .on_edit(|s: &mut Workspace| &mut s.settings_nav.search)
        .on_change(|s: &mut Workspace, _: &mut Tx| s.settings_search_changed())
        .on_key(search_key)
        .width(None);
    if searching {
        input = input
            .active_descendant(Some(format!("settings-hit:{}", nav.selected())))
            .trailing(keycap("Esc", false));
    }
    search_box(
        "settings-search-box",
        30.,
        input.build(kit()).key("settings-search"),
    )
}

/// The section list, in gapped groups, each section behind its tile.
fn sections(s: &Workspace) -> El {
    let mut list = col()
        .key("settings-sections")
        .role("tablist")
        .focus_group()
        .on_key(type_ahead)
        .gap(2.);
    for section in SettingsSection::ALL {
        let selected = s.settings_section == section;
        let mut item = row()
            .key(format!("settings-section:{}", section.key()))
            .role("tab")
            .label(section.title())
            .w_full()
            .h(32.)
            .px(6.)
            .gap(9.)
            .center_y()
            .radius(8.)
            .continuous_corners()
            .cursor(Cursor::Pointer)
            .focusable(true)
            .focus_selected(selected)
            .focus_ring(2., 0., rgb(accent()))
            .on_press(move |s: &mut Workspace| s.navigate(section))
            .child(tile(section, selected))
            .child(
                text(section.title())
                    .font(if selected { medium(13.) } else { ui(13.) })
                    .color(rgb(if selected { ink() } else { muted() })),
            );
        if selected {
            // Settings opens with the keyboard here, not in the search field before it.
            item = item.autofocus();
        }
        if starts_group(section) {
            item = item.mt(12.);
        }
        item = if selected {
            item.fill(white(0.09))
        } else {
            item.hover(white(0.05))
        };
        list = list.child(item);
    }
    list
}

/// What the search found, in place of the section list: a page, or a row with where it is.
fn results(s: &Workspace) -> El {
    let nav = &s.settings_nav;
    if nav.hits().is_empty() {
        return text("No settings match.")
            .key("settings-no-hits")
            .font(ui(12.5))
            .color(rgb(muted()))
            .px(6.)
            .pt(6.);
    }
    let mut list = col()
        .key("settings-hits")
        .role("listbox")
        .label("Search results")
        .w_full()
        .gap(2.);
    for (index, hit) in nav.hits().iter().enumerate() {
        let selected = index == nav.selected();
        // A name too long for the sidebar goes on to a second line before it is cut.
        let mut words = col().grow(1.).shrink().min_w(0.).child(
            text(hit.label.clone())
                .font(if selected { medium(13.) } else { ui(13.) })
                .color(rgb(ink()))
                .max_lines(2),
        );
        if !hit.place.is_empty() {
            words = words.child(
                text(hit.place.clone())
                    .font(ui(11.))
                    .color(if selected {
                        rgb(ink()).alpha(0.75)
                    } else {
                        rgb(muted())
                    })
                    .truncate(),
            );
        }
        let full = if hit.place.is_empty() {
            hit.label.clone()
        } else {
            format!("{} · {}", hit.label, hit.place)
        };
        let item = row()
            .key(format!("settings-hit:{index}"))
            .role("option")
            .label(full.clone())
            .help_when_truncated(full)
            .w_full()
            .min_h(if hit.place.is_empty() { 32. } else { 42. })
            .px(6.)
            .py(4.)
            .gap(9.)
            .center_y()
            .radius(8.)
            .continuous_corners()
            .cursor(Cursor::Pointer)
            .preserve_focus_on_press(true)
            .tab_index(-1)
            .on_hover(move |s: &mut Workspace| s.settings_nav.select(index))
            .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| s.open_settings_hit(index, tx))
            // The selection's slot is always there, so the row's content never moves.
            .child(if selected {
                highlight("sel:settings-hits", 8., false)
            } else {
                el().pin(Some(0.), Some(0.), Some(0.), Some(0.))
            })
            .child(tile(hit.page.section(), false))
            .child(words);
        list = list.child(item);
    }
    col()
        .key("settings-hits-scroll")
        .grow(1.)
        .min_h(0.)
        .w_full()
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .scroll_edge_fade(10.)
        .child(list)
}

/// The sidebar: Search Settings over the section list, or over what a search found.
pub fn sidebar(s: &Workspace, width: f32) -> El {
    let searching = !s.settings_nav.query().is_empty();
    col()
        .key("settings-sidebar")
        .w(width)
        .h_full()
        .px(10.)
        .pt((HEADER - 30.) / 2.)
        .pb(10.)
        .gap(13.)
        .child(search_field(s))
        .child(if searching {
            results(s)
        } else {
            col()
                .key("settings-sections-scroll")
                .grow(1.)
                .min_h(0.)
                .w_full()
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .scroll_edge_fade(10.)
                .child(sections(s))
        })
}

/// Back or Forward in the header: dimmed and out of reach while there is nowhere to go.
fn history_button(s: &Workspace, direction: isize) -> El {
    let (key, icon, label, arrow) = if direction < 0 {
        ("settings-back", &LEFT, "Back", "Left")
    } else {
        ("settings-forward", &RIGHT, "Forward", "Right")
    };
    let keys = spell(&[vec!["Alt".to_owned(), arrow.to_owned()]]);
    let button = row()
        .key(key)
        .role("button")
        .label(label)
        .size(30., 28.)
        .center()
        .radius(9.)
        .continuous_corners()
        .child(glyph(icon, 16., rgb(ink())));
    if !s.settings_can_go(direction) {
        return button.opacity(0.32).disabled(true).focusable(false);
    }
    on_return(
        button
            .cursor(Cursor::Pointer)
            .hover(white(0.08))
            .press(white(0.12))
            .focusable(true)
            .focus_ring(2., 0., rgb(accent()))
            .help(format!("{label} · {keys}"))
            .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| {
                s.settings_history(direction, tx);
            }),
        move |s, tx| {
            s.settings_history(direction, tx);
        },
    )
}

/// The current section's title.
fn title(page: super::settings_nav::Page) -> El {
    row().grow(1.).min_w(0.).child(
        text(page.title())
            .key("settings-title")
            .font(strong(17.))
            .color(rgb(ink()))
            .truncate()
            .shrink()
            .help_when_truncated(page.title()),
    )
}

/// The header over the page: Back and Forward, the title, and Close.
pub fn header(s: &Workspace, page: super::settings_nav::Page) -> El {
    row()
        .key("settings-header")
        .h(HEADER)
        .px(PAGE_INSET)
        .gap(INSET)
        .center_y()
        .child(
            chip(row())
                .key("settings-history")
                .h(28.)
                .center_y()
                .child(history_button(s, -1))
                .child(el().w(1.).h(14.).fill(white(0.09)))
                .child(history_button(s, 1)),
        )
        .child(title(page))
        .child(
            IconButton::new(&CLOSE)
                .label("Close")
                .on_press_tx(|s: &mut Workspace, tx: &mut Tx| s.close_panel(tx))
                .build(kit())
                .key("close-settings"),
        )
}
