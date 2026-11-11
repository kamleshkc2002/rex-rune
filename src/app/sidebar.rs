//! The optional host → session sidebar. It slides in and out; rows activate on click only
//! (keyboard session navigation is the Ctrl K switcher, so arrows never switch sessions).
use super::{
    activity::{program_icon, progress_bar, tab_activity, tab_name, tab_progress, tab_word},
    keycaps::keycaps,
    panel::Intent,
    panes::divider_handle,
    tabs::{TabAction, muted_mark, target_ring},
    theme::*,
    widgets::{chrome_button, glyph},
    workspace::Workspace,
};
use rex_linux::connection::HostCommand;
use rune::{
    Cursor, Cx, Edge, El, PointerButton, ScrollSpec, SpringPair, Transition, Tx, col, el, row, text,
};

const HOST_ROW: f32 = 28.;
const SESSION_ROW: f32 = 32.;
const NOTE_ROW: f32 = 28.;
const GROUP_GAP: f32 = 12.;

fn host_header(s: &Workspace, index: usize) -> El {
    let config = &s.settings.hosts[index];
    let connected = s.hosts[index].model.connected;
    let active = index == s.active;
    // An offline host dims; its Retry connection row below says why.
    let tone = rgb(if active && connected { ink() } else { muted() });
    let new_help = format!("New session on {}", config.label);
    // One step to a new session; offline hosts keep the slot so the column lines up.
    let plus = if connected {
        el().key(format!("host-new-session:{index}"))
            .role("button")
            .label(new_help.clone())
            .help(new_help)
            .size(22., 22.)
            .center()
            .radius(6.)
            .hover(white(0.06))
            .press(white(0.1))
            .cursor(Cursor::Pointer)
            .preserve_focus_on_press(true)
            .tab_index(-1)
            .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| {
                s.create_session(index, None, tx);
                tx.blur();
            })
            .child(glyph(&PLUS, 13., rgb(muted())))
    } else {
        el().key(format!("host-new-session-slot:{index}"))
            .size(22., 22.)
    };
    row()
        .key(format!("host:{index}"))
        .h(HOST_ROW)
        .pl(2.)
        // The + lines up under the panel header's +.
        .pr(1.)
        .gap(4.)
        .center_y()
        .on_secondary_press(move |s: &mut Workspace, p, _| s.host_menu(index, (p.x, p.y)))
        .child(
            el().key(format!("host-disclosure:{index}"))
                .role("button")
                .label(if config.collapsed {
                    "Expand host"
                } else {
                    "Collapse host"
                })
                .size(18., 22.)
                .center()
                .radius(6.)
                .hover(white(0.06))
                .cursor(Cursor::Pointer)
                .preserve_focus_on_press(true)
                .help(if config.collapsed {
                    "Expand host"
                } else {
                    "Collapse host"
                })
                .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| {
                    s.settings.hosts[index].collapsed = !s.settings.hosts[index].collapsed;
                    s.save();
                    tx.blur();
                })
                .child(glyph(
                    if config.collapsed { &RIGHT } else { &CHEVRON },
                    13.,
                    rgb(muted()),
                )),
        )
        .child(
            row()
                .key(format!("host-label:{index}"))
                .role("button")
                .label(config.label.clone())
                .grow(1.)
                .shrink()
                .h_full()
                .gap(6.)
                .center_y()
                .cursor(Cursor::Pointer)
                .preserve_focus_on_press(true)
                .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| {
                    s.activate_host(index, tx);
                    tx.blur();
                })
                .child(glyph(s.host_icon(index), 13., tone))
                .child(
                    text(config.label.clone())
                        .font(label_caps())
                        .text_case(rune::TextCase::Uppercase)
                        .color(tone)
                        .truncate()
                        .grow(1.)
                        .shrink(),
                ),
        )
        .child(plus)
}

