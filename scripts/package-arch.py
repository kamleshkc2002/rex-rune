#!/usr/bin/env python3
"""Repackage both native tarballs with real makepkg; no secrets or server access.

Requires Docker. The x86_64 container also repackages aarch64: no cross-compilation
or emulation, and the executable bytes are compared, never run on the wrong CPU.
"""
import argparse
import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parent.parent
ARCH_IMAGE = 'archlinux:latest@sha256:d4594bb2300cba11f41c2f27923eb56f63d9988909c402f9c95517d1e362c899'
ARCHES = ('x86_64', 'aarch64')


def package(dist, version):
    if not re.fullmatch(r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)', version):
        raise ValueError('expected stable X.Y.Z')
    dist = dist.resolve()
    recipe = (ROOT / 'packaging/arch/PKGBUILD.in').read_text().replace('@VERSION@', version)
    for arch in ARCHES:
        source = dist / f'rex-linux-{version}-{arch}-linux-gnu.tar.gz'
        with source.open('rb') as stream:
            digest = hashlib.file_digest(stream, 'sha256').hexdigest()
        recipe = recipe.replace(f'@SHA256_{arch.upper()}@', digest)
    # Output the concrete, checksummed recipe for inspection. Not an AUR push.
    (dist / 'PKGBUILD').write_text(recipe)
    for arch in ARCHES:
        with tempfile.TemporaryDirectory(prefix='rex-makepkg-') as tmp:
            stage = Path(tmp)
            stage.chmod(0o755)
            (stage / 'PKGBUILD').write_text(recipe)
            source = dist / f'rex-linux-{version}-{arch}-linux-gnu.tar.gz'
            shutil.copy2(source, stage / source.name)
            subprocess.run([
                'docker', 'run', '--rm', '--platform', 'linux/amd64',
                '-v', f'{stage}:/build', '-w', '/build', ARCH_IMAGE,
                'bash', '-euc', '''
                    pacman -Syu --noconfirm --needed base-devel
                    useradd -m builder
                    chown -R builder:builder /build
                    cp /etc/makepkg.conf /build/makepkg.conf
                    printf '\nCARCH=%s\nPKGEXT=.pkg.tar.zst\n' "$1" >> /build/makepkg.conf
                    runuser -u builder -- makepkg --config /build/makepkg.conf --nodeps --cleanbuild
                    chown -R "$2:$3" /build
                ''', 'bash', arch, str(os.getuid()), str(os.getgid()),
            ], check=True)
            name = f'rex-linux-bin-{version}-1-{arch}.pkg.tar.zst'
            built = stage / name
            packaged = subprocess.check_output(['tar', '--zstd', '-xOf', str(built), 'usr/bin/rex-linux'])
            with tarfile.open(source) as archive:
                original = archive.extractfile(f'rex-linux-{version}-{arch}-linux-gnu/bin/rex-linux').read()
            if packaged != original:
                raise ValueError(f'{arch}: makepkg changed the binary')
            shutil.copy2(built, dist / name)
            print(dist / name)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dist', type=Path, default=Path('dist'))
    parser.add_argument('--version', required=True)
    args = parser.parse_args()
    package(args.dist, args.version)
