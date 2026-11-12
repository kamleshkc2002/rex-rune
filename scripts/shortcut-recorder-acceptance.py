#!/usr/bin/env python3
"""M4.4 native recorder acceptance on a private Rex server and Xvfb display.

Requires rex, Xvfb, openbox, xdotool and ImageMagick. REX_BIN selects the built
client. Reuses the X11 desktop wrapper, not its server or sessions. All server
calls have an explicit disposable Unix endpoint and autostart disabled.
Artifacts (including screenshots and workspace.json) stay in the printed folder.
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
binary = os.environ.get("REX_BIN", str(ROOT / "target/release/rex-linux"))
for tool in ("rex", "Xvfb", "openbox", "xdotool", "import"):
    assert shutil.which(tool), f"{tool} required"
work = Path(tempfile.mkdtemp(prefix="rex-recorder-"))
for name in ("home", "config", "state", "runtime", "cache", "captures"):
    (work / name).mkdir(mode=0o700)
for name in list(os.environ):
    if name.startswith(("REX_", "HERDR_")) or name in (
        "DBUS_SESSION_BUS_ADDRESS", "DISPLAY", "WAYLAND_DISPLAY"
    ):
        del os.environ[name]
endpoint = "unix://" + str(work / "rex.sock")
os.environ.update(
    HOME=str(work / "home"), XDG_CONFIG_HOME=str(work / "config"),
    XDG_STATE_HOME=str(work / "state"), XDG_RUNTIME_DIR=str(work / "runtime"),
    XDG_CACHE_HOME=str(work / "cache"), REX_BIN=binary, REX_SERVER=endpoint,
    REX_CAPTURE_OUT=str(work / "captures"),
)
spec = importlib.util.spec_from_file_location("accept", ROOT / "scripts/rune-acceptance-x11.py")
accept = importlib.util.module_from_spec(spec)
spec.loader.exec_module(accept)


def api(method, **payload):
    return json.loads(subprocess.check_output([
        "rex", "--autostart=false", "--server", endpoint, "api", "call",
        method, json.dumps(payload),
    ], text=True, timeout=12))


def wait(predicate, description):
    deadline = time.monotonic() + 12
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(.1)
    raise AssertionError(description)


accept.api = api
config = work / "init.lua"
config.write_text('rex.bind("ctrl+alt+j", "pane.copy")\n')
server = subprocess.Popen([
    "rex", "server", "run", "--listen", endpoint, "--restore=false",
    "--config", str(config), "--state-dir", str(work / "server-state"),
], stdout=(work / "server.log").open("w"), stderr=subprocess.STDOUT)
desk = None
try:
    print("Private recorder artifacts:", work, flush=True)
    wait(lambda: (work / "rex.sock").exists(), "private server did not start")
    sid, block = accept.session("recorder-private")
    workspace = work / "workspace.json"
    shortcuts = work / "workspace.shortcuts.json"
    contents = '[\n  {"keys":"ctrl+alt+j", "action":"pane.zoom"}\n]\n'
    shortcuts.write_text(contents)
    desk = accept.Desktop()
    app, window = desk.launch(sid, workspace, log=(work / "client.log").open("w"))

    def view():
        return api("session.view", session_id=sid)

    def tabs():
        return len(view()["windows"])

    def edit(title):
        desk.key("ctrl+shift+p")
        desk.type(title)
        desk.key("ctrl+Return")
        time.sleep(.4)

    # Cat makes any leaked printable input visible, in the original pane.
    desk.type("stty -ixon; cat -vT")
    desk.key("Return")
    before = accept.terminal_text(sid, block)
    edit("New tab")
    desk.type("LEAKCHECK")  # Plain keys are refused; no text may reach cat.
    desk.key("ctrl+alt+j")
    desk.capture(window, "recorder-conflict")
    desk.key("Return", "Escape")
    wait(lambda: workspace.exists() and "recorded_shortcuts" in json.loads(workspace.read_text()),
         "GUI layer was not saved")
    assert accept.terminal_text(sid, block) == before, "recording leaked terminal input"
    count = tabs()
    desk.key("ctrl+alt+j")
    wait(lambda: tabs() == count + 1, "GUI layer did not override file and server")
    desk.key("ctrl+shift+t")
    time.sleep(.4)
    assert tabs() == count + 1, "replaced default still creates a tab"
    assert shortcuts.read_text() == contents
    print("PASS native recording, input isolation, four-layer precedence and default removal", flush=True)

    # Relaunch attaches only; the recorded key still wins over the other layers.
    app.terminate()
    app.wait(timeout=10)
    app, window = desk.launch(sid, workspace, log=(work / "restart.log").open("w"))
    count = tabs()
    desk.key("ctrl+alt+j")
    wait(lambda: tabs() == count + 1, "recorded shortcut did not survive restart")
    edit("New tab")
    desk.key("ctrl+b", "n", "Return", "Escape")
    count = tabs()
    desk.key("ctrl+b", "n")
    wait(lambda: tabs() == count + 1, "recorded sequence did not run")
    edit("New tab")
    desk.key("BackSpace", "Escape")
    count = tabs()
    desk.key("ctrl+b", "n")
    time.sleep(.4)
    assert tabs() == count, "removed sequence still runs"
    assert shortcuts.read_text() == contents
    desk.capture(window, "recorder-live-finished")
    print("PASS restart, sequence recording and removal; shortcuts.json bytes preserved", flush=True)
except Exception:
    if desk and "window" in locals():
        desk.capture(window, "failure")
    raise
finally:
    if desk:
        desk.close()
    server.terminate()
    server.wait(timeout=10)
