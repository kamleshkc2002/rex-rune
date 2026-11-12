use super::*;
use crate::app::updates::Installation;
use rex_linux::update::{InstallChannel, UpdateState};

#[test]
fn updater_check_and_install_are_mutually_exclusive() {
    let mut ws = workspace();
    ws.updates.channel = InstallChannel::Tarball;
    ws.updates.state.latest_version = Some("99.0.0".into());
    ws.updates.checking = true;
    ws.install_update_now();
    assert!(matches!(ws.updates.installation, Installation::Idle));
    ws.updates.checking = false;
    ws.updates.installation = Installation::Working("Downloading…".into());
    ws.check_updates_now();
    assert!(!ws.updates.checking);
    ws.updates.installation = Installation::Ready("/nonexistent/rex-linux".into());
    ws.check_updates_now();
    assert!(!ws.updates.checking);
    let _ = std::fs::remove_file(&ws.settings_path);
}

#[test]
fn updater_stale_workspace_notice_actions_never_lower_verified_version() {
    use rex_linux::update::CheckOutcome;
    let mut ws = workspace();
    let temp = tempfile::tempdir().unwrap();
    ws.updates.config.state_path = temp.path().join("state.json");
    let old = UpdateState {
        highest_verified: Some("0.8.0".into()),
        latest_version: Some("0.8.0".into()),
        ..UpdateState::default()
    };
    let newer = UpdateState {
        highest_verified: Some("99.0.0".into()),
        latest_version: Some("99.0.0".into()),
        ..UpdateState::default()
    };
    newer.save(&ws.updates.config.state_path).unwrap();
    ws.updates.state = old.clone();
    ws.dismiss_update_version("0.8.0");
    assert_eq!(
        UpdateState::load(&ws.updates.config.state_path).highest_verified,
        newer.highest_verified
    );
    ws.finish_update_check(
        true,
        Ok((
            CheckOutcome::UpdateAvailable {
                version: "0.8.0".into(),
                notes_url: String::new(),
                published: String::new(),
            },
            old,
        )),
    );
    let saved = UpdateState::load(&ws.updates.config.state_path);
    assert_eq!(saved.highest_verified, newer.highest_verified);
    assert_eq!(saved.latest_version, newer.latest_version);
    assert_eq!(saved.dismissed_version, None);
    assert_eq!(ws.updates.state, saved);
    let _ = std::fs::remove_file(&ws.settings_path);
}

#[test]
fn updater_restart_requests_normal_shutdown_instead_of_exec_in_callback() {
    let mut ws = workspace();
    ws.updates.channel = InstallChannel::Tarball;
    let binary = std::path::PathBuf::from("/nonexistent/rex-linux");
    ws.updates.installation = Installation::Ready(binary.clone());
    let restart = ws.restart.clone();
    ws.open_updates_settings();
    let mut rt = runtime(ws);
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    click(&mut rt, "updates-restart");
    assert_eq!(*restart.lock().unwrap(), Some(binary));
    assert!(
        !rt.state
            .notices
            .iter()
            .any(|notice| notice.text.contains("Restart failed"))
    );
    let _ = std::fs::remove_file(&rt.state.settings_path);
}

