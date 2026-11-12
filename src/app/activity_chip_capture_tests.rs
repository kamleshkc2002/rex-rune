//! Real-font evidence; this file also runs unchanged on master for the quiet-pixel proof.
use super::pane_verb_tests::{Camera, framed_terminal};
use super::*;
use rex_rune::connection::activity::{ProgramState, StatusRecord};

fn draw(case: &str) {
    let Some(mut camera) = Camera::new() else {
        return;
    };
    let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
    let _families = rune::text::with_families("Inter", "IBM Plex Mono");
    let (mut ws, _host) = tabbed_workspace("single");
    let (sender, _terminal) = flume::unbounded();
    framed_terminal(
        &mut ws,
        "block:a",
        &sender,
        b"$ cargo test\r\nAll tests passed.\r\n$ ",
    );
    if case != "quiet" {
        ws.hosts[0].model.activity.insert(
            "block:b".into(),
            [(
                String::new(),
                StatusRecord {
                    state: if case.starts_with("error") {
                        ProgramState::Error
                    } else {
                        ProgramState::Blocked
                    },
                    live: true,
                    ..Default::default()
                },
            )]
            .into(),
        );
    }
    if case.ends_with("download") {
        let mut t = rex_rune::download::Transfer::new(
            1,
            "s".into(),
            "block:a".into(),
            "~/build.log".into(),
            std::env::temp_dir(),
            1,
        );
        t.name = Some("build.log".into());
        t.state = rex_rune::download::State::Receiving {
            received: 42,
            hint: 100,
        };
        ws.hosts[0].model.downloads.push(t);
    }
    let size = if std::env::var_os("REX_M32_MINIMUM").is_some() {
        (640., 480.)
    } else {
        (1024., 700.)
    };
    ws.viewport = [size.0, size.1];
    let mut rt = Runtime::with_text(
        ws,
        view,
        size,
        Env::new(true, 1.),
        rune::text::TextSystem::new(fonts.files, fonts.families),
    );
    for _ in 0..6 {
        rt.update(|s, tx| {
            crate::app::poll(s, 0., tx);
        });
        for _ in 0..30 {
            rt.frame(1. / 60.);
        }
    }
    camera.snap(&mut rt, &format!("activity-chip-{case}"));
}
#[test]
fn activity_chip_quiet_draw() {
    draw("quiet");
}
#[test]
fn activity_chip_waiting_draw() {
    draw("waiting");
}
#[test]
fn activity_chip_error_draw() {
    draw("error");
}
#[test]
fn activity_chip_waiting_download_draw() {
    draw("waiting-download");
}
#[test]
fn activity_chip_error_download_draw() {
    draw("error-download");
}
