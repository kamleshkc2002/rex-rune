//! List popups: the session switcher, the host step, Commands and the pane and tab menus. No scrim;
//! an outside press dismisses and is consumed.
use super::{
    input::{list_key, rename_key},
    keycaps::keycap,
    panel::{Intent, PanelKind},
    theme::*,
    widgets::{Row, SEPARATOR, glyph, list_row, row_action, separator},
    workspace::Workspace,
};
use rune::{El, ScrollSpec, TextEdit, Tx, col, el, row, text};
use rune_kit::TextInput;

const HEADER: f32 = 26.;
/// Commands shows about this many rows and scrolls to the rest, whatever the window's height.
const COMMAND_ROWS: f32 = 20.;

#[derive(Clone)]
enum Hidden {
    Session,
    Offline(String),
    /// A command under its group's heading: the group's name, and where the bottom of that
    /// heading is in the list.
    Command(String, f32),
    Other,
}

/// What Commands says is below the fold, whose bottom edge is `bottom` in the list: the
/// groups still to come, in order. A group whose heading is already on screen (or above) is
/// not below, though its last rows may be: it is named only when nothing else is.
fn groups_below(hidden: &[&Hidden], bottom: f32) -> String {
    let mut groups: Vec<&str> = Vec::new();
    let mut started = None;
    for row in hidden {
        let Hidden::Command(group, heading) = row else {
            continue;
        };
        if *heading <= bottom + 1. {
            started = Some(group.as_str());
        } else if groups.last() != Some(&group.as_str()) {
            groups.push(group);
        }
    }
    match (groups.is_empty(), started) {
        (false, _) => format!("More below: {}", groups.join(" · ")),
        (true, Some(group)) => format!("More {group} commands below"),
        (true, None) => "More results below".into(),
    }
}

/// A group's heading: its words in small capitals, and `name`, the part of it someone chose
/// (a host, a session, a tab), in the case they gave it. The name gives way first.
fn heading_words(group: &str, name: Option<&str>) -> El {
    let caps = |words: &str| {
        text(words.to_owned())
            .font(label_caps())
            .text_case(rune::TextCase::Uppercase)
            .color(rgb(muted()))
    };
    let words = row()
        .role("heading")
        .label(group.to_owned())
        .gap(4.)
        .center_y()
        .grow(1.)
        .shrink()
        .min_w(0.);
    let Some((at, name)) = name
        .filter(|name| !name.is_empty())
        .and_then(|name| Some((group.find(name)?, name)))
    else {
        return words.child(caps(group).truncate().shrink());
    };
    let (before, after) = (
        group[..at].trim_end(),
        group[at + name.len()..].trim_start(),
    );
    words
        .children((!before.is_empty()).then(|| caps(before)))
        .child(
            text(name.to_owned())
                .font(medium(11.))
                .color(rgb(muted()))
                .truncate()
                .shrink()
                .help_when_truncated(group.to_owned()),
        )
        .children((!after.is_empty()).then(|| caps(after)))
}

/// A transparent full-window layer that eats presses outside the popup and closes it.
/// The window-sized layer menus and dialogs open in, mounted even while empty. A surface that
/// mounts with the layer rides the layer's fade, which composites offscreen, so its backdrop
/// blur would see an empty layer until the fade fully settled, and then snap on.
pub fn overlay() -> El {
    el().key("overlay")
        .pin(Some(0.), Some(0.), Some(0.), Some(0.))
}

pub(super) fn dismiss_layer() -> El {
    el().key("popup-dismiss")
        .pin(Some(0.), Some(0.), Some(0.), Some(0.))
        .preserve_focus_on_press(true)
        .on_press_tx(|s: &mut Workspace, tx: &mut Tx| dismiss(s, tx))
        .on_secondary_press(|s: &mut Workspace, _, tx| dismiss(s, tx))
}

/// A press outside a list closes it; one opened from a Settings control goes back to Settings.
fn dismiss(s: &mut Workspace, tx: &mut Tx) {
    if s.panel
        .as_ref()
        .is_some_and(|p| super::settings_kit::under(p).is_some())
    {
        s.cancel_panel(tx)
    } else {
        s.close_panel(tx)
    }
}

