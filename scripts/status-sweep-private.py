#!/usr/bin/env python3
"""M3.2 sweep measurements/native acceptance on private Rex, Xvfb and D-Bus only.

Run --bench 12 / --bench 40, or --native. Requires Rex API 13 and built REX_BIN;
bench also requires REX_SWEEP_PROBE (examples/status_sweep_probe.rs). Native needs
Xvfb/openbox/xdotool/ImageMagick/Tesseract and dbus-python/GLib for the fake daemon.
All evidence stays under ~/.cache/agent-ws/rex-m32-sweep/. Stops children by PID.
The transparent Unix proxy observes JSON WebSocket bytes, not TLS/socket overhead.
"""
import argparse
import asyncio
import collections
import datetime
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import statistics
import subprocess
import sys
import tempfile
import threading
import time
from zoneinfo import ZoneInfo

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
BASE = Path.home() / ".cache/agent-ws/rex-m32-sweep"
BASE.mkdir(parents=True, exist_ok=True)
parser = argparse.ArgumentParser()
parser.add_argument("--bench", type=int)
parser.add_argument("--native", action="store_true")
parser.add_argument("--delay-ms", type=float, default=0,
                    help="bench only: hold every server-to-client frame this long, as a slow link would")
args = parser.parse_args()
assert bool(args.bench) != args.native
binary = os.environ.get("REX_BIN", str(BASE / "target/release/rex-linux"))
probe = os.environ.get("REX_SWEEP_PROBE", str(BASE / "target/release/examples/status_sweep_probe"))
work = Path(tempfile.mkdtemp(prefix=f"{'native' if args.native else args.bench}-", dir=BASE))
for key in list(os.environ):
    if key.startswith(("REX_", "HERDR_")) or key in ("DISPLAY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS"):
        del os.environ[key]
for name in ("home", "config", "state", "runtime", "cache", "tmp", "captures"):
    (work / name).mkdir(mode=0o700)
os.environ.update(HOME=str(work / "home"), XDG_CONFIG_HOME=str(work / "config"),
                  XDG_STATE_HOME=str(work / "state"), XDG_RUNTIME_DIR=str(work / "runtime"),
                  XDG_CACHE_HOME=str(work / "cache"), TMPDIR=str(work / "tmp"),
                  REX_SERVER="unix://" + str(work / "rex.sock"), REX_BIN=binary,
                  REX_CAPTURE_OUT=str(work / "captures"))
tempfile.tempdir = str(work / "tmp")
spec = importlib.util.spec_from_file_location("accept", ROOT / "scripts/rune-acceptance-x11.py")
accept = importlib.util.module_from_spec(spec)
spec.loader.exec_module(accept)
processes = []
desk = proxy = None


def spawn(argv, name):
    p = subprocess.Popen(argv, stdout=(work / f"{name}.log").open("w"), stderr=subprocess.STDOUT)
    processes.append(p)
    return p


def wait(predicate, description, timeout=20):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        if predicate():
            return
        time.sleep(.05)
    raise AssertionError(description)


class Sniffer:
    def __init__(self, callback):
        self.buf, self.upgraded, self.callback = b"", False, callback

    def feed(self, data):
        frames = []
        self.buf += data
        if not self.upgraded:
            end = self.buf.find(b"\r\n\r\n")
            if end < 0:
                return frames
            frames.append((self.buf[:end + 4], None))
            self.buf, self.upgraded = self.buf[end + 4:], True
        while len(self.buf) >= 2:
            b = self.buf
            op, masked, n, pos = b[0] & 15, b[1] & 128, b[1] & 127, 2
            if n in (126, 127):
                size = 2 if n == 126 else 8
                if len(b) < 2 + size:
                    return frames
                n, pos = int.from_bytes(b[2:2 + size], "big"), 2 + size
            key = b""
            if masked:
                if len(b) < pos + 4:
                    return frames
                key, pos = b[pos:pos + 4], pos + 4
            if len(b) < pos + n:
                return frames
            payload, self.buf = b[pos:pos + n], b[pos + n:]
            message = None
            if op == 1:
                text = bytes(c ^ key[i % 4] for i, c in enumerate(payload)) if masked else payload
                try:
                    message = json.loads(text)
                    self.callback(message, len(text))
                except (ValueError, UnicodeDecodeError):
                    pass
            frames.append((b[:pos + n], message))
        return frames


