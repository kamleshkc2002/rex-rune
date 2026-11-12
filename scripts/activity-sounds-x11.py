#!/usr/bin/env python3
"""M3.4 sound dispatch, NEVER audible: private Rex, Xvfb, bus, HOME and PATH.

Requires built REX_BIN (default target/release/rex-linux), Rex API 13,
Xvfb/openbox/xdotool/ImageMagick, Mesa lavapipe and Python. Only fake audio executables exist
on the child's PATH; all three names are checked before every launch. No real
notification daemon or audio player is started. Evidence stays under
~/.cache/agent-ws/rex-m34-sounds/. Children stop by PID, never by name.
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
BINARY = Path(os.environ.get("REX_BIN", ROOT / "target/release/rex-linux")).resolve()
BASE = Path.home() / ".cache/agent-ws/rex-m34-sounds"
BASE.mkdir(parents=True, exist_ok=True)
work = Path(tempfile.mkdtemp(prefix="native-", dir=BASE))
tools = {}
for name in ("rex", "dbus-daemon", "Xvfb", "openbox", "xdotool", "import", "bash", "sh", "env", "sleep", "python3"):
    found = shutil.which(name)
    assert found, f"{name} required"
    tools[name] = Path(found).resolve()
assert BINARY.is_file(), f"build {BINARY} first"
ICD = Path("/usr/share/vulkan/icd.d/lvp_icd.json")
assert ICD.is_file(), "Mesa lavapipe Vulkan driver required"
for name in ("home", "config", "state", "runtime", "cache", "data", "captures", "tmp", "bin"):
    (work / name).mkdir(mode=0o700)
for name, path in tools.items():
    (work / "bin" / name).symlink_to(path)
# Absolute interpreter, no shell; the fake only appends its argument vector.
for name in ("canberra-gtk-play", "pw-play", "paplay"):
    fake = work / "bin" / name
    fake.write_text("#!/usr/bin/python3\nimport json,sys\n"
                    f"with open({str(work / 'audio.jsonl')!r}, 'a') as f: f.write(json.dumps(sys.argv[1:]) + '\\n')\n")
    fake.chmod(0o755)
for name in list(os.environ):
    if name.startswith(("REX_", "HERDR_")) or name in ("DISPLAY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS"):
        del os.environ[name]
endpoint = "unix://" + str(work / "rex.sock")
os.environ.update(HOME=str(work / "home"), XDG_CONFIG_HOME=str(work / "config"),
                  XDG_STATE_HOME=str(work / "state"), XDG_RUNTIME_DIR=str(work / "runtime"),
                  XDG_CACHE_HOME=str(work / "cache"), XDG_DATA_HOME=str(work / "data"),
                  XDG_DATA_DIRS=str(work / "data"), TMPDIR=str(work / "tmp"),
                  PATH=str(work / "bin"), SHELL=str(tools["bash"]),
                  VK_DRIVER_FILES=str(ICD),
                  REX_BIN=str(BINARY), REX_CAPTURE_OUT=str(work / "captures"), REX_SERVER=endpoint)
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
        time.sleep(.05)
    raise AssertionError(description)


def audio():
    path = work / "audio.jsonl"
    return [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []


def spawn(args, name):
    p = subprocess.Popen(args, stdout=(work / f"{name}.log").open("w"), stderr=subprocess.STDOUT)
    processes.append(p)
    return p


def stop(p):
    if p.poll() is None:
        p.terminate()
        try:
            p.wait(timeout=5)
        except subprocess.TimeoutExpired:
            p.kill()
            p.wait(timeout=5)


try:
    print("Artifacts:", work, flush=True)
    config = work / "bus.conf"
    config.write_text(f'<busconfig><type>session</type><listen>unix:path={work}/bus</listen>'
                      '<auth>EXTERNAL</auth><policy context="default"><allow own="*"/>'
                      '<allow send_destination="*"/><allow receive_sender="*"/></policy></busconfig>')
    bus = subprocess.Popen(["dbus-daemon", "--nofork", "--config-file", str(config), "--print-address=1"], stdout=subprocess.PIPE, text=True)
    processes.append(bus)
    assert bus.stdout.readline().startswith("unix:")
    os.environ["DBUS_SESSION_BUS_ADDRESS"] = f"unix:path={work}/bus"
    spawn(["rex", "server", "run", "--listen", endpoint, "--restore=false", "--config", "/dev/null", "--state-dir", str(work / "rex-state")], "rex")
    wait(lambda: (work / "rex.sock").exists(), "private Rex did not start")
    sid = accept.api("session.create", label="Sounds acceptance", initial_windows=[
        {"window_label": f"Tab-{i}", "layout": accept.SHELL} for i in range(6)])["session_id"]
    windows = accept.api("session.view", session_id=sid)["windows"]
    program = work / "report.py"
    program.write_text("import pathlib,sys,time\np=pathlib.Path(sys.argv[1]); old=''; p.with_suffix('.ready').touch()\n"
                       "while True:\n text=p.read_text() if p.exists() else ''\n"
                       " if text != old:\n  old=text; print(text, end='', flush=True)\n time.sleep(.01)\n")
    time.sleep(.5)
    for i, tab in enumerate(windows):
        block = tab["focused_block_id"]
        accept.api("block.event", session_id=sid, block_id=block, event={"type": "paste", "payload": {"text": f"/usr/bin/python3 {program} {work / f'output-{i}'}"}})
        accept.api("block.event", session_id=sid, block_id=block, event={"type": "key", "payload": {"key": "enter", "action": "press", "mods": ""}})
    wait(lambda: all((work / f"output-{i}.ready").exists() for i in range(6)), "private reporters did not start")
    desk = accept.Desktop()
    serial = 0

    def report(index, state):
        global serial
        serial += 1
        (work / f"output-{index}").write_text(f"\x1b]7501;state={state}:app=private-test\x07{serial}\r\n")

    expected = []
    for enabled, desktop in [(False, True), (True, False), (True, True)]:
        # No fallback can escape this PATH, including when the first fake fails.
        for name in ("canberra-gtk-play", "pw-play", "paplay"):
            assert Path(shutil.which(name, path=desk.env["PATH"])).resolve() == work / "bin" / name
        for i in range(6):
            report(i, "working")
        time.sleep(.2)
        config = work / "workspace.json"
        config.write_text(json.dumps({"font_family": "IBM Plex Mono", "hosts": [{"label": "Private", "endpoint": endpoint}],
            "sessions": {endpoint: sid}, "notifications": {"play_sounds": enabled, "system_notifications": desktop}}))
        app, window = desk.launch(sid, config, size="1024x700", log=(work / f"client-{enabled}-{desktop}.log").open("w"))
        title = desk.x("getwindowname", window)
        front = next(i for i, tab in enumerate(windows) if title.startswith(tab["label"]))
        back = [i for i in range(6) if i != front]
        assert audio() == expected, "read-back records must not play"
        for state, sound in [("blocked", "dialog-warning"), ("error", "dialog-error"), ("done", "complete")]:
            report(back[0], state)
            if enabled:
                expected.append(["-i", sound])
                wait(lambda: len(audio()) == len(expected), f"missing {state} sound")
            time.sleep(1.15)
            assert audio() == expected
        for i in back:
            report(i, "working")
        time.sleep(.3)
        # Five reporters, same burst: needs-you wins over error and done.
        for i, state in zip(back, ["done", "error", "blocked", "done", "done"]):
            report(i, state)
        if enabled:
            expected.append(["-i", "dialog-warning"])
            wait(lambda: len(audio()) == len(expected), "burst did not play")
        time.sleep(1.3)
        assert audio() == expected, "chorus or delayed replay"
        # The event phase with the switch off has remained completely silent.
        # Now explicitly test Play through the real Settings search.
        if not enabled:
            desk.key("ctrl+comma")
            desk.x("mousemove", "--window", window, 160, 92)
            desk.x("click", 1)
            desk.type("Play sounds")
            desk.key("Return", "Escape")
            time.sleep(.4)
            desk.capture(window, "off-sound-settings")
            (work / "off-event-audio.json").write_text(json.dumps(audio()))
            assert not audio(), "off events must not invoke even a fake player"
            # Verified in the native capture: Play, immediately before the off switch.
            desk.x("mousemove", "--window", window, 821, 244)
            desk.x("click", 1)
            expected.append(["-i", "complete"])
            wait(lambda: audio() == expected, "explicit Play with switch off")
            assert not json.loads(config.read_text())["notifications"].get("play_sounds", False)
            desk.key("Escape")
        # Graceful app close exercises the post-event-loop worker join.
        desk.x("mousemove", "--window", window, 995, 26)
        desk.x("click", 1)
        app.wait(timeout=10)
    (work / "result.json").write_text(json.dumps({"result": "passed", "invocations": audio(),
        "asserted": ["off silent", "three fixed ids", "single highest-priority burst", "no replay", "desktop off", "notification daemon absent", "private fake-only PATH", "explicit Play while off"],
        "not_verified": ["audible playback", "PipeWire/PulseAudio", "real sound themes"]}, indent=2))
    print("PASS off silent; independent channel; fixed ids; severity burst; no replay; fake-only PATH", flush=True)
finally:
    if desk:
        for p in desk.apps:
            stop(p)
        stop(desk.wm)
        stop(desk.server)
        shutil.rmtree(desk.stub, ignore_errors=True)
    for p in reversed(processes):
        stop(p)