#[test]
fn updater_controls_are_channel_gated_and_show_progress_error_and_restart() {
    let mut ws = workspace();
    ws.updates.channel = InstallChannel::Tarball;
    ws.updates.state = UpdateState {
        latest_version: Some("99.0.0".into()),
        notes_url: Some("https://github.com/shuv1337/rex-rune/releases/tag/v99.0.0".into()),
        ..UpdateState::default()
    };
    ws.open_updates_settings();
    let mut rt = runtime(ws);
    for _ in 0..60 {
        rt.frame(1. / 60.);
    }
    assert!(has(&rt, "updates-install-now"));
    assert!(has(&rt, "updates-copy-command"));
    assert!(!rt.state.settings.updates.install_automatically);
    click(&mut rt, "updates-install-auto-switch");
    assert!(
        Settings::load(&rt.state.settings_path)
            .unwrap()
            .updates
            .install_automatically
    );
    rt.update(|s, _| {
        s.updates.installation = Installation::Working("Downloading update… 42%".into())
    });
    rt.frame(1. / 60.);
    assert!(!has(&rt, "updates-install-now"));
    assert!(
        rt.elements()
            .iter()
            .any(|e| e.text.as_deref().is_some_and(|t| t.contains("42%")))
    );
    rt.update(|s, _| {
        s.updates.installation =
            Installation::Failed("tarball SHA-256 differs from signed manifest".into())
    });
    rt.frame(1. / 60.);
    assert!(has(&rt, "updates-install-now"));
    assert!(
        rt.elements()
            .iter()
            .any(|e| e.text.as_deref().is_some_and(|t| t.contains("SHA-256")))
    );
    rt.update(|s, _| s.updates.installation = Installation::Ready("/nonexistent/rex-linux".into()));
    rt.frame(1. / 60.);
    assert!(has(&rt, "updates-restart"));
    assert!(!has(&rt, "updates-install-now"));
    // Do not re-exec in a headless test. A second click cannot start a new install.
    rt.state.install_update_now();
    assert!(matches!(
        rt.state.updates.installation,
        Installation::Ready(_)
    ));
    rt.update(|s, _| {
        s.updates.installation = Installation::Idle;
        s.updates.state.unsupported_floor = Some("98.0.0".into());
    });
    rt.frame(1. / 60.);
    assert!(!has(&rt, "updates-install-now"));
    for channel in [
        InstallChannel::Source,
        InstallChannel::Nix,
        InstallChannel::Unmanaged,
        InstallChannel::Deb { custom_hint: None },
        InstallChannel::Pacman {
            omarchy: false,
            custom_hint: None,
        },
    ] {
        rt.update(|s, _| {
            s.updates.channel = channel;
            s.updates.state.unsupported_floor = None;
        });
        rt.frame(1. / 60.);
        assert!(!has(&rt, "updates-install-now"));
        assert!(!has(&rt, "updates-install-auto-switch"));
        rt.state.install_update_now();
        assert!(matches!(rt.state.updates.installation, Installation::Idle));
    }
    let _ = std::fs::remove_file(&rt.state.settings_path);
}

/// Run alone with REX_TEST_SNAP_DIR for real-font captures; optionally REX_UPDATE_MINIMUM=1.
#[test]
fn updater_settings_render_install_states() {
    let fonts = || {
        let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
        rune::text::TextSystem::new(fonts.files, fonts.families)
    };
    let mut camera = super::pane_verb_tests::Camera::new();
    if camera.is_some() {
        drop(fonts());
    }
    let mut ws = workspace();
    ws.updates.channel = InstallChannel::Tarball;
    ws.updates.state = UpdateState {
        latest_version: Some("99.0.0".into()),
        notes_url: Some("https://github.com/shuv1337/rex-rune/releases/tag/v99.0.0".into()),
        ..UpdateState::default()
    };
    ws.open_updates_settings();
    let size = if std::env::var_os("REX_UPDATE_MINIMUM").is_some() {
        (640., 480.)
    } else {
        (1024., 700.)
    };
    let mut rt = if camera.is_some() {
        Runtime::with_text(ws, view, size, Env::new(true, 2.), fonts())
    } else {
        Runtime::new(ws, view, size, Env::new(true, 2.))
    };
    for (name, state) in [
        ("updater-available", Installation::Idle),
        (
            "updater-progress",
            Installation::Working("Downloading update… 42%".into()),
        ),
        (
            "updater-failed",
            Installation::Failed("tarball SHA-256 differs from signed manifest".into()),
        ),
        (
            "updater-ready",
            Installation::Ready("/nonexistent/rex-linux".into()),
        ),
    ] {
        rt.update(|s, _| s.updates.installation = state);
        for _ in 0..60 {
            rt.frame(1. / 60.);
        }
        if let Some(camera) = &mut camera {
            camera.snap(&mut rt, name);
        }
    }
}
