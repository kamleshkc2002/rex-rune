#!/usr/bin/env python3
"""Remote UI acceptance on two disposable Rex servers and a private Xvfb.

Requires rex (API 12 or 13), Xvfb, openbox, xdotool, ImageMagick and a built client
(REX_BIN). Uses ~/.cache/agent-ws, private HOME/XDG/config/socket paths and an
explicit REX_SERVER with autostart disabled. Never uses installed Rex servers,
packages or units. If sshd can run as this user, host B uses a real private SSH
socket forward, with disposable keys and a private ssh config. All owned
processes are stopped by PID. Artifacts and screenshots remain in the printed dir.
"""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
BINARY = os.environ.get("REX_BIN", str(ROOT / "target/release/rex-linux"))
REX = shutil.which("rex")
EXPECTED_REVISION = os.environ.get("REX_EXPECT_REVISION")
LABEL = "remote-accept"
for tool in ("rex", "Xvfb", "openbox", "xdotool", "import"):
    assert shutil.which(tool), f"{tool} required"
base = Path.home() / ".cache/agent-ws"
base.mkdir(parents=True, exist_ok=True)
work = Path(tempfile.mkdtemp(prefix="rex-remote-", dir=base))
for name in ("home", "config", "state", "runtime", "cache", "captures", "bin"):
    (work / name).mkdir(mode=0o700)
for name in list(os.environ):
    if name.startswith(("REX_", "HERDR_")) or name in ("DBUS_SESSION_BUS_ADDRESS", "DISPLAY", "WAYLAND_DISPLAY", "SSH_AUTH_SOCK"):
        del os.environ[name]
endpoints = ["unix://" + str(work / f"{name}.sock") for name in ("a", "b")]
os.environ.update(HOME=str(work / "home"), XDG_CONFIG_HOME=str(work / "config"),
                  XDG_STATE_HOME=str(work / "state"), XDG_RUNTIME_DIR=str(work / "runtime"),
                  XDG_CACHE_HOME=str(work / "cache"), REX_SERVER=endpoints[0],
                  REX_BIN=BINARY, REX_CAPTURE_OUT=str(work / "captures"), TMPDIR=str(work))
# Never discover real tailnet hosts, even if a failed test opens Add host.
(work / "bin/tailscale").write_text("#!/bin/sh\nexit 1\n")
(work / "bin/tailscale").chmod(0o755)
os.environ["PATH"] = str(work / "bin") + os.pathsep + os.environ["PATH"]
spec = importlib.util.spec_from_file_location("accept", ROOT / "scripts/rune-acceptance-x11.py")
accept = importlib.util.module_from_spec(spec)
spec.loader.exec_module(accept)
spec = importlib.util.spec_from_file_location("private_sshd", ROOT / "scripts/private_ssh.py")
private_sshd = importlib.util.module_from_spec(spec)
spec.loader.exec_module(private_sshd)
processes = []
desk = None


def stop(process):
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


def wait(predicate, message, timeout=12):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(.08)
    raise AssertionError(message)


def cli(host, *args):
    return subprocess.run([REX, "--autostart=false", "--server", endpoints[host], *args],
                          capture_output=True, text=True, timeout=15)


def api(host, method, **payload):
    result = cli(host, "api", "call", method, json.dumps(payload))
    assert result.returncode == 0, result.stderr
    return json.loads(result.stdout)


def client_id(host):
    for client in api(host, "client.list")["clients"]:
        if client.get("info", {}).get("kind") == "app" and client["info"].get("label") == LABEL:
            cid = client.get("id", client.get("client_id"))
            if "client.tab.next" in {a["name"] for a in api(host, "client.inspect", client_id=cid).get("actions", [])}:
                return cid
    return None


def perform(host, action, args=None, error=None, direct=False, **context):
    if direct:
        result = cli(host, "api", "call", "client.perform", json.dumps({
            "client_id": client_id(host), "action": action, "args": args or {},
            "timeout_ms": 3000, **context}))
    else:
        result = cli(host, "-C", LABEL, "do", action,
                     *[f"{key}={value}" for key, value in (args or {}).items()])
    if error:
        assert result.returncode != 0 and error in result.stderr, (action, result.stdout, result.stderr)
    else:
        assert result.returncode == 0, (action, result.stdout, result.stderr)
    return result


def private_ssh():
    started = private_sshd.start(work)
    if isinstance(started, str):
        print(f"SKIP SSH: {started}; using two local sockets", flush=True)
        return endpoints[1]
    server, port, user = started
    processes.append(server)
    print("PASS real private SSH server and disposable-key authentication", flush=True)
    return f"ssh://{user}@127.0.0.1:{port}{work}/b.sock"


