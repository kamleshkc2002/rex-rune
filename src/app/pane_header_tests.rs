//! Pane headers: the Split, Zoom and Close buttons, the Zoomed chip, and where a header is
//! under each Pane headers choice (Auto, Always, On hover, Hidden).
use super::pane_verb_tests::{Camera, framed_terminal, rect, sent_verbs};
use super::*;
use crate::app::{actions, pane_header};
use rex_rune::settings::{PaneDensity, PaneHeader};
use serde_json::Value;

fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
}

/// The tab `active` of `tabbed_workspace`, drawn under `header` and `density`.
fn drawn(
    active: &str,
    header: PaneHeader,
    density: PaneDensity,
) -> (Runtime<Workspace>, flume::Receiver<HostCommand>) {
    let (mut ws, host) = tabbed_workspace(active);
    ws.settings.pane_header = header;
    ws.settings.pane_density = density;
    let mut rt = runtime(ws);
    settle(&mut rt);
    (rt, host)
}

fn element(rt: &Runtime<Workspace>, key: &str) -> rune::ElementInfo {
    rt.elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("missing {key}"))
}

/// Rest the pointer at (x, y) long enough for a delayed hover to be heard.
fn rest_at(rt: &mut Runtime<Workspace>, x: f32, y: f32) {
    rt.pointer_move(x, y);
    for _ in 0..30 {
        rt.frame(1. / 60.);
    }
}

const BUTTONS: [&str; 4] = [
    "pane-split-right",
    "pane-split-down",
    "pane-zoom",
    "pane-close",
];

