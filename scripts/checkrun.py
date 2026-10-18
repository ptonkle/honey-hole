#!/usr/bin/env python3
"""Runs one stage of a check and prints it for a person watching live.

scripts/check.sh and scripts/tags.py quick (the quick check) both run their
stages through this, so the two print the same way:

    == 3/9 core and boundary tests — usually ~4 s
       tool_core lib        13 passed   0.0 s
       boundary             14 passed   0.1 s
       ✓ core and boundary tests: 40 passed, 0 failed (3.2 s)

and end with one summary line per stage. The `TIMES`, `QUICK ...`,
`... CHECK PASSED/FAILED` lines that scripts/timed-checks.sh and
`scripts/tags.py failures` read come after.

    checkrun.py begin <kind>                       start a run (check, quick)
    checkrun.py stage <key> <n/m> <label> <mode> -- <cmd...>
    checkrun.py summary                            the closing table
    checkrun.py times                              the `TIMES` line
    checkrun.py plain [width] < raw > log          a terminal recording as plain lines

`kind` picks the rows of notes/check-times.md the estimates come from; `key`
is the stage's name in the `TIMES` line. Modes:

    plain   the command's output as it comes
    quiet   nothing unless it fails (then all of it)
    cargo   cargo test, one line per test binary; failures in full
    wasm    scripts/wasm_imports.py: one line on a pass, all of it on a flag

The run's state (start time, finished stages) is in the file $CHECK_STATE.
Watched in a terminal (stdout a TTY, CHECK_PLAIN unset): colour, the cargo
tests as cargo-nextest's progress bar (when installed; CHECK_NEXTEST=0 to
skip it) and the browser checks' rich bars (tests/browser/progress.py).
Anywhere else, e.g. logged to a file: the plain lines.
"""
import json
import os
import pathlib
import re
import shutil
import statistics
import struct
import subprocess
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
LOG = ROOT / "notes" / "check-times.md"


def colour(code, text):
    on = sys.stdout.isatty() and os.environ.get("CHECK_PLAIN") != "1"
    return f"\033[{code}m{text}\033[0m" if on else text


def secs(s):
    return f"{s:.1f} s" if s < 10 else f"{s:.0f} s"


def emit(kind, **fields):
    """One progress event for the dashboard (scripts/check-dashboard.py),
    appended to $CHECK_EVENTS as a JSON line; nothing without it."""
    path = os.environ.get("CHECK_EVENTS")
    if path:
        with open(path, "a") as f:
            f.write(json.dumps({"k": kind, "at": time.time(), **fields}) + "\n")


def estimate(kind, key):
    """The median seconds of `key` in the last five passing `kind` rows, or None."""
    try:
        rows = LOG.read_text().splitlines()
    except OSError:
        return None
    seen = []
    for row in rows:
        cells = [c.strip() for c in row.split("|")]
        if len(cells) < 9 or cells[8] != "pass" or not cells[5].startswith(kind + " "):
            continue
        m = re.search(rf"(?:^| ){re.escape(key)}=([0-9.]+)", cells[6])
        if m:
            seen.append(float(m.group(1)))
    return statistics.median(seen[-5:]) if seen else None


def header(n, label, est):
    tail = f" — usually ~{secs(est)}" if est is not None and est >= 1 else ""
    emit("stage", n=n, label=label, est=est)
    print("\n" + colour("1", f"== {n} {label}{tail}"), flush=True)


def footer(ok, label, detail, took):
    mark = colour("32", "✓") if ok else colour("31", "✗")
    emit("done", label=label, ok=ok, detail=detail, secs=took)
    print(f"   {mark} {label}: {detail} ({secs(took)})" if detail else f"   {mark} {label} ({secs(took)})", flush=True)


# ------------------------------------------------------------------ runners


def live():
    """Whether a person watches this in a terminal (bars, progress, colour)."""
    return sys.stdout.isatty() and os.environ.get("CHECK_PLAIN") != "1"


