#!/usr/bin/env python3
"""M2.5 clipboard/PRIMARY acceptance on a private Rex server and Xvfb.

Requires rex, Xvfb, openbox, xdotool, ImageMagick import and Python tkinter.
Uses Tk as an independent X11 clipboard client (no xclip required).
REX_BIN selects the built client. Never uses an inherited server, HOME or display.
Artifacts go under ~/.cache/agent-ws unless REX_SELECTION_WORK is set.
"""
import importlib.util
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import tempfile
import time
import tkinter

ROOT = Path(__file__).resolve().parents[1]
BINARY = os.environ.get("REX_BIN", str(ROOT / "target/release/rex-rune"))
BASE = Path(os.environ.get("REX_SELECTION_WORK", Path.home() / ".cache/agent-ws"))
BASE.mkdir(parents=True, exist_ok=True)
work = Path(tempfile.mkdtemp(prefix="rex-m25-x11-", dir=BASE))
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
board = None


def wait(predicate, description, timeout=10):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if board:
            board.update()
        if predicate():
            return
        time.sleep(.05)
    raise AssertionError(description)


def read(selection="CLIPBOARD"):
    board.update()
    try:
        return board.selection_get(selection=selection)
    except tkinter.TclError:
        return ""


def sentinel():
    board.clipboard_clear()
    board.clipboard_append("clipboard sentinel")
    board.selection_handle(lambda offset, length: "primary sentinel"[int(offset):int(offset)+int(length)], selection="PRIMARY")
    board.selection_own(selection="PRIMARY")
    board.update()


def command(text):
    accept.api("block.event", session_id=sid, block_id=block,
               event={"type": "paste", "payload": {"text": text}})
    desk.key("Return")


def sample():
    command("printf '\\033[2J\\033[H\\033[2 qalpha beta gamma 世界 é\\n'")
    time.sleep(.5)


def start_drag():
    desk.x("mousemove", "--window", window, *accept.cell_point(0, 0, fx=.1))
    desk.x("mousedown", 1)
    desk.x("mousemove", "--window", window, *accept.cell_point(2, 0))
    desk.x("mousemove", "--window", window, *accept.cell_point(4, 0, fx=.9))
    time.sleep(.3)


def collect(mode, gesture):
    """A real raw PTY program: record exactly what the client sent, including mouse reports."""
    path = work / f"input-{mode}.bin"
    ready = work / f"ready-{mode}"
    program = work / f"read-{mode}.py"
    tracking = mode.startswith("mouse")
    program.write_text("import os,sys,tty,termios,select,time\n"
        "fd=0; old=termios.tcgetattr(fd); tty.setraw(fd)\n"
        + ("sys.stdout.write('\\x1b[?1000h\\x1b[?1006h');sys.stdout.flush()\n" if tracking else "")
        + f"open({str(ready)!r},'w').close()\n"
        "data=b''; until=time.monotonic()+2\n"
        "while time.monotonic()<until:\n"
        " if select.select([fd],[],[],.05)[0]: data+=os.read(fd,4096)\n"
        "sys.stdout.write('\\x1b[?1000l\\x1b[?1006l');sys.stdout.flush()\n"
        "termios.tcsetattr(fd,termios.TCSANOW,old)\n"
        + f"open({str(path)!r},'wb').write(data)\n")
    command("python3 " + shlex.quote(str(program)))
    wait(ready.exists, "raw reader never started")
    time.sleep(.25)
    gesture()
    wait(path.exists, "raw reader never ended")
    return path.read_bytes()


