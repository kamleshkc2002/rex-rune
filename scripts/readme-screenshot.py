#!/usr/bin/env python3
"""Captures the README screenshot: the release binary on a private headless labwc, attached to a
private Rex server whose fixture sessions run in a throwaway home with a plain bash prompt, so no
user, host or path from the machine running it appears in the image.

Needs a `rex` server binary, labwc, wlr-randr, grim, xkbcli, bat, eza, cargo, a release build
(`cargo build --release`, or REX_BIN names another build) and the `vptr` helper (`cargo build
--release` in tools/vptr, or SHOT_VPTR). Writes docs/screenshot.png (or SHOT_OUT).

Everything runs on a private socket (every `rex` call names it, autostart off). The client sees a
`tailscale` that finds no tailnet and does no update check, so nothing of the machine's tailnet can
appear in the image.
"""
import json
import os
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = Path(os.environ.get("SHOT_OUT", ROOT / "docs/screenshot.png"))
APP = Path(os.environ.get("REX_BIN") or ROOT / "target/release/rex-rune")
W, H, SCALE = 2160, 1350, 1.5
# `cargo test` in the fixture shell prints only the suites that ran tests, one line per result.
CARGO = r"""cargo() { command cargo "$@" 2>/dev/null | grep -E '^(running [1-9]|\.+$|test result: ok\. [1-9])' | sed -E 's/; 0 ignored.*finished in/ in/'; }"""
PS1 = r"\[\e[38;5;110m\]rex-rune\[\e[0m\] \[\e[38;5;245m\]❯\[\e[0m\] "


