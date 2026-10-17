#!/usr/bin/env python3
"""Checks on the art pipeline that need no TeX: plain python3, stdlib only.

    python3 tests/art_check.py

- artlib.check accepts a good PNG and refuses each kind of bad one (wrong
  size, a cutout touching an edge or without alpha, an opaque picture with
  transparency, a file over its cap, a busy calm region, a missing file).
  The PNGs are written by this file, so no fixture is committed.
- Fig outlines every filled shape in ink; document refuses text-setting TeX
  and roles the style pack does not define.
- style.py reads a pack and refuses one without "ink".
- art.py --check-only passes with no pictures, fails on a missing PNG and
  passes once the PNG is right.
- artlib.py, art.py and style.py hold no colour literal.

Exits non-zero on the first failure.
"""
import json
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ART = ROOT / "tools" / "art"
sys.path.insert(0, str(ART))
import artlib  # noqa: E402
import style  # noqa: E402

FAILS = []


def ok(cond, what):
    print(("ok   " if cond else "FAIL ") + what)
    if not cond:
        FAILS.append(what)


def png_write(path, w, h, pixel, alpha=True):
    """A PNG whose pixel(x, y) gives (r, g, b, a); RGB when alpha is False."""
    raw = bytearray()
    for y in range(h):
        raw.append(0)
        for x in range(w):
            r, g, b, a = pixel(x, y)
            raw += bytes((r, g, b, a) if alpha else (r, g, b))

    def chunk(tag, data):
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    ihdr = struct.pack(">IIBBBBB", w, h, 8, 6 if alpha else 2, 0, 0, 0)
    Path(path).write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(bytes(raw))) + chunk(b"IEND", b""))


# Test colours, made up for this file only.
A = (200, 100, 50)
B = (20, 40, 60)


def cutout_px(x, y, w=16, h=16):
    inside = 3 <= x < w - 3 and 3 <= y < h - 3
    return (*A, 255) if inside else (0, 0, 0, 0)


def test_png_and_check(t):
    good_cut = t / "cut.png"
    png_write(good_cut, 16, 16, cutout_px)
    w, h, ch, rows = artlib.png_read(good_cut)
    ok((w, h, ch) == (16, 16, 4) and tuple(rows[8][32:36]) == (*A, 255), "png_read reads an RGBA PNG")
    ok(artlib.check(good_cut, (16, 16), cutout=True) == [], "a good cutout passes")
    ok(any("size" in p for p in artlib.check(good_cut, (16, 17), cutout=True)), "a wrong size is refused")

    touch = t / "touch.png"
    png_write(touch, 16, 16, lambda x, y: (*A, 255) if x >= 3 and 3 <= y < 13 else (0, 0, 0, 0))
    ok(any("right edge" in p for p in artlib.check(touch, (16, 16), cutout=True)), "a cutout touching an edge is refused")
    ok(artlib.check(touch, (16, 16), cutout=True, clear_edges="tbl") == [], "an edge left out of clear_edges may be touched")

    opaque = t / "opaque.png"
    png_write(opaque, 16, 16, lambda x, y: (*A, 255), alpha=False)
    w, h, ch, rows = artlib.png_read(opaque)
    ok(ch == 3 and tuple(rows[0][0:4]) == (*A, 255), "png_read reads an RGB PNG")
    ok(artlib.check(opaque, (16, 16)) == [], "a good opaque picture passes")
    ok(any("alpha" in p for p in artlib.check(opaque, (16, 16), cutout=True)), "a cutout without alpha is refused")
    ok(any("transparent" in p for p in artlib.check(good_cut, (16, 16), cutout=False)), "an opaque picture with transparency is refused")
    ok(any("cap" in p for p in artlib.check(opaque, (16, 16), max_bytes=10)), "a file over its cap is refused")
    ok(any("missing" in p for p in artlib.check(t / "nope.png", (16, 16))), "a missing PNG is refused")

    calm_img = t / "calm.png"
    png_write(calm_img, 16, 16, lambda x, y: (*B, 255) if x >= 8 else (*A, 255), alpha=False)
    calm = {"box": (0, 0, 8, 16), "colours": [A], "min_share": 0.95}
    ok(artlib.check(calm_img, (16, 16), calm=calm) == [], "a calm region in the reference colour passes")
    busy = dict(calm, box=(0, 0, 12, 16))
    ok(any("calm" in p for p in artlib.check(calm_img, (16, 16), calm=busy)), "a busy calm region is refused")