try:
    print("Private remote-control artifacts:", work, flush=True)
    version = subprocess.check_output([REX, "version", "--json"], text=True)
    revision = json.loads(version)["api_revision"]
    assert revision in (12, 13), version
    if EXPECTED_REVISION:
        assert revision == int(EXPECTED_REVISION), version
    (work / "version.txt").write_text(version)
    (work / "init.lua").write_text('rex.bind("ctrl+b>c", "client.sidebar.toggle")\n')
    for i, endpoint in enumerate(endpoints):
        process = subprocess.Popen([REX, "server", "run", "--listen", endpoint, "--restore=false",
                                    "--config", str(work / "init.lua"), "--state-dir", str(work / f"server-{i}")],
                                   stdout=(work / f"server-{i}.log").open("w"), stderr=subprocess.STDOUT)
        processes.append(process)
        wait(lambda: Path(endpoint.removeprefix("unix://")).exists(), "private server did not start")
    ssh_endpoint = private_ssh()
    shell = {"block": {"flavor": "com.superlogical.terminal.shell", "options": {"shell": "/bin/sh"}}}
    sessions = [api(i, "session.create", label=f"remote-{i}", initial_windows=[
        {"window_label": "first", "layout": shell},
        {"window_label": "second", "layout": {"split": {"direction": "horizontal", "ratio": .5, "before": shell, "after": shell}}},
        {"window_label": "last", "layout": shell},
    ])["session_id"] for i in range(2)]
    workspace = work / "workspace.json"
    config = {"client_label": "remote-accept", "hosts": [
        {"label": "private-A", "endpoint": endpoints[0]}, {"label": "private-B", "endpoint": ssh_endpoint}],
        "notifications": {"system_notifications": False}}
    workspace.write_text(json.dumps(config))
    desk = accept.Desktop()
    app, window = desk.launch(sessions[0], workspace, log=(work / "client-off.log").open("w"))
    for i in range(2):
        wait(lambda: client_id(i), f"host {i} did not register actions")
    declaration = next((a for a in api(1, "client.inspect", client_id=client_id(1))["actions"] if a["name"] == "session.select"), None)
    if declaration:
        assert declaration["args"]["required"] == ["session_id"]
        assert "window_id" in declaration["args"]["properties"]
    title = desk.x("getwindowname", window)
    perform(1, "client.tab.next", error="Remote control is off")
    assert desk.x("getwindowname", window) == title
    print("PASS schema advertised, client-label targeting and default-off refusal without navigation", flush=True)
    stop(app)
    config = json.loads(workspace.read_text())
    # The old abruptly stopped client remains in the server reconnect grace period.
    # A new label also verifies that edits are advertised after reopening.
    LABEL = "remote-accept-enabled"
    config["client_label"] = LABEL
    for host in config["hosts"]:
        host["remote_control"] = True
    workspace.write_text(json.dumps(config))
    app, window = desk.launch(sessions[0], workspace, log=(work / "client-on.log").open("w"))
    for i in range(2):
        wait(lambda: client_id(i), f"enabled host {i} did not register")

    # Part 1 has seven actions; part 2 additionally supports cross-host selection.
    active = 1 if declaration else 0

    def view(host=active):
        return api(host, "session.view", session_id=sessions[host])

    windows = [w["window_id"] for w in view()["windows"]]
    if declaration:
        perform(1, "session.select", {"session_id": sessions[1], "window_id": windows[1]})
        wait(lambda: "private-B" in desk.x("getwindowname", window) and "remote-1" in desk.x("getwindowname", window), "second host/session not showing")
    else:
        perform(active, "client.tab.goto", {"index": 2})
    wait(lambda: view()["active_window_id"] == windows[1], "second tab not selected")
    time.sleep(.2)  # Part 1 replies on dispatch, before the UI applies navigation acknowledgement.
    perform(active, "pane.focus", {"direction": "right"})
    perform(active, "pane.zoom")
    perform(active, "pane.zoom")
    perform(active, "pane.scroll", {"lines": -10})
    for action, args, index in [("client.tab.goto", {"index": 2}, 1),
                                ("client.tab.goto", {"index": -1 if declaration else 3}, 2),
                                ("client.tab.previous", None, 1), ("client.tab.next", None, 2)]:
        perform(active, action, args)
        wait(lambda: view()["active_window_id"] == windows[index], f"{action} did not focus expected tab")
    perform(active, "client.sidebar.toggle")
    perform(active, "client.sidebar.toggle")
    print("PASS all eight view-only actions, cross-host session/tab selection and goto -1" if declaration else
          "PASS all seven part-1 view-only actions", flush=True)
    perform(1 - active, "client.sidebar.toggle", error="not on screen")
    perform(active, "client.sidebar.toggle", {"extra": 1}, error="Invalid arguments", direct=True)
    if declaration:
        perform(1, "session.select", {"session_id": "session:missing"}, error="does not exist")
        perform(1, "session.select", {"session_id": sessions[1], "window_id": "window:missing"}, error="does not exist")
    before = view()
    block = next(w for w in before["windows"] if w["window_id"] == before["active_window_id"])["focused_block_id"]
    def text():
        return api(active, "com.superlogical.terminal.format", session_id=sessions[active], block_id=block,
                   args={"format": "text", "trim": True, "unwrap": True})["content"]
    before_text = text()
    for action, args, hint in [("client.tab.new", None, "rex window new"), ("pane.paste", None, "rex send"),
                               ("pane.send_key", {"key": "x"}, "rex send-key")]:
        perform(active, action, args, error=hint)
    assert len(view()["windows"]) == len(before["windows"])
    assert text() == before_text
    print("PASS malformed/unknown/cross-host targets and dangerous actions refused without terminal changes", flush=True)
    desk.key("ctrl+comma")
    perform(active, "client.sidebar.toggle", error="dialog or popup")
    # Settings is 816x680, centered under the 52 pt toolbar of the 1440x900 window (its corner
    # is at 312, 136); the sixth navigation row is Remote Hosts, 102 pt in and 254 pt down from
    # that corner. `the_acceptance_scripts_settings_click_lands_on_remote_hosts` in
    # src/app/settings_tests.rs holds this point to that row.
    desk.x("mousemove", "--window", window, 414, 390)
    desk.x("click", 1)
    time.sleep(.4)
    desk.capture(window, "settings-remote-hosts")
    desk.key("Escape")
    desk.type("echo REMOTE_TYPING")
    perform(active, "client.tab.next", error="typing")
    if declaration:
        perform(0, "session.select", {"session_id": sessions[0]}, error="typing")
    desk.key("ctrl+u")
    time.sleep(1.6)

    perform(active, "client.tab.goto", {"index": 1})
    wait(lambda: view()["active_window_id"] == windows[0], "first tab not selected")
    block = next(w for w in view()["windows"] if w["window_id"] == windows[0])["focused_block_id"]

    # Regression: search edits used not to count as typing. A remote tab change
    # blurred the field and sent subsequent text into the other tab's shell.
    before = view()
    before_text = text()
    desk.key("ctrl+shift+f")
    desk.type("abc")
    perform(active, "client.tab.next", error="text field")
    time.sleep(1.6)
    # A pause is not permission to take focus from a text field, for any action.
    for action, args in [("client.tab.next", None), ("client.sidebar.toggle", None),
                         ("pane.scroll", {"lines": 1})]:
        perform(active, action, args, error="text field")
    if declaration:
        perform(1 - active, "session.select", {"session_id": sessions[1 - active]}, error="text field")
    desk.type("REMOTE_SEARCH_MUST_NOT_REACH_SHELL")
    assert view()["active_window_id"] == before["active_window_id"]
    assert text() == before_text, "search input leaked into the shell"
    desk.capture(window, "search-remote-refused")
    desk.key("Escape")
    print("PASS search-field guard: immediate and paused remote requests refused; tab unchanged; no shell input", flush=True)
    desk.key("ctrl+b")
    time.sleep(1.6)
    perform(active, "client.tab.next", error="key sequence")
    perform(active, "client.sidebar.toggle", error="key sequence")
    desk.key("c")
    time.sleep(1.6)
    perform(active, "client.sidebar.toggle")
    print("PASS pending sequence guard survives pause and accepts its original tail", flush=True)
    if declaration:
        perform(0, "session.select", {"session_id": sessions[0]}, direct=True, session_id=sessions[0])
        wait(lambda: "private-A" in desk.x("getwindowname", window), "first host not showing after acknowledged navigation")
    else:
        perform(active, "client.tab.next", direct=True, session_id=sessions[active])
    desk.capture(window, "performed-notice")
    print("PASS panel and typing guards; direct client.perform acknowledges navigation", flush=True)
    print("PASS remote UI acceptance (SSH host B)" if ssh_endpoint.startswith("ssh:") else "PASS remote UI acceptance (two local servers; SSH skipped)", flush=True)
except Exception:
    if desk and "window" in locals():
        desk.capture(window, "failure")
    raise
finally:
    if desk:
        for app in desk.apps:
            stop(app)
        stop(desk.wm)
        stop(desk.server)
    for process in reversed(processes):
        stop(process)
