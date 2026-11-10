"""Isolated OpenPGP signing context. Only a protected job supplies these secrets."""
import os
from pathlib import Path
import subprocess
import tempfile
import time


class Signer:
    def __init__(self, certificate):
        # Remove secrets before starting any subprocess (including Docker).
        secret = os.environ.pop('PACKAGE_SIGNING_KEY', '')
        password = os.environ.pop('PACKAGE_SIGNING_PASSPHRASE', '')
        if not secret or not password:
            raise ValueError('PACKAGE_SIGNING_KEY and PACKAGE_SIGNING_PASSPHRASE required')
        self.temp = tempfile.TemporaryDirectory(prefix='rex-pgp-')
        self.home = Path(self.temp.name)
        self.password = password
        try:
            self.run('--import', input=Path(certificate).read_bytes())
            public = self.run('--with-colons', '--list-keys').decode().splitlines()
            primary = [line.split(':')[9] for line in public if line.startswith('fpr:')][0]
            self.run('--import', input=secret.encode())
            listing = self.run('--with-colons', '--with-subkey-fingerprint', '--list-secret-keys').decode().splitlines()
            keys = [line.split(':') for line in listing if line.startswith('sec:')]
            if len(keys) != 1 or keys[0][14] != '#' or ''.join(c for c in keys[0][11] if c.islower()) != 'c':
                raise ValueError('CI must contain exactly one stub certify-only primary, never its secret')
            fingerprints = [line.split(':')[9] for line in listing if line.startswith('fpr:')]
            if fingerprints[0] != primary:
                raise ValueError('CI key does not match committed certificate')
            self.subkeys = []
            for index, line in enumerate(listing):
                fields = line.split(':')
                if fields[0] != 'ssb' or fields[14] != '+':
                    continue
                if fields[1] in ('r', 'e', 'd') or fields[11] != 's':
                    raise ValueError('CI subkey must be valid and signing-only')
                if not fields[6] or int(fields[6]) - time.time() < 180 * 86400:
                    raise ValueError('Signing subkey expires in under 180 days; rotate offline')
                fpr = listing[index + 1].split(':')[9]
                if fpr not in [line.split(':')[9] for line in public if line.startswith('fpr:')]:
                    raise ValueError('Subkey not in committed public certificate')
                self.subkeys.append(fpr)
            if not self.subkeys:
                raise ValueError('No usable signing subkey')
        except BaseException:
            self.close()
            raise

    def run(self, *args, input=None):
        result = subprocess.run(['gpg', '--homedir', str(self.home), '--batch', '--yes',
                                 '--pinentry-mode', 'loopback', *args],
                                input=input, capture_output=True)
        if result.returncode:
            # Never include import diagnostics or secret material in logs.
            raise ValueError('OpenPGP operation failed; check certificate, subkey and passphrase')
        return result.stdout

    def sign(self, path):
        selectors = [arg for key in self.subkeys for arg in ('--local-user', key + '!')]
        self.run('--passphrase-fd', '0', *selectors, '--digest-algo', 'SHA512',
                 '--output', str(path) + '.sig', '--detach-sign', str(path),
                 input=(self.password + '\n').encode())
        self.run('--verify', str(path) + '.sig', str(path))

    def close(self):
        subprocess.run(['gpgconf', '--homedir', str(self.home), '--kill', 'gpg-agent'],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.temp.cleanup()

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()
