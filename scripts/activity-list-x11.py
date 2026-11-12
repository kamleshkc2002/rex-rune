#!/usr/bin/env python3
"""M3.2 Activity: private Rex, two sessions, Xvfb, HOME/XDG and session bus.

Requires built REX_BIN, Rex API 13, Xvfb/openbox/xdotool/ImageMagick/Tesseract.
No personal display, Rex server, notification daemon or bus is used. Notifications
are off and the private bus has no activation directories. All artifacts stay in
~/.cache/agent-ws/rex-m32-activity/. Children are stopped by PID.
"""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
BINARY = os.environ.get("REX_BIN", str(ROOT / "target/release/rex-linux"))
BASE = Path.home() / ".cache/agent-ws/rex-m32-activity"
BASE.mkdir(parents=True, exist_ok=True)
work = Path(tempfile.mkdtemp(prefix="native-", dir=BASE))
for tool in ("rex", "dbus-daemon", "Xvfb", "openbox", "xdotool", "import", "magick", "tesseract"):
    assert shutil.which(tool), f"{tool} required"
for name in ("home", "config", "state", "runtime", "cache", "captures", "tmp"):
    (work / name).mkdir(mode=0o700)
for name in list(os.environ):
    if name.startswith(("REX_", "HERDR_")) or name in ("DISPLAY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS"):
        del os.environ[name]
endpoint = "unix://" + str(work / "rex.sock")
os.environ.update(HOME=str(work / "home"), XDG_CONFIG_HOME=str(work / "config"),
                  XDG_STATE_HOME=str(work / "state"), XDG_RUNTIME_DIR=str(work / "runtime"),
                  XDG_CACHE_HOME=str(work / "cache"), TMPDIR=str(work / "tmp"),
                  REX_BIN=BINARY, REX_CAPTURE_OUT=str(work / "captures"), REX_SERVER=endpoint)
tempfile.tempdir = str(work / "tmp")
spec = importlib.util.spec_from_file_location("accept", ROOT / "scripts/rune-acceptance-x11.py")
accept = importlib.util.module_from_spec(spec)
spec.loader.exec_module(accept)
processes = []
desk = None


def wait(predicate, description, timeout=15):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        if predicate():
            return
        time.sleep(.08)
    raise AssertionError(description)


def spawn(args, name):
    process = subprocess.Popen(args, stdout=(work / f"{name}.log").open("w"), stderr=subprocess.STDOUT)
    processes.append(process)
    return process


def capture(name):
    desk.capture(window, name)
    original = work / "captures" / f"{name}.png"
    ocr = work / "captures" / f"{name}-ocr.png"
    subprocess.run(["magick", str(original), "-crop", "470x580+277+55", "-resize", "300%", "-colorspace", "Gray", "-negate", str(ocr)], check=True)
    words = subprocess.check_output(["tesseract", str(ocr), "stdout"], text=True, stderr=subprocess.DEVNULL)
    (work / "captures" / f"{name}.txt").write_text(words)
    return words


def show():
    desk.key("ctrl+shift+p")
    desk.type("Show activity")
    desk.key("Return")
    time.sleep(.5)


def report(name, state, serial):
    (work / name).write_text(f"\x1b]7501;state={state}:app=cargo\x07{serial}\r\n")


