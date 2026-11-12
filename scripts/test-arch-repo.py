#!/usr/bin/env python3
"""Real makepkg/repo-add/pacman install, upgrade and tamper tests in archlinux:latest.

The oldest fixture release is the package as it was before the rename, rex-linux-bin, so
the consumer starts where existing installs are and pacman carries it to rex-rune-bin.

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


# packaging/arch/PKGBUILD.in as released in 0.7.5, the last rex-linux-bin.
LEGACY_RECIPE = r'''pkgname=rex-linux-bin
pkgver=@VERSION@
pkgrel=1
pkgdesc='Native Linux client for existing Rex terminal servers'
arch=('x86_64' 'aarch64')
url='https://github.com/shuv1337/rex-rune'
license=('MIT')
provides=('rex-linux')
conflicts=('rex-linux')
depends=('glibc>=2.39' 'gcc-libs' 'fontconfig' 'libxkbcommon' 'libxkbcommon-x11'
         'wayland' 'libx11' 'libxcb' 'libglvnd' 'vulkan-icd-loader'
         'xdg-utils' 'ca-certificates' 'hicolor-icon-theme')
options=('!strip' '!debug')
source_x86_64=("https://github.com/shuv1337/rex-rune/releases/download/v${pkgver}/rex-linux-${pkgver}-x86_64-linux-gnu.tar.gz")
source_aarch64=("https://github.com/shuv1337/rex-rune/releases/download/v${pkgver}/rex-linux-${pkgver}-aarch64-linux-gnu.tar.gz")
sha256sums_x86_64=('@SHA256_X86_64@')
sha256sums_aarch64=('@SHA256_AARCH64@')

package() {
    cd "rex-linux-${pkgver}-${CARCH}-linux-gnu"
    install -Dm755 bin/rex-linux "$pkgdir/usr/bin/rex-linux"
    install -Dm644 resources/dev.shuv.rex-linux.desktop "$pkgdir/usr/share/applications/dev.shuv.rex-linux.desktop"
    for size in 32 128 256; do
        install -Dm644 "resources/icons/${size}x${size}/apps/dev.shuv.rex-linux.png" \
            "$pkgdir/usr/share/icons/hicolor/${size}x${size}/apps/dev.shuv.rex-linux.png"
    done
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
    cp -r licenses "$pkgdir/usr/share/licenses/$pkgname/dependencies"
    install -Dm644 BUILD-INFO "$pkgdir/usr/share/doc/rex-linux/BUILD-INFO"
    install -dm755 "$pkgdir/usr/share/rex-linux"
    printf 'channel=pacman\nhint=sudo pacman -Syu\n' > "$pkgdir/usr/share/rex-linux/channel"
}
'''


def fixture(dist, version, client='rex-rune'):
    """A release's packages: under the old name it has six, since the rename eight."""
    dist.mkdir()
    for a in arch.ARCHES:
        name = f'{client}-{version}-{a}-linux-gnu'
        stage = dist / name; (stage / 'bin').mkdir(parents=True)
        binary = stage / f'bin/{client}'
        # Like the client, the stub names itself as it was started.
        binary.write_text(f'#!/bin/sh\necho "${{0##*/}} {version}"\n'); binary.chmod(0o755)
        shutil.copytree(ROOT / 'resources', stage / 'resources')
        if client == 'rex-linux':
            for path in sorted((stage / 'resources').rglob('*rex-rune*')):
                path.rename(path.with_name(path.name.replace('rex-rune', 'rex-linux')))
        shutil.copy2(ROOT / 'LICENSE', stage)
        (stage / 'licenses').mkdir(); (stage / 'licenses/fixture').touch()
        (stage / 'BUILD-INFO').write_text(f'version={version}\n')
        with tarfile.open(dist / (name + '.tar.gz'), 'w:gz') as tar:
            tar.add(stage, arcname=name)
        shutil.rmtree(stage)
        if client != 'rex-linux':
            (dist / f'rex-linux-{version}-{a}-linux-gnu.tar.gz').write_text('fixture bridge tarball\n')
    for a in ('amd64', 'arm64'):
        (dist / f'{client}_{version}_{a}.deb').write_text('fixture deb\n')
    arch.package(dist, version, LEGACY_RECIPE if client == 'rex-linux' else None, client)


def release(tag, immutable=True, complete=True, prerelease=False, pacman=True, base='rex-rune-bin'):
    assets = [{'name': 'SHA256SUMS'}, {'name': 'SHA256SUMS.minisig'}]
    if pacman:
        for a in arch.ARCHES:
            assets.append({'name': f'{base}-{tag[1:]}-1-{a}.pkg.tar.zst'})
            if complete:
                assets.append({'name': f'{base}-{tag[1:]}-1-{a}.pkg.tar.zst.sig'})
    return {'tag_name': tag, 'draft': False, 'prerelease': prerelease, 'immutable': immutable, 'assets': assets}


