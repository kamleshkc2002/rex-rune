from datetime import datetime
import hashlib
import html
import json
from pathlib import Path
from zoneinfo import ZoneInfo
import zipfile

from PIL import Image

root = Path(__file__).resolve().parent.parent
thumbs = root / "thumbnails"
thumbs.mkdir(exist_ok=True)
entries = []
for path in sorted((root / "screenshots").glob("*.png")):
    image = Image.open(path)
    width, height = image.size
    image.thumbnail((640, 360))
    thumb = thumbs / (path.stem + ".jpg")
    image.convert("RGB").save(thumb, quality=88)
    entries.append({
        "file": str(path.relative_to(root)),
        "thumbnail": str(thumb.relative_to(root)),
        "title": path.stem.replace("-", " "),
        "width": width,
        "height": height,
        "captured_at": datetime.fromtimestamp(path.stat().st_mtime, ZoneInfo("America/Los_Angeles")).isoformat(),
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
    })
(root / "manifest.json").write_text(json.dumps(entries, indent=2) + "\n")
cards = "\n".join(
    '<figure><a href="{file}"><img src="{thumbnail}" width="640" height="352" '
    'loading="lazy" alt="{title}"></a><figcaption>{title}</figcaption></figure>'.format(
        **{k: html.escape(str(v), quote=True) for k, v in entry.items()}
    )
    for entry in entries
)
(root / "index.html").write_text('''<!doctype html>
<html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Rex Linux — screenshot inventory</title>
<style>
body{margin:32px auto;padding:0 24px;max-width:1440px;background:#f3f4f6;color:#182334;font:16px/1.5 system-ui,sans-serif}
h1{font-size:30px;margin-bottom:4px}header{margin-bottom:28px}a{color:#245a93}main{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,400px),1fr));gap:24px}
figure{margin:0}img{display:block;width:100%;height:auto;border:1px solid #c6ced8;border-radius:6px}figcaption{padding-top:8px;font-size:14px}a:focus-visible{outline:3px solid #187ca6;outline-offset:4px}
</style><header><h1>Rex Linux — screenshot inventory</h1>
<p>50 captures · 2560 × 1410 · November 11, 2026 PST · Rex Linux 0.6.0</p>
<p><a href="README.md">Coverage and scope</a> · <a href="manifest.json">Capture manifest</a> · Click an image for the original.</p></header>
<main>''' + cards + "</main></html>\n")
archive = root.parent / (root.name + ".zip")
with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as bundle:
    for name in ["README.md", "index.html", "manifest.json"]:
        bundle.write(root / name, root.name + "/" + name)
    for folder in ["screenshots", "thumbnails"]:
        for path in sorted((root / folder).iterdir()):
            bundle.write(path, root.name + "/" + str(path.relative_to(root)))
with zipfile.ZipFile(archive) as bundle:
    assert bundle.testzip() is None
    assert len(bundle.namelist()) == 103
print(f"{len(entries)} originals indexed; archive verified: {archive} ({archive.stat().st_size:,} bytes)")
