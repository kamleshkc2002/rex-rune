#!/usr/bin/env python3
"""Package an already-built native release; requires cargo and (for --deb) dpkg-dev.

Run from any directory: python3 scripts/package-release.py [--deb]
No server connections or changes to the user's installation.
"""

import argparse
import gzip
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tarfile
import tempfile
import tomllib


def run(*args, **kwargs):
    return subprocess.check_output(args, text=True, **kwargs).strip()


def bridge(temp, stage, binary, version, arch):
    """The same release laid out as the 0.7.x updater expects it, for that updater alone.

    Before the rename the client was rex-linux, and its updater installs only a tarball
    whose root is rex-linux-<version>-<target>, holding bin/rex-linux (which must print
    "rex-linux <version>"), resources/dev.shuv.rex-linux.desktop and the three icons under
    that name. It takes nothing else. The binary is this release's; started as rex-linux it
    reports that name, and at its first start it moves the installation to the rex-rune
    names (update::installer::migrate_legacy_install). Nothing but that updater should
    install from this archive. A copy of the 0.7.x install.sh that someone kept would pick
    it by name and run its install.sh, so that script only says where to go instead.
    """
    tree = temp / f"rex-linux-{version}-{arch}-linux-gnu"
    (tree / "bin").mkdir(parents=True)
    shutil.copy2(binary, tree / "bin/rex-linux")
    actual = run(str(tree / "bin/rex-linux"), "--version")
    if actual != f"rex-linux {version}":
        raise SystemExit(f"The 0.7.x updater needs 'rex-linux {version}' from bin/rex-linux, got {actual!r}")
    resources = tree / "resources"
    resources.mkdir()
    launcher = (stage / "resources/dev.shuv.rex-rune.desktop").read_text().splitlines(keepends=True)
    # Until the first start replaces it: the old command and the icons that updater installs.
    (resources / "dev.shuv.rex-linux.desktop").write_text("".join(
        line.replace("rex-rune", "rex-linux") if line.startswith(("Exec=", "TryExec=", "Icon=")) else line
        for line in launcher
    ))
    for size in (32, 128, 256):
        icons = resources / f"icons/{size}x{size}/apps"
        icons.mkdir(parents=True)
        shutil.copy2(stage / f"resources/icons/{size}x{size}/apps/dev.shuv.rex-rune.png", icons / "dev.shuv.rex-linux.png")
    for filename in ("LICENSE", "BUILD-INFO"):
        shutil.copy2(stage / filename, tree / filename)
    shutil.copytree(stage / "licenses", tree / "licenses")
    (tree / "install.sh").write_text(
        "#!/bin/sh\n"
        "# Run by an install.sh kept from Rex Linux 0.7.x, which picks this archive by its name.\n"
        "cat >&2 <<'EOF'\n"
        "Rex Linux is now Rex Rune, and this archive is only for the updater inside Rex Linux 0.7.x.\n"
        "Nothing was installed. Use the current installer instead:\n"
        "  curl --proto '=https' --tlsv1.2 -fsSL https://github.com/shuv1337/rex-rune/releases/latest/download/install.sh | sh\n"
        "It replaces a rex-linux install, keeps your settings and leaves rex-linux as a link to rex-rune.\n"
        "EOF\n"
        "exit 1\n"
    )
    (tree / "install.sh").chmod(0o755)
    (tree / "README").write_text(
        f"Rex Rune {version}, packaged for the updater built into Rex Linux 0.7.x.\n\n"
        "Rex Linux was renamed Rex Rune. This archive keeps the old layout only so that\n"
        "updater can install this release; the client then moves itself to the new names.\n"
        f"To install by hand, use rex-rune-{version}-{arch}-linux-gnu.tar.gz or install.sh.\n"
    )
    return tree


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--deb", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    version = tomllib.loads((root / "Cargo.toml").read_text())["package"]["version"]
    arch, deb_arch = {"x86_64": ("x86_64", "amd64"), "aarch64": ("aarch64", "arm64")}[platform.machine()]
    binary = root / "target/release/rex-rune"
    actual = run(str(binary), "--version")
    if actual != f"rex-rune {version}":
        raise SystemExit(f"Rebuild before packaging: {actual!r}, expected rex-rune {version}")
    dist = root / "dist"
    dist.mkdir(exist_ok=True)
    name = f"rex-rune-{version}-{arch}-linux-gnu"
    epoch = int(os.environ.get("SOURCE_DATE_EPOCH", run("git", "show", "-s", "--format=%ct", cwd=root)))
    metadata = json.loads(run("cargo", "metadata", "--locked", "--format-version=1", cwd=root))
    with tempfile.TemporaryDirectory(prefix="rex-package-", dir=dist) as temp:
        stage = Path(temp) / name
        (stage / "bin").mkdir(parents=True)
        shutil.copy2(binary, stage / "bin/rex-rune")
        shutil.copytree(root / "resources", stage / "resources")
        (stage / "scripts").mkdir()
        shutil.copy2(root / "scripts/install-desktop.sh", stage / "scripts/install-desktop.sh")
        for filename in ("LICENSE", "README.md", "CHANGELOG.md"):
            shutil.copy2(root / filename, stage / filename)
        (stage / "install.sh").write_text(
            '#!/usr/bin/env bash\nset -euo pipefail\n'
            'root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"\n'
            f'exec bash "$root/scripts/install-desktop.sh" --copy --receipt-version {version} "$root/bin/rex-rune"\n'
        )
        (stage / "install.sh").chmod(0o755)
        licenses = stage / "licenses"
        licenses.mkdir()
        index = []
        for package in sorted(metadata["packages"], key=lambda p: p["id"]):
            source = Path(package["manifest_path"]).parent
            dest = licenses / f'{package["name"]}-{package["version"]}'
            dest.mkdir(exist_ok=True)
            candidates = set()
            for pattern in ("LICENSE*", "LICENCE*", "COPYING*", "NOTICE*", "*LICENSE*", "licenses/*"):
                candidates.update(source.glob(pattern))
            if package.get("license_file"):
                candidates.add(source / package["license_file"])
            for path in sorted(candidates):
                if path.is_file():
                    shutil.copy2(path, dest / path.name)
            index.append({"name": package["name"], "version": package["version"], "license": package["license"], "source": package["source"]})
        shutil.copytree(root / "vendor/rune/crates/rune/assets", licenses / "rune-assets", ignore=shutil.ignore_patterns("*.ttf", "*.otf", "*.png", "*.svg"))
        shutil.copy2(root / "vendor/rune/LICENSE", licenses / "Rune-LICENSE")
        # Ghostty is fetched by the build script, rather than appearing in Cargo metadata.
        shutil.copy2(root / "resources/icons/README.md", licenses / "ICON-PROVENANCE.md")
        (licenses / "dependencies.json").write_text(json.dumps(index, indent=2) + "\n")
        shutil.copy2(root / "vendor/ghostty-vt/LICENSE-GHOSTTY", licenses / "Ghostty-LICENSE")
        (stage / "BUILD-INFO").write_text(
            f"version={version}\ncommit={run('git', 'rev-parse', 'HEAD', cwd=root)}\n"
            f"architecture={arch}\n{run('rustc', '--version')}\nzig={run('zig', 'version')}\n"
        )

        def normalize(info):
            info.uid = info.gid = 0
            info.uname = info.gname = "root"
            info.mtime = epoch
            return info

        def write_archive(tree, name):
            archive = dist / f"{name}.tar.gz"
            # gzip's timestamp is fixed too; do not encode a temporary filename.
            with archive.open("wb") as output:
                with gzip.GzipFile(filename="", mode="wb", fileobj=output, mtime=epoch) as compressed:
                    with tarfile.open(fileobj=compressed, mode="w") as tar:
                        tar.add(tree, arcname=name, filter=normalize)
            print(archive)

        write_archive(stage, name)
        write_archive(bridge(Path(temp), stage, binary, version, arch), f"rex-linux-{version}-{arch}-linux-gnu")
        if args.deb:
            deb = Path(temp) / "deb"
            (deb / "usr/bin").mkdir(parents=True)
            shutil.copy2(binary, deb / "usr/bin/rex-rune")
            # The old command keeps working for scripts and key bindings.
            (deb / "usr/bin/rex-linux").symlink_to("rex-rune")
            (deb / "usr/share/rex-rune").mkdir(parents=True)
            (deb / "usr/share/rex-rune/channel").write_text("channel=deb\nhint=Install the latest .deb from GitHub Releases\n")
            (deb / "usr/share/applications").mkdir(parents=True)
            shutil.copy2(root / "resources/dev.shuv.rex-rune.desktop", deb / "usr/share/applications")
            for size in (32, 128, 256):
                icons = deb / f"usr/share/icons/hicolor/{size}x{size}/apps"
                icons.mkdir(parents=True)
                shutil.copy2(root / f"resources/icons/{size}x{size}/apps/dev.shuv.rex-rune.png", icons)
            doc = deb / "usr/share/doc/rex-rune"
            shutil.copytree(licenses, doc / "licenses")
            for filename in ("LICENSE", "README.md", "CHANGELOG.md", "BUILD-INFO"):
                shutil.copy2(stage / filename, doc / filename)
            (deb / "DEBIAN").mkdir()
            (deb / "debian").mkdir()
            (deb / "debian/control").write_text("Source: rex-rune\n\nPackage: rex-rune\nArchitecture: any\nDescription: Rex client\n")
            # Resolve linked-library minimum versions on the actual build distro.
            shlibs = run("dpkg-shlibdeps", "--ignore-missing-info", "-O", f"-e{binary}", cwd=deb)
            shutil.rmtree(deb / "debian")
            depends = shlibs.removeprefix("shlibs:Depends=")
            depends += ", libfontconfig1, libwayland-client0, libwayland-cursor0, libwayland-egl1, libx11-6, libx11-xcb1, libxcb1, libxcb-render0, libxcb-shape0, libxcb-xfixes0, libxkbcommon-x11-0, libegl1, libgl1, libvulkan1, fontconfig, ca-certificates, xdg-utils"
            (deb / "DEBIAN/control").write_text(
                f"Package: rex-rune\nVersion: {version}\nArchitecture: {deb_arch}\n"
                "Maintainer: shuv1337 <shuv@shuv.dev>\nSection: utils\nPriority: optional\n"
                f"Installed-Size: {sum(p.stat().st_size for p in (deb / 'usr').rglob('*') if p.is_file() and not p.is_symlink()) // 1024}\n"
                f"Depends: {depends}\n"
                # Takes over from the package this was before the rename, in one transaction.
                "Replaces: rex-linux\nConflicts: rex-linux\nProvides: rex-linux\n"
                "Homepage: https://github.com/shuv1337/rex-rune\n"
                "Description: Native Linux client for existing Rex terminal servers\n"
                " Unofficial Rune-based client. Requires a Rex server with API revisions 10–13.\n"
            )
            package = dist / f"rex-rune_{version}_{deb_arch}.deb"
            run("dpkg-deb", "--root-owner-group", "--build", str(deb), str(package), env={**os.environ, "SOURCE_DATE_EPOCH": str(epoch)})
            print(package)


if __name__ == "__main__":
    main()
