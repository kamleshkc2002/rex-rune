#!/usr/bin/env python3
"""Verify OSC 133 through real Rex snapshots/live data and the client Ghostty model.

Requires rex, cargo and build dependencies. Always starts/stops its own server,
with private HOME, state and socket; never uses the configured personal server.
Artifacts remain in ~/.cache/agent-ws/rex-prompt-*. CARGO_TARGET_DIR is honoured.
"""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
REX = shutil.which("rex")
assert REX, "rex required"
base = Path.home() / ".cache/agent-ws"
base.mkdir(parents=True, exist_ok=True)
work = Path(tempfile.mkdtemp(prefix="rex-prompt-", dir=base))
(work / "private-prompt-fixture").touch()
env = {k: v for k, v in os.environ.items() if not k.startswith(("REX_", "HERDR_"))}
server_env = dict(env)
for variable, directory in (("HOME", "home"), ("XDG_CONFIG_HOME", "config"),
                            ("XDG_STATE_HOME", "state"), ("XDG_RUNTIME_DIR", "runtime"),
                            ("XDG_CACHE_HOME", "cache")):
    (work / directory).mkdir(mode=0o700)
    server_env[variable] = str(work / directory)
server_env.pop("DBUS_SESSION_BUS_ADDRESS", None)
endpoint = "unix://" + str(work / "rex.sock")
print("Private prompt artifacts:", work, flush=True)
with (work / "server.log").open("w") as log:
    server = subprocess.Popen([REX, "server", "run", "--listen", endpoint,
                               "--restore=false", "--config", "/dev/null",
                               "--state-dir", str(work / "server-state")],
                              env=server_env, stdout=log, stderr=subprocess.STDOUT)
    try:
        deadline = time.monotonic() + 10
        while not (work / "rex.sock").exists():
            assert server.poll() is None and time.monotonic() < deadline, "server did not start"
            time.sleep(.05)
        version = subprocess.check_output([REX, "version"], text=True, env=server_env)
        (work / "version.txt").write_text(version)
        print(version, flush=True)
        env["REX_PROMPT_WORK"] = str(work)
        result = subprocess.run(["cargo", "test", "--locked", "--no-default-features",
                                 "--test", "prompt_contract", "--", "--ignored", "--nocapture"],
                                cwd=ROOT, env=env, stdout=subprocess.PIPE,
                                stderr=subprocess.STDOUT, text=True)
        (work / "contract.log").write_text(result.stdout)
        print(result.stdout, end="", flush=True)
        result.check_returncode()
    finally:
        server.terminate()
        try:
            server.wait(timeout=10)
        except subprocess.TimeoutExpired:
            server.kill()
            server.wait(timeout=5)
