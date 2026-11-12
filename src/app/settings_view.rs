//! Settings: a section list on the left and the chosen section's page on the right. Pages are
//! built from `settings_kit` (sections, cards, rows and their controls).
use super::settings_nav::{HEADER, PAGE_INSET};
use super::{
    dialogs::modal_layer,
    panel::{Intent, SettingsSection},
    settings_kit::{
        NAV_WIDTH, PANEL_HEIGHT, PANEL_WIDTH, Segment, button, card, disclosure, footnote, page,
        row_frame, section, segmented, select_row, setting, stepper, unavailable, value_popup,
    },
    theme::*,
    widgets::{Pill, glyph},
    workspace::Workspace,
};
use rex_rune::settings::{
    NoticePosition, NotificationsConfig, PaneDensity, PaneHeader,
    interface::{InterfaceConfig, ToolbarBehavior},
};
use rune::{El, KeyEvent, PointerButton, ScrollPos, ScrollSpec, Tx, col, el, row, text};
use rune_kit::Switch;

/// The least space between the dialog and the toolbar above it, and the window's bottom edge.
const CLEARANCE: f32 = 12.;

/// The Pane headers choices, in the order the control lists them.
const PANE_HEADERS: [PaneHeader; 4] = [
    PaneHeader::Auto,
    PaneHeader::Always,
    PaneHeader::OnHover,
    PaneHeader::Hidden,
];

const TOOLBARS: [ToolbarBehavior; 3] = [
    ToolbarBehavior::Always,
    ToolbarBehavior::OnHover,
    ToolbarBehavior::Hidden,
];

fn header_fade_switch(
    key: &str,
    label: &str,
    on: bool,
    field: fn(&mut InterfaceConfig) -> &mut bool,
) -> El {
    Switch::new(on)
        .label(label.to_owned())
        .hide_label()
        .on_toggle(move |s: &mut Workspace| {
            let value = field(&mut s.settings.interface);
            *value = !*value;
            s.save();
        })
        .build(kit())
        .key(key.to_owned())
}

