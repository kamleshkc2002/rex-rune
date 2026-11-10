#!/usr/bin/env python3
"""Real makepkg/repo-add/pacman install, upgrade and tamper tests in archlinux:latest.

Only throwaway signing keys and stub executables, no production secrets, releases,
server sessions or host package installs. Requires Docker, gpg, minisign, zstd.
aarch64 is repackaged and inspected; the official Arch image is x86_64 only.
"""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent
SCRIPTS = ROOT / 'scripts'

def load(name):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / f'{name}.py')
    module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
    return module

arch = load('package-arch')
site_builder = load('build-repo-site')


def run(*args, **kw):
    return subprocess.check_output(args, **kw)


def fixture(dist, version):
    dist.mkdir()
    for a in arch.ARCHES:
        name = f'rex-linux-{version}-{a}-linux-gnu'
        stage = dist / name; (stage / 'bin').mkdir(parents=True)
        binary = stage / 'bin/rex-linux'
        binary.write_text(f'#!/bin/sh\necho rex-linux {version}\n'); binary.chmod(0o755)
        shutil.copytree(ROOT / 'resources', stage / 'resources')
        shutil.copy2(ROOT / 'LICENSE', stage)
        (stage / 'licenses').mkdir(); (stage / 'licenses/fixture').touch()
        (stage / 'BUILD-INFO').write_text(f'version={version}\n')
        with tarfile.open(dist / (name + '.tar.gz'), 'w:gz') as tar:
            tar.add(stage, arcname=name)
        shutil.rmtree(stage)
    for a in ('amd64', 'arm64'):
        (dist / f'rex-linux_{version}_{a}.deb').write_text('fixture deb\n')
    arch.package(dist, version)