class Proxy:
    def __init__(self):
        self.path = work / "proxy.sock"
        self.endpoint = "unix://" + str(self.path)
        self.loop = asyncio.new_event_loop()
        self.ready = threading.Event()
        self.records, self.writers = [], set()
        self.sweep_pending = set()
        self.sweep_completed = 0
        self.cpu_start = self.cpu_end = None
        self.gate = threading.Event()
        if not args.native:
            self.gate.set()
        self.thread = threading.Thread(target=self.run, daemon=True)
        self.thread.start()
        assert self.ready.wait(10)

    def run(self):
        asyncio.set_event_loop(self.loop)
        self.server = self.loop.run_until_complete(asyncio.start_unix_server(self.handle, str(self.path)))
        self.ready.set()
        self.loop.run_forever()

    async def handle(self, reader, writer):
        upstream, upstream_writer = await asyncio.open_unix_connection(str(work / "rex.sock"))
        self.writers.update((writer, upstream_writer))
        connection = len(self.records)
        async def pump(source, dest, direction):
            held = []
            def observe(message, size):
                self.records.append(dict(time=time.monotonic(), connection=connection, direction=direction, bytes=size, message=message))
                key = (connection, message.get("id"))
                method = message.get("method", "")
                if args.bench and direction == "sent" and str(message.get("id")).startswith("detached:") and (method == "session.view" or method.endswith(".program_status")):
                    if self.cpu_start is None:
                        self.cpu_start = cpu(server.pid)
                    self.sweep_pending.add(key)
                if args.bench and direction == "received" and message.get("type") == "response" and key in self.sweep_pending:
                    self.sweep_pending.remove(key)
                    self.sweep_completed += 1
                    if self.sweep_completed == 9 * (args.bench - 1):
                        self.cpu_end = cpu(server.pid)
            sniffer = Sniffer(observe)
            async def release(frame):
                while not self.gate.is_set():
                    await asyncio.sleep(.01)
                dest.write(frame)
                await dest.drain()
            try:
                while data := await source.read(65536):
                    for frame, message in sniffer.feed(data):
                        # Hold ONLY the sweep request. Startup resize/ownership traffic
                        # must flow, so the native baseline isolates the sweep correctly.
                        gated = direction == "sent" and message and message.get("method") == "session.view" and str(message.get("id")).startswith("detached:") and not self.gate.is_set()
                        if gated:
                            held.append(asyncio.create_task(release(frame)))
                        elif args.delay_ms and direction == "received":
                            # Same delay for every frame, so their order is kept.
                            self.loop.call_later(args.delay_ms / 1000, dest.write, frame)
                        else:
                            dest.write(frame)
                    await dest.drain()
            except (ConnectionError, asyncio.CancelledError):
                pass
            finally:
                for task in held:
                    task.cancel()
                await asyncio.gather(*held, return_exceptions=True)
                dest.close()
        await asyncio.gather(pump(reader, upstream_writer, "sent"), pump(upstream, writer, "received"))
        self.writers.difference_update((writer, upstream_writer))

    def drop(self):
        async def drop_all():
            for writer in list(self.writers):
                writer.close()
        asyncio.run_coroutine_threadsafe(drop_all(), self.loop).result(5)

    def close(self):
        async def close_all():
            self.server.close()
            tasks = [t for t in asyncio.all_tasks() if t is not asyncio.current_task()]
            for task in tasks:
                task.cancel()
            await asyncio.gather(*tasks, return_exceptions=True)
            self.loop.stop()
        asyncio.run_coroutine_threadsafe(close_all(), self.loop)
        self.thread.join(5)


def snapshot(sessions):
    views = [accept.api("session.view", session_id=s) for s in sessions]
    sizes = {b["block_id"]: accept.api("com.superlogical.terminal.size", session_id=s, block_id=b["block_id"])
             for s, view in zip(sessions, views) for w in view["windows"] for layer in w["layers"] for b in layer["blocks"]}
    return dict(views=views, sizes=sizes)


def cpu(pid):
    stat = Path(f"/proc/{pid}/stat").read_text().split()
    return (int(stat[13]) + int(stat[14])) / os.sysconf("SC_CLK_TCK")


def summary(records):
    pending, sweeps, interactive = {}, [], []
    for r in records:
        m = r["message"]
        key = (r["connection"], m.get("id"))
        if m.get("type") == "request":
            pending[key] = r
        elif m.get("type") == "response" and key in pending:
            request = pending.pop(key)
            method = request["message"].get("method", "")
            data = dict(method=method, sent=request["bytes"], received=r["bytes"], start=request["time"], end=r["time"], ms=1000 * (r["time"] - request["time"]))
            if str(m.get("id")).startswith("detached:") and (method == "session.view" or method.endswith(".program_status")):
                sweeps.append(data)
            elif method == "session.view":
                interactive.append(data)
    finish = max(s["end"] for s in sweeps)
    def stats(samples):
        values = sorted(s["ms"] for s in samples)
        return dict(n=len(values), median_ms=statistics.median(values), p95_ms=values[int(.95 * (len(values)-1))], max_ms=max(values))
    return dict(requests=dict(collections.Counter(s["method"] for s in sweeps)),
                json_bytes_sent=sum(s["sent"] for s in sweeps), json_bytes_received=sum(s["received"] for s in sweeps),
                first_to_last_read_ms=1000 * (finish - min(s["start"] for s in sweeps)),
                interactive_during=stats(s for s in interactive if s["start"] < finish),
                interactive_at_rest=stats(s for s in interactive if s["start"] >= finish))