fn interface(s: &Workspace) -> El {
    let k = kit();
    let percent = (s.settings.ui_scale * 100.).round() as i32;
    let corner = match s.settings.notice_position {
        NoticePosition::TopRight => 0,
        NoticePosition::BottomRight => 1,
    };
    page([
        section(
            "Appearance",
            [card([
                setting(
                    "Theme",
                    "Terminals take the matching theme until you choose one.",
                    value_popup(
                        "chrome-theme",
                        s.settings.chrome_theme.name(),
                        Intent::ChooseChrome,
                    ),
                ),
                setting(
                    "Interface zoom",
                    "Chrome and terminals together, from 70% to 150%.",
                    row()
                        .gap(8.)
                        .center_y()
                        .child(stepper(
                            "zoom",
                            format!("{percent}%"),
                            |s, tx| s.change_ui_scale(-1, tx),
                            |s, tx| s.change_ui_scale(1, tx),
                        ))
                        .child(button("zoom-reset", "Reset", Pill::Neutral, |s, tx| {
                            s.change_ui_scale(0, tx)
                        })),
                ),
            ])],
        ),
        section(
            "Toolbar",
            [
                card([select_row(
                    "toolbar-behavior",
                    "Toolbar visibility",
                    "Show on hover and Hidden give its row to the terminals.",
                    &["Always show", "Show on hover", "Hidden"],
                    TOOLBARS
                        .iter()
                        .position(|t| *t == s.settings.interface.toolbar)
                        .unwrap_or_default(),
                    |s, i, tx| s.set_toolbar(TOOLBARS[i.min(TOOLBARS.len() - 1)], tx),
                )]),
                {
                    let keys =
                        s.shortcut_detail(super::toolbar_reveal::TOGGLE, &serde_json::Value::Null);
                    let reveal = if keys.is_empty() {
                        "Reveal toolbar, in Commands,".to_owned()
                    } else {
                        format!("Reveal toolbar ({keys}, and in Commands)")
                    };
                    footnote(format!("{reveal} floats the toolbar until run again. Window-manager Move and Close still work."))
                        .key("toolbar-reveal-note")
                },
            ],
        ),
        section(
            "Panes",
            [
                card([
                    setting(
                        "Pane density",
                        "Compact joins panes edge to edge, with no gaps.",
                        segmented(
                            "pane-density",
                            &[
                                Segment::new("Compact").icon(&PANES_JOINED),
                                Segment::new("Comfortable").icon(&PANES_APART),
                            ],
                            Some(usize::from(
                                s.settings.pane_density == PaneDensity::Comfortable,
                            )),
                            |s, i, tx| {
                                s.settings.pane_density = if i == 0 {
                                    PaneDensity::Compact
                                } else {
                                    PaneDensity::Comfortable
                                };
                                s.save();
                                tx.instant();
                            },
                        ),
                    ),
                    select_row(
                        "pane-header",
                        "Pane headers",
                        "Auto shows them on split panes in Comfortable density.",
                        &["Auto", "Always", "On hover", "Hidden"],
                        PANE_HEADERS
                            .iter()
                            .position(|h| *h == s.settings.pane_header)
                            .unwrap_or_default(),
                        |s, i, tx| {
                            s.settings.pane_header = PANE_HEADERS[i.min(PANE_HEADERS.len() - 1)];
                            // A header that was brought out by hand goes back with the old choice.
                            s.header_reveal = Default::default();
                            s.save();
                            tx.instant();
                        },
                    ),
                    select_row(
                        "active-pane-outline",
                        "Active pane outline",
                        "Auto follows density. A single edge-to-edge pane has no outline.",
                        &["Auto", "On", "Off"],
                        match s.settings.interface.highlight_active_pane {
                            None => 0,
                            Some(true) => 1,
                            Some(false) => 2,
                        },
                        |s, i, tx| {
                            s.settings.interface.highlight_active_pane = match i {
                                1 => Some(true),
                                2 => Some(false),
                                _ => None,
                            };
                            s.save();
                            tx.instant();
                        },
                    ),
                    setting(
                        "Fade inactive pane headers",
                        "For panes without the keyboard.",
                        header_fade_switch(
                            "fade-inactive-headers",
                            "Fade inactive pane headers",
                            s.settings.interface.fade_inactive_pane_headers,
                            |i| &mut i.fade_inactive_pane_headers,
                        ),
                    ),
                    setting(
                        "Fade headers in inactive windows",
                        "While this window is not focused.",
                        header_fade_switch(
                            "fade-headers-inactive-window",
                            "Fade headers in inactive windows",
                            s.settings.interface.fade_pane_headers_in_inactive_window,
                            |i| &mut i.fade_pane_headers_in_inactive_window,
                        ),
                    ),
                ]),
                footnote(
                    "Only visible header names and available buttons fade. Activity, progress, exceptions and terminal content stay unchanged.",
                ),
            ],
        ),
        section(
            "Sidebar and tabs",
            [card([
                setting(
                    "Session sidebar",
                    "Hosts and their sessions beside the terminals.",
                    Switch::new(s.sidebar.is_open())
                        .label("Show session sidebar")
                        .hide_label()
                        .on_toggle_tx(|s: &mut Workspace, tx: &mut Tx| {
                            s.perform(Intent::Sidebar, tx)
                        })
                        .build(k)
                        .key("sidebar-switch"),
                ),
                setting(
                    "Tab position",
                    "Sidebar tabs show while the sidebar is open.",
                    segmented(
                        "tab-position",
                        &[
                            Segment::new("Top").icon(&TABS_TOP),
                            Segment::new("Sidebar").icon(&SIDEBAR),
                        ],
                        Some(usize::from(s.settings.vertical_tabs)),
                        |s, i, _| s.set_vertical_tabs(i == 1),
                    ),
                ),
                setting(
                    "Show tab icons",
                    "0–5 distinct programs; focused pane first.",
                    stepper(
                        "tab-icons",
                        s.settings.interface.tab_icon_count.to_string(),
                        |s, _| s.step_tab_icons(-1),
                        |s, _| s.step_tab_icons(1),
                    ),
                ),
                select_row(
                    "tab-icon-color",
                    "Use colour for tab icons",
                    if s.settings.interface.tab_icon_count == 0 {
                        "No icons are shown while Show tab icons is 0."
                    } else {
                        "Active pane colours only its first icon in the tab on screen."
                    },
                    &["Always", "Active pane", "Active tab", "Never"],
                    match s.settings.interface.tab_icon_color {
                        rex_rune::settings::interface::TabIconColor::Always => 0,
                        rex_rune::settings::interface::TabIconColor::ActivePane => 1,
                        rex_rune::settings::interface::TabIconColor::ActiveTab => 2,
                        rex_rune::settings::interface::TabIconColor::Never => 3,
                    },
                    |s, i, _| {
                        use rex_rune::settings::interface::TabIconColor;
                        s.settings.interface.tab_icon_color = [
                            TabIconColor::Always,
                            TabIconColor::ActivePane,
                            TabIconColor::ActiveTab,
                            TabIconColor::Never,
                        ][i];
                        s.save();
                    },
                ),
                super::tab_icons::preview(s),
                setting(
                    "Tabs fill available space",
                    "Only toolbar tabs share the available width.",
                    Switch::new(s.settings.interface.tabs_fill_available_space)
                        .label("Tabs fill available space")
                        .hide_label()
                        .on_toggle(|s: &mut Workspace| {
                            s.settings.interface.tabs_fill_available_space =
                                !s.settings.interface.tabs_fill_available_space;
                            s.reveal_tab_again = true;
                            s.save();
                        })
                        .build(k)
                        .key("tabs-fill"),
                ),
            ])],
        ),
        section(
            "Downloads",
            [card([disclosure(
                "download-folder",
                "Download folder",
                "Where Download file… saves.",
                &super::downloads::folder_text(
                    &s.download_folder(),
                    super::downloads::FOLDER_CHARS,
                ),
                |s, tx| s.perform(Intent::DownloadFolder, tx),
            )])],
        ),
        section(
            "Notices",
            [card([select_row(
                "notice-position",
                "Position",
                "Where in-app notices appear.",
                &["Top right", "Bottom right"],
                corner,
                |s, i, _| {
                    s.set_notice_position(if i == 0 {
                        NoticePosition::TopRight
                    } else {
                        NoticePosition::BottomRight
                    })
                },
            )])],
        ),
    ])
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
    use super::notify::{Command, Service, TestResult};
    use super::settings_kit::{ChipTone, state_chip};
    let health = &s.notification_health;
    let (status, tone) = match &health.service {
        Service::Unchecked => ("Not checked yet", ChipTone::Off),
        Service::Available => ("Available", ChipTone::Good),
        Service::NotFound => ("Not found", ChipTone::Attention),
        Service::Unreachable(_) => ("Unreachable", ChipTone::Attention),
        Service::Rejected => ("Rejected", ChipTone::Attention),
    };
    let test = match &health.test {
        TestResult::None => "No test sent yet.".to_owned(),
        TestResult::Sending => "Sending test…".to_owned(),
        TestResult::Sent => "Last test: sent to the service.".to_owned(),
        TestResult::Failed(reason) => format!("Last test failed: {reason}."),
    };
    let n = &s.settings.notifications;
    let off = !n.system_notifications;
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
    page([
        section(
            "",
            [card([
                setting(
                    "Activity in tabs",
                    "A word on a tab that needs you, failed, or finished while you were away.",
                    notify_switch(
                        "notify-tabs",
                        "Show activity in tabs",
                        n.show_in_tabs,
                        false,
                        |n| &mut n.show_in_tabs,
                    ),
                ),
                setting(
                    "Desktop notifications",
                    "For tabs you aren't looking at. Clicking one shows its tab.",
                    notify_switch(
                        "notify-system",
                        "Desktop notifications",
                        n.system_notifications,
                        false,
                        |n| &mut n.system_notifications,
                    ),
                ),
            ])],
        ),
        section(
            "Notify when a program",
            [card(states.into_iter().map(
                |(key, label, detail, on, field)| {
                    setting(label, detail, notify_switch(key, label, on, off, field))
                },
            ))],
        ),
        section(
            "Programs",
            [card([setting(
                "Messages from programs",
                "Notifications programs send themselves with OSC 9 or OSC 777.",
                notify_switch(
                    "notify-messages",
                    "Messages from programs",
                    n.program_messages,
                    off,
                    |n| &mut n.program_messages,
                ),
            )])],
        ),
        section(
            "Desktop service",
            [
                card([
                    setting(
                        "Notification service",
                        "Last check, not a delivery receipt.",
                        state_chip("notification-health", status, tone, health.service.reason()),
                    ),
                    setting(
                        "Send test notification",
                        &test,
                        row()
                            .gap(6.)
                            .center_y()
                            .children(matches!(health.test, TestResult::Failed(_)).then(|| {
                                state_chip(
                                    "notification-test-failed",
                                    "Failed",
                                    ChipTone::Attention,
                                    &test,
                                )
                            }))
                            .child(button(
                                "notification-test",
                                "Send test",
                                Pill::Neutral,
                                |s, _| {
                                    s.notification_command(Command::Test);
                                },
                            )),
                    ),
                ]),
                footnote("Tests work even when desktop notifications are off."),
            ],
        ),
    ])
}

