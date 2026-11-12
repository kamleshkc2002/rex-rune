#!/usr/bin/env python3
"""Acceptance checks on real X11 events (Xvfb + openbox, xdotool).

Covers:
  corner-close: the corner above Close closes the window instead of resizing it;
  keys: Return, Space, Tab, Escape and Shift+F10 reach the terminal while a toast shows and
      right after a toolbar click (no default button or focus traversal takes them);
  perf: at 1917x1147 (REX_FRAME_STATS): real btop scrolling keeps p95 frame CPU under 8 ms, and a
      synthetic stress dashboard (every cell box drawing, blocks or braille, recoloured ~30 times
      a second) stays within one 60 Hz frame;
  grid: a Nerd glyph / CJK / box-drawing / right-aligned prompt capture for the grid check.

Environment: REX_BIN is the client to test (default target/debug/rex-rune). REX_SERVER must name a
disposable Rex server (a unix:// socket you started yourself); nothing here starts, stops or
autostarts one, and the script refuses to run without it. It creates and destroys only its own Rex
sessions. Writes captures to REX_CAPTURE_OUT (default in the system temporary directory). The
captures show the shell prompt of that server, so start it with a throwaway HOME and a plain PS1
if the images will be shared.

The scripts that import this module share its helpers: `api`, `Desktop` (Xvfb, openbox and a
`tailscale` that finds no tailnet, so the Add host list never shows or probes a real one) and
`grid_origin`, the pointer position of a terminal's first cell, which every script that selects
text or clicks in the grid uses instead of a hard-coded offset. Pin `pane_padding` in the
private workspace.json (`pin_layout`) so the origin does not move when the default does.
"""
import json, os, re, shlex, shutil, subprocess, sys, tempfile, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BIN = os.environ.get("REX_BIN", str(ROOT / "target/debug/rex-rune"))
OUT = Path(os.environ.get("REX_CAPTURE_OUT", Path(tempfile.gettempdir()) / "rex-rune-acceptance"))
SHELL = {"block": {"flavor": "com.superlogical.terminal.shell"}}


# Layout that pointer scripts depend on, pinned into each private workspace.json instead of
# inherited from the app's defaults: the toolbar is 52 pt tall and a single pane has no card edge,
# so the grid starts PANE_PADDING in from the left and PANE_PADDING below the toolbar.
PANE_PADDING = 8
TOOLBAR_HEIGHT = 52
# JetBrainsMono at font_size 15: 9 px per column and 20 px per row at scale 1.
FONT_SIZE = 15
CELL = (9, 21)


def pin_layout(config, **settings):
    """Write (or update) a private workspace.json with the layout the pointer scripts assume."""
    config = Path(config)
    data = json.loads(config.read_text()) if config.exists() else {}
    data.update(pane_padding=PANE_PADDING, font_size=FONT_SIZE, **settings)
    config.write_text(json.dumps(data))
    return config


def grid_origin(sidebar=0):
    """Window pixel (x, y) of the top-left corner of a single pane's grid.

    `sidebar` is the open sidebar's width in points, 0 when it is closed."""
    return sidebar + PANE_PADDING, TOOLBAR_HEIGHT + PANE_PADDING


def cell_point(column, row, fx=0.5, fy=0.5, sidebar=0):
    """Window pixel inside a cell of the single-pane grid, `fx`/`fy` of the way across it."""
    x0, y0 = grid_origin(sidebar)
    return round(x0 + (column + fx) * CELL[0]), round(y0 + (row + fy) * CELL[1])


def api(method, **payload):
    # Never fall through to the user's own server: REX_SERVER must be set to a disposable one.
    server = os.environ.get("REX_SERVER")
    if not server:
        raise SystemExit("Set REX_SERVER to a disposable Rex server; refusing to use the default one")
    return json.loads(subprocess.check_output(
        ["rex", "--autostart=false", "--server", server, "api", "call", method, json.dumps(payload)],
        text=True, timeout=30))


results = []


def check(name, ok, detail=""):
    results.append(ok)
    print(("PASS " if ok else "FAIL ") + name + (f" — {detail}" if detail else ""), flush=True)


# A btop-like dashboard: panels of box drawing, block-element meters and braille graphs,
# every cell redrawn in 256 colours ~30 times a second for 10 seconds. Synthetic, so the
# capture shows nothing from the machine.
TUI = r"""
import math, os, shutil, sys, time
cols, rows = shutil.get_terminal_size()
out = sys.stdout
out.write("\x1b[?1049h\x1b[?25l")
blocks = " ▁▂▃▄▅▆▇█"
braille = "⠀⣀⣤⣶⣿"
start = time.monotonic()
frame = 0
while time.monotonic() - start < 10:
    buf = ["\x1b[H"]
    for y in range(rows):
        line = []
        for x in range(cols):
            edge_x, edge_y = x % 40 in (0, 39), y % 12 in (0, 11)
            if edge_x and edge_y:
                ch = "╋" if (x // 40 + y // 12) % 2 else "┼"
            elif edge_y:
                ch = "━" if y % 24 == 0 else "─"
            elif edge_x:
                ch = "│"
            elif y % 12 < 6:
                v = (math.sin((x + frame) / 6.0 + y) + 1) / 2
                ch = blocks[int(v * 8)]
            else:
                v = (math.cos((x - frame) / 5.0 + y / 3.0) + 1) / 2
                ch = braille[int(v * 4)]
            line.append(f"\x1b[38;5;{(x + y + frame) % 216 + 16}m{ch}")
        buf.append("".join(line))
        if y < rows - 1:
            buf.append("\r\n")
    out.write("".join(buf))
    out.flush()
    frame += 1
    time.sleep(1 / 30)
out.write("\x1b[0m\x1b[?25h\x1b[?1049l")
"""