def main():
    work = Path(tempfile.mkdtemp(prefix="rex-shot-"))
    socket = work / "rex.sock"
    endpoint = f"unix://{socket}"
    home = work / "home"
    home.mkdir()
    for rc in (".bashrc", ".bash_profile"):
        (home / rc).write_text(f"PS1='{PS1}'\nexport PAGER=cat GIT_PAGER=cat\n{CARGO}\ncd {ROOT}\nclear\n")
    processes = []
    log = open(work / "processes.log", "w")

    def spawn(args, **kw):
        p = subprocess.Popen(args, stdout=log, stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL, **kw)
        processes.append(p)
        return p

    def api(method, **payload):
        return json.loads(subprocess.check_output(["rex", "--autostart=false", "-S", endpoint, "api", "call", method, json.dumps(payload)], text=True))

    def send(block, text):
        subprocess.check_call(["rex", "--autostart=false", "-S", endpoint, "send", "-b", block, text + "\r"], stdout=subprocess.DEVNULL)

    base = {k: v for k, v in os.environ.items() if k not in ("WAYLAND_DISPLAY", "DISPLAY", "WAYLAND_SOCKET")}
    real_home = Path.home()
    shell_env = dict(base, HOME=str(home), SHELL="/bin/bash", USER="user", LOGNAME="user",
                     CARGO_HOME=os.environ.get("CARGO_HOME", str(real_home / ".cargo")),
                     RUSTUP_HOME=os.environ.get("RUSTUP_HOME", str(real_home / ".rustup")))
    try:
        spawn(["rex", "server", "run", "--listen", endpoint, "--restore=false", "--state-dir", str(work / "state")],
              env=shell_env, cwd=home)
        deadline = time.monotonic() + 10
        while not socket.exists():
            assert time.monotonic() < deadline, "private Rex server did not start"
            time.sleep(0.05)
        time.sleep(0.3)

        shell = {"block": {"flavor": "com.superlogical.terminal.shell"}}

        def session(label, windows):
            sid = api("session.create", label=label, initial_windows=windows)["session_id"]
            return sid, api("session.list_blocks", session_id=sid)

        main_layout = {"split": {"direction": "horizontal", "ratio": 0.5, "before": shell,
                                 "after": {"split": {"direction": "vertical", "ratio": 0.58,
                                                     "before": shell, "after": shell}}}}
        main_sid, blocks = session("rex-rune", [{"window_label": "dev", "layout": main_layout},
                                                 {"window_label": "server", "layout": shell},
                                                 {"window_label": "notes", "layout": shell}])
        for label, tab in (("api-gateway", "logs"), ("dotfiles", "main"), ("blog", "drafts")):
            session(label, [{"window_label": tab, "layout": shell}])
        view = api("session.view", session_id=main_sid)
        dev = view["windows"][0]
        ids = [b["block_id"] for b in blocks["blocks"] if b.get("window_id") == dev["window_id"]]
        assert len(ids) == 3, blocks
        left, top_right, bottom_right = ids
        for block, label in ((left, "src"), (top_right, "theme.rs"), (bottom_right, "tests")):
            api("session.set_block_label", session_id=main_sid, block_id=block, label=label)
        api("session.focus_window", session_id=main_sid, window_id=dev["window_id"])
        api("session.focus_block", session_id=main_sid, block_id=left)

        wl = work / "labwc"
        wl.mkdir()
        (wl / "rc.xml").write_text('<?xml version="1.0"?><labwc_config><core><decoration>client</decoration></core></labwc_config>')
        marker = work / "wayland-display"
        spawn(["labwc", "-C", str(wl), "-s", f"sh -c 'echo $WAYLAND_DISPLAY > {marker}'"],
              env=dict(base, WLR_BACKENDS="headless", WLR_RENDERER="vulkan", WLR_HEADLESS_OUTPUTS="1"))
        deadline = time.monotonic() + 10
        while not marker.exists() or not marker.read_text().strip():
            assert time.monotonic() < deadline, "headless labwc did not start"
            time.sleep(0.05)
        env = dict(base, WAYLAND_DISPLAY=marker.read_text().strip())
        subprocess.check_call(["wlr-randr", "--output", "HEADLESS-1", "--custom-mode", f"{W}x{H}@60", "--scale", str(SCALE)], env=env)
        config = work / "workspace.json"
        config.write_text(json.dumps({
            "hosts": [{"label": "workstation", "endpoint": endpoint}],
            "last_host": endpoint, "sessions": {endpoint: main_sid}, "font_size": 14}))
        stub = work / "bin"
        stub.mkdir()
        (stub / "tailscale").write_text("#!/bin/sh\nexit 1\n")
        (stub / "tailscale").chmod(0o755)
        spawn([str(APP), "--endpoint", endpoint, "--session", main_sid,
               "--config", str(config), "--window-size", f"{int(W / SCALE)}x{int(H / SCALE)}"],
              env=dict(env, PATH=f"{stub}{os.pathsep}{env['PATH']}", REX_RUNE_NO_UPDATE_CHECK="1"))
        # Run the fixture commands only once the client has attached and sized the panes.
        time.sleep(4)
        send(left, "eza --tree --color=always src")
        send(top_right, "bat -pp --color=always -r 54:73 src/app/theme.rs")
        send(bottom_right, "cargo test -q")
        # The sidebar starts collapsed: open it with Ctrl Shift B through the virtual keyboard.
        keymap = work / "us.xkb"
        keymap.write_text(subprocess.check_output(["xkbcli", "compile-keymap", "--layout", "us"], text=True))
        driver = subprocess.Popen([os.environ.get("SHOT_VPTR", str(ROOT / "tools/vptr/target/release/vptr")),
                                   str(W), str(H), str(keymap)], env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        processes.append(driver)
        assert driver.stdout.readline().strip() == "ready"
        for line in ("kdown 29", "kdown 42", "mods 5", "kdown 48", "kup 48",
                     "kup 42", "kup 29", "mods 0"):
            driver.stdin.write(line + "\n")
            driver.stdin.flush()
            time.sleep(0.05)
        time.sleep(float(os.environ.get("SHOT_WAIT", "20")))
        OUT.parent.mkdir(parents=True, exist_ok=True)
        subprocess.check_call(["grim", "-o", "HEADLESS-1", str(OUT)], env=env)
        print(OUT)
    finally:
        for p in reversed(processes):
            if p.poll() is None:
                p.terminate()
                try:
                    p.wait(5)
                except subprocess.TimeoutExpired:
                    p.kill()
        if os.environ.get("SHOT_KEEP") != "1":
            shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    main()
