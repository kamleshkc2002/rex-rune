#!/usr/bin/env python3
"""Captures the documentation screenshots from the real app, headless.

Two private Rex servers start in a throwaway home with a plain bash prompt: "workstation" with
fixture sessions and "build-box" with none. The release binary attaches to them and renders
each screen through Rune's own tooling (`--script` to click, `--snap` for the image, `--tree`
for every element's rectangle), so no compositor, pointer or desktop is involved, and no
user, host or path from the machine running it appears in an image.

For each screen OUT gets <id>.png (raw), <id>.rects.json (the measured highlight boxes, keyed
a0, a1, ...) and <id>.page.json (the highlight labels). Draw the numbered highlights with
WTFM's annotate tool, for example:

    wtfm-annotate.py apply --page-json OUT/workspace.page.json --png OUT/workspace.png \\
        --rects-file OUT/workspace.rects.json --out docs/images/workspace.png \\
        --legend docs/images/workspace.legend.md

Needs a `rex` server binary, bat, eza and a release build (`cargo build --release`, or REX_BIN
names another build). OUT is DOCS_CAPTURE_OUT, default a new temporary directory (printed at the
end).

Everything stays private: the servers listen on sockets in a throwaway directory, the app and the
shells get a throwaway HOME and XDG directories, the sidebar is opened by the saved config (not by
a click, which would toggle a remembered state), the Add host list sees a `tailscale` that
reports no tailnet (so no machine of the one running this appears or is probed), and the
`cargo test -q` pane prints fixed text so reruns capture the same pixels.
"""
import json
import os
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
APP = Path(os.environ.get("REX_BIN") or ROOT / "target/release/rex-linux")
SIZE, SCALE = "1440x900", "1.5"
PS1 = r"\[\e[38;5;110m\]rex-linux\[\e[0m\] \[\e[38;5;245m\]❯\[\e[0m\] "
# `cargo test -q` in the fixture shell prints fixed text, never a real run: timings, test counts
# and compile output would differ from one capture to the next.
CARGO = r"""cargo() { printf '%s\n' 'running 162 tests' '..................................................' \
  'test result: ok. 162 passed; 0 failed in 0.84s' 'running 196 tests' \
  '....................................................' 'test result: ok. 196 passed; 0 failed in 2.37s'; }"""
SHELL = {"block": {"flavor": "com.superlogical.terminal.shell"}}
# The sidebar's open state is a saved setting (it opens at launch as it was left), so a tap on
# toggle-sidebar would close it again whenever the config says it is open. The config pins it open.
# Add host is reached through the Commands palette by keyboard: tapping its row does not open it.
PALETTE_ADD_HOST = [
    {"t": 1.0, "type": "key", "name": "p", "ctrl": True, "shift": True},
    {"t": 1.5, "type": "type", "text": "Add host"},
    {"t": 2.4, "type": "key", "name": "Enter"},
]

# Each screen: the clicks that reach it, when to take the frame, and what to point at.
# A target is an element key (`key`) or exact visible text (`text`).
SCREENS = [
    {
        "id": "workspace",
        "script": [],
        "at": 2.0,
        "annotations": [
            ({"key": "identity"}, "Current session and host: click to switch or create sessions"),
            ({"key": "tab-track"}, "Tabs are the session's Rex windows; right-click one for its menu"),
            ({"key": "new-tab"}, "New tab (Ctrl Shift T)"),
            ({"key": "commands"}, "Every command, session and tab (Ctrl Shift P)"),
            ({"key": "host-new-session:0"}, "New session on this host in one click"),
        ],
    },
    {
        "id": "switcher",
        "script": [{"t": 1.0, "type": "tap", "key": "identity"}],
        "at": 2.2,
        "annotations": [
            ({"key": "popup-search"}, "Filter sessions, or type a name for the next new session"),
            ({"text": "New session"}, "A host with no sessions offers a new one"),
            ({"key": "group-create:0"}, "+ creates a session on that host"),
            ({"text": "Add host…"}, "Save another Rex server"),
        ],
    },
    {
        "id": "commands",
        "script": [{"t": 1.0, "type": "tap", "key": "commands"}],
        "at": 2.2,
        "annotations": [
            ({"key": "popup-search"}, "Search commands, sessions and tabs"),
            ({"text": "Ctrl Shift N"}, "Shortcuts are listed next to each command"),
        ],
    },
    {
        "id": "add-host",
        "script": PALETTE_ADD_HOST,
        "at": 3.6,
        "annotations": [
            ({"key": "popup-search"}, "Search; machines on your tailnet are listed here when Tailscale answers"),
            ({"text": "Enter an endpoint…"}, "Or type an endpoint by hand"),
        ],
    },

]


