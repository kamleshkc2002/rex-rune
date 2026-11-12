#!/usr/bin/env python3
"""M2.8 tab icon colour on private Rex/Xvfb; never personal server/display/clipboard.

Requires rex API 13, Xvfb, openbox, xdotool, ImageMagick and Pillow.
REX_BIN selects an already-built client. Captures, raw PTY input and results stay
in ~/.cache/agent-ws. Tests Settings, overflow arrows, tab navigation, mute and
reorder through actual input; does not test native Wayland or IME.
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
work = Path(tempfile.mkdtemp(prefix="rex-tab-icon-colours-", dir=Path.home() / ".cache/agent-ws"))
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
spec = importlib.util.spec_from_file_location("accept", ROOT / "scripts/rune-acceptance-x11.py")
accept = importlib.util.module_from_spec(spec)
spec.loader.exec_module(accept)


def wait(predicate, description, timeout=15):
    """Wait for an asynchronous native/server event, never retry a failed assertion."""
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        if predicate():
            return
        time.sleep(.08)
    raise AssertionError(description)


def click(x, y, button=1):
    desk.x("mousemove", "--window", window, x, y)
    desk.x("click", button)
    time.sleep(.3)


def view():
    return accept.api("session.view", session_id=sid)


def active():
    return view()["active_window_id"]


def size(block):
    return accept.api("com.superlogical.terminal.size", session_id=sid, block_id=block)


def start_recorder(block, index):
    accept.api("block.event", session_id=sid, block_id=block,
               event={"type": "paste", "payload": {"text": f"python3 {program} {index}"}})
    accept.api("block.event", session_id=sid, block_id=block,
               event={"type": "key", "payload": {"key": "enter", "action": "press", "mods": ""}})
    wait(lambda: (work / f"ready-{index}").exists(), f"recorder {index} did not start")


def header(name):
    return Image.open(work / "captures" / f"{name}.png").convert("RGB").crop((0, 0, 1024, 52))


server = subprocess.Popen(["rex", "server", "run", "--listen", os.environ["REX_SERVER"],
                           "--restore=false", "--config", "/dev/null", "--state-dir", str(work / "server")],
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
desk = None
try:
    print("Artifacts:", work, flush=True)
    wait(lambda: (work / "rex.sock").exists(), "private server did not start")
    program = work / "record.py"
    program.write_text("import os,sys,tty,termios,select,time\n"
                       "index=sys.argv[1]; old=termios.tcgetattr(0); tty.setraw(0)\n"
                       "print('\\x1b[2J\\x1b[H\\x1b[?25lPrivate tab '+index,end='',flush=True)\n"
                       + f"root={str(work)!r}\n"
                       "open(root+'/ready-'+index,'w').close()\n"
                       "data=b''; until=time.monotonic()+120\n"
                       "while time.monotonic()<until and not os.path.exists(root+'/stop'):\n"
                       " if select.select([0],[],[],.05)[0]: data+=os.read(0,4096)\n"
                       "termios.tcsetattr(0,termios.TCSANOW,old)\n"
                       "open(root+'/input-'+index,'wb').write(data)\n")
    sid = accept.api("session.create", label="Build", initial_windows=[
        {"window_label": f"Tab {i}", "layout": accept.SHELL} for i in range(2)]) ["session_id"]
    initial = view()["windows"]
    ids = [tab["window_id"] for tab in initial]
    blocks = [tab["focused_block_id"] for tab in initial]
    for i, block in enumerate(blocks):
        start_recorder(block, i)
    accept.api("session.focus_window", session_id=sid, window_id=ids[0])
    desk = accept.Desktop()
    config = work / "workspace.json"
    config.write_text(json.dumps({"sidebar_open": False, "font_family": "IBM Plex Mono", "pane_padding": 8}))
    log = (work / "client.log").open("w")
    app, window = desk.launch(sid, config, size="1024x700", log=log)
    before = size(blocks[0])
    desk.capture(window, "tabs-default")
    modes = ["always", "active_pane", "active_tab", "never"]
    for index, mode in enumerate(modes):
        desk.key("ctrl+comma")
        click(200, 92)
        desk.type("Use colour for tab icons")
        desk.key("Return")
        time.sleep(2.8)
        desk.capture(window, f"search-{mode}")
        click(835, 405)
        desk.capture(window, f"choice-{mode}")
        current = 3 if index == 0 else index - 1
        for _ in range(index, current):
            desk.key("Up")
        for _ in range(current, index):
            desk.key("Down")
        desk.key("Return")
        wait(lambda: json.loads(config.read_text())["interface"]["tab_icon_color"] == mode,
             f"{mode} did not save")
        desk.capture(window, f"settings-{mode}")
        desk.key("Escape", "Escape")
        desk.capture(window, f"tabs-{mode}")
        assert size(blocks[0]) == before, f"{mode} changed terminal size or ownership"
    # Unknown recorder programs use the green terminal glyph. Two single-pane tabs
    # distinguish Always / Active tab / Never; Active pane equals Active tab here.
    assert ImageChops.difference(header("tabs-default"), header("tabs-never")).getbbox() is None
    assert ImageChops.difference(header("tabs-always"), header("tabs-active_tab")).getbbox() is not None
    assert ImageChops.difference(header("tabs-active_tab"), header("tabs-active_pane")).getbbox() is None
    assert ImageChops.difference(header("tabs-never"), header("tabs-active_tab")).getbbox() is not None
    print("PASS native actual-label search, anchored choices, keyboard selection, save, unchanged size/owner, fallback glyph pixels", flush=True)
    # Persist a non-default mode across a real process relaunch.
    desk.key("ctrl+comma")
    click(200, 92)
    desk.type("Use colour for tab icons")
    desk.key("Return")
    time.sleep(2.8)
    click(835, 405)
    desk.key("Up", "Return")  # Never -> Active tab
    wait(lambda: json.loads(config.read_text())["interface"]["tab_icon_color"] == "active_tab", "active_tab did not save")
    desk.key("Escape", "Escape")
    app.terminate()
    app.wait(timeout=5)
    app, window = desk.launch(sid, config, size="1024x700", log=log)
    assert json.loads(config.read_text())["interface"]["tab_icon_color"] == "active_tab"
    desk.capture(window, "relaunched-active-tab")
    assert ImageChops.difference(header("tabs-active_tab"), header("relaunched-active-tab")).getbbox() is None
    print("PASS saved colour mode survives private client relaunch with identical toolbar", flush=True)

    for i in range(2, 8):
        accept.api("session.new_window", session_id=sid, window_label=f"Tab {i}", layout=accept.SHELL, focus=False)
    tabs = view()["windows"]
    ids = [tab["window_id"] for tab in tabs]
    blocks = [tab["focused_block_id"] for tab in tabs]
    for i in range(2, 8):
        start_recorder(blocks[i], i)
    time.sleep(.5)
    desk.capture(window, "overflow-start")
    # Identity has the existing 120 pt minimum. The right action group and 48 pt
    # drag region stay fixed; these are the tab track's two existing scroll arrows.
    click(759, 26)
    desk.capture(window, "overflow-next")
    assert active() == ids[0], "scrolling selected a different tab"
    assert ImageChops.difference(header("overflow-start"), header("overflow-next")).getbbox() is not None
    click(194, 26)
    desk.capture(window, "overflow-previous")
    assert ImageChops.difference(header("overflow-start"), header("overflow-previous")).getbbox() is None
    desk.key("ctrl+Tab")
    wait(lambda: active() == ids[1], "Ctrl Tab did not select tab 1")
    click(255, 26)
    wait(lambda: active() == ids[0], "tab click did not select tab 0")
    desk.key("ctrl+shift+Tab")
    wait(lambda: active() == ids[-1], "reverse navigation did not select last tab")
    desk.capture(window, "selected-last")
    click(724, 26, 3)
    desk.capture(window, "tab-menu")
    desk.key("Down", "Return")
    wait(lambda: any(t["window"] == ids[-1] for t in json.loads(config.read_text())["muted_tabs"]), "tab menu did not mute target")
    click(724, 26, 3)
    desk.key("Down", "Down", "Return")
    wait(lambda: view()["windows"][-2]["window_id"] == ids[-1], "tab menu did not move target left")
    desk.capture(window, "reordered-muted")
    print("PASS overflow arrows both directions; key/click navigation and last-tab reveal; targeted mute/reorder menu", flush=True)
    desk.x("windowsize", window, 640, 480)
    time.sleep(.5)
    desk.capture(window, "minimum-overflow")
    desk.key("x")
    (work / "stop").touch()
    wait(lambda: all((work / f"input-{i}").exists() for i in range(8)), "recorders did not finish")
    received = [(work / f"input-{i}").read_bytes() for i in range(8)]
    assert received == [b""] * 7 + [b"x"], received
    print("PASS all eight raw PTYs: no Settings/navigation/menu leakage; only explicit x reached the selected tab", flush=True)
    (work / "results.json").write_text(json.dumps({"size_before_preference": before,
        "input_hex": [data.hex() for data in received], "preferences": json.loads(config.read_text())}, indent=2))
finally:
    if desk:
        desk.close()
    server.terminate()
    server.wait(timeout=5)
