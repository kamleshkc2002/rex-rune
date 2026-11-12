#!/usr/bin/env python3
"""Kitty graphics pixel regression on a disposable Rex server and Xvfb.

Requires rex, Xvfb, openbox, xdotool and ImageMagick. Set REX_BIN to the client.
Checks live RGBA/PNG images, fresh-client snapshot restoration and deletion.
Never connects to your desktop display, settings, clipboard or running server.
"""
import base64
import importlib.util
import json
import os
from pathlib import Path
import shlex
import shutil
import struct
import subprocess
import tempfile
import time
import zlib

ROOT = Path(__file__).resolve().parents[1]
BINARY = os.environ.get("REX_BIN", str(ROOT / "target/debug/rex-rune"))
SERVER = shutil.which("rex")
for tool in ("rex", "Xvfb", "openbox", "xdotool", "import", "convert"):
    assert shutil.which(tool), f"{tool} required"
work = Path(tempfile.mkdtemp(prefix="rex-kitty-graphics-"))
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


def emit(data):
    program = "import base64,os; os.write(1,base64.b64decode(" + repr(base64.b64encode(data).decode()) + "))"
    api("block.event", session_id=sid, block_id=block,
        event={"type": "paste", "payload": {"text": "python3 -c " + shlex.quote(program)}})
    desk.key("Return")
    time.sleep(.5)


def capture(name, channel):
    desk.capture(window, name)
    data = subprocess.check_output(["convert", str(work / "captures" / (name + ".png")), "-depth", "8", "rgb:-"])
    count = sum(1 for i in range(0, len(data), 3) if data[i + channel] > 245
                and all(data[i + other] < 12 for other in range(3) if other != channel))
    print(name, "solid-color pixels:", count, flush=True)
    return count


def png():
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 2, 2, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress((b"\0" + bytes([0, 255, 0, 255]) * 2) * 2)) + chunk(b"IEND", b""))


server = subprocess.Popen([SERVER, "server", "run", "--listen", os.environ["REX_SERVER"], "--restore=false",
                           "--config", "/dev/null", "--state-dir", str(work / "server-state")],
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
desk = None
try:
    print("Private graphics artifacts:", work, flush=True)
    wait(lambda: (work / "rex.sock").exists(), "private server did not start")
    sid = api("session.create", label="kitty-private", initial_windows=[{"window_label": "Shell", "layout": accept.SHELL}])["session_id"]
    block = api("session.view", session_id=sid)["windows"][0]["focused_block_id"]
    desk = accept.Desktop()
    log = (work / "client.log").open("w")
    config = work / "workspace.json"
    app, window = desk.launch(sid, config, log=log)
    red = base64.b64encode(bytes([255, 0, 0, 255]) * 4)
    emit(b"\x1b[2J\x1b[H\x1b_Ga=T,f=32,s=2,v=2,c=12,r=6,i=42,q=2;" + red + b"\x1b\\\r\n")
    assert capture("live-rgba", 0) > 2000, "live Kitty RGBA image did not render"
    app.terminate()
    app.wait(timeout=10)
    app, window = desk.launch(sid, config, log=log)
    assert capture("reattached-rgba", 0) > 2000, "Kitty image did not survive fresh-client snapshot restore"
    green = base64.b64encode(png())
    # Split a PNG transmission in the middle of its base64 stream, as real programs do.
    emit(b"\x1b[2J\x1b[H\x1b_Ga=d,d=A,q=2\x1b\\"
         + b"\x1b_Ga=T,f=100,c=12,r=6,i=43,q=2,m=1;" + green[:40] + b"\x1b\\"
         + b"\x1b_Gm=0;" + green[40:] + b"\x1b\\\r\n")
    assert capture("live-png", 1) > 2000, "chunked Kitty PNG image did not render"
    emit(b"\x1b_Ga=d,d=A,q=2\x1b\\")
    assert capture("deleted-png", 1) < 100, "deleted image left stale pixels"
    print("PASS live Kitty RGBA/PNG pixels, chunked transmission, snapshot restoration and deletion", flush=True)
finally:
    if desk:
        desk.close()
    server.terminate()
    server.wait(timeout=8)
