#!/usr/bin/env python3
"""Records a highlight demo of the real app: a private headless labwc (wlroots, Vulkan on the
GPU) at 1920x1080 and scale 1.25, a private Rex server with fixture sessions, the release binary
on Wayland driven by a virtual pointer and keyboard (`vptr`, a tiny wlroots client), and
wf-recorder at 60 fps.

Nothing touches the user's desktop or their Rex server: the server listens on a private socket
and every `rex` call names it with autostart off. REX_BIN selects the client (default
target/release/rex-linux), which runs with a `tailscale` that finds no tailnet and no update
check. `DEMO_PROBE=1` skips the recording and
writes screenshots at the interesting moments instead, for calibrating coordinates.

Needs a `rex` server binary, labwc, wlr-randr, wf-recorder, grim, xkbcli, ffmpeg, the `vptr`
helper (`cargo build --release` in tools/vptr, or DEMO_VPTR) and, for the fixture panes,
btop, bat, eza, tree and git.
Output in $XDG_CACHE_HOME/rex-linux/demo (default ~/.cache/rex-linux/demo): raw.mkv, beats.json (seconds from the start of the recording)
and rex-linux-demo.mp4, the trimmed clip.
"""
import json
import os
import shutil
import signal
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORK = Path(os.environ.get("DEMO_DIR", Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "rex-linux/demo"))
REPO = str(ROOT)
SCALE = 1.25
W, H = 1920, 1080
PROBE = os.environ.get("DEMO_PROBE") == "1"
APP = Path(os.environ.get("REX_BIN") or ROOT / "target/release/rex-linux")

# Evdev key codes on a US layout: (code, needs shift).
KEYS = {"Enter": (28, False), "Escape": (1, False), "Tab": (15, False), "BackSpace": (14, False),
        "Up": (103, False), "Down": (108, False), "Left": (105, False), "Right": (106, False),
        "PageUp": (104, False), "PageDown": (109, False)}
CHARS = {}
for row, codes in (("qwertyuiop", range(16, 26)), ("asdfghjkl", range(30, 39)), ("zxcvbnm", range(44, 51))):
    for ch, code in zip(row, codes):
        CHARS[ch], CHARS[ch.upper()] = (code, False), (code, True)
for ch, code in zip("1234567890", range(2, 12)):
    CHARS[ch] = (code, False)
for ch, code in zip("!@#$%^&*()", range(2, 12)):
    CHARS[ch] = (code, True)
CHARS.update({" ": (57, False), "-": (12, False), "_": (12, True), "=": (13, False), "+": (13, True),
              "[": (26, False), "]": (27, False), ";": (39, False), ":": (39, True), "'": (40, False),
              '"': (40, True), ",": (51, False), "<": (51, True), ".": (52, False), ">": (52, True),
              "/": (53, False), "?": (53, True), "\\": (43, False), "|": (43, True), "~": (41, True)})


