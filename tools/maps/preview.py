#!/usr/bin/env python3
"""Top-down previews of generated maps, for the person's review.

Reads the JSON the core prints (`cargo run --release --example maps -- dump
seed...`) and draws each map as a PNG: a cartoony view (water by depth,
shallows, land by height, trees, reeds, islands, the spawn and dock, fish
by tier colour, sample buoys) and, with --radar, the green low-fidelity view
the Fish-Boy radar will draw. It decides nothing: every place and verdict is
the core's. PNGs are written with zlib, so no imaging library is needed.

    python3 tools/maps/preview.py OUT_DIR SEED...            cartoony views
    python3 tools/maps/preview.py --radar OUT_DIR SEED...    radar views
    python3 tools/maps/preview.py --json FILE OUT_DIR        from a saved dump
"""

import json
import struct
import subprocess
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PX = 4  # pixels per grid cell

TIER = {
    "Legendary": (255, 140, 26),
    "Rare": (155, 77, 255),
    "Uncommon": (61, 125, 255),
    "Common": (60, 194, 60),
}
TREE = {"Pine": (38, 84, 46), "Birch": (170, 205, 110), "Maple": (70, 128, 52)}

# a 5x7 font, enough for captions
FONT = {
    "A": "01110100011000111111100011000110001", "B": "11110100011000111110100011000111110",
    "C": "01110100011000010000100001000101110", "D": "11110100011000110001100011000111110",
    "E": "11111100001000011110100001000011111", "F": "11111100001000011110100001000010000",
    "G": "01110100011000010111100011000101111", "H": "10001100011000111111100011000110001",
    "I": "01110001000010000100001000010001110", "J": "00111000100001000010000101001001100",
    "K": "10001100101010011000101001001010001", "L": "10000100001000010000100001000011111",
    "M": "10001110111010110101100011000110001", "N": "10001100011100110101100111000110001",
    "O": "01110100011000110001100011000101110", "P": "11110100011000111110100001000010000",
    "Q": "01110100011000110001101011001001101", "R": "11110100011000111110101001001010001",
    "S": "01111100001000001110000010000111110", "T": "11111001000010000100001000010000100",
    "U": "10001100011000110001100011000101110", "V": "10001100011000110001100010101000100",
    "W": "10001100011000110101101011010101010", "X": "10001100010101000100010101000110001",
    "Y": "10001100010101000100001000010000100", "Z": "11111000010001000100010001000011111",
    "0": "01110100011001110101110011000101110", "1": "00100011000010000100001000010001110",
    "2": "01110100010000100010001000100011111", "3": "11111000100010000010000011000101110",
    "4": "00010001100101010010111110001000010", "5": "11111100001111000001000011000101110",
    "6": "00110010001000011110100011000101110", "7": "11111000010001000100010000100001000",
    "8": "01110100011000101110100011000101110", "9": "01110100011000101111000010001001100",
    " ": "0" * 35, ".": "00000000000000000000000000110001100", ",": "00000000000000000110001000100000000",
    ":": "00000011000110000000011000110000000",
    ";": "00000011000110000000011000010001000", "%": "11000110010001000100010001001100011",
    "-": "00000000000000011111000000000000000", "(": "00010001000100001000010000010000010",
    ")": "01000001000001000010000100010001000", "/": "00000000010001000100010001000000000",
    "²": "01100000100010001110000000000000000",
}


def png(path, w, h, pix):
    raw = b"".join(b"\x00" + bytes(pix[y * w * 3:(y + 1) * w * 3]) for y in range(h))

    def chunk(kind, data):
        c = kind + data
        return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)

    out = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
    out += chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")
    Path(path).write_bytes(out)


