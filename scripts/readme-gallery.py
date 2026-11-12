#!/usr/bin/env python3
"""Capture the README gallery from a built client, with real fonts on private X11.

Uses the same private-server fixture approach as readme-screenshot.py/docs-captures.py
and the Xvfb driver from rune-acceptance-x11.py. Needs rex, Xvfb, openbox, xdotool,
ImageMagick import, bat, eza and Pillow. REX_BIN selects the release build.
SHOT_OUT_DIR defaults to docs; outputs screenshot.png and images/settings-*.png.
SHOT_KEEP=1 retains the private work directory for troubleshooting.

The fixture is a fictional project, not this checkout or real test results. All
processes receive a minimal environment, temporary HOME/XDG directories, and an
explicit private Rex endpoint. Tailscale discovery and automatic update checks
are disabled. No existing server, desktop, clipboard or personal config is used.
"""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
APP = Path(os.environ.get("REX_BIN", ROOT / "target/release/rex-rune")).resolve()
OUT = Path(os.environ.get("SHOT_OUT_DIR", ROOT / "docs")).resolve()
SIZE = "1280x800"
SHELL = {"block": {"flavor": "com.superlogical.terminal.shell"}}


def main():
    for tool in ("rex", "Xvfb", "openbox", "xdotool", "import", "bat", "eza"):
        assert shutil.which(tool), f"{tool} required"
    assert APP.is_file(), f"Build rex-rune first: {APP}"
    keep = os.environ.get("SHOT_KEEP") == "1"
    work = Path(tempfile.mkdtemp(prefix="rex-gallery-"))
    for name in ("home", "config", "state", "runtime", "cache", "data", "captures"):
        (work / name).mkdir(mode=0o700)
    endpoint = f"unix://{work / 'rex.sock'}"
    # Do not inherit shell startup hooks, Rex options, bus addresses or credentials.
    path = os.environ.get("PATH", "/usr/local/bin:/usr/bin:/bin")
    os.environ.clear()
    os.environ.update(PATH=path, LANG="C.UTF-8", HOME=str(work / "home"),
                      USER="demo", LOGNAME="demo", SHELL="/bin/bash",
                      XDG_CONFIG_HOME=str(work / "config"), XDG_STATE_HOME=str(work / "state"),
                      XDG_RUNTIME_DIR=str(work / "runtime"), XDG_CACHE_HOME=str(work / "cache"),
                      XDG_DATA_HOME=str(work / "data"), REX_RUNE_NO_UPDATE_CHECK="1",
                      REX_SERVER=endpoint, REX_BIN=str(APP), REX_CAPTURE_OUT=str(work / "captures"))
    spec = importlib.util.spec_from_file_location("accept", ROOT / "scripts/rune-acceptance-x11.py")
    accept = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(accept)

    project = work / "home/orbit"
    for name in ("src/api", "src/store", "tests", "scripts", "docs"):
        (project / name).mkdir(parents=True, exist_ok=True)
    files = {
        "Cargo.toml": '[package]\nname = "orbit"\nversion = "0.1.0"\nedition = "2024"\n',
        "README.md": "# Orbit\nA small task service. Documentation fixture.\n",
        "src/main.rs": "fn main() { println!(\"Orbit is ready\"); }\n",
        "src/api/mod.rs": "pub mod routes;\n",
        "src/api/routes.rs": "// A small, typed API.\n\nuse axum::{Json, Router, routing::get};\n\npub fn routes() -> Router {\n    Router::new()\n        .route(\"/health\", get(health))\n        .route(\"/tasks\", get(list_tasks))\n}\n\nasync fn health() -> &'static str {\n    \"ready\"\n}\n\nasync fn list_tasks() -> Json<Vec<Task>> {\n    Json(store::recent_tasks().await)\n}\n",
        "src/store/mod.rs": "pub mod tasks;\n",
        "src/store/tasks.rs": "// Local task storage.\n",
        "tests/api.rs": "// API fixture cases.\n",
        "tests/store.rs": "// Storage fixture cases.\n",
        "docs/api.md": "# API\nGET /health\nGET /tasks\n",
        # These are explicitly demo output, not a claim about tests run for this repo.
        "scripts/check": "#!/bin/sh\nprintf '\\033[36mOrbit · fixture checks\\033[0m\\n\\n'\nprintf '\\033[32m  PASS\\033[0m  %s\\n' 'health endpoint' 'create a task' 'list recent tasks' 'persist across restart' 'reject invalid input'\nprintf '\\n5 fixture checks passed\\n'\n",
    }
    for name, content in files.items():
        (project / name).write_text(content)
    (project / "scripts/check").chmod(0o755)
    prompt = r"\[\e[38;5;110m\]orbit\[\e[0m\] \[\e[38;5;245m\]❯\[\e[0m\] "
    for name in (".bashrc", ".bash_profile"):
        (work / "home" / name).write_text(
            f"PS1='{prompt}'\nexport PAGER=cat GIT_PAGER=cat\ncd ~/orbit\nclear\n")

    server = desk = None
    log = (work / "processes.log").open("w")
    try:
        server = subprocess.Popen(["rex", "server", "run", "--listen", endpoint,
                                   "--restore=false", "--config", "/dev/null",
                                   "--state-dir", str(work / "server")],
                                  cwd=project, stdout=log, stderr=log, stdin=subprocess.DEVNULL)
        deadline = time.monotonic() + 10
        while not (work / "rex.sock").exists():
            assert server.poll() is None and time.monotonic() < deadline, "private server did not start"
            time.sleep(.05)
        api = accept.api
        layout = {"split": {"direction": "horizontal", "ratio": .42, "before": SHELL,
                            "after": {"split": {"direction": "vertical", "ratio": .58,
                                                "before": SHELL, "after": SHELL}}}}
        sid = api("session.create", label="orbit", initial_windows=[
            {"window_label": "dev", "layout": layout},
            {"window_label": "server", "layout": SHELL},
            {"window_label": "notes", "layout": SHELL}])["session_id"]
        for name in ("api-gateway", "dotfiles", "docs"):
            api("session.create", label=name, initial_windows=[{"window_label": "main", "layout": SHELL}])
        dev = api("session.view", session_id=sid)["windows"][0]
        blocks = [b["block_id"] for b in api("session.list_blocks", session_id=sid)["blocks"]
                  if b.get("window_id") == dev["window_id"]]
        assert len(blocks) == 3
        for block, label in zip(blocks, ("project", "routes.rs", "checks")):
            api("session.set_block_label", session_id=sid, block_id=block, label=label)
        api("session.focus_window", session_id=sid, window_id=dev["window_id"])
        api("session.focus_block", session_id=sid, block_id=blocks[0])
        config = work / "workspace.json"
        config.write_text(json.dumps({"hosts": [{"label": "workstation", "endpoint": endpoint}],
                                      "last_host": endpoint, "sessions": {endpoint: sid},
                                      "client_label": "Documentation", "sidebar_open": True,
                                      "font_fallbacks": ["Noto Sans Mono", "Noto Sans CJK JP"]}))
        desk = accept.Desktop(size=SIZE)
        app, window = desk.launch(sid, config, size=SIZE, log=log)
        for block, command in zip(blocks, ("eza --tree --color=always", "bat -pp --color=always src/api/routes.rs", "./scripts/check")):
            subprocess.run(["rex", "--autostart=false", "-S", endpoint, "send", "-b", block, command + "\r"],
                           stdout=log, stderr=log, check=True, timeout=15)
        time.sleep(3)

        def click(x, y):
            desk.x("mousemove", "--window", window, x, y)
            desk.x("click", 1)
            time.sleep(.3)

        def capture(name):
            # Keep the pointer off controls so no tooltip or hover competes with the UI.
            desk.x("mousemove", "--window", window, 1260, 780)
            time.sleep(.6)
            png = work / "captures" / (name + ".png")
            subprocess.run(["import", "-window", window, str(png)], env=desk.env, check=True, timeout=15)
            dest = OUT / ("screenshot.png" if name == "workspace" else f"images/{name}.png")
            dest.parent.mkdir(parents=True, exist_ok=True)
            # Lossless RGB compression, dropping timestamps and capture-machine metadata.
            with Image.open(png) as image:
                if name != "workspace":
                    # The 816×680 Settings surface, with 12 px of surrounding context.
                    image = image.crop((220, 74, 1060, 778))
                image.convert("RGB").save(dest, optimize=True, compress_level=9)
            print(dest, flush=True)

        def search(text):
            click(330, 110)
            desk.key("ctrl+a")
            desk.type(text)
            time.sleep(.6)

        capture("workspace")
        desk.key("ctrl+comma")
        search("font")
        capture("settings-search")
        # The first font result opens the advanced page. Escape clears the search,
        # leaving the normal section list and Back/Forward controls visible.
        desk.key("Return", "Escape")
        time.sleep(3)
        capture("settings-fonts")
        search("Show tab icons")
        desk.key("Return", "Escape")
        time.sleep(3)
        # Demonstrate the optional icon count/colour settings in their real preview.
        click(1004, 455)
        click(1004, 455)
        click(1000, 504)
        desk.key("Up", "Up", "Up", "Return")
        time.sleep(.6)
        saved = json.loads(config.read_text())["interface"]
        assert saved["tab_icon_count"] == 3 and saved["tab_icon_color"] == "always", saved
        capture("settings-tabs")
        assert app.poll() is None, "client exited during captures"
    finally:
        if desk:
            desk.close()
            for process in [*desk.apps, desk.wm, desk.server]:
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
        if server:
            server.terminate()
            server.wait(timeout=5)
        log.close()
        if keep:
            print("Private artifacts:", work)
        else:
            shutil.rmtree(work)


if __name__ == "__main__":
    main()
