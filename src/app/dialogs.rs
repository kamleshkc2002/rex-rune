//! Modal forms, the destructive confirmations and notices.
use super::widgets::{Pill, glyph, pill};
use super::{
    panel::{Form, PanelKind},
    popups::{overlay, target_outline},
    theme::*,
    workspace::Workspace,
};
use rex_linux::settings::NoticePosition;
use rune::{Cursor, El, Tx, col, el, row, text};
use rune_kit::TextField;

fn modal(s: &Workspace, viewport: [f32; 2], body: El) -> El {
    let panel = col()
        .w(520f32.min(viewport[0] * 0.9))
        .max_h(viewport[1] * 0.85)
        .padding(22.)
        .gap(14.)
        .child(body);
    modal_frame(s, panel)
}

/// A centered dialog surface over a dark scrim; `surface` sets its own size and padding.
pub fn modal_frame(s: &Workspace, surface: El) -> El {
    modal_layer(s, surface, false)
}

/// [`modal_frame`], or with `under` the same layer and dialog drawn but inert, for a list
/// opened from one of the dialog's controls to sit on: the dialog keeps its place, its scroll
/// position and its rects, and takes no input until the list closes.
pub fn modal_layer(s: &Workspace, surface: El, under: bool) -> El {
    // Padding, dividers and disabled controls must not pass a press to the dismiss scrim.
    // Children still handle their own input; empty space leaves the keyboard focus alone.
    let name = s
        .panel
        .as_ref()
        .map(|p| p.title.clone())
        .unwrap_or_default();
    let surface = surface
        .key("dialog")
        .role("dialog")
        .label(name)
        .preserve_focus_on_press(true)
        .on_press(|_: &mut Workspace| {});
    let mut panel = glass(surface, 16., true).transition(pop_in());
    if under {
        panel = panel.inert();
    }
    let mut layer = overlay().center();
    // The list over it has the keyboard: a focus trap here would take focus for the layer
    // itself and keep the list's keys (handled with nothing focused) from reaching it.
    if !under {
        layer = layer.modal(false, |s: &mut Workspace| s.cancel_panel_quiet());
    }
    let mut layer = layer.child(
        el().key("scrim")
            .pin(Some(0.), Some(0.), Some(0.), Some(0.))
            // No blur: it would hide the pane a confirmation names, and re-blur every
            // frame while terminals stream behind it.
            .fill(black(0.5))
            .on_press_tx(|s: &mut Workspace, tx: &mut Tx| s.cancel_panel(tx)),
    );
    if let Some(outline) = target_outline(s) {
        layer = layer.child(outline);
    }
    layer.child(panel)
}

fn title(t: &str) -> El {
    text(t.to_owned()).font(strong(17.)).color(rgb(ink()))
}

fn form(s: &Workspace, form: &Form) -> El {
    let panel = s.panel.as_ref().expect("form panel");
    let k = kit();
    let mut body = col().gap(12.).child(title(&panel.title));
    for (i, edit) in panel.inputs.iter().enumerate() {
        let (label, placeholder) = &panel.fields[i];
        body = body.child(
            TextField::new(label.clone(), edit)
                .placeholder(placeholder.clone())
                .on_edit(move |s: &mut Workspace| s.edit_input(i))
                .build(k)
                .on_key(super::input::global_key)
                .key(format!("field:{i}")),
        );
    }
    if let Some(error) = &panel.error {
        body = body.child(
            text(error.clone())
                .font(ui(13.))
                .color(rgb(danger()))
                .key("form-error"),
        );
    }
    if let Form::EditHost(_) = form {
        body = body.child(
            text("A new endpoint connects to a different server; sessions on the old one keep running. The shell applies to new sessions, tabs and splits.")
                .font(ui(12.))
                .color(rgb(muted()))
                .wrap(),
        );
    }
    if let Form::AddSsh(index) = form {
        body = body.child(
            text(s.machine_form_note(*index))
                .font(ui(12.))
                .color(rgb(muted()))
                .wrap(),
        );
    }
    if let Form::Discovered(note) = form {
        body = body.child(
            col()
                .max_h(240.)
                .scroll(rune::ScrollSpec {
                    stick_bottom: false,
                })
                .child(text(note.clone()).font(ui(12.)).color(rgb(muted())).wrap()),
        );
    }
    if let Form::AddHost = form {
        body = body.child(
            text("https:// reaches a Rex server over the network (a bare host name is tried on port 443, then 7297); ssh://user@host[/socket] forwards a server's socket over SSH; unix:// is a socket on this machine.")
                .font(ui(12.))
                .color(rgb(muted()))
                .wrap(),
        );
    }
    if let Form::HostShell(_) = form {
        body = body.child(
            text("Used for new sessions, tabs and splits on this host. Running terminals are unchanged.")
                .font(ui(12.))
                .color(rgb(muted()))
                .wrap(),
        );
    }
    if let Form::Fonts = form {
        body = body.child(
            text("Applied when you reopen the client. --font and --font-fallback override saved choices for that launch. Server sessions keep running.")
                .font(ui(12.))
                .color(rgb(muted()))
                .wrap(),
        );
    }
    let primary = if matches!(form, Form::AddHost | Form::Discovered(_) | Form::AddSsh(_)) {
        "Connect"
    } else {
        "Save"
    };
    body.child(
        row()
            .gap(8.)
            .justify_end()
            .child(pill(
                "cancel-form",
                "Cancel",
                Pill::Neutral,
                32.,
                |s, tx| s.cancel_panel(tx),
            ))
            .child(
                pill("save-form", primary, Pill::Primary, 32., |s, tx| {
                    s.submit(tx)
                })
                .default_button(),
            ),
    )
}

