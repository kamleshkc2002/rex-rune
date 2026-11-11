#!/usr/bin/env python3
"""Exercise the real desktop installer against a temporary home, never a server."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
with tempfile.TemporaryDirectory(prefix='rex-receipt-') as temp:
    home = Path(temp) / 'home with "quotes" and \\slashes'
    home.mkdir()
    binary = Path(temp) / 'rex-linux'
    binary.write_text('#!/bin/sh\necho rex-linux 99.0.0\n'); binary.chmod(0o755)
    env = {**os.environ, 'HOME': str(home), 'XDG_BIN_HOME': str(home / 'bin'),
           'XDG_DATA_HOME': str(home / 'data')}
    command = ['bash', str(ROOT / 'scripts/install-desktop.sh')]
    receipt = home / 'data/rex-linux/install.json'
    installed = home / 'bin/rex-linux'
    for _ in range(2):
        res = subprocess.run([*command, '--copy', '--receipt-version', '99.0.0', str(binary)], env=env, check=True, text=True, capture_output=True)
        assert f'Installed the Rex Linux desktop client (rex-linux 99.0.0) at {installed}' in res.stdout
        assert 'This does not install or change the Rex server (rex).' in res.stdout
        assert json.loads(receipt.read_text()) == {
            'schema': 1, 'path': str(installed.resolve()), 'version': '99.0.0',
            'sha256': hashlib.sha256(installed.read_bytes()).hexdigest(),
        }
        assert receipt.stat().st_mode & 0o777 == 0o600
    fake_rex = home / 'bin/rex'
    fake_rex.write_text('#!/bin/sh\necho rex\n'); fake_rex.chmod(0o755)
    res = subprocess.run([*command, '--copy', str(binary)], env={**env, 'PATH': f'{home / "bin"}:{env["PATH"]}'}, check=True, text=True, capture_output=True)
    assert f'Using your Rex server at {fake_rex}.' in res.stdout
    fake_rex.unlink()
    assert not receipt.exists(), 'source install must remove receipt'
    subprocess.run([*command, '--copy', '--receipt-version', '99.0.0', str(binary)], env=env, check=True)
    subprocess.run([*command, '--uninstall'], env=env, check=True)
    assert not receipt.exists() and not installed.exists()
print('Receipt path, version, hash, JSON escaping, source install and uninstall passed')