def main():
    if WORK.exists():
        shutil.rmtree(WORK)
    WORK.mkdir(parents=True)
    socket = WORK / "rex.sock"
    endpoint = f"unix://{socket}"
    processes = []
    log = open(WORK / "processes.log", "w")

    def spawn(args, **kw):
        p = subprocess.Popen(args, stdout=log, stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL, **kw)
        processes.append(p)
        return p

    def api(method, **payload):
        return json.loads(subprocess.check_output(["rex", "--autostart=false", "-S", endpoint, "api", "call", method, json.dumps(payload)], text=True))

    def send(block, text):
        subprocess.check_call(["rex", "--autostart=false", "-S", endpoint, "send", "-b", block, text], stdout=subprocess.DEVNULL)

    try:
        spawn(["rex", "server", "run", "--listen", endpoint, "--restore=false", "--state-dir", str(WORK / "state")],
              env=dict(os.environ, PAGER="cat", GIT_PAGER="cat", REPO=REPO))
        deadline = time.monotonic() + 10
        while not socket.exists():
            assert time.monotonic() < deadline, "private Rex server did not start"
            time.sleep(0.05)
        time.sleep(0.3)

        shell = {"block": {"flavor": "com.superlogical.terminal.shell"}}

        def session(label, tabs):
            sid = api("session.create", label=label, initial_windows=[{"window_label": t, "layout": shell} for t in tabs])["session_id"]
            view = api("session.view", session_id=sid)["windows"]
            return sid, [w["focused_block_id"] for w in view]

        cd = f"cd {REPO} && clear\r"
        main_sid, (dev, server, notes) = session("rex-linux", ["dev", "server", "notes"])
        api("session.set_block_label", session_id=main_sid, block_id=dev, label="shell")
        _, (infra,) = session("infra", ["monitor"])
        _, (api_gw,) = session("api-gateway", ["logs"])
        _, (dots,) = session("dotfiles", ["main"])
        _, (blog,) = session("blog", ["drafts"])
        time.sleep(1.0)
        for block in [dev, server, notes, api_gw, dots, blog]:
            send(block, cd)
        send(infra, "btop\r")
        send(server, "git log --oneline --graph --color=always -40\r")
        send(notes, "bat --paging=never --style=plain --color=always README.md | head -60\r")
        send(api_gw, "git log --stat --color=always -3 | head -60\r")
        send(dots, "eza -la --color=always src/app\r")
        send(blog, "tree -C -L 2 docs | head -50\r")
        api("session.focus_window", session_id=main_sid, window_id=api("session.view", session_id=main_sid)["windows"][0]["window_id"])

        # A private headless compositor on the GPU: nothing appears on the user's desktop.
        wl = WORK / "labwc"
        wl.mkdir()
        (wl / "rc.xml").write_text('<?xml version="1.0"?><labwc_config><core><decoration>client</decoration></core></labwc_config>')
        marker = WORK / "wayland-display"
        base = {k: v for k, v in os.environ.items() if k not in ("WAYLAND_DISPLAY", "DISPLAY", "WAYLAND_SOCKET")}
        spawn(["labwc", "-C", str(wl), "-s", f"sh -c 'echo $WAYLAND_DISPLAY > {marker}'"],
              env=dict(base, WLR_BACKENDS="headless", WLR_RENDERER="vulkan", WLR_HEADLESS_OUTPUTS="1",
                       XCURSOR_THEME="Adwaita", XCURSOR_SIZE="24"))
        deadline = time.monotonic() + 10
        while not marker.exists() or not marker.read_text().strip():
            assert time.monotonic() < deadline, "headless labwc did not start"
            time.sleep(0.05)
        env = dict(base, WAYLAND_DISPLAY=marker.read_text().strip(), XCURSOR_THEME="Adwaita", XCURSOR_SIZE="24")
        subprocess.check_call(["wlr-randr", "--output", "HEADLESS-1", "--custom-mode", f"{W}x{H}@60", "--scale", str(SCALE)], env=env)
        config = WORK / "workspace.json"
        config.write_text(json.dumps({
            "hosts": [{"label": "workstation", "endpoint": endpoint}],
            "last_host": endpoint, "sessions": {endpoint: main_sid}, "font_size": 14}))
        stub = WORK / "bin"
        stub.mkdir()
        (stub / "tailscale").write_text("#!/bin/sh\nexit 1\n")
        (stub / "tailscale").chmod(0o755)
        spawn([str(APP), "--endpoint", endpoint, "--session", main_sid,
               "--config", str(config), "--window-size", f"{int(W / SCALE)}x{int(H / SCALE)}"],
              env=dict(env, PATH=f"{stub}{os.pathsep}{env['PATH']}", REX_LINUX_NO_UPDATE_CHECK="1"))
        keymap = WORK / "us.xkb"
        keymap.write_text(subprocess.check_output(["xkbcli", "compile-keymap", "--layout", "us"], text=True))
        driver = subprocess.Popen([os.environ.get("DEMO_VPTR", str(ROOT / "tools/vptr/target/release/vptr")),
                                   str(W), str(H), str(keymap)], env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        processes.append(driver)
        assert driver.stdout.readline().strip() == "ready"

        def drive(*lines):
            driver.stdin.write("".join(f"{line}\n" for line in lines))
            driver.stdin.flush()

        time.sleep(2.0)
        pos = [W / SCALE / 2, H / SCALE * 0.4]
        drive(f"move {round(pos[0] * SCALE)} {round(pos[1] * SCALE)}", "down 272", "up 272")
        time.sleep(1.0)

        t0 = None
        beats = {}
        recorder = None
        if not PROBE:
            recorder = subprocess.Popen(
                ["wf-recorder", "-y", "-o", "HEADLESS-1", "-r", "60", "-c", "libx264", "-p", "preset=ultrafast", "-p", "crf=8",
                 "-x", "yuv444p", "-f", str(WORK / "raw.mkv")], env=env, stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL, stderr=open(WORK / "recorder.log", "w"))
            time.sleep(0.8)
        t0 = time.monotonic()

        def beat(name):
            beats[name] = round(time.monotonic() - t0, 3)
            print(f"{beats[name]:7.2f}  {name}", flush=True)

        shots = []

        def snap(name):
            if PROBE:
                shots.append(name)
                subprocess.run(["grim", "-o", "HEADLESS-1", str(WORK / f"probe-{len(shots):02}-{name}.png")], env=env)

        def wait(s):
            time.sleep(s)

        def move(tx, ty, dur=0.45, drag=False):
            """An eased (cubic in-out) pointer move in logical points, one step per 60 Hz frame."""
            sx, sy = pos
            steps = max(2, int(dur * 60))
            start = time.monotonic()
            for i in range(1, steps + 1):
                t = i / steps
                e = 4 * t ** 3 if t < 0.5 else 1 - (-2 * t + 2) ** 3 / 2
                drive(f"move {round((sx + (tx - sx) * e) * SCALE)} {round((sy + (ty - sy) * e) * SCALE)}")
                time.sleep(max(0, start + i / 60 - time.monotonic()))
            pos[:] = [tx, ty]

        def click(button=1):
            code = {1: 272, 3: 273}[button]
            drive(f"down {code}")
            time.sleep(0.05)
            drive(f"up {code}")

        def press(code, mods=0, hold=0.03):
            held = [c for bit, c in ((4, 29), (1, 42), (8, 56)) if mods & bit]
            for c in held:
                drive(f"kdown {c}")
            if mods:
                drive(f"mods {mods}")
            drive(f"kdown {code}")
            time.sleep(hold)
            drive(f"kup {code}")
            for c in held:
                drive(f"kup {c}")
            if mods:
                drive("mods 0")

        def key(stroke, after=0.25):
            *mods, name = stroke.split("+")
            mask = sum({"ctrl": 4, "shift": 1, "alt": 8}[m] for m in mods)
            code, shifted = KEYS[name] if name in KEYS else CHARS[name]
            press(code, mask | (1 if shifted else 0))
            wait(after)

        def type_(text, delay=38, after=0.15):
            for ch in text:
                code, shifted = CHARS[ch]
                press(code, 1 if shifted else 0, 0.012)
                time.sleep(delay / 1000)
            wait(after)

        LW, LH = W / SCALE, H / SCALE
        top, mid = 52, (52 + LH - 10) / 2

        # 1. A command in the first pane.
        beat("start")
        wait(0.6)
        type_("git log --oneline --graph --color=always -18")
        key("Enter", 0.9)
        snap("log")
        # 2. Split right and watch btop come up.
        beat("split-right")
        key("ctrl+shift+d", 0.7)
        type_("btop", 45)
        key("Enter", 1.4)
        snap("btop")
        # 3. Back to the left, split it down.
        beat("split-down")
        key("alt+Left", 0.35)
        key("ctrl+alt+d", 0.7)
        type_("bat -pp -r 54:70 $REPO/src/app/theme.rs", 32)
        key("Enter", 0.9)
        snap("bat")
        # 4. Drag the column divider with the mouse, then back.
        beat("drag")
        divider = LW / 2
        move(divider, mid + 120, 0.55)
        wait(0.2)
        drive("down 272")
        move(divider - 200, mid + 120, 0.6)
        wait(0.15)
        move(divider - 120, mid + 120, 0.45)
        drive("up 272")
        wait(0.4)
        snap("dragged")
        # 5. The pane menu from the btop card's header; hover down its rows.
        beat("pane-menu")
        card_x = (divider - 120 + LW - 10) / 2
        move(card_x - 80, top + 15, 0.5)
        click()
        wait(0.35)
        click(3)
        wait(0.45)
        snap("menu")
        for dy in (40, 76, 112, 148):
            move(card_x - 30, top + 15 + dy, 0.22)
            wait(0.18)
        snap("menu-hover")
        key("Escape", 0.3)
        # 6. Zoom the focused pane and restore it.
        beat("zoom")
        key("ctrl+shift+Enter", 0.9)
        snap("zoomed")
        key("ctrl+shift+Enter", 0.8)
        # 7. Commands: search and walk the list.
        beat("commands")
        key("ctrl+shift+p", 0.5)
        type_("tab", 90, 0.35)
        snap("commands")
        for _ in range(3):
            key("Down", 0.22)
        key("Up", 0.3)
        key("Escape", 0.35)
        # 8. Tabs: the active pill slides.
        beat("tabs")
        key("ctrl+PageDown", 0.7)
        snap("tab2")
        key("ctrl+PageDown", 0.7)
        key("ctrl+PageUp", 0.5)
        key("ctrl+PageUp", 0.6)
        # 9. The sidebar slides in; switch sessions with the mouse.
        beat("sidebar")
        key("ctrl+shift+b", 0.7)
        SIDEBAR_ROWS = json.loads(os.environ.get("DEMO_ROWS", "[190, 224, 258, 291, 325]"))
        snap("sidebar")
        move(120, SIDEBAR_ROWS[1], 0.5)
        click()
        wait(1.0)
        snap("infra")
        move(120, SIDEBAR_ROWS[2], 0.4)
        click()
        wait(0.9)
        snap("api")
        move(120, SIDEBAR_ROWS[0], 0.4)
        click()
        wait(0.6)
        # 10. The switcher: walk sessions by keyboard, open one, come home.
        beat("switcher")
        move(LW * 0.62, LH * 0.7, 0.5)
        key("ctrl+k", 0.5)
        for _ in range(3):
            key("Down", 0.25)
        snap("switcher")
        key("Up", 0.3)
        key("Escape", 0.35)
        key("ctrl+shift+b", 0.8)
        beat("end")
        wait(1.2)
        snap("end")

        if recorder:
            recorder.send_signal(signal.SIGINT)
            recorder.wait(30)
        (WORK / "beats.json").write_text(json.dumps(beats, indent=1))
        if recorder:
            # The highlight clip: skip the idle second before typing, fade in and out, H.264 at 60 fps.
            raw = WORK / "raw.mkv"
            duration = float(subprocess.check_output(["ffprobe", "-v", "error", "-show_entries", "format=duration",
                                                      "-of", "csv=p=0", str(raw)], text=True))
            length = round(duration - 1.05, 2)
            subprocess.check_call(["ffmpeg", "-v", "error", "-y", "-ss", "1.0", "-i", str(raw), "-t", str(length), "-vf",
                                   f"fps=60,format=yuv420p,fade=t=in:st=0:d=0.35,fade=t=out:st={length - 0.5}:d=0.5",
                                   "-c:v", "libx264", "-preset", "slow", "-crf", "18", "-profile:v", "high",
                                   "-movflags", "+faststart", "-an", str(WORK / "rex-linux-demo.mp4")])
    finally:
        for p in reversed(processes):
            if p.poll() is None:
                p.terminate()
                try:
                    p.wait(5)
                except subprocess.TimeoutExpired:
                    p.kill()


if __name__ == "__main__":
    main()