/// How far a list opened from a control sits from it.
pub(super) const CONTROL_GAP: f32 = 4.;

/// The captured pane's outline, in window points, above any scrim and below the panel.
pub fn target_outline(s: &Workspace) -> Option<El> {
    let panel = s.panel.as_ref()?;
    // A session's menu, rename or confirm outlines its sidebar row instead.
    if panel.session.is_some() {
        return None;
    }
    if !matches!(
        panel.kind,
        PanelKind::Pane
            | PanelKind::Form(super::panel::Form::RenamePane(_))
            | PanelKind::Download(_)
            | PanelKind::Directory(_)
            | PanelKind::Confirm { .. }
    ) {
        return None;
    }
    let target = panel.target.as_ref()?;
    let current = s.model().window()?;
    let placement = rex_linux::layout::placements(current, None)
        .0
        .into_iter()
        .find(|p| p.block == target.block)?;
    let [ax, ay, aw, ah] = s.area;
    let bleed = super::pane_header::full_bleed(s, current);
    let ([x, y, w, h], radius) = if bleed {
        ([0., 0., aw, ah], 0.)
    } else {
        let radius = if s.settings.pane_density == rex_linux::settings::PaneDensity::Compact {
            0.
        } else {
            CARD_RADIUS
        };
        (super::panes::card_rect(s, placement.rect), radius)
    };
    Some(
        el().key("target-outline")
            .absolute(ax + x, ay + y)
            .size(w, h)
            .radius(radius)
            .continuous_corners()
            .border(1.5, rgb(accent())),
    )
}

