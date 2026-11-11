//! The 52pt bar on the window ground: sidebar toggle, identity chip, tab track, drag space,
//! Commands, New tab and Settings, a divider, and Close.
use super::{
    activity::{program_icon, progress_bar, tab_activity, tab_name, tab_progress, tab_word},
    panel::Intent,
    theme::*,
    widgets::{chrome_button, glyph},
    workspace::Workspace,
};
use rune::{Cursor, El, PointerButton, ScrollPos, Tx, col, el, row, text};

/// A chip's press: consume it (chips never drag the window) and keep the terminal focused.
fn chip_press(e: El) -> El {
    e.cursor(Cursor::Pointer)
        .preserve_focus_on_press(true)
        .tab_index(-1)
}

fn identity(s: &Workspace) -> El {
    let model = s.model();
    let host = &s.settings.hosts[s.active].label;
    let session = model
        .view()
        .and_then(|v| v.label.clone())
        .unwrap_or_else(|| "Choose session".into());
    // Only trouble is worth words: a healthy host shows just its name.
    let (detail, tone) = if model.connected {
        (host.clone(), muted())
    } else {
        (format!("{host} · Reconnecting"), warning())
    };
    chip_press(chip(
        row()
            .key("identity")
            .role("button")
            .label(format!("{session} · {host} · Switch session"))
            .w(210.)
            .h(34.)
            .pl(6.)
            .pr(8.)
            .gap(9.)
            .center_y()
            .hover(white(0.08))
            .help(format!("{session} · {host} · Switch session · Ctrl K"))
            .on_press_tx(|s: &mut Workspace, tx: &mut Tx| {
                s.picker(true);
                tx.blur();
            })
            .on_secondary_press(|s: &mut Workspace, p, _| {
                if let Some(session) = s.model().selected.clone() {
                    s.session_menu(s.active, session, (p.x, p.y));
                }
            })
            .child(el().size(22., 22.).center().child(glyph(
                s.host_icon(s.active),
                16.,
                rgb(muted()),
            )))
            .child(
                col()
                    .grow(1.)
                    .min_w(0.)
                    .gap(1.)
                    .child(
                        text(session)
                            .font(strong(12.5))
                            .color(rgb(ink()))
                            .truncate(),
                    )
                    .child(text(detail).font(ui(10.5)).color(rgb(tone)).truncate()),
            )
            .child(glyph(&CHEVRON, 14., rgb(muted()))),
    ))
}