def terminal_text(sid, block):
    return api("com.superlogical.terminal.format", session_id=sid, block_id=block,
               args={"format": "text", "trim": True, "unwrap": True})["content"]


def wait_text(sid, block, needle, timeout=6):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        text = terminal_text(sid, block)
        if needle in text:
            return text
        time.sleep(0.1)
    return terminal_text(sid, block)


class Desktop:
    def __init__(self, size="1920x1200"):
        read_fd, write_fd = os.pipe()
        self.server = subprocess.Popen(["Xvfb", "-displayfd", str(write_fd), "-screen", "0", f"{size}x24", "-nolisten", "tcp"],
                                       pass_fds=(write_fd,), stderr=subprocess.DEVNULL)
        os.close(write_fd)
        with os.fdopen(read_fd) as pipe:
            self.display = ":" + pipe.readline().strip()
        self.env = dict(os.environ, DISPLAY=self.display, REX_RUNE_NO_UPDATE_CHECK="1")
        self.env.pop("WAYLAND_DISPLAY", None)
        # The Add host list runs `tailscale status`: give the client one that finds no tailnet.
        self.stub = tempfile.mkdtemp(prefix="rex-no-tailscale-")
        stub = Path(self.stub) / "tailscale"
        stub.write_text("#!/bin/sh\nexit 1\n")
        stub.chmod(0o755)
        self.env["PATH"] = f"{self.stub}{os.pathsep}{self.env.get('PATH', '')}"
        self.wm = subprocess.Popen(["openbox"], env=self.env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(0.8)
        self.apps = []

    def x(self, *args):
        return subprocess.check_output(["xdotool", *map(str, args)], env=self.env, text=True, timeout=15).strip()

    def launch(self, sid, config, size="1440x900", extra_env=None, log=None, binary=None):
        env = dict(self.env, **(extra_env or {}))
        app = subprocess.Popen([binary or BIN, "--session", sid, "--config", str(config), "--window-size", size], env=env,
                               stdout=log or subprocess.DEVNULL, stderr=log or subprocess.DEVNULL)
        self.apps.append(app)
        window = self.x("search", "--sync", "--pid", app.pid)
        self.x("windowmove", window, 0, 0)
        self.x("windowfocus", window)
        time.sleep(1.5)
        return app, window

    def key(self, *strokes):
        for s in strokes:
            self.x("key", s)
            time.sleep(0.15)

    def type(self, text):
        self.x("type", "--delay", "8", text)
        time.sleep(0.15)

    def capture(self, window, name):
        OUT.mkdir(parents=True, exist_ok=True)
        g = dict(l.split("=") for l in self.x("getwindowgeometry", "--shell", window).splitlines() if "=" in l)
        self.x("mousemove", "--window", window, int(g["WIDTH"]) // 2, int(g["HEIGHT"]) // 2)
        time.sleep(0.3)
        subprocess.run(["import", "-window", window, str(OUT / f"{name}.png")], env=self.env, check=True)

    def close(self):
        for p in self.apps:
            p.terminate()
        for p in (self.wm, self.server):
            p.terminate()
        shutil.rmtree(self.stub, ignore_errors=True)


def session(label):
    sid = api("session.create", label=label, initial_windows=[{"window_label": "Shell", "layout": SHELL}])["session_id"]
    block = api("session.view", session_id=sid)["windows"][0]["focused_block_id"]
    return sid, block


def run():
    owned = []
    desk = Desktop()
    tmp = tempfile.TemporaryDirectory(prefix="rex-accept-", dir=os.environ.get("TMPDIR", "/tmp"))
    try:
        if os.environ.get("REX_ACCEPT_ONLY") != "perf":
            # keys: keys reach the PTY while a sticky toast shows and after a toolbar click.
            sid, block = session(f"rex-accept-keys-{os.getpid()}")
            owned.append(sid)
            bad_config = Path("/proc/rex-accept-unwritable/workspace.json")  # saves fail: an error toast
            app, window = desk.launch(sid, bad_config)
            desk.type("cat -vT")
            desk.key("Return")
            time.sleep(0.5)
            desk.key("ctrl+shift+b", "ctrl+shift+b")  # sidebar save fails → "Preferences not saved" toast
            time.sleep(0.4)
            desk.capture(window, "keys-toast")
            g = dict(l.split("=") for l in desk.x("getwindowgeometry", "--shell", window).splitlines() if "=" in l)
            width = int(g["WIDTH"])
            # A toolbar click (the sidebar toggle), then keys.
            desk.x("mousemove", "--window", window, 20, 22)
            desk.x("click", 1)
            time.sleep(0.3)
            desk.x("mousemove", "--window", window, 20, 22)
            desk.x("click", 1)
            time.sleep(0.3)
            desk.type("A")
            desk.key("Tab", "space", "Escape", "shift+F10", "Return")
            text = wait_text(sid, block, "A^I ^[")
            line = next((l for l in text.splitlines() if l.startswith("A^I")), "")
            check("Tab, Space, Escape, Shift+F10, Return reach the PTY (toast shown, after toolbar click)",
                  line.startswith("A^I ^[^[[21;2~") or line.startswith("A^I ^[^[[1;2Q") or "^I ^[" in line, repr(line))
            # The tty echoes the raw line; only a delivered Return makes cat print its `^I` form.
            check("Return reached the PTY (cat printed the submitted line)", bool(line), repr(text[-200:]))
            desk.key("ctrl+c")

            # corner-close: the corner above Close closes the window.
            desk.x("mousemove", "--window", window, width - 1, 1)
            desk.x("click", 1)
            try:
                app.wait(timeout=4)
                check("click 1pt inside the top-right corner closes the window", True)
            except subprocess.TimeoutExpired:
                check("click 1pt inside the top-right corner closes the window", False, "still running")

            # grid: the grid stays on columns with Nerd glyphs, CJK, box drawing and a right prompt.
            sample = ("printf '\\033[2J\\033[H'; printf '%s\\n' "
                      + shlex.quote("界面 wide│cjk│ok \ue0b0 \uf418 nerd ─┼─╋━ box") + "; "
                      + "printf '%*s\\n' $(tput cols) 'right-aligned ❯'; printf 'abcdefghij0123456789\\n'")
            clients = [("grid", BIN)]
            for name, binary in clients:
                app3, window3 = desk.launch(sid, Path(tmp.name) / f"{name}.json", binary=binary)
                desk.key("Right")  # intentional input takes size ownership
                time.sleep(0.5)
                api("block.event", session_id=sid, block_id=block, event={"type": "paste", "payload": {"text": sample}})
                desk.key("Return")
                time.sleep(1.0)
                desk.capture(window3, name)
                app3.terminate()
                app3.wait(timeout=10)

        # perf at 1917x1147: real btop scrolled through its process list (gated at p95 < 8 ms),
        # then the synthetic stress dashboard (every cell recoloured each frame; gated at one
        # 60 Hz frame). btop is never captured: it shows this machine's processes.
        sid2, block2 = session(f"rex-accept-perf-{os.getpid()}")
        owned.append(sid2)
        log_path = Path(tmp.name) / "perf.log"
        pattern = r"p50=([\d.]+)ms p95=([\d.]+)ms max=([\d.]+)ms"
        with open(log_path, "w") as log:
            app2, window2 = desk.launch(sid2, Path(tmp.name) / "perf.json", "1917x1147", {"REX_FRAME_STATS": "1"}, log)
            desk.x("windowsize", window2, 1917, 1147)
            time.sleep(1)
            desk.type("btop")
            desk.key("Return")
            time.sleep(3)
            mark = len(log_path.read_text())
            desk.x("key", "--delay", "60", *(["Down"] * 80), *(["Up"] * 80))
            time.sleep(2.2)
            btop = [tuple(map(float, m)) for m in re.findall(pattern, log_path.read_text()[mark:])]
            desk.key("q")
            time.sleep(1)
            tui = Path(tmp.name) / "dashboard.py"
            tui.write_text(TUI)
            desk.type(f"python3 {shlex.quote(str(tui))}")
            desk.key("Return")
            time.sleep(2)
            mark = len(log_path.read_text())
            time.sleep(4)
            desk.capture(window2, "perf-tui")
            time.sleep(4)
            stress = [tuple(map(float, m)) for m in re.findall(pattern, log_path.read_text()[mark:])]
        if os.environ.get("REX_ACCEPT_LOG"):
            print(log_path.read_text()[-3000:])
        build = f"{Path(BIN).parent.name} build, Xvfb/lavapipe"
        if btop:
            check(f"btop scrolling at 1917x1147: p95 frame CPU < 8 ms ({build})", max(x[1] for x in btop) < 8,
                  f"p95s {[x[1] for x in btop]}")
        else:
            check("btop frame stats collected", False, "no REX_FRAME_STATS output (is btop installed?)")
        if stress:
            check(f"stress dashboard (all {1917 // 9}x{1147 // 21} cells recoloured per frame): p95 < 16.7 ms ({build})",
                  max(x[1] for x in stress) < 16.7, f"p95s {[x[1] for x in stress]}")
        else:
            check("stress frame stats collected", False, "no REX_FRAME_STATS output")
    finally:
        desk.close()
        for sid in owned:
            try:
                api("session.destroy", session_id=sid)
            except subprocess.CalledProcessError:
                pass
        tmp.cleanup()
    return all(results)


if __name__ == "__main__":
    sys.exit(0 if run() else 1)
