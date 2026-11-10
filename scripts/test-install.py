#!/usr/bin/env python3
"""Run ONLY in a disposable container via install-script-test.sh (writes /usr/bin).

Real 0.5.1 assets exercise the production keys and both verifiers. Local packages
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
        installed = home / ".local/bin/rex-linux"
        desktop = home / ".local/share/applications/dev.shuv.rex-linux.desktop"
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
        for verifier in (False, True):
            if compatible:
                invoke(args=("--version", "0.5.1"), mini_sign=verifier, contains="Installed Rex Linux 0.5.1")
                assert checked([str(installed), "--version"]) == "rex-linux 0.5.1"
                assert desktop.exists()
                invoke(args=("--version", "0.5.1"), mini_sign=verifier, contains="already current")
                invoke(args=("--uninstall",))
                assert not installed.exists() and not desktop.exists()
            else:
                invoke(args=("--version", "0.5.1", "--check"), mini_sign=verifier, contains="0.5.1 available")
        print("PASS real v0.5.1 assets with minisign and OpenSSL", flush=True)

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
        arch = checked(["uname", "-m"])
        names = ["SHA256SUMS", "SHA256SUMS.minisig"]
        if compatible:
            names.append(f"rex-linux-0.5.1-{arch}-linux-gnu.tar.gz")
        for name in names:
            checked(["curl", "--proto", "=https", "-fsSL", f"{REPO}/releases/download/v0.5.1/{name}", "-o", str(real / name)])
        real_script = root / "real-assets.sh"
        real_script.write_text(SOURCE.replace(REPO, url))
        for name in names:
            path = real / name
            original = path.read_bytes()
            path.write_bytes(original + b"tampered\n")
            for verifier in (False, True):
                args = ("--version", "0.5.1") + (() if compatible else ("--check",))
                invoke(real_script, args, ok=False, mini_sign=verifier)
                assert not installed.exists()
            path.write_bytes(original)
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
            executable(build / "target/release/rex-linux", f"#!/bin/sh\necho 'rex-linux {version}'\n")
            checked(["python3", str(build / "scripts/package-release.py")], env={**env, "PATH": f"{stubs}:{tools}"})
            release = site / "releases/download" / f"v{version}"
            release.mkdir(parents=True)
            for package in (build / "dist").glob(f"rex-linux-{version}-*.tar.gz"):
                shutil.copy2(package, release / package.name)
            package = next(release.glob("*.tar.gz"))
            (release / "SHA256SUMS").write_text(f"{hashlib.sha256(package.read_bytes()).hexdigest()}  {package.name}\n")

        def sign(version="0.5.2", key="ci", comment=None):
            path = site / "releases/download" / f"v{version}" / "SHA256SUMS"
            checked(["minisign", "-S", "-s", str(keys / f"{key}.key"), "-m", str(path), "-t", comment or f"rex-linux v{version} SHA256SUMS"])

        sign("0.5.1")
        sign()
        latest = site / "releases/latest/download"
        latest.parent.mkdir(parents=True)
        latest.symlink_to(site / "releases/download/v0.5.2", target_is_directory=True)
        # Sentinel preferences and unrelated files must survive uninstall.
        preferences = home / ".config/rex-linux/workspace.json"
        preferences.parent.mkdir(parents=True)
        preferences.write_text('{"keep":true}')
        unrelated = home / ".local/bin/not-rex"
        unrelated.parent.mkdir(parents=True, exist_ok=True)
        unrelated.write_text("keep")

        def filelist():
            # Desktop-file-utils owns and regenerates its shared MIME cache.
            return {str(p.relative_to(home)): p.read_bytes() for p in home.rglob("*")
                    if p.is_file() and p.name not in ("mimeinfo.cache", "icon-theme.cache")}

        baseline = filelist()
        for shell in ("dash", "bash", "busybox"):
            for verifier in (False, True):
                invoke(fixture, ("--version", "0.5.1"), shell=shell, mini_sign=verifier)
                invoke(fixture, ("--upgrade",), shell=shell, mini_sign=verifier, contains="Installed Rex Linux 0.5.2")
                invoke(fixture, shell=shell, mini_sign=verifier, contains="already current")
                latest.unlink()
                latest.symlink_to(site / "releases/download/v0.5.1", target_is_directory=True)
                invoke(fixture, ok=False, mini_sign=verifier, contains="refusing downgrade")
                invoke(fixture, ("--force",), ok=False, mini_sign=verifier, contains="refusing downgrade")
                invoke(fixture, ("--version", "0.5.1"), mini_sign=verifier)
                latest.unlink()
                latest.symlink_to(site / "releases/download/v0.5.2", target_is_directory=True)
                invoke(fixture, ("--uninstall",), shell=shell)
                assert filelist() == baseline, "uninstall changed unrelated files"
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
        package_binary = Path("/usr/bin/rex-linux")
        assert not package_binary.exists()
        package_binary.write_text("package sentinel")
        try:
            invoke(fixture, ok=False, contains="package install detected")
            invoke(fixture, ("--force", "--check"))
            invoke(fixture, ("--uninstall",))
            assert package_binary.read_text() == "package sentinel"
        finally:
            package_binary.unlink()
        receipt = home / ".local/share/rex-linux/install.json"
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
        invoke(fixture, ("--force",), contains="Installed Rex Linux 0.5.2")
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
        server.shutdown()


if __name__ == "__main__":
    main()
