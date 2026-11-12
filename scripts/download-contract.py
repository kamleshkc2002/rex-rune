#!/usr/bin/env python3
"""Observe the Rex file-download contract (API 13) against a private server.

Pins down, by observation, what `com.superlogical.terminal.download_begin` answers and how the
`rex.data.bytes.v1` stream that carries the file is opened, framed and ended, so a client can be
written to the facts rather than to the help text (which does not describe the stream's end).

Requires rex (API 13; set --rex to choose the binary) and Python 3.11+. It needs no third-party
packages: it speaks WebSocket itself so it can see every frame, opcode and close code.

Safe by construction: it starts its OWN server under the output directory, with its own socket,
HOME and XDG directories, an empty config and --restore=false, scrubs every REX_* variable so it
cannot reach another server, and stops that server by PID when it finishes. It never connects to
any other socket. Fixtures (including a 256 MiB sparse file) and raw evidence stay under
--out (default ~/.cache/agent-ws/rex-downloads/d0): `transcript.jsonl` has every control request
and response and each data stream's summary, `results.json` has each scenario's facts, and
`checks.json` the pass/fail of each pinned expectation. Pass --quick to skip the scenarios that
wait for minutes (ticket expiry, a stalled reader). Pass --rex12 PATH (an API 12 rex) to also
check what a revision 10-12 server answers; without one, that check uses only revision gating on
the API 13 server. Exit status is 1 if an expectation no longer holds, which means the contract
moved and the client's download code (and docs/server-api13.md) must be rechecked.

Not the client: nothing here exercises Rex Rune itself.
"""
import argparse
import base64
import collections
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import struct
import subprocess
import sys
import threading
import time
import traceback
import uuid

TERMINAL = "com.superlogical.terminal"
BYTES = "rex.data.bytes.v1"
MIB = 1 << 20

OUT = None
FX = None  # fixtures directory, the shell's working directory is its parent's "home"
SOCK = None
SERVER_PID = None
TRANSCRIPT = None
_log_lock = threading.Lock()
RESULTS = {}
CHECKS = []


def stamp():
    return time.strftime("%Y-%m-%dT%H:%M:%S%z")


def log(kind, **fields):
    line = json.dumps({"t": round(time.time(), 3), "kind": kind, **fields}, default=repr)
    with _log_lock:
        TRANSCRIPT.write(line + "\n")
        TRANSCRIPT.flush()


def check(name, ok, detail=""):
    CHECKS.append({"name": name, "ok": bool(ok), "detail": str(detail)})
    print(("  ok   " if ok else "  FAIL ") + name + (f"  [{detail}]" if detail and not ok else ""), flush=True)
    return ok


# ---------------------------------------------------------------- WebSocket (RFC 6455) client