fn terminal(s: &Workspace) -> El {
    super::terminal_page::terminal(s)
}

fn host_card(s: &Workspace, i: usize, last: bool) -> El {
    let host = &s.settings.hosts[i];
    let connected = s.hosts.get(i).is_some_and(|h| h.model.connected);
    let mut actions = row().gap(6.).center_y();
    // Quiet when healthy: only trouble gets words, beside the action that fixes it.
    if !connected {
        actions = actions
            .child(text("Offline").font(medium(12.)).color(rgb(warning())))
            .child(button(
                &format!("host-retry:{i}"),
                "Retry",
                Pill::Warning,
                move |s, tx| s.perform(Intent::RetryHost(i), tx),
            ));
    }
    actions = actions.child(button(
        &format!("host-edit:{i}"),
        "Edit…",
        Pill::Neutral,
        move |s, tx| s.perform(Intent::EditHost(i), tx),
    ));
    actions = actions.child(if last {
        unavailable(
            &format!("host-remove:{i}"),
            "Remove…",
            "The workspace needs at least one host",
        )
    } else {
        button(
            &format!("host-remove:{i}"),
            "Remove…",
            Pill::Danger,
            move |s, tx| s.perform(Intent::RemoveHost(i), tx),
        )
    });
    let mut address = row().gap(6.).center_y().min_w(0.).child(
        text(host.endpoint.clone())
            .font(mono(11.))
            .color(rgb(muted()))
            .truncate()
            .selectable(true),
    );
    if let Some(shell) = &host.shell {
        address = address.child(
            text(format!("· {shell}"))
                .font(mono(11.))
                .color(rgb(muted())),
        );
    }
    card([
        row_frame(true)
            .key(format!("host-row:{i}"))
            .gap(12.)
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
                    .child(address),
            )
            .child(actions),
        setting(
            "Remote control",
            "Let rex do and other clients on this host change the view in this window.",
            Switch::new(host.remote_control)
                .label(format!("Remote control for {}", host.label))
                .hide_label()
                .on_toggle(move |s: &mut Workspace| s.toggle_remote_control(i))
                .build(kit())
                .key(format!("remote-control:{i}")),
        ),
    ])
}

