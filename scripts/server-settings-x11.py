#!/usr/bin/env python3
"""M3.3 Server settings on two private Rex servers and private Xvfb.

Requires built REX_BIN, Rex API 13, Xvfb, openbox, xdotool, ImageMagick,
Tesseract and Pillow (the shared acceptance helper imports it). No native copy,
personal display/server/clipboard, or network Tailscale. Artifacts stay under
~/.cache/agent-ws. Observer API call counts are NOT client wire request counts.
"""
import collections
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
BINARY = os.environ.get("REX_BIN", str(ROOT / "target/release/rex-rune"))
work = Path(tempfile.mkdtemp(prefix="rex-server-settings-", dir=Path.home() / ".cache/agent-ws"))
for tool in ("rex", "Xvfb", "openbox", "xdotool", "import", "magick", "tesseract"):
    assert shutil.which(tool), f"{tool} required"
for name in ("home", "config", "state", "runtime", "cache", "captures"):
    (work / name).mkdir(mode=0o700)
for name in list(os.environ):
    if name.startswith(("REX_", "HERDR_")) or name in ("DISPLAY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS"):
        del os.environ[name]
endpoints = ["unix://" + str(work / f"server-{i}.sock") for i in range(2)]
os.environ.update(HOME=str(work / "home"), XDG_CONFIG_HOME=str(work / "config"),
                  XDG_STATE_HOME=str(work / "state"), XDG_RUNTIME_DIR=str(work / "runtime"),
                  XDG_CACHE_HOME=str(work / "cache"), REX_BIN=BINARY,
                  REX_CAPTURE_OUT=str(work / "captures"), REX_SERVER=endpoints[0])
spec = importlib.util.spec_from_file_location("accept", ROOT / "scripts/rune-acceptance-x11.py")
accept = importlib.util.module_from_spec(spec)
spec.loader.exec_module(accept)
counts = collections.Counter()


def api(index, method, **payload):
    counts[f"{index}:{method}"] += 1
    return json.loads(subprocess.check_output([
        "rex", "--autostart=false", "--server", endpoints[index], "api", "call", method,
        json.dumps(payload)], text=True, timeout=30))


def wait(predicate, description, timeout=15):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        if predicate():
            return
        time.sleep(.08)
    raise AssertionError(description)


def click(x, y):
    desk.x("mousemove", "--window", window, x, y)
    desk.x("click", 1)
    time.sleep(.3)


def capture(name):
    desk.capture(window, name)
    # OCR the actual page, enlarged so small version punctuation remains distinguishable.
    # Keep the original capture too; no fuzzy text matching or assertion retries.
    geometry = dict(line.split("=") for line in desk.x("getwindowgeometry", "--shell", window).splitlines() if "=" in line)
    width, height = int(geometry["WIDTH"]), int(geometry["HEIGHT"])
    panel_width = min(816, width * .92)
    left = round((width - panel_width) / 2 + min(204, panel_width * .3) + 1)
    ocr = work / "captures" / f"{name}-ocr.png"
    subprocess.run(["magick", str(work / "captures" / f"{name}.png"),
                    "-crop", f"{width-left}x{height-64}+{left}+64", "-resize", "300%",
                    "-colorspace", "Gray", "-negate", str(ocr)], check=True)
    words = subprocess.check_output(["tesseract", str(ocr), "stdout"],
                                   stderr=subprocess.DEVNULL, text=True)
    (work / "captures" / f"{name}.txt").write_text(words)
    return words


def search(label):
    click(160, 92)
    desk.key("ctrl+a")
    desk.type(label)
    desk.key("Return")
    time.sleep(.4)


def terminal_size(index):
    return api(index, "com.superlogical.terminal.size", session_id=sessions[index], block_id=blocks[index])