#[test]
fn a_split_panes_header_ends_in_its_four_buttons() {
    let (rt, _host) = drawn("split", PaneHeader::Auto, PaneDensity::Comfortable);
    for block in ["block:b", "block:c"] {
        let header = rect(&rt, &format!("pane-header:{block}"));
        let mut right = header[0] + header[2];
        for name in BUTTONS.iter().rev() {
            let button = element(&rt, &format!("{name}:{block}"));
            assert_eq!(button.role, "button");
            let [x, y, w, h] = button.rect;
            assert_eq!((w, h), (24., 24.), "{name}: a 24 pt target");
            assert!(
                y >= header[1] && y + h <= header[1] + header[3],
                "{name} is inside the header row"
            );
            assert!(x + w <= right + 0.01, "{name} is left of what follows it");
            right = x;
        }
        let inset = header[0] + header[2] - rect(&rt, &format!("pane-close:{block}"))[0] - 24.;
        assert!(
            (inset - 8.).abs() < 0.01,
            "Close ends the row, 8 pt in so its hover fill clears the card's corner: {inset}"
        );
    }
    let labels: Vec<_> = BUTTONS
        .iter()
        .map(|name| element(&rt, &format!("{name}:block:c")).label.unwrap())
        .collect();
    assert_eq!(
        labels,
        ["Split right", "Split down", "Zoom pane", "Close pane"]
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn header_buttons_act_on_their_own_pane_and_close_asks_by_name() {
    let (mut rt, host) = drawn("split", PaneHeader::Auto, PaneDensity::Comfortable);
    assert_eq!(rt.state.focused_block().as_deref(), Some("block:b"));
    let _ = requests(&host);

    // The unfocused pane's buttons name that pane.
    for (button, direction) in [
        ("pane-split-right:block:c", "horizontal"),
        ("pane-split-down:block:c", "vertical"),
    ] {
        click(&mut rt, button);
        let sent = requests(&host);
        assert_eq!(sent.len(), 1, "{sent:?}");
        assert_eq!(sent[0].0, "session.new_split");
        assert_eq!(sent[0].2["anchor_id"], "block:c", "{button}");
        assert_eq!(sent[0].2["direction"], direction, "{button}");
        assert_eq!(sent[0].2["side"], "after");
        assert_eq!(sent[0].2["cwd_from"], "block:c");
    }

    click(&mut rt, "pane-zoom:block:c");
    let sent = requests(&host);
    assert_eq!(
        sent.iter().map(|r| r.0.as_str()).collect::<Vec<_>>(),
        ["session.focus_block", "session.zoom_block"]
    );
    assert!(sent.iter().all(|r| r.2["block_id"] == "block:c"));
    assert!(rt.state.panel.is_none(), "none of them opens anything");

    click(&mut rt, "pane-close:block:c");
    assert!(requests(&host).is_empty(), "Close asks first");
    let panel = rt.state.panel.as_ref().expect("the confirmation");
    assert_eq!(panel.title, "Close Terminal in dev?");
    assert!(
        matches!(&panel.kind, PanelKind::Confirm { method, payload, .. }
            if method == "block.close" && payload["block_id"] == "block:c"
                && payload["session_id"] == "s")
    );
    assert_eq!(
        panel.target.as_ref().map(|t| t.block.as_str()),
        Some("block:c")
    );
    assert_eq!(
        focused(&rt).and_then(|path| {
            rt.elements()
                .into_iter()
                .find(|e| e.path == path)
                .and_then(|e| e.label.or(e.text))
        }),
        Some("Keep running".into()),
        "and opens on Keep running"
    );
    press(&mut rt, Key::Escape);
    assert!(rt.state.panel.is_none());
    assert!(requests(&host).is_empty(), "nothing was closed");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn header_buttons_show_the_bound_keys_and_never_hold_focus() {
    let (mut rt, _host) = drawn("split", PaneHeader::Auto, PaneDensity::Comfortable);
    let help = |rt: &Runtime<Workspace>, key: &str| -> String {
        let state = &rt.state;
        let args = match key {
            "right" | "down" => serde_json::json!({ "direction": key }),
            _ => Value::Null,
        };
        state.shortcut_detail(
            match key {
                "right" | "down" => "pane.split",
                other => other,
            },
            &args,
        )
    };
    assert!(
        !help(&rt, "right").is_empty() && !help(&rt, "down").is_empty(),
        "the default keymap binds both splits"
    );
    assert_ne!(help(&rt, "right"), help(&rt, "down"));
    click(&mut rt, "pane-zoom:block:b");
    assert_eq!(
        focused(&rt),
        None,
        "a header button leaves keys to the terminal"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_zoomed_pane_says_so_and_its_button_restores() {
    let (mut ws, host) = tabbed_workspace("split");
    ws.hosts[0].model.views.get_mut("s").unwrap().windows[1].zoomed_block_id =
        Some("block:b".into());
    let mut rt = runtime(ws);
    settle(&mut rt);
    assert!(has(&rt, "pane-header:block:b") && !has(&rt, "pane:block:c"));
    let chip = rect(&rt, "pane-zoomed:block:b");
    let header = rect(&rt, "pane-header:block:b");
    assert!(chip[1] >= header[1] && chip[1] + chip[3] <= header[1] + header[3]);
    assert!(
        rt.elements()
            .iter()
            .any(|e| !e.exiting && e.text.as_deref() == Some("Zoomed"))
    );
    let zoom = element(&rt, "pane-zoom:block:b");
    assert_eq!(zoom.label.as_deref(), Some("Restore pane"));
    let _ = requests(&host);
    click(&mut rt, "pane-zoom:block:b");
    assert!(
        requests(&host)
            .iter()
            .any(|r| r.0 == "session.zoom_block" && r.2["block_id"] == "block:b"),
        "the same request toggles it back"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();

    let (rt, _host) = drawn("split", PaneHeader::Auto, PaneDensity::Comfortable);
    assert!(!has(&rt, "pane-zoomed:block:b"), "no chip when not zoomed");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn auto_gives_a_single_pane_a_header_row_edge_to_edge_and_compact_none() {
    let (mut ws, host) = tabbed_workspace("single");
    let (sender, _sent) = flume::unbounded();
    framed_terminal(&mut ws, "block:a", &sender, b"$ ");
    assert_eq!(ws.settings.pane_header, PaneHeader::Auto);
    assert_eq!(ws.settings.pane_density, PaneDensity::Comfortable);
    let mut rt = runtime(ws);
    settle(&mut rt);
    // Still no card or margin: the pane fills the area, with the header across its top.
    assert_eq!(rect(&rt, "pane:block:a"), rt.state.area);
    let header = rect(&rt, "pane-header:block:a");
    assert_eq!(
        header,
        [0., rt.state.area[1], rt.state.area[2], 30.],
        "a row across the top"
    );
    let grid = rect(&rt, "grid:block:a");
    assert_eq!(
        grid[1],
        header[1] + header[3] + rt.state.settings.padding().1,
        "the terminal starts under it"
    );
    assert!(
        !has(&rt, "pane-header-band:block:a"),
        "a row, so nothing to hover"
    );
    // Edge to edge, the name lines up with the terminal's text under it.
    assert_eq!(rect(&rt, "pane-name:block:a")[0], grid[0]);
    // The same four buttons a split pane has, in the same order, at the trailing end.
    let at: Vec<f32> = BUTTONS
        .iter()
        .map(|name| rect(&rt, &format!("{name}:block:a"))[0])
        .collect();
    assert!(
        at.windows(2).all(|w| w[1] - w[0] == pane_header::BUTTON),
        "{at:?}"
    );
    let glyph_opacity = |rt: &Runtime<Workspace>, key: &str| {
        let button = element(rt, key);
        rt.elements()
            .into_iter()
            .filter(|e| !e.exiting && e.path.starts_with(&button.path) && e.path != button.path)
            .map(|e| e.opacity)
            .fold(1., f32::min)
    };
    // Split right and Split down are what the header is there for; Zoom has nothing to
    // hide. This session has a second tab, so the pane can be closed.
    for name in ["pane-split-right", "pane-split-down", "pane-close"] {
        assert_eq!(glyph_opacity(&rt, &format!("{name}:block:a")), 1., "{name}");
    }
    assert!((glyph_opacity(&rt, "pane-zoom:block:a") - pane_header::UNAVAILABLE).abs() < 0.01);
    let _ = requests(&host);
    click(&mut rt, "pane-split-down:block:a");
    assert!(
        requests(&host).iter().any(|r| r.0 == "session.new_split"),
        "Split down splits the lone pane"
    );

    // The session's last pane: Close keeps its place, dim, says why, and does nothing.
    rt.update(|s, _| {
        s.hosts[0]
            .model
            .views
            .get_mut("s")
            .unwrap()
            .windows
            .retain(|w| w.window_id.as_deref() == Some("single"));
    });
    settle(&mut rt);
    assert!((glyph_opacity(&rt, "pane-close:block:a") - pane_header::UNAVAILABLE).abs() < 0.01);
    assert_eq!(
        element(&rt, "pane-close:block:a").help.as_deref(),
        Some("Close pane · the session's only pane")
    );
    assert_eq!(glyph_opacity(&rt, "pane-split-right:block:a"), 1.);
    let _ = requests(&host);
    click(&mut rt, "pane-close:block:a");
    settle(&mut rt);
    assert!(rt.state.panel.is_none(), "no confirmation opens");
    assert!(requests(&host).is_empty());
    // The pane menu still offers it: only the header follows Rex for macOS here.
    let target = rt.state.target("block:a").unwrap();
    assert!(
        rt.state
            .pane_verb_available(&target, crate::app::panel::PaneAction::Close)
    );
    std::fs::remove_file(&rt.state.settings_path).ok();

    for active in ["single", "split"] {
        let (rt, _host) = drawn(active, PaneHeader::Auto, PaneDensity::Compact);
        for block in ["block:a", "block:b", "block:c"] {
            assert!(
                !has(&rt, &format!("pane-header:{block}")),
                "{active} {block}"
            );
        }
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn always_puts_a_header_on_a_single_pane_whose_zoom_cannot_run() {
    let (mut ws, host) = tabbed_workspace("single");
    ws.settings.pane_header = PaneHeader::Always;
    let (sender, _sent) = flume::unbounded();
    framed_terminal(&mut ws, "block:a", &sender, b"$ ");
    let mut rt = runtime(ws);
    settle(&mut rt);
    let card = rect(&rt, "pane:block:a");
    let area = rt.state.area;
    assert!(
        card[0] > 0. && card[2] < area[2],
        "a card inside the margins: {card:?}"
    );
    let header = rect(&rt, "pane-header:block:a");
    assert_eq!(header[3], 30.);
    let grid = rect(&rt, "grid:block:a");
    assert!(
        grid[1] >= header[1] + header[3],
        "the terminal starts under it"
    );
    for name in BUTTONS {
        assert!(has(&rt, &format!("{name}:block:a")), "{name}");
    }

    // Zoom has nothing to hide: dim, in its place, and a press does nothing.
    let glyph_opacity = |rt: &Runtime<Workspace>, key: &str| {
        let button = element(rt, key);
        rt.elements()
            .into_iter()
            .filter(|e| !e.exiting && e.path.starts_with(&button.path) && e.path != button.path)
            .map(|e| e.opacity)
            .fold(1., f32::min)
    };
    assert!((glyph_opacity(&rt, "pane-zoom:block:a") - pane_header::UNAVAILABLE).abs() < 0.01);
    assert_eq!(glyph_opacity(&rt, "pane-close:block:a"), 1.);
    assert_eq!(glyph_opacity(&rt, "pane-split-right:block:a"), 1.);
    let _ = requests(&host);
    click(&mut rt, "pane-zoom:block:a");
    assert!(requests(&host).is_empty() && rt.state.panel.is_none());

    click(&mut rt, "pane-split-right:block:a");
    assert_eq!(requests(&host)[0].0, "session.new_split");
    click(&mut rt, "pane-close:block:a");
    let panel = rt.state.panel.as_ref().expect("the confirmation");
    assert_eq!(panel.title, "Close Terminal in server?");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn always_gives_compact_panes_a_header_row_and_keeps_them_flush() {
    let (rt, _host) = drawn("split", PaneHeader::Always, PaneDensity::Compact);
    assert!(has(&rt, "pane-header:block:b") && has(&rt, "pane-header:block:c"));
    let (left, right) = (rect(&rt, "pane:block:b"), rect(&rt, "pane:block:c"));
    assert_eq!(left[0], 0.);
    assert_eq!(left[0] + left[2], right[0], "no gap or overlap");
    assert_eq!(right[0] + right[2], rt.state.area[2]);
    assert_eq!(
        rect(&rt, "pane-header:block:b")[1],
        left[1],
        "at the pane's top"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn hidden_draws_no_header_and_keeps_the_cards_and_retry() {
    let (mut ws, _host) = tabbed_workspace("split");
    ws.settings.pane_header = PaneHeader::Hidden;
    let (sender, _sent) = flume::unbounded();
    live_terminal(&mut ws, "block:b", &sender);
    live_terminal(&mut ws, "block:c", &sender);
    ws.terminals[&ws.term_key("block:c")]
        .connection
        .model
        .lock()
        .unwrap()
        .connected = false;
    let mut rt = runtime(ws);
    settle(&mut rt);
    assert!(!has(&rt, "pane-header:block:b") && !has(&rt, "pane-header:block:c"));
    assert!(!has(&rt, "pane-header-band:block:b"));
    let (left, right) = (rect(&rt, "pane:block:b"), rect(&rt, "pane:block:c"));
    assert!(
        right[0] - (left[0] + left[2]) >= 7.9,
        "still cards in gutters"
    );
    assert!(
        has(&rt, "strip-action:block:c"),
        "Retry floats on the pane with no header to hold it"
    );
    // Resting on the top edge brings nothing out.
    rest_at(&mut rt, left[0] + 40., left[1] + 10.);
    assert!(!has(&rt, "pane-header:block:b"));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn on_hover_floats_the_header_over_the_top_edge_without_moving_the_terminal() {
    let (mut ws, host) = tabbed_workspace("single");
    ws.settings.pane_header = PaneHeader::OnHover;
    let (sender, sent) = flume::unbounded();
    framed_terminal(&mut ws, "block:a", &sender, b"$ make\r\nok");
    let mut rt = runtime(ws);
    settle(&mut rt);
    let pane = rect(&rt, "pane:block:a");
    assert_eq!(pane, rt.state.area, "a single pane is still edge to edge");
    assert!(!has(&rt, "pane-header:block:a"));
    assert_eq!(rect(&rt, "pane-header-band:block:a")[3], 36.);
    let grid = rect(&rt, "grid:block:a");
    let (x, top) = (pane[0] + 200., pane[1]);

    // Below the band: nothing.
    rest_at(&mut rt, x, top + 60.);
    assert!(!has(&rt, "pane-header:block:a"));
    // A pass through the band on the way elsewhere: nothing either.
    rt.pointer_move(x, top + 20.);
    rt.frame(1. / 60.);
    rt.pointer_move(x, top + 80.);
    settle(&mut rt);
    assert!(!has(&rt, "pane-header:block:a"), "too quick to mean it");

    // Resting in the top 36 pt brings it out, within 150 ms.
    rt.pointer_move(x, top + 35.);
    for _ in 0..9 {
        rt.frame(1. / 60.);
    }
    assert!(has(&rt, "pane-header:block:a"), "revealed");
    for name in BUTTONS {
        assert!(has(&rt, &format!("{name}:block:a")), "{name}");
    }
    settle(&mut rt);
    let header = rect(&rt, "pane-header:block:a");
    assert_eq!((header[1], header[3]), (pane[1], 30.));
    assert_eq!(
        rect(&rt, "grid:block:a"),
        grid,
        "the terminal keeps its place"
    );
    assert!(sent_verbs(&sent).is_empty() && requests(&host).is_empty());

    // On the header itself it stays, and its buttons work.
    let close = rect(&rt, "pane-close:block:a");
    rest_at(&mut rt, close[0] + 12., close[1] + 12.);
    assert!(has(&rt, "pane-header:block:a"));
    click(&mut rt, "pane-split-down:block:a");
    assert_eq!(requests(&host)[0].0, "session.new_split");

    // Leaving the band puts it away.
    rt.pointer_move(x, top + 120.);
    settle(&mut rt);
    assert!(!has(&rt, "pane-header:block:a"), "gone again");
    assert_eq!(rt.state.header_reveal.hover, None);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_hover_band_leaves_presses_to_the_terminal_and_the_search_field_alone() {
    let (mut ws, _host) = tabbed_workspace("single");
    ws.settings.pane_header = PaneHeader::OnHover;
    let (sender, sent) = flume::unbounded();
    framed_terminal(&mut ws, "block:a", &sender, b"first line\r\nsecond");
    let mut rt = runtime(ws);
    settle(&mut rt);
    let grid = rect(&rt, "grid:block:a");
    // A press in the band, before any header shows, starts a selection in the terminal.
    let (x, y) = (grid[0] + 30., grid[1] + 6.);
    let band = rect(&rt, "pane-header-band:block:a");
    assert!(y < band[1] + band[3], "inside the band");
    rt.pointer_move(x, y);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1. / 60.);
    assert!(
        sent_verbs(&sent)
            .iter()
            .any(|(block, verb)| block == "block:a" && verb.starts_with("selection Down")),
        "the terminal took the press"
    );

    // With the search field open the header stays away from that corner.
    rt.update(|s, _| s.open_search());
    settle(&mut rt);
    assert!(has(&rt, "search:block:a"));
    assert!(!has(&rt, "pane-header-band:block:a"));
    rest_at(&mut rt, grid[0] + 30., grid[1] + 6.);
    assert!(!has(&rt, "pane-header:block:a"));
    rt.update(|s, _| s.toggle_pane_headers());
    settle(&mut rt);
    assert!(
        !has(&rt, "pane-header:block:a"),
        "not even when headers are revealed"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn on_hover_reveals_one_split_pane_at_a_time_inside_its_card() {
    let (mut rt, _host) = drawn("split", PaneHeader::OnHover, PaneDensity::Comfortable);
    assert!(!has(&rt, "pane-header:block:b") && !has(&rt, "pane-header:block:c"));
    let (left, right) = (rect(&rt, "pane:block:b"), rect(&rt, "pane:block:c"));
    rest_at(&mut rt, right[0] + 50., right[1] + 12.);
    assert!(has(&rt, "pane-header:block:c") && !has(&rt, "pane-header:block:b"));
    settle(&mut rt);
    let header = rect(&rt, "pane-header:block:c");
    assert_eq!(
        (header[0], header[1], header[2]),
        (right[0] + 1., right[1] + 1., right[2] - 2.),
        "inside the card's border"
    );
    rest_at(&mut rt, left[0] + 50., left[1] + 12.);
    settle(&mut rt);
    assert!(has(&rt, "pane-header:block:b") && !has(&rt, "pane-header:block:c"));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn reveal_pane_headers_is_a_local_toggle_that_never_resizes_a_terminal() {
    let spec = actions::find("client.pane_headers.toggle").expect("registered");
    assert_eq!(spec.title(), "Reveal pane headers");
    assert!(matches!(spec.remote(), actions::Remote::Never(_)));
    assert!(spec.accepts(&Value::Null) && !spec.accepts(&serde_json::json!({"x": 1})));
    assert!(!spec.repeats() && !spec.global());
    assert!(
        rex_rune::keymap::defaults()
            .bindings
            .iter()
            .all(|b| b.action != "client.pane_headers.toggle"),
        "no key is taken from terminal programs by default"
    );

    let commands_row = |rt: &Runtime<Workspace>| {
        rt.state
            .entries()
            .into_iter()
            .find(|e| e.label.ends_with("pane headers"))
            .map(|e| (e.label, e.enabled))
            .expect("a Commands row")
    };
    for header in [PaneHeader::Auto, PaneHeader::OnHover, PaneHeader::Hidden] {
        let (mut ws, host) = tabbed_workspace("single");
        ws.settings.pane_header = header;
        // Auto has rows on every pane in Comfortable, and so nothing to reveal there.
        if header == PaneHeader::Auto {
            ws.settings.pane_density = PaneDensity::Compact;
        }
        let (sender, sent) = flume::unbounded();
        framed_terminal(&mut ws, "block:a", &sender, b"$ ");
        let mut rt = runtime(ws);
        settle(&mut rt);
        let grid = rect(&rt, "grid:block:a");
        assert!(!has(&rt, "pane-header:block:a"), "{header:?}");
        let saved = std::fs::read(&rt.state.settings_path).ok();

        rt.update(|s, _| s.picker(false));
        rt.frame(1. / 60.);
        assert_eq!(commands_row(&rt), ("Reveal pane headers".into(), true));
        press(&mut rt, Key::Escape);

        rt.update(|s, tx| {
            assert!(actions::run(
                s,
                "client.pane_headers.toggle",
                &Value::Null,
                tx
            ));
        });
        settle(&mut rt);
        assert!(has(&rt, "pane-header:block:a"), "{header:?}: revealed");
        assert!(has(&rt, "pane-close:block:a"));
        assert_eq!(rect(&rt, "grid:block:a"), grid, "{header:?}: same rows");
        rt.update(|s, _| s.picker(false));
        rt.frame(1. / 60.);
        assert_eq!(commands_row(&rt), ("Hide pane headers".into(), true));
        press(&mut rt, Key::Escape);

        rt.update(|s, tx| {
            actions::run(s, "client.pane_headers.toggle", &Value::Null, tx);
        });
        settle(&mut rt);
        assert!(!has(&rt, "pane-header:block:a"), "{header:?}: put back");
        assert!(
            sent_verbs(&sent).is_empty() && requests(&host).is_empty(),
            "{header:?}: nothing was sent"
        );
        assert_eq!(
            std::fs::read(&rt.state.settings_path).ok(),
            saved,
            "and nothing was saved"
        );
        std::fs::remove_file(&rt.state.settings_path).ok();
    }

    // Always has nothing to bring out.
    let (mut rt, _host) = drawn("single", PaneHeader::Always, PaneDensity::Comfortable);
    rt.update(|s, _| s.toggle_pane_headers());
    assert!(!rt.state.header_reveal.pinned);
    rt.update(|s, _| s.picker(false));
    rt.frame(1. / 60.);
    assert_eq!(commands_row(&rt), ("Reveal pane headers".into(), false));
    std::fs::remove_file(&rt.state.settings_path).ok();
}

/// Opens the Pane headers popup button in Settings and presses the row named `label`.
fn choose_pane_headers(rt: &mut Runtime<Workspace>, label: &str) {
    let r = rect(rt, "pane-header");
    let (x, y) = (r[0] + r[2] / 2., r[1] + r[3] / 2.);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    settle(rt);
    assert!(
        matches!(
            rt.state.panel.as_ref().map(|p| &p.kind),
            Some(PanelKind::Choice(_))
        ),
        "the control opens its list"
    );
    let r = rt
        .elements()
        .into_iter()
        .find(|e| !e.exiting && e.role == "menuitem" && e.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("no {label} row"))
        .rect;
    let (x, y) = (r[0] + r[2] / 2., r[1] + r[3] / 2.);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    settle(rt);
}

#[test]
fn the_pane_headers_setting_saves_each_choice_and_a_relaunch_keeps_it() {
    let (mut ws, _host) = tabbed_workspace("split");
    ws.perform(Intent::Settings, &mut rune::Tx::default());
    let mut rt = runtime_at(ws, (1440., 1000.));
    settle(&mut rt);
    assert_eq!(element(&rt, "pane-header").label.as_deref(), Some("Auto"));
    assert_eq!(rt.state.settings.pane_header, PaneHeader::Auto);
    for (label, header) in [
        ("Always", PaneHeader::Always),
        ("On hover", PaneHeader::OnHover),
        ("Hidden", PaneHeader::Hidden),
        ("Auto", PaneHeader::Auto),
    ] {
        choose_pane_headers(&mut rt, label);
        assert_eq!(rt.state.settings.pane_header, header);
        assert_eq!(relaunch(&rt.state).settings.pane_header, header);
        assert!(
            matches!(
                rt.state.panel.as_ref().map(|p| &p.kind),
                Some(PanelKind::Settings)
            ),
            "Settings stays open"
        );
        assert_eq!(
            element(&rt, "pane-header").label.as_deref(),
            Some(label),
            "the control shows the choice"
        );
    }
    // A choice puts away headers that were revealed by hand. (Auto has a row on every pane
    // in Comfortable, and so nothing to reveal: start from a choice that has.)
    choose_pane_headers(&mut rt, "On hover");
    rt.update(|s, _| s.toggle_pane_headers());
    assert!(rt.state.header_reveal.pinned);
    choose_pane_headers(&mut rt, "Hidden");
    assert!(!rt.state.header_reveal.pinned);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_narrow_pane_keeps_zoom_and_close() {
    let (mut ws, _host) = tabbed_workspace("split");
    // A 15% left pane of a 640 pt window is under 200 pt wide.
    let view = ws.hosts[0].model.views.get_mut("s").unwrap();
    let layout = serde_json::json!({"split": {"direction": "horizontal", "ratio": 0.15,
        "before": {"block_id": "block:b"}, "after": {"block_id": "block:c"}}});
    view.windows[1].layers[0].layout = serde_json::from_value(layout).unwrap();
    let mut rt = runtime_at(ws, (640., 480.));
    settle(&mut rt);
    let narrow = rect(&rt, "pane:block:b");
    assert!(narrow[2] < 200., "{narrow:?}");
    assert!(has(&rt, "pane-zoom:block:b") && has(&rt, "pane-close:block:b"));
    assert!(!has(&rt, "pane-split-right:block:b") && !has(&rt, "pane-split-down:block:b"));
    let close = rect(&rt, "pane-close:block:b");
    assert!(
        close[0] + close[2] <= narrow[0] + narrow[2],
        "inside the pane"
    );
    assert!(
        has(&rt, "pane-split-right:block:c"),
        "the wide pane has all"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

/// The header under each choice as drawn, for a look (see `Camera`).
#[test]
fn pane_headers_draw_under_each_choice() {
    use rex_rune::connection::activity::{BlockedKind, ProgramState::*};
    let Some(mut camera) = Camera::new() else {
        return;
    };
    let scene = |active: &str, header: PaneHeader, density: PaneDensity| {
        let (mut ws, _host) = tabbed_workspace(active);
        ws.settings.pane_header = header;
        ws.settings.pane_density = density;
        let (sender, _sent) = flume::unbounded();
        framed_terminal(
            &mut ws,
            "block:a",
            &sender,
            b"$ tail -f server.log\r\nlistening on :8080\r\nGET /health 200",
        );
        framed_terminal(
            &mut ws,
            "block:b",
            &sender,
            b"$ cargo test\r\nrunning 12 tests\r\ntest result: ok",
        );
        framed_terminal(&mut ws, "block:c", &sender, b"$ claude\r\n> waiting");
        let mut blocked = record(Blocked, true);
        blocked.kind = Some(BlockedKind::Question);
        ws.hosts[0]
            .model
            .activity
            .insert("block:c".into(), [("".into(), blocked)].into());
        let mut rt = Runtime::new(ws, view, (1024., 700.), Env::new(true, 2.));
        rt.on_key = Some(crate::app::input::on_key);
        settle(&mut rt);
        rt
    };

    let mut rt = scene("split", PaneHeader::Auto, PaneDensity::Comfortable);
    camera.snap(&mut rt, "header-split");
    let close = rect(&rt, "pane-close:block:b");
    rt.pointer_move(close[0] + 12., close[1] + 12.);
    settle(&mut rt);
    camera.snap(&mut rt, "header-split-close-hover");
    let split = rect(&rt, "pane-split-right:block:c");
    rt.pointer_move(split[0] + 12., split[1] + 12.);
    settle(&mut rt);
    camera.snap(&mut rt, "header-split-button-hover");
    std::fs::remove_file(&rt.state.settings_path).ok();

    let mut rt = scene("split", PaneHeader::Auto, PaneDensity::Comfortable);
    rt.update(|s, _| {
        s.hosts[0].model.views.get_mut("s").unwrap().windows[1].zoomed_block_id =
            Some("block:b".into());
    });
    settle(&mut rt);
    camera.snap(&mut rt, "header-zoomed");
    std::fs::remove_file(&rt.state.settings_path).ok();

    // A lone pane in Auto: its header across the top, edge to edge.
    let mut rt = scene("single", PaneHeader::Auto, PaneDensity::Comfortable);
    camera.snap(&mut rt, "header-auto-single");
    let split = rect(&rt, "pane-split-right:block:a");
    rt.pointer_move(split[0] + 12., split[1] + 12.);
    settle(&mut rt);
    camera.snap(&mut rt, "header-auto-single-split-hover");
    // The session's last pane: Zoom and Close keep their places, dim.
    rt.update(|s, _| {
        s.hosts[0]
            .model
            .views
            .get_mut("s")
            .unwrap()
            .windows
            .retain(|w| w.window_id.as_deref() == Some("single"));
    });
    rt.pointer_move(500., 400.);
    settle(&mut rt);
    camera.snap(&mut rt, "header-auto-last-pane");
    std::fs::remove_file(&rt.state.settings_path).ok();

    let mut rt = scene("single", PaneHeader::Always, PaneDensity::Comfortable);
    camera.snap(&mut rt, "header-always-single");
    std::fs::remove_file(&rt.state.settings_path).ok();

    let mut rt = scene("split", PaneHeader::Always, PaneDensity::Compact);
    camera.snap(&mut rt, "header-always-compact");
    std::fs::remove_file(&rt.state.settings_path).ok();

    let mut rt = scene("single", PaneHeader::OnHover, PaneDensity::Comfortable);
    camera.snap(&mut rt, "header-hover-single-rest");
    let pane = rect(&rt, "pane:block:a");
    rt.pointer_move(pane[0] + 300., pane[1] + 14.);
    settle(&mut rt);
    camera.snap(&mut rt, "header-hover-single");
    std::fs::remove_file(&rt.state.settings_path).ok();

    let mut rt = scene("split", PaneHeader::OnHover, PaneDensity::Comfortable);
    let pane = rect(&rt, "pane:block:c");
    rt.pointer_move(pane[0] + 200., pane[1] + 14.);
    settle(&mut rt);
    camera.snap(&mut rt, "header-hover-split");
    std::fs::remove_file(&rt.state.settings_path).ok();

    let mut rt = scene("split", PaneHeader::Hidden, PaneDensity::Comfortable);
    camera.snap(&mut rt, "header-hidden-split");
    std::fs::remove_file(&rt.state.settings_path).ok();

    let mut rt = scene("split", PaneHeader::Auto, PaneDensity::Comfortable);
    rt.update(|s, tx| s.perform(Intent::Settings, tx));
    settle(&mut rt);
    camera.snap(&mut rt, "header-settings");
    std::fs::remove_file(&rt.state.settings_path).ok();
}
