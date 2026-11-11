#!/usr/bin/env python3
"""Verify scroll-anchored text selection and native clipboard contents.

Requires rex, Xvfb, openbox, xdotool, xclip and ImageMagick's import. Set REX_BIN to the client.
Starts its own private Rex servers, HOME and display (REX_SERVER is set for you, never read), so no
daily sessions, display, clipboard or settings are used. Pointer positions come from
rune-acceptance-x11.py's grid_origin, with the pane padding pinned in the private workspace.json.
"""
import importlib.util
import json
import shlex
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
BINARY = os.environ.get("REX_BIN", str(ROOT / "target/debug/rex-linux"))
SERVER = shutil.which("rex")
for tool in ("rex", "Xvfb", "openbox", "xdotool", "import"):
    assert shutil.which(tool), f"{tool} required"
work = Path(tempfile.mkdtemp(prefix="rex-scroll-selection-"))
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
    print("Private scroll-selection artifacts:", work, flush=True)
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
    accept.pin_layout(config, hosts=[
        {"label": "Elphael-private", "endpoint": "unix://" + str(work / "rex-0.sock")},
        {"label": "Erdtree-private", "endpoint": "unix://" + str(work / "rex-1.sock")}],
        sidebar_open=False, vertical_tabs=False, sidebar_width=280)
    # Ghostty starts a selection in the cell under the pointer only before 60% of its width: start
    # just inside the first column of a row (never a fixed x, which the pane padding moves).
    left = accept.cell_point(0, 0, fx=0.2)[0]
    row = lambda n: accept.cell_point(0, n)[1]
    assert shutil.which("xclip"), "xclip required"
    desk = accept.Desktop()
    log = (work / "client.log").open("w")
    app, window = desk.launch(sid, config, log=log)
    program="import sys;sys.stdout.write('\\x1b[2J\\x1b[H');[print(f'line-{i:03d} 世界 e\\u0301 selection') for i in range(180)];sys.stdout.flush()"
    # The generated command's string literals must contain escapes, not escaped backslashes.
    program=program.replace("\\\\x1b","\\x1b").replace("\\\\u0301","\\u0301")
    api("block.event",session_id=sid,block_id=block,event={"type":"paste","payload":{"text":"python3 -c "+shlex.quote(program)}})
    desk.key("Return"); time.sleep(.7)
    desk.x("mousemove","--window",window,100,150)
    desk.x("click","--repeat",4,"--delay",80,4)
    time.sleep(.4)
    # A full visible line includes wide and combining characters.
    desk.x("mousemove","--window",window,left,row(2))
    desk.x("mousedown",1)
    desk.x("mousemove","--window",window,420,row(2))
    desk.x("mouseup",1)
    desk.key("ctrl+shift+c")
    time.sleep(.2)
    def clipboard():
        return subprocess.check_output(["xclip","-selection","clipboard","-o"],env=desk.env,timeout=5).decode()
    selected=clipboard()
    assert "line-" in selected and "世界" in selected and "e\u0301" in selected, repr(selected)
    subprocess.run(["xclip","-selection","clipboard"],input=b"sentinel",env=desk.env,check=True,timeout=5)
    desk.x("click",4); time.sleep(.4)
    desk.key("ctrl+shift+c"); time.sleep(.2)
    assert clipboard()==selected, "scrolling discarded or shifted the selection: "+repr(clipboard())
    desk.capture(window,"selection-after-scroll")
    print("PASS Unicode selection stays anchored while the viewport scrolls",flush=True)
    desk.x("mousemove","--window",window,left,row(21))
    desk.x("mousedown",1)
    desk.x("mousemove","--window",window,30,54)
    time.sleep(.8)
    desk.x("mouseup",1)
    def extended_selection_visible():
        # Wait for the native frame, not a fixed worker/renderer scheduling delay.
        pixels=subprocess.check_output(["import","-window",window,"-crop","1x900+400+0","-depth","8","rgb:-"],env=desk.env,timeout=5)
        return sum(pixels[i:i+3]==bytes([47,74,110]) for i in range(0,len(pixels),3))>=25*21
    wait(extended_selection_visible,"autoscroll selection frame did not reach the UI")
    desk.key("ctrl+shift+c"); time.sleep(.2)
    dragged=clipboard()
    assert dragged.count("line-")>=27, "edge drag did not autoscroll: "+repr(dragged)
    desk.capture(window,"drag-autoscroll")
    print("PASS dragging beyond the grid autoscrolls and copies across viewport boundaries",flush=True)

    events=work/"tui-events.bin"
    ready=work/"tui-ready"
    tui="""import os,sys,termios,tty,select,time
old=termios.tcgetattr(0)
tty.setraw(0)
e=chr(27)
try:
 sys.stdout.write(e+'[?1049h'+e+'[?1003h'+e+'[?1006h'+e+'[2J'+e+'[HTUI-before-scroll 世界 e'+chr(769)+' selection')
 sys.stdout.flush()
 with open(EVENTS,'wb') as f:
  open(READY,'w').close()
  deadline=time.monotonic()+15
  while time.monotonic()<deadline:
   if select.select([0],[],[],.1)[0]:
    data=os.read(0,4096)
    f.write(data);f.flush()
    if b'[<64;' in data:
     sys.stdout.write(e+'[HTUI-after-scroll  世界 e'+chr(769)+' selection'+e+'[K');sys.stdout.flush()
finally:
 termios.tcsetattr(0,termios.TCSANOW,old)
 sys.stdout.write(e+'[?1003l'+e+'[?1006l'+e+'[?1049l');sys.stdout.flush()
""".replace("EVENTS",repr(str(events))).replace("READY",repr(str(ready)))
    api("block.event",session_id=sid,block_id=block,event={"type":"paste","payload":{"text":"python3 -c "+shlex.quote(tui)}})
    desk.key("Return")
    wait(ready.exists,"mouse-tracking TUI did not start")
    time.sleep(.3)
    desk.x("keydown","Shift_L")
    before=events.read_bytes()
    desk.x("mousemove","--window",window,left,row(0))
    desk.x("mousedown",1)
    desk.x("mousemove","--window",window,490,row(0))
    desk.x("mouseup",1)
    desk.key("ctrl+shift+c"); time.sleep(.2)
    assert "TUI-before-scroll" in clipboard(), repr(clipboard())
    assert b"\x1b[<" not in events.read_bytes()[len(before):], "local Shift selection leaked mouse reports to the TUI"
    desk.x("keyup","Shift_L")
    desk.x("click",4)
    wait(lambda:b"[<64;" in events.read_bytes(),"ordinary TUI wheel reporting stopped working")
    time.sleep(.3)
    desk.x("keydown","Shift_L")
    desk.x("mousemove","--window",window,left,row(0))
    desk.x("mousedown",1)
    desk.x("mousemove","--window",window,490,row(0))
    desk.x("mouseup",1)
    desk.key("ctrl+shift+c"); time.sleep(.2)
    assert "TUI-after-scroll" in clipboard(), repr(clipboard())
    desk.x("keyup","Shift_L")
    desk.capture(window,"tui-selection-after-scrolling")
    print("PASS fullscreen TUI wheel scrolling and Shift selection, without leaking local mouse reports",flush=True)
finally:
    if desk: desk.close()
    for server in servers:
        server.terminate();server.wait(timeout=8)
