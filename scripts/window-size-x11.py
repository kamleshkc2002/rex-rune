#!/usr/bin/env python3
"""Check remembered window size, maximize exclusion, zoom and CLI precedence on X11.

Requires a built client (REX_BIN, default target/debug/rex-linux), Xvfb, openbox,
xdotool, xprop and ImageMagick's import. Uses a private display, HOME and preferences
and a nonexistent socket: no Rex server or daily sessions are touched. Captures and
logs remain in the printed temporary directory.
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
BINARY = os.environ.get("REX_BIN", str(ROOT / "target/debug/rex-linux"))
for tool in ("Xvfb", "openbox", "xdotool", "xprop", "import"):
    assert shutil.which(tool), f"{tool} required"
work = Path(tempfile.mkdtemp(prefix="rex-window-size-x11-"))
for name in ("home", "config", "state", "runtime", "cache", "data", "captures"):
    (work / name).mkdir(mode=0o700)
for name in list(os.environ):
    if name.startswith(("REX_", "HERDR_")) or name in ("DBUS_SESSION_BUS_ADDRESS", "DISPLAY", "WAYLAND_DISPLAY"):
        del os.environ[name]
os.environ.update(HOME=str(work / "home"), XDG_CONFIG_HOME=str(work / "config"),
                  XDG_STATE_HOME=str(work / "state"), XDG_RUNTIME_DIR=str(work / "runtime"),
                  XDG_CACHE_HOME=str(work / "cache"), XDG_DATA_HOME=str(work / "data"),
                  REX_BIN=BINARY, REX_CAPTURE_OUT=str(work / "captures"))
spec = importlib.util.spec_from_file_location("accept", ROOT / "scripts/rune-acceptance-x11.py")
accept = importlib.util.module_from_spec(spec)
spec.loader.exec_module(accept)


def wait(predicate, description, timeout=10):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(.08)
    raise AssertionError(description)


def geometry(window):
    values = dict(line.split("=") for line in desk.x("getwindowgeometry", "--shell", window).splitlines())
    return [int(values["WIDTH"]), int(values["HEIGHT"])]


def saved():
    return json.loads(config.read_text()).get("window_size")


def launch(*args):
    with (work / "client.log").open("a") as log:
        app = subprocess.Popen([BINARY, "--config", str(config), *args], env=desk.env,
                               stdout=log, stderr=log)
    desk.apps.append(app)
    window = desk.x("search", "--sync", "--pid", app.pid)
    desk.x("windowactivate", "--sync", window)
    return app, window


def close(app, window):
    desk.x("windowactivate", "--sync", window)
    desk.x("key", "alt+F4")
    app.wait(timeout=10)


desk = None
try:
    print("Private window-size artifacts:", work, flush=True)
    config = work / "workspace.json"
    config.write_text(json.dumps({"hosts": [{"label": "Offline test", "endpoint": "unix://" + str(work / "absent.sock")}],
                                  "ui_scale": 1.5}))
    # Pin WM bindings instead of depending on the machine's Openbox defaults.
    wm_config = work / "config/openbox/rc.xml"
    wm_config.parent.mkdir()
    wm_config.write_text('''<openbox_config xmlns="http://openbox.org/3.4/rc">
      <keyboard>
        <keybind key="A-F10"><action name="ToggleMaximize"/></keybind>
        <keybind key="A-F4"><action name="Close"/></keybind>
      </keyboard>
    </openbox_config>''')
    desk = accept.Desktop()
    app, window = launch()
    wait(lambda: geometry(window) == [1440, 900], "first launch uses 1440x900")
    wait(lambda: saved() == [1440, 900], "idle startup size was not saved")
    desk.x("windowsize", window, 1100, 750)
    wait(lambda: geometry(window) == [1100, 750], "resize not applied")
    wait(lambda: saved() == [1100, 750], "window points not saved at 1.5x interface zoom")
    stamp = config.stat().st_mtime_ns
    time.sleep(1.3)
    assert config.stat().st_mtime_ns == stamp, "idle frames rewrote preferences"
    desk.capture(window, "normal")
    desk.x("key", "alt+F10")
    wait(lambda: "_NET_WM_STATE_MAXIMIZED_VERT" in subprocess.check_output(
        ["xprop", "-id", window, "_NET_WM_STATE"], env=desk.env, text=True), "window did not maximize")
    wait(lambda: geometry(window) != [1100, 750], "maximized geometry not applied")
    time.sleep(1.3)
    assert saved() == [1100, 750], "maximized size replaced normal size"
    desk.capture(window, "maximized")
    close(app, window)
    app, window = launch()
    wait(lambda: geometry(window) == [1100, 750], "relaunch did not restore normal size")
    close(app, window)
    app, window = launch("--window-size", "1280x720")
    wait(lambda: geometry(window) == [1280, 720], "CLI size did not override saved size")
    close(app, window)
    app, window = launch("--window-size", "320x200")
    wait(lambda: geometry(window) == [640, 480], "minimum size not enforced")
    close(app, window)
    print("PASS default, resize, idle save, zoom, maximize, relaunch, CLI override and minimum", flush=True)
finally:
    if desk:
        desk.close()
