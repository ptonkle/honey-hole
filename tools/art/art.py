#!/usr/bin/env python3
"""Draw the project's 2D art: every picture is TikZ text written by Python,
compiled by pdflatex and rendered by pdftocairo, and the PNGs are committed.
Rendering is a step a person runs on a machine with TeX; ./scripts/check.sh
runs only `--check-only`, which needs no TeX.

    python3 tools/art/art.py                 # every picture
    python3 tools/art/art.py title units     # by name or by folder
    python3 tools/art/art.py --check-only    # check the committed PNGs

Pictures are registered in a PICTURES dict, name -> function, in
tools/art/pictures.py and in any tools/art/art_*.py module. A function takes
a seeded random.Random and returns an artlib.Fig. The name "units/cat"
writes web/art/units/cat.png and keeps its TeX at tools/art/src/units/cat.tex.

Sizes, caps and transparency per kind come from data/art.json "sizes"; the
colours and ink widths from the style pack it names (style.py). Each
picture's random generator is seeded from its name, so a run draws the same
pictures every time.
"""
import importlib
import os
import random
import shutil
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import artlib  # noqa: E402
import style  # noqa: E402

ROOT = HERE.parents[1]
OUT = ROOT / "web" / "art"
SRC = HERE / "src"
EXTRA_BIN = ("/Library/TeX/texbin", "/opt/homebrew/bin", "/usr/local/bin")


def pictures():
    found = {}
    mods = ["pictures"] + sorted(p.stem for p in HERE.glob("art_*.py"))
    for name in mods:
        if not (HERE / f"{name}.py").exists():
            continue
        mod = importlib.import_module(name)
        for key, make in getattr(mod, "PICTURES", {}).items():
            if key in found:
                raise SystemExit(f"art: picture {key!r} is registered twice")
            found[key] = make
    return found


def selected(names, only):
    if not only:
        return list(names)
    return [n for n in names if only & {n, n.split("/")[0], n.split("/")[-1]}]


def build(name, make, cfg):
    """The Fig for one picture; Fig(kind) takes its size from data/art.json."""
    fig = make(random.Random(zlib.crc32(name.encode())))
    if not isinstance(fig, artlib.Fig):
        raise SystemExit(f"art: {name} did not return an artlib.Fig")
    spec = cfg["sizes"].get(fig.kind)
    if spec is None:
        raise SystemExit(f"art: {name}: kind {fig.kind!r} is not in data/art.json sizes")
    if fig.size is None:
        fig.size = tuple(spec["cm"])
    return fig, spec


def calm_for(fig, spec, sty):
    if not fig.calm:
        return None
    W, H = fig.size
    PW, PH = spec["px"]
    x0, y0, x1, y1 = fig.calm["box"]
    return {
        "box": (x0 / W * PW, (H - y1) / H * PH, x1 / W * PW, (H - y0) / H * PH),
        "colours": [sty.channels(r) for r in fig.calm["roles"]],
        "min_share": fig.calm["min_share"],
    }


def check_one(name, fig, spec, sty):
    return artlib.check(
        OUT / f"{name}.png",
        spec["px"],
        cutout=spec.get("cutout", False),
        max_bytes=spec.get("max_bytes"),
        clear_edges=spec.get("clear_edges", "tblr"),
        calm=calm_for(fig, spec, sty),
    )


def tool(name):
    path = os.environ.get("PATH", "") + os.pathsep + os.pathsep.join(EXTRA_BIN)
    return shutil.which(name, path=path)


def main(argv):
    check_only = "--check-only" in argv
    only = {a for a in argv if not a.startswith("--")}
    cfg = style.config(ROOT)
    artlib.SIZES.update({k: tuple(v["cm"]) for k, v in cfg["sizes"].items()})
    pics = pictures()
    names = selected(pics, only)
    if only and not names:
        raise SystemExit(f"art: no picture matches {' '.join(sorted(only))}")
    if not names:
        print("art: no pictures registered (tools/art/pictures.py); nothing to check")
        return 0
    try:
        sty = style.load(ROOT)
    except style.StyleError as e:
        raise SystemExit(str(e))

    if not check_only:
        pdflatex, pdftocairo = tool("pdflatex"), tool("pdftocairo")
        if not pdflatex:
            print("art: pdflatex not found; install TeX Live or MacTeX. Skipping is the check script's decision.", file=sys.stderr)
            return 3
        if not pdftocairo:
            print("art: pdftocairo not found; install Poppler. Skipping is the check script's decision.", file=sys.stderr)
            return 3

    failures = []
    for name in names:
        fig, spec = build(name, pics[name], cfg)
        if not check_only:
            try:
                tex = artlib.document(name, fig, sty.defines(), sty.ink_for(fig.kind), sty.roles())
            except ValueError as e:
                raise SystemExit(f"art: {e}")
            with tempfile.TemporaryDirectory() as tmp:
                t = Path(tmp)
                (t / "art.tex").write_text(tex)
                r = subprocess.run([pdflatex, "-interaction=nonstopmode", "-halt-on-error", "art.tex"],
                                   cwd=t, capture_output=True, text=True)
                if r.returncode != 0:
                    print(r.stdout[-3000:])
                    raise SystemExit(f"art: {name}: pdflatex failed")
                clear = ["-transp"] if spec.get("cutout") else []
                PW, PH = spec["px"]
                subprocess.run([pdftocairo, "-png", *clear, "-scale-to-x", str(PW), "-scale-to-y", str(PH),
                                "-singlefile", str(t / "art.pdf"), str(t / "art")], check=True)
                out = OUT / f"{name}.png"
                out.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy(t / "art.png", out)
                src = SRC / f"{name}.tex"
                src.parent.mkdir(parents=True, exist_ok=True)
                src.write_text(tex)
            print("wrote", out.relative_to(ROOT))
        failures += [f"{name}: {p}" for p in check_one(name, fig, spec, sty)]

    for line in failures:
        print(line, file=sys.stderr)
    if failures:
        return 1
    print(f"art: {len(names)} picture(s) pass")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
