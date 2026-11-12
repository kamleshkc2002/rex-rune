#!/usr/bin/env python3
"""Check the client's actual Wayland bindings on a compositor with multiple seats.

Briefly opens a native window with an isolated config and a fresh Rex fixture.
No input is injected. This verifies seat/resource selection, not physical input
or IME behavior. Requires wayland-info, rex and a built client.

Environment:
  WAYLAND_DISPLAY  the compositor to open the window on. The window appears there for a few
                   seconds, so point it at a disposable compositor (for example a headless
                   `labwc` with WLR_BACKENDS=headless) unless you want it on your desktop.
                   A headless compositor has no keyboard or pointer, so the client gets no input
                   resources to check: run tools/vptr in it first (see mouse-check.py).
  REX_SERVER       required: a disposable Rex server you started (unix:// socket). Autostart is
                   off, and the script exits rather than fall back to the default server.
  REX_BIN          the client to test (default target/debug/rex-rune).
The client runs with its own HOME and XDG directories, no update check, and a `tailscale` that
finds no tailnet.
"""
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = os.environ.get("REX_BIN", str(ROOT / "target/debug/rex-rune"))


def api(method, **payload):
    return json.loads(subprocess.check_output(
        ["rex", "--autostart=false", "--server", os.environ["REX_SERVER"],
         "api", "call", method, json.dumps(payload)], text=True, timeout=15))


def run():
    if not os.environ.get("REX_SERVER"):
        raise SystemExit("Set REX_SERVER explicitly to a disposable Rex server")
    assert os.environ.get("WAYLAND_DISPLAY"), "Requires a running Wayland desktop"
    inventory = subprocess.check_output(["wayland-info"], text=True, timeout=15)
    seats = [(int(number), name) for number, name in re.findall(
        r"interface: 'wl_seat',[^\n]*name:\s*(\d+)\n\s*name:\s*([^\n]+)", inventory)]
    assert seats, "No wl_seat advertised"
    if len(seats) < 2:
        print("NOTE only one seat advertised: this run checks binding, not the multi-seat regression")
    session = api("session.create", label=f"rex-wayland-seat-{os.getpid()}", initial_windows=[{
        "layout": {"block": {"flavor": "com.superlogical.terminal.shell"}}
    }])["session_id"]
    app = None
    try:
        with tempfile.TemporaryDirectory(prefix="rex-wayland-seat-") as directory:
            home = Path(directory) / "home"
            stub = Path(directory) / "bin"
            home.mkdir()
            stub.mkdir()
            (stub / "tailscale").write_text("#!/bin/sh\nexit 1\n")
            (stub / "tailscale").chmod(0o755)
            env = {k: v for k, v in os.environ.items()
                   if not k.startswith(("XDG_CONFIG", "XDG_STATE", "XDG_CACHE", "XDG_DATA"))}
            env.update(HOME=str(home), WAYLAND_DEBUG="client", REX_RUNE_NO_UPDATE_CHECK="1",
                       PATH=f"{stub}{os.pathsep}{env.get('PATH', '')}")
            app = subprocess.Popen([
                BINARY, "--session", session,
                "--config", str(Path(directory) / "workspace.json"),
            ], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
            try:
                _, trace = app.communicate(timeout=3)
                raise AssertionError(f"Native window exited early ({app.returncode})")
            except subprocess.TimeoutExpired:
                app.terminate()
                _, trace = app.communicate(timeout=10)

            # Inspect actual protocol requests, including the IDs used by input
            # and clipboard resources, instead of testing a selection helper.
            bindings = re.findall(
                r'-> wl_registry[#@]\d+\.bind\((\d+), "wl_seat", \d+, new id [^\n]*?[#@](\d+)\)', trace)
            bound_globals = [int(number) for number, _ in bindings]
            assert bound_globals == [seats[0][0]], (
                f"Expected only the first seat {seats[0]}, bound {bound_globals}; advertised {seats}")
            seat_id = bindings[0][1]
            for resource in ["keyboard", "pointer"]:
                owners = re.findall(rf'-> wl_seat[#@](\d+)\.get_{resource}\(', trace)
                assert owners == [seat_id], f"{resource} resources attached to {owners}, expected {seat_id}"
            clipboard_seats = re.findall(
                r'-> wl_data_device_manager[#@]\d+\.get_data_device\([^\n]*wl_seat[#@](\d+)\)', trace)
            # Rune's clipboard (arboard, wlr data-control) opens its own connection on first use,
            # so a window that has not copied yet binds no data device here.
            assert clipboard_seats in ([], [seat_id]), f"Clipboard is on a different seat: {clipboard_seats}"
            assert not re.search(rf'-> wl_seat[#@]{seat_id}\.release\(', trace), "Selected seat was replaced"
            assert Path(directory, "workspace.json").exists(), "Workspace never initialized"
            print(f"PASS native Wayland input/clipboard bind only to {seats[0][1]} "
                  f"(global {seats[0][0]}); {len(seats)-1} additional seats ignored")
    finally:
        if app and app.poll() is None:
            app.terminate()
            app.wait(timeout=10)
        api("session.destroy", session_id=session)


if __name__ == "__main__":
    run()