def test_fig():
    fig = artlib.Fig("icon", (2, 2))
    fig.poly([(0, 0), (1, 0), (0, 1)], "roleA")
    fig.rect(0, 0, 1, 1, "roleA", r=0.1)
    fig.blob([(0, 0), (1, 0), (1, 1)], "roleA")
    fig.circle(1, 1, 0.5, "roleA")
    with fig.frame(1, 1, s=0.5, rot=30, flip=True):
        fig.ellipse(0, 0, 1, 0.5, "roleA!50!roleB", rot=10)
    fills = [l for l in fig.lines if l.startswith("\\filldraw") or l.startswith("\\fill[")]
    ok(len(fills) == 5 and all(l.startswith("\\filldraw[inked,") for l in fills), "every shape fills with the inked outline")
    fig2 = artlib.Fig("icon", (2, 2))
    with fig2.union("roleA"):
        fig2.circle(0.5, 0.5, 0.4)
        fig2.circle(1.0, 0.6, 0.4)
    ok(fig2.lines[0].startswith("\\filldraw[draw=ink,fill=ink") and fig2.lines[-1].startswith("\\fill[roleA]"),
       "a union draws its ink pass under its fills")
    ok(fig.roles == {"ink", "roleA", "roleB"}, "Fig records the roles it uses, mixes included")

    roles = {"ink", "roleA", "roleB"}
    tex = artlib.document("t", fig, "", 1.5, roles)
    ok("\\setlength{\\Ink}{1.5pt}" in tex and "\\useasboundingbox (0,0) rectangle (2,2)" in tex, "document sets ink width and size")
    try:
        artlib.document("t", fig, "", 1.5, {"ink", "roleA"})
        ok(False, "an unknown role is refused")
    except ValueError:
        ok(True, "an unknown role is refused")
    for bad in list(artlib.FORBIDDEN) + ["-- node[left] {x}", "node {x}"]:
        f3 = artlib.Fig("icon", (2, 2))
        f3.lines.append(bad)
        try:
            artlib.document("t", f3, "", 1.5, roles)
            ok(False, f"text-setting TeX {bad!r} is refused")
        except ValueError:
            ok(True, f"text-setting TeX {bad!r} is refused")


def test_style(t):
    pack = t / "pack"
    pack.mkdir()
    (pack / "palette.json").write_text(json.dumps({"_about": "test", "ink": "#102030", "roleA": "a0b0c0"}))
    (pack / "ink.json").write_text(json.dumps({"default": 1.0, "icon": 2.0}))
    s = style.Style(pack)
    ok(s.channels("ink") == (16, 32, 48) and s.ink_for("icon") == 2.0 and s.ink_for("scene") == 1.0, "style reads palette roles and ink widths")
    ok("\\definecolor{roleA}{HTML}{A0B0C0}" in s.defines(), "style emits \\definecolor lines")
    (pack / "palette.json").write_text(json.dumps({"roleA": "#a0b0c0"}))
    try:
        style.Style(pack)
        ok(False, "a pack without ink is refused")
    except style.StyleError:
        ok(True, "a pack without ink is refused")
    return pack


def test_driver(t, pack):
    proj = t / "proj"
    shutil.copytree(ART, proj / "tools" / "art", ignore=shutil.ignore_patterns("__pycache__", "src"))
    (proj / "data").mkdir()
    cfg = json.loads((ROOT / "data" / "art.json").read_text())
    cfg["style"] = str(pack)
    (proj / "data" / "art.json").write_text(json.dumps(cfg))
    (pack / "palette.json").write_text(json.dumps({"ink": "#102030", "roleA": "#a0b0c0"}))
    cmd = [sys.executable, str(proj / "tools" / "art" / "art.py"), "--check-only"]
    r = subprocess.run(cmd, capture_output=True, text=True)
    ok(r.returncode == 0, "art.py --check-only passes with no pictures")
    (proj / "tools" / "art" / "pictures.py").write_text(
        "from artlib import Fig\n"
        "def dot(rng):\n    fig = Fig('icon')\n    fig.circle(1, 1, 0.5, 'roleA')\n    return fig\n"
        "PICTURES = {'icons/dot': dot}\n")
    r = subprocess.run(cmd, capture_output=True, text=True)
    ok(r.returncode == 1 and "missing" in r.stderr, "art.py --check-only fails on a missing PNG")
    W, H = cfg["sizes"]["icon"]["px"]
    out = proj / "web" / "art" / "icons"
    out.mkdir(parents=True)
    png_write(out / "dot.png", W, H, lambda x, y: cutout_px(x, y, W, H))
    r = subprocess.run(cmd, capture_output=True, text=True)
    ok(r.returncode == 0, "art.py --check-only passes once the PNG is right")


def test_no_colour_literal():
    hex_re = re.compile(r"#[0-9A-Fa-f]{3,8}\b|(?<![0-9A-Za-z_])(?=[0-9A-Fa-f]*\d)[0-9A-Fa-f]{6}(?![0-9A-Za-z_])|\b[0-9A-F]{6}\b")
    model_re = re.compile(r"rgb\s*\(|\{rgb\}|\{RGB\}|\{cmyk\}|\{gray\}", re.I)
    planted = 'fill="#12AB9F" x = "ABC123" y = rgb(1, 2, 3)'
    ok(len(hex_re.findall(planted)) == 2 and model_re.search(planted), "the colour-literal check finds planted literals")
    for name in ("artlib.py", "art.py", "style.py"):
        text = (ART / name).read_text()
        hits = [m.group(0) for m in hex_re.finditer(text)] + [m.group(0) for m in model_re.finditer(text)]
        ok(not hits, f"{name} holds no colour literal" + (f" (found {hits})" if hits else ""))


def main():
    with tempfile.TemporaryDirectory() as tmp:
        t = Path(tmp)
        test_png_and_check(t)
        test_fig()
        pack = test_style(t)
        test_driver(t, pack)
    test_no_colour_literal()
    if FAILS:
        print(f"\nart check: {len(FAILS)} failure(s)")
        return 1
    print("\nart check: all passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