fn hosts(s: &Workspace) -> El {
    let last = s.settings.hosts.len() == 1;
    let cards = (0..s.settings.hosts.len()).map(|i| host_card(s, i, last));
    page([
        section(
            "This client",
            [card([disclosure(
                "client-label",
                "Client label",
                "Its name on every host. Takes effect when you reopen the app.",
                &s.settings.client_label(),
                |s, tx| s.perform(Intent::ClientLabel, tx),
            )])],
        ),
        section(
            "Hosts",
            cards.chain([
                row().px(2.).child(button(
                    "settings-add-host",
                    "Add host…",
                    Pill::Neutral,
                    |s, tx| s.perform(Intent::AddHost, tx),
                )),
                footnote(
                    "Removing a host only stops this app connecting to it; its sessions keep running.",
                ),
            ]),
        ),
    ])
}

/// The headings of the Shortcuts page, in order. Every action's `category()` is one of them
/// (a test checks), and an action this client does not know is listed under Server.
pub(super) const SHORTCUT_CATEGORIES: [&str; 6] = [
    "Navigate",
    "Create and close",
    "Panes",
    "Text and view",
    "Interface",
    "Server",
];

/// Build only the page content. Search reads these same labelled Settings-kit rows.
pub(super) fn body(s: &Workspace, page: super::settings_nav::Page) -> El {
    let section = match page {
        super::settings_nav::Page::Section(section) => section,
        super::settings_nav::Page::Sub(super::settings_nav::SubPage::AdvancedFont) => {
            return super::font_page::advanced(s);
        }
    };
    match section {
        SettingsSection::Server => super::server_page::section(s),
        SettingsSection::Interface => interface(s),
        SettingsSection::Terminal => terminal(s),
        SettingsSection::Notifications => notifications(s),
        SettingsSection::Integrations => super::integrations::section(s),
        SettingsSection::Hosts => hosts(s),
        SettingsSection::Shortcuts => super::shortcuts_page::page(s),
        SettingsSection::Updates => super::updates::section(s),
        SettingsSection::About => super::about::section(s),
    }
}

