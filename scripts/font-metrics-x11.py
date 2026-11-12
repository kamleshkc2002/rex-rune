#!/usr/bin/env python3
"""M2.6 live metrics and size-ownership acceptance on a private Rex server/Xvfb.

Requires rex (API 13), Xvfb, openbox, xdotool and ImageMagick import. REX_BIN selects
an already-built client. No inherited server, display, clipboard, HOME or XDG state
is used. Creates one disposable session; artifacts stay in ~/.cache/agent-ws.
Reuses the private-desktop and observable control helpers, not their test runs.
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
work = Path(tempfile.mkdtemp(prefix="rex-m26-x11-", dir=Path.home() / ".cache/agent-ws"))
for tool in ("rex", "Xvfb", "openbox", "xdotool", "import"):
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


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


accept = module("accept", ROOT / "scripts/rune-acceptance-x11.py")
contract = module("contract", ROOT / "scripts/download-contract.py")
contract.SOCK = str(work / "rex.sock")


def wait(predicate, description, timeout=12):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(.08)
    raise AssertionError(description)


def click(x, y):
    desk.x("mousemove", "--window", window, x, y)
    desk.x("click", 1)
    time.sleep(.2)


def pref():
    return json.loads(config.read_text()).get("terminal", {}).get("font", {})


def size():
    return accept.api("com.superlogical.terminal.size", session_id=sid, block_id=block)


def advanced():
    desk.key("ctrl+comma")
    click(210, 92)  # Search Settings, in the pinned 1024×700 window.
    desk.type("Advanced font settings")
    desk.key("Return")
    time.sleep(.3)


server = subprocess.Popen(["rex", "server", "run", "--listen", os.environ["REX_SERVER"],
                           "--restore=false", "--config", "/dev/null", "--state-dir", str(work / "server")],
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
desk = peer = None
try:
    print("Artifacts:", work, flush=True)
    wait(lambda: (work / "rex.sock").exists(), "private server did not start")
    sid, block = accept.session("M2.6 private metrics")
    desk = accept.Desktop()
    config = accept.pin_layout(work / "workspace.json", sidebar_open=False, font_family="IBM Plex Mono",
                              terminal={"cursor_style": "bar", "cursor_force": True})
    log = (work / "client.log").open("w")
    app, window = desk.launch(sid, config, size="1024x700", log=log)
    wait(lambda: size()["columns"] == 112, "default IBM Plex grid was not 112 columns")
    plain = size()
    assert plain["rows"] == 30, plain
    print("PASS default grid", plain, flush=True)
    advanced()
    desk.capture(window, "advanced-before")
    click(876, 259)  # Line height +; Return keeps using the focused control as it moves.
    desk.key("Return", "Return", "Return")
    wait(lambda: pref().get("line_height") == 20, "line height control did not save +20%")
    wait(lambda: size()["rows"] == 24, "owned terminal did not resize to 24 rows")
    tall = size()
    assert tall["columns"] == plain["columns"], tall
    print("PASS live +20% height changes owned rows only", tall, flush=True)
    # Open the unit popup, now 15 pt lower because the preview has taller rows.
    click(858, 372)
    desk.key("Down", "Return")
    wait(lambda: pref().get("spacing_unit") == "percent", "spacing unit did not become Percent")
    click(876, 323)
    desk.key("Return", "Return", "Return")
    wait(lambda: pref().get("spacing") == 20, "spacing control did not save +20%")
    wait(lambda: size()["columns"] == 93, "owned terminal did not resize to 93 columns")
    wide = size()
    assert wide["rows"] == 24, wide
    desk.capture(window, "advanced-metrics-20")
    print("PASS live +20% spacing changes owned columns", wide, flush=True)

    # A second, persistent control client takes ownership. The window never loses/gains
    # focus, and Settings input is not terminal typing: changing a preference cannot claim it.
    peer = contract.Control(name="m26-other-owner", log_requests=False)
    for method, payload in [
        ("session.attach", {"session_id": sid}),
        ("session.own_block", {"session_id": sid, "block_id": block}),
    ]:
        reply = peer.call(method, payload)
        assert "error" not in reply, reply
    time.sleep(.3)
    unowned = size()
    assert unowned["owner"] != wide["owner"], "peer did not take ownership"
    click(876, 274)  # Line height + to 25%, while another client owns the pane.
    wait(lambda: pref().get("line_height") == 25, "unowned local preference did not save")
    time.sleep(.6)
    assert size() == unowned, (size(), unowned)
    print("PASS unowned preference changes do not resize or reclaim ownership", flush=True)
    desk.capture(window, "advanced-unowned")
    (work / "results.json").write_text(json.dumps({"default": plain, "height20": tall, "spacing20": wide,
                                                   "unowned": size(), "preferences": pref()}, indent=2))
    # Restore +20% locally, then close Settings. Only explicit PTY input reclaims ownership.
    click(784, 277)
    wait(lambda: pref().get("line_height") == 20, "restore +20% failed")
    desk.key("Escape", "Escape", "Escape")
    ready, received = work / "raw-ready", work / "mouse.bin"
    program = work / "mouse.py"
    program.write_text("import sys,tty,termios,select,time\n"
                      "old=termios.tcgetattr(0); tty.setraw(0)\n"
                      "print('\\x1b[2J\\x1b[H┌─────────┐ ██\\r\\n│ 世界 é  │\\r\\n└─────────┘\\r\\n\\x1b[2 q\\x1b[?1000h\\x1b[?1006h',end='',flush=True)\n"
                      + f"open({str(ready)!r},'w').close()\n"
                      "data=b''; until=time.monotonic()+3\n"
                      "while time.monotonic()<until:\n"
                      " if select.select([0],[],[],.05)[0]: data+=__import__('os').read(0,4096)\n"
                      "print('\\x1b[?1000l\\x1b[?1006l',end='',flush=True)\n"
                      "termios.tcsetattr(0,termios.TCSANOW,old)\n"
                      + f"open({str(received)!r},'wb').write(data)\n")
    accept.api("block.event", session_id=sid, block_id=block,
               event={"type": "paste", "payload": {"text": "python3 " + str(program)}})
    desk.key("Return")
    wait(ready.exists, "raw program did not start")
    wait(lambda: size()["owner"] == wide["owner"], "explicit input did not reclaim ownership")
    desk.x("mousemove", "--window", window, round(8 + 20.5 * 10.8), round(60 + 3.5 * 26))
    desk.x("click", 1)
    desk.capture(window, "grid-metrics-20-forced-bar")
    wait(received.exists, "raw program did not finish")
    assert received.read_bytes() == b"\x1b[<0;21;4M\x1b[<0;21;4m", received.read_bytes()
    print("PASS real PTY mouse reports use the adjusted cell geometry", flush=True)
    # Selection uses those same cells; copy-on-selection/PRIMARY stay off.
    desk.x("mousemove", "--window", window, round(8 + 2.1 * 10.8), round(60 + 1.5 * 26))
    desk.x("mousedown", 1)
    desk.x("mousemove", "--window", window, round(8 + 5 * 10.8), round(60 + 1.5 * 26))
    desk.x("mousemove", "--window", window, round(8 + 9.8 * 10.8), round(60 + 1.5 * 26))
    desk.x("mouseup", 1)
    desk.capture(window, "grid-metrics-20-selection")
    # The real installed-font catalog, not the headless list fixture. No choice is saved.
    desk.key("ctrl+comma")
    click(210, 92)
    desk.type("Terminal font")
    desk.key("Return", "Tab", "Tab", "Return")
    desk.capture(window, "font-picker-real-normal")
    desk.key("Tab")  # All fonts
    desk.capture(window, "font-picker-real-all")
    desk.x("windowsize", window, 640, 480)
    time.sleep(.4)
    desk.key("Escape", "Return")  # Reopen anchored to the minimum-size control.
    desk.capture(window, "font-picker-real-minimum")
    desk.key("Escape")
    assert json.loads(config.read_text())["font_family"] == "IBM Plex Mono"
    print("PASS native font picker captured at normal and minimum sizes without saving a choice", flush=True)
finally:
    if peer:
        peer.close()
    if desk:
        desk.close()
    server.terminate()
    server.wait(timeout=8)
