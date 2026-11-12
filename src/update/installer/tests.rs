use super::*;
use std::net::TcpListener;

fn serve(bodies: Vec<Vec<u8>>) -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let thread = std::thread::spawn(move || {
        for body in bodies {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            assert!(stream.read(&mut request).unwrap() > 0);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
        }
    });
    (base, thread)
}

#[test]
fn untrusted_equal_older_and_unsupported_releases_never_download_or_modify_install() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    for (version, floor, expected, tamper, unknown, highest) in [
        ("0.7.3", "0.7.0", "0.7.3", false, false, None),
        ("0.7.2", "0.7.0", "0.7.2", false, false, None),
        ("0.9.0", "0.8.0", "0.9.0", false, false, None),
        ("0.8.0", "0.7.0", "0.9.0", false, false, None),
        ("0.8.0", "0.7.0", "0.8.0", false, false, Some("0.9.0")),
        ("0.8.0", "0.7.0", "0.8.0", true, false, None),
        ("0.8.0", "0.7.0", "0.8.0", false, true, None),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let layout = layout(temp.path());
        let old = contents(&layout);
        let mut bytes = super::super::tests::sample_manifest_json(version, floor);
        let (id, pair, key) = super::super::tests::test_keypair();
        let sig = super::super::tests::sign_minisig(
            &id,
            &pair,
            &bytes,
            &format!("rex-linux v{version} latest.json"),
            true,
        );
        if tamper {
            bytes.push(b' ');
        }
        let (base, server) = serve(vec![bytes, sig.into_bytes()]);
        let config = CheckConfig {
            current_version: "0.7.3".into(),
            manifest_url: format!("{base}/latest.json"),
            signature_url: format!("{base}/latest.json.minisig"),
            public_keys: vec![if unknown { CI_PUBLIC_KEY.into() } else { key }],
            state_path: temp.path().join("state.json"),
            allow_loopback_http: true,
        };
        UpdateState {
            highest_verified: highest.map(str::to_owned),
            ..UpdateState::default()
        }
        .save(&config.state_path)
        .unwrap();
        assert!(
            install_with(&config, expected, &layout, &mut |_| {}, |_, _, _| panic!(
                "must not download"
            ))
            .is_err()
        );
        server.join().unwrap();
        assert_eq!(contents(&layout), old);
        assert!(!layout.transaction().exists());
    }
}

fn layout(temp: &Path) -> InstallLayout {
    let layout = InstallLayout {
        local: temp.join(".local"),
        binary: temp.join(".local/bin/rex-linux"),
        data: temp.join(".local/share"),
    };
    for path in layout.destinations() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"old").unwrap();
    }
    fs::write(
        layout.receipt(),
        serde_json::to_vec(&InstallReceipt {
            schema: 1,
            path: layout.binary.to_string_lossy().into(),
            version: "0.7.3".into(),
            sha256: hash_file(&layout.binary).unwrap(),
        })
        .unwrap(),
    )
    .unwrap();
    layout
}

fn archive(files: &[(&str, &[u8], tar::EntryType)]) -> Vec<u8> {
    let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut builder = tar::Builder::new(gzip);
    for (name, bytes, kind) in files {
        let mut header = tar::Header::new_gnu();
        header.set_mode(0o644);
        header.set_size(bytes.len() as u64);
        header.set_entry_type(*kind);
        // Real Python tarfile releases use PAX for long paths.
        if name.len() > 100 {
            builder
                .append_pax_extensions([("path", name.as_bytes())])
                .unwrap();
            header.set_path("pax-placeholder").unwrap();
        } else {
            // Raw paths deliberately allow constructing hostile fixture entries.
            header.as_mut_bytes()[..name.len()].copy_from_slice(name.as_bytes());
        }
        if kind.is_symlink() || kind.is_hard_link() {
            header.set_link_name("../../escape").unwrap();
        }
        header.set_cksum();
        builder.append(&header, *bytes).unwrap();
    }
    builder.into_inner().unwrap().finish().unwrap()
}

