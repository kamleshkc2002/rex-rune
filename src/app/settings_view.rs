//! Settings: a section list on the left and the chosen section's controls on the right.
use super::{
    dialogs::modal_frame,
    panel::{Intent, SettingsSection},
    theme::*,
    widgets::{Pill, glyph, pill},
    workspace::Workspace,
};
use rex_linux::settings::{NoticePosition, NotificationsConfig, PaneDensity, PaneHeader};
use rune::{Cursor, El, ScrollSpec, Tx, col, el, row, text};
use rune_kit::{IconButton, Keybind, SegmentedControl, Switch};

/// A labelled setting: name and optional explanation on the left, its control on the right.
pub(super) fn setting(label: &str, detail: &str, control: El) -> El {
    let mut words = col()
        .grow(1.)
        .min_w(0.)
        .gap(2.)
        .child(text(label.to_owned()).font(ui(13.)).color(rgb(ink())));
    if !detail.is_empty() {
        words = words.child(
            text(detail.to_owned())
                .font(ui(11.5))
                .color(rgb(muted()))
                .wrap(),
        );
    }
    row()
        .w_full()
        .min_h(44.)
        .gap(16.)
        .center_y()
        .child(words)
        .child(control)
}

pub(super) fn group(name: &str) -> El {
    text(name.to_owned())
        .font(label_caps())
        .text_case(rune::TextCase::Uppercase)
        .color(rgb(muted()))
        .mt(10.)
}

pub(super) fn divider() -> El {
    el().w_full().h(1.).fill(white(0.06))
}

/// A stepper: − value +, with Return acting on the focused button as Space does.
fn stepper(
    key: &str,
    value: String,
    down: impl Fn(&mut Workspace, &mut Tx) + Clone + 'static,
    up: impl Fn(&mut Workspace, &mut Tx) + Clone + 'static,
) -> El {
    let k = kit();
    let (down2, up2) = (down.clone(), up.clone());
    row()
        .gap(8.)
        .center_y()
        .child(
            IconButton::new(&MINUS)
                .label("Smaller")
                .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| down(s, tx))
                .build(k)
                .key(format!("{key}-smaller"))
                .on_key(move |s: &mut Workspace, ev: &rune::KeyEvent, tx: &mut Tx| {
                    (ev.key == rune::Key::Enter).then(|| down2(s, tx)).is_some()
                }),
        )
        .child(
            row()
                .w(52.)
                .center()
                .child(text(value).font(medium(13.)).color(rgb(ink()))),
        )
        .child(
            IconButton::new(&PLUS)
                .label("Larger")
                .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| up(s, tx))
                .build(k)
                .key(format!("{key}-larger"))
                .on_key(move |s: &mut Workspace, ev: &rune::KeyEvent, tx: &mut Tx| {
                    (ev.key == rune::Key::Enter).then(|| up2(s, tx)).is_some()
                }),
        )
}

/// The Pane headers choices, in the order the control lists them.
const PANE_HEADERS: [PaneHeader; 4] = [
    PaneHeader::Auto,
    PaneHeader::Always,
    PaneHeader::OnHover,
    PaneHeader::Hidden,
];

