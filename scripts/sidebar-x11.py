#!/usr/bin/env python3
"""Verify remembered sidebar state and render both tab layouts on private servers.

Requires rex, Xvfb, openbox, xdotool and ImageMagick's import. Set REX_BIN.
No daily sessions, display or settings are used.
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
BINARY = os.environ.get("REX_BIN", str(ROOT / "target/debug/rex-rune"))
SERVER = shutil.which("rex")
for tool in ("rex", "Xvfb", "openbox", "xdotool", "import"):
    assert shutil.which(tool), f"{tool} required"
work = Path(tempfile.mkdtemp(prefix="rex-sidebar-x11-"))
for name in ("home", "config", "state", "runtime", "cache", "captures"):
    (work / name).mkdir(mode=0o700)
for name in list(os.environ):
    if name.startswith(("REX_", "HERDR_")) or name in ("DBUS_SESSION_BUS_ADDRESS", "DISPLAY", "WAYLAND_DISPLAY"):
        del os.environ[name]
os.environ.update(HOME=str(work / "home"), XDG_CONFIG_HOME=str(work / "config"),
                  XDG_STATE_HOME=str(work / "state"), XDG_RUNTIME_DIR=str(work / "runtime"),
                  XDG_CACHE_HOME=str(work / "cache"), REX_BIN=BINARY,
                  REX_CAPTURE_OUT=str(work / "captures"), REX_SERVER="unix://" + str(work / "rex-0.sock"))
spec = importlib.util.spec_from_file_location("accept", ROOT / "scripts/rune-acceptance-x11.py")
accept = importlib.util.module_from_spec(spec)
spec.loader.exec_module(accept)


def api(method, server=0, **payload):
    return json.loads(subprocess.check_output([SERVER, "--autostart=false", "--server", "unix://" + str(work / f"rex-{server}.sock"),
        "api", "call", method, json.dumps(payload)], text=True, timeout=12))


def wait(predicate, description, timeout=10):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(.08)
    raise AssertionError(description)


servers = []
desk = None
try:
    print("Private sidebar artifacts:", work, flush=True)
    for i in range(2):
        servers.append(subprocess.Popen([SERVER, "server", "run", "--listen", "unix://" + str(work / f"rex-{i}.sock"),
            "--restore=false", "--config", "/dev/null", "--state-dir", str(work / f"server-{i}")],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL))
        wait(lambda: (work / f"rex-{i}.sock").exists(), "private server did not start")
    sessions = []
    for i, label in enumerate(("work", "remote-check")):
        sid = api("session.create", server=i, label=label, initial_windows=[
            {"window_label": label, "layout": accept.SHELL} for _ in range(3 - i)])["session_id"]
        sessions.append(sid)
    sid = sessions[0]
    view = api("session.view", session_id=sid)
    block = next(w["focused_block_id"] for w in view["windows"] if w["window_id"] == view["active_window_id"])
    config = work / "workspace.json"
    config.write_text(json.dumps({"hosts": [
        {"label": "Elphael-private", "endpoint": "unix://" + str(work / "rex-0.sock")},
        {"label": "Erdtree-private", "endpoint": "unix://" + str(work / "rex-1.sock")}],
        "sidebar_open": True, "vertical_tabs": True, "sidebar_width": 280}))
    desk = accept.Desktop()
    log = (work / "client.log").open("w")
    app, window = desk.launch(sid, config, log=log)
    columns = lambda: api("com.superlogical.terminal.size", session_id=sid, block_id=block)["columns"]
    wait(lambda: columns() > 80, "initial terminal size was not applied")
    initial = columns()
    desk.capture(window, "vertical-tabs")
    # The stable sidebar toggle is the first toolbar control.
    desk.x("mousemove", "--window", window, 24, 26)
    desk.x("click", 1)
    wait(lambda: not json.loads(config.read_text())["sidebar_open"], "closing sidebar was not saved")
    wait(lambda: columns() > initial + 15, "closing sidebar did not expand the terminal")
    app.terminate(); app.wait(timeout=10)
    app, window = desk.launch(sid, config, log=log)
    wait(lambda: columns() > initial + 15, "reopened client forgot the closed sidebar")
    desk.capture(window, "reopened-closed")
    desk.x("mousemove", "--window", window, 24, 26)
    desk.x("click", 1)
    wait(lambda: json.loads(config.read_text())["sidebar_open"], "opening sidebar was not saved")
    app.terminate(); app.wait(timeout=10)
    app, window = desk.launch(sid, config, log=log)
    wait(lambda: columns() <= initial + 1, "reopened client forgot the open sidebar")
    desk.capture(window, "reopened-open")
    app.terminate(); app.wait(timeout=10)
    saved = json.loads(config.read_text()); saved["vertical_tabs"] = False
    config.write_text(json.dumps(saved))
    app, window = desk.launch(sid, config, log=log)
    desk.capture(window, "top-tabs")
    assert json.loads(config.read_text())["sidebar_open"]
    print("PASS native sidebar open/closed persistence and both tab layouts", flush=True)
finally:
    if desk:
        desk.close()
    for server in servers:
        server.terminate(); server.wait(timeout=8)
