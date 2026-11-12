//! The 52pt bar on the window ground: sidebar toggle, identity chip, tab track, drag space,
//! Commands, New tab and Settings, a divider, and Close. Tooltips name each control's shortcut
//! from the effective keymap (`keycaps.rs`), never from a literal.
use super::{
    activity::{progress_bar, tab_activity, tab_name, tab_programs, tab_progress, tab_word},
    panel::Intent,
    tab_icons,
    tabs::{muted_mark, target_ring},
    theme::*,
    widgets::{chrome_button, glyph},
    workspace::Workspace,
};
use rune::{Cursor, El, PointerButton, ScrollPos, Tx, col, el, row, text};

/// The identity chip's width limits.
const IDENTITY_MIN: f32 = 120.;
const IDENTITY_MAX: f32 = 210.;

/// A chip's press: consume it (chips never drag the window) and keep the terminal focused.
fn chip_press(e: El) -> El {
    e.cursor(Cursor::Pointer)
        .preserve_focus_on_press(true)
        .tab_index(-1)
}

/// The host line under the session's name: trouble always, and otherwise the host only while
/// a second host is connected, when the session's name alone could be either's.
fn identity_detail(s: &Workspace) -> Option<(String, u32)> {
    let host = &s.settings.hosts[s.active].label;
    if !s.model().connected {
        Some((format!("{host} · Reconnecting"), warning()))
    } else if s.hosts.iter().filter(|h| h.model.connected).count() > 1 {
        Some((host.clone(), muted()))
    } else {
        None
    }
}

fn identity(s: &Workspace) -> El {
    let model = s.model();
    let host = &s.settings.hosts[s.active].label;
    let session = model
        .view()
        .and_then(|v| v.label.clone())
        .unwrap_or_else(|| "Choose session".into());
    let mut words = col().grow(1.).shrink().min_w(0.).gap(1.).child(
        text(session.clone())
            .font(strong(12.5))
            .color(rgb(ink()))
            .truncate(),
    );
    if let Some((detail, tone)) = identity_detail(s) {
        words = words.child(
            text(detail)
                .key("identity-host")
                .font(ui(10.5))
                .color(rgb(tone))
                .truncate(),
        );
    }
    chip_press(chip(
        row()
            .key("identity")
            .role("button")
            .label(format!("{session} · {host} · Switch session"))
            // As wide as its words, between a floor that keeps it a comfortable target and
            // the width a long name truncates at.
            .min_w(IDENTITY_MIN)
            .max_w(IDENTITY_MAX)
            .h(34.)
            .pl(6.)
            .pr(8.)
            .gap(9.)
            .center_y()
            .hover(white(0.08))
            .help(format!(
                "{session} · {host} · {}",
                s.shortcut_help("Switch session", "session.switch")
            ))
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
            // The words take any room the floor leaves, so the chevron keeps to the edge.
            .child(words)
            .child(glyph(&CHEVRON, 14., rgb(muted()))),
    ))
}

/// The fill-mode floor reserves 40 pt of name, the actual icon strip and padding, and
/// whole activity/mute marks. Status words are short; 7 pt per character is generous.
fn fill_floor(icons: usize, word: Option<&str>, quiet: bool) -> f32 {
    (28. + tab_icons::leading_width(icons)
        + 40.
        + word.map_or(0., |word| 8. + word.chars().count() as f32 * 7.)
        + if quiet { 8. + 14. } else { 0. })
    .max(96.)
}

/// The session's tabs on their track. With no tab to show (no session on screen, or one whose
/// view hasn't arrived) there is no track: an empty one would be a sliver of chip.
fn tab_track(s: &Workspace) -> El {
    let model = s.model();
    let tabs = model
        .view()
        .is_some_and(|view| view.windows.iter().any(|tab| tab.window_id.is_some()));
    if !tabs {
        return el();
    }
    let session = model.selected.clone().unwrap_or_default();
    let fill = s.settings.interface.tabs_fill_available_space;
    let mut floors = -2.;
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
            let label = tab_name(model, tab, s.generated_titles());
            let activity = tab_activity(model, tab);
            let icons = tab_icons::icons_of(
                &tab_programs(model, tab),
                s.settings.interface.tab_icon_count,
            );
            let shown = s.tab_activity_shown(s.active, &id);
            let quiet = s.tab_muted(s.active, &id);
            let word =
                tab_word(&activity, s.unseen(&id), s.bell_on(s.active, &id)).filter(|_| shown);
            let (press_id, rename_id, menu_id) = (id.clone(), id.clone(), id.clone());
            let menu_session = session.clone();
            let mut tab_el = row()
                .key(format!("tab:{id}"))
                .role("tab")
                .focus_selected(active)
                .label(match &word {
                    Some((word, _)) => format!("{label} · {word}"),
                    None if quiet => format!("{label} · Muted"),
                    None => label.clone(),
                })
                .h_full()
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
                    s.tab_menu(s.active, &menu_session, &menu_id, (p.x, p.y))
                });
            tab_el = if fill {
                let floor = fill_floor(
                    icons.len(),
                    word.as_ref().map(|(word, _)| word.as_ref()),
                    quiet,
                );
                floors += floor + 2.;
                tab_el.grow(1.).min_w(floor).justify_center()
            } else {
                tab_el.min_w(96.).max_w(240.)
            };
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
            tab_el = tab_icons::append(tab_el, &format!("tab-icon:{id}"), &icons, active);
            tab_el = tab_el.child(
                text(label.clone())
                    .key(format!("tab-name:{id}"))
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
            if quiet {
                tab_el = tab_el.child(muted_mark(&id));
            }
            if shown && let Some(progress) = tab_progress(&activity) {
                tab_el = tab_el.child(progress_bar(format!("tab-progress:{id}"), progress, 14.));
            }
            if s.tab_targeted(s.active, &id) {
                tab_el = tab_el.child(target_ring(&id, 8.));
            }
            names = names.child(tab_el);
        }
    }
    if fill {
        names = names.grow(1.).min_w(floors.max(0.));
    }
    let overflow = s.tab_strip.overflows();
    let (fade, pos) = (s.tab_fade.clone(), s.tab_pos.clone());
    let mut strip = row()
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
    if fill {
        track = track.grow(1.);
        strip = strip.grow(1.);
    }
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