servers, sessions, blocks, records = [], [], [], []
desk = None
try:
    print("Artifacts:", work, flush=True)
    for i in range(2):
        servers.append(subprocess.Popen([
            "rex", "server", "run", "--listen", endpoints[i], "--restore=false", "--config", "/dev/null",
            "--state-dir", str(work / f"rex-state-{i}")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL))
        wait(lambda: (work / f"server-{i}.sock").exists(), f"private server {i} did not start")
        sid = api(i, "session.create", label=["Alpha terminal", "Beta terminal"][i],
                  initial_windows=[{"window_label": "Shell", "layout": accept.SHELL}])["session_id"]
        sessions.append(sid)
        blocks.append(api(i, "session.view", session_id=sid)["windows"][0]["focused_block_id"])
        program = work / f"record-{i}.py"
        program.write_text("import os,tty,termios,select\nold=termios.tcgetattr(0); tty.setraw(0)\n"
                           + f"root={str(work)!r}; i={i}\n"
                           "print('Private Server settings test',flush=True)\n"
                           "open(root+f'/ready-{i}','w').close()\ndata=b''\n"
                           "while not os.path.exists(root+'/stop'):\n"
                           " if select.select([0],[],[],.05)[0]: data+=os.read(0,4096)\n"
                           "termios.tcsetattr(0,termios.TCSANOW,old)\n"
                           "open(root+f'/input-{i}','wb').write(data)\n")
        api(i, "block.event", session_id=sid, block_id=blocks[i],
            event={"type": "paste", "payload": {"text": f"python3 {program}"}})
        api(i, "block.event", session_id=sid, block_id=blocks[i],
            event={"type": "key", "payload": {"key": "enter", "action": "press", "mods": ""}})
        wait(lambda: (work / f"ready-{i}").exists(), f"recorder {i} did not start")
    # Distinct server-reported counts make active-host isolation observable.
    api(1, "session.create", label="Beta extra", initial_windows=[])
    config = work / "workspace.json"
    config.write_text(json.dumps({"font_family": "IBM Plex Mono", "pane_padding": 8,
        "hosts": [{"label": label, "endpoint": endpoint} for label, endpoint in zip(
            ["Alpha private", "Beta private"], endpoints)],
        "sessions": dict(zip(endpoints, sessions))}))
    desk = accept.Desktop()
    app, window = desk.launch(sessions[0], config, size="1024x700", log=(work / "client.log").open("w"))
    for i, label in enumerate(["Alpha private", "Beta private"]):
        if i:
            desk.key("ctrl+k")
            desk.type("Beta terminal")
            desk.key("Return")
            time.sleep(1.)
        before = api(i, "server.status")
        version = api(i, "server.version")
        (work / f"expected-{i}.json").write_text(json.dumps({"status": before, "version": version}, indent=2))
        size_before = terminal_size(i)
        view_before = api(i, "session.view", session_id=sessions[i])
        desk.key("ctrl+comma")
        search("Server")
        words = capture(f"host-{i}-top")
        assert label in words, words
        search("Server version")
        words = capture(f"host-{i}-version")
        assert version["rex_version"] in words, words
        search("Process ID")
        words = capture(f"host-{i}-process")
        assert str(before["pid"]) in words, words
        assert re.search(r"Session count\s+" + str(before["session_count"]) + r"\b", words), words
        search("Listeners")
        words = capture(f"host-{i}-listeners")
        assert "Listener 1" in words, words
        desk.key("Escape")  # Search -> section list, not out of Settings.
        click(180, 182)  # Interface, next group after Server.
        desk.key("alt+Left")
        assert "Server" in capture(f"host-{i}-back")
        desk.key("alt+Right")
        assert "Interface" in capture(f"host-{i}-forward")
        desk.key("alt+Left", "Escape")
        after = api(i, "server.status")
        assert before["pid"] == after["pid"] and before["started_at"] == after["started_at"]
        assert before["session_count"] == after["session_count"] == i + 1
        assert before["listeners"] == after["listeners"]
        assert terminal_size(i) == size_before, "page use changed terminal size/owner"
        assert api(i, "session.view", session_id=sessions[i]) == view_before, "page use changed session"
        records.append({"host": label, "before": before, "after": after, "version": version,
                        "size_before": size_before, "size_after": terminal_size(i)})
        print(f"PASS {label}: actual version/PID/count/listener; history; unchanged session/size/owner", flush=True)
    desk.x("windowsize", window, 640, 480)
    time.sleep(.5)
    desk.key("ctrl+comma")
    search("Server")
    assert "Beta private" in capture("minimum-current-host")
    search("Tailscale address")
    assert "Not reported" in capture("minimum-tailscale")
    desk.key("Escape", "Escape")
    (work / "stop").touch()
    wait(lambda: all((work / f"input-{i}").exists() for i in range(2)), "recorders did not finish")
    received = [(work / f"input-{i}").read_bytes() for i in range(2)]
    assert received == [b"", b""], received
    servers[1].terminate()
    servers[1].wait(timeout=5)
    time.sleep(.8)
    desk.key("ctrl+comma")
    search("Server")
    words = capture("minimum-disconnected")
    assert "Disconnected:" in words and "last reported" in words, words
    (work / "results.json").write_text(json.dumps({"records": records,
        "observer_api_calls": dict(counts), "client_wire_requests": "not intercepted",
        "native_clipboard": "not used", "pty_input_hex": [data.hex() for data in received]}, indent=2))
    print("PASS 640x480 current host and disconnected last-reported facts; both PTYs empty; no native clipboard", flush=True)
finally:
    (work / "stop").touch()
    if desk:
        desk.close()
    for server in servers:
        if server.poll() is None:
            server.terminate()
            server.wait(timeout=5)
