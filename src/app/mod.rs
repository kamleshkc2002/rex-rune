//! The Rune front end.
mod about;
mod actions;
mod activity;
mod bindings;
mod commands;
mod compatibility;
mod dialogs;
mod discover;
mod downloads;
mod downloads_view;
mod find;
mod font_size;
mod fonts;
mod help;
mod input;
mod integrations;
mod keycaps;
mod keys;
mod navigation;
mod notify;
mod pane_header;
mod pane_moves;
mod pane_verbs;
mod panel;
mod panes;
mod popups;
mod recorder;
mod remote;
mod settings_kit;
mod settings_nav;
mod settings_shell;
mod settings_view;
mod shortcuts_page;
mod sidebar;
mod splits;
mod sprites;
mod ssh_setup;
mod stats;
mod tabs;
mod term;
mod theme;
mod titles;
mod toolbar;
mod updates;
mod upload;
mod widgets;
mod workspace;

use rex_linux::{connection::Options, settings::Settings};
use rune::{AppSpec, Cursor, Cx, El, Tx, WindowEdge, col, el, row};
use std::time::Duration;
use theme::*;
use workspace::Workspace;

const APP_ICON: &[u8] = include_bytes!("../../resources/icons/256x256/apps/dev.shuv.rex-linux.png");

fn resize_band(e: WindowEdge) -> El {
    let b = 6.;
    let corner = 12.;
    let el = el()
        .key(format!("resize:{e:?}"))
        .window_resize(e)
        .cursor(Cursor::ResizeEdge(e));
    match e {
        WindowEdge::North => el.pin(Some(corner), Some(0.), Some(corner), None).h(b),
        WindowEdge::South => el.pin(Some(corner), None, Some(corner), Some(0.)).h(b),
        WindowEdge::West => el.pin(Some(0.), Some(corner), None, Some(corner)).w(b),
        WindowEdge::East => el.pin(None, Some(corner), Some(0.), Some(corner)).w(b),
        WindowEdge::NorthWest => el.pin(Some(0.), Some(0.), None, None).size(corner, corner),
        WindowEdge::NorthEast => el.pin(None, Some(0.), Some(0.), None).size(corner, corner),
        WindowEdge::SouthWest => el.pin(Some(0.), None, None, Some(0.)).size(corner, corner),
        WindowEdge::SouthEast => el.pin(None, None, Some(0.), Some(0.)).size(corner, corner),
    }
}

pub(crate) fn view(s: &Workspace, cx: &mut Cx<Workspace>) -> El {
    set_palette(palette_of(s.settings.chrome_theme));
    let scale = s.ui_scale();
    let viewport = s.viewport.map(|v| v / scale);
    let maximized = cx.window_maximized();
    let mut body = row().key("body").grow(1.).w_full().min_h(0.);
    if s.sidebar.is_open() {
        body = body
            .child(sidebar::sidebar(s, cx))
            .child(sidebar::sidebar_edge(s));
    }
    // Device pixels per point, so terminal box drawing lands on whole pixels.
    body = body.child(panes::area(s, cx.env().scale * scale));
    let mut root = el()
        .key("root")
        .absolute(0., 0.)
        .size(viewport[0], viewport[1])
        .origin(0., 0.)
        .scale(scale)
        .scale_motion(STILL)
        .fill(rgb(ground()))
        .child(
            col()
                .absolute(0., 0.)
                .size(viewport[0], viewport[1])
                .pt(TOOLBAR)
                .child(body),
        )
        .child(toolbar::toolbar(s).absolute(0., 0.));
    if let Some(toasts) = dialogs::toasts(s) {
        root = root.child(toasts);
    }
    let open = match &s.panel {
        Some(panel) if panel.is_list() => Some(popups::list_popup(s, viewport)),
        Some(panel) if matches!(panel.kind, panel::PanelKind::Downloads) => {
            Some(downloads_view::popover(s, viewport))
        }
        Some(_) => dialogs::dialog(s, viewport),
        None => None,
    };
    root = root.child(open.unwrap_or_else(popups::overlay));
    let mut window = el()
        .key("window")
        .pointer_buttons([rune::PointerButton::Back, rune::PointerButton::Forward])
        .on_press_tx(|s: &mut Workspace, tx: &mut Tx| {
            if s.panel.is_some() || s.drag.is_some() {
                return;
            }
            let action = match tx.button() {
                rune::PointerButton::Back => actions::HISTORY_BACK,
                rune::PointerButton::Forward => actions::HISTORY_FORWARD,
                _ => return,
            };
            actions::run(s, action, &serde_json::Value::Null, tx);
        })
        // With no focused control, Rune sends releases only to the outermost node.
        .on_key_up(input::on_key_up)
        // Rune delivers native window focus to the view's first element only.
        .on_window_focus(|s: &mut Workspace, focused, _| s.window_focus(focused))
        .w_full()
        .h_full()
        .fill(rgb(ground()))
        .on_layout(move |s: &mut Workspace, r| s.window_resized([r[2], r[3]], maximized))
        .child(root);
    // Native resize targets stay in window points at every content zoom. The corner above
    // Close belongs to Close rather than the resize band.
    if !cx.window_maximized() {
        for edge in [
            WindowEdge::South,
            WindowEdge::West,
            WindowEdge::East,
            WindowEdge::SouthWest,
            WindowEdge::SouthEast,
            WindowEdge::North,
            WindowEdge::NorthWest,
        ] {
            window = window.child(resize_band(edge));
        }
    }
    window
}