class SelectionTests(unittest.TestCase):
    def tags(self, releases, exclude=(), require_tag=None):
        return [tag for _, tag, _ in site_builder.select(releases, set(exclude), require_tag)]

    def test_newest_five_numeric_order_only_pacman_releases(self):
        releases = [release(f'v0.{n}.0') for n in (5, 12, 7, 9, 6, 10, 11)] + [release('v0.4.2', pacman=False)]
        self.assertEqual(self.tags(releases), ['v0.12.0', 'v0.11.0', 'v0.10.0', 'v0.9.0', 'v0.7.0'])

    def test_broken_release_fails_inside_window_and_ages_out(self):
        broken = release('v0.5.3', complete=False)
        self.assertRaises(ValueError, self.tags, [broken, release('v0.5.4')])
        self.assertRaises(ValueError, self.tags, [release('v0.5.3', immutable=False), release('v0.5.4')])
        newer = [release(f'v0.6.{n}') for n in range(5)]
        self.assertEqual(len(self.tags(newer + [broken])), 5)

    def test_releases_from_before_and_after_the_rename_share_one_window(self):
        releases = [release('v0.7.4', base='rex-linux-bin'), release('v0.7.5', base='rex-linux-bin'), release('v0.8.0')]
        selected = site_builder.select(releases, set())
        self.assertEqual([tag for _, tag, _ in selected], ['v0.8.0', 'v0.7.5', 'v0.7.4'])
        self.assertIn('rex-rune-bin-0.8.0-1-x86_64.pkg.tar.zst.sig', selected[0][2])
        self.assertIn('rex-linux-bin-0.7.5-1-aarch64.pkg.tar.zst', selected[1][2])
        # A release's signed checksums name exactly its own generation of packages.
        self.assertEqual(len(site_builder.release_packages('0.7.5', 'rex-linux-bin')), 6)
        after = site_builder.release_packages('0.8.0', 'rex-rune-bin')
        self.assertEqual(len(after), 8)
        self.assertIn('rex-linux-0.8.0-x86_64-linux-gnu.tar.gz', after, 'the 0.7.x updater bridge')
        self.assertNotIn('rex-linux_0.8.0_amd64.deb', after)
        # The old package name with half its files is still an incomplete release.
        broken = release('v0.7.5', complete=False, base='rex-linux-bin')
        self.assertRaises(ValueError, self.tags, [broken, release('v0.8.0')])

    def test_exclude_prerelease_require_tag(self):
        releases = [release('v0.5.3'), release('v0.5.4'), release('v0.5.5', prerelease=True), release('v0.5.6-rc1')]
        self.assertEqual(self.tags(releases), ['v0.5.4', 'v0.5.3'])
        self.assertEqual(self.tags(releases, exclude=['0.5.4']), ['v0.5.3'])
        self.assertEqual(self.tags(releases, require_tag='v0.5.4'), ['v0.5.4', 'v0.5.3'])
        self.assertRaises(ValueError, self.tags, releases, [], 'v0.5.5')
        self.assertRaises(ValueError, self.tags, releases, ['0.5.4'], 'v0.5.4')
        self.assertRaises(ValueError, self.tags, releases + [release('v0.5.4')])


