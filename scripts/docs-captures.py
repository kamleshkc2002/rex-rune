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

Needs a `rex` server binary, bat, eza, cargo and a release build (`cargo build --release`).
OUT is DOCS_CAPTURE_OUT, default a new temporary directory (printed at the end).
"""
import json
import os
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
APP = ROOT / "target/release/rex-linux"
SIZE, SCALE = "1440x900", "1.5"
PS1 = r"\[\e[38;5;110m\]rex-linux\[\e[0m\] \[\e[38;5;245m\]❯\[\e[0m\] "
# `cargo test` in the fixture shell prints only the suites that ran tests, one line per result.
CARGO = r"""cargo() { command cargo "$@" 2>/dev/null | grep -E '^(running [1-9]|\.+$|test result: ok\. [1-9])' | sed -E 's/; 0 ignored.*finished in/ in/'; }"""
SHELL = {"block": {"flavor": "com.superlogical.terminal.shell"}}
OPEN_SIDEBAR = {"t": 0.3, "type": "tap", "key": "toggle-sidebar"}

# Each screen: the clicks that reach it, when to take the frame, and what to point at.
# A target is an element key (`key`) or exact visible text (`text`).
SCREENS = [
    {
        "id": "workspace",
        "script": [OPEN_SIDEBAR],
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
        "script": [OPEN_SIDEBAR, {"t": 1.0, "type": "tap", "key": "identity"}],
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
        "script": [OPEN_SIDEBAR, {"t": 1.0, "type": "tap", "key": "commands"}],
        "at": 2.2,
        "annotations": [
            ({"key": "popup-search"}, "Search commands, sessions and tabs"),
            ({"text": "Ctrl Shift N"}, "Shortcuts are listed next to each command"),
        ],
    },
    {
        "id": "add-host",
        # Commands opens on its first groups; a search brings the Host group's row into view.
        "script": [OPEN_SIDEBAR, {"t": 1.0, "type": "tap", "key": "commands"},
                   {"t": 1.4, "type": "type", "text": "add host"}, {"t": 1.8, "tap_text": "Add host…"}],
        "at": 3.0,
        "annotations": [
            ({"key": "popup-search"}, "Search the machines on your tailnet"),
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
    real_home = Path.home()
    shell_env = dict(os.environ, HOME=str(home), SHELL="/bin/bash", USER="user", LOGNAME="user",
                     CARGO_HOME=os.environ.get("CARGO_HOME", str(real_home / ".cargo")),
                     RUSTUP_HOME=os.environ.get("RUSTUP_HOME", str(real_home / ".rustup")))
    for k in ("WAYLAND_DISPLAY", "DISPLAY", "WAYLAND_SOCKET"):
        shell_env.pop(k, None)
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
                ["rex", "-S", main_ep, "api", "call", method, json.dumps(payload)], text=True))

        def send(block, text):
            subprocess.check_call(["rex", "-S", main_ep, "send", "-b", block, text + "\r"], stdout=subprocess.DEVNULL)

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
            "last_host": main_ep, "sessions": {main_ep: sid}, "font_size": 14}))

        def render(screen, png, tree):
            script = work / f"{screen['id']}.script.json"
            steps = []
            for step in screen["script"]:
                if "tap_text" in step:
                    steps.append({"t": step["t"], "type": "tap", "at": center(find(tree_of(screen["id"]), {"text": step["tap_text"]}))})
                else:
                    steps.append(step)
            script.write_text(json.dumps(steps))
            subprocess.run([str(APP), "--endpoint", main_ep, "--session", sid, "--config", str(config), "--",
                            "--script", str(script), "--snap", str(png), "--tree", str(tree),
                            "--at", str(screen["at"]), "--size", SIZE, "--scale", SCALE],
                           env=shell_env, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)

        trees = {}

        def tree_of(screen_id):
            return trees[screen_id]

        # Attach once with the sidebar open so the panes take their final size, then run the
        # fixture commands into them.
        render({"id": "warmup", "script": [OPEN_SIDEBAR], "at": 1.5}, work / "warmup.png", work / "warmup.tree.json")
        send(blocks[0], "eza --tree --color=always src")
        send(blocks[1], "bat -pp --color=always -r 54:73 src/app/theme.rs")
        send(blocks[2], "cargo test -q")
        time.sleep(float(os.environ.get("DOCS_CAPTURE_WAIT", "15")))

        for screen in SCREENS:
            # A step that clicks visible text needs the previous frame's layout: render once
            # without it to measure, then again with the click.
            if any("tap_text" in s for s in screen["script"]):
                pre = dict(screen, script=[s for s in screen["script"] if "tap_text" not in s],
                           at=max(s["t"] for s in screen["script"]))
                render(pre, work / f"{screen['id']}.pre.png", work / f"{screen['id']}.pre.tree.json")
                trees[screen["id"]] = json.loads((work / f"{screen['id']}.pre.tree.json").read_text())
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


def center(rect):
    x, y, w, h = rect
    return [x + w / 2, y + h / 2]


if __name__ == "__main__":
    main()
