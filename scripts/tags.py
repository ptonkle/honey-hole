#!/usr/bin/env python3
"""Test tags: which tests a change needs (tests/tags.toml).

    scripts/tags.py guard                 fail if a cargo test, browser step or
                                          tracked file has no tags
    scripts/tags.py quick [base] [opts]   the quick check (scripts/quick-check.sh)
    scripts/tags.py failures LOG          the failing tests named in a check's output

quick options:
    --range A..B      take the changed files from A..B only (not the working
                      tree); the tests run are today's
    --dry-run         print the plan, run nothing
    --all-browsers    the browser check in Chromium, Firefox and WebKit (also
                      ALL_BROWSERS=1); Chromium only otherwise

A test runs when its tags meet the tags of the changed files (`all` runs
everything). A changed file is placed line by line where it can be
(file_plan): a test file runs its own tests only; a line in a
`#[cfg(test)] mod` runs its test; a [lines] pattern (say, a version bump)
takes that rule's tags; a line in a function named in [functions] takes that
function's tags (outside a fn, a struct, an enum or a one-line const is named
the same way); a line in a browser step of a [steps] file runs that step.
Any other line takes the file's [sources] tags. A file no glob matches runs
everything.

The base defaults to the commit of the newest passing `check` row in
notes/check-times.md, so a quick check only ever skips relative to a full pass.
"""
import json
import os
import pathlib
import re
import subprocess
import sys
import time
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]
TAGS = ROOT / "tests" / "tags.toml"
ALL = "all"


def load():
    return tomllib.loads(TAGS.read_text())


def glob_re(glob):
    out = ""
    i = 0
    while i < len(glob):
        if glob.startswith("**/", i):
            out += "(?:.*/)?"
            i += 3
        elif glob.startswith("**", i):
            out += ".*"
            i += 2
        elif glob[i] == "*":
            out += "[^/]*"
            i += 1
        elif glob[i] == "?":
            out += "[^/]"
            i += 1
        else:
            out += re.escape(glob[i])
            i += 1
    return re.compile(out + r"\Z")


def file_tags(cfg, path):
    """The tags of one changed file, or None when no glob matches it."""
    hit = None
    for glob, tags in cfg["sources"].items():
        if glob_re(glob).match(path):
            hit = (hit or set()) | set(tags)
    return hit


def sh(cmd, **kw):
    return subprocess.run(cmd, cwd=ROOT, text=True, check=True, capture_output=True, **kw).stdout


# --------------------------------------------------------------- cargo tests


def cargo_tests():
    """Every cargo test as an id: `lib:<path>` for the core's own tests,
    `shim:<path>` for the shim's, `test:<file>:<name>` for integration tests."""
    out = subprocess.run(
        ["cargo", "test", "--workspace", "--no-run", "--message-format=json", "--quiet"],
        cwd=ROOT, text=True, check=True, stdout=subprocess.PIPE,
    ).stdout
    ids = []
    for line in out.splitlines():
        if not line.startswith("{"):
            continue
        m = json.loads(line)
        if m.get("reason") != "compiler-artifact" or not m.get("executable") or not m["profile"]["test"]:
            continue
        t = m["target"]
        if "test" in t["kind"]:
            prefix = f"test:{t['name']}:"
        elif t["name"] == "tool_core":
            prefix = "lib:"
        else:
            prefix = "shim:"
        listed = subprocess.run(
            [m["executable"], "--list", "--format", "terse"], cwd=ROOT, text=True, check=True, capture_output=True
        ).stdout
        for row in listed.splitlines():
            if row.endswith(": test"):
                ids.append(prefix + row[: -len(": test")])
    return ids


def key_prefix(key):
    # `test:ai` covers the file's tests, not `test:ai_more`
    return key + ":" if key.startswith("test:") and key.count(":") == 1 else key


def test_tags(cfg, tid):
    best = None
    for key, tags in cfg["tests"].items():
        p = key_prefix(key)
        if tid.startswith(p) and (best is None or len(p) > len(best[0])):
            best = (p, tags)
    return None if best is None else set(best[1])


def implied(cfg, hit):
    """The hit tags with what [implies] adds (a tag whose change reaches another's tests)."""
    out = set(hit)
    for t in hit:
        out |= set(cfg.get("implies", {}).get(t, []))
    return out


def wanted(tags, hit):
    """A test runs when its tags meet the hit ones; `all` runs every test."""
    return ALL in hit or bool(tags & hit)


# --------------------------------------------------------------------- steps


