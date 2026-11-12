#!/usr/bin/env python3
"""Header-fading acceptance on private Rex/Xvfb; never personal server/display/clipboard.

Requires rex API 13, Xvfb, openbox, xdotool, xmessage, ImageMagick and Pillow.
REX_BIN selects an already-built client. Evidence remains in ~/.cache/agent-ws.
Native focus-in retains Rex's existing visible-pane ownership claim policy.
"""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
from PIL import Image, ImageChops

ROOT = Path(__file__).resolve().parents[1]
BINARY = os.environ.get("REX_BIN", str(ROOT / "target/release/rex-rune"))
work = Path(tempfile.mkdtemp(prefix="rex-header-fading-", dir=Path.home() / ".cache/agent-ws"))
for tool in ("rex", "Xvfb", "openbox", "xdotool", "xmessage", "import"):
    assert shutil.which(tool), f"{tool} required"
for name in ("home", "config", "state", "runtime", "cache", "captures"):
    (work / name).mkdir(mode=0o700)
for name in list(os.environ):
    if name.startswith(("REX_", "HERDR_")) or name in ("DISPLAY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS"):
        del os.environ[name]
os.environ.update(HOME=str(work / "home"), XDG_CONFIG_HOME=str(work / "config"),
                  XDG_STATE_HOME=str(work / "state"), XDG_RUNTIME_DIR=str(work / "runtime"),
                  XDG_CACHE_HOME=str(work / "cache"), REX_BIN=BINARY,
                  REX_CAPTURE_OUT=str(work / "captures"), REX_SERVER="unix://" + str(work / "rex.sock"))
spec = importlib.util.spec_from_file_location("accept", ROOT / "scripts/rune-acceptance-x11.py")
accept = importlib.util.module_from_spec(spec)
spec.loader.exec_module(accept)


def wait(predicate, description, timeout=15):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        if predicate():
            return
        time.sleep(.08)
    raise AssertionError(description)


server = subprocess.Popen(["rex", "server", "run", "--listen", os.environ["REX_SERVER"],
                           "--restore=false", "--config", "/dev/null", "--state-dir", str(work / "server")],
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
desk = other = None
try:
    print("Artifacts:", work, flush=True)
    wait(lambda: (work / "rex.sock").exists(), "server did not start")
    sid, block = accept.session("Private header fading")
    desk = accept.Desktop()
    config = work / "workspace.json"
    config.write_text(json.dumps({"sidebar_open": False, "font_family": "IBM Plex Mono",
                                 "pane_header": "always", "pane_padding": 8,
                                 "interface": {"fade_pane_headers_in_inactive_window": True}}))
    log = (work / "client.log").open("w")
    app, window = desk.launch(sid, config, size="1024x700", log=log)
    ready, stop, received = work / "ready", work / "stop", work / "input.bin"
    program = work / "record.py"
    program.write_text("import os,tty,termios,select,time\n"
                       "old=termios.tcgetattr(0); tty.setraw(0)\n"
                       "print('\\x1b[2J\\x1b[H\\x1b[?25l\\x1b[31mred output\\x1b[0m unchanged',end='',flush=True)\n"
                       + f"open({str(ready)!r},'w').close()\n"
                       "data=b''; until=time.monotonic()+120\n"
                       + f"while time.monotonic()<until and not os.path.exists({str(stop)!r}):\n"
                       " if select.select([0],[],[],.05)[0]: data+=os.read(0,4096)\n"
                       "termios.tcsetattr(0,termios.TCSANOW,old)\n"
                       + f"open({str(received)!r},'wb').write(data)\n")
    accept.api("block.event", session_id=sid, block_id=block,
               event={"type": "paste", "payload": {"text": "python3 " + str(program)}})
    desk.key("Return")
    wait(ready.exists, "recorder did not start")
    size = lambda: accept.api("com.superlogical.terminal.size", session_id=sid, block_id=block)
    initial = size()
    other = subprocess.Popen(["xmessage", "-title", "rex-header-focus-target", "-geometry", "200x100+1200+800", "private focus target"],
                             env=desk.env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    # Xaw's xmessage does not publish _NET_WM_PID; this display is ours alone.
    other_window = desk.x("search", "--sync", "--name", "^rex-header-focus-target$")
    for fading in (True, False):
        desk.x("windowfocus", window)
        time.sleep(.5)
        if not fading:
            # Actual-label Settings search, then toggle the centered row's existing-kit switch.
            desk.key("ctrl+comma")
            desk.x("mousemove", "--window", window, 200, 92)
            desk.x("click", 1)
            desk.type("Fade headers in inactive windows")
            desk.key("Return")
            time.sleep(2.8)
            desk.capture(window, "settings-native")
            desk.x("mousemove", "--window", window, 864, 406)
            desk.x("click", 1)
            wait(lambda: not json.loads(config.read_text())["interface"]["fade_pane_headers_in_inactive_window"],
                 "Settings switch did not save")
            desk.key("Escape", "Escape")
        prefix = "on" if fading else "off"
        desk.capture(window, prefix + "-focused")
        desk.x("windowfocus", other_window)
        time.sleep(.5)
        desk.capture(window, prefix + "-unfocused")
        desk.x("windowfocus", window)
        time.sleep(.5)
        desk.capture(window, prefix + "-refocused")
        pictures = [Image.open(work / "captures" / f"{prefix}-{state}.png").convert("RGB")
                    for state in ("focused", "unfocused", "refocused")]
        # Card body excludes the pre-existing active edge and shadow. Cursor hidden by PTY.
        body = (20, 95, 1000, 680)
        header = (24, 55, 1000, 80)
        for picture in pictures[1:]:
            assert ImageChops.difference(pictures[0].crop(body), picture.crop(body)).getbbox() is None
        changed = ImageChops.difference(pictures[0].crop(header), pictures[1].crop(header)).getbbox()
        assert bool(changed) == fading, (prefix, changed)
        assert ImageChops.difference(pictures[0].crop(header), pictures[2].crop(header)).getbbox() is None
        now = size()
        for key in ("columns", "rows", "owner"):
            assert now[key] == initial[key], (key, initial, now)
        view = accept.api("session.view", session_id=sid)
        assert view["windows"][0]["focused_block_id"] == block
        print(f"PASS fade={fading}: native focus out/in, header pixels, identical terminal body, same size/owner/focused pane", flush=True)
    desk.key("x")
    stop.touch()
    wait(received.exists, "recorder did not finish")
    assert received.read_bytes() == b"x", received.read_bytes()
    print("PASS no leaked Settings or focus input; explicit x reaches the PTY", flush=True)
finally:
    if other:
        other.terminate()
        other.wait(timeout=5)
    if desk:
        desk.close()
    server.terminate()
    server.wait(timeout=5)