fn interface(s: &Workspace) -> El {
    let k = kit();
    let percent = (s.settings.ui_scale * 100.).round() as i32;
    let corner = match s.settings.notice_position {
        NoticePosition::TopRight => 0,
        NoticePosition::BottomRight => 1,
    };
    col()
        .gap(4.)
        .child(setting(
            "Theme",
            "The window's colours. Until you pick a terminal theme, terminals take the matching one (Follow chrome).",
            row()
                .gap(10.)
                .center_y()
                .child(
                    text(s.settings.chrome_theme.name())
                        .font(medium(13.))
                        .color(rgb(ink()))
                        .truncate()
                        .max_w(160.),
                )
                .child(pill("chrome-theme", "Change…", Pill::Neutral, 26., |s, tx| {
                    s.perform(Intent::ChooseChrome, tx)
                })),
        ))
        .child(divider())
        .child(setting(
            "Interface zoom",
            "Chrome and terminals together, 70–150%. Ctrl + and Ctrl − also work.",
            row()
                .gap(8.)
                .center_y()
                .child(stepper(
                    "zoom",
                    format!("{percent}%"),
                    |s, tx| s.change_ui_scale(-1, tx),
                    |s, tx| s.change_ui_scale(1, tx),
                ))
                .child(pill("zoom-reset", "Reset", Pill::Neutral, 26., |s, tx| {
                    s.change_ui_scale(0, tx)
                })),
        ))
        .child(divider())
        .child(setting(
            "Pane density",
            "Compact uses straight separators, without card borders or gaps.",
            SegmentedControl::new(
                &["Compact", "Comfortable"],
                Some(usize::from(s.settings.pane_density == PaneDensity::Comfortable)),
            )
            .fit()
            .on_pick(|s: &mut Workspace, i, tx| {
                s.settings.pane_density = if i == 0 {
                    PaneDensity::Compact
                } else {
                    PaneDensity::Comfortable
                };
                s.save();
                tx.instant();
            })
            .build(k)
            .key("pane-density"),
        ))
        .child(setting(
            "Pane headers",
            "Auto: on split panes in Comfortable density. On hover: at a pane's top edge.",
            SegmentedControl::new(
                &["Auto", "Always", "On hover", "Hidden"],
                PANE_HEADERS
                    .iter()
                    .position(|h| *h == s.settings.pane_header),
            )
            .fit()
            .on_pick(|s: &mut Workspace, i, tx| {
                s.settings.pane_header = PANE_HEADERS[i];
                // A header that was brought out by hand goes back with the old choice.
                s.header_reveal = Default::default();
                s.save();
                tx.instant();
            })
            .build(k)
            .key("pane-header"),
        ))
        .child(divider())
        .child(setting(
            "Session sidebar",
            "Hosts and their sessions beside the terminals. Ctrl Shift B.",
            Switch::new(s.sidebar.is_open())
                .label("Show session sidebar")
                .hide_label()
                .on_toggle_tx(|s: &mut Workspace, tx: &mut Tx| s.perform(Intent::Sidebar, tx))
                .build(k)
                .key("sidebar-switch"),
        ))
        .child(setting(
            "Tab position",
            "In the toolbar, or under each session in the sidebar while it's open.",
            SegmentedControl::new(
                &["Top", "Sidebar"],
                Some(usize::from(s.settings.vertical_tabs)),
            )
            .fit()
            .on_pick(|s: &mut Workspace, i, _| s.set_vertical_tabs(i == 1))
            .build(k)
            .key("tab-position"),
        ))
        .child(divider())
        .child(setting(
            "Notices",
            "Where in-app notices appear.",
            SegmentedControl::new(&["Top right", "Bottom right"], Some(corner))
                .fit()
                .on_pick(|s: &mut Workspace, i, _| {
                    s.set_notice_position(if i == 0 {
                        NoticePosition::TopRight
                    } else {
                        NoticePosition::BottomRight
                    })
                })
                .build(k)
                .key("notice-position"),
        ))
}

/// Where a notification switch keeps its value.
type Field = fn(&mut NotificationsConfig) -> &mut bool;

/// A switch for one notification preference, saved as soon as it flips.
fn notify_switch(key: &str, label: &str, on: bool, disabled: bool, field: Field) -> El {
    Switch::new(on)
        .label(label.to_owned())
        .hide_label()
        .disabled(disabled)
        .on_toggle(move |s: &mut Workspace| {
            let value = field(&mut s.settings.notifications);
            *value = !*value;
            s.save();
        })
        .build(kit())
        .key(key.to_owned())
}