pub fn list_popup(s: &Workspace, viewport: [f32; 2]) -> El {
    let k = kit();
    let panel = s.panel.as_ref().expect("list panel");
    let sessions = matches!(panel.kind, PanelKind::Picker(true) | PanelKind::Hosts(_));
    // Context menus (a pane's, a tab's, a sidebar session's) share one shape.
    let pane = matches!(
        panel.kind,
        PanelKind::Pane | PanelKind::Session | PanelKind::Host(_) | PanelKind::Window
    );
    let commands = matches!(panel.kind, PanelKind::Picker(false));
    let activity = matches!(panel.kind, PanelKind::Activity);
    // The values of one setting: a short list with the current one checked.
    let values = matches!(panel.kind, PanelKind::Choice(_) | PanelKind::Chromes);
    // Lists that mark the current value keep the check's column on every row, so names align.
    let checks = values || matches!(panel.kind, PanelKind::Themes);
    let [vw, vh] = viewport;
    // The control a Settings popup button opened this list from, `[x, y, w, h]` where layout
    // put it: the list goes right under it, or right above when there is more room there.
    // Include the page's scroll so choices stay beside a row reached by scrolling/search.
    let control = panel
        .anchor_to
        .as_deref()
        .and_then(rune::memo::window_layout);
    let room = control.map(|[_, y, _, h]| {
        let below = vh - (y + h + CONTROL_GAP) - 8.;
        let above = y - CONTROL_GAP - 8.;
        (below, above)
    });
    // Add host: machines in columns, each name whole.
    let discover = matches!(panel.kind, PanelKind::Discover);
    let width = (if values {
        220.
    } else if pane || matches!(panel.kind, PanelKind::ShortcutMenu) {
        280.
    } else if sessions {
        340.
    } else if discover {
        super::discover::LIST_WIDTH
    } else {
        460.
    })
    .min(vw - 16.);
    let top = if sessions || pane {
        TOOLBAR - 4.
    } else if vh < 750. {
        60.
    } else {
        76.
    };
    let (ax, ay) = panel.anchor.unwrap_or((
        if sessions {
            MARGIN + 38.
        } else if pane {
            vw - 450.
        } else {
            (vw - width) / 2.
        },
        top,
    ));
    let entries = s.entries();
    // A machine already added carries a chip, in a column every row of the list keeps.
    let added_column = discover && entries.iter().any(|e| e.current);
    let name_limit = super::discover::name_limit(width, added_column);
    let row_height = if activity {
        44.
    } else if commands {
        30.
    } else {
        28.
    };
    let footer_count = entries.iter().filter(|e| e.footer).count();
    // A list under its control needs no title: the control's row names it.
    let titled = control.is_none() || !panel.inputs.is_empty();
    let input_height = if !titled {
        0.
    } else if panel.inputs.is_empty() {
        32.
    } else {
        40.
    };
    let directory_extra = if let PanelKind::Directory(browser) = &panel.kind {
        26. + if browser.truncated { 28. } else { 0. }
    } else {
        0.
    };
    let mut list_max = match room {
        Some((below, above)) => (below.max(above) - input_height - 42.).clamp(60., 360.),
        None => (vh
            - top
            - 16.
            - input_height
            - directory_extra
            - if activity { 28. } else { 0. }
            - footer_count as f32 * 29.
            - 56.)
            .max(60.),
    };
    if commands {
        list_max = list_max.min(COMMAND_ROWS * (row_height + 1.));
    }
    // Commands under their headings, rather than the one list a search shows.
    let headed = commands && s.query().is_empty();

    // Rows, with their content spans for the overflow hint.
    let mut list = col().key("results").w_full().pr(4.).gap(1.);
    let mut footer = col().w_full().gap(1.);
    let surface_id = match panel.kind {
        PanelKind::Activity => "activity",
        PanelKind::Picker(true) => "switcher",
        PanelKind::Picker(false) => "commands",
        PanelKind::Hosts(_) => "hosts",
        PanelKind::Themes => "themes",
        PanelKind::Chromes => "chromes",
        PanelKind::Choice(_) => "choice",
        PanelKind::Discover => "discover",
        PanelKind::Session => "session-menu",
        PanelKind::Host(_) => "host-menu",
        PanelKind::Window => "tab-menu",
        PanelKind::ShortcutMenu => "shortcut-menu",
        _ => "pane-menu",
    };
    let mut spans: Vec<(f32, f32, Hidden)> = Vec::new();
    let mut y = 0.;
    let mut group = String::new();
    // Where the heading of the group being listed ends.
    let mut heading_bottom = 0.;
    let mut list_rows = 0;
    // The menu group of the row above, under the same header. A filtered list is flat: its
    // rows are matches, not a menu.
    let mut section = None;
    let grouped = s.query().is_empty();
    for (i, entry) in entries.iter().enumerate() {
        if !entry.footer && entry.group != group {
            section = None;
            group = entry.group.clone();
            if !group.is_empty() {
                let first = y == 0.;
                list = list.child(
                    row()
                        .w_full()
                        .h(HEADER - if first { 4. } else { 0. })
                        .pt(if first { 0. } else { 6. })
                        .px(10.)
                        .gap(7.)
                        .center_y()
                        .children(entry.group_icon.map(|i| glyph(i, 13., rgb(muted()))))
                        .child(
                            heading_words(&group, entry.group_name.as_deref())
                                .key(format!("group-heading:{i}")),
                        )
                        .children(entry.group_create.clone().map(|(action, help)| {
                            el().key(format!("group-create:{i}"))
                                .role("button")
                                .label(help.clone())
                                .help(help)
                                .size(22., 22.)
                                .mr(-4.)
                                .center()
                                .radius(6.)
                                .hover(white(0.08))
                                .press(white(0.12))
                                .cursor(rune::Cursor::Pointer)
                                .preserve_focus_on_press(true)
                                .tab_index(-1)
                                .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| {
                                    s.perform(action.clone(), tx)
                                })
                                .child(glyph(&PLUS, 13., rgb(muted())))
                        })),
                );
                y += HEADER - if first { 4. } else { 0. } + 1.;
                heading_bottom = y;
            }
        }
        if !entry.footer {
            if grouped && section.is_some_and(|above| above != entry.section) {
                list = list.child(separator());
                y += SEPARATOR + 1.;
            }
            section = Some(entry.section);
        }
        let action = entry.action.clone();
        let enabled = entry.enabled;
        // Switcher sessions are renamed in place and closed from the row itself.
        let switcher_session = match (&panel.kind, &entry.action) {
            (PanelKind::Picker(true), Intent::Select(host, session)) => {
                Some((*host, session.clone()))
            }
            _ => None,
        };
        let renaming = switcher_session.as_ref().and_then(|(host, session)| {
            panel
                .rename
                .as_ref()
                .filter(|(t, _)| t.host == *host && &t.session == session)
        });
        let r = if let Some((_, edit)) = renaming {
            rename_row(edit, row_height, entry.current, surface_id)
        } else {
            let actions = switcher_session.clone().map(|(host, session)| {
                let close = session.clone();
                row()
                    .gap(2.)
                    .mr(-4.)
                    .child(row_action(
                        format!("rename:{i}"),
                        &PENCIL,
                        "Rename",
                        move |s: &mut Workspace, _: &mut Tx| s.start_rename(host, session.clone()),
                    ))
                    .child(row_action(
                        format!("close:{i}"),
                        &CLOSE,
                        "Close session",
                        move |s: &mut Workspace, tx: &mut Tx| {
                            s.close_from_switcher(host, &close, tx)
                        },
                    ))
            });
            let mut r = list_row(
                Row {
                    key: format!("entry:{i}"),
                    surface: surface_id,
                    activity: entry.activity.as_ref(),
                    label: &entry.label,
                    detail: &entry.detail,
                    tone: entry.tone,
                    keys: &entry.keys,
                    height: if entry.footer { 28. } else { row_height },
                    // The Add host list marks an added machine with its chip instead.
                    check: (sessions || checks || (entry.current && !discover))
                        .then_some(entry.current),
                    followup: entry.followup,
                    enabled,
                    danger: entry.danger,
                    selected: i == panel.selected && enabled,
                    actions,
                    columns: discover
                        .then(|| {
                            s.machine_columns(
                                &entry.action,
                                i == panel.selected && enabled,
                                added_column,
                            )
                        })
                        .flatten()
                        .map(|columns| (columns, name_limit)),
                },
                i,
                move |s: &mut Workspace, tx: &mut Tx| s.perform(action.clone(), tx),
            );
            if let Some((host, session)) = switcher_session {
                r = r.on_secondary_press(move |s: &mut Workspace, _, _| {
                    s.start_rename(host, session.clone())
                });
            }
            r
        };
        if entry.footer {
            footer = footer.child(r);
        } else {
            let h = row_height + 1.;
            spans.push((
                y,
                y + h,
                match &entry.action {
                    Intent::Select(..) => Hidden::Session,
                    Intent::RetryHost(host) => {
                        Hidden::Offline(s.settings.hosts[*host].label.clone())
                    }
                    // The heading's first words: "Tab" of "Tab · shell".
                    _ if headed => Hidden::Command(
                        entry.group.split(" · ").next().unwrap_or_default().into(),
                        heading_bottom,
                    ),
                    _ => Hidden::Other,
                },
            ));
            y += h;
            list = list.child(r);
            list_rows += 1;
        }
    }
    if list_rows == 0 {
        let empty = match panel.kind {
            PanelKind::Activity if s.query().is_empty() => "Nothing needs you.".into(),
            PanelKind::Activity => "No matching activity.".into(),
            PanelKind::Directory(_) => panel
                .error
                .clone()
                .unwrap_or_else(|| "Reading folder…".into()),
            PanelKind::Themes => "No matching themes.".to_owned(),
            PanelKind::Discover => s.discovery_empty_message(),
            _ => "No matches. Try another name or create a session.".to_owned(),
        };
        list = list.child(
            row()
                .px(12.)
                .py(16.)
                .child(text(empty).font(ui(13.)).color(rgb(muted()))),
        );
        y += 52.;
    }
    let content = y;
    let overflow = content > list_max + 1.;
    let view_h = content.min(list_max);
    let fx = &s.switcher_fx;
    let mut scroller = col()
        .key("results-scroll")
        .w_full()
        .max_h(list_max)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        // Rows leave under a short fade, not a hard cut above the hint.
        .scroll_edge_fade(10.)
        .child(list);
    if overflow {
        let switcher = matches!(panel.kind, PanelKind::Picker(true));
        scroller = scroller.on_scroll_offset(fx.watch(width - 6., view_h, move |offset, max| {
            if offset >= max - 1. {
                return if sessions {
                    "Earlier sessions above"
                } else {
                    "Earlier results above"
                }
                .into();
            }
            let bottom = offset + view_h;
            let hidden: Vec<&Hidden> = spans
                .iter()
                .filter(|(_, b, _)| *b > bottom + 1.)
                .map(|(_, _, h)| h)
                .collect();
            if !switcher {
                return if sessions {
                    "More sessions below".into()
                } else {
                    groups_below(&hidden, bottom)
                };
            }
            let sessions_below = hidden
                .iter()
                .filter(|h| matches!(h, Hidden::Session))
                .count();
            let offline: Vec<&String> = hidden
                .iter()
                .filter_map(|h| {
                    if let Hidden::Offline(l) = h {
                        Some(l)
                    } else {
                        None
                    }
                })
                .collect();
            let mut parts = Vec::new();
            if sessions_below > 0 {
                parts.push(format!(
                    "{sessions_below} more session{}",
                    if sessions_below == 1 { "" } else { "s" }
                ));
            }
            match offline.len() {
                0 => {}
                1 => parts.push(format!("{} offline", offline[0])),
                n => parts.push(format!("{n} offline hosts below")),
            }
            if parts.is_empty() {
                "More sessions below".into()
            } else {
                parts.join(" · ")
            }
        }));
    }
    let mut surface = glass(
        col()
            .key("menu-surface")
            .role("menu")
            .w(width)
            .max_h(vh - 16.)
            .padding(5.)
            .gap(1.)
            .preserve_focus_on_press(true),
        12.,
        true,
    )
    .clip_rounded(12.)
    .transition(pop_in());
    if matches!(panel.kind, PanelKind::Directory(_)) {
        surface = surface.child(
            row().h(26.).px(10.).center_y().child(
                text(panel.title.clone())
                    .font(medium(11.5))
                    .color(rgb(muted()))
                    .truncate(),
            ),
        );
    }
    if titled && panel.inputs.is_empty() {
        surface = surface.child(
            row().h(26.).px(10.).center_y().child(
                text(panel.title.clone())
                    .font(medium(11.5))
                    .color(rgb(muted()))
                    .truncate(),
            ),
        );
    } else if titled {
        let placeholder = panel
            .fields
            .first()
            .map(|f| f.1.clone())
            .unwrap_or_default();
        surface = surface.child(
            row()
                .w_full()
                .h(34.)
                .mb(3.)
                .px(2.)
                .center_y()
                .radius(8.)
                .continuous_corners()
                .fill(white(0.045))
                .child(
                    TextInput::new(&panel.inputs[0])
                        .placeholder(placeholder)
                        .active_descendant(Some(format!("entry:{}", panel.selected)))
                        // A magnifying glass marks search fields only.
                        .leading_icon(if matches!(panel.kind, PanelKind::Directory(_)) {
                            &FOLDER
                        } else {
                            &SEARCH
                        })
                        .leading_icon_size(14.)
                        .text_style(ui(13.))
                        .height(32.)
                        .plain()
                        .trailing(keycap("Esc", false))
                        .on_edit(|s: &mut Workspace| s.edit_input(0))
                        .on_change(|s: &mut Workspace, _: &mut Tx| s.query_changed())
                        .on_key(list_key)
                        .width(None)
                        .build(k)
                        .key("popup-search")
                        .w_full(),
                ),
        );
    }
    let coverage = s.activity_coverage();
    if activity && !coverage.is_empty() {
        surface = surface.child(
            row()
                .h(28.)
                .px(10.)
                .center_y()
                .child(text(coverage).font(ui(11.)).color(rgb(muted())).truncate()),
        );
    }
    let mut results = el().w_full().min_h(0.).child(scroller);
    if overflow {
        results = results.child(fx.thumb_el());
    }
    surface = surface.child(results);
    if let PanelKind::Directory(browser) = &panel.kind
        && browser.truncated
    {
        surface = surface.child(
            row().px(10.).py(6.).child(
                text("Some entries aren't shown. Type more of the name.")
                    .font(ui(11.5))
                    .color(rgb(muted())),
            ),
        );
    }
    if overflow {
        surface = surface.child(fx.hint_el().w_full());
    }
    if footer_count > 0 {
        surface = surface
            .child(el().h(1.).mx(8.).my(3.).fill(white(0.07)))
            .child(footer.px(0.).py(0.));
    }
    // Keep the whole surface on screen: an estimate of its height bounds the anchor.
    let estimated = input_height
        + directory_extra
        + view_h
        + if pane { 24. } else { 0. }
        + footer_count as f32 * 29.
        + 18.
        + if overflow { 24. } else { 0. };
    let x = ax.clamp(8., (vw - width - 8.).max(8.));
    let y = ay.clamp(8., (vh - estimated - 8.).max(8.));
    // Over Settings when opened from one of its controls: the page stays where it was, inert.
    let mut layer = match super::settings_kit::under(panel) {
        Some(_) => super::settings_view::settings_under(s, viewport),
        None => overlay(),
    }
    .child(dismiss_layer());
    // The page under the list is inert, so the control lost focus when the list opened. The
    // list holds the keyboard as a modal (its field, or the surface itself when it has none),
    // and this empty marker, removed with the list, hands focus back to the control: Rune
    // returns focus that was inside the marker's parent.
    if let Some(control) = panel.anchor_to.as_ref().filter(|_| control.is_some()) {
        surface = surface.modal(true, |s: &mut Workspace| s.cancel_panel_quiet());
        // With a field, the field has the keys; without one the surface takes them itself.
        if panel.inputs.is_empty() {
            surface = surface.focusable(true).no_focus_ring().on_key(list_key);
        }
        layer = layer.child(
            el().key("list-focus-return")
                .absolute(0., 0.)
                .size(0., 0.)
                .focus_return(control.clone()),
        );
    }
    if let Some(outline) = target_outline(s) {
        layer = layer.child(outline);
    }
    let Some(([cx, cy, cw, ch], (below, above))) = control.zip(room) else {
        return layer.child(surface.absolute(x, y));
    };
    // Its trailing edge on the control's; pinned by the edge that touches the control, so
    // the gap holds whatever the list's height turns out to be.
    let x = (cx + cw - width).clamp(8., (vw - width - 8.).max(8.));
    layer.child(if estimated > below && above > below {
        surface.pin(Some(x), None, None, Some(vh - (cy - CONTROL_GAP)))
    } else {
        surface.absolute(x, cy + ch + CONTROL_GAP)
    })
}