fn tab_track(s: &Workspace) -> El {
    let model = s.model();
    let session = model.selected.clone().unwrap_or_default();
    let mut names = row()
        .key("tab-names")
        .h(28.)
        .gap(2.)
        .on_layout(|s: &mut Workspace, r| s.tab_strip.content = r[2]);
    if let Some(view) = model.view() {
        for tab in &view.windows {
            let Some(id) = tab.window_id.clone() else {
                continue;
            };
            let active = view.active_window_id.as_ref() == Some(&id);
            let label = tab_name(model, tab);
            let activity = tab_activity(model, tab);
            let word = tab_word(&activity, s.unseen(&id), s.bell_on(s.active, &id))
                .filter(|_| s.settings.notifications.show_in_tabs);
            let (press_id, rename_id, menu_id) = (id.clone(), id.clone(), id.clone());
            let mut tab_el = row()
                .key(format!("tab:{id}"))
                .role("tab")
                .focus_selected(active)
                .label(match &word {
                    Some((word, _)) => format!("{label} · {word}"),
                    None => label.clone(),
                })
                .h_full()
                .min_w(96.)
                .max_w(240.)
                .px(14.)
                .gap(8.)
                .center_y()
                .cursor(Cursor::Pointer)
                .preserve_focus_on_press(true)
                .pointer_buttons([PointerButton::Middle])
                .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| {
                    // Middle-click closes, behind the same confirmation as Close tab.
                    if tx.button() == PointerButton::Middle {
                        s.perform(Intent::CloseWindow(press_id.clone()), tx);
                    } else {
                        s.perform(Intent::Tab(press_id.clone()), tx);
                        tx.blur();
                    }
                })
                .on_double_press_tx(move |s: &mut Workspace, tx: &mut Tx| {
                    s.perform(Intent::RenameWindow(rename_id.clone()), tx);
                })
                .on_secondary_press(move |s: &mut Workspace, p, _| {
                    s.tab_menu(menu_id.clone(), (p.x, p.y))
                });
            if active {
                tab_el = tab_el.child(
                    el().pin(Some(0.), Some(0.), Some(0.), Some(0.))
                        .radius(8.)
                        .continuous_corners()
                        .fill(white(0.11))
                        .box_shadow(shadow(0., 1., 3., black(0.35)))
                        .box_shadow(inset(1., white(0.09)))
                        .matched(format!("tab-pill:{session}"))
                        .transition(no_presence())
                        .motion(SNAPPY),
                );
            } else {
                tab_el = tab_el.radius(8.).hover(white(0.05));
            }
            let program = activity.program.as_deref();
            let mut icon = el().key(format!("tab-icon:{id}")).child(glyph(
                program_icon(program),
                15.,
                rgb(if active { ink() } else { muted() }),
            ));
            if let Some(program) = program {
                icon = icon.help(program.to_owned());
            }
            tab_el = tab_el.child(icon);
            tab_el = tab_el.child(
                text(label.clone())
                    .font(if active { medium(12.5) } else { ui(12.5) })
                    .color(rgb(if active { ink() } else { muted() }))
                    .truncate()
                    .help_when_truncated(label)
                    .shrink(),
            );
            // The label truncates first; the word always stays whole.
            if let Some((word, tone)) = word {
                let done = word == "Done";
                let mut mark = text(word)
                    .key(format!("tab-word:{id}"))
                    .font(medium(11.5))
                    .color(rgb(tone));
                if let Some(msg) = activity.msg.clone().filter(|_| !done) {
                    mark = mark.help(msg);
                }
                tab_el = tab_el.child(mark);
            }
            if s.settings.notifications.show_in_tabs
                && let Some(progress) = tab_progress(&activity)
            {
                tab_el = tab_el.child(progress_bar(format!("tab-progress:{id}"), progress, 14.));
            }
            names = names.child(tab_el);
        }
    }
    let overflow = s.tab_strip.overflows();
    let (fade, pos) = (s.tab_fade.clone(), s.tab_pos.clone());
    let strip = row()
        .key("tabs")
        .min_w(0.)
        .shrink()
        .h(28.)
        .scroll_x()
        .scroll_edge_fade(16.)
        .on_layout(|s: &mut Workspace, r| s.tab_strip.viewport = r[2])
        .on_scroll_offset(move |p: ScrollPos| {
            pos.set([p.offset[0], p.max[0]]);
            fade[0].set(if p.offset[0] > 0.5 { 1. } else { 0.35 });
            fade[1].set(if p.offset[0] < p.max[0] - 0.5 {
                1.
            } else {
                0.35
            });
        })
        .child(names);
    let mut track = row()
        .key("tab-track")
        .h(34.)
        .px(3.)
        .gap(2.)
        .center_y()
        .min_w(0.)
        .shrink();
    if overflow {
        track = track.child(
            chrome_button(
                "previous-tabs",
                &LEFT,
                "Scroll previous tabs",
                26.,
                |s, tx| s.scroll_tabs(-1, tx),
            )
            .live_opacity(&s.tab_fade[0]),
        );
    }
    track = track.child(strip);
    if overflow {
        track = track.child(
            chrome_button("next-tabs", &RIGHT, "Scroll next tabs", 26., |s, tx| {
                s.scroll_tabs(1, tx)
            })
            .live_opacity(&s.tab_fade[1]),
        );
    }
    chip_press(chip(track).on_press(|_: &mut Workspace| {}))
}