fn session_row(s: &Workspace, index: usize, id: &str, label: &str) -> El {
    let host = &s.hosts[index];
    let selected = index == s.active && host.model.selected.as_deref() == Some(id);
    // A count earns its place only when it says something: a session has one tab unless
    // told otherwise.
    let count = host
        .model
        .views
        .get(id)
        .map(|v| v.windows.len())
        .filter(|count| *count > 1);
    let target = id.to_owned();
    let menu_id = id.to_owned();
    // The session a session menu, its rename form or its close confirm acts on.
    let targeted = s
        .panel
        .as_ref()
        .and_then(|p| p.session.as_ref())
        .is_some_and(|t| t.host == index && t.session == id);
    let mut e = row()
        .key(format!("session:{index}:{id}"))
        .role("button")
        .label(if selected {
            format!("{label} (current)")
        } else {
            label.to_owned()
        })
        .h(if s.settings.vertical_tabs {
            24.
        } else {
            SESSION_ROW
        })
        .px(10.)
        .gap(10.)
        .center_y()
        .radius(9.)
        .continuous_corners()
        .cursor(Cursor::Pointer)
        .preserve_focus_on_press(true)
        .help_when_truncated(format!("{} · {label}", s.settings.hosts[index].label))
        .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| {
            s.select(index, target.clone(), tx);
            tx.blur();
        })
        .on_secondary_press(move |s: &mut Workspace, p, _| {
            s.session_menu(index, menu_id.clone(), (p.x, p.y))
        });
    if selected && !s.settings.vertical_tabs {
        e = e.child(
            el().pin(Some(0.), Some(0.), Some(0.), Some(0.))
                .radius(9.)
                .continuous_corners()
                .fill(white(0.1))
                .box_shadow(inset(1., white(0.07)))
                .box_shadow(shadow(0., 1., 3., black(0.3)))
                .matched("sb-current")
                .transition(no_presence())
                .motion(SNAPPY),
        );
    } else {
        e = e.hover(white(0.05));
    }
    e = e
        .child(if s.settings.vertical_tabs {
            el()
        } else {
            glyph(
                &TERMINAL,
                14.,
                rgb(if selected { accent() } else { muted() }),
            )
        })
        .child(
            text(label.to_owned())
                .font(if s.settings.vertical_tabs {
                    medium(11.5)
                } else if selected {
                    medium(13.)
                } else {
                    ui(13.)
                })
                .color(rgb(if selected && !s.settings.vertical_tabs {
                    ink()
                } else {
                    muted()
                }))
                .truncate()
                .grow(1.)
                .shrink(),
        );
    if let Some(count) = count {
        e = e.child(
            text(count.to_string())
                .key(format!("session-count:{index}:{id}"))
                .label(format!("{count} tabs"))
                .font(medium(10.5))
                .color(rgb(muted())),
        );
    }
    if targeted {
        // An overlay, so the outline never shifts the row; above the selected fill.
        e = e.child(
            el().key(format!("session-target:{index}:{id}"))
                .pin(Some(0.), Some(0.), Some(0.), Some(0.))
                .radius(9.)
                .continuous_corners()
                .border(1.5, rgb(accent())),
        );
    }
    e
}

fn window_row(s: &Workspace, index: usize, session: &str, tab: &rexrs::api::session::Window) -> El {
    let Some(id) = tab.window_id.as_deref() else {
        return el();
    };
    let model = &s.hosts[index].model;
    let active = index == s.active
        && model.selected.as_deref() == Some(session)
        && model
            .views
            .get(session)
            .is_some_and(|v| v.active_window_id.as_deref() == Some(id));
    let activity = tab_activity(model, tab);
    let label = tab_name(model, tab);
    let (target_session, target_window) = (session.to_owned(), id.to_owned());
    let (rename_session, rename_window) = (session.to_owned(), id.to_owned());
    let (menu_session, menu_window) = (session.to_owned(), id.to_owned());
    let shown = s.tab_activity_shown(index, id);
    let quiet = s.tab_muted(index, id);
    let mut row = row()
        .key(format!("sidebar-tab:{index}:{session}:{id}"))
        .role("tab")
        .focus_selected(active)
        .label(if quiet {
            format!("{label} · Muted")
        } else {
            label.clone()
        })
        .h(32.)
        .ml(12.)
        .pl(10.)
        .pr(8.)
        .gap(8.)
        .center_y()
        .radius(10.)
        .continuous_corners()
        .cursor(Cursor::Pointer)
        .preserve_focus_on_press(true)
        .help_when_truncated(label.clone())
        .pointer_buttons([PointerButton::Middle])
        .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| {
            // Middle-click closes, behind the same confirmation as Close tab.
            if tx.button() == PointerButton::Middle {
                if let Some(target) = s.tab_target(index, &target_session, &target_window) {
                    s.perform(Intent::Window(target, TabAction::Close), tx);
                }
            } else {
                s.select_window(index, target_session.clone(), target_window.clone(), tx);
                tx.blur();
            }
        })
        .on_double_press_tx(move |s: &mut Workspace, tx: &mut Tx| {
            if let Some(target) = s.tab_target(index, &rename_session, &rename_window) {
                s.perform(Intent::Window(target, TabAction::Rename), tx);
            }
        })
        .on_secondary_press(move |s: &mut Workspace, p, _| {
            s.tab_menu(index, &menu_session, &menu_window, (p.x, p.y))
        })
        .child(glyph(
            program_icon(activity.program.as_deref()),
            15.,
            rgb(if active { ink() } else { muted() }),
        ))
        .child(
            text(label)
                .font(if active { medium(12.5) } else { ui(12.5) })
                .color(rgb(if active { ink() } else { muted() }))
                .truncate()
                .grow(1.)
                .min_w(0.),
        );
    if active {
        row = row
            .fill(white(0.11))
            .border_gradient(1., white(0.15), white(0.04));
    } else {
        row = row.hover(white(0.05));
    }
    if shown
        && let Some((word, tone)) =
            tab_word(&activity, s.unseen_on(index, id), s.bell_on(index, id))
    {
        row = row.child(text(word).font(medium(10.5)).color(rgb(tone)));
    }
    if quiet {
        row = row.child(muted_mark(id));
    }
    if shown && let Some(progress) = tab_progress(&activity) {
        row = row.child(progress_bar(format!("tab-progress:{id}"), progress, 10.));
    }
    if s.tab_targeted(index, id) {
        row = row.child(target_ring(id, 10.));
    }
    row
}