class WS:
    """A small, observable WebSocket client over a Unix socket.

    Server frames are exposed one at a time with their FIN bit and opcode; pings are answered
    and counted. Nothing here compresses or defragments, so what the server really sends is seen.
    """

    def __init__(self, path, protocol, timeout=30):
        self.s = socket.socket(socket.AF_UNIX)
        self.s.settimeout(timeout)
        self.s.connect(SOCK)
        key = base64.b64encode(os.urandom(16)).decode()
        self.s.sendall((f"GET {path} HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\n"
                        f"Connection: Upgrade\r\nSec-WebSocket-Key: {key}\r\n"
                        f"Sec-WebSocket-Version: 13\r\nSec-WebSocket-Protocol: {protocol}\r\n\r\n").encode())
        raw = b""
        while b"\r\n\r\n" not in raw:
            chunk = self.s.recv(4096)
            if not chunk:
                raise EOFError("closed during upgrade: %r" % raw)
            raw += chunk
        head, rest = raw.split(b"\r\n\r\n", 1)
        self.buf = bytearray(rest)
        self.pos = 0
        self.head = head.decode()
        self.status = self.head.split("\r\n")[0]
        self.headers = {k.lower(): v for k, v in (line.split(": ", 1) for line in self.head.split("\r\n")[1:])}
        self.pings = []
        self.t0 = time.monotonic()

    def _need(self, n):
        while len(self.buf) - self.pos < n:
            if self.pos > MIB:
                del self.buf[:self.pos]
                self.pos = 0
            chunk = self.s.recv(1 << 20)
            if not chunk:
                raise EOFError("eof")
            self.buf += chunk

    def _take(self, n):
        self._need(n)
        out = bytes(self.buf[self.pos:self.pos + n])
        self.pos += n
        return out

    def send(self, opcode, data=b"", fin=True):
        mask = os.urandom(4)
        n = len(data)
        head = bytes([(0x80 if fin else 0) | opcode])
        if n < 126:
            head += bytes([0x80 | n])
        elif n < 65536:
            head += bytes([0x80 | 126]) + struct.pack(">H", n)
        else:
            head += bytes([0x80 | 127]) + struct.pack(">Q", n)
        masked = bytes(b ^ mask[i % 4] for i, b in enumerate(data)) if n < 4096 else \
            (int.from_bytes(data, "big") ^ int.from_bytes((mask * (n // 4 + 1))[:n], "big")).to_bytes(n, "big")
        self.s.sendall(head + mask + masked)

    def frame(self):
        """Return (fin, opcode, payload); answers pings itself and still returns them."""
        b0, b1 = self._take(2)
        fin, rsv, opcode = b0 >> 7, (b0 >> 4) & 7, b0 & 15
        length = b1 & 127
        if length == 126:
            length = struct.unpack(">H", self._take(2))[0]
        elif length == 127:
            length = struct.unpack(">Q", self._take(8))[0]
        masked = b1 >> 7
        assert not masked, "server frames must not be masked"
        data = self._take(length)
        if rsv:
            raise AssertionError("RSV bits set: compression or extension in use")
        if opcode == 9:
            self.pings.append(round(time.monotonic() - self.t0, 3))
            self.send(10, data)
        return fin, opcode, data

    def close(self, code=None, reason=""):
        try:
            if code is not None:
                self.send(8, struct.pack(">H", code) + reason.encode())
        except OSError:
            pass
        try:
            self.s.shutdown(socket.SHUT_RDWR)
        except OSError:
            pass
        self.s.close()

    def abort(self):
        """Drop the TCP connection with no close frame (RST)."""
        self.s.setsockopt(socket.SOL_SOCKET, socket.SO_LINGER, struct.pack("ii", 1, 0))
        self.s.close()


class Control:
    """One control connection (rex.control.v1), with a transcript of every request."""

    def __init__(self, rev=13, minimum=10, name="download-contract", log_requests=True):
        self.ws = WS("/api/control", "rex.control.v1")
        self.log_requests = log_requests
        self.n = 0
        self.events = []
        hello = {"type": "hello", "api_revision": rev, "api_min_revision": minimum,
                 "client": {"kind": "cli", "name": name, "version": "0"}}
        self.ws.send(1, json.dumps(hello).encode())
        self.welcome = self._text()
        assert self.welcome["type"] == "welcome", self.welcome
        self.revision = self.welcome.get("api_revision")
        self.session = self.block = None

    def _text(self, timeout=None):
        while True:
            fin, op, data = self.ws.frame()
            if op == 1:
                return json.loads(data)
            if op == 8:
                raise EOFError("control closed %r" % (data,))

    def call(self, method, payload=None, timeout=20):
        self.n += 1
        ident = str(self.n)
        message = {"type": "request", "id": ident, "method": method}
        if payload is not None:
            message["payload"] = payload
        self.ws.s.settimeout(timeout)
        self.ws.send(1, json.dumps(message).encode())
        started = time.monotonic()
        while True:
            reply = self._text()
            if reply.get("type") == "response" and reply.get("id") == ident:
                break
            self.events.append(reply)
        if self.log_requests:
            log("control", method=method, request=payload, response=reply,
                ms=round((time.monotonic() - started) * 1000, 1), revision=self.revision)
        return reply

    def block_call(self, method, args=None, session=None, block=None, **kw):
        return self.call(TERMINAL + "." + method, {"session_id": session or self.session,
                                                    "block_id": block or self.block, "args": args or {}}, **kw)

    def new_session(self, label="d0"):
        reply = self.call("session.create", {"label": label, "initial_windows": [
            {"layout": {"block": {"flavor": TERMINAL + ".shell", "options": {"shell": "/bin/sh"}}}}]})
        self.session = reply["payload"]["session_id"]
        self.block = reply["payload"]["initial_windows"][0]["block_ids"][0]
        attached = self.call("session.attach", {"session_id": self.session})
        assert "error" not in attached, attached
        return self.session, self.block

    def begin(self, path, ref=None, args=None, release=False, **kw):
        """download_begin. Returns the whole response message (payload or error).

        The server reserves a slot (at most 16 per terminal) until the ticket's stream finishes or
        is cancelled, so a probe that only wants the reply passes release=True: its ticket is
        opened and closed with 1000 at once, which frees the slot."""
        args = dict(args) if args is not None else {}
        args.setdefault("ref", new_ref() if ref is None else ref)
        args.setdefault("path", path)
        reply = self.block_call("download_begin", args, **kw)
        if release and reply.get("payload", {}).get("ticket"):
            release_ticket(reply["payload"]["ticket"])
        return reply

    def probe(self, path, **kw):
        return self.begin(path, release=True, **kw)

    def close(self):
        self.ws.close(1000)


def new_ref():
    return "download:" + str(uuid.uuid4())


def hello_message(ticket):
    return b"\x01" + base64.urlsafe_b64decode(ticket + "=" * (-len(ticket) % 4))


# ---------------------------------------------------------------- the data stream

def release_ticket(ticket):
    """Open the stream and cancel it with close 1000 (waits for the server's close)."""
    ws = WS("/api/data", BYTES)
    ws.s.settimeout(10)
    ws.send(2, hello_message(ticket))
    ws.frame()
    ws.send(8, struct.pack(">H", 1000))
    try:
        while ws.frame()[1] != 8:
            pass
    except (EOFError, OSError):
        pass
    ws.close()


def parse_welcome(raw):
    if len(raw) == 32 and raw[0] == 0x81:
        return {"kind": "WELCOME", "token": raw[1:17].hex(), "zero": raw[17:20].hex(),
                "retention_ms": struct.unpack("<I", raw[20:24])[0],
                "nonce": struct.unpack("<Q", raw[24:32])[0], "len": 32}
    if raw and raw[0] == 0x82:
        return {"kind": "REJECT", "reason": raw[1] if len(raw) > 1 else None, "len": len(raw)}
    return {"kind": "UNKNOWN", "hex": raw[:64].hex(), "len": len(raw)}


def stream(ticket, protocol=BYTES, hello=None, hook=None, keep=0, deadline=120, cancel=None,
           pace=None, drain_after_close=0.5, first_frame_timeout=None, ws=None, record_frames=0):
    """Open a data connection, run the HELLO handshake and read the stream to its end.

    hello    override the first message (bytes); default is HELLO with the ticket.
    record_frames  keep the first N frames (WELCOME included) as fin, opcode, length and up to 48 bytes of hex
             under "raw_frames".
    hook     called after each data frame as hook(frame_count, byte_count, ws); return "stop" to
             stop reading (the connection is then closed by cancel, if given, else left open).
    keep     keep up to this many received bytes in the result under "data".
    cancel   how to stop when hook returns "stop": "close1000", "close1001", "abort" (RST) or
             "shutdown" (FIN, no close frame).
    Returns a dict of observations; nothing is asserted here.
    """
    info = {"protocol": protocol}
    started = time.monotonic()
    try:
        ws = ws or WS("/api/data", protocol)
    except Exception as error:
        return {"upgrade_error": repr(error)}
    info["upgrade"] = {"status": ws.status, "subprotocol": ws.headers.get("sec-websocket-protocol")}
    ws.s.settimeout(deadline)
    ws.send(2, hello if hello is not None else hello_message(ticket))
    info["handshake_sent"] = (hello if hello is not None else hello_message(ticket)).hex()
    sizes = collections.Counter()
    first, last = [], collections.deque(maxlen=4)
    digest = hashlib.sha256()
    total = nframes = nempty = nfrag = 0
    other = []
    kept = bytearray()
    data_started = None
    seq = []  # compact ordered log of everything after the handshake message
    frames_log = []
    stopped = False
    try:
        fin, op, raw = ws.frame()
        info["first_message"] = {"opcode": op, "fin": fin, **(parse_welcome(raw) if op == 2 else {"hex": raw.hex()})}
        if op != 2 or info["first_message"]["kind"] != "WELCOME":
            # a REJECT: read what follows (the close).
            info["after_reject"] = []
            try:
                while True:
                    fin, op, raw = ws.frame()
                    if op == 8:
                        code = struct.unpack(">H", raw[:2])[0] if len(raw) >= 2 else None
                        info["end"] = {"how": "close", "code": code, "reason": raw[2:].decode(errors="replace")}
                        break
                    info["after_reject"].append({"opcode": op, "len": len(raw), "hex": raw[:32].hex()})
            except (EOFError, ConnectionError, OSError, TimeoutError) as error:
                info["end"] = {"how": "eof", "error": repr(error)}
            info["elapsed_s"] = round(time.monotonic() - started, 3)
            ws.close()
            return info
        info["welcome"] = info["first_message"]
        if record_frames:
            info["raw_frames"] = frames_log
            frames_log.append({"fin": fin, "opcode": op, "len": len(raw), "hex": raw.hex()})
        data_started = time.monotonic()
        while True:
            fin, op, raw = ws.frame()
            if len(frames_log) < record_frames:
                frames_log.append({"fin": fin, "opcode": op, "len": len(raw), "hex": raw[:48].hex()})
            if op == 2 or op == 0:
                if not fin:
                    nfrag += 1
                nframes += 1
                size = len(raw)
                if size == 0:
                    nempty += 1
                    seq.append(("empty", nframes))
                sizes[size] += 1
                if nframes <= 6:
                    first.append(size)
                last.append(size)
                total += size
                digest.update(raw)
                if len(kept) < keep:
                    kept += raw[:keep - len(kept)]
                if hook and hook(nframes, total, ws) == "stop":
                    stopped = True
                    break
                if pace:
                    time.sleep(pace)
            elif op == 8:
                code = struct.unpack(">H", raw[:2])[0] if len(raw) >= 2 else None
                info["end"] = {"how": "close", "code": code, "reason": raw[2:].decode(errors="replace"),
                               "raw_close_payload_hex": raw.hex()}
                break
            elif op == 9:
                pass
            else:
                other.append({"opcode": op, "len": len(raw), "hex": raw[:32].hex()})
    except (EOFError, ConnectionError, OSError, TimeoutError) as error:
        info["end"] = {"how": "timeout" if isinstance(error, TimeoutError) else "eof", "error": repr(error)}
    now = time.monotonic()
    info["stopped_by_hook"] = stopped
    if stopped and cancel:
        {"close1000": lambda: ws.close(1000), "close1001": lambda: ws.close(1001, "going away"),
         "abort": ws.abort, "shutdown": lambda: ws.close()}[cancel]()
        info["cancel"] = cancel
    elif not stopped:
        # After the server's close, does anything else arrive, and does the TCP close?
        if info["end"]["how"] == "close":
            ws.s.settimeout(drain_after_close)
            try:
                trailing = ws.frame()
                info["after_close"] = {"opcode": trailing[1], "len": len(trailing[2])}
            except (EOFError, ConnectionError, OSError, TimeoutError) as error:
                info["after_close"] = "eof" if isinstance(error, EOFError) else repr(error)
        ws.close()
    info.update(bytes=total, frames=nframes, empty_frames=nempty, fragmented_frames=nfrag,
                max_frame=max(sizes) if sizes else 0, min_nonempty_frame=min((s for s in sizes if s), default=0),
                size_histogram=dict(sizes.most_common(6)), first_frame_sizes=first, last_frame_sizes=list(last),
                non_binary_frames=other, empty_at_frames=[n for k, n in seq], sha256=digest.hexdigest(),
                pings=ws.pings, elapsed_s=round(now - started, 3))
    if data_started:
        span = max(now - data_started, 1e-6)
        info["mb_per_s"] = round(total / MIB / span, 1)
    if keep:
        info["data"] = bytes(kept)
    return info


def summary(info):
    """The stream result without bulky fields, for results.json."""
    out = {k: v for k, v in info.items() if k not in ("data",)}
    return out


def fetch(ticket, **kw):
    info = stream(ticket, keep=kw.pop("keep", 1 << 30), **kw)
    log("stream", **{k: v for k, v in summary(info).items()})
    return info


def download(ctl, path, **kw):
    """download_begin then read the whole stream. Returns (begin response, stream info or None)."""
    reply = ctl.begin(path)
    payload = reply.get("payload")
    if not payload or "ticket" not in payload:
        return reply, None
    return reply, fetch(payload["ticket"], **kw)


def error_of(reply):
    e = reply.get("error")
    return {"code": e["code"], "status": e["status"], "message": e["message"]} if e else None


# ---------------------------------------------------------------- server and fixtures

def server_fds(match):
    """Descriptors the server holds that point under `match` (a path prefix), by /proc."""
    found = []
    try:
        for name in os.listdir(f"/proc/{SERVER_PID}/fd"):
            try:
                target = os.readlink(f"/proc/{SERVER_PID}/fd/{name}")
            except OSError:
                continue
            if target.startswith(str(match)) or target.replace(" (deleted)", "").startswith(str(match)):
                found.append(target)
    except OSError:
        pass
    return found


def start_server(rex, work, tag="api13"):
    work.mkdir(parents=True, exist_ok=True)
    env = {k: v for k, v in os.environ.items() if not k.startswith(("REX_", "HERDR_"))
           and k not in ("DBUS_SESSION_BUS_ADDRESS", "DISPLAY", "WAYLAND_DISPLAY", "TS_SOCKET")}
    for var, sub in (("HOME", "home"), ("XDG_CONFIG_HOME", "config"), ("XDG_STATE_HOME", "state"),
                     ("XDG_CACHE_HOME", "cache"), ("XDG_RUNTIME_DIR", "runtime")):
        (work / sub).mkdir(mode=0o700, exist_ok=True)
        env[var] = str(work / sub)
    sock = work / "rex.sock"
    env["REX_SERVER"] = "unix://" + str(sock)
    proc = subprocess.Popen([rex, "server", "run", "--listen", env["REX_SERVER"], "--restore=false",
                             "--config", "/dev/null", "--state-dir", str(work / "server-state")],
                            env=env, stdout=(work / "server.log").open("w"), stderr=subprocess.STDOUT,
                            start_new_session=True)
    deadline = time.monotonic() + 15
    while not sock.exists():
        assert proc.poll() is None and time.monotonic() < deadline, "private server did not start"
        time.sleep(.1)
    return proc, str(sock), env


def stop_server(proc):
    if proc.poll() is None:
        proc.send_signal(signal.SIGTERM)
        try:
            proc.wait(10)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()


def pattern(size, seed=0):
    """Deterministic content where each 8 bytes name their own offset, so any copy can be located."""
    block = b"".join(struct.pack(">Q", i) for i in range(seed, seed + (MIB // 8)))
    return (block * (size // MIB + 1))[:size]


def make_file(path, data=b"", mode=0o644):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    path.chmod(mode)
    return path


def settle(path=None, seconds=1.25):
    """Wait out the server's one-second rule: it gives no `version` for a file whose inode changed
    (ctime, not just mtime) within the last second, so backdating mtime does not help."""
    time.sleep(seconds)


# ---------------------------------------------------------------- scenarios

SCENARIOS = []


def scenario(slow=False):
    def wrap(fn):
        SCENARIOS.append((fn.__name__, fn, slow))
        return fn
    return wrap


@scenario()
def basic(ctx):
    """The plain case: request, reply, stream framing and end, byte-exact."""
    ctl = ctx.ctl
    content = b"hello world\n"
    path = make_file(FX / "hello.txt", content)
    reply = ctl.begin(str(path))
    payload = reply["payload"]
    result = {"begin": reply, "keys": sorted(payload)}
    check("begin reply has path,name,size,modified_at,ticket,expires_in_ms",
          {"path", "name", "size", "modified_at", "ticket", "expires_in_ms"} <= set(payload), sorted(payload))
    check("fresh file: no version (changed within the last second)", "version" not in payload, payload)
    info = fetch(payload["ticket"], keep=1 << 20, record_frames=16)
    result["stream"] = summary(info)
    (OUT / "basic-raw-frames.json").write_text(json.dumps(
        {"request": reply, "frames": info["raw_frames"], "end": info["end"]}, indent=1))
    w = info["welcome"]
    check("upgrade negotiates rex.data.bytes.v1", info["upgrade"]["subprotocol"] == BYTES, info["upgrade"])
    check("first message is a 32-byte WELCOME", w["kind"] == "WELCOME" and w["len"] == 32, w)
    check("content is byte-exact", info.get("data") == content, info.get("data"))
    end = info["end"]
    check("success ends with a close frame, status 1000, empty reason",
          end["how"] == "close" and end["code"] == 1000 and end["reason"] == "", end)
    check("the last data message before the close is an empty binary message (end marker)",
          info["empty_frames"] == 1 and info["last_frame_sizes"][-1] == 0, info["last_frame_sizes"])
    check("no frame is fragmented; only binary opcode used", info["fragmented_frames"] == 0 and not info["non_binary_frames"])
    check("the server sends no pings on a short stream", info["pings"] == [], info["pings"])
    result["server_fds_after"] = server_fds(FX)
    check("server holds no descriptor on the file after the stream ends", result["server_fds_after"] == [],
          result["server_fds_after"])
    return result


@scenario()
def version_and_conditional(ctx):
    """`version` presence, if_none_match, not_modified."""
    ctl = ctx.ctl
    path = make_file(FX / "settled.txt", b"settled content\n")
    immediate = ctl.probe(str(path))
    settle(path)
    first = ctl.probe(str(path))
    payload = first["payload"]
    result = {"immediately_after_write": immediate, "first": first}
    check("a file written a moment ago has no version", "version" not in immediate["payload"], immediate)
    check("a file unchanged for >1 s has a version", "version" in payload, payload)
    version = payload.get("version")
    # Abandon this ticket without using it, then ask conditionally.
    cond = ctl.probe(str(path), args={"if_none_match": version})
    result["conditional_match"] = cond
    cp = cond.get("payload", {})
    check("if_none_match with the current version -> not_modified, no ticket/expires_in_ms",
          cp.get("not_modified") is True and "ticket" not in cp and "expires_in_ms" not in cp, cond)
    check("not_modified reply still has path,name,size,modified_at,version",
          {"path", "name", "size", "modified_at"} <= set(cp), cond)
    mismatch = ctl.probe(str(path), args={"if_none_match": "bogus"})
    result["conditional_mismatch"] = mismatch
    check("if_none_match with another version -> normal reply with ticket",
          "ticket" in mismatch.get("payload", {}) and "not_modified" not in mismatch["payload"], mismatch)
    empty = ctl.probe(str(path), args={"if_none_match": ""})
    result["conditional_empty_string"] = empty
    wrong_type = ctl.probe(str(path), args={"if_none_match": 5})
    result["conditional_wrong_type"] = wrong_type
    # Does the version change with content, with same-size content, with touch, with chmod?
    versions = {"initial": version}
    path.write_bytes(b"settled content!\n")       # same size, different bytes
    settle(path)
    versions["same_size_rewrite"] = ctl.probe(str(path))["payload"].get("version")
    t = os.stat(path).st_mtime
    path.write_bytes(b"settled content?\n")       # same size, same mtime restored
    os.utime(path, (t, t))
    settle(path)
    versions["same_size_same_mtime"] = ctl.probe(str(path))["payload"].get("version")
    os.utime(path, (t - 100, t - 100))
    settle(path)
    versions["mtime_only_change"] = ctl.probe(str(path))["payload"].get("version")
    path.chmod(0o600)
    settle(path)
    versions["after_chmod"] = ctl.probe(str(path))["payload"].get("version")
    path.chmod(0o644)
    path.write_bytes(b"x" * 5000)
    settle(path)
    versions["grown"] = ctl.probe(str(path))["payload"].get("version")
    result["versions"] = versions
    check("version differs after a content change", versions["same_size_rewrite"] != versions["initial"], versions)
    result["version_of_same_size_same_mtime_equals_previous"] = (
        versions["same_size_same_mtime"] == versions["same_size_rewrite"])
    # A not_modified for a file whose copy changed: must produce a ticket.
    stale = ctl.probe(str(path), args={"if_none_match": version})
    result["stale_version_after_change"] = stale
    check("a stale version after the file changed gives a ticket, not not_modified",
          "ticket" in stale.get("payload", {}), stale)
    return result


@scenario()
def version_rules(ctx):
    """When `version` is withheld: which timestamps the server's one-second rule looks at."""
    ctl = ctx.ctl
    path = FX / "vrules.txt"
    table = {}

    def has_version(label):
        table[label] = "version" in ctl.probe(str(path))["payload"]
        return table[label]
    for label, offset in (("mtime 100 s ago", -100), ("mtime 0.5 s ago", -0.5), ("mtime 100 s ahead", 100),
                          ("mtime at the epoch plus 1 s", None)):
        make_file(path, b"a")
        time.sleep(1.3)
        now = time.time()
        os.utime(path, (1, 1) if offset is None else (now + offset, now + offset))
        has_version(f"{label}; inode changed just now")
        time.sleep(1.3)
        has_version(f"{label}; inode changed 1.3 s ago")
    make_file(path, b"a")
    time.sleep(1.3)
    has_version("untouched for 1.3 s")
    os.chmod(path, 0o600)
    has_version("chmod just now (ctime only)")
    time.sleep(1.3)
    has_version("chmod 1.3 s ago")
    os.rename(path, str(path) + ".moved")
    has_version_path = ctl.probe(str(path) + ".moved")["payload"]
    table["renamed just now"] = "version" in has_version_path
    os.rename(str(path) + ".moved", path)
    check("version is withheld while the inode changed within the last second (ctime), whatever mtime says",
          table["mtime 100 s ago; inode changed just now"] is False and table["mtime 100 s ago; inode changed 1.3 s ago"] is True
          and table["chmod just now (ctime only)"] is False, table)
    return {"has_version": table}


@scenario()
def refs(ctx):
    """Malformed and repeated refs; missing and extra arguments."""
    ctl = ctx.ctl
    path = str(make_file(FX / "ref.txt", b"ref\n"))
    good = str(uuid.uuid4())
    cases = {
        "valid": "download:" + good,
        "uppercase_uuid": "download:" + good.upper(),
        "no_prefix": good,
        "upload_prefix": "upload:" + good,
        "empty": "",
        "prefix_only": "download:",
        "not_uuid": "download:not-a-uuid",
        "uuid_no_hyphens": "download:" + good.replace("-", ""),
        "uuid_braced": "download:{" + good + "}",
        "nil_uuid": "download:00000000-0000-0000-0000-000000000000",
        "uuid_v7": "download:01a1c9b5-beb5-7731-b3d7-6ed7c2b65398",
        "trailing_space": "download:" + str(uuid.uuid4()) + " ",
        "prefix_capital": "Download:" + str(uuid.uuid4()),
        "double_prefix": "download:download:" + str(uuid.uuid4()),
        "very_long": "download:" + "a" * 5000,
        "nul": "download:" + str(uuid.uuid4()) + "\x00",
    }
    result = {}
    for name, ref in cases.items():
        r = ctl.probe(path, ref=ref)
        result[name] = {"ref": ref[:80], "error": error_of(r), "ok": "payload" in r,
                        "has_ticket": "ticket" in r.get("payload", {})}
    check("a valid ref is accepted", result["valid"]["ok"], result["valid"])
    for name in cases:
        if name not in ("valid",):
            result[name]["accepted"] = result[name]["ok"]
    # duplicate refs
    r1 = ctl.probe(path, ref="download:" + good)
    result["duplicate_same_uuid_unused_ticket_replay"] = {"error": error_of(r1), "ok": "payload" in r1}
    fresh = "download:" + str(uuid.uuid4())
    a = ctl.begin(path, ref=fresh)
    ia = fetch(a["payload"]["ticket"], keep=100)
    b = ctl.probe(path, ref=fresh)
    result["duplicate_after_completed_stream"] = {"first_end": ia["end"], "second": error_of(b) or "ok",
                                                  "second_has_ticket": "ticket" in b.get("payload", {})}
    inflight = "download:" + str(uuid.uuid4())
    big = make_file(FX / "dup-big.bin", pattern(8 * MIB))
    first = ctl.begin(str(big), ref=inflight)
    second = ctl.begin(str(big), ref=inflight)
    result["duplicate_while_first_unopened"] = {"second": error_of(second) or "ok",
                                                "tickets_equal": first["payload"].get("ticket") == second.get("payload", {}).get("ticket")}
    # open the first ticket; if the second was accepted, try it too
    s1 = fetch(first["payload"]["ticket"], keep=0)
    result["duplicate_first_stream_end"] = s1["end"]
    result["duplicate_first_stream_bytes"] = s1["bytes"]
    if "payload" in second and "ticket" in second["payload"]:
        s2 = fetch(second["payload"]["ticket"], keep=0)
        result["duplicate_second_stream"] = {"end": s2.get("end"), "bytes": s2.get("bytes"),
                                             "first_message": s2.get("first_message")}
    # duplicate while the first stream is still being read
    live = "download:" + str(uuid.uuid4())
    first = ctl.begin(str(big), ref=live)
    seen = {}

    def during(frames, total, ws):
        if frames == 2 and not seen:
            r = ctl.probe(str(big), ref=live)
            seen["second"] = error_of(r) or "ok"
            seen["second_has_ticket"] = "ticket" in r.get("payload", {})
        return None
    s = fetch(first["payload"]["ticket"], hook=during, keep=0)
    result["duplicate_while_first_streaming"] = {**seen, "first_end": s["end"], "first_bytes": s["bytes"]}
    # missing / extra / typed args
    shapes = {
        "no_ref": {"path": path},
        "no_path": {"ref": "download:" + str(uuid.uuid4())},
        "no_args_at_all": {},
        "extra_arg": {"ref": "download:" + str(uuid.uuid4()), "path": path, "bogus": 1},
        "ref_number": {"ref": 5, "path": path},
        "path_number": {"ref": "download:" + str(uuid.uuid4()), "path": 5},
        "path_null": {"ref": "download:" + str(uuid.uuid4()), "path": None},
        "path_empty": {"ref": "download:" + str(uuid.uuid4()), "path": ""},
    }
    result["shapes"] = {}
    for name, args in shapes.items():
        r = ctl.block_call("download_begin", args)
        result["shapes"][name] = error_of(r) or "ok"
    # block-method envelope problems
    result["envelope"] = {
        "no_session": error_of(ctl.call(TERMINAL + ".download_begin", {"block_id": ctl.block, "args": {"ref": new_ref(), "path": path}})),
        "bad_session": error_of(ctl.call(TERMINAL + ".download_begin", {"session_id": "session:" + str(uuid.uuid4()), "block_id": ctl.block, "args": {"ref": new_ref(), "path": path}})),
        "bad_block": error_of(ctl.call(TERMINAL + ".download_begin", {"session_id": ctl.session, "block_id": "block:" + str(uuid.uuid4()), "args": {"ref": new_ref(), "path": path}})),
        "no_args_key": error_of(ctl.call(TERMINAL + ".download_begin", {"session_id": ctl.session, "block_id": ctl.block})),
    }
    other = Control(name="unattached")
    result["envelope"]["unattached_connection"] = error_of(other.call(TERMINAL + ".download_begin", {
        "session_id": ctl.session, "block_id": ctl.block, "args": {"ref": new_ref(), "path": path}}))
    other.close()
    return result


@scenario()
def kinds(ctx):
    """What each kind of path answers."""
    ctl = ctx.ctl
    d = FX / "kinds"
    d.mkdir(exist_ok=True)
    make_file(d / "file.txt", b"target\n")
    (d / "dir").mkdir(exist_ok=True)
    make_file(d / "dir" / "inner.txt", b"inner\n")
    for name, target in (("link_to_file", "file.txt"), ("link_to_dir", "dir"), ("link_dangling", "nowhere"),
                         ("link_loop", "link_loop")):
        p = d / name
        if p.is_symlink() or p.exists():
            p.unlink()
        p.symlink_to(target)
    (d / "link_to_link").unlink(missing_ok=True)
    (d / "link_to_link").symlink_to("link_to_file")
    fifo = d / "fifo"
    if not fifo.exists():
        os.mkfifo(fifo)
    unreadable = make_file(d / "unreadable.txt", b"secret\n", mode=0o000)
    empty = make_file(d / "empty.bin", b"")
    nodir = d / "locked"
    nodir.mkdir(exist_ok=True)
    make_file(nodir / "in.txt", b"in\n")
    nodir.chmod(0)
    sock_path = d / "unix.sock"
    sock_path.unlink(missing_ok=True)
    srv = socket.socket(socket.AF_UNIX)
    srv.bind(str(sock_path))
    result = {}
    cases = {
        "missing": d / "missing.txt",
        "directory": d / "dir",
        "directory_trailing_slash": str(d / "dir") + "/",
        "file_trailing_slash": str(d / "file.txt") + "/",
        "symlink_to_file": d / "link_to_file",
        "symlink_to_symlink": d / "link_to_link",
        "symlink_to_dir": d / "link_to_dir",
        "symlink_dangling": d / "link_dangling",
        "symlink_loop": d / "link_loop",
        "unreadable_0000": unreadable,
        "inside_unsearchable_dir": nodir / "in.txt",
        "empty_file": empty,
        "unix_socket": sock_path,
        "dev_null": "/dev/null",
        "dev_zero": "/dev/zero",
        "proc_cpuinfo_size0_hint": "/proc/cpuinfo",
        "root_dir": "/",
        "path_through_file": str(d / "file.txt") + "/x",
    }
    for name, path in cases.items():
        reply = ctl.begin(str(path), ref=new_ref(), timeout=15)
        entry = {"path": str(path), "begin_error": error_of(reply)}
        payload = reply.get("payload")
        if payload:
            entry["begin"] = {k: v for k, v in payload.items() if k != "ticket"}
            info = fetch(payload["ticket"], keep=64, deadline=20)
            entry["stream"] = {k: info.get(k) for k in ("bytes", "frames", "empty_frames", "end", "first_message", "last_frame_sizes")}
            entry["head"] = info.get("data", b"")[:64].decode(errors="replace")
        result[name] = entry
    # FIFO: open() with no writer can block the server; use a separate connection and a timeout.
    other = Control(name="fifo")
    other.session, other.block = ctl.session, ctl.block
    other.call("session.attach", {"session_id": ctl.session})
    t0 = time.monotonic()
    try:
        reply = other.begin(str(fifo), timeout=8)
        result["fifo"] = {"begin_error": error_of(reply), "seconds": round(time.monotonic() - t0, 2),
                          "payload_keys": sorted(reply.get("payload", {}))}
        if reply.get("payload", {}).get("ticket"):
            info = fetch(reply["payload"]["ticket"], keep=32, deadline=8)
            result["fifo"]["stream"] = {k: info.get(k) for k in ("bytes", "end", "first_message")}
    except (TimeoutError, socket.timeout) as error:
        result["fifo"] = {"begin_error": "no reply within 8 s (hung)", "seconds": round(time.monotonic() - t0, 2)}
        # Unblock the server's open() so it doesn't stay parked: open the FIFO read-write.
        fd = os.open(fifo, os.O_RDWR | os.O_NONBLOCK)
        time.sleep(1)
        os.close(fd)
    try:
        other.ws.s.settimeout(5)
        result["fifo"]["server_responsive_after"] = "error" not in ctl.call("server.status")
    except Exception as error:
        result["fifo"]["server_responsive_after"] = repr(error)
    try:
        other.close()
    except Exception:
        pass
    srv.close()
    nodir.chmod(0o755)
    unreadable.chmod(0o644)

    def only_regular(name):
        e = result[name]
        return e.get("begin_error") is not None and "begin" not in e
    for name in ("missing", "directory", "directory_trailing_slash", "symlink_to_dir", "symlink_dangling",
                 "symlink_loop", "unreadable_0000", "unix_socket", "dev_null", "dev_zero", "root_dir",
                 "inside_unsearchable_dir", "path_through_file"):
        check(f"{name}: refused with an error and no ticket", only_regular(name), result[name].get("begin_error"))
    check("file_trailing_slash: accepted, path cleaned of the slash (a client must not assume it is echoed)",
          result["file_trailing_slash"].get("begin", {}).get("path", "").endswith("file.txt"),
          result["file_trailing_slash"].get("begin"))
    check("symlink_to_file: accepted, path left unexpanded",
          result["symlink_to_file"].get("begin", {}).get("path", "").endswith("link_to_file"),
          result["symlink_to_file"].get("begin"))
    check("empty file: accepted, size 0",
          result["empty_file"].get("begin", {}).get("size") == 0, result["empty_file"].get("begin"))
    return result


@scenario()
def names_and_paths(ctx):
    """What `name` and `path` look like for odd file names and path spellings."""
    ctl = ctx.ctl
    d = FX / "names"
    d.mkdir(exist_ok=True)
    names = {
        "spaces": "a file with spaces.txt",
        "unicode": "caf\u00e9 \u65e5\u672c\u8a9e \U0001f600.txt",
        "combining": "e\u0301.txt",
        "newline": "line1\nline2.txt",
        "tab": "tab\there.txt",
        "escape": "esc\x1b[31mred.txt",
        "del": "del\x7f.txt",
        "bell": "bell\x07.txt",
        "carriage_return": "cr\rname.txt",
        "rlo": "invoice\u202egpj.exe",
        "zero_width": "zero\u200bwidth.txt",
        "leading_dash": "-rf.txt",
        "double_dash": "--help",
        "leading_dot": ".hidden",
        "trailing_dot": "trailing.",
        "trailing_space": "trailing ",
        "dots_only": "...",
        "quote": "q'uote\"s.txt",
        "backslash": "back\\slash.txt",
        "percent": "100%25.txt",
        "colon": "a:b.txt",
        "long_255": "n" * 251 + ".txt",
    }
    result = {}
    for key, name in names.items():
        path = str(d / name)
        make_file(path, ("content of " + key + "\n").encode())
        reply = ctl.begin(path)
        entry = {"asked": path.replace(str(d) + "/", "<d>/"), "error": error_of(reply)}
        payload = reply.get("payload")
        if payload:
            entry["name"] = payload["name"]
            entry["path_is_asked"] = payload["path"] == path
            entry["path_tail"] = payload["path"].replace(str(d) + "/", "<d>/")
            entry["name_is_last_component_of_path"] = payload["path"].rsplit("/", 1)[-1] == payload["name"]
            info = fetch(payload["ticket"], keep=100, deadline=15)
            entry["body_ok"] = info.get("data", b"").startswith(b"content of ")
            entry["end"] = info.get("end")
        result[key] = entry
    check("a newline, a tab, ESC, DEL and a leading dash come back verbatim in name (client must sanitize)",
          all(result[k].get("name") == names[k] for k in ("newline", "tab", "escape", "del", "leading_dash", "rlo")),
          {k: result[k].get("name") for k in ("newline", "tab", "escape", "del", "leading_dash", "rlo")})
    # path spellings
    spell = FX / "spell"
    (spell / "sub").mkdir(parents=True, exist_ok=True)
    make_file(spell / "sub" / "f.txt", b"f\n")
    (spell / "link").unlink(missing_ok=True)
    (spell / "link").symlink_to("sub")
    forms = {
        "dot_dot": str(spell / "sub" / ".." / "sub" / "f.txt"),
        "dot": str(spell / "." / "sub" / "." / "f.txt"),
        "double_slash": str(spell) + "//sub//f.txt",
        "through_dir_symlink": str(spell / "link" / "f.txt"),
        "trailing_slash": str(spell / "sub" / "f.txt") + "/",
        "dot_dot_above_root": "/../../../" + str(spell / "sub" / "f.txt").lstrip("/"),
        "empty": "",
        "single_space": " ",
        "path_with_nul": str(spell / "sub" / "f.txt") + "\x00.png",
        "dot_only": ".",
        "dot_dot_only": "..",
        "very_long": "/" + "a" * 5000,
        "name_longer_than_255": "/" + "a" * 300,
        "unicode_escape_text": "\\u0066.txt",
        "tilde_only": "~",
        "tilde_slash": "~/",
        "tilde_user": "~root/x",
        "tilde_nonexistent_user": "~nosuchuser_xyz/x",
        "tilde_in_middle": str(spell) + "/~/x",
        "env_var": "$HOME/x",
        "glob": str(spell / "sub" / "*"),
        "file_url": "file://" + str(spell / "sub" / "f.txt"),
        "windows_style": "C:\\x\\y.txt",
    }
    paths = {}
    for key, path in forms.items():
        reply = ctl.probe(path)
        payload = reply.get("payload", {})
        paths[key] = {"asked": path[:60].replace(str(spell), "<spell>"), "error": error_of(reply),
                      "path": payload.get("path", "").replace(str(spell), "<spell>") or None,
                      "name": payload.get("name")}
    result["path_forms"] = paths
    return result


@scenario()
def home_and_cwd(ctx):
    """`~`, and paths relative to the foreground process's directory, including after cd."""
    ctl = ctx.ctl
    session, block = ctl.new_session("cwd")
    root = FX / "cwd"
    (root / "dir one").mkdir(parents=True, exist_ok=True)
    (root / "dir two").mkdir(parents=True, exist_ok=True)
    make_file(root / "dir one" / "one.txt", b"one\n")
    make_file(root / "dir two" / "two.txt", b"two\n")
    make_file(root / "dir one" / "both.txt", b"in one\n")
    make_file(root / "dir two" / "both.txt", b"in two\n")
    home = Path(ctx.env["HOME"])
    make_file(home / "tilde.txt", b"from HOME\n")
    result = {"HOME_env_of_server": str(home), "passwd_home": os.path.expanduser("~root") and
              subprocess.check_output(["getent", "passwd", str(os.getuid())], text=True).split(":")[5]}

    def process():
        r = ctl.block_call("process", session=session, block=block)
        return r.get("payload", {})

    def cwd_is(expected, timeout=10):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            p = process()
            if (p.get("child") or {}).get("cwd") == str(expected):
                return True
            time.sleep(.1)
        return False

    def ask(path, label):
        r = ctl.begin(path, session=session, block=block)
        payload = r.get("payload", {})
        entry = {"asked": path, "error": error_of(r), "path": payload.get("path"), "size": payload.get("size")}
        if payload.get("ticket"):
            entry["body"] = fetch(payload["ticket"], keep=200).get("data", b"").decode(errors="replace")
        result[label] = entry
        return entry

    result["process_at_start"] = process()
    result["pwd_method"] = ctl.block_call("pwd", session=session, block=block).get("payload")
    ask("~/tilde.txt", "tilde_file")
    ask("~", "tilde_alone")
    ask("tilde.txt", "relative_at_start")
    ask("./tilde.txt", "dot_relative_at_start")
    ctl.block_call("write", {"data": f"cd '{root / 'dir one'}'\n"}, session=session, block=block)
    result["cwd_after_cd_one_matched"] = cwd_is(root / "dir one")
    ask("one.txt", "relative_after_cd")
    ask("both.txt", "relative_both_in_one")
    ask("../dir two/two.txt", "relative_dotdot_after_cd")
    ask("~/tilde.txt", "tilde_after_cd")
    # Foreground process other than the shell, in another directory.
    ctl.block_call("write", {"data": f"sh -c 'cd \"{root / 'dir two'}\" && exec sleep 600'\n"}, session=session, block=block)
    time.sleep(.5)
    deadline = time.monotonic() + 8
    p = process()
    while time.monotonic() < deadline and (p.get("child") or {}).get("cwd") != str(root / "dir two"):
        time.sleep(.2)
        p = process()
    result["process_with_foreground_sleep"] = p
    ask("two.txt", "relative_with_foreground_sleep_in_two")
    ask("both.txt", "relative_both_with_foreground_in_two")
    ask("one.txt", "relative_one_with_foreground_in_two")
    ctl.block_call("write", {"data": "\x03"}, session=session, block=block)
    result["cwd_back_in_one_after_ctrl_c"] = cwd_is(root / "dir one")
    ask("both.txt", "relative_both_after_ctrl_c")
    # A directory whose name is not UTF-8: how does `path` look when the cwd contains it?
    bad = os.fsencode(root) + b"/bad-\xff-dir"
    os.makedirs(bad, exist_ok=True)
    with open(bad + b"/in-bad.txt", "wb") as fh:
        fh.write(b"in bad dir\n")
    ctl.block_call("write", {"data": "cd \"$(printf '%s/bad-\\377-dir' '" + str(root) + "')\"\n"}, session=session, block=block)
    time.sleep(1.0)
    result["process_in_non_utf8_cwd"] = process()
    ask("in-bad.txt", "relative_in_non_utf8_cwd")
    ctl.block_call("write", {"data": f"cd '{root / 'dir one'}'\n"}, session=session, block=block)
    cwd_is(root / "dir one")
    # Process that deleted its cwd.
    gone = root / "gone"
    gone.mkdir(exist_ok=True)
    ctl.block_call("write", {"data": f"cd '{gone}'\n"}, session=session, block=block)
    cwd_is(gone)
    gone.rmdir()
    time.sleep(.3)
    result["process_in_deleted_cwd"] = process()
    ask("tilde.txt", "relative_in_deleted_cwd")
    ask("/etc/hostname", "absolute_in_deleted_cwd")
    check("~ is the home the server's own HOME names or the passwd entry (recorded)",
          True, (result["tilde_file"].get("path"), result["HOME_env_of_server"], result["passwd_home"]))
    return result


@scenario()
def throughput(ctx):
    """A 256 MiB sparse file, a 64 MiB patterned file, a 1 MiB file: frames, throughput, integrity."""
    ctl = ctx.ctl
    big = FX / "sparse-256MiB.bin"
    with open(big, "wb") as fh:
        fh.truncate(256 * MIB)
    expected = hashlib.sha256()
    zeros = bytes(MIB)
    for _ in range(256):
        expected.update(zeros)
    r = ctl.begin(str(big))
    result = {"begin": {k: v for k, v in r["payload"].items() if k != "ticket"}}
    info = fetch(r["payload"]["ticket"], keep=0)
    result["sparse_256MiB"] = summary(info)
    check("256 MiB sparse: all bytes arrive, hash matches, ends 1000",
          info["bytes"] == 256 * MIB and info["sha256"] == expected.hexdigest() and info["end"].get("code") == 1000,
          {k: info.get(k) for k in ("bytes", "end")})
    check("data frames are at most 65536 bytes (observed: exactly 32768 except the last)", info["max_frame"] <= 65536, info["max_frame"])
    patterned = make_file(FX / "pattern-64MiB.bin", pattern(64 * MIB))
    expect2 = hashlib.sha256(patterned.read_bytes()).hexdigest()
    r2 = ctl.begin(str(patterned))
    info2 = fetch(r2["payload"]["ticket"], keep=0)
    result["pattern_64MiB"] = summary(info2)
    check("64 MiB patterned: byte-exact", info2["sha256"] == expect2 and info2["bytes"] == 64 * MIB)
    one = make_file(FX / "one-mib-plus.bin", pattern(MIB + 17))
    r3 = ctl.begin(str(one))
    info3 = fetch(r3["payload"]["ticket"], keep=2 * MIB)
    result["one_MiB_plus_17"] = summary(info3)
    check("frame sizes for a 1 MiB+17 file", True, info3["size_histogram"])
    return result


@scenario()
def sizes_and_read_errors(ctx):
    """Very large size hints (no data is transferred), and a read error part way (EIO)."""
    ctl = ctx.ctl
    result = {}
    for label, size in (("5_GiB", 5 << 30), ("1_TiB", 1 << 40)):
        path = FX / f"sparse-{label}.bin"
        try:
            with open(path, "wb") as fh:
                fh.truncate(size)
        except OSError as error:
            result[label] = {"skipped": repr(error)}
            continue
        reply = ctl.probe(str(path))
        result[label] = {"asked": size, "size_in_reply": reply.get("payload", {}).get("size"),
                         "error": error_of(reply)}
        path.unlink()
    check("a size hint above 2^32 is reported exactly", result["5_GiB"].get("size_in_reply") == 5 << 30, result["5_GiB"])
    # /proc/self/mem is a regular file for the server's own process; reading offset 0 fails with EIO.
    reply = ctl.begin("/proc/self/mem")
    entry = {"begin": {k: v for k, v in reply.get("payload", {}).items() if k != "ticket"}, "error": error_of(reply)}
    if reply.get("payload", {}).get("ticket"):
        info = fetch(reply["payload"]["ticket"], keep=64, deadline=20, record_frames=8)
        entry["stream"] = {k: info.get(k) for k in ("bytes", "frames", "empty_frames", "end", "raw_frames")}
    result["read_error_proc_self_mem"] = entry
    return result


@scenario()
def mutation(ctx):
    """A file changing while it is being sent. The reader pauses after a few frames."""
    ctl = ctx.ctl
    base_size = 16 * MIB
    result = {}

    def run(label, mutate, size=base_size, rewind=False, before_stream=False, pause=1.5, keep=0):
        path = FX / f"mut-{label}.bin"
        make_file(path, pattern(size))
        reply = ctl.begin(str(path))
        payload = reply["payload"]
        entry = {"begin_size": payload["size"], "fds_after_begin": len(server_fds(FX))}
        state = {}

        def hook(frames, total, ws):
            if frames == 4 and "done" not in state:
                state["done"] = True
                state["bytes_at_mutation"] = total
                time.sleep(.2)
                mutate(path)
                time.sleep(pause)
            return None
        if before_stream:
            mutate(path)
            info = fetch(payload["ticket"], keep=keep)
        else:
            info = fetch(payload["ticket"], hook=hook, keep=keep, deadline=60)
        entry.update(state)
        entry["stream"] = {k: info.get(k) for k in ("bytes", "frames", "end", "empty_frames", "last_frame_sizes", "sha256")}
        entry["delta_vs_hint"] = info.get("bytes", 0) - payload["size"]
        entry["fds_after_stream"] = len(server_fds(FX))
        return entry, info, path

    def grow(path):
        with open(path, "ab") as fh:
            fh.write(pattern(2 * MIB, seed=900000))

    def shrink(path):
        os.truncate(path, 1 * MIB)

    def to_zero(path):
        os.truncate(path, 0)

    def replace_rename(path):
        tmp = path.with_suffix(".new")
        tmp.write_bytes(b"REPLACEMENT" * 1000)
        os.replace(tmp, path)

    def delete(path):
        path.unlink()

    def overwrite_same_size(path):
        with open(path, "r+b") as fh:
            fh.seek(8 * MIB)
            fh.write(b"\xee" * MIB)

    def chmod_000(path):
        path.chmod(0)

    for label, fn in (("grow_2MiB_midstream", grow), ("truncate_to_1MiB_midstream", shrink),
                      ("truncate_to_zero_midstream", to_zero), ("replace_by_rename_midstream", replace_rename),
                      ("delete_midstream", delete), ("overwrite_in_place_midstream", overwrite_same_size),
                      ("chmod_000_midstream", chmod_000)):
        entry, info, path = run(label, fn)
        result[label] = entry
        path.chmod(0o644) if path.exists() else None
    # Changes between download_begin and opening the stream
    for label, fn in (("grow_before_stream", grow), ("truncate_before_stream", shrink),
                      ("delete_before_stream", delete), ("replace_before_stream", replace_rename)):
        entry, info, path = run(label, fn, before_stream=True, size=4 * MIB)
        result[label] = entry
    # Does the server hold the file open between begin and HELLO?
    path = make_file(FX / "mut-held.bin", pattern(2 * MIB))
    reply = ctl.begin(str(path))
    result["fd_held_between_begin_and_hello"] = server_fds(FX / "mut-held.bin")
    fetch(reply["payload"]["ticket"], keep=0)
    result["fd_held_after_stream"] = server_fds(FX / "mut-held.bin")
    # Truncation of a large file after the server's read position (it reads ahead several MiB).
    path = FX / "mut-large-truncate.bin"
    with open(path, "wb") as fh:
        fh.truncate(256 * MIB)
    reply = ctl.begin(str(path))
    state = {}

    def trunc(frames, total, ws):
        if frames == 4 and not state:
            state["bytes_at_truncate"] = total
            os.truncate(path, 64 * MIB)
            time.sleep(.5)
    info = fetch(reply["payload"]["ticket"], hook=trunc, keep=0)
    result["truncate_256MiB_to_64MiB_midstream"] = {"hint": reply["payload"]["size"], **state, "bytes": info["bytes"],
                                                    "end": info.get("end")}
    path.unlink()
    # Continuous growth: a writer appends faster than a paced reader reads. The server reads to EOF
    # whenever it gets there and has no cap of its own; the reader gives up at 120 MiB.
    path = make_file(FX / "mut-endless.bin", pattern(16 * MIB))
    reply = ctl.begin(str(path))
    stop = threading.Event()
    written = {"bytes": 0}

    def writer():
        chunk = b"g" * (256 * 1024)
        with open(path, "ab") as fh:
            while not stop.is_set() and written["bytes"] < 400 * MIB:
                fh.write(chunk)
                fh.flush()
                written["bytes"] += len(chunk)
                time.sleep(.002)
    thread = threading.Thread(target=writer)
    thread.start()
    limit = 120 * MIB
    t0 = time.monotonic()

    def cap(frames, total, ws):
        return "stop" if total >= limit or time.monotonic() - t0 > 40 else None
    info = fetch(reply["payload"]["ticket"], hook=cap, cancel="close1000", keep=0, pace=.004)
    stop.set()
    thread.join()
    path.unlink()
    result["continuous_growth"] = {"hint": reply["payload"]["size"], "reader_gave_up_at_bytes": info["bytes"],
                                   "stopped_by_reader": info["stopped_by_hook"], "end": info.get("end"),
                                   "seconds": info["elapsed_s"], "writer_appended_bytes": written["bytes"]}
    check("the server does not stop a growing file at the size hint (a client needs its own cap)",
          info["stopped_by_hook"] and info["bytes"] >= 6 * reply["payload"]["size"], result["continuous_growth"])
    check("truncation after the server's read position ends the stream normally (1000) but short",
          result["truncate_256MiB_to_64MiB_midstream"]["bytes"] < 256 * MIB
          and result["truncate_256MiB_to_64MiB_midstream"]["end"].get("code") == 1000,
          result["truncate_256MiB_to_64MiB_midstream"])
    return result


@scenario()
def cancel_and_drop(ctx):
    """What the client can do mid-stream and what the server does about it."""
    ctl = ctx.ctl
    path = make_file(FX / "cancel.bin", pattern(128 * MIB))
    result = {}

    def after(cancel):
        reply = ctl.begin(str(path))
        info = fetch(reply["payload"]["ticket"], hook=lambda f, t, ws: "stop" if f >= 3 else None, cancel=cancel, keep=0)
        time.sleep(1.0)
        fds = server_fds(FX / "cancel.bin")
        return reply, info, fds
    for cancel in ("close1000", "close1001", "abort", "shutdown"):
        reply, info, fds = after(cancel)
        result[cancel] = {"bytes_before_cancel": info["bytes"], "server_fds_1s_later": fds}
    check("after a client close 1000, the server releases the file", result["close1000"]["server_fds_1s_later"] == [],
          result["close1000"])
    # Cancel by an empty binary message (the upload-style end marker) and by garbage: ignored?
    reply = ctl.begin(str(path))
    flags = {}

    def poke(frames, total, ws):
        if frames == 2 and "poked" not in flags:
            flags["poked"] = True
            ws.send(2, b"")
            ws.send(2, b"\x00\x01\x02")
            ws.send(1, b"hello")
        return "stop" if frames >= 40 else None
    info = fetch(reply["payload"]["ticket"], hook=poke, cancel="close1000", keep=0)
    result["client_sends_empty_binary_binary_and_text_midstream"] = {
        "continued_after_poke": info["frames"] >= 40, "end": info.get("end"), "frames": info["frames"]}
    # Resume after an abrupt drop (RST): the docs say a non-1000 close leaves the stream waiting for RESUME.
    reply = ctl.begin(str(path))
    state = {}

    def hook(frames, total, ws):
        if frames == 3:
            state["total"] = total
            state["welcome"] = ws
            return "stop"
    info = fetch(reply["payload"]["ticket"], hook=hook, cancel="abort", keep=0)
    token = bytes.fromhex(info["welcome"]["token"])
    received = info["bytes"]
    try:
        resumed = stream(None, hello=b"\x02" + token + struct.pack("<Q", received), keep=1024, deadline=20,
                         hook=lambda f, t, ws: "stop" if f >= 5 else None, cancel="close1000")
        result["resume_after_rst"] = {"received_before": received, "first_message": resumed.get("first_message"),
                                      "bytes_after_resume": resumed.get("bytes"), "end": resumed.get("end")}
        if resumed.get("data") is not None and resumed.get("first_message", {}).get("kind") == "WELCOME":
            expected = pattern(128 * MIB)[received:received + len(resumed["data"])]
            result["resume_after_rst"]["continues_exactly"] = resumed["data"] == expected
    except Exception as error:
        result["resume_after_rst"] = {"error": repr(error)}
    # Control connection closes mid-stream.
    other = Control(name="goes-away")
    other.session, other.block = ctl.session, ctl.block
    other.call("session.attach", {"session_id": ctl.session})
    reply = other.begin(str(path))
    flags2 = {}

    def hook2(frames, total, ws):
        if frames == 3:
            other.close()
            flags2["closed_control"] = True
            time.sleep(.5)
        return "stop" if frames >= 2000 else None
    info = fetch(reply["payload"]["ticket"], hook=hook2, cancel="close1000", keep=0, deadline=30)
    result["control_connection_closed_midstream"] = {"frames_read": info["frames"], "end": info.get("end"),
                                                     "continued": info["frames"] >= 2000}
    # Ticket outlives its control connection?
    other = Control(name="issues-ticket-then-leaves")
    other.session, other.block = ctl.session, ctl.block
    other.call("session.attach", {"session_id": ctl.session})
    small = make_file(FX / "ticket-outlives.txt", b"still here\n")
    reply = other.begin(str(small))
    other.close()
    time.sleep(.5)
    info = fetch(reply["payload"]["ticket"], keep=100)
    result["ticket_after_issuing_control_closed"] = {"first_message": info.get("first_message"), "end": info.get("end"),
                                                     "data": info.get("data")}
    return result


@scenario()
def tickets(ctx):
    """Reused, concurrent, malformed, wrong-subprotocol tickets and other handshake errors."""
    ctl = ctx.ctl
    path = make_file(FX / "tk.bin", pattern(2 * MIB))
    result = {}
    # reuse after a completed stream
    r = ctl.begin(str(path))
    t = r["payload"]["ticket"]
    first = fetch(t, keep=0)
    again = fetch(t, keep=0)
    result["reused_after_complete"] = {"first_end": first["end"], "second": {k: again.get(k) for k in ("first_message", "end", "bytes")}}
    check("a ticket used once is rejected the second time (REJECT 0x01, then close)",
          again["first_message"].get("kind") == "REJECT" and again["first_message"].get("reason") == 1, again["first_message"])
    result["reject_close"] = again.get("end")
    # two connections race for one ticket
    r = ctl.begin(str(path))
    t = r["payload"]["ticket"]
    a, b = WS("/api/data", BYTES), WS("/api/data", BYTES)
    out = {}

    def go(name, ws):
        out[name] = stream(t, ws=ws, keep=0)
    ta, tb = threading.Thread(target=go, args=("a", a)), threading.Thread(target=go, args=("b", b))
    ta.start()
    tb.start()
    ta.join()
    tb.join()
    result["concurrent_same_ticket"] = {k: {x: v.get(x) for x in ("first_message", "bytes", "end")} for k, v in out.items()}
    # reuse while the first is mid-stream
    big = make_file(FX / "tk-big.bin", pattern(32 * MIB))
    r = ctl.begin(str(big))
    t = r["payload"]["ticket"]
    seen = {}

    def hook(frames, total, ws):
        if frames == 3 and not seen:
            seen["second"] = {k: v for k, v in stream(t, keep=0).items() if k in ("first_message", "end", "bytes")}
    first = fetch(t, hook=hook, keep=0)
    result["reuse_while_first_midstream"] = {"first": {k: first.get(k) for k in ("bytes", "end")}, **seen}
    # malformed HELLOs
    r = ctl.begin(str(path))
    t = r["payload"]["ticket"]
    raw = hello_message(t)
    bad = {
        "empty_message": b"",
        "type_only": b"\x01",
        "short_ticket_15": raw[:16],
        "ticket_plus_trailing_bytes": raw + b"extra",   # docs: trailing bytes ignored
        "wrong_type_byte_0x03": b"\x03" + raw[1:],
        "all_zero_ticket": b"\x01" + bytes(16),
        "text_garbage": b"GET / HTTP/1.0",
    }
    result["handshakes"] = {}
    for name, message in bad.items():
        fresh = ctl.begin(str(path))["payload"]["ticket"] if name == "ticket_plus_trailing_bytes" else t
        if name == "ticket_plus_trailing_bytes":
            message = hello_message(fresh) + b"extra"
        info = stream(None, hello=message, keep=0, deadline=10)
        result["handshakes"][name] = {k: info.get(k) for k in ("first_message", "end", "bytes", "after_reject")}
        if name not in ("ticket_plus_trailing_bytes",):
            t = ctl.begin(str(path))["payload"]["ticket"]
    # no HELLO at all within 5 s
    ws = WS("/api/data", BYTES)
    ws.s.settimeout(12)
    t0 = time.monotonic()
    try:
        got = []
        while True:
            fin, op, data = ws.frame()
            got.append({"opcode": op, "hex": data[:16].hex()})
            if op == 8:
                break
        result["no_hello"] = {"seconds": round(time.monotonic() - t0, 1), "frames": got}
    except Exception as error:
        result["no_hello"] = {"seconds": round(time.monotonic() - t0, 1), "error": repr(error), "frames": got}
    ws.close()
    # wrong subprotocol on the WebSocket
    t = ctl.begin(str(path))["payload"]["ticket"]
    info = stream(t, protocol="rex.data.terminal.v1", keep=0, deadline=10)
    result["download_ticket_on_terminal_subprotocol"] = {k: info.get(k) for k in ("upgrade", "first_message", "end", "upgrade_error", "bytes")}
    # a data.connect (terminal) ticket on the bytes subprotocol
    dc = ctl.call("data.connect", {"session_id": ctl.session, "block_id": ctl.block, "data_ref_id": "terminal",
                                   "subprotocol": "rex.data.terminal.v1"})
    result["terminal_ticket_on_bytes_subprotocol"] = {}
    if "payload" in dc:
        info = stream(dc["payload"]["ticket"], protocol=BYTES, keep=0, deadline=10)
        result["terminal_ticket_on_bytes_subprotocol"] = {k: info.get(k) for k in ("first_message", "end", "bytes")}
    # upgrade with no subprotocol / unknown subprotocol
    for name, proto in (("none_requested", ""), ("unknown", "rex.data.nope.v1")):
        try:
            s = socket.socket(socket.AF_UNIX)
            s.settimeout(5)
            s.connect(SOCK)
            hdr = ("GET /api/data HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n"
                   f"Sec-WebSocket-Key: {base64.b64encode(os.urandom(16)).decode()}\r\nSec-WebSocket-Version: 13\r\n"
                   + (f"Sec-WebSocket-Protocol: {proto}\r\n" if proto else "") + "\r\n")
            s.sendall(hdr.encode())
            result.setdefault("upgrade_subprotocols", {})[name] = s.recv(2000).decode(errors="replace").split("\r\n")[0:6]
            s.close()
        except Exception as error:
            result.setdefault("upgrade_subprotocols", {})[name] = repr(error)
    return result


@scenario(slow=True)
def ticket_expiry(ctx):
    """A ticket that is never used: its lifetime and the descriptor the server holds meanwhile."""
    ctl = ctx.ctl
    path = make_file(FX / "expiry.bin", pattern(MIB))
    r = ctl.begin(str(path))
    payload = r["payload"]
    life = payload["expires_in_ms"]
    t0 = time.monotonic()
    result = {"expires_in_ms": life, "fds_at_start": server_fds(FX / "expiry.bin")}
    time.sleep(life / 1000 / 2)
    result["fds_at_half_life"] = server_fds(FX / "expiry.bin")
    time.sleep(life / 1000 / 2 + 2.5)
    result["fds_after_expiry"] = server_fds(FX / "expiry.bin")
    info = fetch(payload["ticket"], keep=0)
    result["use_after_expiry"] = {k: info.get(k) for k in ("first_message", "end", "bytes")}
    result["seconds_waited"] = round(time.monotonic() - t0, 1)
    check("an expired ticket is rejected with REJECT 0x01",
          info["first_message"].get("kind") == "REJECT" and info["first_message"].get("reason") == 1, info["first_message"])
    # a ticket used right at ~ half life works
    r = ctl.begin(str(path))
    time.sleep(life / 1000 / 2)
    info = fetch(r["payload"]["ticket"], keep=0)
    result["use_at_half_life"] = {k: info.get(k) for k in ("first_message", "end", "bytes")}
    return result


@scenario(slow=True)
def stalled_reader(ctx):
    """A reader that stops reading for a long time: is it dropped, and with what?"""
    ctl = ctx.ctl
    path = make_file(FX / "stall.bin", pattern(64 * MIB))
    r = ctl.begin(str(path))
    state = {}

    def hook(frames, total, ws):
        if frames == 3:
            state["paused_at"] = total
            state["fds_while_paused"] = len(server_fds(FX / "stall.bin"))
            time.sleep(100)
    info = fetch(r["payload"]["ticket"], hook=hook, keep=0, deadline=240)
    return {"paused_for_s": 100, **state, "after_pause": {k: info.get(k) for k in ("bytes", "end", "pings")},
            "completed_whole_file": info["bytes"] == 64 * MIB}


@scenario(slow=True)
def idle_pings(ctx):
    """Does an idle control connection get pings within 65 s? (The data-connection case is covered by
    stalled_reader, which answers pings if any arrive.)"""
    ctl = Control(name="idle")
    start = time.monotonic()
    ws = ctl.ws
    ws.s.settimeout(70)
    seen = []
    try:
        while time.monotonic() - start < 65:
            fin, op, data = ws.frame()
            seen.append((round(time.monotonic() - start, 1), op))
    except (TimeoutError, OSError) as error:
        seen.append(repr(error))
    return {"control_connection_frames_in_65s": seen, "pings_answered": ws.pings}


@scenario()
def reservations(ctx):
    """The cap on unopened or running downloads per terminal, and what releases a slot."""
    ctl = ctx.ctl
    path = make_file(FX / "reserve.bin", pattern(2 * MIB))
    result = {}

    def fill(c=None, block=None):
        c = c or ctl
        held = []
        while True:
            r = c.begin(str(path), block=block)
            if "error" in r:
                return held, error_of(r)
            held.append(r["payload"]["ticket"])
            if len(held) > 64:
                return held, None

    def one(c=None, block=None):
        r = (c or ctl).begin(str(path), block=block)
        return r, ("ok" if "payload" in r else error_of(r))

    held, err = fill()
    result["unopened_until_refused"] = {"accepted": len(held), "refusal": err}
    check("the 17th reservation on a terminal is refused with conflict", len(held) == 16 and err and err["code"] == "conflict", result["unopened_until_refused"])
    r, v = one()
    result["still_full_17th_again"] = v
    # completion frees a slot
    done = fetch(held.pop(), keep=0)
    r, v = one()
    result["after_one_stream_completed"] = v
    if "payload" in r:
        held.append(r["payload"]["ticket"])
    # cancel with close 1000 after a frame
    fetch(held.pop(), hook=lambda f, t, ws: "stop", cancel="close1000", keep=0)
    time.sleep(.3)
    r, v = one()
    result["after_close_1000_midstream"] = v
    if "payload" in r:
        held.append(r["payload"]["ticket"])
    # abort (RST) after a frame: stream waits for a resume, so the slot may stay taken
    fetch(held.pop(), hook=lambda f, t, ws: "stop", cancel="abort", keep=0)
    time.sleep(.3)
    r, v = one()
    result["after_rst_midstream"] = v
    if "payload" in r:
        held.append(r["payload"]["ticket"])
    time.sleep(.5)
    r, v = one()
    result["after_rst_midstream_second_try"] = v
    if "payload" in r:
        held.append(r["payload"]["ticket"])
    # a refused reply does not take a slot; errors for missing files do not either
    result["missing_file_when_full"] = error_of(ctl.begin(str(FX / "nope")))
    # another client on the same terminal shares the cap
    other = Control(name="second")
    other.session, other.block = ctl.session, ctl.block
    other.call("session.attach", {"session_id": ctl.session})
    r, v = one(other)
    result["second_client_same_terminal"] = v
    if "payload" in r:
        held.append(r["payload"]["ticket"])
    # another terminal has its own cap
    split = ctl.call("session.new_split", {"session_id": ctl.session, "anchor_id": ctl.block, "direction": "horizontal",
                                           "side": "after", "focus": False,
                                           "layout": {"block": {"flavor": TERMINAL + ".shell", "options": {"shell": "/bin/sh"}}}})
    block2 = split["payload"]["block_ids"][0]
    r, v = one(block=block2)
    result["second_terminal"] = v
    if "payload" in r:
        held.append(r["payload"]["ticket"])
    # a reconnecting client: the first client leaves with its slots reserved
    first_client_slots = len(held)
    other.close()
    time.sleep(.5)
    r, v = one()
    result["after_second_client_left"] = v
    ctl2 = Control(name="third")
    ctl2.session, ctl2.block = ctl.session, ctl.block
    ctl2.call("session.attach", {"session_id": ctl.session})
    r2, v2 = one(ctl2)
    result["new_client_same_terminal_while_first_holds_slots"] = v2
    ctl2.close()
    # slots held by a client whose control connection closes: freed with it, or kept until expiry?
    leaver = Control(name="leaves-holding-slots")
    leaver.session, leaver.block = ctl.session, ctl.block
    leaver.call("session.attach", {"session_id": ctl.session})
    block3 = ctl.call("session.new_split", {"session_id": ctl.session, "anchor_id": ctl.block, "direction": "vertical",
                                            "side": "after", "focus": False,
                                            "layout": {"block": {"flavor": TERMINAL + ".shell", "options": {"shell": "/bin/sh"}}}})["payload"]["block_ids"][0]
    taken = 0
    while "payload" in leaver.begin(str(path), block=block3):
        taken += 1
        if taken > 20:
            break
    leaver.close()
    time.sleep(.5)
    r, v = one(block=block3)
    result["after_client_closed_holding_16_unopened_tickets"] = {"slots_it_held": taken, "next_begin": v}
    if "payload" in r:
        release_ticket(r["payload"]["ticket"])
    # drain everything so later scenarios start clean
    for ticket in held:
        try:
            release_ticket(ticket)
        except Exception:
            pass
    time.sleep(.3)
    r, v = one()
    result["after_everything_released"] = v
    if "payload" in r:
        release_ticket(r["payload"]["ticket"])
    return result


@scenario(slow=True)
def reservation_expiry(ctx):
    """Does an unopened ticket's slot come back when the ticket expires (about 60 s)?"""
    ctl = ctx.ctl
    path = make_file(FX / "reserve-expiry.bin", pattern(MIB))
    held = []
    for _ in range(16):
        held.append(ctl.begin(str(path))["payload"])
    full = error_of(ctl.begin(str(path)))
    t0 = time.monotonic()
    life = held[0]["expires_in_ms"] / 1000
    timeline = []
    while time.monotonic() - t0 < life + 8:
        time.sleep(4)
        r = ctl.begin(str(path))
        timeline.append((round(time.monotonic() - t0, 1), "ok" if "payload" in r else r["error"]["code"]))
        if "payload" in r:
            break
    return {"refused_when_full": full, "timeline_of_retries": timeline, "expires_in_ms": held[0]["expires_in_ms"]}


@scenario()
def concurrency(ctx):
    """Several downloads at once on one terminal (up to the cap), throughput, and ordinary use meanwhile."""
    ctl = ctx.ctl
    path = make_file(FX / "conc.bin", pattern(24 * MIB))
    expected = hashlib.sha256(path.read_bytes()).hexdigest()
    result = {}
    for n in (3, 8, 16):
        refs_ = [ctl.begin(str(path)) for _ in range(n)]
        errors = [error_of(r) for r in refs_ if "error" in r]
        tickets = [r["payload"]["ticket"] for r in refs_ if "payload" in r]
        t0 = time.monotonic()
        with concurrent.futures.ThreadPoolExecutor(n) as pool:
            infos = list(pool.map(lambda tk: stream(tk, keep=0, deadline=120), tickets))
        secs = time.monotonic() - t0
        result[f"{n}_at_once"] = {
            "begin_errors": errors[:3], "begun": len(tickets),
            "all_complete_exact": all(i.get("sha256") == expected and i["end"].get("code") == 1000 for i in infos),
            "rejects": [i["first_message"] for i in infos if i["first_message"].get("kind") != "WELCOME"][:3],
            "ends": dict(collections.Counter(str(i["end"].get("code")) + ":" + i["end"].get("reason", "") for i in infos)),
            "seconds": round(secs, 2), "aggregate_mb_per_s": round(n * 24 / secs, 1)}
    check("16 concurrent downloads of one terminal complete byte-exact", result["16_at_once"]["all_complete_exact"], result["16_at_once"])
    other = Control(name="second-client")
    other.session, other.block = ctl.session, ctl.block
    other.call("session.attach", {"session_id": ctl.session})
    ra, rb = ctl.begin(str(path)), other.begin(str(path))
    with concurrent.futures.ThreadPoolExecutor(2) as pool:
        ia, ib = pool.map(lambda tk: stream(tk, keep=0, deadline=60), [ra["payload"]["ticket"], rb["payload"]["ticket"]])
    result["two_clients_one_terminal"] = {"a": ia["end"], "b": ib["end"], "both_exact": ia["sha256"] == ib["sha256"] == expected}
    shared = "download:" + str(uuid.uuid4())
    ra, rb = ctl.begin(str(path), ref=shared), other.begin(str(path), ref=shared)
    result["same_ref_from_two_clients"] = {"a": error_of(ra) or "ok", "b": error_of(rb) or "ok"}
    for r in (ra, rb):
        if "payload" in r:
            release_ticket(r["payload"]["ticket"])
    other.close()
    r = ctl.begin(str(path))
    during = {}

    def hook(frames, total, ws):
        if frames == 5 and not during:
            t0 = time.monotonic()
            during["write"] = error_of(ctl.block_call("write", {"data": "echo during\n"})) or "ok"
            during["call_ms"] = round((time.monotonic() - t0) * 1000)
    info = fetch(r["payload"]["ticket"], hook=hook, keep=0)
    result["control_calls_during_stream"] = {**during, "stream_exact": info["sha256"] == expected}
    return result


@scenario()
def lifecycle(ctx):
    """The block closing, the session being destroyed or detached while a download streams."""
    result = {}
    for what in ("block_closed", "session_killed", "detach"):
        ctl = Control(name=what)
        session, block = ctl.new_session(what)
        # a second block so closing one keeps the session alive in the block_closed case
        path = make_file(FX / f"life-{what}.bin", pattern(128 * MIB))
        r = ctl.begin(str(path))
        flags = {}

        def hook(frames, total, ws, ctl=ctl, session=session, block=block, what=what):
            if frames == 3 and not flags:
                if what == "block_closed":
                    flags["reply"] = error_of(ctl.call("block.close", {"session_id": session, "block_id": block})) or "ok"
                elif what == "session_killed":
                    flags["reply"] = error_of(ctl.call("session.destroy", {"session_id": session})) or "ok"
                else:
                    flags["reply"] = error_of(ctl.call("session.detach", {"session_id": session})) or "ok"
                time.sleep(.3)
            return "stop" if frames > 5000 else None
        info = fetch(r["payload"]["ticket"], hook=hook, cancel="close1000", keep=0, deadline=30)
        result[what] = {"action_reply": flags.get("reply"), "frames": info["frames"], "end": info.get("end"),
                        "stopped_by_hook": info["stopped_by_hook"], "bytes": info["bytes"]}
        try:
            ctl.close()
        except Exception:
            pass
    return result


@scenario()
def revisions(ctx):
    """What a connection negotiated below 13 gets from the method."""
    result = {}
    path = str(make_file(FX / "rev.txt", b"rev\n"))
    for rev in (12, 11, 10):
        ctl = Control(rev=rev, minimum=10, name=f"rev{rev}")
        ctl.session, ctl.block = ctx.ctl.session, ctx.ctl.block
        ctl.call("session.attach", {"session_id": ctx.ctl.session})
        reply = ctl.probe(path)
        result[f"api_{rev}_on_api13_server"] = {"negotiated": ctl.revision, "error": error_of(reply), "ok": "payload" in reply}
        ctl.close()
    if ctx.rex12:
        work = OUT / "server12"
        proc, sock, env = start_server(ctx.rex12, work, "api12")
        global SOCK
        saved = SOCK
        SOCK = sock
        try:
            ver = subprocess.run([ctx.rex12, "version"], capture_output=True, text=True).stdout
            result["api12_binary_version"] = ver
            ctl = Control(rev=13, minimum=0, name="to-api12")
            result["api12_negotiated"] = ctl.revision
            session, block = ctl.new_session("rev12")
            reply = ctl.probe(str(path))
            result["api12_server_download_begin"] = {"error": error_of(reply), "ok": "payload" in reply}
            listed = ctl.call("server.help") if False else None
            ctl.close()
            methods = subprocess.run([ctx.rex12, "--autostart=false", "--server", "unix://" + sock, "api", "list", "--json"],
                                     capture_output=True, text=True, env=env).stdout
            result["api12_lists_download_begin"] = "download_begin" in methods
            check("an API 12 server has no download_begin", not result["api12_lists_download_begin"])
        finally:
            SOCK = saved
            stop_server(proc)
    else:
        result["api12_binary"] = "not provided (--rex12); relying on revision gating"
    return result


@scenario(slow=True)
def server_shutdown_midstream(ctx):
    """A private server told to stop (SIGTERM) while a download streams. Uses its own server."""
    global SOCK, SERVER_PID
    work = OUT / "server-shutdown"
    proc, sock, env = start_server(ctx.rex, work)
    saved_sock, saved_pid = SOCK, SERVER_PID
    SOCK, SERVER_PID = sock, proc.pid
    try:
        ctl = Control(name="shutdown")
        ctl.new_session("shutdown")
        path = make_file(FX / "shutdown.bin", pattern(256 * MIB))
        r = ctl.begin(str(path))
        flags = {}

        def hook(frames, total, ws):
            if frames == 3 and not flags:
                flags["signalled"] = True
                proc.send_signal(signal.SIGTERM)
            return "stop" if frames > 40000 else None
        info = fetch(r["payload"]["ticket"], hook=hook, cancel="close1000", keep=0, deadline=30)
        return {"frames": info["frames"], "bytes": info["bytes"], "end": info.get("end"), "stopped_by_hook": info["stopped_by_hook"]}
    finally:
        SOCK, SERVER_PID = saved_sock, saved_pid
        stop_server(proc)


@scenario()
def docs(ctx):
    """What the server's own documents say about downloads and the bytes protocol (quoted verbatim)."""
    result = {}
    s = socket.socket(socket.AF_UNIX)
    s.settimeout(10)
    s.connect(SOCK)
    s.sendall(b"GET /llms.txt HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
    raw = b""
    while True:
        chunk = s.recv(1 << 16)
        if not chunk:
            break
        raw += chunk
    s.close()
    text = raw.split(b"\r\n\r\n", 1)[1].decode()
    (OUT / "llms.txt").write_text(text)
    lines = text.splitlines()
    hits = [i + 1 for i, line in enumerate(lines) if "download" in line.lower() or "rex.data.bytes" in line.lower()]
    result["llms_txt_lines_mentioning_download_or_bytes"] = hits
    result["llms_txt_sha256"] = hashlib.sha256(text.encode()).hexdigest()
    result["llms_txt_mentions_bytes_protocol_framing"] = any(
        "rex.data.bytes" in line and ("empty" in line.lower() or "end" in line.lower()) for line in lines)
    help_text = subprocess.run([ctx.rex, "--autostart=false", "--server", "unix://" + SOCK, "api", "describe", TERMINAL + ".download_begin"],
                               capture_output=True, text=True, env=ctx.env).stdout
    (OUT / "describe-download_begin.txt").write_text(help_text)
    ver = subprocess.run([ctx.rex, "version"], capture_output=True, text=True).stdout
    result["rex_version"] = ver
    return result


# ---------------------------------------------------------------- driver

class Ctx:
    pass


def main():
    global OUT, FX, SOCK, SERVER_PID, TRANSCRIPT
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--rex", default=shutil.which("rex"), help="rex binary for the private server (API 13)")
    parser.add_argument("--rex12", default=None, help="an API 12 rex binary, to see what it answers")
    parser.add_argument("--out", default=str(Path.home() / ".cache/agent-ws/rex-downloads/d0"))
    parser.add_argument("--quick", action="store_true", help="skip scenarios that wait for minutes")
    parser.add_argument("--only", default="", help="comma-separated scenario names")
    args = parser.parse_args()
    assert args.rex, "rex required (or pass --rex)"
    OUT = Path(args.out).resolve()
    if OUT.exists() and (OUT / "run").exists():
        shutil.rmtree(OUT / "run", ignore_errors=True)
    OUT.mkdir(parents=True, exist_ok=True)
    work = OUT / "run"
    work.mkdir(mode=0o700, exist_ok=True)
    FX = work / "fixtures"
    FX.mkdir(exist_ok=True)
    only = set(filter(None, args.only.split(",")))
    # A partial run (--only) adds to the evidence of earlier runs instead of replacing it.
    TRANSCRIPT = (OUT / "transcript.jsonl").open("a" if only else "w")
    if only and (OUT / "results.json").exists():
        RESULTS.update(json.loads((OUT / "results.json").read_text()))
    ctx = Ctx()
    ctx.rex, ctx.rex12 = args.rex, args.rex12
    proc, SOCK, ctx.env = start_server(args.rex, work / "server")
    SERVER_PID = proc.pid
    log("start", rex=args.rex, version=subprocess.run([args.rex, "version"], capture_output=True, text=True).stdout,
        pid=proc.pid, when=stamp(), out=str(OUT))
    print("private server pid", proc.pid, "socket", SOCK, "evidence", OUT, flush=True)
    status = None
    try:
        ctx.ctl = Control()
        status = ctx.ctl.call("server.status")["payload"]
        print("server API revision", status.get("api_revision"), flush=True)
        check("the private server speaks API revision 13", status.get("api_revision") == 13, status)
        failed = []
        for name, fn, slow in SCENARIOS:
            if (only and name not in only) or (slow and args.quick and not only):
                continue
            print(f"== {name}: {(fn.__doc__ or '').strip().splitlines()[0]}", flush=True)
            started = time.monotonic()
            # Each scenario gets its own terminal: reservations (16 per terminal) never leak across.
            ctx.ctl = Control()
            ctx.ctl.new_session(name)
            try:
                RESULTS[name] = fn(ctx)
            except Exception:
                RESULTS[name] = {"scenario_error": traceback.format_exc()}
                failed.append(name)
                print(RESULTS[name]["scenario_error"], flush=True)
            try:
                ctx.ctl.call("session.destroy", {"session_id": ctx.ctl.session})
                ctx.ctl.close()
            except Exception:
                pass
            log("scenario", name=name, seconds=round(time.monotonic() - started, 1))
            (OUT / "results.json").write_text(json.dumps(RESULTS, indent=1, default=repr))
    finally:
        stop_server(proc)
        previous = json.loads((OUT / "checks.json").read_text()) if only and (OUT / "checks.json").exists() else []
        (OUT / "checks.json").write_text(json.dumps(previous + CHECKS, indent=1))
        TRANSCRIPT.close()
    bad = [c for c in CHECKS if not c["ok"]]
    print(f"{len(CHECKS) - len(bad)} of {len(CHECKS)} expectations held; scenario errors: {failed or 'none'}")
    if bad or failed:
        for c in bad:
            print("  moved:", c["name"], "-", c["detail"][:300])
        sys.exit(1)


if __name__ == "__main__":
    main()
