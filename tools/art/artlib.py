"""Generic helpers for a Builder tool's 2D art: a picture is TikZ text that
pdflatex compiles and pdftocairo renders to PNG, never generated pixels.

This file knows no style. It names colours only by role ("ink", "sky",
"wallA" ...); the style pack (see style.py and data/art.json) says what each
role looks like. It holds no colour literal of any kind, and
tests/art_check.py fails if one appears.

What it enforces, and where:

- every filled shape is outlined in the role "ink": `Fig` has no method
  that fills without an outline, except `band`, a full-width strip behind
  everything (a sky), and `union`, whose outline is drawn as a wider ink
  pass under the fills;
- no words in a picture: `document` refuses TeX that could set text
  (FORBIDDEN);
- every role a picture uses is defined by the style pack: `document`
  refuses an unknown role;
- the rendered PNG has the agreed size, alpha and edges, stays under its
  byte cap, and keeps an optional calm region plain: `check`, which reads
  the PNG with `png_read` (stdlib only, no TeX needed).
"""
import math
import re
import struct
import zlib
from contextlib import contextmanager
from pathlib import Path

PT_PER_CM = 28.4528

# kind -> (width, height) in cm, filled from data/art.json "sizes" by
# tools/art/art.py before any picture is drawn, so Fig(kind) knows its size.
SIZES = {}

# TeX that can put a letter, digit or word on the page. A picture carries
# none; the page sets every word.
FORBIDDEN = ("\\node", "\\text", "\\put", "\\char", "\\pgftext", "label=", "\\verb", "\\symbol")
_NODE_OP = re.compile(r"(?<![A-Za-z\\])node\s*[\[{(]")
_ROLE = re.compile(r"^[A-Za-z][A-Za-z0-9]*$")


def f(x):
    """A number for TeX: three decimals at most, no trailing zeros."""
    return f"{x:.3f}".rstrip("0").rstrip(".") if abs(x) > 0.0005 else "0"


def roles_in(expr):
    """The role names in an xcolor expression such as "wallA!60!windark"."""
    names = [t for t in expr.split("!") if t and not t.replace(".", "").isdigit()]
    for n in names:
        if not _ROLE.match(n):
            raise ValueError(f"colour {expr!r}: {n!r} is not a role name (letters and digits only)")
    return names