ANSI = re.compile(r"\x1b\[[0-9;?]*[A-Za-z]|\x1b\][^\x07]*\x07|\x1b[()][0-9A-B]")


def plain_lines(raw, width=None):
    """Terminal output as the lines a person saw last: a small terminal that
    follows the cursor moves, line clears and wrapping of progress bars, then
    gives each line (wrapped rows joined again) without colour."""
    width = width or int(os.environ.get("COLUMNS", "0") or 0) or 200
    rows, wrapped = [[]], [False]  # each row's characters; whether it wrapped on from the row above
    r = c = 0
    i, n = 0, len(raw)

    def row(k):
        while len(rows) <= k:
            rows.append([])
            wrapped.append(False)
        return rows[k]

    while i < n:
        ch = raw[i]
        if ch == "\x1b":
            m = re.compile(r"\x1b\[([0-9;?]*)([A-Za-z])").match(raw, i)
            if m:
                arg, op = m.group(1), m.group(2)
                k = int(arg.split(";")[0]) if arg[:1].isdigit() else None
                if op == "A":
                    r = max(0, r - (k or 1))
                elif op == "B":
                    r += k or 1
                elif op == "G":
                    c = (k or 1) - 1
                elif op == "K":
                    line = row(r)
                    if k == 2:
                        line.clear()
                        wrapped[r] = False
                    elif not k:
                        del line[c:]
                elif op == "J" and k in (None, 0):
                    del row(r)[c:]
                    del rows[r + 1:], wrapped[r + 1:]
                i = m.end()
                continue
            m = re.compile(r"\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1b[()][0-9A-B]|\x1b[=>78]").match(raw, i)
            i = m.end() if m else i + 1
            continue
        if ch == "\r":
            c = 0
        elif ch == "\n":  # as a terminal with onlcr: a new line starts at its left
            r, c = r + 1, 0
            row(r)
            wrapped[r] = False
        elif ch == "\b":
            c = max(0, c - 1)
        elif ch >= " ":
            if c >= width:
                r, c = r + 1, 0
                row(r)
                wrapped[r] = True
            line = row(r)
            line.extend(" " * (c - len(line)))
            if c < len(line):
                line[c] = ch
            else:
                line.append(ch)
            c += 1
        i += 1
    out = []
    for line, cont in zip(rows, wrapped):
        text = "".join(line)
        if cont and out and out[-1][-1:] not in ("", " "):  # a long line, not a padded bar row
            out[-1] += text
        else:
            out.append(text)
    return [l.rstrip() for l in out]


# the widest a redrawn bar line may be: narrower than any window it will
# reasonably be watched in
BAR_COLS = 80


def run_tty(cmd, env):
    """Runs `cmd` on a terminal of its own (so it draws its bars), copying
    what it draws to ours; returns (exit code, what it printed)."""
    import fcntl
    import pty
    import termios

    master, slave = pty.openpty()
    # At most BAR_COLS wide: this terminal's size is set once, so a bar sized
    # to the window at the start wrapped when the window narrowed (moved to a
    # monitor with other scaling) and every redraw stacked a new line.
    rows, cols = 40, BAR_COLS
    try:
        rows, cols = struct.unpack("HHHH", fcntl.ioctl(sys.stdout.fileno(), termios.TIOCGWINSZ, b"\0" * 8))[:2]
    except OSError:
        pass
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, min(cols, BAR_COLS), 0, 0))
    p = subprocess.Popen(cmd, cwd=ROOT, env=env, stdin=subprocess.DEVNULL, stdout=slave, stderr=slave)
    os.close(slave)
    seen = []
    out = sys.stdout.buffer
    while True:
        try:
            chunk = os.read(master, 65536)
        except OSError:
            break
        if not chunk:
            break
        seen.append(chunk)
        out.write(chunk)
        out.flush()
    os.close(master)
    return p.wait(), b"".join(seen).decode("utf-8", "replace")


def detail_file(env):
    """A file the browser scripts write their counts to (tests/browser/progress.py)."""
    import tempfile

    fd, path = tempfile.mkstemp(prefix="check-detail-")
    os.close(fd)
    env["CHECK_DETAIL"] = path
    return path


