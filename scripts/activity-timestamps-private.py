#!/usr/bin/env python3
"""Observe OSC 7501 updated_at on a private Rex server (Python websockets required).

Artifacts under ~/.cache/agent-ws/rex-m32-elapsed; private HOME and socket, no UI.
Stops the server by PID. Reports query records and matching live events verbatim.
"""
import asyncio
import json
import os
from pathlib import Path
import subprocess
import tempfile

from websockets.asyncio.client import unix_connect

BASE = Path.home() / ".cache/agent-ws/rex-m32-elapsed"
BASE.mkdir(parents=True, exist_ok=True)
work = Path(tempfile.mkdtemp(prefix="timestamps-", dir=BASE))


async def observe():
    socket = work / "rex.sock"
    env = dict(os.environ, HOME=str(work), XDG_CONFIG_HOME=str(work / "config"),
               XDG_STATE_HOME=str(work / "state"), XDG_CACHE_HOME=str(work / "cache"))
    server = subprocess.Popen(["rex", "server", "run", "--listen", f"unix://{socket}",
                               "--restore=false", "--config", "/dev/null", "--state-dir", str(work / "rex-state")],
                              env=env, stdout=(work / "server.log").open("w"), stderr=subprocess.STDOUT)
    transcript = []
    try:
        for _ in range(100):
            if socket.exists():
                break
            await asyncio.sleep(.05)
        async with unix_connect(str(socket), uri="ws://localhost/api/control", subprotocols=["rex.control.v1"]) as ws:
            await ws.send(json.dumps({"type":"hello", "api_revision":13, "api_min_revision":13,
                                      "client":{"kind":"cli", "name":"elapsed-observation", "version":"0"}}))
            transcript.append(json.loads(await ws.recv()))
            serial = 0

            async def call(method, payload):
                nonlocal serial
                serial += 1
                ident = str(serial)
                await ws.send(json.dumps(dict(type="request", id=ident, method=method, payload=payload)))
                while True:
                    message = json.loads(await asyncio.wait_for(ws.recv(), 10))
                    transcript.append(message)
                    if message.get("id") == ident and message.get("type") == "response":
                        assert "error" not in message, message
                        return message["payload"]

            result = await call("session.create", {"label":"Timestamp observation", "initial_windows":[
                {"layout":{"block":{"flavor":"com.superlogical.terminal.shell", "options":{"shell":"/bin/sh"}}}}]})
            sid = result["session_id"]
            block = result["initial_windows"][0]["block_ids"][0]
            target = dict(session_id=sid, block_id=block)
            await call("session.attach", dict(session_id=sid))
            reporter = work / "report.py"
            reporter.write_text("import pathlib,time\np=pathlib.Path(" + repr(str(work / "report")) + ")\nold=''\nwhile True:\n text=p.read_text() if p.exists() else ''\n if text != old:\n  old=text; print(text,end='',flush=True)\n time.sleep(.02)\n")
            await asyncio.sleep(.5)
            await call("block.event", dict(**target, event={"type":"paste", "payload":{"text":f"/usr/bin/python3 {reporter}"}}))
            await call("block.event", dict(**target, event={"type":"key", "payload":{"key":"enter", "action":"press", "mods":""}}))
            observations = []
            for fields in ["state=working:progress=1", "state=working:progress=2",
                           "state=working:progress=2:msg=c2Vjb25k",
                           "state=blocked:kind=permission", "state=blocked:kind=permission:msg=YWdhaW4=",
                           "state=error", "state=done", "state=idle"]:
                (work / "report").write_text(f"\x1b]7501;{fields}:app=elapsed-test\x07")
                await asyncio.sleep(1.1)
                reply = await call("com.superlogical.terminal.program_status", dict(**target, args={}))
                assert len(reply["records"]) == 1, reply
                record = reply["records"][0]
                assert record["state"] == fields.split(":")[0].split("=")[1], record
                assert record.get("updated_at"), record
                events = [m.get("payload", {}).get("payload", {}).get("event", {}).get("payload", {}) for m in transcript if m.get("type") == "event"]
                assert any(e.get("name") == "program_status_changed" and e["payload"]["record"] == record for e in events), record
                observations.append(dict(report=fields, reply=reply))
            stamps = [o["reply"]["records"][0]["updated_at"] for o in observations]
            assert len(set(stamps)) == len(stamps), stamps
            (work / "version.txt").write_text(subprocess.check_output(["rex", "version"], text=True))
            (work / "observations.json").write_text(json.dumps(observations, indent=2))
            print(json.dumps(observations, indent=2))
    finally:
        (work / "wire.json").write_text(json.dumps(transcript, indent=2))
        server.terminate()
        try:
            server.wait(timeout=5)
        except subprocess.TimeoutExpired:
            server.kill()
            server.wait()
        print("Artifacts:", work)


asyncio.run(observe())
