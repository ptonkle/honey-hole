#!/usr/bin/env python3
"""Report what the built WebAssembly module can reach.

A WebAssembly module reaches the page only through its imports. This script
lists those imports, then follows each one into the JavaScript glue that
wasm-bindgen generated (and into any snippet files the glue loads), and
reports every network-capable API it finds there.

    python3 scripts/wasm_imports.py dist

It reads the module's import section itself (about forty lines below, in
`read_imports`), so it needs nothing but Python. If `wasm-tools` is on PATH it
also asks `wasm-tools print` for the import list and fails if the two
disagree, so the hand-written reader is checked against a standard tool.

What a pass establishes: the module's imports, as built, lead only to the
glue bodies printed below, and none of those bodies or snippets names a
network API. The one `fetch` the glue is allowed is the call that loads the
module itself. What it does not establish: anything about the page's own
JavaScript (the browser check and the boundary tests watch that) or about
behaviour a user has to start.

Exit status: 0 if nothing was flagged, 1 if something was, 2 on a usage error.
"""

import pathlib
import re
import shutil
import subprocess
import sys

# Names that can send data off the page. Matched as whole words in the code of
# each import body and snippet, after comments and string contents are removed.
NETWORK_APIS = [
    "fetch",
    "XMLHttpRequest",
    "WebSocket",
    "EventSource",
    "sendBeacon",
    "RTCPeerConnection",
    "Worker",
    "SharedWorker",
    "importScripts",
    "serviceWorker",
    "open",  # window.open: a popup is a request too
]

KIND = {0: "func", 1: "table", 2: "memory", 3: "global", 4: "tag"}


def leb_u32(data, i):
    result = shift = 0
    while True:
        byte = data[i]
        i += 1
        result |= (byte & 0x7F) << shift
        if byte < 0x80:
            return result, i
        shift += 7


def name(data, i):
    n, i = leb_u32(data, i)
    return data[i : i + n].decode("utf-8"), i + n


def limits(data, i):
    flag = data[i]
    i += 1
    _, i = leb_u32(data, i)
    if flag & 1:
        _, i = leb_u32(data, i)
    return i


def read_imports(wasm):
    """(module, field, kind) for every entry in the import section."""
    data = wasm.read_bytes()
    if data[:4] != b"\0asm":
        raise ValueError(f"{wasm} is not a WebAssembly module")
    i, out = 8, []
    while i < len(data):
        section, i = data[i], i + 1
        size, i = leb_u32(data, i)
        end = i + size
        if section == 2:
            count, i = leb_u32(data, i)
            for _ in range(count):
                module, i = name(data, i)
                field, i = name(data, i)
                kind = data[i]
                i += 1
                if kind == 0:
                    _, i = leb_u32(data, i)
                elif kind == 1:
                    i = limits(data, i + 1)
                elif kind == 2:
                    i = limits(data, i)
                elif kind == 3:
                    i += 2
                elif kind == 4:
                    _, i = leb_u32(data, i + 1)
                out.append((module, field, KIND.get(kind, str(kind))))
        i = end
    return out


def imports_from_wasm_tools(wasm):
    tool = shutil.which("wasm-tools")
    if not tool:
        return None
    text = subprocess.run([tool, "print", str(wasm)], capture_output=True, text=True, check=True).stdout
    return [(m, f) for m, f in re.findall(r'\(import "([^"]*)" "([^"]*)"', text)]


def strip_js(src):
    """Blank out comments and string/template contents, keeping offsets."""
    out, i, n = [], 0, len(src)
    while i < n:
        c, nxt = src[i], src[i + 1] if i + 1 < n else ""
        if c == "/" and nxt == "/":
            while i < n and src[i] != "\n":
                out.append(" ")
                i += 1
            continue
        if c == "/" and nxt == "*":
            j = src.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append(re.sub(r"[^\n]", " ", src[i:j]))
            i = j
            continue
        if c in "\"'`":
            j = i + 1
            while j < n and src[j] != c:
                j += 2 if src[j] == "\\" else 1
            out.append(" " * (min(j, n - 1) - i + 1))
            i = j + 1
            continue
        out.append(c)
        i += 1
    return "".join(out)


def body_of(glue, field):
    """The source of the glue function that provides import `field`."""
    m = re.search(r"\b" + re.escape(field) + r"\s*:\s*function\s*\(", glue)
    if not m:
        m = re.search(r"\bfunction\s+" + re.escape(field) + r"\s*\(", glue)
    if not m:
        return None
    start = glue.index("{", m.end())
    depth, i = 0, start
    code = strip_js(glue)
    while i < len(code):
        if code[i] == "{":
            depth += 1
        elif code[i] == "}":
            depth -= 1
            if depth == 0:
                return glue[m.start() : i + 1]
        i += 1
    return None


def network_names(code):
    stripped = strip_js(code)
    return sorted({api for api in NETWORK_APIS if re.search(r"(?<![\w$])" + api + r"(?![\w$])", stripped)})


def main():
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    dist = pathlib.Path(sys.argv[1])
    modules = sorted(dist.rglob("*_bg.wasm"))
    if not modules:
        print(f"no *_bg.wasm under {dist}: run scripts/build.sh first")
        return 2
    flagged = 0
    for wasm in modules:
        glue_path = wasm.with_name(wasm.name.replace("_bg.wasm", ".js"))
        glue = glue_path.read_text(encoding="utf-8")
        imports = read_imports(wasm)
        print(f"module  {wasm.relative_to(dist)}  ({wasm.stat().st_size:,} bytes)")
        print(f"glue    {glue_path.relative_to(dist)}")
        print(f"imports {len(imports)}")

        cross = imports_from_wasm_tools(wasm)
        if cross is None:
            print("        (wasm-tools not on PATH; import list read by this script only)")
        elif cross != [(m, f) for m, f, _ in imports]:
            print("FLAG    wasm-tools print disagrees with this script's import list")
            print(f"        wasm-tools: {cross}")
            flagged += 1
        else:
            print("        wasm-tools print agrees with this list")

        for module, field, kind in imports:
            print(f"\n  {kind:6} {module} :: {field}")
            body = body_of(glue, field) if kind == "func" else None
            if kind == "func" and body is None:
                print("  FLAG   no glue function found for this import")
                flagged += 1
                continue
            if body:
                for line in body.splitlines():
                    print("         | " + line)
                found = network_names(body)
                if found:
                    print(f"  FLAG   reaches {', '.join(found)}")
                    flagged += 1

        snippets = sorted((glue_path.parent / "snippets").rglob("*.js")) if (glue_path.parent / "snippets").exists() else []
        for snip in snippets:
            found = network_names(snip.read_text(encoding="utf-8"))
            print(f"\n  snippet {snip.relative_to(dist)}: {', '.join(found) if found else 'no network API'}")
            if found:
                flagged += 1

        fetches = len(re.findall(r"(?<![\w$.])fetch\s*\(", strip_js(glue)))
        print(f"\n  fetch calls in the whole glue file: {fetches} (1 expected: the module load)")
        if fetches != 1:
            flagged += 1
            print("  FLAG   unexpected number of fetch calls in the glue")
        print()

    print("WASM IMPORT AUDIT " + ("PASSED" if flagged == 0 else f"FLAGGED {flagged}"))
    return 0 if flagged == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