#[test]
fn hostile_tar_paths_links_duplicates_and_sizes_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    for name in [
        "/pkg/file",
        "pkg/../escape",
        "pkg/./file",
        "other/file",
        "pkg//file",
    ] {
        assert!(safe_path(name.as_bytes(), "pkg").is_err(), "{name}");
        let file = temp.path().join("bad.tar.gz");
        fs::write(&file, archive(&[(name, b"bad", tar::EntryType::Regular)])).unwrap();
        let out = tempfile::tempdir().unwrap();
        assert!(extract(&file, out.path(), "pkg").is_err(), "{name}");
    }
    for kind in [
        tar::EntryType::Symlink,
        tar::EntryType::Link,
        tar::EntryType::Fifo,
        tar::EntryType::Char,
    ] {
        let file = temp.path().join("bad.tar.gz");
        fs::write(&file, archive(&[("pkg/link", b"", kind)])).unwrap();
        let out = tempfile::tempdir().unwrap();
        assert!(extract(&file, out.path(), "pkg").is_err());
    }
    let file = temp.path().join("duplicate.tar.gz");
    fs::write(
        &file,
        archive(&[
            ("pkg/file", b"a", tar::EntryType::Regular),
            ("pkg/file", b"b", tar::EntryType::Regular),
        ]),
    )
    .unwrap();
    let out = tempfile::tempdir().unwrap();
    assert!(extract(&file, out.path(), "pkg").is_err());

    // A huge declared size must fail before allocating/writing its payload.
    let mut header = tar::Header::new_gnu();
    header.set_path("pkg/bomb").unwrap();
    header.set_size(MAX_UNPACKED + 1);
    header.set_mode(0o644);
    header.set_cksum();
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gzip.write_all(header.as_bytes()).unwrap();
    fs::write(&file, gzip.finish().unwrap()).unwrap();
    let error = extract(&file, out.path(), "pkg").unwrap_err();
    assert!(error.to_string().contains("exceeds 1 GiB"), "{error:#}");
    assert!(!out.path().join("pkg/bomb").exists());

    let names: Vec<_> = (0..20_001).map(|i| format!("pkg/{i}")).collect();
    let files: Vec<_> = names
        .iter()
        .map(|n| (n.as_str(), &b""[..], tar::EntryType::Regular))
        .collect();
    fs::write(&file, archive(&files)).unwrap();
    let out = tempfile::tempdir().unwrap();
    let error = extract(&file, out.path(), "pkg").unwrap_err();
    assert!(
        error.to_string().contains("too many tar entries"),
        "{error:#}"
    );
    assert!(!out.path().join("pkg/20000").exists());
}

#[test]
fn download_is_exactly_bounded_and_hash_verified() {
    for error in [
        anyhow::Error::from(ureq::Error::Timeout(ureq::Timeout::Global)),
        anyhow::Error::from(io::Error::new(io::ErrorKind::TimedOut, "receive stalled")),
        anyhow::Error::from(io::Error::other(ureq::Error::Timeout(
            ureq::Timeout::Global,
        ))),
    ] {
        assert!(download_error(error).to_string().contains("timed out"));
    }
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("tarball");
    for (bytes, limit) in [
        (&b"abc"[..], 2),
        (&b"a"[..], 2),
        (&b""[..], 0),
        (&b""[..], MAX_ARCHIVE + 1),
    ] {
        assert!(copy_download(bytes, &file, limit, &mut |_| {}).is_err());
    }
    copy_download(&b"abc"[..], &file, 3, &mut |_| {}).unwrap();
    let mut artifact = TarballArtifact {
        url: "https://example.com/a".into(),
        size: 3,
        sha256: hex_sha256(b"abc"),
    };
    verify_archive(&file, &artifact).unwrap();
    artifact.sha256 = hex_sha256(b"bad");
    assert!(verify_archive(&file, &artifact).is_err());
    artifact.size = 4;
    assert!(verify_archive(&file, &artifact).is_err());
    assert!(
        download(
            &TarballArtifact {
                url: "http://127.0.0.1/a".into(),
                ..artifact
            },
            &file,
            &mut |_| {}
        )
        .is_err()
    );
}

