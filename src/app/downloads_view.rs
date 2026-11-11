//! The Download dialog and the Downloads popover. Both only read the workspace; every press
//! goes back to it (`downloads.rs`).
use super::{
    dialogs::title,
    downloads::{Dialog, Row, middle, progress_text, question, state_line, tilde, too_old},
    input::global_key,
    popups::overlay,
    theme::*,
    widgets::{Pill, pill},
    workspace::{HostId, Workspace},
};
use rex_linux::download::{self, State, Transfer};
use rune::{El, KeyEvent, LiveRegion, Tx, col, el, row, text};
use rune_kit::TextField;

/// How many characters of a name the chip and a row show before cutting it in the middle. The
/// full name is in the tooltip.
const NAME_CHARS: usize = 44;
const CHIP_NAME_CHARS: usize = 22;

fn note(message: String, tone: u32) -> El {
    text(message)
        .font(ui(13.))
        .color(rgb(tone))
        .wrap()
        .min_w(0.)
}

fn buttons(left: El, right: Option<El>) -> El {
    row().gap(8.).justify_end().child(left).children(right)
}

/// Enter applies a folder typed in the field; Escape is the dialog's own.
fn folder_key(s: &mut Workspace, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if ev.key == rune::Key::Enter {
        s.apply_download_folder();
        return true;
    }
    global_key(s, ev, tx)
}

/// The Download from <host> dialog: ask, wait for Rex, question, or an error before any bytes.
pub fn dialog(s: &Workspace, d: &Dialog) -> El {
    let panel = s.panel.as_ref().expect("download panel");
    let k = kit();
    let host = panel
        .target
        .as_ref()
        .map_or("this host", |t| s.settings.hosts[t.host].label.as_str());
    let mut body = col().gap(12.).child(title(&panel.title));

    // A host whose Rex can't send files: the message, and nothing to start.
    if panel
        .target
        .as_ref()
        .is_some_and(|t| s.download_unsupported(t.host))
        && d.id.is_none()
    {
        return body
            .child(note(too_old(host), warning()).key("download-old-server"))
            .child(buttons(
                pill("cancel-download", "Close", Pill::Neutral, 32., |s, tx| {
                    s.cancel_panel(tx)
                })
                .default_button(),
                None,
            ));
    }

    // Asked, and Rex has not answered, or has and wants a yes.
    if d.id.is_some() {
        let transfer = s.dialog_transfer();
        let name = transfer.map_or_else(
            || download::name::sanitize(panel.inputs[0].text.trim()),
            Transfer::display_name,
        );
        if let Some(Transfer {
            state: State::AwaitingConfirmation(confirmation),
            directory,
            resolved,
            ..
        }) = transfer
        {
            let mut b = body.child(
                note(question(&name, &tilde(directory), confirmation), ink())
                    .key("download-question"),
            );
            if let Some(path) = resolved {
                b = b.child(
                    text(format!("{host}: {path}"))
                        .font(ui(12.))
                        .color(rgb(muted()))
                        .wrap()
                        .min_w(0.)
                        .key("download-resolved"),
                );
            }
            return b.child(buttons(
                pill("cancel-download", "Cancel", Pill::Neutral, 32., |s, tx| {
                    s.cancel_panel(tx)
                })
                .default_button()
                .tab_index(1),
                Some(
                    pill(
                        "confirm-download",
                        "Download",
                        Pill::Primary,
                        32.,
                        |s, _| s.confirm_download(),
                    )
                    .label(format!("Download {name}"))
                    .tab_index(2),
                ),
            ));
        }
        return body
            .child(note(format!("Asking {host} for {name}…"), muted()).key("download-waiting"))
            .child(buttons(
                // Not the default button and out of the Tab order: an Enter still held down from
                // starting the download must not cancel it. Escape and a click still do.
                pill("cancel-download", "Cancel", Pill::Neutral, 32., |s, tx| {
                    s.cancel_panel(tx)
                })
                .focusable(false),
                None,
            ));
    }

    body = body.child(
        TextField::new(panel.fields[0].0.clone(), &panel.inputs[0])
            .placeholder(panel.fields[0].1.clone())
            .on_edit(|s: &mut Workspace| s.edit_input(0))
            .build(k)
            .on_key(global_key)
            .key("field:0"),
    );
    if d.changing {
        body = body
            .child(
                TextField::new(panel.fields[1].0.clone(), &panel.inputs[1])
                    .placeholder(panel.fields[1].1.clone())
                    .on_edit(|s: &mut Workspace| s.edit_input(1))
                    .build(k)
                    .on_key(folder_key)
                    .key("field:1"),
            )
            .child(
                text("An absolute folder, or one starting with ~/. Leave it empty for your system's Downloads folder.")
                    .font(ui(12.))
                    .color(rgb(muted()))
                    .wrap()
                    .min_w(0.),
            );
    } else {
        let folder = tilde(&s.download_folder());
        body = body.child(
            row()
                .key("save-in")
                .role("group")
                .label(format!("Save in {folder}"))
                .w_full()
                .gap(10.)
                .center_y()
                .child(
                    text("Save in")
                        .font(medium(12.))
                        .color(rgb(muted()))
                        .a11y_hidden(),
                )
                .child(
                    text(middle(&folder, 42))
                        .key("save-in-folder")
                        .font(ui(13.))
                        .color(rgb(ink()))
                        .truncate()
                        .help_when_truncated(folder)
                        .grow(1.)
                        .shrink()
                        .a11y_hidden(),
                )
                .child(
                    pill("change-folder", "Change", Pill::Neutral, 26., |s, _| {
                        s.change_download_folder()
                    })
                    .label("Change the folder downloads are saved in"),
                ),
        );
    }
    if let Some(error) = &panel.error {
        body = body.child(
            note(error.clone(), danger())
                .role("alert")
                .live_region(LiveRegion::Assertive)
                .key("download-error"),
        );
    }
    let cancel = if d.changing {
        pill("cancel-download", "Cancel", Pill::Neutral, 32., |s, _| {
            s.stop_changing_folder()
        })
    } else {
        pill("cancel-download", "Cancel", Pill::Neutral, 32., |s, tx| {
            s.cancel_panel(tx)
        })
    };
    let go = if d.changing {
        pill("use-folder", "Use folder", Pill::Primary, 32., |s, _| {
            s.apply_download_folder()
        })
    } else {
        pill("start-download", "Download", Pill::Primary, 32., |s, _| {
            s.start_download()
        })
    };
    body.child(buttons(cancel, Some(go.default_button())))
}