fn notifications(s: &Workspace) -> El {
    let n = &s.settings.notifications;
    let off = !n.system_notifications;
    let mut body = col()
        .gap(4.)
        .child(setting(
            "Activity in tabs",
            "A word on a tab whose programs need you or failed, and Done on one that finished while you were away.",
            notify_switch("notify-tabs", "Show activity in tabs", n.show_in_tabs, false, |n| &mut n.show_in_tabs),
        ))
        .child(divider())
        .child(setting(
            "Desktop notifications",
            "For tabs you aren't looking at, through your desktop's notification service. Clicking one shows its tab.",
            notify_switch("notify-system", "Desktop notifications", n.system_notifications, false, |n| &mut n.system_notifications),
        ))
        .child(group("Notify when a program"));
    let states: [(&str, &str, &str, bool, Field); 4] = [
        (
            "notify-blocked",
            "Needs you",
            "Waiting for approval, an answer or a sign-in.",
            n.blocked,
            |n| &mut n.blocked,
        ),
        ("notify-error", "Fails", "Reports an error.", n.error, |n| {
            &mut n.error
        }),
        (
            "notify-done",
            "Finishes",
            "Stops working, or reports that it's done.",
            n.done,
            |n| &mut n.done,
        ),
        ("notify-working", "Starts working", "", n.working, |n| {
            &mut n.working
        }),
    ];
    for (i, (key, label, detail, on, field)) in states.into_iter().enumerate() {
        if i > 0 {
            body = body.child(divider());
        }
        body = body.child(setting(
            label,
            detail,
            notify_switch(key, label, on, off, field),
        ));
    }
    body.child(divider()).child(setting(
        "Messages from programs",
        "Notifications programs send themselves with OSC 9 or OSC 777.",
        notify_switch(
            "notify-messages",
            "Messages from programs",
            n.program_messages,
            off,
            |n| &mut n.program_messages,
        ),
    ))
}

fn terminal(s: &Workspace) -> El {
    let saved = &s.settings.font_family;
    let active = s.terminal_family.unwrap_or("IBM Plex Mono (bundled)");
    let font = if *saved != s.launch_saved_font {
        format!("{saved} is saved and applies when you reopen the client. Active: {active}.")
    } else if s.terminal_family.is_none() {
        format!(
            "{} isn't installed, so terminals use {active}.",
            s.launch_font
        )
    } else {
        format!("Active: {active}. Changes apply when you reopen the client.")
    };
    let mut body = col()
        .gap(4.)
        .child(setting(
            "Font family",
            &font,
            pill("font-change", "Change…", Pill::Neutral, 26., |s, tx| s.perform(Intent::Fonts, tx)),
        ))
        .child(divider())
        .child(setting(
            "Font size",
            "8–48 pt, for every terminal.",
            stepper(
                "font",
                format!("{} pt", s.settings.font_size),
                |s, _| s.change_font(-1.),
                |s, _| s.change_font(1.),
            ),
        ))
        .child(divider())
        .child(setting(
            "Pane padding",
            "0–32 pt around each terminal's content. Applies immediately to every pane.",
            stepper(
                "pane-padding",
                format!("{} pt", s.settings.pane_padding),
                |s, _| {
                    s.settings.pane_padding = (s.settings.pane_padding - 1.).max(0.);
                    s.save();
                },
                |s, _| {
                    s.settings.pane_padding = (s.settings.pane_padding + 1.).min(32.);
                    s.save();
                },
            ),
        ))
        .child(divider())
        .child(setting(
            "Fallback fonts",
            "Used for missing glyphs, followed by installed system fallbacks. Changes apply when you reopen the client.",
            text(if s.settings.font_fallbacks.is_empty() { "Automatic".into() } else { s.settings.font_fallbacks.join(", ") })
                .font(medium(13.)).color(rgb(ink())).wrap().max_w(220.),
        ))
        .child(divider())
        .child(setting(
            "Theme",
            "Declared to Rex for panes this client owns or nobody owns. Follow server declares nothing until this client has declared a theme once; from then on it resets those panes to Rex's colours, once each per connection.",
            row()
                .gap(10.)
                .center_y()
                .child(
                    text(Workspace::theme_label(&s.settings.terminal_theme).to_owned())
                        .font(medium(13.))
                        .color(rgb(ink()))
                        .truncate()
                        .max_w(160.),
                )
                .child(pill("terminal-theme", "Change…", Pill::Neutral, 26., |s, tx| {
                    s.perform(Intent::ChooseTheme, tx)
                })),
        ))
        .child(group("Default shell"));
    for (i, host) in s.settings.hosts.iter().enumerate() {
        let detail = match &host.shell {
            Some(shell) => format!("New sessions, tabs and splits start {shell}"),
            None => "New sessions, tabs and splits use the server's default shell".into(),
        };
        body = body.child(setting(
            &host.label,
            &detail,
            pill(
                &format!("host-shell:{i}"),
                "Change…",
                Pill::Neutral,
                26.,
                move |s, tx| s.perform(Intent::HostShell(i), tx),
            ),
        ));
    }
    body.child(
        text(format!("Saved in {}", s.settings_path.display()))
            .font(ui(11.))
            .color(rgb(muted()))
            .wrap()
            .selectable(true)
            .mt(12.),
    )
}