fn stage(layout: &InstallLayout) -> tempfile::TempDir {
    let stage = tempfile::tempdir_in(layout.binary.parent().unwrap()).unwrap();
    for i in 0..layout.destinations().len() {
        fs::write(stage.path().join(format!("new-{i}")), b"new").unwrap();
    }
    stage
}

fn interrupt(layout: &InstallLayout, replaced: usize, running_test_binary: bool) {
    let stage = stage(layout);
    if running_test_binary {
        let binary = stage.path().join("new-4");
        fs::remove_file(&binary).unwrap();
        fs::copy(std::env::current_exe().unwrap(), binary).unwrap();
    }
    let destinations = layout.destinations();
    let existed: Vec<_> = destinations.iter().map(|p| p.exists()).collect();
    for (i, dest) in destinations.iter().enumerate() {
        if existed[i] {
            fs::hard_link(dest, stage.path().join(format!("old-{i}"))).unwrap();
        }
    }
    let journal = Journal {
        binary: layout.binary.clone(),
        data: layout.data.clone(),
        existed,
        new_inodes: (0..destinations.len())
            .map(|i| identity(&stage.path().join(format!("new-{i}"))).unwrap())
            .collect(),
    };
    fs::write(
        stage.path().join("journal.json"),
        serde_json::to_vec(&journal).unwrap(),
    )
    .unwrap();
    fs::rename(stage.path(), layout.transaction()).unwrap();
    for (i, dest) in destinations.iter().enumerate().take(replaced) {
        fs::rename(layout.transaction().join(format!("new-{i}")), dest).unwrap();
    }
}

#[test]
fn manual_install_repair_supersedes_interrupted_update() {
    let temp = tempfile::tempdir().unwrap();
    let layout = layout(temp.path());
    interrupt(&layout, 5, false);
    let repair = temp.path().join("repair-binary");
    fs::write(&repair, b"#!/bin/sh\necho 'rex-linux 99.0.0'\n").unwrap();
    fs::set_permissions(&repair, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        Command::new("bash")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/scripts/install-desktop.sh"
            ))
            .args(["--copy", "--receipt-version", "99.0.0"])
            .arg(&repair)
            .env("HOME", temp.path())
            .env("XDG_BIN_HOME", layout.binary.parent().unwrap())
            .env("XDG_DATA_HOME", &layout.data)
            .env("REX_INSTALL_QUIET_SUMMARY", "1")
            .status()
            .unwrap()
            .success()
    );
    let repaired = contents(&layout);
    assert_eq!(recover(&layout).unwrap(), Recovery::Superseded);
    assert_eq!(contents(&layout), repaired);
    layout.validate_receipt("99.0.0").unwrap();
    assert!(!layout.transaction().exists());
}

#[test]
fn later_file_at_previously_absent_destination_supersedes_whole_transaction() {
    let temp = tempfile::tempdir().unwrap();
    let layout = layout(temp.path());
    let dest = &layout.destinations()[0];
    fs::remove_file(dest).unwrap();
    interrupt(&layout, 5, false);
    let later = temp.path().join("later");
    fs::write(&later, b"belongs to later installer").unwrap();
    fs::rename(later, dest).unwrap();
    let before = contents(&layout);
    assert_eq!(recover(&layout).unwrap(), Recovery::Superseded);
    assert_eq!(contents(&layout), before);
    assert!(!layout.transaction().exists());
}

