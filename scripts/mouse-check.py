#!/usr/bin/env python3
"""Real-input check of terminal mouse reporting on a private headless Wayland display.

Run with dbus-run-session -- python3 scripts/mouse-check.py. MOUSE_DIR must be a fresh
directory. The script owns its upstream Rex fixture, compositor and client, so the user's live
sessions, display and settings are never used. REX_BIN selects the client (default
target/release/rex-rune) and MOUSE_VPTR the `vptr` helper (default tools/vptr/target/release/vptr,
built with `cargo build --release` there). Any inherited REX_SERVER is dropped: the script starts
and talks only to its own private server. The client gets a `tailscale` that finds no tailnet and
no update check. Every position is relative to the pane (the click offsets are distances in points
and rows), and the pane padding is pinned in the private config.

A small program in the pane turns on mouse modes and logs the bytes it receives, so
each step asserts on the exact reports the program sees.
"""
import json
import os
from pathlib import Path
import re
import runpy
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
WORK = Path(os.environ.get("MOUSE_DIR", Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "rex-rune/mouse"))
W, H = 1280, 800
APP = Path(os.environ.get("REX_BIN") or ROOT / "target/release/rex-rune")
KEYMAP = runpy.run_path(str(ROOT / "scripts/demo-record.py"))
REPORT = re.compile(rb"\x1b\[<(\d+);(\d+);(\d+)([Mm])")

ECHO = r"""
import os, sys, termios, tty
modes, log = sys.argv[1].split(","), open(sys.argv[2], "ab", buffering=0)
on = "".join(f"\x1b[?{m}h" for m in modes)
off = "".join(f"\x1b[?{m}l" for m in modes)
saved = termios.tcgetattr(0)
os.write(1, ("\x1b[?1049h\x1b[2J\x1b[Hmouse echo: " + sys.argv[1] + on).encode())
tty.setraw(0)
try:
    while (data := os.read(0, 4096)) != b"q":
        log.write(data)
finally:
    os.write(1, (off + "\x1b[?1049l").encode())
    termios.tcsetattr(0, termios.TCSADRAIN, saved)
"""