class Fig:
    """One picture: a list of TikZ lines drawn in centimetres, origin at the
    bottom left, under an affine frame so a part can be drawn once about its
    own origin and placed, scaled, turned or mirrored anywhere. The ink
    width never scales with the frame.

    kind names a row of data/art.json "sizes" (scene, unit, icon ...); size
    is (width, height) in cm and, when left out, comes from that row.
    """

    def __init__(self, kind, size=None):
        self.kind = kind
        size = size or SIZES.get(kind)
        self.size = tuple(size) if size else None
        self.lines = []
        self.roles = {"ink"}
        self.T = (1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
        self._union = None
        self.calm = None

    # frames ------------------------------------------------------------
    def pt(self, x, y):
        a, b, c, d, e, g = self.T
        return (a * x + b * y + e, c * x + d * y + g)

    def k(self):
        """The frame's length scale (for radii)."""
        a, b, c, d, _, _ = self.T
        return math.sqrt(abs(a * d - b * c))

    @contextmanager
    def frame(self, x=0.0, y=0.0, s=1.0, rot=0.0, flip=False):
        """Draw the block about a new origin at (x, y), scaled by s, turned
        rot degrees anticlockwise, mirrored left to right if flip."""
        old = self.T
        r = math.radians(rot)
        sx = -s if flip else s
        la, lb, lc, ld = sx * math.cos(r), -s * math.sin(r), sx * math.sin(r), s * math.cos(r)
        a, b, c, d, e, g = old
        self.T = (a * la + b * lc, a * lb + b * ld, c * la + d * lc, c * lb + d * ld,
                  a * x + b * y + e, c * x + d * y + g)
        try:
            yield self
        finally:
            self.T = old

    def P(self, x, y):
        X, Y = self.pt(x, y)
        return f"({f(X)},{f(Y)})"

    def _use(self, fill):
        if fill is None:
            raise ValueError("a filled shape needs a fill role")
        self.roles.update(roles_in(fill))
        return fill

    # paths -------------------------------------------------------------
    def _poly_path(self, points):
        return " -- ".join(self.P(x, y) for x, y in points) + " -- cycle"

    def _blob_path(self, points, tension):
        return f"plot[smooth cycle, tension={f(tension)}] coordinates {{{' '.join(self.P(x, y) for x, y in points)}}}"

    def _ellipse_path(self, x, y, rx, ry, rot):
        # The frame's linear part sends the ellipse's axes to new axes; with a
        # uniform scale (all frame() makes) the result is still an ellipse.
        a, b, c, d, _, _ = self.T
        r = math.radians(rot)
        ux, uy = math.cos(r), math.sin(r)
        ang = math.degrees(math.atan2(c * ux + d * uy, a * ux + b * uy))
        k = self.k()
        X, Y = self.pt(x, y)
        turn = f"[rotate around={{{f(ang)}:({f(X)},{f(Y)})}}] " if abs(ang) > 1e-6 else ""
        return f"{turn}({f(X)},{f(Y)}) ellipse ({f(rx * k)} and {f(ry * k)})"

    def _shape(self, path, fill, extra=""):
        if self._union is not None:
            self._union.append(path)
            return
        self.lines.append(f"\\filldraw[inked,fill={self._use(fill)}{extra}] {path};")

    # filled shapes, each outlined in ink --------------------------------
    def poly(self, points, fill=None):
        self._shape(self._poly_path(points), fill)

    def rect(self, x0, y0, x1, y1, fill=None, r=0.0):
        """A rectangle (a polygon, so it turns with the frame); r rounds the
        corners, in cm."""
        extra = f",rounded corners={f(r * self.k())}cm" if r else ""
        path = self._poly_path([(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
        if self._union is not None:
            self._union.append(path)
        else:
            self.lines.append(f"\\filldraw[inked,fill={self._use(fill)}{extra}] {path};")

    def blob(self, points, fill=None, tension=0.7):
        """A smooth closed curve through the points."""
        self._shape(self._blob_path(points, tension), fill)

    def circle(self, x, y, r, fill=None):
        X, Y = self.pt(x, y)
        self._shape(f"({f(X)},{f(Y)}) circle ({f(r * self.k())})", fill)

    def ellipse(self, x, y, rx, ry, fill=None, rot=0.0):
        self._shape(self._ellipse_path(x, y, rx, ry, rot), fill)

    @contextmanager
    def union(self, fill):
        """Shapes drawn inside become one outlined shape: an ink pass twice
        the ink width under all of them, then the fills on top, so only the
        outside edge shows (clouds, bushes, crowds of puffs)."""
        if self._union is not None:
            raise ValueError("unions do not nest")
        fill = self._use(fill)
        self._union = []
        try:
            yield self
        finally:
            paths, self._union = self._union, None
        for p in paths:
            self.lines.append(f"\\filldraw[draw=ink,fill=ink,line width=2\\Ink,line join=round,line cap=round] {p};")
        for p in paths:
            self.lines.append(f"\\fill[{fill}] {p};")

    def band(self, y0, y1, fill):
        """A full-width strip from y0 to y1 in picture coordinates (the frame
        is ignored), drawn without an outline because its edges run off the
        picture or meet another band: skies only."""
        if self.size is None:
            raise ValueError("band needs the picture's size")
        W = self.size[0]
        self.lines.append(f"\\fill[{self._use(fill)}] (0,{f(y0)}) rectangle ({f(W)},{f(y1)});")

    # lines ---------------------------------------------------------------
    def stroke(self, points, width=None, role="ink", smooth=False, closed=False, tension=0.6):
        """An open line through the points, round caps and joins. width is
        in pt and defaults to the ink width."""
        self.roles.update(roles_in(role))
        lw = "\\Ink" if width is None else f"{f(width)}pt"
        style = f"draw={role},line width={lw},line cap=round,line join=round"
        if smooth:
            kind = "smooth cycle" if closed else "smooth"
            path = f"plot[{kind}, tension={f(tension)}] coordinates {{{' '.join(self.P(x, y) for x, y in points)}}}"
        else:
            path = " -- ".join(self.P(x, y) for x, y in points) + (" -- cycle" if closed else "")
        self.lines.append(f"\\draw[{style}] {path};")

    # the calm region -------------------------------------------------------
    def keep_calm(self, box, roles, min_share=0.95):
        """Ask `check` to confirm that the box (x0, y0, x1, y1 in cm, picture
        coordinates) is at least min_share made of these roles' colours, so
        the page can set text over it."""
        for r in roles:
            roles_in(r)
        self.calm = {"box": tuple(box), "roles": list(roles), "min_share": min_share}

    def body(self):
        return "\n".join(self.lines) + "\n"


def refuse_text(name, tex):
    """The no-words rule, checked on the TeX before it is compiled."""
    for bad in FORBIDDEN:
        if bad in tex:
            raise ValueError(f"{name}: {bad!r} found; pictures carry no words")
    if _NODE_OP.search(tex):
        raise ValueError(f"{name}: a TikZ node found; pictures carry no words")


def document(name, fig, colour_defs, ink_pt, known_roles):
    """The standalone TeX for one picture. colour_defs is the style pack's
    \\definecolor block; known_roles the roles it defines."""
    if fig.size is None:
        raise ValueError(f"{name}: no size")
    unknown = sorted(fig.roles - set(known_roles))
    if unknown:
        raise ValueError(f"{name}: roles not in the style pack: {', '.join(unknown)}")
    W, H = fig.size
    tex = (
        "\\documentclass[tikz,border=0pt]{standalone}\n"
        "\\usepackage{xcolor}\n"
        + colour_defs
        + f"\\newlength{{\\Ink}}\\setlength{{\\Ink}}{{{f(ink_pt)}pt}}\n"
        "\\tikzset{inked/.style={draw=ink,line width=\\Ink,line join=round,line cap=round}}\n"
        "\\begin{document}\n\\begin{tikzpicture}\n"
        f"\\useasboundingbox (0,0) rectangle ({f(W)},{f(H)});\n"
        f"\\clip (0,0) rectangle ({f(W)},{f(H)});\n"
        + fig.body()
        + "\\end{tikzpicture}\n\\end{document}\n"
    )
    refuse_text(name, tex)
    return tex


# ---- checks (stdlib only; no TeX) -------------------------------------------

def png_read(path):
    """(width, height, channels, rows) of an 8-bit PNG, rows as RGBA bytearrays.
    Reads grey, grey+alpha, RGB, RGBA and palette images."""
    data = Path(path).read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{path}: not a PNG")
    pos, idat, plte, trns = 8, b"", None, None
    w = h = depth = ctype = interlace = None
    while pos < len(data):
        n, = struct.unpack(">I", data[pos:pos + 4])
        tag = data[pos + 4:pos + 8]
        chunk = data[pos + 8:pos + 8 + n]
        if tag == b"IHDR":
            w, h, depth, ctype, _, _, interlace = struct.unpack(">IIBBBBB", chunk)
        elif tag == b"PLTE":
            plte = chunk
        elif tag == b"tRNS":
            trns = chunk
        elif tag == b"IDAT":
            idat += chunk
        elif tag == b"IEND":
            break
        pos += 12 + n
    if w is None:
        raise ValueError(f"{path}: no IHDR")
    if depth != 8 or interlace:
        raise ValueError(f"{path}: only 8-bit, non-interlaced PNGs are read")
    bpp = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}[ctype]
    raw = zlib.decompress(idat)
    stride = w * bpp
    rows, prev = [], bytearray(stride)
    for y in range(h):
        ft = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - bpp] if i >= bpp else 0
            b = prev[i]
            c = prev[i - bpp] if i >= bpp else 0
            if ft == 1:
                line[i] = (line[i] + a) & 255
            elif ft == 2:
                line[i] = (line[i] + b) & 255
            elif ft == 3:
                line[i] = (line[i] + (a + b) // 2) & 255
            elif ft == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        prev = line
        out = bytearray()
        for x in range(w):
            px = line[x * bpp:(x + 1) * bpp]
            if ctype == 0:
                out += bytes((px[0], px[0], px[0], 255))
            elif ctype == 4:
                out += bytes((px[0], px[0], px[0], px[1]))
            elif ctype == 2:
                out += px + b"\xff"
            elif ctype == 6:
                out += px
            else:
                i = px[0]
                alpha = trns[i] if trns and i < len(trns) else 255
                out += plte[i * 3:i * 3 + 3] + bytes((alpha,))
        rows.append(out)
    channels = {0: 1, 2: 3, 3: 3, 4: 2, 6: 4}[ctype] + (1 if ctype == 3 and trns else 0)
    return w, h, channels, rows


def check(path, px, cutout=False, max_bytes=None, clear_edges="tblr", calm=None):
    """The problems with one rendered picture, as a list of strings (empty
    when it passes).

    px: the exact (width, height) in pixels.
    cutout: True for a picture on a transparent background, which must have
      an alpha channel and fully transparent pixels along clear_edges (any of
      "t", "b", "l", "r"); False for an opaque picture, which must have no
      transparent pixel.
    max_bytes: the file size cap.
    calm: None, or {"box": (x0, y0, x1, y1) in pixels from the top left,
      "colours": [(r, g, b), ...], "min_share": 0.95, "tolerance": 24}; at
      least min_share of the box's pixels must lie within tolerance (per
      channel) of one of the reference colours.
    """
    problems = []
    p = Path(path)
    if not p.exists():
        return [f"{p}: missing; render it with tools/art/art.py"]
    if max_bytes is not None and p.stat().st_size > max_bytes:
        problems.append(f"{p.stat().st_size} bytes, over the cap of {max_bytes}")
    try:
        w, h, channels, rows = png_read(p)
    except (ValueError, KeyError, zlib.error) as e:
        return [f"unreadable PNG: {e}"]
    if (w, h) != tuple(px):
        problems.append(f"size {w}x{h}, wanted {px[0]}x{px[1]}")
        return problems
    alpha = [[row[x * 4 + 3] for x in range(w)] for row in rows]
    if cutout:
        if channels not in (2, 4):
            problems.append("a cutout without an alpha channel")
        edges = {"t": alpha[0], "b": alpha[-1], "l": [r[0] for r in alpha], "r": [r[-1] for r in alpha]}
        names = {"t": "top", "b": "bottom", "l": "left", "r": "right"}
        for e in clear_edges:
            if max(edges[e]) > 0:
                problems.append(f"something touches the {names[e]} edge")
        if not any(a for r in alpha for a in r):
            problems.append("the cutout is empty")
    else:
        if any(a < 255 for r in alpha for a in r):
            problems.append("an opaque picture has transparent pixels")
    if calm:
        x0, y0, x1, y1 = (int(round(v)) for v in calm["box"])
        tol = calm.get("tolerance", 24)
        refs = [tuple(c) for c in calm["colours"]]
        total = hit = 0
        for y in range(max(0, y0), min(h, y1)):
            row = rows[y]
            for x in range(max(0, x0), min(w, x1)):
                r, g, b = row[x * 4], row[x * 4 + 1], row[x * 4 + 2]
                total += 1
                if any(abs(r - R) <= tol and abs(g - G) <= tol and abs(b - B) <= tol for R, G, B in refs):
                    hit += 1
        share = hit / total if total else 1.0
        if share < calm.get("min_share", 0.95):
            problems.append(f"calm region is {share:.1%} reference colours, wanted at least {calm.get('min_share', 0.95):.0%}")
    return problems