#[test]
fn partial_rollback_failure_retains_journal_and_retries_remaining_files() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let layout = layout(temp.path());
    let before = contents(&layout);
    let error = commit(&layout, stage(&layout), |i| {
        if i == 5 {
            fs::create_dir(layout.transaction().join("restore-4"))?;
            bail!("injected commit failure");
        }
        Ok(())
    })
    .unwrap_err();
    assert!(format!("{error:#}").contains("rollback needs recovery"));
    assert!(layout.transaction().join("journal.json").exists());
    assert_eq!(&contents(&layout)[..4], &before[..4]);
    assert_eq!(fs::read(&layout.binary).unwrap(), b"new");
    fs::remove_dir(layout.transaction().join("restore-4")).unwrap();
    assert_eq!(
        recover(&layout).unwrap(),
        Recovery::Restored(layout.binary.clone())
    );
    assert_eq!(contents(&layout), before);
    assert!(!layout.transaction().exists());
}

#[test]
fn ordinary_launch_skips_busy_lock_and_root_housekeeping_without_notice() {
    let temp = tempfile::tempdir().unwrap();
    let layout = layout(temp.path());
    let stage = layout.binary.parent().unwrap().join(".rex-update-stale");
    fs::create_dir(&stage).unwrap();
    let lock = layout.lock().unwrap();
    let start = Instant::now();
    assert_eq!(
        recover_install_at(&layout, RECOVERY_WAIT, false).unwrap(),
        Recovery::Unchanged
    );
    assert!(start.elapsed() < Duration::from_millis(500));
    assert!(stage.exists());
    drop(lock);
    // Inject effective-root status without requiring privileged tests or changing uid.
    assert_eq!(
        recover_install_at(&layout, RECOVERY_WAIT, true).unwrap(),
        Recovery::Unchanged
    );
    assert!(stage.exists());
    fs::create_dir(layout.transaction()).unwrap();
    assert!(recover_install_at(&layout, RECOVERY_WAIT, true).is_err());
}

#[test]
fn rollback_cleanup_failure_preserves_restored_binary_result() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let layout = layout(temp.path());
    let before = contents(&layout);
    interrupt(&layout, 5, false);
    let result = recover_with_cleanup(&layout, |_| bail!("injected cleanup failure"));
    assert_eq!(result.unwrap(), Recovery::Restored(layout.binary.clone()));
    assert_eq!(contents(&layout), before);
    assert!(layout.transaction().join("committed").exists());
    assert_eq!(recover(&layout).unwrap(), Recovery::Unchanged);
    assert!(!layout.transaction().exists());
}

#[test]
fn sweep_failure_does_not_prevent_rollback_or_reexec() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let layout = layout(temp.path());
    let before = contents(&layout);
    interrupt(&layout, 5, false);
    let stage = layout
        .binary
        .parent()
        .unwrap()
        .join(".rex-update-unreadable");
    fs::create_dir(&stage).unwrap();
    fs::write(stage.join("partial"), b"partial").unwrap();
    fs::set_permissions(&stage, fs::Permissions::from_mode(0o000)).unwrap();
    assert!(sweep_staging(&layout).is_err());
    let result = recover_install_at(&layout, RECOVERY_WAIT, false);
    fs::set_permissions(&stage, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(result.unwrap(), Recovery::Restored(layout.binary.clone()));
    assert_eq!(contents(&layout), before);
}