def read_detail(path):
    try:
        return pathlib.Path(path).read_text().strip()
    finally:
        pathlib.Path(path).unlink(missing_ok=True)


def run_plain(cmd, env):
    """Passes output through (on a terminal of its own when watched, so the
    browser checks draw their bars); the detail is their counts."""
    path = detail_file(env)
    if live():
        code, raw = run_tty(cmd, env)
        verdict = next((l.strip() for l in reversed(plain_lines(raw)) if re.search(r"CHECK (PASSED|FAILED)", l)), "")
        detail = read_detail(path)
        return code, detail or verdict or ("" if not code else f"exit {code}")
    p = subprocess.Popen(cmd, cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
    verdict, passes, fails = "", 0, 0
    for line in p.stdout:
        sys.stdout.write(line)
        sys.stdout.flush()
        if re.search(r"CHECK (PASSED|FAILED)", line):
            verdict = line.strip()
        passes += line.startswith("  PASS ")
        fails += line.startswith("  FAIL ")
    code = p.wait()
    detail = read_detail(path) or (f"{passes} checks passed, {fails} failed" if passes or fails else "")
    if code and not fails and verdict and not detail:
        detail = verdict
    return code, detail


def run_quiet(cmd, env):
    p = subprocess.run(cmd, cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    if p.returncode:
        sys.stdout.write(p.stdout)
    return p.returncode, "" if not p.returncode else f"exit {p.returncode}"


def run_wasm(cmd, env):
    p = subprocess.run(cmd, cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    out = p.stdout
    if p.returncode:
        sys.stdout.write(out)
        last = [l for l in out.splitlines() if l.startswith("WASM IMPORT AUDIT")]
        return p.returncode, last[-1] if last else f"exit {p.returncode}"
    mods = re.findall(r"^module  (\S+)  \(([\d,]+) bytes\)", out, re.M)
    imports = sum(int(n) for n in re.findall(r"^imports (\d+)", out, re.M))
    agree = "wasm-tools agrees" if "wasm-tools print agrees" in out else "wasm-tools not on PATH"
    what = ", ".join(f"{m} {b} bytes" for m, b in mods)
    print(f"   {what}; {imports} imports, none reaches a network API; 1 fetch (the module load); {agree}")
    print("   WASM IMPORT AUDIT PASSED")
    return 0, f"{imports} imports, nothing flagged"


# cargo's own status lines, which say nothing a watcher needs
CARGO_NOISE = re.compile(r"^\s*(Compiling|Finished|Blocking|Downloaded|Downloading|Updating|Locking|Adding|Fresh)\b")


def binary_name(line):
    """`Running unittests src/lib.rs (target/debug/deps/tool_core-1a2b)` -> `tool_core lib`."""
    m = re.match(r"\s*Running (unittests )?(\S+)(?: \((\S+)\))?", line)
    src, exe = m.group(2), m.group(3) or ""
    crate = re.sub(r"-[0-9a-f]+$", "", pathlib.Path(exe).name) if exe else ""
    if m.group(1):
        return f"{crate} {'lib' if src.endswith('lib.rs') else pathlib.Path(src).stem}".strip()
    return pathlib.Path(src).stem


def nextest_cmd(cmd):
    """`cargo test <targets> [-- --exact names]` as the same run in cargo-nextest."""
    args = [c for c in cmd[2:] if c != "--quiet"]
    names = []
    if "--" in args:
        names = [a for a in args[args.index("--") + 1:] if a != "--exact"]
        args = args[: args.index("--")]
    out = ["cargo", "nextest", "run", *args, "--show-progress", "bar", "--max-progress-running", "0", "--status-level", "slow",
           "--final-status-level", "slow", "--no-input-handler", "--cargo-quiet"]
    if names:
        out += ["-E", " | ".join(f"test(={n})" for n in names)]
    return out


def run_nextest(cmd, env):
    """cargo-nextest with its progress bar; failures print in full."""
    code, raw = run_tty(nextest_cmd(cmd), env)
    text = "\n".join(plain_lines(raw))
    failing = list(dict.fromkeys(f"{a}: {b}" for a, b in NEXTEST_FAIL.findall(text)))
    runs = re.findall(r"Summary \[[^\]]*\]\s+(\d+) tests? run: (\d+) passed(?:, (\d+) failed)?(?:[^\n]*?(\d+) skipped)?", text)
    detail = f"{runs[-1][1]} passed, {runs[-1][2] or 0} failed" + (f", {runs[-1][3]} skipped" if runs[-1][3] else "") if runs else ""
    if code == 0 and "--doc" not in cmd and not ("--lib" in cmd or "--tests" in cmd):
        # nextest does not run doctests; cargo test does (the workspace has none today)
        doc = subprocess.run(["cargo", "test", "--workspace", "--doc", "--quiet"], cwd=ROOT, env=env,
                             stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        n = sum(int(x) for x in re.findall(r"test result: \w+\. (\d+) passed", doc.stdout))
        if doc.returncode:
            sys.stdout.write(doc.stdout)
            return doc.returncode, f"{detail}; doctests failed"
        detail += f", {n} doctests"
    if failing:
        detail += " — " + "; ".join(failing)
    elif code and not detail:
        detail = f"did not build or stopped (exit {code})"
    return code, detail


# a failing test in nextest's output: `FAIL [   0.004s] (1/21) tool_core page::tests::x`
NEXTEST_FAIL = re.compile(r"(?m)^\s*(?:FAIL|SIGSEGV|SIGABRT|SIGKILL|TIMEOUT|ABORT|LEAK-FAIL)\s+\[\s*[\d.]+s\]\s+(?:\(\s*\d+/\d+\)\s+)?(\S+)\s+(\S+)")


def run_nextest_events(cmd, env):
    """cargo-nextest for the dashboard: its JSON events (libtest-json) say
    which tests are running, for a bar event naming the longest-running one;
    the log gets failures and the summary only."""
    args = [a for a in nextest_cmd(cmd) if a not in ("--max-progress-running", "0")]
    i = args.index("--show-progress")
    del args[i:i + 2]
    args += ["--message-format", "libtest-json"]
    env = {**env, "NEXTEST_EXPERIMENTAL_LIBTEST_JSON": "1"}
    import shlex
    import tempfile

    # nextest sends its JSON as it happens only on a terminal (on a pipe every
    # event came at the end): `script` gives it one, its human output going
    # to a file the summary is read from afterwards
    err = tempfile.NamedTemporaryFile("w+", suffix=".nextest", delete=False)
    err.close()
    inner = shlex.join(args) + " 2>" + shlex.quote(err.name)
    p = subprocess.Popen(["script", "-q", "-f", "-e", "-c", inner, "/dev/null"], cwd=ROOT, env=env,
                         stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, stdin=subprocess.DEVNULL)
    total, done, running, last = 0, 0, {}, 0.0

    known = []

    def planned():
        """nextest's own count ("Starting 757 tests"), from its human output."""
        if not known:
            try:
                m = re.search(r"Starting (\d+) tests?", open(err.name, errors="replace").read())
            except OSError:
                m = None
            if m:
                known.append(int(m.group(1)))
        return known[0] if known else 0

    def short(name):
        return name.split("$", 1)[-1]

    def tick(force=False):
        nonlocal last
        if force or time.time() - last > 0.2:
            last = time.time()
            note = ""
            if running:
                name, began = min(running.items(), key=lambda kv: kv[1])
                note = f"{short(name)} ({secs(time.time() - began)})"
                if len(running) > 1:
                    note += f" +{len(running) - 1} more"
            emit("bar", id="core", label="core tests", done=done, total=max(planned() or total, done, 1), note=note)

    def json_lines():
        for raw in p.stdout:
            yield raw.decode("utf-8", "replace").strip()

    for line in json_lines():
        try:
            e = json.loads(line)
        except ValueError:
            continue
        if e.get("type") == "suite" and e.get("event") == "started":
            total += e.get("test_count", 0)
        elif e.get("type") == "test":
            if e.get("event") == "started":
                running[e["name"]] = time.time()
            elif e.get("event") in ("ok", "failed", "timeout"):
                running.pop(e.get("name"), None)
                done += 1
            else:  # ignored: never started, not counted
                running.pop(e.get("name"), None)
        tick()
    code = p.wait()
    tick(force=True)
    with open(err.name, errors="replace") as f:
        text = ANSI.sub("", f.read())
    os.unlink(err.name)
    for line in text.splitlines(True):
        if NEXTEST_FAIL.match(line) or line.lstrip().startswith(("Summary", "error")):
            sys.stdout.write(line)
    sys.stdout.flush()
    failing = list(dict.fromkeys(f"{a}: {b}" for a, b in NEXTEST_FAIL.findall(text)))
    runs = re.findall(r"Summary \[[^\]]*\]\s+(\d+) tests? run: (\d+) passed(?:, (\d+) failed)?(?:[^\n]*?(\d+) skipped)?", text)
    detail = f"{runs[-1][1]} passed, {runs[-1][2] or 0} failed" + (f", {runs[-1][3]} skipped" if runs[-1][3] else "") if runs else ""
    if failing:
        detail += " — " + "; ".join(failing)
    elif code and not detail:
        sys.stdout.write(text)
        detail = f"did not build or stopped (exit {code})"
    return code, detail


def run_cargo(cmd, env):
    """cargo test: in a watched terminal cargo-nextest's progress bar when it
    is installed; otherwise (a log) one line per test binary."""
    nextest = shutil.which("cargo-nextest") and os.environ.get("CHECK_NEXTEST") != "0"
    if nextest and os.environ.get("CHECK_EVENTS") and not live():
        return run_nextest_events(cmd, env)
    if live() and nextest:
        return run_nextest(cmd, env)
    cmd = [c for c in cmd if c != "--quiet"]
    live_ = live()
    p = subprocess.Popen(cmd, cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
    name, buf, done, planned = None, [], 0, 0
    total = {"passed": 0, "failed": 0, "ignored": 0}
    failing, idle, last_tick = [], 0, 0.0
    for line in p.stdout:
        s = line.rstrip("\n")
        if re.match(r"\s*(Running|Doc-tests) ", s):
            name = binary_name(s) if "Running" in s else "doc " + s.split()[-1]
            buf, done, planned = [], 0, 0
            continue
        if name is None:
            if not CARGO_NOISE.match(s) and s.strip():
                print(s, flush=True)
            continue
        m = re.match(r"running (\d+) tests?", s)
        if m:
            planned = int(m.group(1))
            continue
        if re.match(r"test .* \.\.\. ok$", s) or re.match(r"test .* \.\.\. ignored", s):
            done += 1
            if live_ and planned > 20 and time.time() - last_tick > 0.5:
                last_tick = time.time()
                print(f"\r\033[K   {name:<18} {done}/{planned}", end="", flush=True)
            continue
        if "has been running for over" in s:
            print(("\r\033[K" if live_ else "") + "   " + s, flush=True)
            continue
        m = re.match(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored.*finished in ([0-9.]+)s", s)
        if m:
            ok, n_pass, n_fail, n_ign, t = m.group(1) == "ok", int(m.group(2)), int(m.group(3)), int(m.group(4)), float(m.group(5))
            total["passed"] += n_pass
            total["failed"] += n_fail
            total["ignored"] += n_ign
            if live_:
                print("\r\033[K", end="")
            if n_pass + n_fail == 0:
                idle += 1
            else:
                fail = colour("31", f"  {n_fail} FAILED") if n_fail else ""
                ign = f"  ({n_ign} ignored)" if n_ign else ""
                print(f"   {name:<18} {n_pass:>4} passed{fail}  {t:5.1f} s{ign}", flush=True)
            if not ok:  # the binary's own failure report, in full
                for b in buf:
                    print(b)
                print(s, flush=True)
                failing += [f"{name}: {x}" for x in re.findall(r"^test (\S+) \.\.\. FAILED", "\n".join(buf), re.M)]
            name = None
            continue
        if s.strip():
            buf.append(s)
    code = p.wait()
    if idle:
        print(f"   ({idle} test binaries ran no test: none picked, or none in them)")
    if name is not None and buf:  # a binary that stopped without a result: show what it said
        print(f"   {name} stopped without a result:")
        for b in buf:
            print(b)
    detail = f"{total['passed']} passed, {total['failed']} failed"
    if total["ignored"]:
        detail += f", {total['ignored']} ignored"
    if failing:
        detail += " — " + "; ".join(failing)
    elif code and not total["failed"]:
        detail = f"did not build or stopped (exit {code}); {detail}"
    return code, detail


RUNNERS = {"plain": run_plain, "quiet": run_quiet, "cargo": run_cargo, "wasm": run_wasm}


# ------------------------------------------------------------------ state


class Run:
    """One check run: its stages, for the estimates and the closing table."""

    def __init__(self, kind, state=None):
        self.kind, self.state = kind, state
        self.t0 = time.time()
        self.stages = []  # (key, label, ok, seconds, detail)
        if state and state.exists() and state.read_text().strip():
            d = json.loads(state.read_text())
            self.kind, self.t0, self.stages = d["kind"], d["t0"], [tuple(s) for s in d["stages"]]

    def save(self):
        if self.state:
            self.state.write_text(json.dumps({"kind": self.kind, "t0": self.t0, "stages": self.stages}))

    def stage(self, key, n, label, mode, cmd, env=None):
        """Runs one stage; returns its exit code (the summary is printed on a failure)."""
        header(n, label, estimate(self.kind, key))
        t = time.time()
        code, detail = RUNNERS[mode](cmd, {**os.environ, **(env or {})})
        took = time.time() - t
        footer(code == 0, label, detail, took)
        self.stages.append((key, label, code == 0, took, detail))
        self.save()
        if code:
            self.summary()
        return code

    def skip(self, key, n, label, why):
        header(n, label, None)
        print(f"   - {label}: {why}")
        self.stages.append((key, label, True, 0.0, why))
        self.save()

    def times(self):
        return " ".join(f"{k}={s:.1f}" for k, _, _, s, _ in self.stages)

    def summary(self):
        print("\n" + colour("1", "Summary"))
        w = max([32] + [len(label) for _, label, _, _, _ in self.stages])
        for _, label, ok, s, detail in self.stages:
            mark = colour("32", "✓") if ok else colour("31", "✗")
            print(f"   {mark} {label:<{w}} {secs(s):>7}" + ("" if ok else f"   {detail}"))
        print(f"     {'total':<{w}} {secs(time.time() - self.t0):>7}", flush=True)


def main(argv):
    state = pathlib.Path(os.environ["CHECK_STATE"]) if os.environ.get("CHECK_STATE") else None
    if not argv:
        sys.exit(__doc__)
    if argv[0] == "begin":
        r = Run(argv[1])
        r.state = state
        r.save()
        return 0
    if argv[0] == "stage":
        key, n, label, mode = argv[1:5]
        cmd = argv[argv.index("--") + 1:]
        return Run("check", state).stage(key, n, label, mode, cmd)
    if argv[0] == "plain":  # stdin's terminal output as plain lines: `plain [width]`
        width = int(argv[1]) if len(argv) > 1 else None
        sys.stdout.write("\n".join(plain_lines(sys.stdin.buffer.read().decode("utf-8", "replace"), width)) + "\n")
        return 0
    if argv[0] == "times":
        r = Run("check", state)
        print(f"TIMES {r.times()} total={time.time() - r.t0:.1f}")
        return 0
    if argv[0] == "summary":
        Run("check", state).summary()
        return 0
    sys.exit(__doc__)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