def browser_steps():
    sys.path.insert(0, str(ROOT / "tests" / "browser"))
    import interactions

    return [name for name, _ in interactions.STEPS]


def step_hit(tags, hit):
    return ALL in hit or "page" in hit or bool(set(tags) & hit)


# --------------------------------------------------------------------- guard


def guard():
    cfg = load()
    known = set(cfg["tags"])
    problems = []

    def check_names(where, tags):
        for t in tags:
            if t not in known:
                problems.append(f"{where}: unknown tag {t!r}")

    for section in ("sources", "tests", "browser"):
        for k, tags in cfg[section].items():
            check_names(f"[{section}] {k}", tags)
    check_names("[always]", cfg["always"]["tags"])
    for k, tags in cfg.get("implies", {}).items():
        check_names(f"[implies] {k}", [k, *tags])
    for path, rule in cfg.get("lines", {}).items():
        check_names(f"[lines] {path}", rule["tags"])
        for pat in rule["patterns"]:
            try:
                re.compile(pat)
            except re.error as e:
                problems.append(f"[lines] {path}: bad pattern {pat!r}: {e}")

    for path, funcs in cfg.get("functions", {}).items():
        for pat, tags in funcs.items():
            check_names(f"[functions] {path} {pat}", tags)
            try:
                re.compile(pat)
            except re.error as e:
                problems.append(f"[functions] {path}: bad pattern {pat!r}: {e}")
    for path, pat in cfg.get("steps", {}).items():
        try:
            if re.compile(pat).groups < 1:
                problems.append(f"[steps] {path}: {pat!r} has no group for the step")
        except re.error as e:
            problems.append(f"[steps] {path}: bad pattern {pat!r}: {e}")

    ids = cargo_tests()
    used = set()
    for tid in ids:
        tags = test_tags(cfg, tid)
        if not tags:
            problems.append(f"test {tid} has no tags in [tests]")
        for key in cfg["tests"]:
            if tid.startswith(key_prefix(key)):
                used.add(key)
    for key in cfg["tests"]:
        if key not in used:
            problems.append(f"[tests] {key} matches no test (stale)")

    steps = browser_steps()
    for s in steps:
        if not cfg["browser"].get(s):
            problems.append(f"browser step {s} has no tags in [browser]")
    for s in cfg["browser"]:
        if s not in steps:
            problems.append(f"[browser] {s} is not a step in interactions.py (stale)")

    files = sh(["git", "ls-files"]).split()
    unmatched = [f for f in files if file_tags(cfg, f) is None]
    for f in unmatched:
        problems.append(f"tracked file {f} matches no glob in [sources]")

    print(f"tags: {len(ids)} cargo tests, {len(steps)} browser steps, {len(files)} tracked files")
    if problems:
        print("TAG GUARD FAILED (classify them in tests/tags.toml):")
        for p in problems:
            print("  - " + p)
        return 1
    print("every test, step and file is classified")
    return 0


# --------------------------------------------------------------------- quick


def default_base():
    log = ROOT / "notes" / "check-times.md"
    rows = log.read_text().splitlines() if log.exists() else []
    for row in reversed(rows):
        cells = [c.strip() for c in row.split("|")]
        # a quick run that hit `all` ran every test and step, so it counts too
        kind = cells[5] if len(cells) > 8 else ""
        ran_all = kind.startswith("quick") and re.search(r"tags [^)]*\ball\b", kind)
        if len(cells) > 8 and (kind.split(" ")[0] == "check" or ran_all) and cells[8] == "pass":
            return cells[3].rstrip("+")
    sys.exit("no passing check row in notes/check-times.md: run the full check (./scripts/timed-checks.sh check)")


def changed_files(base, rng):
    if rng:
        return sorted(set(sh(["git", "diff", "--name-only", rng]).split()))
    files = set(sh(["git", "diff", "--name-only", base, "HEAD"]).split())
    files |= set(sh(["git", "diff", "--name-only", "HEAD"]).split())
    files |= set(sh(["git", "ls-files", "--others", "--exclude-standard"]).split())
    return sorted(files)



ITEM = re.compile(
    r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:(?:const|async|unsafe|extern\s+"[^"]*")\s+)*(fn|mod|struct|enum)\s+(\w+)')
# a one-line `const`/`static` item: [functions] names it as it names a fn
CONST = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const|static)\s+(?:mut\s+)?([A-Z_][A-Z0-9_]*)\s*:')
LEAD = re.compile(r"^\s*(//|#\[|#!\[)")