#[test]
fn reused_inode_with_different_size_or_nanosecond_mtime_supersedes_transaction() {
    for size_changed in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let layout = layout(temp.path());
        interrupt(&layout, 5, false);
        let recorded = identity(&layout.binary).unwrap();
        // Deterministically simulate reuse: retain dev/ino while changing either
        // size or only mtime's nanoseconds (not its seconds or file length).
        fs::write(
            &layout.binary,
            if size_changed {
                b"repair".as_slice()
            } else {
                b"NEW"
            },
        )
        .unwrap();
        let nanos = if size_changed {
            recorded.mtime_nsec
        } else {
            (recorded.mtime_nsec + 1) % 1_000_000_000
        };
        File::open(&layout.binary)
            .unwrap()
            .set_modified(UNIX_EPOCH + Duration::new(recorded.mtime as u64, nanos as u32))
            .unwrap();
        let foreign = identity(&layout.binary).unwrap();
        assert_eq!((foreign.dev, foreign.ino), (recorded.dev, recorded.ino));
        assert_ne!(foreign, recorded);
        let before = contents(&layout);
        assert_eq!(recover(&layout).unwrap(), Recovery::Superseded);
        assert_eq!(contents(&layout), before);
        assert!(!layout.transaction().exists());
    }
}

#[test]
fn recovery_waits_for_lock_times_out_and_sweeps_only_retired_stages() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let layout = layout(temp.path());
    let before = contents(&layout);
    interrupt(&layout, 5, false);
    let parent = layout.binary.parent().unwrap();
    let stage = parent.join(".rex-update-stale");
    let cleanup = parent.join(".rex-update-cleanup-stale");
    fs::create_dir(&stage).unwrap();
    fs::create_dir(&cleanup).unwrap();
    fs::write(stage.join("archive"), b"partial").unwrap();
    let lock = layout.lock().unwrap();
    let error = recover_install_at(&layout, Duration::from_millis(40), false).unwrap_err();
    assert!(format!("{error:#}").contains("timed out"));
    assert!(stage.exists());
    assert!(layout.transaction().exists());
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(80));
        drop(lock);
    });
    assert_eq!(
        recover_install_at(&layout, Duration::from_secs(2), false).unwrap(),
        Recovery::Restored(layout.binary.clone())
    );
    release.join().unwrap();
    assert_eq!(contents(&layout), before);
    assert!(!stage.exists() && !cleanup.exists());
    // Also sweep if a crash left a stage but never wrote a journal.
    fs::create_dir(&stage).unwrap();
    assert_eq!(
        recover_install_at(&layout, RECOVERY_WAIT, false).unwrap(),
        Recovery::Unchanged
    );
    assert!(!stage.exists());
}

#[test]
fn recover_current_install_uses_actual_executable_and_journal_paths() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let layout = layout(temp.path());
    let before = contents(&layout);
    interrupt(&layout, 5, true);
    let output = Command::new(&layout.binary)
        .args([
            "--ignored",
            "--exact",
            "update::installer::tests::recovery_subprocess",
            "--nocapture",
        ])
        .env("HOME", temp.path())
        .env("XDG_DATA_HOME", temp.path().join("changed-data"))
        .env("REX_RECOVERY_TEST", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("restored running binary"));
    assert_eq!(contents(&layout), before);
}

#[test]
#[ignore = "invoked only in an isolated HOME by recover_current_install test"]
fn recovery_subprocess() {
    if std::env::var_os("REX_RECOVERY_TEST").is_none() {
        return;
    }
    let binary = std::env::current_exe().unwrap();
    assert_eq!(
        recover_current_install().unwrap(),
        Recovery::Restored(binary)
    );
    println!("restored running binary");
}

#[test]
fn binary_version_has_deadline_and_glibc_is_checked_before_install() {
    let temp = tempfile::tempdir().unwrap();
    let binary = temp.path().join("hang");
    fs::write(&binary, b"#!/bin/sh\nwhile :; do :; done\n").unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
    let start = Instant::now();
    let error = binary_version(&binary, Duration::from_millis(60)).unwrap_err();
    assert!(error.to_string().contains("timed out"));
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(check_glibc("2.0").is_ok());
    assert!(
        check_glibc("999.0")
            .unwrap_err()
            .to_string()
            .contains("requires glibc 999.0")
    );
}

fn contents(layout: &InstallLayout) -> Vec<Option<Vec<u8>>> {
    layout
        .destinations()
        .iter()
        .map(|p| fs::read(p).ok())
        .collect()
}