try:
    print("Artifacts:", work, flush=True)
    server = spawn(["rex", "server", "run", "--listen", os.environ["REX_SERVER"], "--restore=false", "--config", "/dev/null", "--state-dir", str(work / "rex-state")], "rex")
    wait(lambda: (work / "rex.sock").exists(), "private Rex did not start")
    shell = {"block": {"flavor": "com.superlogical.terminal.shell", "options": {"shell": "/bin/sh"}}}
    def layout(n):
        return shell if n == 1 else {"split": {"direction": "horizontal", "ratio": .5, "before": shell, "after": layout(n-1)}}
    sessions = []
    for i in range(args.bench or 3):
        names = [f"Task-{i}-{j}" for j in range(5)] if args.bench else [["Front"], ["Waiting"], ["Quiet"]][i]
        sid = accept.api("session.create", label=f"Session-{i}", initial_windows=[
            {"window_label": name, "layout": layout([1, 1, 1, 2, 3][j] if args.bench else 1)} for j, name in enumerate(names)]) ["session_id"]
        sessions.append(sid)
    proxy = Proxy()
    if args.bench:
        time.sleep(1)
        before = snapshot(sessions)
        (work / "before.json").write_text(json.dumps(before, indent=2))
        cpu_start = cpu(server.pid)
        started = time.monotonic()
        # The comparison read takes two round trips; keep its period well above that so the
        # probe itself does not saturate the host loop on a delayed link.
        os.environ["REX_SWEEP_PROBE_PERIOD_MS"] = str(int(max(100, 8 * args.delay_ms)))
        p = spawn([probe, proxy.endpoint, sessions[0]], "probe")
        assert p.wait(timeout=300) == 0
        cpu_end = cpu(server.pid)
        after = snapshot(sessions)
        (work / "after.json").write_text(json.dumps(after, indent=2))
        assert before == after, "read-only worker changed views/sizes/owners"
        result = summary(proxy.records)
        result.update(sessions=len(sessions), tabs=len(sessions)*5, panes=len(before["sizes"]),
                      injected_delay_ms=args.delay_ms,
                      probe=json.loads((work / "probe.log").read_text()),
                      server_cpu_seconds_over_sweep=proxy.cpu_end-proxy.cpu_start,
                      server_cpu_seconds_including_5s_rest=cpu_end-cpu_start,
                      probe_wall_seconds=time.monotonic()-started, unchanged_views_sizes_owners=True)
        (work / "result.json").write_text(json.dumps(result, indent=2))
        print(json.dumps(result, indent=2), flush=True)
    else:
        # The report is emitted once, BEFORE the client connects; the program then sleeps.
        view = accept.api("session.view", session_id=sessions[1])
        block = view["windows"][0]["focused_block_id"]
        reporter = work / "report.py"
        reporter.write_text("import time\nprint('\\x1b]7501;state=blocked:kind=permission:app=sweep-test\\x07',end='',flush=True)\ntime.sleep(300)\n")
        time.sleep(.5)
        accept.api("block.event", session_id=sessions[1], block_id=block, event={"type":"paste","payload":{"text":f"/usr/bin/python3 {reporter}"}})
        accept.api("block.event", session_id=sessions[1], block_id=block, event={"type":"key","payload":{"key":"enter","action":"press","mods":""}})
        wait(lambda: any(r.get("state") == "blocked" for r in accept.api("com.superlogical.terminal.program_status", session_id=sessions[1], block_id=block).get("records", [])), "pre-connect OSC status missing")
        bus_config = work / "bus.conf"
        bus_config.write_text(f'<busconfig><type>session</type><listen>unix:path={work}/bus</listen><auth>EXTERNAL</auth><policy context="default"><allow own="*"/><allow send_destination="*"/><allow receive_sender="*"/></policy></busconfig>')
        bus = subprocess.Popen(["dbus-daemon", "--nofork", "--config-file", str(bus_config), "--print-address=1"], stdout=subprocess.PIPE, text=True)
        processes.append(bus)
        os.environ["DBUS_SESSION_BUS_ADDRESS"] = bus.stdout.readline().strip()
        spawn(["/usr/bin/python3", str(ROOT / "scripts/notification-quality-x11.py"), "--service", str(work)], "notifications")
        wait(lambda: (work / "service-ready").exists(), "fake service unavailable")
        config = work / "workspace.json"
        config.write_text(json.dumps({"font_family":"IBM Plex Mono", "sidebar_open":True,
            "hosts":[{"label":"Private", "endpoint":proxy.endpoint}], "sessions":{proxy.endpoint:sessions[0]},
            "notifications":{"system_notifications":True}}))
        os.environ["REX_SERVER"] = proxy.endpoint
        desk = accept.Desktop()
        app, window = desk.launch(sessions[0], config, size="1024x700", log=(work / "client.log").open("w"))
        # API snapshots go directly to the private server, not the measured proxy.
        os.environ["REX_SERVER"] = "unix://" + str(work / "rex.sock")
        before = snapshot(sessions)
        (work / "before.json").write_text(json.dumps(before, indent=2))
        proxy.gate.set()
        time.sleep(2)
        def capture(name):
            desk.capture(window, name)
            words = subprocess.check_output(["tesseract", str(work / "captures" / f"{name}.png"), "stdout"], text=True, stderr=subprocess.DEVNULL)
            (work / "captures" / f"{name}.txt").write_text(words)
            return words
        def check_surfaces(name):
            words = capture(name)
            assert "needs you" in words.lower(), words
            assert "Needs approval" in words, words
            assert "Session-0" in desk.x("getwindowname", window), "selected session changed"
            desk.key("ctrl+shift+p")
            desk.type("Show activity")
            desk.key("Return")
            time.sleep(.5)
            words = capture(name + "-list")
            assert "Waiting" in words and "Needs approval" in words, words
            assert "Reading activity" not in words and "could not be read" not in words, words
            desk.key("Escape")
        check_surfaces("initial")
        initial = snapshot(sessions)
        (work / "initial.json").write_text(json.dumps(initial, indent=2))
        assert before == initial, "sweep changed views/sizes/owners"
        proxy.gate.clear()
        proxy.drop()
        time.sleep(5)
        reconnect_before = snapshot(sessions)
        (work / "reconnect-before.json").write_text(json.dumps(reconnect_before, indent=2))
        proxy.gate.set()
        time.sleep(1)
        check_surfaces("reconnected")
        after = snapshot(sessions)
        (work / "after.json").write_text(json.dumps(after, indent=2))
        assert reconnect_before == after, "reconnect sweep changed views/sizes/owners"
        # Ordinary selected-session restoration can advance view revision and the last
        # resize declaration's timestamp. The sweep must change neither on either pass.
        assert before["views"][1:] == after["views"][1:], "background views changed across reconnect"
        for block_id, size in before["sizes"].items():
            old, new = json.loads(json.dumps(size)), json.loads(json.dumps(after["sizes"][block_id]))
            for entry in old.get("desired", []) + new.get("desired", []):
                entry.pop("updated_at", None)
            assert old == new, "size or owner changed across reconnect"
        calls = [json.loads(line) for line in (work / "calls.jsonl").read_text().splitlines()] if (work / "calls.jsonl").exists() else []
        assert not any(c["method"].startswith("Notify") for c in calls), calls
        requests = [r["message"] for r in proxy.records if r["message"].get("type") == "request"]
        boots = [m for m in requests if m["method"] == "session.bootstrap"]
        assert len(boots) == 2
        assert all(m["payload"]["attach_session_ids"] == [sessions[0]] for m in boots)
        background = [m for m in requests if m.get("payload", {}).get("session_id") in sessions[1:]]
        assert all(m["method"] in ("session.view", "com.superlogical.terminal.program_status") for m in background), background
        assert sum(m["method"].endswith(".program_status") and m["payload"]["block_id"] == block for m in requests) == 2
        result = dict(passed=True, sessions=3, blocked_before_connect=True, surfaces=["chip", "Activity list", "sidebar"],
                      reconnected_without_new_report=True, no_background_attach=True, notifications=calls,
                      unchanged_views_sizes_owners=True, baseline="each connection: after selected-session restore, before releasing first sweep layout request",
                      reconnect_restore="selected view revision/resize timestamp may advance; actual sizes and owners unchanged",
                      not_exercised=["Wayland", "remote latency", "multiple hosts", "frame statistics"])
        (work / "result.json").write_text(json.dumps(result, indent=2))
        print(json.dumps(result, indent=2), flush=True)
    result.update(measured_at=datetime.datetime.now(ZoneInfo("America/Los_Angeles")).isoformat(),
                  executable_sha256=hashlib.sha256(Path(probe if args.bench else binary).read_bytes()).hexdigest(),
                  rex_version=subprocess.check_output(["rex", "version"], text=True).strip())
    (work / "result.json").write_text(json.dumps(result, indent=2))
finally:
    if desk:
        desk.close()
        for app in desk.apps:
            app.wait(timeout=5)
        desk.wm.wait(timeout=5)
        desk.server.wait(timeout=5)
    if proxy:
        proxy.close()
        (work / "wire.json").write_text(json.dumps(proxy.records, indent=2))
    for process in reversed(processes):
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
