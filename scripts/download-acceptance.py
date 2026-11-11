#!/usr/bin/env python3
"""File-download acceptance (API 13) through the real UI, on private servers and a private Xvfb.

Requires rex with API 13 on PATH (the server under test), Xvfb and xdotool, and a built client
(REX_BIN, default target/release/rex-linux). Optional, each skipped with its reason when
missing: an API 12 rex (REX12_BIN, default ~/.cache/agent-ws/rex-preview14/bin/rex) for the
"too old" case; sshd, ssh and ssh-keygen for the one real ssh:// transfer; `unshare` with
unprivileged user namespaces for the low-space cases (a size-limited tmpfs is mounted over the
download folder inside a private namespace: the client has no injectable free-space source);
the rex-download-probe binary (REX_DOWNLOAD_PROBE, default next to REX_BIN or in
target/{release,debug}) for the file name with a newline, which a single-line field cannot type.

It starts and owns everything it touches: two private servers (own HOME, XDG, socket,
`--restore=false`, no tailscale on PATH), a throttling proxy, a private sshd and a private
Xvfb, all under one temporary directory below ~/.cache/agent-ws that holds every download
folder, state directory and artifact (the directory is printed; it is deleted when every case
passed and kept when any failed or was skipped, or with --keep). A stub xdg-open on
PATH logs every call; the run fails if it is ever called. A stub xdg-mime answers the file-type
question the client asks before offering Open (by file name, as GNOME and KDE do) and logs it;
the run fails if it is ever asked anything but `query filetype <absolute path>`. Every REX_* and display variable is
scrubbed and an endpoint under /run/user is refused, so nothing can reach another server.
Processes are stopped by PID.

Most cases drive the built binary with Rune's --script (the dialog is opened from Commands,
a path is typed, Enter pressed; the screen is read back as an accessibility tree). The cases
that need real input events (a plain transfer, the ssh:// transfer, quitting and killing the
app mid-transfer) use xdotool on the private Xvfb. Prints PASS, FAIL or SKIP per case; exits 1
when any case fails. A skipped case (a missing tool) does not fail the run by itself: use
--strict, as for a release, to make any skip exit 1, and then the run must also have compared at
least MIN_SAVED saved files. Naming case functions on the command line runs only those (and the
final checks), for development.

Cases: names (spaces, Unicode, leading dash, empty file, symlink, same file twice, modes, Open
offered for a .txt and withheld for programs, scripts, no extension and .svg); a zip with a jar
signature saved as report.zip and as notes.txt, which get no Open (and are never even sent to
the file-type question), next to a plain zip, which does; relative paths
after cd; a folder, a FIFO, a missing and an unreadable file refused in the server's words; API 12
"too old" with nothing sent; the 1 GiB question (held by the server while asked, released by
Escape with the app still running); a 1 GiB file cancelled partway (partial gone, the server's
descriptor closed); a file growing during the transfer; a fourth concurrent transfer; low space (a
yes, and a refusal); a newline in the name (core library only: the probe, not the client binary);
real X11 input; one transfer over a real ssh:// host; Close and SIGKILL mid-transfer; and last,
that xdg-open was never called, nothing is executable, no partial is left and every saved file's
SHA-256 equals its source."""
import asyncio
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time

ROOT = Path(__file__).resolve().parents[1]
APP = os.environ.get("REX_BIN", str(ROOT / "target/release/rex-linux"))
REX = shutil.which("rex")
REX12 = os.environ.get("REX12_BIN") or str(Path.home() / ".cache/agent-ws/rex-preview14/bin/rex")
for tool in ("rex", "Xvfb", "xdotool"):
    assert shutil.which(tool), f"{tool} required"
assert Path(APP).exists(), f"client binary not found: {APP} (build it, or set REX_BIN)"
MIB = 1 << 20
GIB = 1 << 30

base = Path.home() / ".cache/agent-ws"
base.mkdir(parents=True, exist_ok=True)
work = Path(tempfile.mkdtemp(prefix="rex-dl-", dir=base))
assert not str(work.resolve()).startswith("/run/user"), "refusing to work under /run/user"
for name in list(os.environ):
    if name.startswith(("REX_", "HERDR_")) or name in ("DBUS_SESSION_BUS_ADDRESS", "DISPLAY", "WAYLAND_DISPLAY",
                                                        "SSH_AUTH_SOCK", "XDG_SESSION_TYPE"):
        del os.environ[name]
# Short paths: a Unix socket path is limited to about 108 bytes, and the ssh:// forward puts one
# under XDG_RUNTIME_DIR.
for name in ("bin", "runs", "servers", "proxies", "ssh", "home", "rt"):
    (work / name).mkdir(mode=0o700)
os.environ.update(HOME=str(work / "home"), XDG_CONFIG_HOME=str(work / "home/config"),
                  XDG_STATE_HOME=str(work / "home/state"), XDG_CACHE_HOME=str(work / "home/cache"), TMPDIR=str(work))
(work / "bin/tailscale").write_text("#!/bin/sh\nexit 1\n")
XDG_LOG = work / "xdg-open.log"
(work / "bin/xdg-open").write_text(f"#!/bin/sh\nprintf '%s\\n' \"$*\" >> {XDG_LOG}\n")
# The desktop's file-type answer, by name (what GNOME and KDE do), so that only the client's own
# checks of the first bytes can refuse a hostile file with an ordinary name. Logs every call.
XDG_MIME_LOG = work / "xdg-mime.log"
(work / "bin/xdg-mime").write_text(f"""#!/bin/sh
printf '%s\\n' "$*" >> {XDG_MIME_LOG}
[ "$1" = query ] && [ "$2" = filetype ] || exit 2
case "$3" in
  *.txt|*.TXT) echo text/plain ;;
  *.md) echo text/markdown ;;
  *.zip) echo application/zip ;;
  *.svg) echo image/svg+xml ;;
  *.sh) echo application/x-shellscript ;;
  *) echo application/octet-stream ;;
esac
""")
for stub in ("tailscale", "xdg-open", "xdg-mime"):
    (work / "bin" / stub).chmod(0o755)
CLIENT_PATH = f"{work / 'bin'}:/usr/bin:/bin"

spec = importlib.util.spec_from_file_location("private_sshd", ROOT / "scripts/private_ssh.py")
private_sshd = importlib.util.module_from_spec(spec)
spec.loader.exec_module(private_sshd)

owned = []          # Popen objects stopped at the end (by PID)
cleanups = []       # callables run at the end
results = []        # (status, name, detail)
SAVED = []          # (saved file, source file or (source, length)): every one is checked at the end
DOWNLOAD_DIRS = []  # every download folder of every run


ARGS = sys.argv[1:]
STRICT = "--strict" in ARGS  # a skipped case exits 1
KEEP = "--keep" in ARGS      # keep the artifact directory even when every case passed
FILTER = [a for a in ARGS if not a.startswith("--")]  # case function names to run alone; none runs all
MIN_SAVED = 15  # a full run saves and compares at least this many files; fewer means cases did not run