/// A switcher row renamed in place: its name becomes a field, Enter saves and Esc goes back to
/// browsing.
fn rename_row(edit: &TextEdit, height: f32, current: bool, surface: &str) -> El {
    row()
        .key(format!("{surface}-rename"))
        .w_full()
        .h(height)
        .pl(9.)
        .pr(4.)
        .gap(8.)
        .center_y()
        .radius(7.)
        .continuous_corners()
        .fill(white(0.05))
        .border(1., rgb(accent()).alpha(0.55))
        .child(el().w(14.).center().child(if current {
            glyph(&CHECK, 14., rgb(accent()))
        } else {
            el()
        }))
        .child(
            TextInput::new(edit)
                .text_style(medium(13.))
                .height(height - 2.)
                .plain()
                .trailing(keycap("Enter", false))
                .on_edit(|s: &mut Workspace| s.rename_input())
                .on_key(rename_key)
                .width(None)
                .build(kit())
                .key("session-rename")
                // The field's own inset, so the name stays where the row's label was.
                .ml(-8.)
                .grow(1.)
                .shrink(),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hint_names_only_groups_whose_heading_is_still_below() {
        let row = |group: &str, heading: f32| Hidden::Command(group.into(), heading);
        // The list's bottom edge is at 600 pt. Tab's heading ended at 180 pt, so its hidden
        // rows are not a group below; Pane's and Host's headings are further down.
        let rows = [
            row("Tab", 180.),
            row("Tab", 180.),
            row("Pane", 660.),
            row("Pane", 660.),
            row("Host", 1400.),
        ];
        let hidden: Vec<&Hidden> = rows.iter().collect();
        assert_eq!(groups_below(&hidden, 600.), "More below: Pane · Host");
        // Only the rest of a group whose heading is on screen: it is named as that.
        assert_eq!(groups_below(&hidden[..2], 600.), "More Tab commands below");
        // A heading the fold cuts through is not on screen yet.
        assert_eq!(groups_below(&hidden[2..4], 650.), "More below: Pane");
        assert_eq!(groups_below(&[&Hidden::Other], 600.), "More results below");
    }
}