fn hosts(s: &Workspace) -> El {
    let last = s.settings.hosts.len() == 1;
    let mut body = col()
        .gap(4.)
        .child(setting(
            "Client label",
            &format!(
                "{}\nVisible on every host; reopen the app after editing.",
                s.settings.client_label()
            ),
            pill("client-label", "Edit…", Pill::Neutral, 26., |s, tx| {
                s.perform(Intent::ClientLabel, tx)
            }),
        ))
        .child(divider());
    for (i, host) in s.settings.hosts.iter().enumerate() {
        let connected = s.hosts.get(i).is_some_and(|h| h.model.connected);
        let mut actions = row().gap(6.).center_y();
        // Quiet when healthy: only trouble gets words, beside the action that fixes it.
        if !connected {
            actions = actions
                .child(text("Offline").font(medium(12.)).color(rgb(warning())))
                .child(pill(
                    &format!("host-retry:{i}"),
                    "Retry",
                    Pill::Warning,
                    26.,
                    move |s, tx| s.perform(Intent::RetryHost(i), tx),
                ));
        }
        actions = actions.child(pill(
            &format!("host-edit:{i}"),
            "Edit…",
            Pill::Neutral,
            26.,
            move |s, tx| s.perform(Intent::EditHost(i), tx),
        ));
        let remove = if last {
            pill(
                &format!("host-remove:{i}"),
                "Remove…",
                Pill::Neutral,
                26.,
                |_, _| {},
            )
            .opacity(0.4)
            .focusable(false)
            .help("The workspace needs at least one host")
        } else {
            pill(
                &format!("host-remove:{i}"),
                "Remove…",
                Pill::Danger,
                26.,
                move |s, tx| s.perform(Intent::RemoveHost(i), tx),
            )
        };
        let shell = host
            .shell
            .as_deref()
            .map(|sh| format!(" · {sh}"))
            .unwrap_or_default();
        body = body
            .child(
                row()
                    .key(format!("host-row:{i}"))
                    .w_full()
                    .min_h(52.)
                    .gap(12.)
                    .center_y()
                    .child(glyph(s.host_icon(i), 16., rgb(muted())))
                    .child(
                        col()
                            .grow(1.)
                            .min_w(0.)
                            .gap(2.)
                            .child(
                                text(host.label.clone())
                                    .font(medium(13.))
                                    .color(rgb(ink()))
                                    .truncate(),
                            )
                            .child(
                                text(format!("{}{shell}", host.endpoint))
                                    .font(ui(11.5))
                                    .color(rgb(muted()))
                                    .truncate()
                                    .selectable(true),
                            ),
                    )
                    .child(actions.child(remove)),
            )
            .child(setting(
                "Remote control",
                "Let rex do and other clients on this host change the view in this window.",
                rune_kit::Switch::new(host.remote_control)
                    .label(format!("Remote control for {}", host.label))
                    .hide_label()
                    .on_toggle(move |s: &mut Workspace| s.toggle_remote_control(i))
                    .build(kit())
                    .key(format!("remote-control:{i}")),
            ))
            .child(divider());
    }
    body.child(row().mt(8.).child(pill(
        "settings-add-host",
        "Add host…",
        Pill::Neutral,
        30.,
        |s, tx| s.perform(Intent::AddHost, tx),
    )))
    .child(
        text("Removing a host only stops this app connecting to it; its sessions keep running.")
            .font(ui(11.5))
            .color(rgb(muted()))
            .wrap()
            .mt(8.),
    )
}

/// Presentation only: preserve dispatch priority within each action/argument group.
struct ShortcutRow<'a> {
    binding: &'a rex_linux::keymap::ResolvedBinding,
    also_numpad: bool,
    arrows: bool,
}

