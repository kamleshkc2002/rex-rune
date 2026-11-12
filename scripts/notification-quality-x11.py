#!/usr/bin/env python3
"""M3.4 notifications: private bus, fake daemon, Rex, HOME/XDG and Xvfb only.

Requires built REX_BIN, Rex API 13, Xvfb/openbox/xdotool/ImageMagick/Tesseract,
Pillow, and /usr/bin/python3 with dbus-python and GLib. Does not use a personal
bus/display/server or show any real notification. Evidence and all temporary
files stay under ~/.cache/agent-ws/rex-m34-notify/. Stops children by PID.
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


def fake_service(work):
    import dbus
    import dbus.service
    from dbus.mainloop.glib import DBusGMainLoop
    from gi.repository import GLib
    DBusGMainLoop(set_as_default=True)
    bus = dbus.SessionBus()
    name = dbus.service.BusName("org.freedesktop.Notifications", bus)
    interface = "org.freedesktop.Notifications"

    def record(**entry):
        with (work / "calls.jsonl").open("a") as stream:
            stream.write(json.dumps(entry) + "\n")

    class Service(dbus.service.Object):
        def __init__(self):
            super().__init__(name, "/org/freedesktop/Notifications")
            self.next_id = 1
            self.reject = False
            self.live = set()

        @dbus.service.method(interface, in_signature="", out_signature="ssss")
        def GetServerInformation(self):
            record(method="GetServerInformation", pid=os.getpid())
            return ("Private fake", "Rex tests", "1", "1.2")

        @dbus.service.method(interface, in_signature="susssasa{sv}i", out_signature="u")
        def Notify(self, app, replaces, icon, summary, body, actions, hints, expire):
            if self.reject:
                self.reject = False
                record(method="NotifyFailed", summary=str(summary))
                raise dbus.exceptions.DBusException("Deliberate failure", name="org.freedesktop.DBus.Error.Failed")
            ident = int(replaces) if replaces in self.live else self.next_id
            self.next_id = max(self.next_id, ident + 1)
            self.live.add(ident)
            record(method="Notify", id=ident, replaces=int(replaces), summary=str(summary),
                   urgency=int(hints["urgency"]), actions=list(actions), live=sorted(self.live), pid=os.getpid())
            return dbus.UInt32(ident)

        @dbus.service.method(interface, in_signature="u", out_signature="")
        def CloseNotification(self, ident):
            self.live.discard(int(ident))
            record(method="CloseNotification", id=int(ident), live=sorted(self.live), pid=os.getpid())
            self.NotificationClosed(ident, 3)

        @dbus.service.signal(interface, signature="uu")
        def NotificationClosed(self, ident, reason):
            pass

        @dbus.service.signal(interface, signature="us")
        def ActionInvoked(self, ident, action):
            pass

        @dbus.service.method("dev.shuv.NotificationTest", in_signature="", out_signature="")
        def RejectNext(self):
            self.reject = True

        @dbus.service.method("dev.shuv.NotificationTest", in_signature="u", out_signature="")
        def Click(self, ident):
            self.ActionInvoked(ident, "default")

    service = Service()
    (work / "service-ready").write_text(str(os.getpid()))
    GLib.MainLoop().run()
    return service


if len(sys.argv) == 3 and sys.argv[1] == "--service":
    fake_service(Path(sys.argv[2]))
    raise SystemExit

ROOT = Path(__file__).resolve().parents[1]
BINARY = os.environ.get("REX_BIN", str(ROOT / "target/release/rex-linux"))
BASE = Path.home() / ".cache/agent-ws/rex-m34-notify"
BASE.mkdir(parents=True, exist_ok=True)
work = Path(tempfile.mkdtemp(prefix="native-", dir=BASE))
for tool in ("rex", "dbus-daemon", "gdbus", "Xvfb", "openbox", "xdotool", "import", "magick", "tesseract"):
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


def calls(method):
    path = work / "calls.jsonl"
    return [item for line in path.read_text().splitlines() if (item := json.loads(line))["method"] == method] if path.exists() else []


def spawn(args, name):
    process = subprocess.Popen(args, stdout=(work / f"{name}.log").open("w"), stderr=subprocess.STDOUT)
    processes.append(process)
    return process


def start_service():
    (work / "service-ready").unlink(missing_ok=True)
    service = spawn(["/usr/bin/python3", __file__, "--service", str(work)], "service")
    wait(lambda: (work / "service-ready").exists(), "fake service did not start")
    return service


def control(method, *args):
    subprocess.run(["gdbus", "call", "--session", "--dest", "org.freedesktop.Notifications",
                    "--object-path", "/org/freedesktop/Notifications", "--method",
                    "dev.shuv.NotificationTest." + method, *map(str, args)], check=True, stdout=subprocess.DEVNULL)


def click(x, y):
    desk.x("mousemove", "--window", window, x, y)
    desk.x("click", 1)
    time.sleep(.35)


def capture(name):
    desk.capture(window, name)
    ocr = work / "captures" / f"{name}-ocr.png"
    subprocess.run(["magick", str(work / "captures" / f"{name}.png"), "-crop", "600x620+310+64", "-resize", "300%", "-colorspace", "Gray", "-negate", str(ocr)], check=True)
    words = subprocess.check_output(["tesseract", str(ocr), "stdout"], text=True, stderr=subprocess.DEVNULL)
    (work / "captures" / f"{name}.txt").write_text(words)
    return words


def settings():
    desk.key("ctrl+comma")
    click(160, 92)
    desk.type("Notification service")
    desk.key("Return", "Escape")
    time.sleep(.5)


try:
    print("Artifacts:", work, flush=True)
    # No activation directories: an absent fake service cannot launch a real daemon.
    config = work / "bus.conf"
    config.write_text(f'<busconfig><type>session</type><listen>unix:path={work}/bus</listen>'
                      '<auth>EXTERNAL</auth><policy context="default"><allow own="*"/>'
                      '<allow send_destination="*"/><allow receive_sender="*"/></policy></busconfig>')
    bus = subprocess.Popen(["dbus-daemon", "--nofork", "--config-file", str(config), "--print-address=1"], stdout=subprocess.PIPE, text=True)
    processes.append(bus)
    assert bus.stdout.readline().startswith("unix:")
    # Keep a stable private address so replacing this bus can exercise reconnects too.
    os.environ["DBUS_SESSION_BUS_ADDRESS"] = f"unix:path={work}/bus"
    server = spawn(["rex", "server", "run", "--listen", endpoint, "--restore=false", "--config", "/dev/null", "--state-dir", str(work / "rex-state")], "rex")
    wait(lambda: (work / "rex.sock").exists(), "private Rex did not start")
    sid = accept.api("session.create", label="Notification acceptance", initial_windows=[
        {"window_label": title, "layout": accept.SHELL} for title in ("Front", "Back")])["session_id"]
    windows = accept.api("session.view", session_id=sid)["windows"]
    program = work / "report.py"
    program.write_text("import pathlib,sys,time\np=pathlib.Path(sys.argv[1]); old=''; p.with_suffix('.ready').touch()\n"
                       "while True:\n text=p.read_text() if p.exists() else ''\n"
                       " if text != old:\n  old=text; print(text, end='', flush=True)\n time.sleep(.05)\n")
    time.sleep(.5)  # Let the two private shells finish starting before the fixture paste.
    for i, tab in enumerate(windows):
        block = tab["focused_block_id"]
        accept.api("block.event", session_id=sid, block_id=block, event={"type": "paste", "payload": {"text": f"/usr/bin/python3 {program} {work / f'output-{i}'}"}})
        accept.api("block.event", session_id=sid, block_id=block, event={"type": "key", "payload": {"key": "enter", "action": "press", "mods": ""}})
    wait(lambda: all((work / f"output-{i}.ready").exists() for i in range(2)), "private reporter programs did not start")
    config = work / "workspace.json"
    config.write_text(json.dumps({"font_family": "IBM Plex Mono", "hosts": [{"label": "Private", "endpoint": endpoint}], "sessions": {endpoint: sid}}))
    desk = accept.Desktop()
    app, window = desk.launch(sid, config, size="1024x700", log=(work / "client.log").open("w"))
    settings()
    assert "Not found" in capture("absent"), "absent service not reported"
    service = start_service()
    # Re-entering the page is an explicit check, with no test sent by opening it.
    desk.key("Escape")
    settings()
    assert "Available" in capture("available")
    assert not calls("Notify"), "opening Settings must not send a notification"
    # The search result scrolls the page to its end, where Desktop service is the last
    # section; this is its card's trailing button at 1024x700.
    click(851, 618)
    wait(lambda: len(calls("Notify")) == 1, "test button did not send")
    test = calls("Notify")[0]
    assert test["summary"] == "Rex test notification" and test["urgency"] == 1 and test["actions"] == []
    assert "sent to the service" in capture("after-test")
    before_test_click = desk.x("getwindowname", window)
    control("Click", test["id"])  # Even an unsolicited action has no navigation target.
    time.sleep(.3)
    assert desk.x("getwindowname", window) == before_test_click
    desk.key("Escape")
    title = desk.x("getwindowname", window)
    front = next(i for i, tab in enumerate(windows) if title.startswith(tab["label"]))
    back = 1 - front
    serial = 0

    def report(index, state):
        global serial
        serial += 1
        (work / f"output-{index}").write_text(f"\x1b]7501;state={state}:app=private-test\x07{serial}\r\n")
        time.sleep(.45)

    report(back, "blocked")
    wait(lambda: len(calls("Notify")) == 2, "blocked did not notify")
    first = calls("Notify")[-1]
    report(back, "working")
    report(back, "blocked")
    wait(lambda: len(calls("Notify")) == 3, "repeat did not notify")
    repeat = calls("Notify")[-1]
    assert repeat["replaces"] == first["id"] == repeat["id"] and repeat["urgency"] == 2
    desk.key("ctrl+Tab")
    wait(lambda: any(c["id"] == repeat["id"] for c in calls("CloseNotification")), "seeing tab did not close its note")
    assert desk.x("getwindowname", window).startswith(windows[back]["label"]), "tab navigation did not settle"
    report(front, "blocked")
    wait(lambda: len(calls("Notify")) == 4, "other tab did not notify")
    control("RejectNext")
    report(front, "error")
    wait(lambda: len(calls("NotifyFailed")) == 1, "Notify failure not exercised")
    report(front, "blocked")
    wait(lambda: len(calls("Notify")) == 5, "worker died after rejected Notify")
    assert calls("Notify")[-1]["replaces"] == calls("Notify")[-2]["id"]
    service.terminate()
    service.wait(timeout=5)
    report(front, "error")  # Drop this note while the daemon is absent.
    service = start_service()
    report(front, "blocked")
    wait(lambda: len(calls("Notify")) == 6, "worker died after daemon restart")
    assert calls("Notify")[-1]["replaces"] == 0, "old daemon id leaked to restarted service"
    assert calls("Notify")[-1]["urgency"] == 2
    assert not any("Error" in c["summary"] for c in calls("Notify")), "failed notes replayed"
    control("Click", calls("Notify")[-1]["id"])
    wait(lambda: any(c["pid"] == service.pid for c in calls("CloseNotification")), "restart note did not close")
    capture("seen-after-restart")
    (work / "result.json").write_text(json.dumps({"result": "passed", "notify": len(calls("Notify")),
        "rejected": len(calls("NotifyFailed")), "close": len(calls("CloseNotification")),
        "not_exercised": ["real desktop presentation", "Wayland activation", "client wire command counts", "whole-app bus restart"]}, indent=2))
    print("PASS absent/present health, explicit normal test, repeat replacement, critical urgency, click/navigation/close, Notify rejection, service restart and no replay", flush=True)
finally:
    if desk:
        desk.close()
    for process in reversed(processes):
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
