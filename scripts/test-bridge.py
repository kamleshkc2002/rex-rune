#!/usr/bin/env python3
"""Check a bridge tarball against what the updater inside Rex Linux 0.7.x accepts.

The client was rex-linux through 0.7.x. The updater in those installed clients cannot
change, and it is how an install.sh install reaches the renamed release. Following
src/update/installer.rs at v0.7.5, it installs a release only when:

- every tar entry is a regular file or a directory under rex-linux-<version>-<target>,
  with no absolute path, no "." or ".." component and no duplicate;
- bin/rex-linux runs and prints exactly "rex-linux <version>";
- resources/dev.shuv.rex-linux.desktop passes desktop-file-install with Exec and TryExec
  set, then desktop-file-validate;
- the 32, 128 and 256 pixel icons exist as resources/icons/<n>x<n>/apps/dev.shuv.rex-linux.png.

It copies those six files and nothing else. This runs the same checks, and with --binary
also that bin/rex-linux is byte for byte the release's rex-rune. The archive's install.sh
is for a kept copy of the 0.7.x installer script, which would run it: it must install
nothing, fail, and name the current installer.

Run: python3 scripts/test-bridge.py dist/rex-linux-X.Y.Z-<target>.tar.gz [--binary target/release/rex-rune]
Requires desktop-file-utils and a machine that can run the binary. Changes nothing outside
a temporary directory.
"""
import argparse
from pathlib import Path
import re
import subprocess
import tarfile
import tempfile


def check(archive, binary=None):
    match = re.fullmatch(r"(rex-linux-((?:0|[1-9][0-9]*)(?:\.(?:0|[1-9][0-9]*)){2})-(?:x86_64|aarch64)-linux-gnu)\.tar\.gz", archive.name)
    assert match, f"not a bridge tarball name: {archive.name}"
    root, version = match[1], match[2]
    with tempfile.TemporaryDirectory(prefix="rex-bridge-test-") as temp, tarfile.open(archive) as tar:
        seen = set()
        for entry in tar:
            name = entry.name.rstrip("/")
            assert entry.isfile() or entry.isdir(), f"{name}: links and special files are refused"
            parts = name.split("/")
            assert not name.startswith("/") and all(part not in ("", ".", "..") for part in parts), f"{name}: unsafe path"
            assert parts[0] == root, f"{name}: outside {root}"
            assert name not in seen, f"{name}: duplicate"
            seen.add(name)
        assert len(seen) < 20_000
        tar.extractall(temp, filter="data")
        package = Path(temp) / root
        old = package / "bin/rex-linux"
        old.chmod(0o755)
        printed = subprocess.run([str(old), "--version"], capture_output=True, text=True, timeout=10)
        assert printed.returncode == 0 and printed.stdout.strip() == f"rex-linux {version}", (
            f"bin/rex-linux --version printed {printed.stdout!r}; the 0.7.x updater needs 'rex-linux {version}'")
        for size in (32, 128, 256):
            icon = package / f"resources/icons/{size}x{size}/apps/dev.shuv.rex-linux.png"
            assert icon.is_file() and icon.stat().st_size > 0, f"missing {icon.relative_to(package)}"
        installed = Path(temp) / "home/.local/bin/rex-linux"
        applications = Path(temp) / "applications"
        subprocess.run([
            "desktop-file-install", f"--dir={applications}",
            "--set-key=Exec", f'--set-value="{installed}"', "--set-key=TryExec", f"--set-value={installed}",
            str(package / "resources/dev.shuv.rex-linux.desktop"),
        ], check=True)
        launcher = applications / "dev.shuv.rex-linux.desktop"
        subprocess.run(["desktop-file-validate", str(launcher)], check=True)
        text = launcher.read_text()
        # Until the client's first start replaces it, this launcher starts the installed file
        # and shows the icon that updater installs.
        assert f"TryExec={installed}\n" in text and "Icon=dev.shuv.rex-linux\n" in text, text
        assert not (package / "scripts").exists(), "the bridge carries no installer"
        refusal = subprocess.run(["bash", str(package / "install.sh")], capture_output=True, text=True,
                                 env={"HOME": str(Path(temp) / "home"), "PATH": "/usr/bin:/bin"})
        assert refusal.returncode != 0 and "releases/latest/download/install.sh" in refusal.stderr, (
            "install.sh in the bridge must refuse and point at the current installer")
        assert not (Path(temp) / "home").exists(), "the bridge's install.sh wrote something"
        assert (package / "LICENSE").is_file() and (package / "licenses").is_dir() and (package / "BUILD-INFO").is_file()
        if binary:
            assert old.read_bytes() == binary.read_bytes(), "bin/rex-linux is not this release's binary"
    print(f"PASS {archive.name}: the 0.7.x updater's layout, version line, launcher and icons")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("archive", type=Path, nargs="+")
    parser.add_argument("--binary", type=Path)
    args = parser.parse_args()
    for archive in args.archive:
        check(archive, args.binary)