def rust_items(text):
    """The fn, mod, struct and enum items of a Rust file as (kind, name, first, last, attrs):
    1-based lines, `first` taking in the doc comments and attributes above
    the item. Braces are counted outside strings, chars and comments."""
    lines = text.split("\n")
    items, stack, pending = [], [], None
    depth, state, hashes, block = 0, None, 0, 0
    for no, line in enumerate(lines, 1):
        if state is None:
            m = ITEM.match(line)
            if m:
                first = no
                while first > 1 and LEAD.match(lines[first - 2]):
                    first -= 1
                attrs = " ".join(lines[first - 1:no - 1])
                pending = (m.group(1), m.group(2), first, attrs, depth)
        i, n = 0, len(line)
        if state == "line":
            state = None
        while i < n:
            c = line[i]
            if state == "block":
                if line.startswith("*/", i):
                    block -= 1
                    i += 2
                    if not block:
                        state = None
                    continue
                if line.startswith("/*", i):
                    block += 1
                    i += 2
                    continue
                i += 1
                continue
            if state == "str":
                if c == "\\":
                    i += 2
                    continue
                if c == '"':
                    state = None
                i += 1
                continue
            if state == "raw":
                if line.startswith('"' + "#" * hashes, i):
                    state = None
                    i += 1 + hashes
                    continue
                i += 1
                continue
            # normal
            if line.startswith("//", i):
                break
            if line.startswith("/*", i):
                state, block = "block", 1
                i += 2
                continue
            m = re.match(r'b?r(#*)"', line[i:])
            if m and (i == 0 or not (line[i - 1].isalnum() or line[i - 1] == "_")):
                state, hashes = "raw", len(m.group(1))
                i += len(m.group(0))
                continue
            if c == '"':
                state = "str"
            elif c == "'":
                if line.startswith("\\", i + 1):
                    j = line.find("'", i + 2)
                    i = (j if j > 0 else i) + 1
                    continue
                if i + 2 < n and line[i + 2] == "'":
                    i += 3
                    continue
            elif c == ";" and pending and pending[4] == depth:
                pending = None
            elif c == "{":
                if pending and pending[4] == depth:
                    stack.append(pending)
                    pending = None
                depth += 1
            elif c == "}":
                depth -= 1
                if stack and stack[-1][4] == depth:
                    kind, name, first, attrs, _ = stack.pop()
                    items.append((kind, name, first, no, attrs))
            i += 1
    return items


def python_items(text):
    """The top-level defs of a Python file as (kind, name, first, last, attrs)."""
    lines = text.split("\n")
    out, cur = [], None
    for no, line in enumerate(lines, 1):
        if line and not line[0].isspace() and not line.startswith("#"):
            if cur:
                out.append(cur)
                cur = None
            m = re.match(r"(?:async\s+)?def\s+(\w+)", line)
            if m:
                cur = ("fn", m.group(1), no, no, "")
        elif cur and line.strip():
            cur = cur[:3] + (no,) + cur[4:]
    if cur:
        out.append(cur)
    return out


def enclosing(items, line):
    """The items holding a line, outermost first."""
    return sorted((it for it in items if it[2] <= line <= it[3]), key=lambda it: it[2])


def hunks(base, rng, path):
    """The changed lines of one file's diff as (side, line number, text):
    side '-' numbers the old file's lines, '+' the new file's."""
    out = sh(["git", "diff", "-U0", "--no-color", rng or base, "--", path])
    rows, old, new = [], 0, 0
    for ln in out.splitlines():
        m = re.match(r"@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@", ln)
        if m:
            old, new = int(m.group(1)), int(m.group(2))
        elif ln.startswith(("+++ ", "--- ")):
            continue
        elif ln[:1] == "-":
            rows.append(("-", old, ln[1:]))
            old += 1
        elif ln[:1] == "+":
            rows.append(("+", new, ln[1:]))
            new += 1
    return rows


