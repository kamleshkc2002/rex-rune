#!/usr/bin/env python3
"""Sign release metadata, then verify against the committed minisign public keys.

Secrets come only from RELEASE_MINISIGN_KEY and RELEASE_MINISIGN_PASSWORD.
--verify-only supports offline/manual signing without changing the upload path.
"""

import argparse
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile


FILES = ("latest.json", "SHA256SUMS", "install.sh")
spec = importlib.util.spec_from_file_location("manifest", Path(__file__).with_name("release-manifest.py"))
manifest = importlib.util.module_from_spec(spec)
spec.loader.exec_module(manifest)


def verify(dist, keys, tag):
    source = Path(__file__).resolve().parent.parent / "packaging/install/install.sh"
    if (dist / "install.sh").read_bytes() != source.read_bytes():
        raise ValueError("install.sh differs from the reviewed source")
    for name in FILES:
        for key in keys:
            result = subprocess.run(
                ["minisign", "-Vm", str(dist / name), "-p", str(key), "-q"],
                capture_output=True,
            )
            if result.returncode == 0:
                break
        else:
            raise ValueError(f"{name}: signature did not verify against either repository public key")
        # Minisign has authenticated the comment; also bind it to this tag and file.
        lines = (dist / f"{name}.minisig").read_text().splitlines()
        if len(lines) != 4 or lines[2] != f"trusted comment: rex-linux {tag} {name}":
            raise ValueError(f"{name}: trusted comment must be exactly 'rex-linux {tag} {name}'")

    value = json.loads((dist / "latest.json").read_text())
    try:
        expected = manifest.generate(
            dist, tag[1:], value["update_from"], value["published"],
            value["targets"]["x86_64-linux-gnu"]["min_glibc"],
        )
    except (KeyError, TypeError) as error:
        raise ValueError("latest.json: invalid manifest schema") from error
    if value != expected:
        raise ValueError("latest.json does not match the tag, packages and SHA256SUMS")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dist", type=Path, default=Path("dist"))
    parser.add_argument("--keys-dir", type=Path, default=Path("packaging/keys"))
    parser.add_argument("--tag", required=True)
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args()
    # Do not pass these environment variables to minisign or any other child.
    secret = os.environ.pop("RELEASE_MINISIGN_KEY", "")
    password = os.environ.pop("RELEASE_MINISIGN_PASSWORD", "")
    try:
        if not re.fullmatch(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", args.tag):
            raise ValueError("tag must be a stable vX.Y.Z release")
        if not args.verify_only and (not secret or not password):
            message = "RELEASE_MINISIGN_KEY and RELEASE_MINISIGN_PASSWORD must both be set in the release environment"
            raise ValueError(f"refusing unsigned release: {message}")
        keys = [args.keys_dir / name for name in ("release.pub", "release-backup.pub")]
        for key in keys:
            if not key.is_file():
                raise ValueError(f"missing public key {key}; complete docs/releases.md setup before releasing")
        if not args.verify_only:
            # Private directory and file disappear on success or any exception.
            with tempfile.TemporaryDirectory(prefix="rex-release-sign-") as temp:
                key = Path(temp) / "release.key"
                key.touch(mode=0o600)
                key.write_text(secret)
                for name in FILES:
                    result = subprocess.run(
                        ["minisign", "-S", "-s", str(key), "-m", str(args.dist / name),
                         "-t", f"rex-linux {args.tag} {name}", "-q"],
                        input=password + "\n", text=True, capture_output=True,
                    )
                    if result.returncode != 0:
                        raise ValueError(f"minisign failed to sign {name}; check the release key and password")
        verify(args.dist, keys, args.tag)
        print("Verified signatures, trusted comments, and package/metadata consistency")
    except (OSError, ValueError) as error:
        parser.exit(1, f"release signing: {error}\n")


if __name__ == "__main__":
    main()