fn confirm(panel_title: &str, description: &str) -> El {
    confirm_with(
        panel_title,
        description,
        "Keep running",
        "Close and end programs",
    )
}

fn confirm_with(panel_title: &str, description: &str, keep: &str, danger: &str) -> El {
    col()
        .gap(12.)
        .child(title(panel_title))
        .child(
            text(description.to_owned())
                .font(ui(13.))
                .color(rgb(muted()))
                .wrap(),
        )
        .child(
            row()
                .gap(8.)
                .justify_end()
                .child(
                    pill("cancel-close", keep, Pill::Neutral, 32., |s, tx| {
                        s.cancel_panel(tx)
                    })
                    .default_button()
                    .tab_index(1),
                )
                .child(
                    pill("confirm-close", danger, Pill::Danger, 32., |s, tx| {
                        s.submit(tx)
                    })
                    .tab_index(2),
                ),
        )
}

pub fn dialog(s: &Workspace, viewport: [f32; 2]) -> Option<El> {
    let panel = s.panel.as_ref()?;
    let body = match &panel.kind {
        PanelKind::Form(f) => form(s, f),
        PanelKind::Confirm { description, .. } => confirm(&panel.title, description),
        PanelKind::Settings => return Some(super::settings_view::settings(s, viewport)),
        PanelKind::RemoveHost(host) => {
            let label = &s.settings.hosts[*host].label;
            confirm_with(
                &panel.title,
                &format!(
                    "Rex Linux stops connecting to {label} and forgets it. Its sessions and programs keep running on that server, and you can add it again later."
                ),
                "Keep host",
                "Remove host",
            )
        }
        PanelKind::StartSsh { endpoint } => super::ssh_setup::dialog(s, endpoint),
        PanelKind::AgentChange { change, steps } => {
            super::integrations::confirm(s, &panel.title, change, steps)
        }
        _ => return None,
    };
    Some(modal(s, viewport, body))
}

fn toast(key: String, message: String, close: impl Fn(&mut Workspace) + 'static) -> El {
    glass(
        row()
            .key(key)
            .role("status")
            .w_full()
            .pl(14.)
            .pr(6.)
            .py(8.)
            .gap(10.)
            .center_y(),
        12.,
        false,
    )
    .fill(rgb(popover()).alpha(0.96))
    .transition(pop_in())
    .preserve_focus_on_press(true)
    .child(
        el().size(7., 7.)
            .radius(3.5)
            .fill(rgb(warning()))
            .box_shadow(shadow(0., 0., 8., rgb(warning()).alpha(0.6))),
    )
    .child(
        text(message)
            .font(ui(12.5))
            .color(rgb(ink()))
            .wrap()
            .grow(1.)
            .shrink(),
    )
    .child(
        row()
            .role("button")
            .label("Dismiss")
            .size(24., 24.)
            .center()
            .radius(8.)
            .hover(white(0.08))
            .cursor(Cursor::Pointer)
            .preserve_focus_on_press(true)
            .on_press(close)
            .child(glyph(&CLOSE, 13., rgb(muted()))),
    )
}

pub fn toasts(s: &Workspace) -> Option<El> {
    let host = s.model().notice.clone();
    let update = super::updates::toast_card(s);
    if s.notices.is_empty() && host.is_none() && update.is_none() {
        return None;
    }
    let region = col().key("toasts").role("status").w(340.).gap(8.);
    let mut region = match s.settings.notice_position {
        NoticePosition::TopRight => region.pin(None, Some(TOOLBAR + 4.), Some(MARGIN + 6.), None),
        NoticePosition::BottomRight => region.pin(None, None, Some(MARGIN + 6.), Some(MARGIN + 6.)),
    };
    if let Some(card) = update {
        region = region.child(card);
    }
    if let Some(text) = host {
        let active = s.active;
        region = region.child(toast(
            "toast-host".into(),
            text,
            move |s: &mut Workspace| s.dismiss_host_notice(active),
        ));
    }
    let room = 3usize
        .saturating_sub(usize::from(s.model().notice.is_some()))
        .saturating_sub(usize::from(
            s.updates.visible_notice(s.settings.updates.check).is_some(),
        ));
    for notice in s.notices.iter().rev().take(room) {
        let id = notice.id;
        region = region.child(toast(
            format!("toast-{id}"),
            notice.text.clone(),
            move |s: &mut Workspace| s.dismiss_notice(id),
        ));
    }
    Some(region)
}
