#!/usr/bin/env python3
"""M2.7 first-piece acceptance: private Rex/Xvfb, asymmetric padding and ownership.

Requires rex (API 13), Xvfb, openbox, xdotool, ImageMagick import, python3.
REX_BIN selects an already-built client. Never uses inherited HOME, server,
clipboard, display or XDG state. Creates one disposable session. Evidence stays
in ~/.cache/agent-ws. No focus-following or fading is implemented/tested here.
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
work = Path(tempfile.mkdtemp(prefix="rex-m27-padding-", dir=Path.home() / ".cache/agent-ws"))
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
    time.sleep(.25)


def prefs():
    return json.loads(config.read_text())


def size():
    return accept.api("com.superlogical.terminal.size", session_id=sid, block_id=block)


def search(label):
    click(200, 92)
    desk.key("ctrl+a")
    desk.type(label)
    desk.key("Return")
    # The search mark wraps the target row for 2.4 seconds; let it leave before
    # driving repeated presses on that row, so its path is stable throughout.
    time.sleep(2.8)


server = subprocess.Popen(["rex", "server", "run", "--listen", os.environ["REX_SERVER"],
                           "--restore=false", "--config", "/dev/null", "--state-dir", str(work / "server")],
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
desk = peer = None
try:
    print("Artifacts:", work, flush=True)
    wait(lambda: (work / "rex.sock").exists(), "private server did not start")
    sid, block = accept.session("M2.7 private padding")
    desk = accept.Desktop()
    config = work / "workspace.json"
    config.write_text(json.dumps({"sidebar_open": False, "font_family": "IBM Plex Mono",
                                 "pane_padding": 24, "interface": {"pane_padding_vertical": 2},
                                 "terminal": {"font": {"line_height": 20, "spacing": 20,
                                                          "spacing_unit": "percent"}}}))
    log = (work / "client.log").open("w")
    app, window = desk.launch(sid, config, size="1024x700", log=log)
    wait(lambda: size()["columns"] == 90, "asymmetric IBM Plex grid was not 90 columns")
    initial = size()
    assert initial["rows"] == 24, initial
    print("PASS 24×2 padding, +20% font metrics: 90 columns × 24 rows", flush=True)

    # A private raw PTY recorder sees any leaked Settings keys and real mouse reports.
    ready, stop, received = work / "raw-ready", work / "raw-stop", work / "mouse.bin"
    program = work / "mouse.py"
    program.write_text("import os,tty,termios,select,time\n"
                       "old=termios.tcgetattr(0); tty.setraw(0)\n"
                       "print('\\x1b[2J\\x1b[Hpadding probe\\r\\nsecond\\r\\nthird\\x1b[?1000h\\x1b[?1006h',end='',flush=True)\n"
                       + f"open({str(ready)!r},'w').close()\n"
                       "data=b''; until=time.monotonic()+90\n"
                       + f"while time.monotonic()<until and not os.path.exists({str(stop)!r}):\n"
                       " if select.select([0],[],[],.05)[0]: data+=os.read(0,4096)\n"
                       "print('\\x1b[?1000l\\x1b[?1006l',end='',flush=True)\n"
                       "termios.tcsetattr(0,termios.TCSANOW,old)\n"
                       + f"open({str(received)!r},'wb').write(data)\n")
    accept.api("block.event", session_id=sid, block_id=block,
               event={"type": "paste", "payload": {"text": "python3 " + str(program)}})
    desk.key("Return")
    wait(ready.exists, "raw program did not start")
    desk.key("ctrl+comma")
    search("Horizontal padding")
    desk.capture(window, "padding-search")
    # Found rows are centered in the 1024×700 Settings page. Re-hit the button
    # for each step: the transient search mark can disappear during this sequence.
    for expected in range(25, 29):
        click(876, 406)
        wait(lambda: prefs()["pane_padding"] == expected, f"horizontal step {expected} did not save")
    assert prefs()["interface"]["pane_padding_vertical"] == 2
    wait(lambda: size()["columns"] == 89, "owned padding change did not resize to 89 columns")
    owned = size()
    assert owned["rows"] == 24 and owned["owner"] == initial["owner"], owned
    print("PASS independent horizontal change: 89×24, same owner", flush=True)

    search("Vertical padding")
    desk.capture(window, "vertical-search")
    for expected in range(3, 17):
        click(876, 406)
        wait(lambda: prefs()["interface"]["pane_padding_vertical"] == expected, f"vertical step {expected} did not save")
    assert prefs()["pane_padding"] == 28
    wait(lambda: size()["rows"] == 23, "owned vertical padding did not resize to 23 rows")
    vertical_owned = size()
    assert vertical_owned["columns"] == 89 and vertical_owned["owner"] == initial["owner"]
    print("PASS independent vertical change: 89×23, same owner", flush=True)

    peer = contract.Control(name="m27-other-owner", log_requests=False)
    for method, payload in [("session.attach", {"session_id": sid}),
                            ("session.own_block", {"session_id": sid, "block_id": block})]:
        reply = peer.call(method, payload)
        assert "error" not in reply, reply
    wait(lambda: size()["owner"] != initial["owner"], "peer did not take ownership")
    unowned = size()
    # The same controls now change local geometry only, including crossing a row
    # boundary. The server's complete size response (desired timestamps included)
    # must stay unchanged. Restore the tested owned geometry before explicit input.
    for expected in range(17, 27):
        click(876, 406)
        wait(lambda: prefs()["interface"]["pane_padding_vertical"] == expected, f"unowned step {expected} did not save")
    assert size() == unowned, (size(), unowned)
    desk.capture(window, "padding-unowned")
    for expected in range(25, 15, -1):
        click(784, 406)
        wait(lambda: prefs()["interface"]["pane_padding_vertical"] == expected, f"restore step {expected} did not save")
    assert size() == unowned
    search("Active pane outline")
    click(876, 406)
    desk.key("Home", "Down", "Return")
    wait(lambda: prefs()["interface"].get("highlight_active_pane") is True, "outline did not save On")
    assert size() == unowned
    desk.capture(window, "outline-settings-unowned")
    print("PASS padding and outline preferences neither resize nor reclaim the peer's pane", flush=True)
    desk.key("Escape", "Escape")
    desk.capture(window, "before-explicit-input")
    # Only explicit PTY input may reclaim the pane. The known byte also proves no
    # preceding Settings key leaked into the recorder.
    desk.key("x")
    wait(lambda: size()["owner"] == initial["owner"] and size()["rows"] == 23,
         "explicit input did not reclaim the 89×23 pane")
    assert size()["columns"] == 89
    reclaimed = size()
    # Grid origin is (28, 52+16); exact IBM Plex +20% cells are 10.8×26.
    desk.x("mousemove", "--window", window, round(28 + 20.5 * 10.8), round(68 + 2.5 * 26))
    desk.x("click", 1)
    desk.capture(window, "asymmetric-mouse")
    assert size() == reclaimed
    stop.touch()
    wait(received.exists, "raw program did not finish")
    assert received.read_bytes() == b"x\x1b[<0;21;3M\x1b[<0;21;3m", received.read_bytes()
    print("PASS exact PTY mouse coordinates; only the explicit x input reclaimed ownership; no leaked Settings input", flush=True)
    (work / "results.json").write_text(json.dumps({"initial": initial, "horizontal_owned": owned, "vertical_owned": vertical_owned,
        "unowned": unowned, "reclaimed": reclaimed, "preferences": prefs(), "mouse_hex": received.read_bytes().hex()}, indent=2))
finally:
    if peer:
        peer.close()
    if desk:
        desk.close()
    server.terminate()
    server.wait(timeout=8)
