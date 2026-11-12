#!/usr/bin/env python3
"""Check terminal OSC 52 copies against an independent OS clipboard reader.

Starts a disposable server, HOME and Xvfb; never reads or writes your desktop's
clipboard. Requires rex, Xvfb, openbox, xdotool and xclip; set REX_BIN to the client.
REX_SERVER is set for you to the private server; any inherited value is discarded.
"""
import importlib.util
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
BINARY = os.environ.get("REX_BIN", str(ROOT / "target/debug/rex-rune"))
SERVER = shutil.which("rex")
assert SERVER, "rex server binary required"
for tool in ("Xvfb", "openbox", "xdotool", "xclip"):
    assert shutil.which(tool), f"{tool} required"
work = Path(tempfile.mkdtemp(prefix="rex-clipboard-x11-"))
for name in ("home", "config", "state", "runtime", "cache"):
    (work / name).mkdir(mode=0o700)
for name in list(os.environ):
    if name.startswith(("REX_", "HERDR_")) or name in ("DBUS_SESSION_BUS_ADDRESS", "DISPLAY", "WAYLAND_DISPLAY"):
        del os.environ[name]
os.environ.update(HOME=str(work / "home"), XDG_CONFIG_HOME=str(work / "config"),
                  XDG_STATE_HOME=str(work / "state"), XDG_RUNTIME_DIR=str(work / "runtime"),
                  XDG_CACHE_HOME=str(work / "cache"), REX_BIN=BINARY,
                  REX_SERVER="unix://" + str(work / "rex.sock"))
spec = importlib.util.spec_from_file_location("accept", ROOT / "scripts/rune-acceptance-x11.py")
accept = importlib.util.module_from_spec(spec)
spec.loader.exec_module(accept)


def wait(predicate, description, timeout=10):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(.08)
    raise AssertionError(description)


def clipboard():
    return subprocess.check_output(["xclip", "-selection", "clipboard", "-o"], env=desk.env, timeout=3)


def emit(expression, target=None):
    # Only synthetic data; generate large copies in the private shell, not in a control frame.
    program = "import base64,sys; t=" + expression + "; sys.stdout.write('\\x1b]52;c;'+base64.b64encode(t.encode()).decode()+'\\x07')"
    accept.api("block.event", session_id=sid, block_id=target or block,
               event={"type": "paste", "payload": {"text": "python3 -c " + shlex.quote(program)}})
    if target is None:
        desk.key("Return")
    else:
        accept.api("block.event", session_id=sid, block_id=target,
                   event={"type": "key", "payload": {"key": "enter", "action": "press", "mods": ""}})


server = subprocess.Popen([SERVER, "server", "run", "--listen", os.environ["REX_SERVER"], "--restore=false",
                           "--config", "/dev/null", "--state-dir", str(work / "server-state")],
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
desk = None
try:
    wait(lambda: (work / "rex.sock").exists(), "private server did not start")
    sid, block = accept.session("clipboard-private")
    desk = accept.Desktop()
    log = (work / "client.log").open("w")
    config = accept.pin_layout(work / "workspace.json")
    app, window = desk.launch(sid, config, log=log)
    subprocess.run(["xclip", "-selection", "clipboard"], input=b"unchanged clipboard sentinel", env=desk.env,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True, timeout=3)
    emit(repr("Pi copy: 世界 🦀\nsecond line"))
    expected = "Pi copy: 世界 🦀\nsecond line".encode()
    wait(lambda: clipboard() == expected, "OSC 52 copy never reached the system clipboard")
    print("PASS terminal/Pi OSC 52 updates the OS clipboard (Unicode and newlines)", flush=True)
    expected = ("large 世界 🦀\n" * 9000).encode()
    emit("'large 世界 🦀\\n' * 9000")
    wait(lambda: clipboard() == expected, "large OSC 52 copy was missing or truncated", timeout=15)
    print("PASS large OSC 52 copy is fetched completely across UTF-8 chunk boundaries", flush=True)
    # A hidden tab must not overwrite the clipboard just because attach_all hears its event.
    accept.api("session.new_window", session_id=sid, window_label="hidden", layout=accept.SHELL, focus=False)
    view = accept.api("session.view", session_id=sid)
    hidden = next(w["focused_block_id"] for w in view["windows"] if w["focused_block_id"] != block)
    emit(repr("background must not overwrite clipboard"), hidden)
    time.sleep(.8)
    assert clipboard() == expected, "background terminal overwrote the clipboard"
    print("PASS hidden-terminal copy does not hijack the system clipboard", flush=True)
    accept.api("block.event", session_id=sid, block_id=block, event={"type": "paste", "payload": {
        "text": "printf '\\033[2J\\033[Halpha beta-gamma 世界\\n'"}})
    desk.key("Return")
    time.sleep(.6)
    # "alpha beta-gamma": the word starts at column 6; click inside its third letter.
    desk.x("mousemove", "--window", window, *accept.cell_point(8, 0))
    desk.x("click", "--repeat", "2", "--delay", "100", "1")
    desk.key("ctrl+shift+c")
    wait(lambda: clipboard() == b"beta-gamma", "double-click did not select and copy the terminal word")
    print("PASS native double-click selects and copies a complete terminal word", flush=True)
    time.sleep(.6)
    desk.x("click", "1")
    desk.key("ctrl+shift+c")
    assert clipboard() == b"beta-gamma", "a simple focus click created a spurious one-character selection"
    print("PASS a focus click does not create a one-character selection", flush=True)
    print("Artifacts:", work, flush=True)
finally:
    if desk:
        desk.close()
    server.terminate()
    server.wait(timeout=8)