/// One row of the popover: name, host and state, and the buttons that state allows.
fn download_row(s: &Workspace, r: &Row<'_>) -> El {
    let t = r.transfer;
    let name = t.display_name();
    let id = t.id;
    let host: HostId = r.host;
    let state = state_line(r.label, t);
    let (tone, running) = match &t.state {
        State::Failed(_) => (danger(), false),
        State::Requested | State::AwaitingConfirmation(_) | State::Receiving { .. } => {
            (muted(), true)
        }
        _ => (muted(), false),
    };
    let tooltip = match &t.resolved {
        Some(path) => format!("{}: {path}", r.label),
        None => format!("{}: {}", r.label, t.requested),
    };
    // Keyed by state: when a transfer ends, its Cancel is gone, and focus must not slide onto
    // the Show in folder or Open that takes its place.
    let mut actions = row()
        .key(format!(
            "download-actions:{id}:{}",
            if running { "running" } else { "done" }
        ))
        .gap(6.)
        .center_y();
    if running {
        actions = actions.child(
            pill(
                &format!("download-cancel:{id}"),
                "Cancel",
                Pill::Neutral,
                24.,
                move |s, _| s.cancel_download(host, id),
            )
            .label(format!("Cancel download of {name}")),
        );
    }
    if matches!(t.state, State::Saved { .. }) {
        actions = actions.child(
            pill(
                &format!("download-show:{id}"),
                "Show in folder",
                Pill::Neutral,
                24.,
                move |s, _| s.show_download(host, id),
            )
            .label(format!("Show {name} in its folder")),
        );
        if s.downloadable(host, id) {
            actions = actions.child(
                pill(
                    &format!("download-open:{id}"),
                    "Open",
                    Pill::Neutral,
                    24.,
                    move |s, _| s.open_download(host, id),
                )
                .label(format!("Open {name}")),
            );
        }
    }
    let mut details = col()
        .grow(1.)
        .shrink()
        .min_w(0.)
        .gap(2.)
        .child(
            text(middle(&name, NAME_CHARS))
                .key(format!("download-name:{id}"))
                .font(medium(13.))
                .color(rgb(ink()))
                .truncate()
                .help(tooltip),
        )
        .child(
            text(format!("{} · {state}", r.label))
                .key(format!("download-state:{id}"))
                .font(ui(11.5))
                .color(rgb(tone))
                .wrap()
                .min_w(0.),
        );
    if let State::Receiving { received, hint } = &t.state {
        let fraction = t.fraction();
        let bar = row()
            .key(format!("download-progress:{id}"))
            .role("progressbar")
            .label(format!("Progress of {name}"))
            .w_full()
            .h(3.)
            .mt(3.)
            .radius(1.5)
            .a11y_value(progress_text(*received, *hint));
        details = details.child(match fraction {
            Some(f) => {
                let f = f.clamp(0.02, 1.);
                bar.range_value(f64::from(f), 0., 1.)
                    .fill(white(0.08))
                    .child(el().grow(f).h_full().radius(1.5).fill(rgb(accent())))
                    .child(el().grow((1. - f).max(0.0001)))
            }
            None => bar.fill(rgb(accent())).opacity(0.35),
        });
    }
    row()
        .key(format!("download:{id}"))
        .role("listitem")
        .label(format!("{name}, {}, {state}", r.label))
        .w_full()
        .px(10.)
        .py(8.)
        .gap(10.)
        .center_y()
        .child(details)
        .child(actions)
}

