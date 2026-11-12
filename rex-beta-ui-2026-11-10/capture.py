import json
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from zoneinfo import ZoneInfo

from PIL import Image

ROOT = Path(__file__).resolve().parent
SESSION = "run-1794383509331-50041-0"
APP = "Rex Beta"


def run(*args):
    p = subprocess.run(["agent-desktop", "--session", SESSION, *args], capture_output=True, text=True)
    try:
        data = json.loads(p.stdout)
    except ValueError:
        raise RuntimeError(p.stderr or p.stdout)
    with (ROOT / "evidence" / "commands.jsonl").open("a") as f:
        f.write(json.dumps({"time": datetime.now(ZoneInfo("America/Los_Angeles")).isoformat(), "args": args, "result": data}) + "\n")
    if not data.get("ok"):
        raise RuntimeError(json.dumps(data))
    return data["data"]


def walk(node, depth=0):
    yield node, depth
    for child in node.get("children", []):
        yield from walk(child, depth + 1)


def snap(surface="window", window=None, depth=None):
    args = ["snapshot", "--app", APP, "--surface", surface, "--compact", "--include-bounds", "--timeout-ms", "15000"]
    if window:
        args += ["--window-id", window]
    if depth is not None:
        args += ["--max-depth", str(depth)]
    return run(*args)


def show(data):
    print(json.dumps({k: v for k, v in data.items() if k != "tree"}))
    for n, d in walk(data["tree"]):
        label = n.get("name", "")
        value = n.get("value", "")
        if n.get("role") in ("treeitem", "cell", "scrollbar", "handle"):
            continue
        if n.get("role") == "statictext" and any(c.get("name") == label for c in n.get("children", [])):
            continue
        if label or value or n.get("ref_id"):
            print("  " * d + " ".join(str(v) for v in [n.get("ref_id", ""), n.get("role"), label, f"={value}" if value else "", str(n.get("states", [])) if n.get("states") else ""] if v))


def click(name, surface="window", role=None, window=None, occurrence=None):
    data = snap(surface, window, 2 if surface == "menubar" else None)
    matches = [n for n, _ in walk(data["tree"]) if n.get("name") == name and n.get("ref_id") and (not role or n.get("role") == role)]
    if occurrence is not None:
        matches = matches[occurrence:occurrence+1]
    if len(matches) != 1:
        show(data)
        raise RuntimeError(f"Expected one {name}, got {len(matches)}")
    out = run("click", matches[0]["ref_id"])
    print(json.dumps(out))
    return out


def shot(slug, surface="window", window=None, quiet=False):
    data = snap(surface, window)
    path = ROOT / "screenshots" / (slug + ".png")
    if surface in ("menu", "menu-bar-extras"):
        raw = ROOT / "evidence" / (slug + "-display.png")
        run("screenshot", "--screen", "0", str(raw))
        boxes = [n["bounds"] for n, _ in walk(data["tree"]) if n.get("role") == "menu" and n.get("bounds", {}).get("width", 0) > 0 and n.get("bounds", {}).get("height", 0) > 0]
        if not boxes:
            raise RuntimeError("No menu bounds; inspect screen capture")
        image = Image.open(raw)
        x1 = max(0, int(min(b["x"] for b in boxes)) - 8)
        y1 = max(0, int(min(b["y"] for b in boxes)) - 8)
        x2 = min(image.width, int(max(b["x"] + b["width"] for b in boxes)) + 8)
        y2 = min(image.height, int(max(b["y"] + b["height"] for b in boxes)) + 8)
        image.crop((x1, y1, x2, y2)).save(path)
    else:
        args = ["screenshot", "--app", APP]
        if window:
            args += ["--window-id", window]
        run(*args, str(path))
    (ROOT / "evidence" / (slug + ".json")).write_text(json.dumps(data, indent=2))
    with (ROOT / "captures.jsonl").open("a") as f:
        f.write(json.dumps({"file": str(path.relative_to(ROOT)), "surface": surface, "window": data.get("window"), "captured_at": datetime.now(ZoneInfo("America/Los_Angeles")).isoformat()}) + "\n")
    print(str(path))
    if not quiet:
        show(data)


def scroll_to(name, role=None, surface="window", window="w-669"):
    data = snap(surface, window)
    matches = [n for n, _ in walk(data["tree"]) if n.get("name") == name and n.get("ref_id") and (role is None or n.get("role") == role)]
    if len(matches) != 1:
        raise RuntimeError(f"Expected one scroll target {name}, got {len(matches)}")
    return run("scroll-to", matches[0]["ref_id"])


def dropdown(name, slug, window="w-669"):
    click(name, window=window)
    run("wait", "--menu", "--app", APP, "--timeout", "5000")
    shot(slug, "menu")
    data = snap("menu")
    selected = [n for n, _ in walk(data["tree"]) if n.get("role") == "menuitem" and "selected" in n.get("states", [])]
    if len(selected) != 1:
        raise RuntimeError("Inspect menu before dismissal: no unique selected item")
    run("click", selected[0]["ref_id"])
    try:
        run("wait", "--menu-closed", "--app", APP, "--timeout", "5000")
    except RuntimeError as exc:
        if "surface_read" not in str(exc):
            raise
        surfaces = run("list-surfaces", "--app", APP).get("surfaces", [])
        if any("menu" in s.get("type", "") for s in surfaces):
            raise


if __name__ == "__main__":
    action, *args = sys.argv[1:]
    if action == "snap":
        show(snap(*args))
    elif action == "click":
        click(*args)
    elif action == "shot":
        shot(*args)
    elif action == "topmenu":
        click(args[0], "menubar", "menuitem")
        show(snap("menu"))
    elif action == "raw":
        print(json.dumps(run(*args)))
