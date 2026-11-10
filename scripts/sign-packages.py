#!/usr/bin/env python3
"""Sign both pacman release assets inside the protected release job."""
import argparse
from pathlib import Path
from package_signing import Signer

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--dist', type=Path, default=Path('dist'))
parser.add_argument('--certificate', type=Path, default=Path('packaging/keys/rex-linux-archive-key.asc'))
args = parser.parse_args()
packages = sorted(args.dist.glob('*.pkg.tar.zst'))
if len(packages) != 2:
    parser.error('expected exactly two pacman packages')
with Signer(args.certificate) as signer:
    for path in packages:
        signer.sign(path)
print('Verified both OpenPGP package signatures')
