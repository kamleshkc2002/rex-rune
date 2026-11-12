#!/usr/bin/env python3
"""Check global tab shortcuts and mouse history on a private Rex server/Xvfb.

Requires rex, Xvfb, openbox and xdotool. Set REX_BIN to the client wrapper.
No daily server, display or configuration is used. Adapted from joegoldin's check.
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
work = Path(tempfile.mkdtemp(prefix="rex-navigation-x11-"))
for name in ("home", "config", "state", "runtime", "cache", "captures"):
    (work / name).mkdir(mode=0o700)
for name in list(os.environ):
    if name.startswith(("REX_", "HERDR_")) or name in ("DBUS_SESSION_BUS_ADDRESS", "DISPLAY", "WAYLAND_DISPLAY"):
        del os.environ[name]
os.environ.update(HOME=str(work / "home"), XDG_CONFIG_HOME=str(work / "config"),
                  XDG_STATE_HOME=str(work / "state"), XDG_RUNTIME_DIR=str(work / "runtime"),
                  XDG_CACHE_HOME=str(work / "cache"), REX_BIN=BINARY,
                  REX_CAPTURE_OUT=str(work / "captures"), REX_SERVER="unix://" + str(work / "rex.sock"))
spec = importlib.util.spec_from_file_location("accept", ROOT / "scripts/rune-acceptance-x11.py")
accept = importlib.util.module_from_spec(spec)
spec.loader.exec_module(accept)


def api(method, **payload):
    return json.loads(subprocess.check_output([SERVER, "--autostart=false", "--server", os.environ["REX_SERVER"],
        "api", "call", method, json.dumps(payload)], text=True, timeout=12))


def wait(predicate, description, timeout=10):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(.08)
    raise AssertionError(description)


def selected(sid, wid, label):
    return (api("session.view", session_id=sid)["active_window_id"] == wid
            and desk.x("getwindowname", window).startswith(label))


server = subprocess.Popen([SERVER, "server", "run", "--listen", os.environ["REX_SERVER"], "--restore=false",
                           "--config", "/dev/null", "--state-dir", str(work / "server-state")],
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
desk = None
try:
    print("Private navigation artifacts:", work, flush=True)
    wait(lambda: (work / "rex.sock").exists(), "private server did not start")
    sessions = []
    for label in ("nav-A", "nav-B"):
        sid = api("session.create", label=label, initial_windows=[
            {"window_label": "first", "layout": accept.SHELL},
            {"window_label": "second", "layout": accept.SHELL}])["session_id"]
        windows = [w["window_id"] for w in api("session.view", session_id=sid)["windows"]]
        sessions.append((sid, windows, label))
    (a, aw, al), (b, bw, bl) = sessions
    print("Fixtures:", sessions, flush=True)
    # An uncached session is one stop and keeps its server-side active tab.
    for sid, windows, _ in sessions:
        api("session.focus_window", session_id=sid, window_id=windows[0])
    desk = accept.Desktop()
    log = (work / "client.log").open("w")
    app, window = desk.launch(a, work / "workspace.json", log=log)
    wait(lambda: selected(a, aw[0], al), "initial tab was not selected")
    for key, sid, wid, label in [
        ("ctrl+Tab", a, aw[1], al),
        ("ctrl+Tab", b, bw[0], bl),
        ("ctrl+Tab", b, bw[1], bl),
        ("ctrl+Tab", a, aw[0], al),
        ("ctrl+shift+Tab", b, bw[1], bl),
    ]:
        print("Navigate:", key, label, wid, flush=True)
        desk.key(key)
        wait(lambda: selected(sid, wid, label), f"{key} did not reach {label}/{wid}")
    desk.x("mousemove", "--window", window, 600, 350)
    desk.x("click", 8)
    wait(lambda: selected(a, aw[0], al), "mouse Back did not cross the session boundary")
    desk.x("click", 9)
    wait(lambda: selected(b, bw[1], bl), "mouse Forward did not restore the later session tab")
    desk.capture(window, "global-navigation")
    desk.key("ctrl+shift+p")
    desk.type("history")
    desk.capture(window, "history-commands")
    desk.key("Return")
    wait(lambda: selected(a, aw[0], al), "Commands Back did not use the same history")
    desk.key("ctrl+shift+p")
    desk.type("Forward in history")
    desk.key("Return")
    wait(lambda: selected(b, bw[1], bl), "Commands Forward did not use the same history")
    print("PASS global Ctrl+Tab, reverse wrap, mouse Back/Forward and named Commands across sessions", flush=True)
except Exception:
    if desk and 'window' in locals():
        desk.capture(window, "failure")
        print("Window:", desk.x("getwindowname", window), flush=True)
        for sid, _, label in sessions:
            print(label, api("session.view", session_id=sid)["active_window_id"], flush=True)
    raise
finally:
    if desk:
        desk.close()
    server.terminate()
    server.wait(timeout=8)