#[test]
fn every_replacement_failure_rolls_back_all_files_including_receipt() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    for fail_at in 0..6 {
        let temp = tempfile::tempdir().unwrap();
        let layout = layout(temp.path());
        // A missing icon must remain missing after rollback, not become an orphan.
        fs::remove_file(&layout.destinations()[1]).unwrap();
        let old = contents(&layout);
        let result = commit(&layout, stage(&layout), |i| {
            ensure!(i != fail_at, "injected rename failure");
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(contents(&layout), old, "failure at {fail_at}");
        assert!(!layout.transaction().exists());
    }
}

#[test]
fn interrupted_transaction_is_recoverable_and_completed_one_is_kept() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let layout = layout(temp.path());
    let old = contents(&layout);
    let stage = stage(&layout);
    let destinations = layout.destinations();
    for (i, dest) in destinations.iter().enumerate() {
        fs::hard_link(dest, stage.path().join(format!("old-{i}"))).unwrap();
    }
    fs::write(
        stage.path().join("journal.json"),
        serde_json::to_vec(&Journal {
            binary: layout.binary.clone(),
            data: layout.data.clone(),
            existed: vec![true; 6],
            new_inodes: (0..6)
                .map(|i| identity(&stage.path().join(format!("new-{i}"))).unwrap())
                .collect(),
        })
        .unwrap(),
    )
    .unwrap();
    fs::rename(stage.path(), layout.transaction()).unwrap();
    for (i, dest) in destinations.iter().enumerate().take(5) {
        fs::rename(layout.transaction().join(format!("new-{i}")), dest).unwrap();
    }
    // Next launch may have a different XDG_DATA_HOME. Restore the recorded
    // destinations, not whatever the new launch's environment happens to name.
    let changed_environment = InstallLayout {
        data: temp.path().join("other-data"),
        ..layout.clone()
    };
    recover(&changed_environment).unwrap();
    assert!(!changed_environment.data.exists());
    assert_eq!(contents(&layout), old);
    recover(&layout).unwrap();
    commit(&layout, self::stage(&layout), |_| Ok(())).unwrap();
    assert!(
        contents(&layout)
            .iter()
            .all(|v| v.as_deref() == Some(b"new"))
    );
    // Cleanup after commit never tries to roll back, even if backups are gone.
    fs::create_dir(layout.transaction()).unwrap();
    fs::write(layout.transaction().join("committed"), b"").unwrap();
    recover(&layout).unwrap();
    assert!(
        contents(&layout)
            .iter()
            .all(|v| v.as_deref() == Some(b"new"))
    );
}

#[test]
fn receipt_version_hash_and_permissions_guard_installation() {
    let temp = tempfile::tempdir().unwrap();
    let layout = layout(temp.path());
    layout.validate_receipt("0.7.3").unwrap();
    assert!(layout.validate_receipt("0.7.2").is_err());
    fs::write(&layout.binary, b"tampered").unwrap();
    assert!(layout.validate_receipt("0.7.3").is_err());
    fs::set_permissions(&layout.binary, fs::Permissions::from_mode(0o555)).unwrap();
    assert!(layout.validate_paths().is_err());
}

