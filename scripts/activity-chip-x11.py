#!/usr/bin/env python3
"""M3.2 activity chip: private Rex, two sessions, Xvfb, HOME/XDG and session bus.

Requires built REX_BIN, Rex API 13, Xvfb/openbox/xdotool/ImageMagick/Tesseract.
No personal display, Rex server, notification daemon or bus is used. Notifications
are off and the private bus has no activation directories. All artifacts stay in
~/.cache/agent-ws/rex-m32-chip/. Children are stopped by PID.
"""
import importlib.util
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
BINARY = os.environ.get("REX_BIN", str(ROOT / "target/release/rex-linux"))
BASE = Path.home() / ".cache/agent-ws/rex-m32-chip"
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
    # Do not move the pointer onto a list row while checking its initial field focus.
    subprocess.run(["import", "-window", window, str(work / "captures" / f"{name}.png")], env=desk.env, check=True)
    original = work / "captures" / f"{name}.png"
    ocr = work / "captures" / f"{name}-ocr.png"
    subprocess.run(["magick", str(original), "-crop", "470x580+277+55", "-resize", "300%", "-colorspace", "Gray", "-negate", str(ocr)], check=True)
    words = subprocess.check_output(["tesseract", str(ocr), "stdout"], text=True, stderr=subprocess.DEVNULL)
    (work / "captures" / f"{name}.txt").write_text(words)
    return words


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
    for label, names in [("Primary", ("Front", "Background")), ("Elsewhere", ("OtherTask", "OtherSecond"))]:
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

    # Visit both sessions once: this test deliberately exercises known cached layouts,
    # not the deferred question of prefetching status for never-opened sessions.
    for session_name in ("Elsewhere", "Primary"):
        desk.key("ctrl+k")
        desk.type(session_name)
        desk.key("Return")
        wait(lambda: f" - {session_name}" in desk.x("getwindowname", window), "session did not open")
    time.sleep(1)
    report("Front", "blocked", 1)
    report("Background", "working", 1)
    time.sleep(1)

    def toolbar(name):
        desk.capture(window, name)
        crop = work / "captures" / f"{name}-toolbar.png"
        subprocess.run(["magick", str(work / "captures" / f"{name}.png"),
                        "-crop", "1024x52+0+0", "-resize", "300%", "-colorspace", "Gray", "-negate", str(crop)], check=True)
        words = subprocess.check_output(["tesseract", str(crop), "stdout", "--psm", "6"], text=True, stderr=subprocess.DEVNULL)
        (work / "captures" / f"{name}.txt").write_text(words)
        return words, crop

    def snapshot(name):
        data = {"views": [accept.api("session.view", session_id=s) for s in sessions],
                "sizes": {name: accept.api("com.superlogical.terminal.size", session_id=s, block_id=t["focused_block_id"]) for name, (s,t) in tabs.items()}}
        (work / f"{name}.json").write_text(json.dumps(data, indent=2))
        return data

    words, _ = toolbar("front-only")
    assert "needs you" not in words.lower() and "need you" not in words.lower(), words
    before = snapshot("before")
    report("OtherTask", "blocked", 1)
    time.sleep(1)
    words, _ = toolbar("one-waiting")
    assert re.search(r"(?<!\d)1\s*needs\s*you", words.lower()), words
    assert before == snapshot("one-waiting"), "appearance changed view, size or owner"
    report("OtherSecond", "blocked", 1)
    time.sleep(1)
    words, crop = toolbar("two-waiting")
    assert re.search(r"(?<!\d)2\s*need\s*you", words.lower()), words
    assert before == snapshot("two-waiting"), "count change changed view, size or owner"

    # Real click on the toolbar chip, located by the rendered word rather than a guessed x.
    tsv = subprocess.check_output(["tesseract", str(crop), "stdout", "--psm", "6", "tsv"], text=True, stderr=subprocess.DEVNULL)
    cells = [line.split("\t") for line in tsv.splitlines()[1:]]
    word = next(c for c in cells if len(c) == 12 and c[11].lower() == "you")
    x = (int(word[6]) + int(word[8]) / 2) / 3
    y = (int(word[7]) + int(word[9]) / 2) / 3
    desk.x("mousemove", "--window", window, round(x), round(y))
    desk.x("click", 1)
    time.sleep(.6)
    words = capture("clicked-list")
    assert "NEEDS YOU" in words and "OtherTask" in words and "OtherSecond" in words, words
    desk.type("OtherTask")
    words = capture("click-focused-filter")
    assert "OtherTask" in words and "OtherSecond" not in words, words
    desk.key("Escape")
    time.sleep(.5)
    assert before == snapshot("after-click"), "click/filter/Escape changed view, size or owner"

    # Programs are dealt with on their private PTYs; no navigation/ownership changes are
    # needed to dismiss the chip. Done is deliberately not a reason to keep it.
    report("OtherTask", "done", 2)
    time.sleep(.7)
    words, _ = toolbar("one-left")
    assert re.search(r"(?<!\d)1\s*needs\s*you", words.lower()), words
    report("OtherSecond", "done", 2)
    time.sleep(.7)
    words, _ = toolbar("gone")
    assert "needs you" not in words.lower() and "need you" not in words.lower(), words
    assert before == snapshot("after"), "disappearance changed view, size or owner"
    # Errors are a separate word/colour; needs-you has already won over them headlessly.
    report("OtherTask", "error", 3)
    time.sleep(.7)
    words, _ = toolbar("error")
    assert re.search(r"(?<!\d)1\s*error", words.lower()), words
    report("OtherTask", "done", 4)
    time.sleep(.7)
    assert before == snapshot("after-error"), "error chip changed view, size or owner"
    (work / "result.json").write_text(json.dumps({
        "passed": True, "two_sessions": True, "live_osc7501": True,
        "on_screen_blocked_excluded": True, "counts": [0,1,2,1,0],
        "real_chip_click_and_filter_focus": True, "error_chip": True,
        "unchanged_all_views_sizes_owners": True,
        "binary_sha256": __import__("hashlib").sha256(Path(BINARY).read_bytes()).hexdigest(),
        "not_exercised": ["never-opened session coverage", "cross-host native activity", "Wayland", "notification daemon", "screen reader", "native 640pt (headless captures cover it)"]
    }, indent=2))
    print("PASS private chip 0→1→2→1→0, real click/filter, error, unchanged all views/sizes/owners", flush=True)
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