class Canvas:
    def __init__(self, w, h, bg=(0, 0, 0)):
        self.w, self.h = w, h
        self.pix = bytearray(bg * (w * h))

    def put(self, x, y, c, a=1.0):
        if 0 <= x < self.w and 0 <= y < self.h:
            i = (y * self.w + x) * 3
            if a >= 1:
                self.pix[i:i + 3] = bytes(c)
            else:
                for k in range(3):
                    self.pix[i + k] = int(self.pix[i + k] * (1 - a) + c[k] * a)

    def disc(self, cx, cy, r, c, a=1.0):
        for y in range(int(cy - r), int(cy + r) + 1):
            for x in range(int(cx - r), int(cx + r) + 1):
                if (x - cx) ** 2 + (y - cy) ** 2 <= r * r:
                    self.put(x, y, c, a)

    def ring(self, cx, cy, r, c, width=1.0):
        for y in range(int(cy - r - width), int(cy + r + width) + 1):
            for x in range(int(cx - r - width), int(cx + r + width) + 1):
                d = ((x - cx) ** 2 + (y - cy) ** 2) ** 0.5
                if abs(d - r) <= width / 2:
                    self.put(x, y, c)

    def line(self, x0, y0, x1, y1, c, width=1):
        steps = int(max(abs(x1 - x0), abs(y1 - y0))) + 1
        for s in range(steps + 1):
            t = s / steps
            self.disc(x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, width / 2, c)

    def tri(self, cx, cy, r, c):
        for y in range(int(cy - r), int(cy + r) + 1):
            half = (y - (cy - r)) / (2 * r) * r
            for x in range(int(cx - half), int(cx + half) + 1):
                self.put(x, y, c)

    def rect(self, x0, y0, x1, y1, c, a=1.0):
        for y in range(int(y0), int(y1)):
            for x in range(int(x0), int(x1)):
                self.put(x, y, c, a)

    def text(self, x, y, s, c, scale=2):
        for ch in s.upper():
            g = FONT.get(ch, FONT[" "])
            for k, bit in enumerate(g):
                if bit == "1":
                    self.rect(x + (k % 5) * scale, y + (k // 5) * scale,
                              x + (k % 5 + 1) * scale, y + (k // 5 + 1) * scale, c)
            x += 6 * scale
        return x


def mix(a, b, t):
    t = max(0.0, min(1.0, t))
    return tuple(int(a[k] + (b[k] - a[k]) * t) for k in range(3))


def sample(m, fx, fy):
    """Bilinear ground height at a fractional cell position (y north)."""
    n, g = m["n"], m["ground"]
    fx, fy = max(0.0, min(n - 1.001, fx)), max(0.0, min(n - 1.001, fy))
    x0, y0 = int(fx), int(fy)
    tx, ty = fx - x0, fy - y0
    a, b = g[y0 * n + x0], g[y0 * n + x0 + 1]
    c, d = g[(y0 + 1) * n + x0], g[(y0 + 1) * n + x0 + 1]
    return (a * (1 - tx) + b * tx) * (1 - ty) + (c * (1 - tx) + d * tx) * ty


def to_px(m, p):
    """Map millimetres to pixels (north up)."""
    s = PX / m["cell_mm"]
    return p["x"] * s, (m["n"] * m["cell_mm"] - p["y"]) * s


def ground_layer(m, palette):
    n = m["n"]
    size = n * PX
    cv = Canvas(size, size + 92, palette["panel"])
    heights = [[0.0] * size for _ in range(size)]
    for py in range(size):
        fy = (size - 1 - py + 0.5) / PX - 0.5
        for px in range(size):
            heights[py][px] = sample(m, (px + 0.5) / PX - 0.5, fy)
    rows = m["rows"]
    for py in range(size):
        for px in range(size):
            h = heights[py][px]
            cv.put(px, py, palette["colour"](h, rows[py // PX][px // PX], heights, px, py))
    return cv


def cartoon_colour(h, kind, heights, px, py):
    if h < 0:
        if h > -1800:
            return mix((132, 214, 222), (72, 170, 205), (-h) / 1800)
        return mix((60, 140, 196), (28, 74, 140), (-h - 1800) / 12000)
    # an edge pixel next to water is beach
    size = len(heights)
    near_water = any(
        0 <= px + dx < size and 0 <= py + dy < size and heights[py + dy][px + dx] < 0
        for dx, dy in ((3, 0), (-3, 0), (0, 3), (0, -3))
    )
    if near_water and h < 900:
        return (226, 206, 150)
    base = (122, 178, 84) if kind != "I" else (112, 170, 80)
    return mix(base, (84, 120, 60), h / 14000)


def cartoon(m, out):
    pal = {"panel": (245, 240, 225), "colour": cartoon_colour}
    cv = ground_layer(m, pal)
    for t in m["trees"]:
        x, y = to_px(m, t["at"])
        r = 2.2 * t["scale"] / 1000
        if t["kind"] == "Pine":
            cv.tri(x, y, r + 1.2, TREE["Pine"])
        else:
            cv.disc(x, y, r + 0.6, TREE[t["kind"]])
            cv.put(int(x - 1), int(y - 1), (235, 240, 200) if t["kind"] == "Birch" else (110, 165, 80))
    for patch in m["reeds"]:
        for c in patch["clumps"]:
            x, y = to_px(m, c)
            cv.line(x, y + 3, x, y - 4, (96, 120, 36), 1.4)
            cv.line(x - 1.5, y + 3, x - 3.5, y - 2, (160, 182, 64), 1.4)
            cv.line(x + 1.5, y + 3, x + 3.5, y - 2, (160, 182, 64), 1.4)
            cv.disc(x, y - 4, 1.3, (128, 82, 40))
    sp = m["spawn"]
    dx, dy = to_px(m, sp["dock"])
    sx, sy = to_px(m, sp["at"])
    cv.line(dx, dy, dx + (sx - dx) * 0.25, dy + (sy - dy) * 0.25, (140, 92, 50), 4)
    for f in m["fish"] or []:
        x, y = to_px(m, f["at"])
        cv.disc(x, y, 4.2, (255, 255, 255))
        cv.disc(x, y, 3, TIER[f["tier"]])
    for b in m["buoys"][1:]:
        if b["buoy"]:
            x, y = to_px(m, b["buoy"])
            cv.disc(x, y, 6, (255, 255, 255))
            cv.disc(x, y, 4.5, (225, 40, 40))
            cv.disc(x, y, 2, (255, 255, 255))
            for p in b["players"]:
                px, py = to_px(m, p)
                cv.ring(px, py, 7, (255, 255, 255), 2)
    heading(cv, m, sp, (255, 255, 255), (240, 70, 60))
    caption(cv, m, (40, 40, 40), (190, 40, 30))
    png(out, cv.w, cv.h, cv.pix)


def heading(cv, m, sp, fill, arrow):
    import math

    sx, sy = to_px(m, sp["at"])
    a = sp["heading"] / 65536 * 2 * math.pi
    ex, ey = sx + math.cos(a) * 26, sy - math.sin(a) * 26
    cv.disc(sx, sy, 8, fill)
    cv.disc(sx, sy, 5, arrow)
    cv.line(sx, sy, ex, ey, arrow, 3)
    for side in (2.5, -2.5):
        cv.line(ex, ey, ex - math.cos(a + side / 6) * 9, ey + math.sin(a + side / 6) * 9, arrow, 3)


def caption(cv, m, ink, bad):
    size = m["n"] * PX
    r = m["report"]
    x = cv.text(10, size + 8, f"SEED {m['seed']}", ink, 3)
    stats = (f"{r['water_m2'] // 1000}K M² WATER, CROSSING {r['span_mm'] // 1000} M "
             f"({m['crossing_s']} S AT TOP SPEED), {r['islands']} ISLANDS, COVER {r['cover'] / 10:.0f}%")
    cv.text(x + 20, size + 12, stats, ink, 1)
    if m["problems"]:
        words, lines = ("REJECTED: " + "; ".join(m["problems"])).split(" "), [""]
        for w in words:
            if len(lines[-1]) + len(w) + 1 > 62:
                lines.append("")
            lines[-1] = (lines[-1] + " " + w).strip()
        for k, line in enumerate(lines[:3]):
            cv.text(10, size + 34 + k * 18, line, bad, 2)
    else:
        cv.text(10, size + 38, "ACCEPTED", (30, 130, 50), 2)


def radar_colour(h, kind, heights, px, py):
    size = len(heights)
    # chunky: judge the 8-pixel block, outline land against water
    bx, by = px - px % 2, py - py % 2
    water = heights[by][bx] < 0
    edge = any(
        0 <= bx + dx < size and 0 <= by + dy < size and (heights[by + dy][bx + dx] < 0) != water
        for dx, dy in ((2, 0), (-2, 0), (0, 2), (0, -2))
    )
    scan = 0.82 if (py // 2) % 2 else 1.0
    if edge:
        c = (120, 255, 140)
    elif water:
        c = (8, 38, 16) if heights[by][bx] < -1800 else (14, 58, 24)
    else:
        c = (32, 104, 44)
    return tuple(int(v * scan) for v in c)


def radar(m, out):
    """The radar at low fidelity: the map judged in BLOCK-pixel cells, four
    flat greens, a blocky outline, rings and blips snapped to the cells."""
    import math

    n, cell = m["n"], m["cell_mm"]
    size = n * PX
    cells = size // BLOCK
    cv = Canvas(size, size + 92, (4, 18, 8))
    world = n * cell

    def water_at(cx, cy):
        # the block's centre, in map cells (y north)
        fx = (cx + 0.5) * BLOCK / PX - 0.5
        fy = (size - (cy + 0.5) * BLOCK) / PX - 0.5
        return sample(m, fx, fy) < 0

    grid = [[water_at(cx, cy) for cx in range(cells)] for cy in range(cells)]
    land, water, edge, dim = (24, 84, 34), (6, 30, 12), (120, 255, 140), (40, 130, 56)
    colours = {}
    for cy in range(cells):
        for cx in range(cells):
            w = grid[cy][cx]
            shore = any(
                0 <= cx + dx < cells and 0 <= cy + dy < cells and grid[cy + dy][cx + dx] != w
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))
            )
            colours[cx, cy] = edge if (shore and not w) else (water if w else land)

    sx, sy = to_px(m, m["spawn"]["at"])
    scx, scy = int(sx // BLOCK), int(sy // BLOCK)
    # ping rings every quarter of the map, snapped to cells, drawn on water only
    for r in (cells // 6, cells // 3, cells // 2):
        for cy in range(cells):
            for cx in range(cells):
                if abs(round(math.hypot(cx - scx, cy - scy)) - r) == 0 and colours[cx, cy] == water:
                    colours[cx, cy] = dim

    def blip(px, py, c):
        colours[int(px // BLOCK), int(py // BLOCK)] = c

    sp = m["spawn"]
    a = sp["heading"] / 65536 * 2 * math.pi
    blip(sx, sy, edge)
    blip(sx + math.cos(a) * BLOCK * 1.4, sy - math.sin(a) * BLOCK * 1.4, edge)
    b = m["buoys"][0]["buoy"]
    if b:
        bx, by = to_px(m, b)
        for dx, dy in ((0, -1), (0, 1), (-1, 0), (1, 0)):
            blip(bx + dx * BLOCK, by + dy * BLOCK, edge)

    for (cx, cy), c in colours.items():
        for y in range(cy * BLOCK, min(size, (cy + 1) * BLOCK)):
            shade = 0.72 if y == (cy + 1) * BLOCK - 1 else 1.0
            for x in range(cx * BLOCK, min(size, (cx + 1) * BLOCK)):
                cv.put(x, y, tuple(int(v * shade) for v in c))
    cv.text(10, size + 12, f"RADAR  SEED {m['seed']}", edge, 3)
    png(out, cv.w, cv.h, cv.pix)


# Radar cell size in pixels: the person asked for a lower-fidelity radar.
BLOCK = 12


def main(argv):
    draw = cartoon
    if argv and argv[0] == "--radar":
        draw, argv = radar, argv[1:]
        if argv and argv[0].startswith("--block="):
            global BLOCK
            BLOCK, argv = int(argv[0].split("=")[1]), argv[1:]
    if argv and argv[0] == "--json":
        maps = json.loads(Path(argv[1]).read_text())
        out = Path(argv[2])
    else:
        out, seeds = Path(argv[0]), argv[1:]
        cmd = ["cargo", "run", "--release", "-q", "--example", "maps", "--", "dump", *seeds]
        maps = json.loads(subprocess.run(cmd, cwd=ROOT, check=True, capture_output=True, text=True).stdout)
    out.mkdir(parents=True, exist_ok=True)
    for m in maps:
        tag = "radar" if draw is radar else ("accepted" if not m["problems"] else "rejected")
        path = out / f"{m['setting']}-{m['seed']:04d}-{tag}.png"
        draw(m, path)
        print(path)


if __name__ == "__main__":
    main(sys.argv[1:])