def main():
    unittest.main(module=__name__, argv=['selection'], exit=False, verbosity=1)
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
        (keys / site_builder.CERTIFICATE).write_bytes(run(*gpg, '--armor', '--export', primary, env=env))
        secret = run(*gpg, '--armor', '--export-secret-subkeys', primary, env=env).decode()
        sign_env = {**os.environ, 'PACKAGE_SIGNING_KEY': secret, 'PACKAGE_SIGNING_PASSPHRASE': password}
        run('minisign', '-G', '-W', '-p', str(keys / 'release.pub'), '-s', str(root / 'minisign.key'))
        shutil.copy2(keys / 'release.pub', keys / 'release-backup.pub')
        releases = root / 'releases'; releases.mkdir()
        metadata = []
        try:
            # 99.0.0 is the last release under the old name; the two after it are renamed.
            for v, client in (('99.0.0', 'rex-linux'), ('99.0.1', 'rex-rune'), ('99.0.2', 'rex-rune')):
                dist = releases / ('v' + v)
                fixture(dist, v, client)
                subprocess.run(['python3', str(SCRIPTS / 'sign-packages.py'), '--dist', str(dist),
                                '--certificate', str(keys / site_builder.CERTIFICATE)], env=sign_env, check=True)
                packages = sorted(p for p in dist.iterdir() if p.name.endswith(('.tar.gz', '.deb', '.pkg.tar.zst')))
                (dist / 'SHA256SUMS').write_text(''.join(f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n' for p in packages))
                # The signing product id is the same before and after the rename.
                run('minisign', '-S', '-s', str(root / 'minisign.key'), '-m', str(dist / 'SHA256SUMS'), '-t', f'rex-linux v{v} SHA256SUMS')
                if client == 'rex-rune':
                    # Exercise the eight-package manifest, too.
                    manifest = load('release-manifest').generate(dist, v, '0.4.2', '2026-11-10T12:00:00-08:00')
                    assert all(entry['pacman']['name'].startswith('rex-rune-bin-') for entry in manifest['targets'].values())
                    assert all(entry['tarball']['url'].rsplit('/', 1)[1].startswith('rex-linux-') for entry in manifest['targets'].values())
                metadata.append({'tag_name': 'v' + v, 'draft': False, 'prerelease': False, 'immutable': True,
                                 'assets': [{'name': p.name} for p in dist.iterdir()]})
            (releases / 'releases.json').write_text(json.dumps(metadata))
            for label, exclude in (('old', '99.0.1,99.0.2'), ('new', '99.0.2'), ('newer', '')):
                subprocess.run(['python3', str(SCRIPTS / 'build-repo-site.py'), '--site', str(root / label),
                                '--keys-dir', str(keys), '--releases-dir', str(releases), '--exclude', exclude], env=sign_env, check=True)
            # Invalid upstream content must fail BEFORE creating any database.
            damaged = releases / 'v99.0.2/rex-rune-bin-99.0.2-1-x86_64.pkg.tar.zst'
            original = damaged.read_bytes(); damaged.write_bytes(original + b'tamper')
            result = subprocess.run(['python3', str(SCRIPTS / 'build-repo-site.py'), '--site', str(root / 'bad'),
                                     '--keys-dir', str(keys), '--releases-dir', str(releases)], env=sign_env, capture_output=True)
            assert result.returncode != 0 and not list((root / 'bad').rglob('*.db'))
            damaged.write_bytes(original)
            # A tampered detached signature alone, with a wrong-tag checksum comment, both refuse.
            for path, mutate in (
                (damaged.with_name(damaged.name + '.sig'), lambda b: b[:-4] + bytes(x ^ 1 for x in b[-4:])),
                (releases / 'v99.0.2/SHA256SUMS.minisig', lambda b: b.replace(b'v99.0.2 SHA256SUMS', b'v99.0.0 SHA256SUMS')),
            ):
                keep = path.read_bytes(); path.write_bytes(mutate(keep))
                result = subprocess.run(['python3', str(SCRIPTS / 'build-repo-site.py'), '--site', str(root / ('bad-' + path.name)),
                                         '--keys-dir', str(keys), '--releases-dir', str(releases)], env=sign_env, capture_output=True)
                assert result.returncode != 0, path.name
                path.write_bytes(keep)
            # Do not mount the signing home or any secret into the consumer container.
            consumer = root / 'consumer'; consumer.mkdir()
            for name in ('old', 'new', 'newer'):
                shutil.copytree(root / name, consumer / name)
            # The site keeps every release in its window, whatever the package was called.
            for name in ('rex-linux-bin-99.0.0-1-x86_64.pkg.tar.zst', 'rex-rune-bin-99.0.1-1-x86_64.pkg.tar.zst',
                         'rex-rune-bin-99.0.2-1-x86_64.pkg.tar.zst', 'rex-linux.db', 'rex-linux.db.sig'):
                assert (consumer / 'newer/arch/x86_64' / name).is_file(), name
            assert not list((consumer / 'newer').rglob('rex-rune.db*')), 'the repository is still [rex-linux]'
            shutil.copy2(keys / site_builder.CERTIFICATE, consumer / 'key.asc')
            for name in ('bad-package', 'bad-database', 'bad-signature'):
                shutil.copytree(consumer / 'newer', consumer / name)
            sig = consumer / 'bad-signature/arch/x86_64/rex-rune-bin-99.0.2-1-x86_64.pkg.tar.zst.sig'
            sig_bytes = bytearray(sig.read_bytes()); sig_bytes[-1] ^= 1; sig.write_bytes(sig_bytes)
            bad_package = consumer / 'bad-package/arch/x86_64/rex-rune-bin-99.0.2-1-x86_64.pkg.tar.zst'
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
                # The repository section is the one existing installs have: [rex-linux].
                use_repo() {
                    cp /tmp/base.conf /etc/pacman.conf
                    printf '\n[rex-linux]\nSigLevel = Required DatabaseRequired\nServer = file:///fixtures/%s/arch/$arch\n' "$1" >> /etc/pacman.conf
                    rm -f /var/lib/pacman/sync/rex-linux.db*
                }
                old_install() {
                    use_repo old
                    pacman -Syu --noconfirm rex-linux-bin
                    test "$(rex-linux --version)" = 'rex-linux 99.0.0'
                    grep -qx channel=pacman /usr/share/rex-linux/channel
                    pacman -Qi rex-linux-bin | grep -E 'Validated By.*Signature'
                    test -e /usr/share/applications/dev.shuv.rex-linux.desktop
                }
                # One package, one launcher, one set of icons, and the old command as a link.
                renamed() {
                    test "$(rex-rune --version)" = "rex-rune $1"
                    test "$(pacman -Qq rex-rune-bin)" = rex-rune-bin
                    # By exact name: `pacman -Q rex-linux-bin` also answers for what provides it.
                    if pacman -Qq | grep -qx rex-linux-bin; then echo 'rex-linux-bin is still installed'; exit 1; fi
                    test "$(pacman -Qq rex-linux-bin)" = rex-rune-bin
                    pacman -Qi rex-rune-bin | grep -E 'Validated By.*Signature'
                    test "$(readlink /usr/bin/rex-linux)" = rex-rune
                    test "$(rex-linux --version)" = "rex-linux $1"
                    test "$(pacman -Qqo /usr/bin/rex-linux)" = rex-rune-bin
                    grep -qx channel=pacman /usr/share/rex-rune/channel
                    test "$(ls /usr/share/applications | grep -c rex)" = 1
                    test -e /usr/share/applications/dev.shuv.rex-rune.desktop
                    test ! -e /usr/share/rex-linux
                    test -z "$(find /usr/share/icons -name 'dev.shuv.rex-linux*')"
                    test "$(find /usr/share/icons -name 'dev.shuv.rex-rune.png' | wc -l)" = 3
                }
                # A plain system upgrade, as `omarchy update` runs it, replaces the package.
                old_install
                use_repo new
                pacman -Syu --noconfirm
                renamed 99.0.1
                # Asking for the old name afterwards finds its replacement, not an error.
                pacman -S --noconfirm --needed rex-linux-bin
                renamed 99.0.1
                use_repo newer
                pacman -Syu --noconfirm
                renamed 99.0.2
                # The command the 0.7.x client shows for a pacman install does the same.
                pacman -R --noconfirm rex-rune-bin
                test ! -e /usr/bin/rex-linux
                rm -f /var/cache/pacman/pkg/rex-*
                old_install
                use_repo newer
                pacman -Syu --noconfirm rex-linux-bin
                renamed 99.0.2
                # A first install by either name gets the renamed package.
                pacman -R --noconfirm rex-rune-bin
                pacman -S --noconfirm rex-linux-bin
                renamed 99.0.2
                # Remove package/cache so corruption cannot be masked by a cache hit.
                pacman -R --noconfirm rex-rune-bin
                rm -f /var/cache/pacman/pkg/rex-rune-bin-*
                use_repo bad-package
                pacman -Sy --noconfirm
                if pacman -S --noconfirm rex-rune-bin > /tmp/refusal 2>&1; then exit 1; fi
                grep -Ei 'invalid|corrupt|signature|checksum' /tmp/refusal
                # Isolate signature rejection as well, not only the database checksum.
                if pacman-key --verify /fixtures/bad-package/arch/x86_64/rex-rune-bin-99.0.2-1-x86_64.pkg.tar.zst.sig > /tmp/refusal 2>&1; then exit 1; fi
                grep -i 'BAD signature' /tmp/refusal
                # Intact package, altered detached signature: pacman -U reads the .sig beside it.
                if pacman -U --noconfirm /fixtures/bad-signature/arch/x86_64/rex-rune-bin-99.0.2-1-x86_64.pkg.tar.zst > /tmp/refusal 2>&1; then exit 1; fi
                grep -Ei 'signature|invalid|corrupt' /tmp/refusal
                test -z "$(pacman -Qq rex-rune-bin 2>/dev/null)"
                use_repo bad-database
                if pacman -Sy --noconfirm > /tmp/refusal 2>&1; then exit 1; fi
                grep -Ei 'signature|invalid|corrupt' /tmp/refusal
                echo 'PASS: pacman install, rex-linux-bin to rex-rune-bin by three routes, upgrade, package tamper, signature tamper, database tamper'
            ''', 'bash', primary], check=True)
        finally:
            subprocess.run(['gpgconf', '--homedir', str(home), '--kill', 'gpg-agent'])
    print('PASS: both makepkg architectures, signed rebuild, manifest, pacman consumer')


if __name__ == '__main__':
    main()
