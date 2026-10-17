"""Load the style pack a project's art is drawn in.

data/art.json names it:

    {"style": "default"}        the pack in styles/default: <repo>/styles/default
                                when present, else the nearest styles/default
                                found by walking up from this repository
    {"style": "path/to/pack"}   any other value is a directory, relative to the
                                repository root unless absolute

A pack is a directory holding:

    palette.json   role name -> colour, plus "_about" (keys starting with "_"
                   are notes); the role "ink" is required
    ink.json       picture kind -> ink width in pt, plus "default"
    STYLE.md       the look, as rules for whoever draws (optional here)
    motifs.py      helpers that draw with role names (optional)

This file holds no colour; it copies the pack's values into \\definecolor
lines.
"""
import importlib.util
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
_HEX = re.compile(r"^#?([0-9A-Fa-f]{6})$")
_ROLE = re.compile(r"^[A-Za-z][A-Za-z0-9]*$")


class StyleError(Exception):
    pass


def config(root=ROOT):
    path = Path(root) / "data" / "art.json"
    if not path.exists():
        raise StyleError(f"{path} is missing")
    return json.loads(path.read_text())


def find_pack(name, root=ROOT):
    """The directory of the named pack."""
    root = Path(root).resolve()
    if name == "default":
        for base in (root, *root.parents):
            cand = base / "styles" / "default"
            if (cand / "palette.json").exists():
                return cand
        raise StyleError("art: style pack \"default\" not found (no styles/default above "
                         f"{root}); copy one in or name a pack directory in data/art.json")
    pack = Path(name)
    if not pack.is_absolute():
        pack = root / pack
    if not (pack / "palette.json").exists():
        raise StyleError(f"art: style pack {pack} has no palette.json")
    return pack


class Style:
    def __init__(self, pack):
        self.dir = Path(pack)
        raw = json.loads((self.dir / "palette.json").read_text())
        self.palette = {}
        for role, value in raw.items():
            if role.startswith("_"):
                continue
            m = _HEX.match(str(value))
            if not _ROLE.match(role) or not m:
                raise StyleError(f"{self.dir}/palette.json: {role!r}: want a role of letters and digits and a six-digit hex colour")
            self.palette[role] = m.group(1).upper()
        if "ink" not in self.palette:
            raise StyleError(f"{self.dir}/palette.json: the role \"ink\" is required")
        ink = json.loads((self.dir / "ink.json").read_text())
        self.ink = {k: float(v) for k, v in ink.items() if not k.startswith("_")}
        if "default" not in self.ink:
            raise StyleError(f"{self.dir}/ink.json: a \"default\" width is required")

    def roles(self):
        return set(self.palette)

    def ink_for(self, kind):
        return self.ink.get(kind, self.ink["default"])

    def channels(self, role):
        v = self.palette[role]
        return tuple(int(v[i:i + 2], 16) for i in (0, 2, 4))

    def defines(self):
        """The \\definecolor lines for every role."""
        return "".join(f"\\definecolor{{{k}}}{{HTML}}{{{v}}}\n" for k, v in sorted(self.palette.items()))

    def ink_table(self):
        """The ink widths, one per line, as a TeX comment."""
        return "".join(f"% ink {k}: {v:g}pt\n" for k, v in sorted(self.ink.items()))

    def motifs(self):
        """The pack's motifs.py as a module, or None."""
        path = self.dir / "motifs.py"
        if not path.exists():
            return None
        spec = importlib.util.spec_from_file_location(f"style_motifs_{abs(hash(str(path)))}", path)
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        return mod


_cache = {}


def load(root=ROOT):
    """The style pack named in <root>/data/art.json."""
    key = str(Path(root).resolve())
    if key not in _cache:
        _cache[key] = Style(find_pack(config(root).get("style", "default"), root))
    return _cache[key]


if __name__ == "__main__":
    s = load()
    print(f"% style pack: {s.dir}")
    print(s.defines() + s.ink_table(), end="")