fn poll(s: &mut Workspace, _: f64, tx: &mut Tx) -> bool {
    let now = std::time::Instant::now();
    let remote_changed = s.poll_remote_navigation(now);
    let updates_changed = s.poll_updates(now, tx);
    let settings_changed = s.poll_settings(now, tx);
    let mut changed =
        s.poll_navigation(now, tx) || remote_changed || updates_changed || settings_changed;
    if let Some(panel) = &mut s.panel
        && std::mem::take(&mut panel.reveal)
    {
        tx.scroll_into_view(format!("entry:{}", panel.selected));
        changed = true;
    }
    // Reveal the active tab after selection, resize and sidebar changes, and again on the
    // next frame once the strip has its new width.
    // A tab that moved within its session's order is revealed again where it now is.
    let place = s.active_tab_place();
    let tab = place.as_ref().map(|(id, _)| id.clone());
    let reveal = (
        place,
        s.area[2].round() as i32,
        s.sidebar.is_open(),
        s.bar_chips(),
    );
    let again = std::mem::take(&mut s.reveal_tab_again);
    if reveal != s.last_tab || again {
        if let Some(id) = &tab {
            tx.scroll_into_view(format!("tab:{id}"));
        }
        s.reveal_tab_again = reveal != s.last_tab;
        s.last_tab = reveal;
        changed = true;
    }
    if s.expire_notices() {
        changed = true;
    }
    let saving_size = s.resized.is_some();
    if let Some(next) = s.save_window_size() {
        tx.wake_after(next);
    } else if saving_size {
        // Saving may have added a Preferences not saved notice.
        changed = true;
    }
    let failed: Vec<String> = s
        .terminals
        .values_mut()
        .filter_map(|t| t.notice.take())
        .collect();
    for notice in failed {
        s.notify(notice);
        changed = true;
    }
    if s.notices.iter().any(|n| !n.sticky) {
        tx.wake_after(Duration::from_secs(1));
    }
    // A pane waiting for a frame at its size draws anyway at its deadline; be awake for it.
    let now = std::time::Instant::now();
    for term in s.terminals.values_mut() {
        if term.draw_anyway(now) {
            changed = true;
        } else if let Some(deadline) = term.draws_anyway_at() {
            tx.wake_after(deadline - now);
        }
    }
    changed
}

#[cfg(test)]
mod physical_input_tests;
#[cfg(test)]
mod tests;

pub(crate) fn placeholder() -> Workspace {
    unreachable!("App::init builds the workspace")
}

pub fn run(mut options: Options) -> anyhow::Result<()> {
    let (settings, path) = match &options.config {
        Some(path) => (Settings::load(path)?, path.clone()),
        None => Settings::load_default()?,
    };
    let size = options.startup_size(&settings);
    let (family, fallbacks) = options.fonts(&settings);
    let fonts = fonts::load(family, fallbacks);
    rune::text::set_app_font_paths(fonts.paths)
        .map_err(|_| anyhow::anyhow!("terminal fonts were already initialized"))?;
    rune::set_app_id("dev.shuv.rex-linux");
    if let Err(error) = rune::set_app_icon(APP_ICON) {
        eprintln!("Could not load the Rex window icon: {error}");
    }
    Workspace::clean_up_download_leftovers();
    let rune_args = std::mem::take(&mut options.rune);
    let app = rune::App::new(AppSpec {
        min_size: Some((640., 480.)),
        fonts: fonts.files,
        families: fonts.families,
        setup: Some(|s: &mut Workspace, wake| {
            let images = rune::ImageCache::new(wake);
            s.logo = Some(images.load(APP_ICON));
            s.images = Some(images);
        }),
        on_key: Some(input::on_key),
        poll: Some(poll),
        ..AppSpec::new("Rex", size, placeholder, view)
    });
    let handle = app.handle();
    let terminal = fonts.terminal;
    let mut app = app
        .init(move || Workspace::new(options, settings, path, handle, terminal))
        .args(rune_args)
        .window_options(rune::WindowOptions {
            borderless: true,
            min_size: Some((640., 480.)),
            ..Default::default()
        })
        .on_ime(input::on_ime)
        .srgb_blend(true);
    if stats::FrameStats::enabled() {
        let mut frames = stats::FrameStats::default();
        app = app.on_frame_encoded(move |f| frames.record(f));
    }
    app.run();
    // The window is gone and the workspace with it: every host connection has been told to shut
    // down. Give the host threads a moment to drop their transfers, so a download in flight
    // deletes its partial file before the process ends (the next launch sweeps whatever a host
    // stuck connecting leaves behind).
    wait_for_hosts_at_exit();
    Ok(())
}

/// The longest the app waits for its host threads once the window is closed.
const EXIT_WAIT: std::time::Duration = std::time::Duration::from_secs(2);

fn wait_for_hosts_at_exit() {
    rex_linux::connection::wait_for_hosts(EXIT_WAIT);
}