def main():
    out = Path(os.environ.get("DOCS_CAPTURE_OUT") or tempfile.mkdtemp(prefix="rex-docs-"))
    out.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="rex-docs-work-"))
    home = work / "home"
    home.mkdir()
    for rc in (".bashrc", ".bash_profile"):
        (home / rc).write_text(f"PS1='{PS1}'\nexport PAGER=cat GIT_PAGER=cat\n{CARGO}\ncd {ROOT}\nclear\n")
    shell_env = dict(os.environ, HOME=str(home), SHELL="/bin/bash", USER="user", LOGNAME="user")
    # No display, no inherited server or XDG locations (they derive from the private HOME), and
    # no background update check.
    for k in list(shell_env):
        if k in ("WAYLAND_DISPLAY", "DISPLAY", "WAYLAND_SOCKET", "REX_SERVER", "REX_CONFIG") or k.startswith(
                ("XDG_CONFIG", "XDG_STATE", "XDG_CACHE", "XDG_DATA", "HERDR_")):
            shell_env.pop(k)
    shell_env["REX_LINUX_NO_UPDATE_CHECK"] = "1"
    # The Add host list runs `tailscale status`. This one finds no tailnet, so the real one's
    # machine names never appear and none of its machines is probed.
    stub = work / "bin"
    stub.mkdir()
    (stub / "tailscale").write_text("#!/bin/sh\nexit 1\n")
    (stub / "tailscale").chmod(0o755)
    app_env = dict(shell_env, PATH=f"{stub}{os.pathsep}{shell_env['PATH']}")
    processes = []
    log = open(work / "processes.log", "w")

    def server(name):
        socket = work / f"{name}.sock"
        endpoint = f"unix://{socket}"
        processes.append(subprocess.Popen(
            ["rex", "server", "run", "--listen", endpoint, "--restore=false", "--state-dir", str(work / f"{name}-state")],
            env=shell_env, cwd=home, stdout=log, stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL))
        deadline = time.monotonic() + 10
        while not socket.exists():
            assert time.monotonic() < deadline, f"private Rex server {name} did not start"
            time.sleep(0.05)
        time.sleep(0.3)
        return endpoint

    try:
        main_ep = server("workstation")
        empty_ep = server("build-box")

        def api(method, **payload):
            return json.loads(subprocess.check_output(
                ["rex", "--autostart=false", "-S", main_ep, "api", "call", method, json.dumps(payload)], text=True))

        def send(block, text):
            subprocess.check_call(["rex", "--autostart=false", "-S", main_ep, "send", "-b", block, text + "\r"], stdout=subprocess.DEVNULL)

        layout = {"split": {"direction": "horizontal", "ratio": 0.5, "before": SHELL,
                            "after": {"split": {"direction": "vertical", "ratio": 0.58, "before": SHELL, "after": SHELL}}}}
        sid = api("session.create", label="rex-linux", initial_windows=[
            {"window_label": "dev", "layout": layout},
            {"window_label": "server", "layout": SHELL},
            {"window_label": "notes", "layout": SHELL}])["session_id"]
        for label, tab in (("api-gateway", "logs"), ("dotfiles", "main"), ("blog", "drafts")):
            api("session.create", label=label, initial_windows=[{"window_label": tab, "layout": SHELL}])
        dev = api("session.view", session_id=sid)["windows"][0]
        blocks = [b["block_id"] for b in api("session.list_blocks", session_id=sid)["blocks"]
                  if b.get("window_id") == dev["window_id"]]
        assert len(blocks) == 3, blocks
        for block, label in zip(blocks, ("src", "theme.rs", "tests")):
            api("session.set_block_label", session_id=sid, block_id=block, label=label)
        api("session.focus_window", session_id=sid, window_id=dev["window_id"])
        api("session.focus_block", session_id=sid, block_id=blocks[0])

        config = work / "workspace.json"
        config.write_text(json.dumps({
            "hosts": [{"label": "workstation", "endpoint": main_ep}, {"label": "build-box", "endpoint": empty_ep}],
            "last_host": main_ep, "sessions": {main_ep: sid}, "font_size": 14, "sidebar_open": True}))

        def render(screen, png, tree):
            script = work / f"{screen['id']}.script.json"
            script.write_text(json.dumps(screen["script"]))
            subprocess.run([str(APP), "--endpoint", main_ep, "--session", sid, "--config", str(config), "--",
                            "--script", str(script), "--snap", str(png), "--tree", str(tree),
                            "--at", str(screen["at"]), "--size", SIZE, "--scale", SCALE],
                           env=app_env, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)

        # Attach once (the sidebar opens from the config) so the panes take their final size,
        # then run the fixture commands into them.
        render({"id": "warmup", "script": [], "at": 1.5}, work / "warmup.png", work / "warmup.tree.json")
        send(blocks[0], "eza --tree --color=always src")
        send(blocks[1], "bat -pp --color=always -r 54:73 src/app/theme.rs")
        send(blocks[2], "cargo test -q")
        time.sleep(float(os.environ.get("DOCS_CAPTURE_WAIT", "5")))

        for screen in SCREENS:
            png, tree = out / f"{screen['id']}.png", work / f"{screen['id']}.tree.json"
            render(screen, png, tree)
            data = json.loads(tree.read_text())
            width = data["viewport"][0]
            rects, annotations = {}, []
            for i, (target, label) in enumerate(screen["annotations"]):
                x, y, w, h = find(data, target)
                rects[f"a{i}"] = {"found": True, "iw": width, "x": x, "y": y, "w": w, "h": h}
                annotations.append({("selector" if "key" in target else "text"): target.get("key") or target["text"],
                                    "label": label})
            (out / f"{screen['id']}.rects.json").write_text(json.dumps(rects, indent=1))
            (out / f"{screen['id']}.page.json").write_text(json.dumps({"id": screen["id"], "annotations": annotations}, indent=1))
            print(f"captured {screen['id']}", flush=True)
        print(out)
    finally:
        for p in reversed(processes):
            if p.poll() is None:
                p.terminate()
                try:
                    p.wait(5)
                except subprocess.TimeoutExpired:
                    p.kill()
        if os.environ.get("DOCS_CAPTURE_KEEP") != "1":
            shutil.rmtree(work, ignore_errors=True)


def find(tree, target):
    """The rectangle of the first element with this key, or with this exact text or label."""
    for e in tree["elements"]:
        if "key" in target and e.get("key") == target["key"]:
            return e["rect"]
        if "text" in target and target["text"] in (e.get("text"), e.get("label")):
            return e["rect"]
    raise SystemExit(f"no element matches {target}")


if __name__ == "__main__":
    main()