/// One chip while a transfer receives, `↓ build.log 42%` or `↓ 3 files`, and none otherwise,
/// like the pending-sequence chip. When the bar is short of room it is `↓ 42%` (see
/// `downloads_view::chip_label`). A press opens the Downloads popover under the chip.
fn download_chip(s: &Workspace) -> El {
    let Some(text_of) = super::downloads_view::chip_label(s) else {
        return el();
    };
    let super::downloads_view::ChipText {
        label,
        short,
        compact,
        percent,
    } = text_of;
    let spoken = match &percent {
        Some(percent) => format!("{label} {percent}"),
        None => label.clone(),
    };
    let spoken = format!("Downloads: {}", spoken.trim_start_matches("↓ "));
    let shown = if compact { short } else { label };
    chip_press(chip(
        row()
            .key("download-chip")
            .role("button")
            .label(spoken.clone())
            .h(28.)
            .max_w(190.)
            .px(8.)
            .gap(5.)
            .center_y()
            .hover(white(0.08))
            .help(format!("{spoken} · click for the list"))
            .on_layout(|s: &mut Workspace, r| s.downloads.chip = r)
            .on_press_tx(|s: &mut Workspace, tx: &mut Tx| {
                s.open_downloads_from_chip(tx);
                tx.blur();
            })
            // Cut by width in the middle, so a long name keeps its extension and the percent
            // keeps its place.
            .child(
                text(shown)
                    .font(medium(11.5))
                    .color(rgb(ink()))
                    .truncate_middle()
                    .shrink(),
            )
            .children(percent.map(|p| text(p).font(medium(11.5)).color(rgb(muted())))),
    ))
}

/// A pending key sequence remains visible even without the toolbar row.
pub fn sequence_chip(s: &Workspace) -> Option<El> {
    if !s.settings.terminal.key_sequence_hints {
        return None;
    }
    let label = s.bindings.pending_label(s.effective_keymap())?;
    Some(chip(
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
    ))
}

/// Only live status chips float when the toolbar is absent, never invisible bar controls.
pub fn floating_status(s: &Workspace) -> Option<El> {
    let sequence = sequence_chip(s);
    let download = super::downloads_view::chip_label(s).is_some();
    if sequence.is_none() && !download {
        return None;
    }
    Some(
        row()
            .gap(8.)
            .children(sequence)
            .children(download.then(|| download_chip(s))),
    )
}

pub fn toolbar(s: &Workspace) -> El {
    let fill = s.settings.interface.tabs_fill_available_space
        && !s.sidebar_tabs()
        && s.model()
            .view()
            .is_some_and(|v| v.windows.iter().any(|t| t.window_id.is_some()));
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
            &s.shortcut_help("Toggle sidebar", "client.sidebar.toggle"),
            30.,
            |s, tx| s.perform(Intent::Sidebar, tx),
        ))
        .child(identity(s))
        .child(if s.sidebar_tabs() { el() } else { tab_track(s) })
        .child(sequence_chip(s).unwrap_or_else(el))
        .child(download_chip(s))
        // Always at least 48pt of window to drag, outside the tabs.
        .child(
            el().key("title-drag")
                .grow(if fill { 0. } else { 1. })
                .min_w(48.)
                .h_full(),
        )
        .child(
            row()
                .center_y()
                .child(bar_button(
                    "commands",
                    &COMMAND,
                    "Commands",
                    &s.shortcut_help("Commands, sessions and tabs", "client.palette.toggle"),
                    36.,
                    false,
                    |s, _| s.picker(false),
                ))
                .child(bar_button(
                    "new-tab",
                    &PLUS,
                    "New tab",
                    &s.shortcut_help("New tab", "client.tab.new"),
                    36.,
                    false,
                    |s, tx| s.perform(Intent::NewWindow, tx),
                ))
                .child(bar_button(
                    "settings",
                    &SETTINGS,
                    "Settings",
                    &s.shortcut_help("Settings", "client.settings"),
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
