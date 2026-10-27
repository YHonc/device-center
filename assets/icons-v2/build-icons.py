"""Deterministic Pillow resizing only; original generated images are retained."""
from pathlib import Path
from PIL import Image, ImageFilter
from collections import deque

def components(mask, width, height):
    seen = bytearray(len(mask))
    groups = []
    for start, value in enumerate(mask):
        if not value or seen[start]:
            continue
        queue = deque([start]); seen[start] = 1; group = []
        while queue:
            index = queue.popleft(); group.append(index)
            x, y = index % width, index // width
            for neighbor in ((index-1 if x else -1), (index+1 if x+1<width else -1), (index-width if y else -1), (index+width if y+1<height else -1)):
                if neighbor >= 0 and mask[neighbor] and not seen[neighbor]:
                    seen[neighbor] = 1; queue.append(neighbor)
        groups.append(group)
    return sorted(groups, key=len, reverse=True)

ROOT = Path(__file__).resolve().parent
NATIVE = ROOT.parent.parent / "src-tauri" / "icons"
NATIVE.mkdir(exist_ok=True)
app = Image.open(ROOT / "app-original.png").convert("RGBA")
if app.width != app.height:
    raise ValueError("Application icon must be square")
app.resize((1024, 1024), Image.Resampling.LANCZOS).save(ROOT / "app-1024.png")
app.resize((256, 256), Image.Resampling.LANCZOS).save(
    NATIVE / "icon-v2.ico", sizes=[(n, n) for n in (16, 20, 24, 32, 48, 64, 128, 256)]
)
for size in (32, 128, 256):
    app.resize((size, size), Image.Resampling.LANCZOS).save(NATIVE / f"app-v2-{size}.png")
    if size == 128:
        app.resize((size, size), Image.Resampling.LANCZOS).save(ROOT / "app-128.png")
tray_source = ROOT / "tray-original.png"
if tray_source.exists():
    tray = Image.open(tray_source).convert("RGBA")
    if tray.width != tray.height:
        raise ValueError("Tray icon must be square")
    mask = bytearray(255 if a > 128 and min(r,g,b) > 140 else 0 for r,g,b,a in tray.getdata())
    major = components(mask, tray.width, tray.height)[:2]
    clean = bytearray(len(mask))
    for group in major:
        for index in group: clean[index] = 255
    for hole in components(bytearray(0 if v else 255 for v in clean), tray.width, tray.height):
        if len(hole) < 256:
            for index in hole: clean[index] = 255
    alpha = Image.frombytes("L", tray.size, bytes(clean))
    bbox = alpha.getbbox()
    if bbox is None: raise ValueError("No tray mark detected")
    crop = alpha.crop(bbox)
    side = max(crop.size)
    padded = Image.new("L", (int(side*1.12),int(side*1.12)), 0)
    padded.paste(crop, ((padded.width-crop.width)//2,(padded.height-crop.height)//2))
    tray = Image.new("RGBA", padded.size, (255,255,255,0)); tray.putalpha(padded)
    tray.save(ROOT / "tray-clean.png")
    for size in (16, 20, 24, 32):
        icon = tray.resize((size, size), Image.Resampling.LANCZOS)
        # A dark one-pixel outline keeps the same white mark visible on light taskbars.
        outline = Image.new("RGBA", icon.size, (12,18,28,0))
        outline.putalpha(icon.getchannel("A").filter(ImageFilter.MaxFilter(3)))
        outline.alpha_composite(icon)
        icon = outline
        icon.save(ROOT / f"tray-{size}.png")
        (ROOT / f"tray-{size}.rgba").write_bytes(icon.tobytes())