fn shortcut_rows<'a>(
    effective: &[&'a rex_linux::keymap::ResolvedBinding],
    category: &str,
) -> Vec<ShortcutRow<'a>> {
    use super::actions::{SPECS, find};
    use rex_linux::keymap::Key;

    let mut list: Vec<_> = effective
        .iter()
        .copied()
        .filter(|b| {
            find(&b.binding.action)
                .map(|a| a.category())
                .unwrap_or("Server")
                == category
        })
        .collect();
    list.sort_by_cached_key(|b| {
        (
            SPECS
                .iter()
                .position(|s| s.name == b.binding.action)
                .unwrap_or(SPECS.len()),
            b.binding.action.clone(),
            b.binding
                .args
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        )
    });
    let default_focus = |b: &&rex_linux::keymap::ResolvedBinding, direction: &str| {
        b.binding.action == "pane.focus"
            && b.binding.source.as_deref() == Some("linux")
            && b.binding.keys == [format!("ctrl+alt+{direction}")]
            && b.binding.args == Some(serde_json::json!({"direction": direction}))
    };
    let arrows = ["left", "right", "up", "down"]
        .iter()
        .all(|d| list.iter().any(|b| default_focus(b, d)));
    let mut rows: Vec<_> = list
        .iter()
        .map(|b| ShortcutRow {
            binding: b,
            also_numpad: false,
            arrows: false,
        })
        .collect();
    let mut hidden = std::collections::HashSet::new();
    for (i, b) in list.iter().enumerate() {
        if arrows
            && ["left", "right", "up", "down"]
                .iter()
                .any(|d| default_focus(b, d))
        {
            if default_focus(b, "down") {
                rows[i].arrows = true;
            } else {
                hidden.insert(i);
            }
        }
        let Some([chord]) = b.chords.as_deref() else {
            continue;
        };
        let Key::Physical(code) = &chord.key else {
            continue;
        };
        let equivalent = match *code {
            "NumpadAdd" => '+',
            "NumpadSubtract" => '-',
            "Numpad0" => '0',
            _ => continue,
        };
        if let Some(j) = list.iter().position(|other| {
            other.binding.action == b.binding.action
                && other.binding.args == b.binding.args
                && other.binding.source == b.binding.source
                && other.chords.as_deref().is_some_and(|chords| matches!(chords,
                    [other] if other.modifiers == chord.modifiers && other.key == Key::Character(equivalent)))
        }) {
            rows[j].also_numpad = true;
            hidden.insert(i);
        }
    }
    rows.into_iter()
        .enumerate()
        .filter_map(|(i, row)| (!hidden.contains(&i)).then_some(row))
        .collect()
}