fn note(text_: &str, color: u32) -> El {
    row()
        .h(NOTE_ROW)
        .px(12.)
        .center_y()
        .child(text(text_.to_owned()).font(ui(12.5)).color(rgb(color)))
}

fn new_session(s: &Workspace, index: usize) -> El {
    let help = format!("New session on {}", s.settings.hosts[index].label);
    row()
        .key(format!("new-session-on:{index}"))
        .role("button")
        .label(help.clone())
        .help(help)
        .h(SESSION_ROW)
        .px(10.)
        .gap(10.)
        .center_y()
        .radius(9.)
        .continuous_corners()
        .fill(white(0.04))
        .box_shadow(inset(1., white(0.06)))
        .hover(white(0.08))
        .press(white(0.12))
        .cursor(Cursor::Pointer)
        .preserve_focus_on_press(true)
        .tab_index(-1)
        .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| {
            s.perform(Intent::CreateOn(index, None), tx);
            tx.blur();
        })
        .child(glyph(&PLUS, 14., rgb(accent())))
        .child(
            text("New session")
                .font(medium(12.5))
                .color(rgb(accent()))
                .truncate(),
        )
}

pub fn sidebar(s: &Workspace, cx: &mut Cx<Workspace>) -> El {
    let mut list = col().key("sidebar-content").px(8.);
    let mut height = 0.;
    for (index, host) in s.hosts.iter().enumerate() {
        let config = &s.settings.hosts[index];
        let mut group = col()
            .key(format!("group:{index}"))
            .gap(2.)
            .child(host_header(s, index));
        height += HOST_ROW;
        if !config.collapsed {
            for session in &host.model.sessions {
                group = group.child(session_row(s, index, &session.session_id, &session.label));
                height += if s.settings.vertical_tabs {
                    26.
                } else {
                    SESSION_ROW + 2.
                };
                if s.settings.vertical_tabs
                    && let Some(view) = host.model.views.get(&session.session_id)
                {
                    for tab in &view.windows {
                        if tab.window_id.is_some() {
                            group = group.child(window_row(s, index, &session.session_id, tab));
                            height += 34.;
                        }
                    }
                }
            }
            if host.model.sessions.is_empty() {
                if host.model.connected {
                    group = group.child(new_session(s, index));
                    height += SESSION_ROW + 2.;
                } else {
                    group = group.child(note("Waiting for connection…", muted()));
                    height += NOTE_ROW;
                }
            }
            if !host.model.connected {
                let status = host.model.status.clone();
                group = group.child(
                    note("Retry connection", accent())
                        .key(format!("retry-host:{index}"))
                        .role("button")
                        .radius(9.)
                        .hover(white(0.05))
                        .cursor(Cursor::Pointer)
                        .preserve_focus_on_press(true)
                        .help(status)
                        .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| {
                            s.send(index, HostCommand::Reconnect);
                            tx.blur();
                        }),
                );
                height += NOTE_ROW;
            }
        }
        list = list.child(group.pb(GROUP_GAP));
        height += GROUP_GAP;
    }
    let viewport = cx.layout_size("sidebar-list").map_or(0., |[_, h]| h);
    let overflow = viewport > 0. && height > viewport + 1.;
    let width = s.sidebar.width();
    let fx = &s.sidebar_fx;
    let mut scroller = col()
        .key("sidebar-list")
        .grow(1.)
        .min_h(0.)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .scroll_edge_fade(12.)
        .child(list);
    if overflow {
        scroller = scroller.on_scroll_offset(fx.watch(width, viewport, |offset, max| {
            if offset < max - 1. {
                "More sessions and hosts below".into()
            } else {
                "Earlier sessions and hosts above".into()
            }
        }));
    }
    let total: usize = s.hosts.iter().map(|h| h.model.sessions.len()).sum();
    let mut area = col().grow(1.).min_h(0.).child(scroller);
    if overflow {
        area = area.child(fx.thumb_el());
    }
    let find = chip(
        row()
            .key("session-search")
            .role("button")
            .label("Find a session")
            .h(32.)
            .mx(8.)
            .px(10.)
            .gap(8.)
            .center_y()
            .hover(white(0.08))
            .cursor(Cursor::Pointer)
            .preserve_focus_on_press(true)
            .on_press_tx(|s: &mut Workspace, tx: &mut Tx| {
                s.picker(true);
                tx.blur();
            })
            .child(glyph(&SEARCH, 14., rgb(muted())))
            .child(
                text("Find a session")
                    .font(ui(12.5))
                    .color(rgb(muted()))
                    .truncate()
                    .grow(1.)
                    .shrink(),
            )
            // The switcher's shortcut as it is bound now; nothing when it is unbound.
            .child(
                keycaps(
                    &s.shortcut_chords("session.switch", &serde_json::Value::Null),
                    false,
                )
                .key("session-search-keys"),
            ),
    );
    let mut panel = col()
        .w_full()
        .grow(1.)
        .min_h(0.)
        .radius(CARD_RADIUS)
        .continuous_corners()
        .fill(white(0.025))
        .border_gradient(1., white(0.07), white(0.02))
        .pt(8.)
        .gap(8.)
        .child(
            row()
                .h(30.)
                .pl(14.)
                .pr(6.)
                .center_y()
                .child(
                    text("Sessions")
                        .font(strong(13.))
                        .color(rgb(ink()))
                        .grow(1.),
                )
                .child(chrome_button(
                    "new-session",
                    &PLUS,
                    "New session… · choose a host",
                    28.,
                    |s, tx| s.perform(Intent::ChooseHost(String::new()), tx),
                )),
        )
        .child(find)
        .child(area);
    if overflow {
        panel = panel.child(fx.hint_el());
    }
    panel = panel.child(
        row()
            .h(40.)
            .px(8.)
            .center_y()
            .justify_between()
            .child(
                row()
                    .key("add-host")
                    .role("button")
                    .label("Add host")
                    .h(28.)
                    .px(8.)
                    .gap(6.)
                    .center_y()
                    .radius(8.)
                    .hover(white(0.06))
                    .cursor(Cursor::Pointer)
                    .preserve_focus_on_press(true)
                    .on_press_tx(|s: &mut Workspace, tx: &mut Tx| s.perform(Intent::AddHost, tx))
                    .child(glyph(&PLUS, 14., rgb(muted())))
                    .child(text("Add host").font(ui(12.)).color(rgb(muted()))),
            )
            .child(
                text(session_count(total))
                    .key("sidebar-session-count")
                    .font(ui(11.))
                    .color(rgb(muted()))
                    .pr(6.),
            ),
    );
    col()
        .key("sidebar")
        .w(width)
        .h_full()
        .pb(MARGIN)
        .pl(MARGIN)
        .transition(
            Transition::move_edge(Edge::Leading)
                .combined(Transition::opacity())
                .springs(SpringPair::new(SNAPPY, SNAPPY)),
        )
        .child(panel)
}

/// The sidebar's resize edge, in the gutter beside it.
pub fn sidebar_edge(s: &Workspace) -> El {
    divider_handle(
        el().key("sidebar-edge").w(10.).ml(-1.).mr(-9.).h_full(),
        true,
        s.sidebar_drag_start.is_some(),
        false,
    )
    .on_drag(|s: &mut Workspace, d, tx| {
        let start = *s.sidebar_drag_start.get_or_insert(s.sidebar.width());
        s.sidebar.drag_to(start + d.dx / s.ui_scale());
        tx.instant();
        if d.phase == rune::Phase::Up {
            s.sidebar_drag_start = None;
            s.save();
        }
    })
    .on_double_press(|s: &mut Workspace| {
        s.sidebar.reset_width();
        s.save();
    })
}

/// How many sessions the saved hosts have between them, for the footer.
fn session_count(total: usize) -> String {
    if total == 1 {
        "1 session".to_owned()
    } else {
        format!("{total} sessions")
    }
}
