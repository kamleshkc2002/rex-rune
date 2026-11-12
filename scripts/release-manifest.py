#!/usr/bin/env python3
"""Generate latest.json from both architectures' packages and SHA256SUMS.

Only stable X.Y.Z releases are supported. No network access or signing keys.

Two names appear because the client was rex-linux through 0.7.x and those clients still
read this file. `product` stays "rex-linux", which their updater requires. Each target's
`tarball` is the bridge archive laid out for that updater (scripts/package-release.py),
and `archive` is the rex-rune tarball that current clients and install.sh install.
"""

import argparse
from datetime import datetime
import hashlib
import json
from pathlib import Path
import re
from zoneinfo import ZoneInfo


RELEASE_URL = "https://github.com/shuv1337/rex-rune/releases"
TARGETS = {"x86_64-linux-gnu": "amd64", "aarch64-linux-gnu": "arm64"}
# The manifest's product id, and the name of the bridge tarball: see the module docstring.
PRODUCT = "rex-linux"


def version_tuple(value):
    if not re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", value):
        raise ValueError(f"expected a stable X.Y.Z version, got {value!r}")
    return tuple(map(int, value.split(".")))


def generate(dist, version, update_from, published, min_glibc="2.39"):
    if version_tuple(update_from) > version_tuple(version):
        raise ValueError("update_from must not be newer than version")
    if not re.fullmatch(r"[0-9]+\.[0-9]+", min_glibc):
        raise ValueError("min_glibc must be MAJOR.MINOR")
    if not re.fullmatch(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(Z|[+-]\d{2}:\d{2})", published):
        raise ValueError("published must be an RFC 3339 timestamp with a timezone")
    datetime.fromisoformat(published.replace("Z", "+00:00"))
    checksums = {}
    for line in (dist / "SHA256SUMS").read_text().splitlines():
        match = re.fullmatch(r"([0-9a-f]{64}) [ *](\S+)", line)
        if not match or match[2] in checksums:
            raise ValueError("malformed or duplicate SHA256SUMS entry")
        checksums[match[2]] = match[1]
    def names(target, deb_arch):
        return {
            "archive": f"rex-rune-{version}-{target}.tar.gz",
            "tarball": f"{PRODUCT}-{version}-{target}.tar.gz",
            "deb": f"rex-rune_{version}_{deb_arch}.deb",
            "pacman": f"rex-rune-bin-{version}-1-{target.split('-linux-')[0]}.pkg.tar.zst",
        }

    expected = {name for target, deb_arch in TARGETS.items() for name in names(target, deb_arch).values()}
    if set(checksums) != expected:
        raise ValueError("SHA256SUMS must list exactly the eight packages for this version")

    def package(name):
        path = dist / name
        with path.open("rb") as source:
            digest = hashlib.file_digest(source, "sha256").hexdigest()
        if digest != checksums[name]:
            raise ValueError(f"SHA256SUMS mismatch: {name}")
        size = path.stat().st_size
        if size == 0:
            raise ValueError(f"empty package: {name}")
        return {"size": size, "sha256": digest}

    tag = f"v{version}"
    targets = {}
    for target, deb_arch in TARGETS.items():
        files = names(target, deb_arch)
        targets[target] = {
            "min_glibc": min_glibc,
            "tarball": {"url": f"{RELEASE_URL}/download/{tag}/{files['tarball']}", **package(files["tarball"])},
            "archive": {"url": f"{RELEASE_URL}/download/{tag}/{files['archive']}", **package(files["archive"])},
            "deb": {"name": files["deb"], **package(files["deb"])},
            "pacman": {"name": files["pacman"], **package(files["pacman"])},
        }
    return {
        "schema": 1,
        "product": PRODUCT,
        "version": version,
        "tag": tag,
        "published": published,
        "notes_url": f"{RELEASE_URL}/tag/{tag}",
        "update_from": update_from,
        "targets": targets,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dist", type=Path, default=Path("dist"))
    parser.add_argument("--version", required=True)
    parser.add_argument("--update-from", required=True)
    parser.add_argument("--min-glibc", default="2.39")
    parser.add_argument("--published", default=datetime.now(ZoneInfo("America/Los_Angeles")).isoformat(timespec="seconds"))
    args = parser.parse_args()
    try:
        manifest = generate(args.dist, args.version, args.update_from, args.published, args.min_glibc)
        (args.dist / "latest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    except (OSError, ValueError) as error:
        parser.exit(1, f"release manifest: {error}\n")


if __name__ == "__main__":
    main()