class Skip(Exception):
    pass


def private(endpoint):
    assert "/run/user" not in endpoint, f"refusing {endpoint}"
    return endpoint


def stop(process):
    private_sshd.stop(process)


def sha256(path, limit=None):
    h = hashlib.sha256()
    left = limit
    with open(path, "rb") as f:
        while left is None or left > 0:
            chunk = f.read(MIB if left is None else min(MIB, left))
            if not chunk:
                break
            h.update(chunk)
            if left is not None:
                left -= len(chunk)
    return h.hexdigest()


def until(predicate, timeout, proc=None, step=.05):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = predicate()
        if value:
            return value
        if proc is not None and proc.poll() is not None:
            return predicate()
        time.sleep(step)
    return predicate()


# ---------------------------------------------------------------- private servers

class Server:
    def __init__(self, name, binary):
        self.name, self.binary = name, binary
        self.dir = work / "servers" / name
        self.env = {k: v for k, v in os.environ.items()}
        for var, sub in (("HOME", "home"), ("XDG_CONFIG_HOME", "config"), ("XDG_STATE_HOME", "state"),
                         ("XDG_CACHE_HOME", "cache"), ("XDG_RUNTIME_DIR", "runtime")):
            (self.dir / sub).mkdir(mode=0o700, parents=True)
            self.env[var] = str(self.dir / sub)
        self.home = self.dir / "home"
        self.sock = self.dir / "rex.sock"
        self.endpoint = private("unix://" + str(self.sock))
        self.env.update(REX_SERVER=self.endpoint, PATH=f"{work / 'bin'}:{Path(binary).parent}:/usr/bin:/bin",
                        SHELL="/bin/bash")
        (self.home / ".bashrc").write_text("PS1='$ '\nexport PAGER=cat\n")
        (self.home / ".bash_profile").write_text(". ~/.bashrc\n")
        self.proc = subprocess.Popen([binary, "server", "run", "--listen", self.endpoint, "--restore=false",
                                      "--config", "/dev/null", "--state-dir", str(self.dir / "server-state")],
                                     env=self.env, cwd=self.home, stdout=(self.dir / "server.log").open("w"),
                                     stderr=subprocess.STDOUT, start_new_session=True)
        owned.append(self.proc)
        assert until(self.sock.exists, 10), f"{name} server did not start"
        self.revision = json.loads(subprocess.check_output([binary, "version", "--json"], text=True))["api_revision"]

    def api(self, method, **payload):
        out = subprocess.run([self.binary, "--autostart=false", "--server", self.endpoint, "api", "call", method,
                              json.dumps(payload)], env=self.env, cwd=self.home, capture_output=True, text=True,
                             timeout=20)
        assert out.returncode == 0, out.stderr
        return json.loads(out.stdout)

    def new_session(self, label="devbox"):
        shell = {"block": {"flavor": "com.superlogical.terminal.shell"}}
        return self.api("session.create", label=label,
                        initial_windows=[{"window_label": "work", "layout": shell}])["session_id"]

    def open_files(self, needle):
        found = []
        for fd in os.listdir(f"/proc/{self.proc.pid}/fd"):
            try:
                target = os.readlink(f"/proc/{self.proc.pid}/fd/{fd}")
            except OSError:
                continue
            if needle in target:
                found.append(target)
        return found


def zip_bytes(entries):
    import io
    import zipfile
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as archive:
        for info, data in entries:
            archive.writestr(info, data)
    return buffer.getvalue()


def jar_zip():
    import zipfile
    meta = zipfile.ZipInfo("META-INF/")
    meta.extra = b"\xfe\xca\x00\x00"
    return zip_bytes([(meta, b""), ("META-INF/MANIFEST.MF", "Manifest-Version: 1.0\nMain-Class: A\n"),
                      ("A.class", b"\xca\xfe\xba\xbe\0\0\0\x34")])


JAR_ZIP = jar_zip()
assert JAR_ZIP.startswith(b"PK\x03\x04")
PLAIN_ZIP = zip_bytes([("hello.txt", "hello\n")])