/// The Downloads popover: every transfer of this launch, running first.
pub fn popover(s: &Workspace, viewport: [f32; 2]) -> El {
    let [vw, vh] = viewport;
    let width = 440f32.min(vw - 16.);
    let rows = s.download_rows();
    let mut list = col()
        .key("downloads-list")
        .role("list")
        .label("Downloads")
        .w_full();
    if rows.is_empty() {
        list = list.child(
            col().px(12.).py(14.).child(
                text("No downloads yet. Right-click a pane and choose Download file…")
                    .key("downloads-empty")
                    .font(ui(13.))
                    .color(rgb(muted()))
                    .wrap()
                    .min_w(0.),
            ),
        );
    }
    for r in &rows {
        list = list.child(download_row(s, r));
    }
    let surface = glass(
        col()
            .key("downloads-popover")
            .role("dialog")
            .label("Downloads")
            .w(width)
            .max_h(vh - TOOLBAR - 16.)
            .padding(5.)
            .gap(1.)
            .preserve_focus_on_press(true),
        12.,
        true,
    )
    .clip_rounded(12.)
    .transition(pop_in())
    .child(
        row().h(26.).px(10.).center_y().child(
            text("Downloads")
                .font(medium(11.5))
                .color(rgb(muted()))
                .truncate(),
        ),
    )
    .child(
        col()
            .key("downloads-scroll")
            .w_full()
            .min_h(0.)
            // Opens with focus here, not on a button: a stray Enter does nothing.
            .focusable(true)
            .autofocus()
            .scroll(rune::ScrollSpec {
                stick_bottom: false,
            })
            .child(list),
    );
    let x = (vw - width - MARGIN - 6.).max(8.);
    overlay()
        .modal_tx(true, |s: &mut Workspace, tx: &mut Tx| s.close_panel(tx))
        .child(surface.absolute(x, TOOLBAR - 4.))
}

/// The chip's text: `↓ build.log 42%` for one transfer, `↓ 3 files` for several.
pub fn chip_label(s: &Workspace) -> Option<(String, Option<String>)> {
    let rows = s.download_rows();
    let running: Vec<&Row> = rows
        .iter()
        .filter(|r| !r.transfer.state.is_final())
        .collect();
    match running.as_slice() {
        [] => None,
        [one] => Some((
            format!(
                "↓ {}",
                middle(&one.transfer.display_name(), CHIP_NAME_CHARS)
            ),
            one.transfer
                .fraction()
                .map(|f| format!("{}%", (f * 100.).floor() as u32)),
        )),
        many => Some((format!("↓ {} files", many.len()), None)),
    }
}
