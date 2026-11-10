#!/usr/bin/env python3
"""Stateless Pages repository from the newest five signed immutable releases.

Requires gh, minisign, gpg, Docker (repo-add). No private key enters a container.
--bootstrap publishes only the certificate and an honest not-yet-available page.
--releases-dir is an offline test fixture: releases.json plus vX.Y.Z directories.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time

from package_signing import Signer

ROOT = Path(__file__).resolve().parent.parent
REPO = 'shuv1337/rex-rune'
spec = importlib.util.spec_from_file_location('arch', ROOT / 'scripts/package-arch.py')
arch = importlib.util.module_from_spec(spec)
spec.loader.exec_module(arch)
ARCHES = arch.ARCHES


def version(tag):
    if not re.fullmatch(r'v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)', tag):
        raise ValueError(f'not a stable tag: {tag!r}')
    return tuple(map(int, tag[1:].split('.')))


def required_assets(tag):
    names = {f'rex-linux-bin-{tag[1:]}-1-{a}.pkg.tar.zst{suffix}'
             for a in ARCHES for suffix in ('', '.sig')}
    return names | {'SHA256SUMS', 'SHA256SUMS.minisig'}


def select(releases, exclude, require_tag=None):
    candidates = []
    for release in releases:
        tag = release['tag_name']
        if release['draft'] or release['prerelease'] or tag.removeprefix('v') in exclude:
            continue
        try:
            number = version(tag)
        except ValueError:
            continue
        names = {asset['name'] for asset in release['assets']}
        # Older releases have no pacman package: do not retrofit immutable assets.
        if not any(name.endswith('.pkg.tar.zst') for name in names):
            continue
        candidates.append((number, tag, names, bool(release.get('immutable'))))
    candidates.sort(reverse=True)
    if len({tag for _, tag, _, _ in candidates}) != len(candidates):
        raise ValueError('duplicate release tag')
    # Keep the newest five first: a broken old release must drop out of the window
    # eventually instead of blocking every future rebuild. Inside the window it fails.
    selected = []
    for number, tag, names, immutable in candidates[:5]:
        required = required_assets(tag)
        if not required <= names or not immutable:
            raise ValueError(f'{tag}: incomplete or mutable package release')
        selected.append((number, tag, required))
    if require_tag and require_tag not in [tag for _, tag, _ in selected]:
        raise ValueError(f'{require_tag} is not among the publishable releases yet')
    return selected


def verify_release(directory, tag, keys, gpg_home):
    sums = directory / 'SHA256SUMS'
    for key in ('release.pub', 'release-backup.pub'):
        result = subprocess.run(['minisign', '-Vm', str(sums), '-p', str(keys / key), '-q'],
                                capture_output=True)
        if result.returncode == 0:
            break
    else:
        raise ValueError(f'{tag}: untrusted checksum signature')
    lines = (directory / 'SHA256SUMS.minisig').read_text().splitlines()
    if len(lines) != 4 or lines[2] != f'trusted comment: rex-linux {tag} SHA256SUMS':
        raise ValueError(f'{tag}: signed checksums belong to another tag')
    checksums = {}
    for line in sums.read_text().splitlines():
        match = re.fullmatch(r'([0-9a-f]{64})  ([-a-zA-Z0-9_.]+)', line)
        if not match or match[2] in checksums:
            raise ValueError(f'{tag}: malformed or duplicate checksum')
        checksums[match[2]] = match[1]
    v = tag[1:]
    expected = {f'rex-linux-{v}-{a}-linux-gnu.tar.gz' for a in ARCHES}
    expected |= {f'rex-linux_{v}_{a}.deb' for a in ('amd64', 'arm64')}
    expected |= {f'rex-linux-bin-{v}-1-{a}.pkg.tar.zst' for a in ARCHES}
    if set(checksums) != expected:
        raise ValueError(f'{tag}: expected exactly six package checksums')
    for a in ARCHES:
        package = directory / f'rex-linux-bin-{v}-1-{a}.pkg.tar.zst'
        with package.open('rb') as stream:
            digest = hashlib.file_digest(stream, 'sha256').hexdigest()
        if digest != checksums[package.name]:
            raise ValueError(f'{tag}: package checksum mismatch')
        subprocess.run(['gpg', '--homedir', str(gpg_home), '--batch', '--verify',
                        str(package) + '.sig', str(package)], check=True, capture_output=True)


def landing(site, certificate, tags):
    shutil.copy2(certificate, site / 'rex-linux-archive-key.asc')
    (site / 'CNAME').write_text('pkg.shuv.dev\n')
    (site / '.nojekyll').touch()
    (site / 'releases.json').write_text(json.dumps(tags, indent=2) + '\n')
    message = ('Available: ' + ', '.join(tags)) if tags else 'Repository pending the first pacman release. Do not add it to pacman.conf yet.'
    (site / 'index.html').write_text(f'''<!doctype html><html lang="en"><meta charset="utf-8">
<title>Rex Linux packages</title><h1>Rex Linux packages</h1><p>{message}</p>
<p><a href="rex-linux-archive-key.asc">OpenPGP public certificate</a></p>
<p><a href="https://github.com/{REPO}/blob/master/docs/auto-update-operator-walkthrough.md">Setup, fingerprints and recovery</a></p>
<p>Signed pacman repository: https://pkg.shuv.dev/arch/$arch. APT is not available yet.</p></html>\n''')


def list_releases(require_tag):
    # The release was published seconds ago; the API can lag behind.
    for attempt in range(6):
        pages = json.loads(subprocess.check_output([
            'gh', 'api', '--paginate', '--slurp', f'repos/{REPO}/releases?per_page=100']))
        releases = [release for page in pages for release in page]
        visible = [r for r in releases if r['tag_name'] == require_tag]
        if not require_tag or (visible and visible[0].get('immutable') and not visible[0]['draft']):
            return releases
        time.sleep(10 * (attempt + 1))
    return releases


def build(site, keys, exclude, releases_dir=None, bootstrap=False, require_tag=None):
    if site.exists():
        raise ValueError('site output must not already exist; never reuse a partial rebuild')
    site.mkdir(parents=True)
    certificate = keys / 'rex-linux-archive-key.asc'
    if bootstrap:
        landing(site, certificate, [])
        return
    # Import/remove secret env before gh or Docker is started.
    with Signer(certificate) as signer, tempfile.TemporaryDirectory(prefix='rex-repo-') as temp:
        work = Path(temp)
        public_home = work / 'public'; public_home.mkdir(mode=0o700)
        subprocess.run(['gpg', '--homedir', str(public_home), '--batch', '--import', str(certificate)],
                       check=True, capture_output=True)
        if releases_dir:
            releases = json.loads((releases_dir / 'releases.json').read_text())
        else:
            releases = list_releases(require_tag)
        selected = select(releases, exclude, require_tag)
        if not selected:
            raise ValueError('no signed pacman releases; bootstrap explicitly instead')
        for _, tag, required in selected:
            directory = work / tag; directory.mkdir()
            for name in sorted(required):
                if releases_dir:
                    shutil.copy2(releases_dir / tag / name, directory / name)
                else:
                    # Fixed repo + validated tag + fixed names, never asset-provided URLs.
                    subprocess.run(['gh', 'release', 'download', tag, '--repo', REPO,
                                    '--pattern', name, '--dir', str(directory)], check=True)
            verify_release(directory, tag, keys, public_home)
            for a in ARCHES:
                target = site / 'arch' / a; target.mkdir(parents=True, exist_ok=True)
                for suffix in ('', '.sig'):
                    name = f'rex-linux-bin-{tag[1:]}-1-{a}.pkg.tar.zst{suffix}'
                    shutil.copy2(directory / name, target / name)
        newest = selected[0][1][1:]
        for a in ARCHES:
            target = (site / 'arch' / a).resolve()
            subprocess.run(['docker', 'run', '--rm', '-v', f'{target}:/repo', '-w', '/repo',
                            arch.ARCH_IMAGE, 'repo-add', '--include-sigs', 'rex-linux.db.tar.gz',
                            f'rex-linux-bin-{newest}-1-{a}.pkg.tar.zst'], check=True)
            # Pages artifacts forbid symlinks. Canonical .db/.files and compressed names
            # are identical bytes, with detached signatures for both URL spellings.
            for name in ('rex-linux.db', 'rex-linux.files'):
                path = target / name
                if not path.is_symlink():
                    raise ValueError('repo-add did not produce its expected alias')
                data = path.read_bytes(); path.unlink(); path.write_bytes(data)
                signer.sign(path)
                shutil.copy2(str(path) + '.sig', str(path) + '.tar.gz.sig')
        landing(site, certificate, [tag for _, tag, _ in selected])


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--site', type=Path, default=Path('site'))
    parser.add_argument('--keys-dir', type=Path, default=Path('packaging/keys'))
    parser.add_argument('--exclude', default='', help='comma-separated X.Y.Z versions to withdraw')
    parser.add_argument('--releases-dir', type=Path)
    parser.add_argument('--bootstrap', action='store_true')
    parser.add_argument('--require-tag', help='fail unless this vX.Y.Z tag enters the site')
    args = parser.parse_args()
    if args.require_tag:
        version(args.require_tag)
    excluded = set(filter(None, args.exclude.split(',')))
    for item in excluded:
        version('v' + item)
    build(args.site, args.keys_dir, excluded, args.releases_dir, args.bootstrap, args.require_tag)