def side_text(base, rng, path, side):
    """A file's text before ('-') or after ('+') the change."""
    if rng:
        a, b = rng.split("..", 1)
        rev = a if side == "-" else b
    elif side == "-":
        rev = base
    else:
        p = ROOT / path
        return p.read_text(errors="replace") if p.exists() else ""
    r = subprocess.run(["git", "show", f"{rev}:{path}"], cwd=ROOT, capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else ""


def module_path(path):
    """`crates/core/src/menu/screen_tests.rs` -> `menu::screen_tests`."""
    rel = path[len("crates/core/src/"):-len(".rs")]
    rel = re.sub(r"(^|/)(lib|mod)$", "", rel)
    return rel.replace("/", "::")


def is_test_mod(item):
    return item[0] == "mod" and ("cfg(test)" in item[4] or item[1] == "tests" or item[1].endswith("_tests"))


def test_file_prefix(path):
    """The test id prefix of a file that holds only tests, or None."""
    m = re.fullmatch(r"crates/core/tests/(\w+)\.rs", path)
    if m:
        return f"test:{m.group(1)}:"
    if path.startswith("crates/core/src/") and re.search(r"(^|/|_)tests\.rs$", path):
        return f"lib:{module_path(path)}::"
    return None


def picked_by(picks, tid):
    """Whether a test id is one of the picks: an exact id, or a prefix
    ending `:` or `::` (a test file's or a test module's tests)."""
    return any(tid == p or (p.endswith(":") and tid.startswith(p)) for p in picks)


def test_pick(path, chain, ids):
    """The tests a line in a `#[cfg(test)] mod` needs: the test it sits in,
    or the module's tests (a helper, a `use`), or None outside one."""
    mods = []
    for i, it in enumerate(chain):
        if it[0] != "mod":
            return None
        mods.append(it[1])
        if is_test_mod(it):
            mp = "::".join(x for x in [module_path(path), *mods] if x)
            inner = chain[i + 1] if i + 1 < len(chain) else None
            if inner and inner[0] == "fn" and f"lib:{mp}::{inner[1]}" in ids():
                return f"lib:{mp}::{inner[1]}"
            prefix = f"lib:{mp}::"
            return prefix if any(t.startswith(prefix) for t in ids()) else None
    return None


def file_plan(cfg, path, base, rng, ids):
    """One changed file's (tags, cargo test picks, browser step picks).

    A test file runs its own tests only. Otherwise each changed line is
    placed: a [lines] pattern takes that rule's tags; a line in a
    `#[cfg(test)] mod` runs that test (or the module's tests, for a helper);
    a line in a function that a [functions] regex names takes its tags; a
    line in a `def step_<name>` of a [steps] file runs that browser step.
    Any other line takes the file's [sources] tags."""
    fallback = file_tags(cfg, path)
    fallback = {ALL} if fallback is None else fallback
    prefix = test_file_prefix(path)
    if prefix:
        if any(t.startswith(prefix) for t in ids()):
            return set(), {prefix}, set()
        if not (ROOT / path).exists() and not rng:
            return set(), set(), set()
        return fallback, set(), set()
    lines_rule = cfg.get("lines", {}).get(path)
    funcs = cfg.get("functions", {}).get(path)
    steps = cfg.get("steps", {}).get(path)
    rust_core = path.startswith("crates/core/src/") and path.endswith(".rs")
    if not (lines_rule or funcs or steps or rust_core):
        return fallback, set(), set()
    pats = [re.compile(p) for p in (lines_rule or {}).get("patterns", [])]
    fpats = [(re.compile(k), set(v)) for k, v in (funcs or {}).items()]
    items = {}
    tags, picks, bsteps = set(), set(), set()
    rows = hunks(base, rng, path)
    if not rows:  # an untracked file, a binary one: no lines to place
        return fallback, set(), set()
    for side, no, text in rows:
        if not text.strip():  # a blank line breaks nothing
            continue
        if pats and any(p.search(text) for p in pats):
            tags |= set(lines_rule["tags"])
            continue
        if side not in items:
            src = side_text(base, rng, path, side)
            items[side] = python_items(src) if path.endswith(".py") else rust_items(src)
        chain = enclosing(items[side], no)
        if rust_core:
            pick = test_pick(path, chain, ids)
            if pick:
                picks.add(pick)
                continue
        fn = next((it[1] for it in chain if it[0] == "fn"), None)
        if fn is None and funcs:
            # outside any fn: the struct or enum holding it, or the const it
            # declares, is named as a fn would be
            fn = next((it[1] for it in chain if it[0] in ("struct", "enum")), None)
            m = CONST.match(text) if fn is None and not chain else None
            fn = m.group(1) if m else fn
        if fn and steps:
            m = re.fullmatch(steps, fn)
            if m and m.group(1) in cfg["browser"]:
                bsteps.add(m.group(1))
                continue
        hit = set().union(*(t for p, t in fpats if fn and p.search(fn))) if fn else set()
        tags |= hit or fallback
    return tags, picks, bsteps


def plan(cfg, files, base=None, rng=None, ids=None):
    hit = set(cfg["always"]["tags"])
    rows, picks, bsteps = [], set(), set()
    cache = []

    def all_ids():
        if not cache:
            cache.append(set(ids() if ids else cargo_tests()))
        return cache[0]

    for f in files:
        if file_tags(cfg, f) is None:
            print(f"  {f} matches no glob in tests/tags.toml: running everything")
        tags, p, b = file_plan(cfg, f, base, rng, all_ids)
        rows.append((f, tags, p, b))
        hit |= tags
        picks |= p
        bsteps |= b
    return rows, implied(cfg, hit), picks, bsteps


def quick(args):
    rng = None
    dry = "--dry-run" in args
    every = "--all-browsers" in args or os.environ.get("ALL_BROWSERS") == "1"
    rest = [a for a in args if a not in ("--dry-run", "--all-browsers")]
    if "--range" in rest:
        rng = rest[rest.index("--range") + 1]
        rest = [a for a in rest if a not in ("--range", rng)]
    base = rest[0] if rest else (None if rng else default_base())
    cfg = load()
    files = changed_files(base, rng)
    listed = []

    def ids_once():
        if not listed:
            t = time.time()
            listed.append(cargo_tests())
            listed.append(time.time() - t)
        return listed[0]

    rows, hit, picks, direct = plan(cfg, files, base, rng, ids_once)

    print(f"== quick check: changes {rng or base + '..HEAD + working tree'}")
    for f, tags, p, b in rows:
        own = [", ".join(sorted(tags))] if tags else []
        if p:
            own.append(f"its tests {', '.join(sorted(p))}" if len(p) < 4 else f"{len(p)} of its tests")
        if b:
            own.append(f"browser step {', '.join(sorted(b))}")
        print(f"  {f} -> {'; '.join(own) or '(nothing)'}")
    print(f"tags hit: {', '.join(sorted(hit))}")

    engines = "chromium firefox webkit" if every else "chromium"
    py = os.environ.get("BUILDER_PYTHON") or (str(ROOT / ".venv/bin/python") if (ROOT / ".venv/bin/python").exists() else "python3")
    import checkrun  # scripts/checkrun.py: how each stage prints

    run = checkrun.Run("quick")
    times = []

    def plan_rest():
        """The cargo picks (lists the cargo tests once)."""
        ids = ids_once()
        times.append(f"list={listed[1]:.1f}")
        picked = [tid for tid in ids if wanted(test_tags(cfg, tid) or {ALL}, hit) or picked_by(picks, tid)]
        # one invocation by exact name: each test binary runs the names it has
        # (lib names carry a module path; integration names are bare)
        names = sorted({tid.split(":", 2)[-1] if tid.startswith("test:") else tid.split(":", 1)[1] for tid in picked})
        tfiles = sorted({tid.split(":")[1] for tid in picked if tid.startswith("test:")})
        return ids, picked, names, tfiles

    bsteps = [s for s, tags in cfg["browser"].items() if step_hit(tags, hit) or s in direct]
    # boot alone runs too (a tab icon change): it is what boot checks
    browser_run = [s for s in bsteps if s != "boot"] or (["boot"] if "boot" in bsteps else [])
    # the checks outside cargo and the browser, each run when its tag is hit
    extra = [(key, label, cmd) for key, (tag, label, cmd) in {
        "art": ("art", "art pipeline checks (tests/art_check.py)", [py, "tests/art_check.py"]),
        "notebook": ("notes", "notebook form (scripts/notebook_check.py)", [py, "scripts/notebook_check.py"]),
    }.items() if (ALL in hit or tag in hit) and (ROOT / cmd[1]).exists()]
    stages = 6 + len(extra) + bool(browser_run)  # format lint core tags release WebAssembly ...
    n = iter(range(1, stages + 1))

    def step(name, label, mode, cmd, env=None):
        code = run.stage(name, f"{next(n)}/{stages}", label, mode, cmd, env)
        times.append(f"{name}={run.stages[-1][3]:.1f}")
        if code:
            print(f"\nQUICK CHECK FAILED at {name} (exit {code})")
            print(f"QUICK tags={','.join(sorted(hit))} engines={engines.replace(' ', '+')}")
            print("TIMES " + " ".join(times))
            sys.exit(code)

    if not dry:
        step("format", "format (cargo fmt --check)", "plain", ["cargo", "fmt", "--all", "--", "--check"])
        step("lint", "lint (cargo clippy)", "plain", ["cargo", "clippy", "--workspace", "--all-targets", "--quiet", "--", "-D", "warnings"])

    ids, picked, names, tfiles = plan_rest()
    print(f"\ncargo tests: {len(picked)} of {len(ids)} (integration files {', '.join(tfiles) or 'none'})")
    print(f"browser steps: {', '.join(['boot'] + [s for s in browser_run if s != 'boot']) if browser_run else 'none (browser check skipped)'}"
          f"{' in ' + engines if browser_run else ''}")
    print(f"other checks: {', '.join(k for k, _, _ in extra) or 'none'}")
    if dry:
        return 0

    if ALL in hit:
        step("core", "core tests (all)", "cargo", ["cargo", "test", "--workspace"])
    elif names:
        step("core", f"core tests ({len(picked)} picked)", "cargo",
             ["cargo", "test", "--workspace", "--lib", "--tests", "--", "--exact", *names])
    else:
        run.skip("core", f"{next(n)}/{stages}", "core tests", "none picked")
    # the guard is cheap and catches a new test or file left untagged before
    # the full check does
    step("tags", "test tags (scripts/tags.py guard)", "plain", [py, "scripts/tags.py", "guard"])
    step("release", "release build (scripts/build.sh)", "quiet", ["./scripts/build.sh"])
    step("WebAssembly", "WebAssembly import audit", "wasm", [py, "scripts/wasm_imports.py", "dist"])
    for key, label, cmd in extra:
        step(key, label, "quiet" if key == "art" else "plain", cmd)
    if browser_run:
        env = {"BUILDER_ENGINES": engines}
        if not (ALL in hit or "page" in hit):
            env["BUILDER_STEPS"] = " ".join(s for s in browser_run if s != "boot")
        step("browser", f"browser check ({engines})", "plain", [py, "tests/browser/check.py"], env)

    run.summary()
    print(f"QUICK tags={','.join(sorted(hit))} engines={engines.replace(' ', '+') if browser_run else 'none'}")
    print("TIMES " + " ".join(times) + f" total={time.time() - run.t0:.1f}")
    print("\n\033[1mQUICK CHECK PASSED\033[0m")
    return 0


# ------------------------------------------------------------------ failures


def failures(path):
    """The failing tests a check's output names, one per line."""
    out = []
    engine = step = None
    stage = None
    for line in pathlib.Path(path).read_text(errors="replace").splitlines():
        line = re.sub(r"\x1b\[[0-9;]*m", "", line)
        if line.startswith("== "):
            stage = line[3:].split(" — ")[0].split(" (")[0].split(":")[0]
        m = re.match(r"\[([\w-]+)\] ", line)
        if m:
            engine, step = m.group(1), None
        m = re.match(r"  -- step (\w+)", line) or re.match(r"\s*\[[\w-]+ \d+/\d+\] step \d+/\d+ (\w+) …", line)
        if m:
            step = m.group(1)
        m = re.match(r"test (\S+) \.\.\. FAILED", line) or re.match(r"---- (\S+) stdout ----", line)
        if m:
            out.append(f"cargo:{m.group(1)}")
        # cargo-nextest (a watched run): `FAIL [   0.004s] (1/21) tool_core page::tests::x`
        m = re.match(r"\s*(?:FAIL|SIGSEGV|SIGABRT|SIGKILL|TIMEOUT|ABORT|LEAK-FAIL)\s+\[\s*[\d.]+s\]\s+(?:\(\s*\d+/\d+\)\s+)?\S+\s+(\S+)", line)
        if m:
            out.append(f"cargo:{m.group(1)}")
        if line.startswith("  FAIL "):
            where = f"browser[{engine}]" + (f"/{step}" if step else "") if engine else "browser"
            out.append(f"{where}: {line[7:]}")
        if re.match(r"(playwright\.\S+|\w+Error): ", line):
            out.append(f"{stage or '?'}{f' [{engine}]/{step}' if engine and step else ''}: {line[:160]}")
    if not out and stage:
        out.append(f"step {stage} failed")
    seen = set()
    return [x for x in out if not (x in seen or seen.add(x))]


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    cmd = sys.argv[1]
    if cmd == "guard":
        sys.exit(guard())
    if cmd == "quick":
        sys.exit(quick(sys.argv[2:]))
    if cmd == "failures":
        print("\n".join(failures(sys.argv[2])))
        return
    sys.exit(__doc__)


if __name__ == "__main__":
    main()
