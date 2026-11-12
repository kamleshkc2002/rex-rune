#!/usr/bin/env python3
"""Run ONLY in a disposable container via install-script-test.sh (writes /usr/bin).

Real 0.5.1 assets exercise the production keys and both verifiers. That release was
published as rex-linux, so the installer verifies it and then declines it. Local packages
are built by package-release.py with stub toolchains/binaries; the installer test
copy changes exactly the repository URL and the two key constants. A curl shim
allows HTTP solely for the loopback fixture, never in the production installer.
"""
import functools
import hashlib
import http.server
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import threading

ROOT = Path(__file__).resolve().parent.parent
SOURCE = (ROOT / "packaging/install/install.sh").read_text()
REPO = "https://github.com/shuv1337/rex-rune"


def run(args, **kwargs):
    return subprocess.run(args, text=True, capture_output=True, **kwargs)


def checked(args, **kwargs):
    p = run(args, **kwargs)
    assert p.returncode == 0, (args, p.stdout, p.stderr)
    return p.stdout.strip()


def executable(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    path.chmod(0o755)


class Quiet(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass


def main():
    assert Path("/.dockerenv").exists(), "run only in Docker via install-script-test.sh"
    with tempfile.TemporaryDirectory(prefix="rex-installer-test-") as temp:
        root = Path(temp)
        home = root / "home"
        home.mkdir()
        # A PATH with every normal tool except minisign lets us test OpenSSL
        # without a production verifier override. The other PATH adds minisign.
        tools = root / "tools"
        tools.mkdir()
        for directory in ("/usr/bin", "/bin"):
            for path in Path(directory).iterdir():
                if path.is_file() and path.name != "minisign" and not (tools / path.name).exists():
                    (tools / path.name).symlink_to(path)
        mini = root / "mini"
        mini.mkdir()
        (mini / "minisign").symlink_to(shutil.which("minisign"))
        env = {k: v for k, v in os.environ.items() if not k.startswith("XDG_")}
        env.update(HOME=str(home), PATH=str(tools))
        installed = home / ".local/bin/rex-rune"
        desktop = home / ".local/share/applications/dev.shuv.rex-rune.desktop"
        prod = root / "install.sh"
        prod.write_text(SOURCE)

        def invoke(script=prod, args=(), ok=True, contains=None, shell="sh", mini_sign=False):
            e = dict(env)
            if mini_sign:
                e["PATH"] = f"{mini}:{tools}"
            command = [shutil.which(shell)] if shell != "busybox" else [shutil.which("busybox"), "sh"]
            p = run([*command, str(script), *args], env=e)
            assert (p.returncode == 0) == ok, (command, args, p.returncode, p.stdout, p.stderr)
            if contains:
                assert contains in p.stdout + p.stderr, (contains, p.stdout, p.stderr)
            return p

        libc = run(["getconf", "GNU_LIBC_VERSION"])
        compatible = libc.returncode == 0 and tuple(map(int, libc.stdout.split()[-1].split("."))) >= (2, 39)
        if not compatible:
            invoke(args=("--version", "0.5.1"), ok=False, contains="glibc")
            assert not installed.exists()
            print("PASS real installer refuses old glibc / musl", flush=True)
            # Exercise the exact production verifier on OpenSSL 3.0 even where
            # running the binary is unsupported. --check never executes it.
            (tools / "getconf").unlink(missing_ok=True)
            executable(tools / "getconf", "#!/bin/sh\necho 'glibc 2.39'\n")
        # The production signature and its rex-linux trusted comment verify with both
        # verifiers: only a verified release reaches the message about its old name.
        declined = f"0.5.1 was published as rex-linux; install it with its own installer: curl -fsSL {REPO}/releases/download/v0.5.1/install.sh | sh -s -- --version 0.5.1"
        for verifier in (False, True):
            for args in (("--version", "0.5.1"), ("--version", "0.5.1", "--check")):
                invoke(args=args, ok=False, mini_sign=verifier, contains=declined)
            assert not installed.exists() and not desktop.exists()
        print("PASS real v0.5.1 signature with minisign and OpenSSL; pre-rename release declined", flush=True)

        site = root / "site"
        site.mkdir()
        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Quiet, directory=str(site)))
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        url = f"http://127.0.0.1:{server.server_port}"
        # Only curl's loopback transport is changed; all verification is production.
        (tools / "curl").unlink()
        executable(tools / "curl", f'''#!/usr/bin/python3
import os, sys
args=sys.argv[1:]
if args[-1].startswith({url!r} + "/"):
    args=["=http" if a=="=https" else a for a in args]
os.execv("/usr/bin/curl", ["curl", *args])
''')
        # Tamper the actual signed v0.5.1 assets too, without changing trust roots.
        real = site / "releases/download/v0.5.1"
        real.mkdir(parents=True)
        names = ["SHA256SUMS", "SHA256SUMS.minisig"]
        for name in names:
            checked(["curl", "--proto", "=https", "-fsSL", f"{REPO}/releases/download/v0.5.1/{name}", "-o", str(real / name)])
        real_script = root / "real-assets.sh"
        real_script.write_text(SOURCE.replace(REPO, url))
        for name in names:
            path = real / name
            original = path.read_bytes()
            path.write_bytes(original + b"tampered\n")
            for verifier in (False, True):
                invoke(real_script, ("--version", "0.5.1"), ok=False, mini_sign=verifier, contains="signature verification failed")
                assert not installed.exists()
            path.write_bytes(original)
        for verifier in (False, True):
            invoke(real_script, ("--version", "0.5.1"), ok=False, mini_sign=verifier, contains="was published as rex-linux")
        shutil.rmtree(real)
        print("PASS tampered real v0.5.1 assets rejected by both verifiers", flush=True)
        keys = root / "keys"
        keys.mkdir()
        for name in ("ci", "backup", "unknown"):
            checked(["minisign", "-G", "-W", "-p", str(keys / f"{name}.pub"), "-s", str(keys / f"{name}.key")])
        values = {n: (keys / f"{n}.pub").read_text().splitlines()[1] for n in ("ci", "backup", "unknown")}
        source = SOURCE
        replacements = [(REPO, url)] + [
            ((ROOT / "packaging/keys" / filename).read_text().splitlines()[1], values[name])
            for filename, name in (("release.pub", "ci"), ("release-backup.pub", "backup"))]
        assert len(replacements) == 3
        for old, new in replacements:
            assert source.count(old) == 1
            source = source.replace(old, new)
        fixture = root / "fixture.sh"
        fixture.write_text(source)
        # Use the real packager, with empty Cargo metadata and a stub binary.
        build = root / "build"
        (build / "scripts").mkdir(parents=True)
        try:
            import tomllib  # noqa: F401
        except ImportError:
            (build / "scripts/tomllib.py").write_text("from tomli import *\n")
        for name in ("package-release.py", "install-desktop.sh"):
            shutil.copy2(ROOT / "scripts" / name, build / "scripts" / name)
        for name in ("resources", "vendor/rune/crates/rune/assets"):
            dest = build / name
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.symlink_to(ROOT / name, target_is_directory=True)
        for name in ("LICENSE", "README.md", "CHANGELOG.md", "vendor/rune/LICENSE", "vendor/ghostty-vt/LICENSE-GHOSTTY"):
            dest = build / name
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(ROOT / name, dest)
        stubs = root / "stubs"
        for name, output in (("cargo", '{"packages":[]}'), ("rustc", "rustc fixture"), ("zig", "0.17.0"), ("git", "1234567890")):
            executable(stubs / name, f"#!/bin/sh\nprintf '%s\\n' '{output}'\n")
        for version in ("0.5.1", "0.5.2"):
            (build / "Cargo.toml").write_text(f'[package]\nversion = "{version}"\n')
            # Like the client, the stub names itself as it was started: the packager checks
            # that the bridge's bin/rex-linux answers as the 0.7.x updater requires.
            executable(build / "target/release/rex-rune", f'#!/bin/sh\necho "${{0##*/}} {version}"\n')
            checked(["python3", str(build / "scripts/package-release.py")], env={**env, "PATH": f"{stubs}:{tools}"})
            release = site / "releases/download" / f"v{version}"
            release.mkdir(parents=True)
            for package in (build / "dist").glob(f"rex-rune-{version}-*.tar.gz"):
                shutil.copy2(package, release / package.name)
            package = next(release.glob("*.tar.gz"))
            (release / "SHA256SUMS").write_text(f"{hashlib.sha256(package.read_bytes()).hexdigest()}  {package.name}\n")

        def sign(version="0.5.2", key="ci", comment=None):
            path = site / "releases/download" / f"v{version}" / "SHA256SUMS"
            # The signing product id stayed rex-linux through the rename.
            checked(["minisign", "-S", "-s", str(keys / f"{key}.key"), "-m", str(path), "-t", comment or f"rex-linux v{version} SHA256SUMS"])

        sign("0.5.1")
        sign()
        latest = site / "releases/latest/download"
        latest.parent.mkdir(parents=True)
        latest.symlink_to(site / "releases/download/v0.5.2", target_is_directory=True)
        # Sentinel preferences and unrelated files must survive uninstall.
        preferences = home / ".config/rex-rune/workspace.json"
        preferences.parent.mkdir(parents=True)
        preferences.write_text('{"keep":true}')
        unrelated = home / ".local/bin/not-rex"
        unrelated.parent.mkdir(parents=True, exist_ok=True)
        unrelated.write_text("keep")

        def filelist():
            # Desktop-file-utils owns and regenerates its shared caches (intent.cache from 0.28).
            return {str(p.relative_to(home)): p.read_bytes() for p in home.rglob("*")
                    if p.is_file() and p.name not in ("mimeinfo.cache", "intent.cache", "icon-theme.cache")}

        baseline = filelist()
        for shell in ("dash", "bash", "busybox"):
            for verifier in (False, True):
                p = invoke(fixture, ("--version", "0.5.1"), shell=shell, mini_sign=verifier)
                assert p.stdout.count(f"Installed the Rex Rune desktop client (rex-rune 0.5.1) at {installed} and launcher {desktop}.") == 1
                assert p.stdout.count("Installed the Rex Rune desktop client") == 1
                assert p.stdout.count("This does not install or change the Rex server (rex).") == 1
                assert p.stdout.count(f"rex-rune needs a Rex server installed separately; see {url}#build-requirements.") == 1
                p = invoke(fixture, ("--upgrade",), shell=shell, mini_sign=verifier, contains=f"Installed the Rex Rune desktop client (rex-rune 0.5.2) at {installed} and launcher {desktop}.")
                assert p.stdout.count(f"Installed the Rex Rune desktop client (rex-rune 0.5.2) at {installed} and launcher {desktop}.") == 1
                assert p.stdout.count("Installed the Rex Rune desktop client") == 1
                assert p.stdout.count("This does not install or change the Rex server (rex).") == 1
                assert p.stdout.count(f"rex-rune needs a Rex server installed separately; see {url}#build-requirements.") == 1
                invoke(fixture, shell=shell, mini_sign=verifier, contains="already current")
                latest.unlink()
                latest.symlink_to(site / "releases/download/v0.5.1", target_is_directory=True)
                invoke(fixture, ok=False, mini_sign=verifier, contains="refusing downgrade")
                invoke(fixture, ("--force",), ok=False, mini_sign=verifier, contains="refusing downgrade")
                invoke(fixture, ("--version", "0.5.1"), mini_sign=verifier)
                latest.unlink()
                latest.symlink_to(site / "releases/download/v0.5.2", target_is_directory=True)
                invoke(fixture, ("--uninstall",), shell=shell)
                assert filelist() == baseline, f"uninstall changed unrelated files: {sorted(set(filelist()) ^ set(baseline))}"
        print("PASS fresh, upgrade, current, downgrade, explicit pin, uninstall: 3 shells x 2 verifiers", flush=True)

        release = site / "releases/download/v0.5.2"
        for filename in ("SHA256SUMS", "SHA256SUMS.minisig", next(release.glob("*.tar.gz")).name):
            path = release / filename
            original = path.read_bytes()
            path.write_bytes(original + b"tampered\n")
            for verifier in (False, True):
                invoke(fixture, ok=False, mini_sign=verifier)
                assert not installed.exists()
            path.write_bytes(original)
        # Alter only the trusted comment; first signature remains valid.
        sig = release / "SHA256SUMS.minisig"
        original = sig.read_text()
        sig.write_text(original.replace("v0.5.2", "v0.5.3"))
        for verifier in (False, True):
            invoke(fixture, ok=False, mini_sign=verifier)
        sig.write_text(original)
        sign(comment="rex-linux v0.5.3 SHA256SUMS")
        for verifier in (False, True):
            invoke(fixture, ("--version", "0.5.2"), ok=False, mini_sign=verifier)
        # Legacy unprehashed Ed signatures must be rejected identically.
        checked(["minisign", "-S", "-l", "-s", str(keys / "ci.key"), "-m", str(release / "SHA256SUMS"), "-t", "rex-linux v0.5.2 SHA256SUMS"])
        for verifier in (False, True):
            invoke(fixture, ok=False, mini_sign=verifier)
        # The new name in a trusted comment is not a release signature at all.
        sign(comment="rex-rune v0.5.2 SHA256SUMS")
        for verifier in (False, True):
            invoke(fixture, ok=False, mini_sign=verifier, contains="signature verification failed")
        sign(key="backup")
        invoke(fixture, ("--check",), contains="0.5.2 available")
        sign(key="unknown")
        for name in ("release.pub", "release-backup.pub"):
            dest = site / "raw/refs/tags/v0.5.2/packaging/keys" / name
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(keys / "unknown.pub", dest)
        for verifier in (False, True):
            invoke(fixture, ok=False, mini_sign=verifier, contains="signature verification failed")
            invoke(fixture, ("--check", "--trust-repo-keys"), mini_sign=verifier, contains="Explicit --trust-repo-keys")
        for name in ("release.pub", "release-backup.pub"):
            shutil.copy2(keys / "ci.pub", site / "raw/refs/tags/v0.5.2/packaging/keys" / name)
        for verifier in (False, True):
            invoke(fixture, ("--trust-repo-keys",), ok=False, mini_sign=verifier)
        sign()
        print("PASS tampered packages/checksums/signatures/comments; backup, unknown, and opt-in tag keys", flush=True)

        # Existing packages and mismatching receipts refuse before network/install.
        for package_binary in (Path("/usr/bin/rex-rune"), Path("/usr/bin/rex-linux")):
            assert not package_binary.exists()
            package_binary.write_text("package sentinel")
            try:
                invoke(fixture, ok=False, contains="package install detected")
                invoke(fixture, ("--force", "--check"))
                invoke(fixture, ("--uninstall",))
                assert package_binary.read_text() == "package sentinel"
            finally:
                package_binary.unlink()
        receipt = home / ".local/share/rex-rune/install.json"
        receipt.parent.mkdir(parents=True, exist_ok=True)
        receipt.write_text('{"path":"/wrong","sha256":"wrong"}')
        invoke(fixture, ok=False, contains="receipt does not match")
        invoke(fixture, ("--force", "--check"))
        receipt.unlink()
        desktop_tool = tools / "desktop-file-install"
        desktop_tool.unlink()
        invoke(fixture, ok=False, contains="missing desktop-file-install")
        desktop_tool.symlink_to(shutil.which("desktop-file-install"))
        executable(installed, "#!/bin/sh\nexit 1\n")
        invoke(fixture, ok=False, contains="--force to repair")
        fake_rex = tools / "rex"
        executable(fake_rex, "#!/bin/sh\necho 'rex server'\n")
        try:
            p = invoke(fixture, ("--force",), contains=f"Installed the Rex Rune desktop client (rex-rune 0.5.2) at {installed} and launcher {desktop}.")
            assert p.stdout.count(f"Installed the Rex Rune desktop client (rex-rune 0.5.2) at {installed} and launcher {desktop}.") == 1
            assert p.stdout.count("Installed the Rex Rune desktop client") == 1
            assert p.stdout.count("This does not install or change the Rex server (rex).") == 1
            assert p.stdout.count(f"Using your Rex server at {fake_rex}.") == 1
        finally:
            fake_rex.unlink()
        invoke(fixture, ("--uninstall",))
        invoke(fixture, ("--version", "0.5.2\n0.5.1"), ok=False, contains="expected stable")
        # A cut download cannot execute any part of the body, even on a clean EOF.
        cuts = [1, 100, len(source) // 2, *range(source.index('{ rex_install "$@"'), source.rindex("}"))]
        for end in cuts:
            partial = root / "partial.sh"
            partial.write_text(source[:end])
            for shell in ("dash", "bash", "busybox"):
                command = [shutil.which(shell)] if shell != "busybox" else [shutil.which("busybox"), "sh"]
                run([*command, str(partial)], env=env)
                run([*command, "-s", "--", "--uninstall"], input=partial.read_text(), env=env)
                assert filelist() == baseline
        print("PASS package guard, receipt guard, missing dependencies, truncated scripts", flush=True)

        # An install.sh install from before the rename: rex-linux, its launcher, icons
        # and receipt. Upgrading it leaves one launcher, the old command as a link and
        # the preferences where the client will move them from.
        old_binary = home / ".local/bin/rex-linux"
        old_desktop = home / ".local/share/applications/dev.shuv.rex-linux.desktop"
        old_receipt = home / ".local/share/rex-linux/install.json"
        old_icons = [home / f".local/share/icons/hicolor/{n}x{n}/apps/dev.shuv.rex-linux.png" for n in (32, 128, 256)]
        old_preferences = home / ".config/rex-linux/workspace.json"

        def old_install(version="0.5.1", receipt=True):
            executable(old_binary, f"#!/bin/sh\necho 'rex-linux {version}'\n")
            old_desktop.parent.mkdir(parents=True, exist_ok=True)
            old_desktop.write_text("[Desktop Entry]\nType=Application\nName=Rex Linux\nExec=rex-linux\n")
            for icon in old_icons:
                icon.parent.mkdir(parents=True, exist_ok=True)
                icon.write_bytes(b"old icon")
            old_preferences.parent.mkdir(parents=True, exist_ok=True)
            old_preferences.write_text('{"old":true}')
            if receipt:
                old_receipt.parent.mkdir(parents=True, exist_ok=True)
                digest = hashlib.sha256(old_binary.read_bytes()).hexdigest()
                old_receipt.write_text(f'{{"schema":1,"path":"{old_binary}","version":"{version}","sha256":"{digest}"}}')

        for shell in ("dash", "bash", "busybox"):
            for receipt_present in (True, False):
                old_install(receipt=receipt_present)
                invoke(fixture, ("--check",), shell=shell, contains="0.5.2 available (installed: 0.5.1)")
                p = invoke(fixture, ("--upgrade",), shell=shell, contains=f"Installed the Rex Rune desktop client (rex-rune 0.5.2) at {installed} and launcher {desktop}.")
                assert p.stdout.count(f"This replaces the rex-linux install: one launcher, your preferences kept, and {old_binary} is now a link to rex-rune.") == 1
                assert installed.is_file() and not installed.is_symlink()
                assert old_binary.is_symlink() and os.readlink(old_binary) == "rex-rune"
                assert checked([str(old_binary), "--version"]) == "rex-linux 0.5.2", "the old command starts the new client"
                launchers = sorted(path.name for path in desktop.parent.glob("*.desktop"))
                assert launchers == ["dev.shuv.rex-rune.desktop"], launchers
                assert not any(icon.exists() for icon in old_icons) and not old_receipt.parent.exists()
                assert old_preferences.read_text() == '{"old":true}'
                invoke(fixture, shell=shell, contains="already current")
                invoke(fixture, ("--uninstall",), shell=shell)
                assert not installed.exists() and not old_binary.is_symlink() and not old_binary.exists()
                assert old_preferences.read_text() == '{"old":true}'
                shutil.rmtree(old_preferences.parent)
                assert filelist() == baseline, "uninstall left part of the old install"
        # A receipt that does not match the old binary refuses, as for a current install.
        old_install()
        old_binary.write_text("#!/bin/sh\necho 'rex-linux 0.5.1'\n# changed\n")
        invoke(fixture, ok=False, contains="receipt does not match")
        assert not installed.exists() and old_desktop.exists()
        invoke(fixture, ("--uninstall",))
        shutil.rmtree(old_preferences.parent)
        assert filelist() == baseline
        # A release signed and published under the old name is declined, not half-installed.
        old_release = site / "releases/download/v0.5.0"
        old_release.mkdir()
        old_tarball = f"rex-linux-0.5.0-{checked(['uname', '-m'])}-linux-gnu.tar.gz"
        (old_release / old_tarball).write_bytes(b"not read")
        (old_release / "SHA256SUMS").write_text(f"{hashlib.sha256(b'not read').hexdigest()}  {old_tarball}\n")
        sign("0.5.0")
        for verifier in (False, True):
            invoke(fixture, ("--version", "0.5.0"), ok=False, mini_sign=verifier, contains="0.5.0 was published as rex-linux")
        assert filelist() == baseline
        print("PASS takeover of a rex-linux install in 3 shells, with and without a receipt; old releases declined", flush=True)
        server.shutdown()


if __name__ == "__main__":
    main()