def main():
    with tempfile.TemporaryDirectory(prefix='rex-arch-test-') as temp:
        root = Path(temp); root.chmod(0o755)
        keys = root / 'keys'; keys.mkdir()
        home = root / 'gpg'; home.mkdir(mode=0o700)
        env = {**os.environ, 'GNUPGHOME': str(home)}
        password = 'throwaway-package-test-password'
        gpg = ['gpg', '--batch', '--pinentry-mode', 'loopback', '--passphrase', password]
        run(*gpg, '--quick-generate-key', 'Rex fixture <fixture@example.invalid>', 'ed25519', 'cert', 'never', env=env)
        primary = next(l.split(':')[9] for l in run(*gpg, '--with-colons', '--list-keys', env=env).decode().splitlines() if l.startswith('fpr:'))
        run(*gpg, '--quick-add-key', primary, 'ed25519', 'sign', '3y', env=env)
        (keys / 'rex-linux-archive-key.asc').write_bytes(run(*gpg, '--armor', '--export', primary, env=env))
        secret = run(*gpg, '--armor', '--export-secret-subkeys', primary, env=env).decode()
        sign_env = {**os.environ, 'PACKAGE_SIGNING_KEY': secret, 'PACKAGE_SIGNING_PASSPHRASE': password}
        run('minisign', '-G', '-W', '-p', str(keys / 'release.pub'), '-s', str(root / 'minisign.key'))
        shutil.copy2(keys / 'release.pub', keys / 'release-backup.pub')
        releases = root / 'releases'; releases.mkdir()
        metadata = []
        try:
            for v in ('99.0.0', '99.0.1'):
                dist = releases / ('v' + v)
                fixture(dist, v)
                subprocess.run(['python3', str(SCRIPTS / 'sign-packages.py'), '--dist', str(dist),
                                '--certificate', str(keys / 'rex-linux-archive-key.asc')], env=sign_env, check=True)
                packages = sorted(p for p in dist.iterdir() if p.name.endswith(('.tar.gz', '.deb', '.pkg.tar.zst')))
                (dist / 'SHA256SUMS').write_text(''.join(f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n' for p in packages))
                run('minisign', '-S', '-s', str(root / 'minisign.key'), '-m', str(dist / 'SHA256SUMS'), '-t', f'rex-linux v{v} SHA256SUMS')
                # Exercise the six-package manifest, too.
                manifest = load('release-manifest').generate(dist, v, '0.4.2', '2026-11-10T12:00:00-08:00')
                assert all('pacman' in entry for entry in manifest['targets'].values())
                metadata.append({'tag_name': 'v' + v, 'draft': False, 'prerelease': False, 'immutable': True,
                                 'assets': [{'name': p.name} for p in dist.iterdir()]})
            (releases / 'releases.json').write_text(json.dumps(metadata))
            for label, exclude in (('old', '99.0.1'), ('new', '')):
                subprocess.run(['python3', str(SCRIPTS / 'build-repo-site.py'), '--site', str(root / label),
                                '--keys-dir', str(keys), '--releases-dir', str(releases), '--exclude', exclude], env=sign_env, check=True)
            # Invalid upstream content must fail BEFORE creating any database.
            damaged = releases / 'v99.0.1/rex-linux-bin-99.0.1-1-x86_64.pkg.tar.zst'
            original = damaged.read_bytes(); damaged.write_bytes(original + b'tamper')
            result = subprocess.run(['python3', str(SCRIPTS / 'build-repo-site.py'), '--site', str(root / 'bad'),
                                     '--keys-dir', str(keys), '--releases-dir', str(releases)], env=sign_env, capture_output=True)
            assert result.returncode != 0 and not list((root / 'bad').rglob('*.db'))
            damaged.write_bytes(original)
            # Do not mount the signing home or any secret into the consumer container.
            consumer = root / 'consumer'; consumer.mkdir()
            for name in ('old', 'new'):
                shutil.copytree(root / name, consumer / name)
            shutil.copy2(keys / 'rex-linux-archive-key.asc', consumer / 'key.asc')
            for name in ('bad-package', 'bad-database'):
                shutil.copytree(consumer / 'new', consumer / name)
            bad_package = consumer / 'bad-package/arch/x86_64/rex-linux-bin-99.0.1-1-x86_64.pkg.tar.zst'
            data = bytearray(bad_package.read_bytes()); data[len(data) // 2] ^= 1
            bad_package.write_bytes(data)
            # A valid database's changed bytes must fail its detached signature.
            with (consumer / 'bad-database/arch/x86_64/rex-linux.db').open('ab') as f: f.write(b'tamper')
            subprocess.run(['docker', 'run', '--rm', '-v', f'{consumer}:/fixtures:ro', arch.ARCH_IMAGE,
                            'bash', '-euc', '''
                pacman-key --init
                pacman-key --populate archlinux
                pacman-key --add /fixtures/key.asc
                pacman-key --lsign-key "$1"
                cp /etc/pacman.conf /tmp/base.conf
                use_repo() {
                    cp /tmp/base.conf /etc/pacman.conf
                    printf '\n[rex-linux]\nSigLevel = Required DatabaseRequired\nServer = file:///fixtures/%s/arch/$arch\n' "$1" >> /etc/pacman.conf
                    rm -f /var/lib/pacman/sync/rex-linux.db*
                }
                use_repo old
                pacman -Syu --noconfirm rex-linux-bin
                test "$(rex-linux --version)" = 'rex-linux 99.0.0'
                grep -qx channel=pacman /usr/share/rex-linux/channel
                pacman -Qi rex-linux-bin | grep -E 'Validated By.*Signature'
                use_repo new
                pacman -Syu --noconfirm
                test "$(rex-linux --version)" = 'rex-linux 99.0.1'
                # Remove package/cache so corruption cannot be masked by a cache hit.
                pacman -R --noconfirm rex-linux-bin
                rm -f /var/cache/pacman/pkg/rex-linux-bin-*
                use_repo bad-package
                pacman -Sy --noconfirm
                if pacman -S --noconfirm rex-linux-bin > /tmp/refusal 2>&1; then exit 1; fi
                grep -Ei 'invalid|corrupt|signature|checksum' /tmp/refusal
                # Isolate signature rejection as well, not only the database checksum.
                if pacman-key --verify /fixtures/bad-package/arch/x86_64/rex-linux-bin-99.0.1-1-x86_64.pkg.tar.zst.sig > /tmp/refusal 2>&1; then exit 1; fi
                grep -i 'BAD signature' /tmp/refusal
                use_repo bad-database
                if pacman -Sy --noconfirm > /tmp/refusal 2>&1; then exit 1; fi
                grep -Ei 'signature|invalid|corrupt' /tmp/refusal
                echo 'PASS: pacman install, upgrade, package tamper, database tamper'
            ''', 'bash', primary], check=True)
        finally:
            subprocess.run(['gpgconf', '--homedir', str(home), '--kill', 'gpg-agent'])
    print('PASS: both makepkg architectures, signed rebuild, manifest, pacman consumer')


if __name__ == '__main__':
    main()
