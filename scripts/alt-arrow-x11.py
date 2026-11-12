#!/usr/bin/env python3
"""Private legacy/Kitty Alt+Arrow byte checks. Optional REX_PI_KEYS_JS checks Pi's parser."""
import importlib.util,json,os,shlex,subprocess,tempfile,time
from pathlib import Path
BASE=Path(__file__).resolve().parent
binary=os.environ.get("REX_BIN",str(BASE.parent/"target/debug/rex-rune"))
pi_keys=os.environ.get("REX_PI_KEYS_JS")
work=Path(tempfile.mkdtemp(prefix="rex-alt-arrow-x11-"));print("Private Alt+Arrow artifacts:",work,flush=True)
for key in list(os.environ):
    if key.startswith(("REX_","HERDR_")) or key in ("DISPLAY","WAYLAND_DISPLAY","DBUS_SESSION_BUS_ADDRESS"):del os.environ[key]
for key,folder in [("HOME","home"),("XDG_CONFIG_HOME","config"),("XDG_STATE_HOME","state"),("XDG_CACHE_HOME","cache"),("XDG_RUNTIME_DIR","runtime")]:
    path=work/folder;path.mkdir(mode=0o700);os.environ[key]=str(path)
endpoint="unix://"+str(work/"rex.sock")
os.environ.update(REX_SERVER=endpoint,REX_BIN=binary,REX_CAPTURE_OUT=str(work/"captures"))
spec=importlib.util.spec_from_file_location("accept",BASE/"rune-acceptance-x11.py")
accept=importlib.util.module_from_spec(spec);spec.loader.exec_module(accept)
def api(method,**params):
    return json.loads(subprocess.check_output(["rex","--autostart=false","--server",endpoint,"api","call",method,json.dumps(params)],text=True,timeout=10))
def wait(predicate,message):
    end=time.monotonic()+8
    while time.monotonic()<end:
        if predicate():return
        time.sleep(.04)
    raise AssertionError(message)
server=subprocess.Popen(["rex","server","run","--listen",endpoint,"--restore=false","--config","/dev/null","--state-dir",str(work/"server")],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
desk=None
try:
    wait(lambda:(work/"rex.sock").exists(),"private server did not start")
    sid=api("session.create",label="alt-keys",initial_windows=[{"window_label":"keys","layout":accept.SHELL}])["session_id"]
    state=api("session.view",session_id=sid);block=state["windows"][0]["focused_block_id"]
    config=work/"workspace.json";config.write_text(json.dumps({"hosts":[{"label":"private","endpoint":endpoint}]}))
    desk=accept.Desktop();app,window=desk.launch(sid,config,log=(work/"client.log").open("w"))
    for kitty in (False,True):
        name="kitty" if kitty else "legacy";ready=work/(name+"-ready");received=work/(name+"-bytes")
        program="""import os,sys,tty,termios,select,time
from pathlib import Path
old=termios.tcgetattr(0);tty.setraw(0)
try:
 if KITTY:sys.stdout.write(chr(27)+'[>1u');sys.stdout.flush()
 Path(READY).touch();data=b'';deadline=time.monotonic()+8
 while time.monotonic()<deadline:
  if select.select([0],[],[],.1)[0]:
   data+=os.read(0,4096);Path(RECEIVED).write_bytes(data)
   if all(bytes([27])+b'[1;3'+bytes([c]) in data for c in b'ABCD'):break
finally:
 if KITTY:sys.stdout.write(chr(27)+'[<u');sys.stdout.flush()
 termios.tcsetattr(0,termios.TCSANOW,old)
""".replace("KITTY",repr(kitty)).replace("READY",repr(str(ready))).replace("RECEIVED",repr(str(received)))
        api("block.event",session_id=sid,block_id=block,event={"type":"paste","payload":{"text":"python3 -c "+shlex.quote(program)}})
        desk.key("Return");wait(ready.exists,"key receiver did not start");time.sleep(.15)
        desk.key("alt+Up","alt+Down","alt+Left","alt+Right")
        expected=[b"\x1b[1;3"+bytes([c]) for c in b"ABDC"]
        wait(lambda:received.exists() and all(v in received.read_bytes() for v in expected),"Alt arrows did not reach the "+name+" terminal with their Alt modifier")
        data=received.read_bytes();assert data==b"".join(expected),(name,data)
        if pi_keys:
            javascript="const {matchesKey}=await import(process.argv[1]);for(const [data,key] of JSON.parse(process.argv[2])){if(!matchesKey(data,key))throw Error('Pi did not recognize '+key)}"
            subprocess.run(["bun","-e",javascript,pi_keys,json.dumps([[v.decode(),"alt+"+k] for v,k in zip(expected,["up","down","left","right"])])],check=True)
        print("PASS "+name+" Alt+Arrow bytes"+(" recognized by Pi" if pi_keys else ""),flush=True)
        time.sleep(.2)
finally:
    if desk:desk.close()
    server.terminate()
    try:server.wait(timeout=8)
    except subprocess.TimeoutExpired:server.kill();server.wait()