fn shortcuts(s: &Workspace) -> El {
    let k = kit();
    let mut body = col().gap(4.);
    let effective = s.effective_bindings();
    for name in [
        "Navigate",
        "Create and close",
        "Panes",
        "Text and view",
        "Interface",
        "Server",
    ] {
        let list = shortcut_rows(&effective, name);
        if list.is_empty() {
            continue;
        }
        body = body.child(group(name));
        for (index, display) in list.iter().enumerate() {
            let binding = display.binding;
            let mut caps = col().gap(3.);
            for (depth, chord) in binding.chords.as_ref().unwrap().iter().enumerate() {
                if depth > 0 {
                    caps = caps.child(text("then").font(ui(10.5)).color(rgb(muted())));
                }
                let names = if display.arrows {
                    vec!["Ctrl".into(), "Alt".into(), "Arrows".into()]
                } else {
                    chord.keycaps()
                };
                let keys: Vec<_> = names.iter().map(String::as_str).collect();
                caps = caps.child(Keybind::new(&keys).build(k));
            }
            if display.also_numpad {
                caps = caps.child(text("also numpad").font(ui(10.5)).color(rgb(muted())));
            }
            let source = match binding.binding.source.as_deref() {
                Some("linux") => None,
                Some("settings") => Some("shortcuts.json"),
                Some(source) => Some(source),
                None => Some("Server"),
            };
            let mut words = col().grow(1.).min_w(0.).gap(2.).child(
                text(if display.arrows {
                    "Focus adjacent pane".into()
                } else {
                    super::actions::binding_title(&binding.binding)
                })
                .font(ui(12.5))
                .color(rgb(ink()))
                .wrap(),
            );
            if let Some(source) = source {
                words = words.child(text(source).font(ui(10.5)).color(rgb(muted())).wrap());
            }
            body = body.child(
                row()
                    .key(format!("shortcut-{name}-{index}"))
                    .min_h(38.)
                    .py(3.)
                    .gap(12.)
                    .center_y()
                    .child(el().w(190.).child(caps))
                    .child(words),
            );
        }
    }
    body = body.child(group("Pointer")).child(
        row()
            .min_h(30.)
            .gap(12.)
            .center_y()
            .child(
                el().w(190.)
                    .child(Keybind::new(&["Mouse", "Back / Forward"]).build(k)),
            )
            .child(text("Navigation history").font(ui(12.5)).color(rgb(ink()))),
    );
    let diagnostics = &s.effective_keymap().diagnostics;
    if !diagnostics.is_empty() || s.model().keymap_error.is_some() {
        body = body.child(group("Problems"));
        if let Some(error) = &s.model().keymap_error {
            body = body.child(text(error).font(ui(11.5)).color(rgb(warning())).wrap());
        }
        for (index, problem) in diagnostics.iter().enumerate() {
            body = body.child(
                text(format!(
                    "{}: {}",
                    problem.source.as_deref().unwrap_or("keymap"),
                    problem.message
                ))
                .key(format!("shortcut-problem-{index}"))
                .font(ui(11.5))
                .color(rgb(warning()))
                .wrap(),
            );
        }
    }
    body = body.child(group("Files"));
    if let Some(snapshot) = s.keymap_snapshot() {
        body = body.child(
            text(format!(
                "Server: {}{}",
                snapshot.reply.config.path,
                if snapshot.reply.config.exists {
                    ""
                } else {
                    " (not created)"
                }
            ))
            .key("server-shortcuts-path")
            .font(ui(11.5))
            .color(rgb(muted()))
            .selectable(true)
            .wrap(),
        );
    }
    body = body.child(
        text(
            rex_linux::settings::shortcuts::Shortcuts::path(&s.settings_path)
                .display()
                .to_string(),
        )
        .key("shortcuts-path")
        .font(ui(11.5))
        .color(rgb(muted()))
        .selectable(true)
        .wrap()
        .mt(12.),
    );
    body.child(
        text("Edit the overrides file, then return to Rex to reload. Escape cancels a sequence; unbound keys reach the terminal.")
            .font(ui(11.5))
            .color(rgb(muted()))
            .wrap()
            .mt(12.),
    )
}

fn nav(s: &Workspace) -> El {
    let mut list = col()
        .key("settings-sections")
        .role("tablist")
        .focus_group()
        .gap(2.);
    for section in SettingsSection::ALL {
        let selected = s.settings_section == section;
        let mut item = row()
            .key(format!("settings-section:{}", section.key()))
            .role("tab")
            .label(section.title())
            .w_full()
            .h(32.)
            .px(10.)
            .center_y()
            .radius(8.)
            .continuous_corners()
            .cursor(Cursor::Pointer)
            .focusable(true)
            .focus_selected(selected)
            .focus_ring(2., 0., rgb(accent()))
            .on_press(move |s: &mut Workspace| s.open_section(section))
            .child(
                text(section.title())
                    .font(if selected { medium(13.) } else { ui(13.) })
                    .color(rgb(if selected { ink() } else { muted() })),
            );
        item = if selected {
            item.fill(white(0.09))
        } else {
            item.hover(white(0.05))
        };
        list = list.child(item);
    }
    col()
        .w(190.)
        .h_full()
        .padding(14.)
        .gap(12.)
        .child(
            text("Settings")
                .font(strong(15.))
                .color(rgb(ink()))
                .px(10.)
                .pt(6.),
        )
        .child(list)
}

