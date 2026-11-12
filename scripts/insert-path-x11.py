#!/usr/bin/env python3
"""Insert path on private Rex/Xvfb/HOME; never uses the user's server or clipboard.

Requires built REX_BIN, Rex with terminal.list_dir, Xvfb, openbox, xdotool,
ImageMagick, Tesseract and Pillow. All artifacts and helper temp files are under
~/.cache/agent-ws/rex-m31-insert-path/. Observer calls are not client wire counts.
"""
import importlib.util
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
BASE = Path.home() / ".cache/agent-ws/rex-m31-insert-path"
BINARY = os.environ.get("REX_BIN", str(BASE / "target/release/rex-linux"))
BASE.mkdir(parents=True, exist_ok=True)
work = Path(tempfile.mkdtemp(prefix="native-", dir=BASE))
for tool in ("rex", "Xvfb", "openbox", "xdotool", "import", "magick", "tesseract"):
    assert shutil.which(tool), f"{tool} required"
for name in ("home", "config", "state", "runtime", "cache", "captures", "tmp"):
    (work / name).mkdir(mode=0o700)
for name in list(os.environ):
    if name.startswith(("REX_", "HERDR_")) or name in ("DISPLAY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS"):
        del os.environ[name]
endpoint = "unix://" + str(work / "server.sock")
os.environ.update(HOME=str(work / "home"), XDG_CONFIG_HOME=str(work / "config"),
                  XDG_STATE_HOME=str(work / "state"), XDG_RUNTIME_DIR=str(work / "runtime"),
                  XDG_CACHE_HOME=str(work / "cache"), TMPDIR=str(work / "tmp"),
                  REX_BIN=BINARY, REX_CAPTURE_OUT=str(work / "captures"), REX_SERVER=endpoint)
tempfile.tempdir = str(work / "tmp")
spec = importlib.util.spec_from_file_location("accept", ROOT / "scripts/rune-acceptance-x11.py")
accept = importlib.util.module_from_spec(spec)
spec.loader.exec_module(accept)


def wait(predicate, description, timeout=15):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        if predicate():
            return
        time.sleep(.08)
    raise AssertionError(description)


def capture(name):
    desk.capture(window, name)
    original = work / "captures" / f"{name}.png"
    ocr = work / "captures" / f"{name}-ocr.png"
    subprocess.run(["magick", str(original), "-resize", "300%", "-colorspace", "Gray", "-negate", str(ocr)], check=True)
    words = subprocess.check_output(["tesseract", str(ocr), "stdout"], text=True, stderr=subprocess.DEVNULL)
    (work / "captures" / f"{name}.txt").write_text(words)
    return words


def browser():
    desk.key("ctrl+shift+p")
    desk.type("Insert path")
    desk.key("Return")
    time.sleep(.5)


def field(text):
    desk.key("ctrl+a")
    desk.type(text)
    time.sleep(.7)


def terminal():
    return accept.terminal_text(sid, block)


def paste(text):
    accept.api("block.event", session_id=sid, block_id=block,
               event={"type": "paste", "payload": {"text": text}})


def enter():
    accept.api("block.event", session_id=sid, block_id=block,
               event={"type": "key", "payload": {"key": "enter", "action": "press", "mods": ""}})