try:
    print("Artifacts:", work, flush=True)
    bus_config = work / "bus.conf"
    bus_config.write_text(f'<busconfig><type>session</type><listen>unix:path={work}/bus</listen>'
                          '<auth>EXTERNAL</auth><policy context="default"><allow own="*"/>'
                          '<allow send_destination="*"/><allow receive_sender="*"/></policy></busconfig>')
    bus = subprocess.Popen(["dbus-daemon", "--nofork", "--config-file", str(bus_config), "--print-address=1"], stdout=subprocess.PIPE, text=True)
    processes.append(bus)
    assert bus.stdout.readline().startswith("unix:")
    os.environ["DBUS_SESSION_BUS_ADDRESS"] = f"unix:path={work}/bus"
    spawn(["rex", "server", "run", "--listen", endpoint, "--restore=false", "--config", "/dev/null", "--state-dir", str(work / "rex-state")], "rex")
    wait(lambda: (work / "rex.sock").exists(), "private Rex did not start")
    sessions = []
    tabs = {}
    for label, names in [("Primary", ("Front", "Approval", "Compile", "Finished", "Failure")), ("Elsewhere", ("OtherTask",))]:
        sid = accept.api("session.create", label=label, initial_windows=[{"window_label": name, "layout": accept.SHELL} for name in names])["session_id"]
        sessions.append(sid)
        for tab in accept.api("session.view", session_id=sid)["windows"]:
            tabs[tab["label"]] = (sid, tab)
    reporter = work / "report.py"
    reporter.write_text("import pathlib,sys,time\np=pathlib.Path(sys.argv[1]); old=''; p.with_suffix('.ready').touch()\n"
                        "while True:\n text=p.read_text() if p.exists() else ''\n"
                        " if text != old:\n  old=text; print(text, end='', flush=True)\n time.sleep(.05)\n")
    time.sleep(.5)
    for name, (sid, tab) in tabs.items():
        block = tab["focused_block_id"]
        accept.api("block.event", session_id=sid, block_id=block, event={"type":"paste", "payload":{"text":f"/usr/bin/python3 {reporter} {work / name}"}})
        accept.api("block.event", session_id=sid, block_id=block, event={"type":"key", "payload":{"key":"enter", "action":"press", "mods":""}})
    wait(lambda: all((work / f"{name}.ready").exists() for name in tabs), "reporters did not start")
    sid, tab = tabs["Front"]
    accept.api("session.focus_window", session_id=sid, window_id=tab["window_id"])
    config = work / "workspace.json"
    config.write_text(json.dumps({"font_family":"IBM Plex Mono", "hosts":[{"label":"Private", "endpoint":endpoint}], "sessions":{endpoint:sid}, "notifications":{"system_notifications":False}}))
    desk = accept.Desktop()
    app, window = desk.launch(sid, config, size="1024x700", log=(work / "client.log").open("w"))
    for name, state in [("Approval","blocked"),("Compile","working"),("Finished","working"),("Failure","error"),("OtherTask","blocked")]:
        report(name, state, 1)
    time.sleep(1.5)
    report("Finished", "done", 2)
    time.sleep(1)

    def snapshot():
        return {"views": [accept.api("session.view", session_id=s) for s in sessions],
                "sizes": {name: accept.api("com.superlogical.terminal.size", session_id=s, block_id=t["focused_block_id"]) for name, (s,t) in tabs.items()}}

    # The sidebar: the session out of sight says what its tab needs; the one on screen does not.
    desk.key("ctrl+shift+b")
    time.sleep(.6)
    desk.capture(window, "sidebar")
    side = work / "captures" / "sidebar-ocr.png"
    subprocess.run(["magick", str(work / "captures" / "sidebar.png"), "-crop", "236x400+10+140", "-resize", "300%", "-colorspace", "Gray", "-negate", str(side)], check=True)
    rows = subprocess.check_output(["tesseract", str(side), "stdout"], text=True, stderr=subprocess.DEVNULL)
    (work / "captures" / "sidebar.txt").write_text(rows)
    lines = [line for line in rows.splitlines() if line.strip()]
    elsewhere = next((line for line in lines if "Elsewhere" in line), "")
    primary = next((line for line in lines if "Primary" in line), "")
    assert "Needs input" in elsewhere, rows
    assert primary and "Needs" not in primary and "Error" not in primary, rows
    # Showing the sidebar narrows the panes this client owns, as it always has, so the
    # unchanged-server comparison below starts once it is hidden again.
    desk.key("ctrl+shift+b")
    time.sleep(1.2)
    before = snapshot()
    (work / "before.json").write_text(json.dumps(before, indent=2))
    show()
    words = capture("populated")
    for word in ("NEEDS YOU", "ERROR", "DONE", "WORKING", "Approval", "Compile", "Finished", "OtherTask", "Elsewhere"):
        assert word.lower() in words.lower(), (word, words)
    offsets = [words.lower().index(word) for word in ("needs you", "error", "done", "working")]
    assert offsets == sorted(offsets), words
    desk.key("Escape")
    time.sleep(.5)
    after = snapshot()
    (work / "after.json").write_text(json.dumps(after, indent=2))
    assert before == after, "opening/closing changed session view or size/owner"
    show()
    desk.type("OtherTask")
    assert "OtherTask" in capture("filtered")
    desk.key("Return")
    wait(lambda: "OtherTask - Elsewhere" in desk.x("getwindowname", window), "cross-session selection did not show target")
    desk.capture(window, "chosen-other-session")
    show()
    desk.type("Finished")
    desk.key("Return")
    wait(lambda: "Finished - Primary" in desk.x("getwindowname", window), "Done tab not selected")
    show()
    desk.type("Finished")
    assert "No matching activity" in capture("done-seen"), "Done latch not cleared on navigation"
    desk.key("Escape")
    (work / "result.json").write_text(json.dumps({"passed":True, "two_sessions":True, "live_osc7501":True,
        "groups":["Needs you","Error","Done","Working"], "unchanged_view_size_owner":True,
        "cross_session_navigation":True, "sidebar_session_word":True, "done_cleared_only_after_navigation":True,
        "not_exercised":["cross-host native navigation", "Wayland", "notification service"]}, indent=2))
    print("PASS private OSC activity, four groups, filter, cross-session navigation, Done latch, unchanged view/size/owner", flush=True)
finally:
    if desk:
        desk.close()
    for process in reversed(processes):
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