def main():
    WORK.mkdir(parents=True, exist_ok=False)
    processes = []
    results = {}
    endpoint = f"unix://{WORK}/rex.sock"
    base = {k: v for k, v in os.environ.items() if k not in (
        "DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET", "HYPRLAND_INSTANCE_SIGNATURE",
        "REX_SERVER", "REX_CONFIG")}
    for name in ("XDG_CONFIG_HOME", "XDG_RUNTIME_DIR", "XDG_DATA_HOME", "XDG_STATE_HOME"):
        folder = WORK / name
        folder.mkdir(mode=0o700)
        base[name] = str(folder)
    base.update(XDG_SESSION_TYPE="wayland", XDG_CURRENT_DESKTOP="labwc", GIO_USE_VFS="local",
                XCURSOR_THEME="Adwaita", XCURSOR_SIZE="24")
    (WORK / "echo.py").write_text(ECHO)

    def call(args, env=None):
        return subprocess.check_output(list(map(str, args)), env=env or base, text=True, timeout=15).strip()

    def spawn(args, name, env=None, **kwargs):
        with (WORK / f"{name}.log").open("w") as log:
            p = subprocess.Popen(list(map(str, args)), env=env or base, stdout=log, stderr=log,
                                 stdin=subprocess.DEVNULL, **kwargs)
        processes.append(p)
        return p

    def await_(predicate, what="fixture"):
        deadline = time.monotonic() + 10
        while not predicate():
            assert time.monotonic() < deadline, f"{what} did not become ready"
            time.sleep(.05)

    def api(method, **payload):
        return json.loads(call(["rex", "--autostart=false", "-S", endpoint, "api", "call", method, json.dumps(payload)]))

    def send(block, command):
        call(["rex", "--autostart=false", "-S", endpoint, "send", "-b", block, command])

    try:
        activation = ["XDG_CONFIG_HOME", "XDG_RUNTIME_DIR", "XDG_DATA_HOME", "XDG_STATE_HOME",
                      "XDG_SESSION_TYPE", "XDG_CURRENT_DESKTOP", "GIO_USE_VFS"]
        call(["dbus-update-activation-environment", "DISPLAY=", "WAYLAND_DISPLAY=",
              "HYPRLAND_INSTANCE_SIGNATURE=", *activation])
        home = WORK / "home"
        home.mkdir()
        spawn(["rex", "server", "run", "--listen", endpoint, "--restore=false", "--state-dir", WORK / "server"],
              "server", dict(base, HOME=str(home), SHELL="/bin/bash"))
        await_(lambda: (WORK / "rex.sock").exists())
        shell = {"block": {"flavor": "com.superlogical.terminal.shell"}}
        sid = api("session.create", label="mouse", initial_windows=[{"window_label": "Mouse", "layout": shell}])["session_id"]
        v = api("session.view", session_id=sid)
        block = next(w for w in v["windows"] if w["window_id"] == v["active_window_id"])["focused_block_id"]

        compositor = WORK / "labwc"
        compositor.mkdir()
        (compositor / "rc.xml").write_text('<labwc_config><core><decoration>client</decoration></core></labwc_config>')
        marker = WORK / "wayland-display"
        spawn(["labwc", "-C", compositor, "-s", f"sh -c 'echo $WAYLAND_DISPLAY > {marker}'"], "labwc",
              dict(base, WLR_BACKENDS="headless", WLR_RENDERER="vulkan", WLR_HEADLESS_OUTPUTS="1"))
        await_(lambda: marker.exists() and marker.read_text().strip())
        env = dict(base, WAYLAND_DISPLAY=marker.read_text().strip())
        call(["dbus-update-activation-environment", "WAYLAND_DISPLAY", *activation], env)
        call(["wlr-randr", "--output", "HEADLESS-1", "--custom-mode", f"{W}x{H}@60", "--scale", "1"], env)
        config = WORK / "workspace.json"
        config.write_text(json.dumps({"hosts": [{"label": "Mouse host", "endpoint": endpoint}],
            "last_host": endpoint, "sessions": {endpoint: sid}, "font_size": 15, "ui_scale": 1.0,
            "pane_padding": 8}))
        # The Add host list runs `tailscale status`: this one finds no tailnet.
        stub = WORK / "bin"
        stub.mkdir()
        (stub / "tailscale").write_text("#!/bin/sh\nexit 1\n")
        (stub / "tailscale").chmod(0o755)
        app_env = dict(env, PATH=f"{stub}{os.pathsep}{env['PATH']}", REX_RUNE_NO_UPDATE_CHECK="1")
        spawn([APP, "--endpoint", endpoint, "--session", sid,
               "--config", config, "--window-size", f"{W}x{H}"], "app", app_env)
        keymap = WORK / "us.xkb"
        keymap.write_text(call(["xkbcli", "compile-keymap", "--layout", "us"]))
        driver = subprocess.Popen([os.environ.get("MOUSE_VPTR", str(ROOT / "tools/vptr/target/release/vptr")), str(W), str(H), str(keymap)],
                                  env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        processes.append(driver)
        assert driver.stdout.readline().strip() == "ready"

        def drive(*lines):
            driver.stdin.write("".join(f"{line}\n" for line in lines))
            driver.stdin.flush()

        def move(x, y):
            drive(f"move {round(x)} {round(y)}")
            time.sleep(.03)

        def click(x, y, button=272, held=()):
            move(x, y)
            mask = sum({29: 4, 42: 1}[c] for c in held)
            drive(*(f"kdown {c}" for c in held), f"mods {mask}")
            time.sleep(.03)
            drive(f"down {button}")
            time.sleep(.05)
            drive(f"up {button}")
            time.sleep(.03)
            drive(*(f"kup {c}" for c in held), "mods 0")

        def drag(x, y, dx, steps=10):
            move(x, y)
            drive("down 272")
            for i in range(1, steps + 1):
                move(x + dx * i / steps, y)
            drive("up 272")

        def key(stroke):
            *mods, name = stroke.split("+")
            mask = sum({"ctrl": 4, "shift": 1, "alt": 8}[m] for m in mods)
            code, shifted = KEYMAP["KEYS"].get(name, KEYMAP["CHARS"].get(name))
            mask |= 1 if shifted else 0
            held = [c for bit, c in ((4, 29), (1, 42), (8, 56)) if mask & bit]
            drive(*(f"kdown {c}" for c in held), f"mods {mask}", f"kdown {code}")
            time.sleep(.025)
            drive(f"kup {code}", *(f"kup {c}" for c in held), "mods 0")

        def shot(label):
            call(["grim", "-o", "HEADLESS-1", WORK / f"{label}.png"], env)

        log_path = WORK / "reports.bin"

        def reports():
            data = log_path.read_bytes() if log_path.exists() else b""
            return [(int(b), int(x), int(y), k.decode()) for b, x, y, k in REPORT.findall(data)]

        def step(name, action, settle=.5):
            before = len(reports())
            action()
            time.sleep(settle)
            new = reports()[before:]
            results[name] = new
            print(f"{name:28} {new}", flush=True)
            return new

        def echo(modes):
            send(block, f"clear; python3 {WORK / 'echo.py'} {modes} {log_path}\r")
            await_(lambda: "mouse echo" in call(["rex", "--autostart=false", "-S", endpoint,
                                                  "capture", "-b", block, "--trim"]), "echo program")
            time.sleep(.6)

        time.sleep(2.5)
        # The pane fills the window below the toolbar.
        px, py = W * .35, H * .45
        click(px, py)
        time.sleep(.5)
        echo("1000,1002,1006")
        shot("01-echo")

        new = step("click", lambda: click(px, py))
        assert [r[3] for r in new] == ["M", "m"] and all(r[0] == 0 for r in new), new
        x0, y0 = new[0][1], new[0][2]

        new = step("click right of it", lambda: click(px + 120, py))
        assert new and new[0][2] == y0 and 9 <= new[0][1] - x0 <= 17, f"120pt is not ~13 cells: {new}"
        new = step("click below it", lambda: click(px, py + 84))
        assert new and new[0][1] == x0 and new[0][2] - y0 == 4, f"84pt is not 4 rows: {new}"

        new = step("drag", lambda: drag(px, py, 100))
        assert new[0][:1] == (0,) and new[0][3] == "M", new
        assert any(r[0] == 32 for r in new), f"no motion with the button held: {new}"
        assert new[-1][3] == "m" and new[-1][1] > x0, new

        new = step("right click opens the pane menu", lambda: click(px, py, 273))
        assert new == [], f"a plain right-click must not report: {new}"
        time.sleep(.4)
        shot("01-right-click-tracking")
        key("Escape")
        time.sleep(.3)
        new = step("shift right click", lambda: click(px, py, 273, held=(42,)))
        assert [(r[0] & ~4, r[3]) for r in new] == [(2, "M"), (2, "m")], new
        new = step("middle click", lambda: click(px, py, 274))
        assert [(r[0], r[3]) for r in new] == [(1, "M"), (1, "m")], new
        new = step("ctrl click", lambda: click(px, py, held=(29,)))
        assert [(r[0], r[3]) for r in new] == [(16, "M"), (16, "m")], new

        def scroll(amount):
            move(px, py)
            for _ in range(6):
                drive(f"scroll {amount / 6}")
                time.sleep(.03)
        new = step("wheel down", lambda: scroll(90))
        assert new and all(r[0] == 65 for r in new), new
        new = step("wheel up", lambda: scroll(-90))
        assert new and all(r[0] == 64 for r in new), new

        new = step("shift drag selects locally", lambda: (click(px, py, held=(42,))))
        assert new == [], f"Shift must not report: {new}"
        shot("02-after-shift")
        new = step("click after shift", lambda: click(px, py))
        assert [r[3] for r in new] == ["M", "m"], f"reporting stuck after Shift: {new}"

        new = step("hover is quiet under 1002", lambda: move(px + 60, py + 40))
        assert new == [], new
        send(block, "q")
        time.sleep(.5)

        echo("1003,1006")
        new = step("hover under 1003", lambda: [move(px + 30 * i, py) for i in range(1, 5)])
        assert new and all(r[0] == 35 and r[3] == "M" for r in new), new
        send(block, "q")
        time.sleep(.5)

        new = step("no tracking: click", lambda: click(px, py))
        assert new == [], new
        click(px, py, 273)
        time.sleep(.6)
        shot("03-right-click-no-tracking")
        key("Escape")

        (WORK / "results.json").write_text(json.dumps(results, indent=2) + "\n")
        print("PASS terminal mouse reporting")
    finally:
        for p in reversed(processes):
            if p.poll() is None:
                p.terminate()
                try:
                    p.wait(6)
                except subprocess.TimeoutExpired:
                    p.kill()
                    p.wait(6)
        for name in ("doc", "gvfs"):
            mount = WORK / "XDG_RUNTIME_DIR" / name
            if os.path.ismount(mount):
                subprocess.run(["fusermount3", "-u", str(mount)], check=True, timeout=10)


if __name__ == "__main__":
    main()
