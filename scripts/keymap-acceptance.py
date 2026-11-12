#!/usr/bin/env python3
"""Phase 6 live acceptance: native keys, private Rex/Xvfb, no daily hosts.

Requires rex, Xvfb, openbox, xdotool, ImageMagick. REX_BIN selects the built
client (default target/release/rex-rune). Artifacts stay in the printed temp
folder. Starts/stops only its own processes; all API calls explicitly use the
private Unix endpoint with autostart disabled.
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
BINARY = os.environ.get("REX_BIN", str(ROOT / "target/release/rex-rune"))
for tool in ("rex", "Xvfb", "openbox", "xdotool", "import"):
    assert shutil.which(tool), f"{tool} required"
work = Path(tempfile.mkdtemp(prefix="rex-keymap-acceptance-"))
for name in ("home", "config", "state", "runtime", "cache", "captures"):
    (work / name).mkdir(mode=0o700)
for name in list(os.environ):
    if name.startswith(("REX_", "HERDR_")) or name in ("DBUS_SESSION_BUS_ADDRESS", "DISPLAY", "WAYLAND_DISPLAY"):
        del os.environ[name]
endpoint = "unix://" + str(work / "rex.sock")
os.environ.update(HOME=str(work / "home"), XDG_CONFIG_HOME=str(work / "config"),
                  XDG_STATE_HOME=str(work / "state"), XDG_RUNTIME_DIR=str(work / "runtime"),
                  XDG_CACHE_HOME=str(work / "cache"), REX_BIN=BINARY,
                  REX_CAPTURE_OUT=str(work / "captures"), REX_SERVER=endpoint)
spec = importlib.util.spec_from_file_location("accept", ROOT / "scripts/rune-acceptance-x11.py")
accept = importlib.util.module_from_spec(spec)
spec.loader.exec_module(accept)


def api(method, **payload):
    return json.loads(subprocess.check_output(["rex", "--autostart=false", "--server", endpoint,
        "api", "call", method, json.dumps(payload)], text=True, timeout=12))


# Reuse only the private display wrapper. Its API helper gets explicit routing too.
accept.api = api


def wait(predicate, description, timeout=10):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(.08)
    raise AssertionError(description)


config = work / "init.lua"
config.write_text('''rex.bind("ctrl+j", "client.tab.new")
rex.bind("ctrl+b>c", "client.tab.new")
rex.unbind("ctrl+shift+t")
rex.bind("ctrl+shift+l", function(ctx, ev)
  rex.call("session.set_label", {session_id=ctx.session_id, label="lua-direct"})
  return true
end)
rex.bind("ctrl+x>ctrl+s", function(ctx, ev)
  rex.call("session.set_label", {session_id=ctx.session_id, label="lua-sequence"})
  return true
end)
rex.action({name="phase6_action", title="Phase 6 action", run=function(ctx)
  rex.client.queue("client.tab.new")
end})
rex.bind("ctrl+shift+y", "phase6_action")
''')
subprocess.run(["rex", "--autostart=false", "config", "check", str(config)], check=True)
server = subprocess.Popen(["rex", "server", "run", "--listen", endpoint, "--restore=false",
    "--config", str(config), "--state-dir", str(work / "server-state")],
    stdout=(work / "server.log").open("w"), stderr=subprocess.STDOUT)
desk = None
try:
    print("Private Phase 6 artifacts:", work, flush=True)
    wait(lambda: (work / "rex.sock").exists(), "server did not start")
    sid, block = accept.session("phase6-private")
    workspace = work / "workspace.json"
    shortcuts = work / "workspace.shortcuts.json"
    contents = json.dumps({"shortcuts":[{"keys":"ctrl+tab","action":None},
        {"keys":"ctrl+shift+tab","action":None}]})
    shortcuts.write_text(contents)
    desk = accept.Desktop()
    app, window = desk.launch(sid, workspace, log=(work / "client.log").open("w"))

    def view():
        return api("session.view", session_id=sid)

    def tabs():
        return len(view()["windows"])

    def terminal_key(stroke, expected):
        current = view()
        focused = next(w for w in current["windows"]
                       if w["window_id"] == current["active_window_id"])["focused_block_id"]
        # Disambiguation makes modified Tab/T observable even when legacy encoding
        # has no bytes for the chord. cat prints the actual Kitty event from the PTY.
        desk.type(r"stty -ixon; printf '\033[>1u'; cat -vT")
        desk.key("Return", stroke, "Return")
        wait(lambda: expected in accept.terminal_text(sid, focused),
             f"{stroke} did not reach the terminal as {expected}")
        desk.key("ctrl+c")
        desk.type(r"printf '\033[<u'")
        desk.key("Return")

    def new_tab(strokes, description):
        before = tabs()
        desk.key(*strokes)
        wait(lambda: tabs() == before + 1, description)
        print("PASS", description, flush=True)

    new_tab(["ctrl+j"], "config-file client action")
    new_tab(["ctrl+b", "c"], "config-file client sequence")
    count = tabs()
    terminal_key("ctrl+shift+t", "^[[116;6u")
    assert tabs() == count, "server unbind did not remove default New tab"
    print("PASS server unbind reaches terminal as Kitty Ctrl Shift T", flush=True)
    desk.key("ctrl+shift+l")
    wait(lambda: view()["label"] == "lua-direct", "Lua direct function was bypassed")
    desk.key("ctrl+x", "ctrl+s")
    wait(lambda: view()["label"] == "lua-sequence", "Lua sequence prefix was lost")
    print("PASS direct and sequence Lua forwarding", flush=True)
    new_tab(["ctrl+shift+y"], "server action and queued client action exactly once")
    time.sleep(.5)
    count = tabs()
    time.sleep(.5)
    assert tabs() == count, "queued client action replayed"

    active = view()["active_window_id"]
    terminal_key("ctrl+Tab", "^[[9;5u")
    assert view()["active_window_id"] == active, "unbound Ctrl Tab still cycled"
    print("PASS client override removes Ctrl Tab and reaches terminal as Kitty Ctrl Tab", flush=True)

    # Settings reloads a hand edit; preference saves must not overwrite it.
    contents = json.dumps({"shortcuts":[{"keys":"ctrl+q","action":"client.tab.new"},
        {"keys":"ctrl+j","action":None},
        {"keys":"ctrl+b>ctrl+b","action":"pane.send_key","args":{"key":"ctrl+b"}}]})
    shortcuts.write_text(contents)
    # Settings opening is an independent reload trigger and doesn't need another window.
    desk.key("ctrl+comma", "Escape")
    new_tab(["ctrl+q"], "hand-edited overrides reload from Settings")
    count = tabs()
    desk.key("ctrl+j")
    time.sleep(.4)
    assert tabs() == count, "client null did not override server binding"
    desk.type("cat -vT")
    desk.key("Return", "ctrl+b", "ctrl+b", "Return")
    focused = next(w for w in view()["windows"] if w["window_id"] == view()["active_window_id"])["focused_block_id"]
    wait(lambda: "^B" in accept.terminal_text(sid, focused), "explicit send-key did not reach PTY")
    desk.key("ctrl+c")
    print("PASS client override precedence and pane.send_key PTY escape hatch", flush=True)
    assert shortcuts.read_text() == contents

    config.write_text(config.read_text() + '\nrex.bind("ctrl+u", "client.tab.new")\n')
    subprocess.run(["rex", "--autostart=false", "--server", endpoint, "config", "reload"], check=True)
    time.sleep(.5)
    new_tab(["ctrl+u"], "keymap_changed reloads server bindings")

    clients = api("client.list")["clients"]
    (work / "clients.json").write_text(json.dumps(clients, indent=2))
    # The persistent app client is the one that declared the registry, not this CLI probe.
    found = False
    for client in clients:
        if client.get("info", {}).get("kind") != "app":
            continue
        cid = client.get("id", client.get("client_id"))
        if not cid:
            continue
        inspected = api("client.inspect", client_id=cid)
        names = {a["name"] for a in inspected.get("actions", [])}
        if "client.tab.new" in names and "pane.send_key" in names:
            found = True
            (work / "registry.json").write_text(json.dumps(inspected, indent=2))
            count = tabs()
            for _ in range(2):
                declined = subprocess.run(["rex", "--autostart=false", "--server", endpoint,
                    "api", "call", "client.perform", json.dumps({"client_id": cid,
                    "action": "client.tab.new", "timeout_ms": 2000})],
                    capture_output=True, text=True, timeout=5)
                assert declined.returncode != 0, "remote action unexpectedly succeeded"
                assert "Use rex window new instead" in declined.stderr, declined.stderr
                assert "client.tab.new" in declined.stderr, declined.stderr
                time.sleep(.5)
                assert tabs() == count, "declined action changed the workspace"
            print("PASS remote client actions explicitly declined without timeout or execution", flush=True)
    assert found, "registered client actions not inspectable"
    desk.capture(window, "phase6-live")
    print("PASS action registry advertised; shortcut bytes preserved", flush=True)
    print("PASS Phase 6 live keymap acceptance", flush=True)
except Exception:
    if desk and 'window' in locals():
        desk.capture(window, "failure")
    raise
finally:
    if desk:
        desk.close()
    server.terminate()
    server.wait(timeout=10)
