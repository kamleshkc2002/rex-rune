//! List popups: the session switcher, the host step, Commands and the pane and tab menus. No scrim;
//! an outside press dismisses and is consumed.
use super::{
    input::{list_key, rename_key},
    panel::{Intent, PanelKind},
    theme::*,
    widgets::{Row, glyph, list_row, row_action},
    workspace::Workspace,
};
use rune::{El, ScrollSpec, TextEdit, Tx, col, el, row, text};
use rune_kit::TextInput;

const HEADER: f32 = 26.;

#[derive(Clone)]
enum Hidden {
    Session,
    Offline(String),
    Other,
}

/// A transparent full-window layer that eats presses outside the popup and closes it.
/// The window-sized layer menus and dialogs open in, mounted even while empty. A surface that
/// mounts with the layer rides the layer's fade, which composites offscreen, so its backdrop
/// blur would see an empty layer until the fade fully settled, and then snap on.
pub fn overlay() -> El {
    el().key("overlay")
        .pin(Some(0.), Some(0.), Some(0.), Some(0.))
}

fn dismiss_layer() -> El {
    el().key("popup-dismiss")
        .pin(Some(0.), Some(0.), Some(0.), Some(0.))
        .preserve_focus_on_press(true)
        .on_press_tx(|s: &mut Workspace, tx: &mut Tx| s.close_panel(tx))
        .on_secondary_press(|s: &mut Workspace, _, tx| s.close_panel(tx))
}

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
    let bleed = super::panes::full_bleed(current);
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
        PanelKind::Pane | PanelKind::Session | PanelKind::Host(_) | PanelKind::Window(_)
    );
    let commands = matches!(panel.kind, PanelKind::Picker(false));
    let [vw, vh] = viewport;
    let width = (if pane {
        260.
    } else if sessions {
        340.
    } else {
        460.
    } as f32)
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
    let row_height = if commands { 30. } else { 28. };
    let footer_count = entries.iter().filter(|e| e.footer).count();
    let input_height = if panel.inputs.is_empty() { 32. } else { 40. };
    let list_max = (vh - top - 16. - input_height - footer_count as f32 * 29. - 56.).max(60.);

    // Rows, with their content spans for the overflow hint.
    let mut list = col().key("results").w_full().pr(4.).gap(1.);
    let mut footer = col().w_full().gap(1.);
    let surface_id = match panel.kind {
        PanelKind::Picker(true) => "switcher",
        PanelKind::Picker(false) => "commands",
        PanelKind::Hosts(_) => "hosts",
        PanelKind::Themes => "themes",
        PanelKind::Chromes => "chromes",
        PanelKind::Discover => "discover",
        PanelKind::Session => "session-menu",
        PanelKind::Host(_) => "host-menu",
        PanelKind::Window(_) => "tab-menu",
        _ => "pane-menu",
    };
    let mut spans: Vec<(f32, f32, Hidden)> = Vec::new();
    let mut y = 0.;
    let mut group = String::new();
    let mut list_rows = 0;
    for (i, entry) in entries.iter().enumerate() {
        if !entry.footer && entry.group != group {
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
                            text(group.clone())
                                .font(label_caps())
                                .text_case(rune::TextCase::Uppercase)
                                .color(rgb(muted()))
                                .truncate()
                                .grow(1.)
                                .shrink(),
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
            }
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
                    label: &entry.label,
                    detail: &entry.detail,
                    height: if entry.footer { 28. } else { row_height },
                    check: (sessions || entry.current).then_some(entry.current),
                    followup: entry.followup,
                    enabled,
                    selected: i == panel.selected && enabled,
                    actions,
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
            if !switcher {
                return if sessions {
                    "More sessions below"
                } else {
                    "More results below"
                }
                .into();
            }
            let bottom = offset + view_h;
            let hidden: Vec<&Hidden> = spans
                .iter()
                .filter(|(_, b, _)| *b > bottom + 1.)
                .map(|(_, _, h)| h)
                .collect();
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
    if panel.inputs.is_empty() {
        surface = surface.child(
            row().h(26.).px(10.).center_y().child(
                text(panel.title.clone())
                    .font(medium(11.5))
                    .color(rgb(muted()))
                    .truncate(),
            ),
        );
    } else {
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
                        .leading_icon(&SEARCH)
                        .leading_icon_size(14.)
                        .text_style(ui(13.))
                        .height(32.)
                        .plain()
                        .trailing(text("esc").font(medium(10.5)).color(rgb(muted())))
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
    let mut results = el().w_full().min_h(0.).child(scroller);
    if overflow {
        results = results.child(fx.thumb_el());
    }
    surface = surface.child(results);
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
        + view_h
        + if pane { 24. } else { 0. }
        + footer_count as f32 * 29.
        + 18.
        + if overflow { 24. } else { 0. };
    let x = ax.clamp(8., (vw - width - 8.).max(8.));
    let y = ay.clamp(8., (vh - estimated - 8.).max(8.));
    let mut layer = overlay().child(dismiss_layer());
    if let Some(outline) = target_outline(s) {
        layer = layer.child(outline);
    }
    layer.child(surface.absolute(x, y))
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
                .trailing(text("enter").font(medium(10.5)).color(rgb(muted())))
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