/// Real loopback HTTP manifest + test-key Minisign + tarball + desktop-file-utils +
/// atomic install, without changing HOME, production keys, or the user's installation.
#[test]
fn signed_fixture_end_to_end() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let layout = layout(temp.path());
    let target = format!("{}-linux-gnu", std::env::consts::ARCH);
    let root = format!("rex-linux-99.0.0-{target}");
    let binary = b"#!/bin/sh\necho 'rex-linux 99.0.0'\n";
    let desktop = include_bytes!("../../../resources/dev.shuv.rex-linux.desktop");
    let mut names = vec![
        format!("{root}/bin/rex-linux"),
        format!("{root}/resources/{DESKTOP}"),
    ];
    for size in [32, 128, 256] {
        names.push(format!(
            "{root}/resources/icons/{size}x{size}/apps/dev.shuv.rex-linux.png"
        ));
    }
    let mut files = vec![
        (
            names[0].as_str(),
            binary.as_slice(),
            tar::EntryType::Regular,
        ),
        (
            names[1].as_str(),
            desktop.as_slice(),
            tar::EntryType::Regular,
        ),
    ];
    let icons: [&[u8]; 3] = [
        include_bytes!("../../../resources/icons/32x32/apps/dev.shuv.rex-linux.png"),
        include_bytes!("../../../resources/icons/128x128/apps/dev.shuv.rex-linux.png"),
        include_bytes!("../../../resources/icons/256x256/apps/dev.shuv.rex-linux.png"),
    ];
    for (name, icon) in names[2..].iter().zip(icons) {
        files.push((name.as_str(), icon, tar::EntryType::Regular));
    }
    let long_path = format!("{root}/docs/{}.txt", "release-notes-".repeat(12));
    files.push((&long_path, b"PAX release notes", tar::EntryType::Regular));
    let tarball = archive(&files);
    let mut manifest: ReleaseManifest = serde_json::from_slice(
        &super::super::tests::sample_manifest_json("99.0.0", "0.7.0"),
    )
    .unwrap();
    let artifact = &mut manifest.targets.get_mut(&target).unwrap().tarball;
    artifact.size = tarball.len() as u64;
    artifact.sha256 = hex_sha256(&tarball);
    let bytes = serde_json::to_vec(&manifest).unwrap();
    let (id, pair, key) = super::super::tests::test_keypair();
    let sig = super::super::tests::sign_minisig(
        &id,
        &pair,
        &bytes,
        "rex-linux v99.0.0 latest.json",
        true,
    );
    let (base, server) = serve(vec![bytes, sig.into_bytes(), tarball]);
    let config = CheckConfig {
        current_version: "0.7.3".into(),
        manifest_url: format!("{base}/latest.json"),
        signature_url: format!("{base}/latest.json.minisig"),
        public_keys: vec![key],
        state_path: temp.path().join("state.json"),
        allow_loopback_http: true,
    };
    let mut phases = Vec::new();
    install_with(
        &config,
        "99.0.0",
        &layout,
        &mut |p| phases.push(p),
        |artifact, path, progress| {
            // Only transport is substituted. The signed URL and all verification/install code are unchanged.
            assert!(artifact.url.starts_with(REPOSITORY));
            let mut response = ureq::get(format!("{base}/fixture.tar.gz")).call()?;
            copy_download(
                response.body_mut().as_reader(),
                path,
                artifact.size,
                progress,
            )
        },
    )
    .unwrap();
    server.join().unwrap();
    assert_eq!(fs::read(&layout.binary).unwrap(), binary);
    for (path, icon) in layout.destinations()[..3].iter().zip(icons) {
        assert_eq!(fs::read(path).unwrap(), icon);
    }
    layout.validate_receipt("99.0.0").unwrap();
    assert_eq!(
        fs::metadata(layout.receipt()).unwrap().mode() & 0o777,
        0o600
    );
    assert_eq!(fs::metadata(&layout.binary).unwrap().mode() & 0o777, 0o755);
    let installed_desktop = fs::read_to_string(&layout.destinations()[3]).unwrap();
    assert!(installed_desktop.contains(&format!("TryExec={}", layout.binary.display())));
    assert!(matches!(phases.last(), Some(Progress::Installing)));
    assert_eq!(
        phases
            .iter()
            .filter(|p| matches!(p, Progress::Verifying))
            .count(),
        2
    );
    assert!(!layout.transaction().exists());
    assert_eq!(
        Command::new(&layout.binary)
            .arg("--version")
            .output()
            .unwrap()
            .stdout,
        b"rex-linux 99.0.0\n"
    );
}