def make_fixtures(home):
    def write(name, data, mode=None):
        path = home / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        if mode is not None:
            path.chmod(mode)
        return path

    def random_file(name, size):
        with open(home / name, "wb") as f:
            for _ in range(size // MIB):
                f.write(os.urandom(MIB))
            f.write(os.urandom(size % MIB))

    def sparse(name, size):
        with open(home / name, "wb") as f:
            f.truncate(size)

    write("notes.txt", b"plain notes\n")
    write("with space and ünïcode.txt", "héllo wörld\n".encode())
    write("-rf.txt", b"a name that starts with a dash\n")
    write("--help", b"so does this one\n")
    write("logo.svg", b"plain words under a svg name\n")
    write("empty.txt", b"")
    write("run.sh", b"#!/bin/sh\necho hi\n", 0o755)
    shutil.copy("/bin/true", home / "prog")
    (home / "prog").chmod(0o755)
    write("rel.txt", b"this is HOME/rel.txt\n")
    write("sub dir/rel.txt", b"this is in the sub dir\n")
    write("other-dir/only-there.txt", b"this is in the other dir\n")
    write("private.txt", b"secret\n", 0)
    # A zip whose first entry is META-INF/ with the 0xCAFE extra field: `file` calls it a Java
    # archive and the default handler would run it with `java -jar`, under any of these names.
    write("jar/report.zip", JAR_ZIP)
    write("jar/notes.txt", JAR_ZIP)
    write("jar/plain.zip", PLAIN_ZIP)
    (home / "adir").mkdir()
    os.mkfifo(home / "fifo")
    os.symlink("notes.txt", home / "link-to-notes")
    random_file("sha-check.bin", 64 * MIB)
    random_file("grow-a.bin", 64 * MIB)
    random_file("grow-b.bin", 32 * MIB)
    random_file("space-50.bin", 50_000_000)
    random_file("space-90.bin", 90_000_000)
    sparse("one-gib.img", GIB)
    for n in "abcd":
        sparse(f"img-{n}.iso", 600_000_000)


# ---------------------------------------------------------------- the proxy

class Sniffer:
    """Reads the client's WebSocket frames (masked) and keeps the names of the methods it called."""

    def __init__(self):
        self.buf, self.upgraded, self.methods = b"", False, []

    def feed(self, data):
        self.buf += data
        if not self.upgraded:
            end = self.buf.find(b"\r\n\r\n")
            if end < 0:
                return
            self.buf, self.upgraded = self.buf[end + 4:], True
        while len(self.buf) >= 2:
            b = self.buf
            op, masked, n, pos = b[0] & 0x0F, b[1] & 0x80, b[1] & 0x7F, 2
            if n == 126:
                if len(b) < 4:
                    return
                n, pos = int.from_bytes(b[2:4], "big"), 4
            elif n == 127:
                if len(b) < 10:
                    return
                n, pos = int.from_bytes(b[2:10], "big"), 10
            key = b""
            if masked:
                if len(b) < pos + 4:
                    return
                key, pos = b[pos:pos + 4], pos + 4
            if len(b) < pos + n:
                return
            payload, self.buf = b[pos:pos + n], b[pos + n:]
            if op == 1 and n < 65536:
                text = bytes(c ^ key[i % 4] for i, c in enumerate(payload)) if masked else payload
                try:
                    message = json.loads(text)
                except ValueError:
                    continue
                if message.get("type") == "request" and "download" in str(message.get("method")):
                    self.methods.append(message["method"])


class Proxy:
    """A Unix-socket proxy in front of a server: optionally slows server->client bytes per
    connection (so a transfer stays running) and records the download requests the client sends."""

    def __init__(self, name, upstream, rate=None):
        self.path = work / "proxies" / f"{name}.sock"
        self.endpoint = private("unix://" + str(self.path))
        self.upstream, self.rate, self.methods = str(upstream), rate, []
        self.loop = asyncio.new_event_loop()
        self.ready = threading.Event()
        self.thread = threading.Thread(target=self._run, daemon=True)
        self.thread.start()
        assert self.ready.wait(10), "proxy did not start"
        cleanups.append(self.close)

    def _run(self):
        asyncio.set_event_loop(self.loop)
        self.server = self.loop.run_until_complete(asyncio.start_unix_server(self._handle, str(self.path)))
        self.ready.set()
        self.loop.run_forever()

    async def _pump(self, reader, writer, throttle, sniffer=None):
        try:
            while True:
                data = await reader.read(65536)
                if not data:
                    break
                if sniffer:
                    before = len(sniffer.methods)
                    sniffer.feed(data)
                    self.methods.extend(sniffer.methods[before:])
                writer.write(data)
                await writer.drain()
                if throttle and self.rate and len(data) > 4096:
                    await asyncio.sleep(len(data) / self.rate)
        except (ConnectionError, asyncio.CancelledError):
            pass
        finally:
            try:
                writer.close()
            except Exception:
                pass

    async def _handle(self, client_reader, client_writer):
        try:
            up_reader, up_writer = await asyncio.open_unix_connection(self.upstream)
        except OSError:
            client_writer.close()
            return
        await asyncio.gather(self._pump(client_reader, up_writer, False, Sniffer()),
                             self._pump(up_reader, client_writer, True))

    def close(self):
        async def finish():
            self.server.close()
            tasks = [t for t in asyncio.all_tasks(self.loop) if t is not asyncio.current_task()]
            for t in tasks:
                t.cancel()
            await asyncio.gather(*tasks, return_exceptions=True)
            self.loop.stop()
        if self.thread.is_alive():
            asyncio.run_coroutine_threadsafe(finish(), self.loop)
            self.thread.join(5)


# ---------------------------------------------------------------- driving the app

def chord(t, name, ctrl=False, shift=False, alt=False):
    return {"t": t, "type": "key", "name": name, "ctrl": ctrl, "shift": shift, "alt": alt}


def tap(t, key):
    return {"t": t, "type": "tap", "key": key}


def type_(t, text, every=0.02):
    return {"t": t, "type": "type", "text": text, "every": every}


def enter(t):
    return chord(t, "Enter")


def commands(t):
    return chord(t, "p", ctrl=True, shift=True)


def download_steps(t, path, every=0.02):
    """Commands, "Download file…", the path, Enter. Takes about 2 s plus the typing."""
    steps = [commands(t), type_(t + 0.4, "download file"), enter(t + 1.2)]
    if path is not None:
        steps += [type_(t + 1.8, path, every), enter(t + 1.9 + every * len(path))]
    return steps


def popover_steps(t):
    """Commands, "Downloads", Enter: the popover with a row for every transfer of this launch."""
    return [commands(t), type_(t + 0.4, "downloads"), enter(t + 1.2)]


def tree_names(tree):
    """key -> accessible name for every element of a Rune --tree dump that has a key."""
    out = {}

    def walk(node):
        if node.get("key") and node["key"] not in out:
            out[node["key"]] = node.get("name") or ""
        if node.get("key") and "states" in node:
            out.setdefault("states:" + node["key"], node["states"])
        for child in node.get("children", []):
            walk(child)
    walk(tree["accessibility"])
    return out


class Run:
    def __init__(self, name, process, downloads, state, tree_path, log):
        self.name, self.process, self.downloads, self.state, self.log = name, process, Path(downloads), Path(state), log
        self.tree_path = Path(tree_path)
        self.returncode = None
        self.report = None

    @property
    def tree(self):
        assert self.tree_path.exists(), f"no tree: the app did not reach its snapshot ({self.log})"
        return tree_names(json.loads(self.tree_path.read_text()))

    def records(self):
        path = self.state / "rex-linux/downloads.json"
        return json.loads(path.read_text()).get("partials", []) if path.exists() else []

    def listing(self):
        return sorted(p.name for p in self.downloads.iterdir()) if self.downloads.exists() else []

    def partials(self):
        return [n for n in self.listing() if n.endswith(".rex-part")]


def run_app(name, steps, at, *, server, endpoint=None, session=None, label="devbox", watch=None,
            namespace_tmpfs=None, size="1440x900"):
    """Launch the built binary on Rune's script driver and return a Run. `watch(run)` is called
    while the app runs. `namespace_tmpfs="100M"` mounts a tmpfs of that size over the download
    folder inside a private user namespace and records what ends up in it as run.report."""
    rdir = work / "runs" / name
    for sub in ("home", "config", "state", "cache", "data", "downloads"):
        (rdir / sub).mkdir(parents=True)
    endpoint = private(endpoint or server.endpoint)
    session = session or server.new_session(label)
    downloads = rdir / "downloads"
    DOWNLOAD_DIRS.append(downloads)
    (rdir / "workspace.json").write_text(json.dumps({
        "hosts": [{"label": label, "endpoint": endpoint}], "last_host": endpoint, "sessions": {endpoint: session},
        "font_size": 14, "downloads": {"directory": str(downloads)}, "updates": {"check": False},
        "notifications": {"system_notifications": False}}))
    (rdir / "script.json").write_text(json.dumps(steps))
    env = {"HOME": str(rdir / "home"), "XDG_CONFIG_HOME": str(rdir / "config"), "XDG_STATE_HOME": str(rdir / "state"),
           "XDG_CACHE_HOME": str(rdir / "cache"), "XDG_DATA_HOME": str(rdir / "data"), "PATH": CLIENT_PATH,
           "SHELL": "/bin/bash", "USER": "user", "LOGNAME": "user", "LANG": "C.UTF-8"}
    cmd = [APP, "--endpoint", endpoint, "--session", session, "--config", str(rdir / "workspace.json"), "--",
           "--script", str(rdir / "script.json"), "--snap", str(rdir / "end.png"), "--at", str(at),
           "--size", size, "--scale", "1.5", "--realtime", "--tree", str(rdir / "end.tree.json")]
    if namespace_tmpfs:
        snapshot = ("import hashlib,json,os,sys\nd,out=sys.argv[1:3]\nr=[]\nfor n in sorted(os.listdir(d)):\n"
                    "    p=os.path.join(d,n); st=os.lstat(p)\n"
                    "    h=hashlib.sha256(open(p,'rb').read()).hexdigest() if os.path.isfile(p) else None\n"
                    "    r.append({'name':n,'size':st.st_size,'mode':st.st_mode&0o7777,'sha256':h})\n"
                    "v=os.statvfs(d)\njson.dump({'files':r,'free':v.f_bavail*v.f_frsize,'total':v.f_blocks*v.f_frsize},open(out,'w'))\n")
        inner = ('mount -t tmpfs -o size="$2",mode=0755 tmpfs "$1" || exit 97\n'
                 'dl=$1; report=$3; shift 3\n"$@"; rc=$?\n'
                 'python3 -I -c "$SNAPSHOT" "$dl" "$report"\nexit $rc\n')
        env["SNAPSHOT"] = snapshot
        cmd = ["unshare", "-Urm", "sh", "-c", inner, "sh", str(downloads), namespace_tmpfs, str(rdir / "report.json")] + cmd
    log = (rdir / "app.log").open("w")
    process = subprocess.Popen(cmd, env=env, stdout=log, stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL,
                               start_new_session=True)
    owned.append(process)
    run = Run(name, process, downloads, rdir / "state", rdir / "end.tree.json", rdir / "app.log")
    try:
        if watch:
            watch(run)
        run.returncode = process.wait(timeout=at + 90)
    except subprocess.TimeoutExpired:
        stop(process)
        raise AssertionError(f"{name}: the app did not exit (log {rdir / 'app.log'})")
    if namespace_tmpfs:
        assert (rdir / "report.json").exists(), f"namespace run left no report; see {rdir / 'app.log'}"
        run.report = json.loads((rdir / "report.json").read_text())
    return run


def expect_clean_exit(run):
    assert run.returncode == 0, f"{run.name}: the app exited with status {run.returncode} (see {run.log})"


def expect_no_partials(run):
    assert not run.partials(), f"{run.name}: partial files left: {run.partials()}"
    assert run.records() == [], f"{run.name}: downloads.json still records {run.records()}"


# ---------------------------------------------------------------- Xvfb and xdotool

class Desktop:
    """A private Xvfb with no window manager; the app's own chrome has the Close button."""

    def __init__(self):
        read_fd, write_fd = os.pipe()
        self.server = subprocess.Popen(["Xvfb", "-displayfd", str(write_fd), "-screen", "0", "1600x1000x24",
                                        "-nolisten", "tcp"], pass_fds=(write_fd,), stderr=subprocess.DEVNULL,
                                       start_new_session=True)
        os.close(write_fd)
        owned.append(self.server)
        with os.fdopen(read_fd) as pipe:
            self.display = ":" + pipe.readline().strip()
        assert self.display != ":", "Xvfb did not report a display"
        self.env = dict(os.environ, DISPLAY=self.display)
        cleanups.append(self.close)

    def close(self):
        stop(self.server)

    def x(self, *args):
        return subprocess.run(["xdotool", *map(str, args)], env=self.env, capture_output=True, text=True, timeout=20)

    def screenshot(self, name):
        if shutil.which("import"):
            subprocess.run(["import", "-window", "root", str(work / f"{name}.png")], env=self.env, timeout=20,
                           capture_output=True)

    def launch(self, name, server, endpoint, session, label, binary=APP, path=CLIENT_PATH, settle=4):
        rdir = work / "runs" / name
        for sub in ("home", "config", "state", "cache", "data", "downloads"):
            (rdir / sub).mkdir(parents=True, exist_ok=True)
        endpoint = private(endpoint)
        downloads = rdir / "downloads"
        if downloads not in DOWNLOAD_DIRS:
            DOWNLOAD_DIRS.append(downloads)
        (rdir / "workspace.json").write_text(json.dumps({
            "hosts": [{"label": label, "endpoint": endpoint}], "last_host": endpoint, "sessions": {endpoint: session},
            "font_size": 14, "downloads": {"directory": str(downloads)}, "updates": {"check": False},
            "notifications": {"system_notifications": False}}))
        env = {"HOME": str(rdir / "home"), "XDG_CONFIG_HOME": str(rdir / "config"), "XDG_STATE_HOME": str(rdir / "state"),
               "XDG_CACHE_HOME": str(rdir / "cache"), "XDG_DATA_HOME": str(rdir / "data"),
               "XDG_RUNTIME_DIR": str(work / "rt"), "PATH": path, "DISPLAY": self.display, "SHELL": "/bin/bash",
               "LANG": "C.UTF-8", "XDG_DATA_DIRS": "/usr/share"}
        log = (rdir / f"app-{int(time.time() * 1000)}.log").open("w")
        process = subprocess.Popen([binary, "--endpoint", endpoint, "--session", session, "--config",
                                    str(rdir / "workspace.json")], env=env, stdout=log, stderr=subprocess.STDOUT,
                                   stdin=subprocess.DEVNULL, start_new_session=True)
        owned.append(process)
        run = Run(name, process, downloads, rdir / "state", rdir / "none", log.name)
        window = until(lambda: self.x("search", "--onlyvisible", "--pid", process.pid).stdout.split()[-1:] or
                       None, 20, process)
        assert window, f"the app opened no window (see {log.name})"
        run.window = window[0]
        self.x("windowfocus", run.window)
        geometry = dict(line.split("=") for line in self.x("getwindowgeometry", "--shell", run.window).stdout.split()
                        if "=" in line)
        run.size = (int(geometry["WIDTH"]), int(geometry["HEIGHT"]))
        time.sleep(settle)  # connect and attach
        return run

    def start_download(self, run, path):
        self.x("windowfocus", run.window)
        self.x("key", "ctrl+shift+p")
        time.sleep(.8)
        self.x("type", "--delay", "30", "download file")
        time.sleep(.8)
        self.x("key", "Return")
        time.sleep(1.0)
        self.x("type", "--delay", "30", path)
        time.sleep(.4)
        self.x("key", "Return")

    def click_close(self, run):
        width, _ = run.size
        self.x("mousemove", str(width - 21), "26")
        time.sleep(.3)
        self.x("click", "1")


# ---------------------------------------------------------------- cases

def case(name):
    def decorate(fn):
        def runner(*args):
            if FILTER and fn.__name__ not in FILTER and fn.__name__ != "final_checks":
                return
            try:
                detail = fn(*args)
                status = "PASS"
            except Skip as reason:
                status, detail = "SKIP", str(reason)
            except AssertionError as error:
                status, detail = "FAIL", str(error)
            except Exception as error:  # a harness failure is a failure, never a pass
                status, detail = "FAIL", f"{type(error).__name__}: {error}"
            results.append((status, name, detail))
            print(f"{status} {name}" + (f" - {detail}" if detail else ""), flush=True)
        runner.__name__ = fn.__name__
        return runner
    return decorate


def saved(run, name, source, length=None):
    path = run.downloads / name
    assert path.is_file(), f"{name!r} was not saved; folder has {run.listing()}"
    SAVED.append((path, source, length))
    return path


def state_of(run, name):
    """The state line of the popover row for a file name."""
    tree = run.tree
    for key, value in tree.items():
        if key.startswith("download-name:") and value == name:
            line = tree.get("download-state:" + key.split(":", 1)[1], "")
            return line.split(" \u00b7 ", 1)[-1], key.split(":", 1)[1]
    raise AssertionError(f"no Downloads row for {name!r}; rows: {[v for k, v in tree.items() if k.startswith('download-name:')]}")


@case("names: spaces, Unicode, a leading dash, an empty file, a symlink, the same file twice, nothing executable, "
      "Open offered for a .txt only (not programs, no extension or .svg)")
def names(rex13):
    home = rex13.home
    unicode_name = "with space and ünïcode.txt"
    order = [f"~/{unicode_name}", "~/-rf.txt", "--help", "~/empty.txt", "~/link-to-notes", "~/notes.txt",
             "~/notes.txt", "~/prog", "~/run.sh", "~/logo.svg", "~/sha-check.bin"]
    steps, t = [], 2.5
    for path in order:
        steps += download_steps(t, path)
        t += 5.5 if path.endswith("sha-check.bin") else 3.6
    steps += popover_steps(t)
    run = run_app("names", steps, t + 3, server=rex13)
    expect_clean_exit(run)
    sources = {unicode_name: unicode_name, "-rf.txt": "-rf.txt", "--help": "--help", "empty.txt": "empty.txt",
               "link-to-notes": "notes.txt", "notes.txt": "notes.txt", "notes (1).txt": "notes.txt",
               "prog": "prog", "run.sh": "run.sh", "logo.svg": "logo.svg", "sha-check.bin": "sha-check.bin"}
    for name, source in sources.items():
        saved(run, name, home / source)
    assert (run.downloads / "empty.txt").stat().st_size == 0
    assert run.listing() == sorted(sources), f"unexpected files in the folder: {run.listing()}"
    modes = {n: oct((run.downloads / n).stat().st_mode & 0o7777) for n in run.listing()}
    assert all(not (run.downloads / n).stat().st_mode & 0o111 for n in run.listing()), f"executable bits: {modes}"
    assert (home / "prog").stat().st_mode & 0o111 and (home / "run.sh").stat().st_mode & 0o111, "sources are not executable"
    tree = run.tree
    # Open is an allow list: a .txt gets it; a program, a script, a file with no extension and an
    # .svg (plain words, whatever is inside) get only Show in folder.
    for name in ("prog", "run.sh", "--help", "logo.svg"):
        state, ident = state_of(run, name)
        assert state.startswith("Saved to"), (name, state)
        assert f"download-open:{ident}" not in tree, f"Open is offered for {name}"
        assert f"download-show:{ident}" in tree, f"Show in folder is missing for {name}"
    for name in ("notes.txt", "-rf.txt"):
        state, ident = state_of(run, name)
        assert f"download-open:{ident}" in tree and f"download-show:{ident}" in tree, f"Show in folder / Open missing for {name}"
    expect_no_partials(run)
    return f"{len(sources)} files, modes {sorted(set(modes.values()))}, the dialog took each name from the server"


@case("Open: a jar-signature zip saved as report.zip or notes.txt gets Show in folder only, a plain zip gets Open")
def open_needs_more_than_a_name(rex13):
    home = rex13.home
    asked_before = XDG_MIME_LOG.read_text() if XDG_MIME_LOG.exists() else ""
    order = ["~/jar/report.zip", "~/jar/notes.txt", "~/jar/plain.zip"]
    steps, t = [], 2.5
    for path in order:
        steps += download_steps(t, path)
        t += 3.6
    steps += popover_steps(t)
    run = run_app("open-jar", steps, t + 3, server=rex13)
    expect_clean_exit(run)
    for name, source in (("report.zip", "jar/report.zip"), ("notes.txt", "jar/notes.txt"), ("plain.zip", "jar/plain.zip")):
        saved(run, name, home / source)
    assert run.listing() == ["notes.txt", "plain.zip", "report.zip"], run.listing()
    tree = run.tree
    for name in ("report.zip", "notes.txt"):
        state, ident = state_of(run, name)
        assert state.startswith("Saved to"), (name, state)
        assert f"download-show:{ident}" in tree, f"Show in folder is missing for {name}"
        assert f"download-open:{ident}" not in tree, f"Open is offered for the jar saved as {name}"
    state, ident = state_of(run, "plain.zip")
    assert f"download-open:{ident}" in tree and f"download-show:{ident}" in tree, "a plain zip lost Open or Show in folder"
    asked = XDG_MIME_LOG.read_text()[len(asked_before):].splitlines()
    assert len(asked) == 1 and asked[0].startswith("query filetype ") and asked[0].endswith("/plain.zip"), \
        f"the file-type question was asked for something other than the plain zip: {asked}"
    expect_no_partials(run)
    return "report.zip and notes.txt (jar bytes) refused by their first bytes before the desktop was asked; plain.zip asked once and offered Open"


@case("~ and a relative path follow the shell's cd and the foreground program's directory")
def relative_paths(rex13):
    home = rex13.home
    steps = [type_(2.0, "cd 'sub dir'"), enter(2.6)]
    steps += download_steps(3.5, "rel.txt")
    steps += [type_(7.0, "cd .."), enter(7.4), type_(7.8, "sh"), enter(8.1),
              type_(8.5, "cd other-dir"), enter(9.0)]
    steps += download_steps(10.0, "only-there.txt")
    steps += download_steps(13.5, "~/rel.txt")
    steps += popover_steps(17.0)
    run = run_app("relative", steps, 20.0, server=rex13)
    expect_clean_exit(run)
    sub = saved(run, "rel.txt", home / "sub dir/rel.txt")
    saved(run, "only-there.txt", home / "other-dir/only-there.txt")
    saved(run, "rel (1).txt", home / "rel.txt")
    assert sub.read_text() == "this is in the sub dir\n", "relative path did not follow cd"
    expect_no_partials(run)
    return "rel.txt after cd came from 'sub dir', only-there.txt from a nested shell's directory (the foreground program, not the first shell), ~/rel.txt from HOME"


def refusal(name, path, words, rex13):
    run = run_app(f"refuse-{name}", download_steps(2.5, path), 7.5, server=rex13)
    expect_clean_exit(run)
    tree = run.tree
    message = tree.get("download-error", "")
    assert all(w in message for w in words), f"dialog says {message!r}; wanted {words}"
    assert run.listing() == [], f"folder not empty: {run.listing()}"
    expect_no_partials(run)
    return message


@case("a folder, a FIFO, a missing file and an unreadable file are refused in the dialog in the server's words")
def refusals(rex13):
    home = rex13.home
    seen = [refusal("folder", "~/adir", ["is a directory"], rex13),
            refusal("fifo", "~/fifo", ["not a regular file", "p---------"], rex13),
            refusal("missing", "~/nope.txt", ["does not exist"], rex13),
            refusal("denied", "~/private.txt", ["permission denied"], rex13)]
    return " | ".join(m.replace(str(home), "~") for m in seen)


@case("API 12 server: 'too old' message, and nothing is sent")
def api12(rex12, rex13):
    if rex12 is None:
        raise Skip("no API 12 rex (set REX12_BIN)")
    proxy = Proxy("api12", rex12.sock)
    run = run_app("api12", download_steps(2.5, None), 6.0, server=rex12, endpoint=proxy.endpoint,
                  label="shuvbot")
    expect_clean_exit(run)
    message = run.tree.get("download-old-server", "")
    assert message == "Rex on shuvbot is too old to send files; update Rex there.", f"dialog says {message!r}"
    assert proxy.methods == [], f"the client sent {proxy.methods} to the API 12 server"
    assert run.listing() == []
    return f"{message!r}; the server saw no download request"


def cancel_run(name, at_cancel, rate, rex13, path="~/one-gib.img"):
    proxy = Proxy(name, rex13.sock, rate)
    seen = {"partial": 0, "fd": False, "chip": False}

    def watch(run):
        def sample():
            for p in run.downloads.glob("*.rex-part"):
                seen["partial"] = max(seen["partial"], p.stat().st_size)
            seen["fd"] = seen["fd"] or bool(rex13.open_files("one-gib.img"))
            return False
        until(sample, at_cancel + 2, run.process)
    steps = download_steps(2.5, path)
    steps += [tap(6.0, "confirm-download"), tap(at_cancel, "download-chip"), tap(at_cancel + 1.0, "download-cancel:1")]
    return steps, watch, proxy, seen


@case("a 1 GiB sparse file: the question, a yes, cancelled partway: partial gone, the server's descriptor closed")
def cancel_midway(rex13):
    rate = 50_000_000
    steps, watch, proxy, seen = cancel_run("cancel", 16.0, rate, rex13)
    run = run_app("cancel", steps, 19.0, server=rex13, endpoint=proxy.endpoint, watch=watch)
    expect_clean_exit(run)
    percent = round(100 * seen["partial"] / GIB)
    assert seen["fd"], "never saw the server hold the file open, so the descriptor check proves nothing"
    assert 10 <= percent <= 90, f"the partial reached {percent}% when sampled; not mid-transfer"
    state, _ = state_of(run, "one-gib.img")
    assert state == "Cancelled", f"row says {state!r}"
    assert until(lambda: not rex13.open_files("one-gib.img"), 5), f"the server still holds {rex13.open_files('one-gib.img')}"
    assert run.listing() == [], f"folder not empty: {run.listing()}"
    expect_no_partials(run)
    return f"partial seen at about {percent}% of 1 GiB, then cancelled; folder empty, records empty, server fd closed"


@case("the 1 GiB question: size and free space, Cancel focused; the server holds the file while it is asked and "
      "Escape releases it while the app runs")
def large_question(rex13):
    # (a) the question as it looks, with the server holding the file while it is on screen.
    seen = {"held": False}

    def watch_held(run):
        # The question appears about 3 s in and the snapshot is taken at 9 s: sample until then.
        seen["held"] = until(lambda: bool(rex13.open_files("one-gib.img")), 7, run.process)
    run = run_app("large", download_steps(2.5, "~/one-gib.img"), 9.0, server=rex13, watch=watch_held)
    expect_clean_exit(run)
    tree = run.tree
    question = tree.get("download-question", "")
    assert "is 1.1 GB" in question and "free" in question, f"question: {question!r}"
    assert "focused" in tree.get("states:cancel-download", []), "Cancel is not focused"
    assert seen["held"], "the server held no descriptor on the file while the question was on screen"
    assert until(lambda: not rex13.open_files("one-gib.img"), 5), "the server still holds the file after the app left"
    assert run.listing() == []
    expect_no_partials(run)

    # (b) Escape on the question releases the server's file while the app is still running.
    stages = {}

    def watch_escape(run):
        stages["held"] = until(lambda: bool(rex13.open_files("one-gib.img")), 7, run.process)
        stages["released"] = until(lambda: not rex13.open_files("one-gib.img"), 12, run.process)
        stages["running"] = run.process.poll() is None
        stages["released_at"] = time.monotonic()
    steps = download_steps(2.5, "~/one-gib.img") + [chord(10.0, "Escape")]
    run = run_app("large-escape", steps, 16.0, server=rex13, watch=watch_escape)
    expect_clean_exit(run)
    assert stages.get("held"), "the server did not hold the file while the question was shown"
    assert stages.get("released"), "Escape on the question did not release the server's file"
    assert stages.get("running"), "the file was released only when the app exited, not by Escape"
    assert "download-question" not in run.tree, "the question is still on screen after Escape"
    assert run.listing() == [], f"folder not empty: {run.listing()}"
    expect_no_partials(run)
    return question.replace(str(run.downloads), "<folder>") + "; held while asked, released by Escape with the app running"


@case("a file appended to during the transfer ends within the growth cap, or fails with the growth message; no partial")
def grows(rex13):
    outcomes = []
    # (a) one append of 8 MiB while a throttled 64 MiB transfer runs
    source = rex13.home / "grow-a.bin"
    original = source.stat().st_size
    proxy = Proxy("grow-a", rex13.sock, 16_000_000)
    appended = {}

    def watch_a(run):
        if until(lambda: any(p.stat().st_size > 0 for p in run.downloads.glob("*.rex-part")), 20, run.process):
            with open(source, "ab") as f:
                f.write(os.urandom(8 * MIB))
            appended["size"] = source.stat().st_size
    run = run_app("grow-a", download_steps(2.5, "~/grow-a.bin") + popover_steps(11.0), 14.0, server=rex13,
                  endpoint=proxy.endpoint, watch=watch_a)
    expect_clean_exit(run)
    assert appended, "the transfer never started, so nothing was appended"
    state, _ = state_of(run, "grow-a.bin")
    if state.startswith("Saved"):
        path = run.downloads / "grow-a.bin"
        length = path.stat().st_size
        assert length in (original, appended["size"]), f"saved {length} bytes of {original}+{appended['size'] - original}"
        saved(run, "grow-a.bin", source, length)
        outcomes.append(f"append of 8 MiB: saved {length // MIB} MiB, equal to the first {length // MIB} MiB of the source")
    else:
        assert "grew past" in state, f"row says {state!r}"
        outcomes.append(f"append of 8 MiB: {state}")
    expect_no_partials(run)
    # (b) appended to continuously, faster than the throttled reader reads
    source = rex13.home / "grow-b.bin"
    proxy = Proxy("grow-b", rex13.sock, 20_000_000)
    stop_appending = threading.Event()

    def appender():
        with open(source, "ab") as f:
            while not stop_appending.is_set():
                f.write(b"\0" * (4 * MIB))
                f.flush()
                time.sleep(.05)
                if source.stat().st_size > 600 * MIB:
                    break

    def watch_b(run):
        if until(lambda: any(p.stat().st_size > 0 for p in run.downloads.glob("*.rex-part")), 20, run.process):
            threading.Thread(target=appender, daemon=True).start()
    try:
        run = run_app("grow-b", download_steps(2.5, "~/grow-b.bin") + popover_steps(13.0), 16.0, server=rex13,
                      endpoint=proxy.endpoint, watch=watch_b)
    finally:
        stop_appending.set()
    expect_clean_exit(run)
    state, _ = state_of(run, "grow-b.bin")
    if state.startswith("Saved"):
        length = (run.downloads / "grow-b.bin").stat().st_size
        saved(run, "grow-b.bin", source, length)
        outcomes.append(f"continuous growth: saved {length // MIB} MiB")
    else:
        assert "grew past" in state and "MB while downloading" in state, f"row says {state!r}"
        outcomes.append(f"continuous growth: {state}")
    assert not (run.downloads / "grow-b.bin").exists() or state.startswith("Saved")
    expect_no_partials(run)
    return "; ".join(outcomes)


@case("a fourth concurrent transfer is refused, and the server is asked three times")
def fourth(rex13):
    proxy = Proxy("fourth", rex13.sock, 2_000_000)
    steps, t = [], 2.5
    for n in "abcd":
        steps += download_steps(t, f"~/img-{n}.iso")
        t += 3.6
    run = run_app("fourth", steps, t + 1.5, server=rex13, endpoint=proxy.endpoint)
    expect_clean_exit(run)
    message = run.tree.get("download-error", "")
    assert "already running" in message, f"dialog says {message!r}"
    begins = [m for m in proxy.methods if m.endswith("download_begin")]
    assert len(begins) == 3, f"the server saw {len(begins)} download_begin requests: {proxy.methods}"
    expect_no_partials(run)
    return f"{message!r}; the server saw {len(begins)} requests; all three partials removed at exit"


def low_space_ready():
    probe = subprocess.run(["unshare", "-Urm", "true"], capture_output=True)
    if probe.returncode:
        raise Skip("unshare -Urm is not available (unprivileged user namespaces)")


@case("low space (a 100 MB tmpfs): the question says what a yes leaves, a yes saves, a file over the space is refused")
def low_space(rex13):
    low_space_ready()
    home = rex13.home
    asked = run_app("space-ask", download_steps(2.5, "~/space-50.bin"), 7.5, server=rex13, namespace_tmpfs="100M")
    expect_clean_exit(asked)
    tree = asked.tree
    question = tree.get("download-question", "")
    assert "so saving it leaves about" in question and "is 50 MB" in question, f"question: {question!r}"
    assert "focused" in tree.get("states:cancel-download", []), "Cancel is not focused"
    assert asked.report["files"] == [], f"the question left files: {asked.report['files']}"
    yes = run_app("space-yes", download_steps(2.5, "~/space-50.bin") + [tap(6.5, "confirm-download")], 11.0,
                  server=rex13, namespace_tmpfs="100M")
    expect_clean_exit(yes)
    files = yes.report["files"]
    assert [f["name"] for f in files] == ["space-50.bin"], f"folder after a yes: {files}"
    assert files[0]["size"] == 50_000_000 and files[0]["sha256"] == sha256(home / "space-50.bin"), files
    assert not files[0]["mode"] & 0o111
    over = run_app("space-over", download_steps(2.5, "~/space-90.bin"), 7.5, server=rex13, namespace_tmpfs="100M")
    expect_clean_exit(over)
    message = over.tree.get("download-error", "")
    free = re.search(r"has (\d[\d.]* \w+) free", question)
    assert free, f"no free space in the question: {question!r}"
    assert f"only {free.group(1)} free" in message and "download-question" not in over.tree, \
        f"dialog says {message!r}; the question said {free.group(1)} free"
    assert over.report["files"] == [], over.report["files"]
    assert until(lambda: not rex13.open_files("space-90.bin"), 5), "the server still holds the refused file"
    return f"{question!r}; yes saved 50 MB with sha256 equal; refused: {message!r}"


@case("a name with a newline (core library only, through rex-download-probe, not the client binary)")
def newline_name(rex13):
    candidates = [os.environ.get("REX_DOWNLOAD_PROBE"), str(Path(APP).with_name("rex-download-probe")),
                  str(ROOT / "target/release/rex-download-probe"), str(ROOT / "target/debug/rex-download-probe")]
    probe = next((c for c in candidates if c and Path(c).exists()), None)
    if not probe:
        raise Skip("rex-download-probe is not built (cargo build --locked --release --no-default-features --bin "
                   "rex-download-probe, or set REX_DOWNLOAD_PROBE)")
    out = work / "probe"
    out.mkdir()
    done = subprocess.run([probe, rex13.endpoint, str(out), str(rex13.proc.pid)], capture_output=True, text=True,
                          timeout=600, env=dict(rex13.env))
    (work / "probe.log").write_text(done.stdout + done.stderr)
    assert done.returncode == 0, f"rex-download-probe failed; see {work / 'probe.log'}: {done.stdout[-400:]}{done.stderr[-400:]}"
    newline = [line for line in done.stdout.splitlines() if "newline" in line.lower()]
    assert newline, "the probe's output does not mention a newline name"
    return newline[0].strip()


@case("real X11 input: one transfer typed with xdotool, 64 MiB, sha256 equal")
def x11_plain(desk, rex13):
    session = rex13.new_session()
    run = desk.launch("x11-plain", rex13, rex13.endpoint, session, "devbox")
    try:
        desk.start_download(run, "~/sha-check.bin")
        assert until(lambda: "sha-check.bin" in run.listing(), 30), f"nothing saved; folder has {run.listing()}"
        time.sleep(1)
        saved(run, "sha-check.bin", rex13.home / "sha-check.bin")
        expect_no_partials(run)
    finally:
        stop(run.process)
    return "saved through real key events"


@case("one transfer through a real ssh:// host (private sshd, throwaway keys)")
def over_ssh(desk, rex13):
    started = private_sshd.start(work / "ssh")
    if isinstance(started, str):
        raise Skip(started)
    sshd, port, user = started
    owned.append(sshd)
    endpoint = f"ssh://{user}@127.0.0.1:{port}{rex13.sock}"
    session = rex13.new_session("over-ssh")
    run = desk.launch("ssh", rex13, endpoint, session, "over-ssh", path=f"{work / 'ssh/bin'}:{CLIENT_PATH}", settle=10)
    try:
        desk.start_download(run, "~/sha-check.bin")
        if not until(lambda: "sha-check.bin" in run.listing(), 60):
            desk.screenshot("ssh-failure")
            raise AssertionError(f"nothing saved over ssh; folder has {run.listing()} (screenshot ssh-failure.png)")
        time.sleep(1)
        saved(run, "sha-check.bin", rex13.home / "sha-check.bin")
        expect_no_partials(run)
        log = (work / "ssh/sshd.log").read_text()
        assert "Accepted publickey" in log, "sshd saw no login, so the transfer did not use the forward"
    finally:
        stop(run.process)
    return f"64 MiB through ssh://{user}@127.0.0.1:{port}, sha256 equal; sshd logged the key login"


@case("quitting with the Close button mid-transfer leaves no partial and no record")
def quit_midway(desk, rex13):
    proxy = Proxy("quit", rex13.sock, 2_000_000)
    session = rex13.new_session()
    run = desk.launch("x11-quit", rex13, proxy.endpoint, session, "devbox")
    desk.start_download(run, "~/img-a.iso")
    assert until(lambda: run.partials(), 30), "the download did not start"
    time.sleep(3)
    assert run.records(), "no record while running"
    start = time.monotonic()
    desk.click_close(run)
    try:
        code = run.process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        stop(run.process)
        raise AssertionError("the app did not exit after Close")
    took = time.monotonic() - start
    assert code == 0, f"exit status {code}"
    expect_no_partials(run)
    assert run.listing() == [], run.listing()
    return f"exit status 0 after {took:.2f} s; folder empty, records empty"


@case("app killed (SIGKILL) mid-transfer: the partial stays, the next launch removes it")
def kill_midway(desk, rex13):
    proxy = Proxy("kill", rex13.sock, 2_000_000)
    session = rex13.new_session()
    run = desk.launch("x11-kill", rex13, proxy.endpoint, session, "devbox")
    desk.start_download(run, "~/img-b.iso")
    assert until(lambda: run.partials(), 30), "the download did not start"
    time.sleep(2)
    os.kill(run.process.pid, signal.SIGKILL)
    run.process.wait(timeout=10)
    left = run.partials()
    assert left and run.records(), "SIGKILL left nothing behind, so the relaunch proves nothing"
    second = desk.launch("x11-kill", rex13, proxy.endpoint, session, "devbox")
    try:
        assert until(lambda: not second.partials() and second.records() == [], 15), \
            f"after relaunch: {second.listing()} records {second.records()}"
    finally:
        stop(second.process)
    return f"left {left[0][:24]}... and its record; the relaunch removed both"


@case("nothing was opened, nothing is executable, no partial is left, every saved file's sha256 equals its source")
def final_checks():
    opened = XDG_LOG.read_text() if XDG_LOG.exists() else ""
    assert opened == "", f"xdg-open was called: {opened!r}"
    asked = XDG_MIME_LOG.read_text().splitlines() if XDG_MIME_LOG.exists() else []
    stray = [line for line in asked if not re.fullmatch(r"query filetype /.+", line)]
    assert not stray, f"xdg-mime was asked something other than `query filetype <absolute path>`: {stray}"
    for line in asked:
        assert not re.search(r"/(prog|run\.sh|--help|logo\.svg|report\.zip)$", line), f"the file-type question was asked for {line}"
    problems = []
    for folder in DOWNLOAD_DIRS:
        if not folder.exists():
            continue
        for path in folder.iterdir():
            if path.name.endswith(".rex-part"):
                problems.append(f"leftover {path}")
            elif path.is_file() and path.stat().st_mode & 0o111:
                problems.append(f"executable {path}")
    assert not problems, "; ".join(problems)
    checked = 0
    for path, source, length in SAVED:
        if length is None:
            assert path.stat().st_size == source.stat().st_size, f"{path.name}: size {path.stat().st_size} vs {source.stat().st_size}"
        assert sha256(path) == sha256(source, length), f"{path} differs from {source}"
        checked += 1
    if not FILTER:
        assert checked >= MIN_SAVED, f"only {checked} saved files compared with their sources; expected at least {MIN_SAVED}"
    return f"{checked} saved files compared with their sources; xdg-open stub log empty; {len(asked)} file-type questions, all `query filetype <path>`; {len(DOWNLOAD_DIRS)} folders scanned"


def main():
    print("Private download-acceptance artifacts:", work, flush=True)
    rex13 = Server("api13", REX)
    assert rex13.revision == 13, f"rex on PATH is API {rex13.revision}, not 13"
    make_fixtures(rex13.home)
    rex12 = None
    if Path(REX12).exists():
        rex12 = Server("api12", REX12)
        assert rex12.revision == 12, f"{REX12} is API {rex12.revision}"
        make_fixtures(rex12.home)
    else:
        print(f"note: no API 12 rex at {REX12}", flush=True)
    desk = Desktop()
    print(f"servers: API {rex13.revision} {REX}" + (f", API {rex12.revision} {REX12}" if rex12 else "") +
          f"; client {APP}; Xvfb {desk.display}", flush=True)
    names(rex13)
    open_needs_more_than_a_name(rex13)
    relative_paths(rex13)
    refusals(rex13)
    api12(rex12, rex13)
    large_question(rex13)
    cancel_midway(rex13)
    grows(rex13)
    fourth(rex13)
    low_space(rex13)
    newline_name(rex13)
    x11_plain(desk, rex13)
    over_ssh(desk, rex13)
    quit_midway(desk, rex13)
    kill_midway(desk, rex13)
    final_checks()


try:
    main()
finally:
    for fn in reversed(cleanups):
        try:
            fn()
        except Exception as error:
            print(f"cleanup: {error}", flush=True)
    for process in reversed(owned):
        try:
            if process.poll() is None:
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except (ProcessLookupError, PermissionError):
                    pass
                stop(process)
        except Exception as error:
            print(f"cleanup: {error}", flush=True)
failed = [r for r in results if r[0] == "FAIL"]
skipped = [r for r in results if r[0] == "SKIP"]
clean = not failed and not skipped and not KEEP
print(f"\n{len(results) - len(failed) - len(skipped)} passed, {len(failed)} failed, {len(skipped)} skipped "
      f"(artifacts: {'deleted' if clean else work})", flush=True)
for status, name, detail in skipped:
    print(f"  skipped: {name}: {detail}")
if STRICT and skipped:
    print(f"--strict: {len(skipped)} skipped case(s) count as a failure")
if clean:
    for dirpath, dirnames, filenames in os.walk(work):  # a fixture with mode 000 would stop rmtree
        os.chmod(dirpath, 0o700)
        for name in filenames:
            try:
                os.chmod(os.path.join(dirpath, name), 0o600)
            except OSError:
                pass
    shutil.rmtree(work, ignore_errors=True)
sys.exit(1 if failed or (STRICT and skipped) else 0)