pub fn settings(s: &Workspace, viewport: [f32; 2]) -> El {
    let k = kit();
    let section = s.settings_section;
    let body = match section {
        SettingsSection::Interface => interface(s),
        SettingsSection::Terminal => terminal(s),
        SettingsSection::Notifications => notifications(s),
        SettingsSection::Integrations => super::integrations::section(s),
        SettingsSection::Hosts => hosts(s),
        SettingsSection::Shortcuts => shortcuts(s),
        SettingsSection::Updates => super::updates::section(s),
        SettingsSection::About => super::about::section(s),
    };
    let content = col()
        .grow(1.)
        .min_w(0.)
        .h_full()
        .child(
            row()
                .h(56.)
                .px(24.)
                .center_y()
                .justify_between()
                .child(text(section.title()).font(strong(17.)).color(rgb(ink())))
                .child(
                    IconButton::new(&CLOSE)
                        .label("Close")
                        .on_press_tx(|s: &mut Workspace, tx: &mut Tx| s.close_panel(tx))
                        .build(k)
                        .key("close-settings"),
                ),
        )
        .child(
            col()
                .key(format!("settings-scroll:{}", section.key()))
                .grow(1.)
                .min_h(0.)
                .w_full()
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(body.px(24.).pb(20.)),
        );
    modal_frame(
        s,
        row()
            .w(760f32.min(viewport[0] * 0.92))
            .h(520f32.min(viewport[1] * 0.88))
            .child(nav(s))
            .child(el().w(1.).h_full().fill(white(0.06)))
            .child(content),
    )
}

#[cfg(test)]
mod shortcut_tests {
    use super::*;
    use crate::app::actions;
    use rex_linux::keymap::Overrides;
    use serde_json::json;

    #[test]
    fn defaults_are_compact_and_in_registry_order() {
        let map = actions::resolve(&Overrides::default());
        let effective = actions::effective(&map, &[]);
        let panes = shortcut_rows(&effective, "Panes");
        let focus: Vec<_> = panes
            .iter()
            .filter(|r| r.binding.binding.action == "pane.focus")
            .collect();
        assert_eq!(focus.len(), 1);
        assert!(focus[0].arrows);
        let interface = shortcut_rows(&effective, "Interface");
        assert_eq!(interface.iter().filter(|r| r.also_numpad).count(), 3);
        assert!(
            !interface
                .iter()
                .any(|r| r.binding.binding.keys[0].contains("Numpad"))
        );
        for category in [
            "Navigate",
            "Create and close",
            "Panes",
            "Text and view",
            "Interface",
        ] {
            let rows = shortcut_rows(&effective, category);
            let positions: Vec<_> = rows
                .iter()
                .map(|r| {
                    actions::SPECS
                        .iter()
                        .position(|s| s.name == r.binding.binding.action)
                        .unwrap()
                })
                .collect();
            assert!(positions.windows(2).all(|w| w[0] <= w[1]));
        }
        let scroll: Vec<_> = effective
            .iter()
            .filter(|b| b.binding.action == "pane.scroll")
            .map(|b| actions::binding_title(&b.binding))
            .collect();
        assert!(scroll.contains(&"Scroll up · 20 lines".into()));
        assert!(scroll.contains(&"Scroll down · 20 lines".into()));
    }

    #[test]
    fn custom_focus_is_not_folded_and_try_order_is_retained() {
        let map = actions::resolve(&Overrides::parse(&json!([
            {"keys":"ctrl+alt+left","action":"pane.focus","args":{"direction":"right"}},
            {"keys":"ctrl+j","action":"client.tab.new"},
            {"keys":"ctrl+y","action":"client.tab.new"},
            {"keys":"ctrl+[Numpad0]","action":"client.zoom.in"}
        ])));
        let effective = actions::effective(&map, &[]);
        assert!(shortcut_rows(&effective, "Panes").iter().all(|r| !r.arrows));
        let expected: Vec<_> = effective
            .iter()
            .filter(|b| b.binding.action == "client.tab.new")
            .map(|b| &b.binding.keys)
            .collect();
        let rows = shortcut_rows(&effective, "Create and close");
        let actual: Vec<_> = rows
            .iter()
            .filter(|r| r.binding.binding.action == "client.tab.new")
            .map(|r| &r.binding.binding.keys)
            .collect();
        assert_eq!(actual, expected);
        assert!(
            shortcut_rows(&effective, "Interface")
                .iter()
                .any(|r| r.binding.binding.keys == ["ctrl+[Numpad0]"] && !r.also_numpad)
        );
    }
}