server = subprocess.Popen(["rex", "server", "run", "--listen", os.environ["REX_SERVER"],
                           "--restore=false", "--config", "/dev/null", "--state-dir", str(work / "server")],
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
desk = None
try:
    print("Artifacts:", work, flush=True)
    wait(lambda: (work / "rex.sock").exists(), "server never started")
    sid, block = accept.session("M2.5 private")
    desk = accept.Desktop()
    os.environ["DISPLAY"] = desk.display
    board = tkinter.Tk()
    board.withdraw()
    config = accept.pin_layout(work / "workspace.json", sidebar_open=False)
    log = (work / "client.log").open("w")
    app, window = desk.launch(sid, config, size="1024x700", log=log)
    sample()
    sentinel()
    start_drag()
    desk.x("mouseup", 1)
    time.sleep(.3)
    assert read() == "clipboard sentinel" and read("PRIMARY") == "primary sentinel"
    print("PASS defaults do not copy or replace PRIMARY", flush=True)
    app.terminate()
    app.wait(timeout=10)
    data = json.loads(config.read_text())
    data["terminal"] = {"copy_on_selection": True, "primary_selection": True,
                        "cursor_style": "bar", "cursor_force": True,
                        "selection_background": "cell_foreground", "selection_rounded": True}
    config.write_text(json.dumps(data))
    app, window = desk.launch(sid, config, size="1024x700", log=log)
    sample()
    sentinel()
    start_drag()
    assert read() == "clipboard sentinel" and read("PRIMARY") == "primary sentinel"
    desk.x("mouseup", 1)
    wait(lambda: read() == "alpha" and read("PRIMARY") == "alpha", "selection did not copy on release")
    desk.capture(window, "copy-release-forced-bar")
    print("PASS release copies to CLIPBOARD and PRIMARY across X11", flush=True)
    time.sleep(.6)
    desk.x("mousemove", "--window", window, *accept.cell_point(12, 0))
    desk.x("click", 1)
    time.sleep(.2)
    assert read() == "alpha" and read("PRIMARY") == "alpha"
    print("PASS ordinary focus click does not replace either selection", flush=True)

    desk.x("mousemove", "--window", window, *accept.cell_point(0, 0, fx=.1))
    desk.x("mousedown", 1)
    desk.x("mousemove", "--window", window, *accept.cell_point(10, 0))
    desk.x("mousemove", "--window", window, *accept.cell_point(22, 0, fx=.9))
    desk.x("mouseup", 1)
    expected = "alpha beta gamma 世界 é"
    wait(lambda: read() == expected and read("PRIMARY") == expected, "wide/combining selection lost text")
    desk.capture(window, "wide-combining-selection")
    print("PASS wide and combining cells copy without padding or lost characters", flush=True)

    # A focus loss while dragging must not count as a release that copies.
    sentinel()
    start_drag()
    other = tkinter.Toplevel(board)
    other.title("Private focus target")
    other.geometry("100x50+1100+800")
    board.update()
    other_window = desk.x("search", "--name", "^Private focus target$")
    desk.x("windowfocus", other_window)
    desk.x("mouseup", 1)
    time.sleep(.3)
    assert read() == "clipboard sentinel" and read("PRIMARY") == "primary sentinel"
    desk.x("windowfocus", window)
    other.destroy()
    print("PASS focus loss cancels copy", flush=True)

    def middle():
        desk.x("mousemove", "--window", window, *accept.cell_point(20, 3))
        desk.x("click", 2)

    assert collect("paste", middle) == b"primary sentinel"
    print("PASS middle click pastes the independent PRIMARY owner into a real PTY", flush=True)
    received = collect("mouse", middle)
    assert received.startswith(b"\x1b[<1;") and b"primary sentinel" not in received, repr(received)
    print("PASS mouse-reporting program gets reports, never PRIMARY paste", flush=True)

    def middle_drag():
        desk.x("mousemove", "--window", window, *accept.cell_point(2, 3))
        desk.x("mousedown", 2)
        desk.x("mousemove", "--window", window, *accept.cell_point(20, 3))
        desk.x("mouseup", 2)

    assert collect("drag", middle_drag) == b""
    print("PASS middle drag does not paste", flush=True)
finally:
    if board:
        board.destroy()
    if desk:
        desk.close()
    server.terminate()
    server.wait(timeout=8)