server = desk = None
try:
    print("Artifacts:", work, flush=True)
    tree = work / "home" / "tree"
    tree.mkdir()
    for name in ("a space", "Alice's", ".hidden", "bad\nname"):
        (tree / name).touch()
    (tree / "folder").mkdir()
    (tree / "my dir").mkdir()
    (tree / "my file").touch()
    (tree / "folder" / "child").touch()
    server = subprocess.Popen(["rex", "server", "run", "--listen", endpoint, "--restore=false",
                               "--config", "/dev/null", "--state-dir", str(work / "rex-state")],
                              stdout=(work / "server.log").open("w"), stderr=subprocess.STDOUT)
    wait(lambda: (work / "server.sock").exists(), "private Rex did not start")
    sid, block = accept.session("Private insert path")
    paste("exec bash --noprofile --norc")
    enter()
    time.sleep(.4)
    counter = work / "prompts"
    line = work / "readline"
    statuses = work / "exit-statuses"
    # C-g records readline's exact buffer, but never accepts or evaluates that buffer.
    binding = "\"\\C-g\": printf %s \"$READLINE_LINE\" > " + shlex.quote(str(line))
    prompt_command = ('printf "%s\\n" "$?" >> ' + shlex.quote(str(statuses))
                      + '; printf x >> ' + shlex.quote(str(counter)))
    setup = (f"unset HISTFILE; cd {shlex.quote(str(tree))}; PS1='M31> '; "
             f"PROMPT_COMMAND={shlex.quote(prompt_command)}; "
             f"bind -x {shlex.quote(binding)}")
    paste(setup)
    enter()
    wait(counter.exists, "private bash prompt not ready")
    config = work / "workspace.json"
    config.write_text(json.dumps({"font_family": "IBM Plex Mono", "pane_padding": 8,
                                 "hosts": [{"label": "Private paths", "endpoint": endpoint}],
                                 "sessions": {endpoint: sid}}))
    desk = accept.Desktop()
    app, window = desk.launch(sid, config, size="1024x700", log=(work / "client.log").open("w"))
    before_size = accept.api("com.superlogical.terminal.size", session_id=sid, block_id=block)
    before_view = accept.api("session.view", session_id=sid)
    browser()
    words = capture("browser-1024")
    assert "a space" in words and "Alice" in words and "folder" in words, words
    assert "hidden" not in words, words
    # The newline-bearing name is never offered: no row can paste a raw control character.
    assert "bad" not in words, words
    field("~/tree/.")
    assert "hidden" in capture("hidden"), "dot prefix did not reveal dotfile"
    field("~/tree/fol")
    desk.key("Down", "Tab")
    time.sleep(.5)
    assert "child" in capture("descended"), "Tab did not descend"
    desk.key("Escape")
    assert accept.api("com.superlogical.terminal.size", session_id=sid, block_id=block) == before_size
    assert accept.api("session.view", session_id=sid) == before_view
    records = []
    for name in ("a space", "Alice's"):
        desk.type("ls -d ")
        prompts = counter.read_bytes()
        browser()
        field("~/tree/" + name)
        desk.key("Down", "Return")
        time.sleep(.5)
        assert counter.read_bytes() == prompts, "choosing a path executed the line"
        text = terminal()
        (work / f"terminal-{len(records)}-before.txt").write_text(text)
        # POSIX single quote spelling used by the client, not shlex's equivalent double quote splice.
        expected = "ls -d '" + str(tree / name).replace("'", "'\\''") + "'"
        desk.key("ctrl+g")
        wait(line.exists, "readline buffer not recorded")
        actual = line.read_text()
        line.unlink()
        assert actual == expected, (actual, expected)
        assert counter.read_bytes() == prompts, "path insertion accepted a prompt"
        capture(f"inserted-{len(records)}")
        desk.key("Return")  # Only the test script, never the feature, executes the line.
        wait(lambda: len(counter.read_bytes()) == len(prompts) + 1, "explicit Enter did not complete ls")
        text = terminal()
        (work / f"terminal-{len(records)}-after.txt").write_text(text)
        # GNU ls may quote displayed names; readline already established the exact input.
        assert name in text and "No such file" not in text, text
        assert statuses.read_text().splitlines()[-1] == "0", "ls did not resolve the inserted path"
        records.append({"name": name, "expected_buffer": expected, "actual_buffer": actual,
                        "prompts_before": len(prompts), "prompts_after_explicit_enter": len(counter.read_bytes()),
                        "ls_exit_status": statuses.read_text().splitlines()[-1]})
    # Go to directory: `cd <quoted folder>` is typed and nothing runs until the script's Enter.
    prompts = counter.read_bytes()
    desk.key("ctrl+shift+p")
    desk.type("Go to directory")
    desk.key("Return")
    time.sleep(.5)
    field("~/tree/my")
    words = capture("go-browser")
    assert "my dir" in words, "folder not listed"
    # Folders only: the file sharing the typed prefix is not offered.
    assert "my file" not in words, words
    desk.key("Down", "Return")
    time.sleep(.5)
    assert counter.read_bytes() == prompts, "choosing a folder executed a line"
    desk.key("ctrl+g")
    wait(line.exists, "readline buffer not recorded")
    go_buffer = line.read_text()
    line.unlink()
    go_expected = "cd '" + str(tree / "my dir") + "'"
    assert go_buffer == go_expected, (go_buffer, go_expected)
    assert counter.read_bytes() == prompts, "typing cd accepted a prompt"
    capture("go-typed")
    desk.key("Return")  # Only the test script presses Enter.
    wait(lambda: len(counter.read_bytes()) == len(prompts) + 1, "explicit Enter did not complete cd")
    assert statuses.read_text().splitlines()[-1] == "0", "cd did not resolve the typed folder"
    where = work / "pwd"
    paste("pwd > " + shlex.quote(str(where)))
    enter()
    wait(where.exists, "pwd not recorded")
    time.sleep(.2)
    assert where.read_text().strip() == str(tree / "my dir"), where.read_text()
    # Back to the tree, so the checks below list it again rather than the empty folder.
    prompts = counter.read_bytes()
    paste("cd " + shlex.quote(str(tree)))
    enter()
    wait(lambda: len(counter.read_bytes()) == len(prompts) + 1, "could not return to the tree")
    desk.x("windowsize", window, 640, 480)
    time.sleep(.5)
    browser()
    # `folder` appears only in the browser's rows, never in the terminal behind it.
    words = capture("browser-640")
    assert "a space" in words and "folder" in words, words
    field("~/tree/does-not-exist/")
    words = capture("missing-640")
    assert "doesn't exist" in words, words
    desk.key("Escape")
    (work / "results.json").write_text(json.dumps({"records": records,
        "go_to_directory": {"expected_buffer": go_expected, "actual_buffer": go_buffer,
                            "ran_only_after_script_enter": True, "pwd_after": where.read_text().strip()},
        "browse_preserved_size_owner_and_view": True,
        "real_list_hidden_descent_missing": True, "sizes": [[1024, 700], [640, 480]],
        "native_clipboard": "not used", "client_wire_requests": "not intercepted"}, indent=2))
    print("PASS real list, hidden filter, Tab descent, Escape, missing folder; size/owner/view unchanged by browsing", flush=True)
    print("PASS exact space/apostrophe readline buffers; no Enter until script; explicit Enter resolved both paths", flush=True)
    print("PASS Go to directory typed cd with the quoted folder, ran nothing, and the script's Enter changed directory", flush=True)
finally:
    if desk:
        desk.close()
        for process in [*desk.apps, desk.wm, desk.server]:
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
    if server and server.poll() is None:
        server.terminate()
        server.wait(timeout=5)
