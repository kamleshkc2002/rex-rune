#!/usr/bin/env python3
"""Exercise the real desktop installer against a temporary home, never a server."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
with tempfile.TemporaryDirectory(prefix='rex-receipt-') as temp:
    home = Path(temp) / 'home with "quotes" and \\slashes'
    home.mkdir()
    binary = Path(temp) / 'rex-rune'
    binary.write_text('#!/bin/sh\necho rex-rune 99.0.0\n'); binary.chmod(0o755)
    env = {**os.environ, 'HOME': str(home), 'XDG_BIN_HOME': str(home / 'bin'),
           'XDG_DATA_HOME': str(home / 'data')}
    command = ['bash', str(ROOT / 'scripts/install-desktop.sh')]
    receipt = home / 'data/rex-rune/install.json'
    installed = home / 'bin/rex-rune'
    desktop = home / 'data/applications/dev.shuv.rex-rune.desktop'
    rex_on_path = shutil.which('rex', path=env.get('PATH'))
    expected_server_line = (
        f'Using your Rex server at {rex_on_path}.'
        if rex_on_path
        else 'rex-rune needs a Rex server installed separately; see https://github.com/shuv1337/rex-rune#build-requirements.'
    )
    for _ in range(2):
        res = subprocess.run([*command, '--copy', '--receipt-version', '99.0.0', str(binary)], env=env, check=True, text=True, capture_output=True)
        assert res.stdout.count(f'Installed the Rex Rune desktop client (rex-rune 99.0.0) at {installed} and launcher {desktop}.') == 1
        assert res.stdout.count('This does not install or change the Rex server (rex).') == 1
        assert res.stdout.count(expected_server_line) == 1
        assert json.loads(receipt.read_text()) == {
            'schema': 1, 'path': str(installed.resolve()), 'version': '99.0.0',
            'sha256': hashlib.sha256(installed.read_bytes()).hexdigest(),
        }
        assert receipt.stat().st_mode & 0o777 == 0o600
    fake_rex = home / 'bin/rex'
    fake_rex.write_text('#!/bin/sh\necho rex\n'); fake_rex.chmod(0o755)
    res = subprocess.run([*command, '--copy', str(binary)], env={**env, 'PATH': f'{home / "bin"}:{env["PATH"]}'}, check=True, text=True, capture_output=True)
    assert res.stdout.count(f'Installed the Rex Rune desktop client (rex-rune 99.0.0) at {installed} and launcher {desktop}.') == 1
    assert res.stdout.count('This does not install or change the Rex server (rex).') == 1
    assert res.stdout.count(f'Using your Rex server at {fake_rex}.') == 1
    fake_rex.unlink()
    assert not receipt.exists(), 'source install must remove receipt'
    quiet = subprocess.run([*command, '--copy', '--receipt-version', '99.0.0', str(binary)], env={**env, 'REX_INSTALL_QUIET_SUMMARY': '1'}, check=True, text=True, capture_output=True)
    assert 'Installed the Rex Rune desktop' not in quiet.stdout
    assert 'This does not install or change the Rex server (rex).' not in quiet.stdout
    assert 'Rex server' not in quiet.stdout
    subprocess.run([*command, '--uninstall'], env=env, check=True)
    assert not receipt.exists() and not installed.exists()

    # An install made before the rename from rex-linux: its binary, launcher, icons and
    # receipt under the old names. Installing over it leaves one launcher and one set of
    # icons, keeps the old command as a link, and never touches preferences.
    legacy_binary = home / 'bin/rex-linux'
    legacy_binary.write_text('#!/bin/sh\necho rex-linux 98.0.0\n'); legacy_binary.chmod(0o755)
    legacy_desktop = home / 'data/applications/dev.shuv.rex-linux.desktop'
    legacy_desktop.write_text('[Desktop Entry]\nType=Application\nName=Rex Linux\nExec=rex-linux\n')
    legacy_receipt = home / 'data/rex-linux/install.json'
    legacy_receipt.parent.mkdir(parents=True)
    legacy_receipt.write_text('{"schema":1}')
    legacy_icons = [home / f'data/icons/hicolor/{size}x{size}/apps/dev.shuv.rex-linux.png' for size in (32, 128, 256)]
    for icon in legacy_icons:
        icon.parent.mkdir(parents=True, exist_ok=True)
        icon.write_bytes(b'old icon')
    preferences = home / 'config/rex-linux/workspace.json'
    preferences.parent.mkdir(parents=True)
    preferences.write_text('{"keep":true}')
    subprocess.run([*command, '--copy', '--receipt-version', '99.0.0', str(binary)], env=env, check=True, capture_output=True)
    assert installed.is_file() and not installed.is_symlink()
    assert legacy_binary.is_symlink() and os.readlink(legacy_binary) == 'rex-rune'
    assert subprocess.check_output([str(legacy_binary), '--version'], text=True) == 'rex-rune 99.0.0\n'
    assert desktop.exists() and not legacy_desktop.exists(), 'exactly one launcher'
    assert not any(icon.exists() for icon in legacy_icons)
    assert receipt.exists() and not legacy_receipt.parent.exists()
    assert preferences.read_text() == '{"keep":true}'
    launchers = sorted(path.name for path in (home / 'data/applications').glob('*.desktop'))
    assert launchers == ['dev.shuv.rex-rune.desktop'], launchers
    # A fresh install makes no old-name link, and uninstall removes the link with the rest.
    subprocess.run([*command, '--uninstall'], env=env, check=True)
    assert not installed.exists() and not legacy_binary.is_symlink() and not legacy_binary.exists()
    subprocess.run([*command, '--copy', str(binary)], env=env, check=True, capture_output=True)
    assert installed.is_file() and not legacy_binary.exists() and not legacy_binary.is_symlink()
    subprocess.run([*command, '--uninstall'], env=env, check=True)
    assert preferences.read_text() == '{"keep":true}'
print('Receipt path, version, hash, JSON escaping, source install, uninstall and the rex-linux takeover passed')