/// A toolbar button at the right end: no fill at rest and a round hover inside, but the hit
/// area takes the bar's whole height (and the last one its right margin), so the corner above
/// Close closes.
fn bar_button(
    key: &str,
    i: &'static rune::Icon,
    label: &str,
    help: &str,
    width: f32,
    destructive: bool,
    f: impl Fn(&mut Workspace, &mut Tx) + 'static,
) -> El {
    let hover = if destructive {
        rgb(danger()).alpha(0.22)
    } else {
        white(0.09)
    };
    row()
        .key(key.to_owned())
        .role("button")
        .label(label.to_owned())
        .w(width)
        .h(TOOLBAR)
        .pr(width - 34.)
        .center_y()
        .justify_start()
        .cursor(Cursor::Pointer)
        .hover_group()
        .preserve_focus_on_press(true)
        .tab_index(-1)
        .help(help.to_owned())
        .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| {
            f(s, tx);
            tx.blur();
        })
        .child(
            el().size(30., 30.)
                .ml(2.)
                .radius(15.)
                .center()
                .child(
                    el().pin(Some(0.), Some(0.), Some(0.), Some(0.))
                        .radius(15.)
                        .fill(hover)
                        .group_opacity(0., 1.),
                )
                .child(glyph(i, 15., rgb(muted()))),
        )
}

pub fn toolbar(s: &Workspace) -> El {
    row()
        .key("toolbar")
        .w_full()
        .h(TOOLBAR)
        .pl(MARGIN)
        .gap(8.)
        .center_y()
        .window_drag()
        .on_secondary_press(|_: &mut Workspace, p, tx| tx.show_window_menu(p.x, p.y))
        .child(chrome_button(
            "toggle-sidebar",
            &SIDEBAR,
            "Toggle sidebar · Ctrl Shift B",
            30.,
            |s, tx| s.perform(Intent::Sidebar, tx),
        ))
        .child(identity(s))
        .child(if s.sidebar_tabs() { el() } else { tab_track(s) })
        .child(match s.bindings.sequence.label(s.effective_keymap()) {
            Some(label) => chip(
                el().key("pending-sequence")
                    .h(28.)
                    .max_w(170.)
                    .px(8.)
                    .center_y()
                    .help(format!(
                        "{label} — waiting for the next key; Escape cancels"
                    ))
                    .child(
                        text(format!("{label} …"))
                            .font(medium(11.5))
                            .color(rgb(ink()))
                            .truncate(),
                    ),
            ),
            None => el(),
        })
        // Always at least 48pt of window to drag, outside the tabs.
        .child(el().key("title-drag").grow(1.).min_w(48.).h_full())
        .child(
            row()
                .center_y()
                .child(bar_button(
                    "commands",
                    &COMMAND,
                    "Commands",
                    "Commands, sessions and tabs · Ctrl Shift P",
                    36.,
                    false,
                    |s, _| s.picker(false),
                ))
                .child(bar_button(
                    "new-tab",
                    &PLUS,
                    "New tab",
                    "New tab · Ctrl Shift T",
                    36.,
                    false,
                    |s, tx| s.perform(Intent::NewWindow, tx),
                ))
                .child(bar_button(
                    "settings",
                    &SETTINGS,
                    "Settings",
                    "Settings · Ctrl ,",
                    36.,
                    false,
                    |s, tx| s.perform(Intent::Settings, tx),
                ))
                .child(
                    el().key("toolbar-divider")
                        .w(1.)
                        .h(16.)
                        .mx(6.)
                        .fill(white(0.1)),
                )
                .child(bar_button(
                    "close-app",
                    &CLOSE,
                    "Close app",
                    "Close app · server programs keep running",
                    36. + MARGIN,
                    true,
                    |_, tx| tx.close_window(),
                )),
        )
}

impl Workspace {
    pub fn scroll_tabs(&mut self, direction: i32, tx: &mut Tx) {
        let step = (self.tab_strip.viewport * 0.6).max(80.);
        let [offset, max] = self.tab_pos.get();
        tx.scroll_to_x("tabs", (offset + step * direction as f32).clamp(0., max));
    }
}