/// The dialog's surface: the section list, and the page under its header. The page scrolls
/// with a fade at whichever edge hides content and a thumb while there is more than fits.
fn surface(s: &Workspace, viewport: [f32; 2]) -> El {
    let section = s.settings_page();
    let body = super::settings_nav::draw(s, section);
    let width = PANEL_WIDTH.min(viewport[0] * 0.92);
    // Under the toolbar, which stays whole above it, with a margin at both ends.
    let height = PANEL_HEIGHT
        .min((viewport[1] - super::toolbar_reveal::row_height(s) - 2. * CLEARANCE).max(0.));
    let nav_width = NAV_WIDTH.min(width * 0.3);
    let pinned = (section == SettingsSection::Shortcuts)
        .then(|| super::shortcuts_page::filter_field(s, PAGE_INSET));
    let above = HEADER
        + pinned
            .as_ref()
            .map_or(0., |_| super::shortcuts_page::FILTER);
    let (page_width, page_height) = (width - nav_width - 1., height - above);
    let thumb = s.settings_fx.thumb.clone();
    let offset = s.settings_nav.offset();
    let scroller = col()
        .key(format!("settings-scroll:{}", section.key()))
        .grow(1.)
        .min_h(0.)
        .w_full()
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .scroll_edge_fade(16.)
        .on_scroll_offset(move |at: ScrollPos| {
            offset.set(at.offset[1]);
            let hidden = at.max[1];
            if hidden < 1. {
                thumb.set([0.; 4]);
                return;
            }
            let length = (page_height * page_height / (page_height + hidden)).max(24.);
            let top = at.offset[1] / hidden * (page_height - length - 8.);
            thumb.set([page_width - 7., top + 4., 4., length]);
        })
        .child(body.px(PAGE_INSET).pt(2.).pb(24.));
    let content = col()
        .grow(1.)
        .min_w(0.)
        .h_full()
        .child(super::settings_shell::header(s, section))
        .children(pinned)
        .child(
            col()
                .grow(1.)
                .min_h(0.)
                .w_full()
                .child(scroller)
                .child(s.settings_fx.thumb_el()),
        );
    let frame = row()
        .key("settings-frame")
        .w_full()
        .h_full()
        .pointer_buttons([PointerButton::Back, PointerButton::Forward])
        .preserve_focus_on_press(true)
        .on_press_tx(|s: &mut Workspace, tx: &mut Tx| {
            let direction = match tx.button() {
                PointerButton::Back => -1,
                PointerButton::Forward => 1,
                _ => return,
            };
            s.settings_history(direction, tx);
        })
        .child(super::settings_shell::sidebar(s, nav_width))
        .child(el().w(1.).h_full().fill(white(0.06)))
        .child(content);
    row()
        .w(width)
        .h(height)
        .on_key(|s: &mut Workspace, ev: &KeyEvent, tx: &mut Tx| s.settings_key(ev, tx))
        .child(frame)
}

pub fn settings(s: &Workspace, viewport: [f32; 2]) -> El {
    modal_layer(
        s,
        surface(s, viewport),
        false,
        super::toolbar_reveal::row_height(s),
    )
}

/// Settings under a list opened from one of its controls: drawn where it was, taking no input.
pub fn settings_under(s: &Workspace, viewport: [f32; 2]) -> El {
    modal_layer(
        s,
        surface(s, viewport),
        true,
        super::toolbar_reveal::row_height(s),
    )
}
