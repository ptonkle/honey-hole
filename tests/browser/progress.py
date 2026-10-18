"""How the browser check reports as it goes.

In a terminal (and with `rich` installed) a live display: one bar per engine
with its steps done of total, the step now running and the time so far, and
an overall bar; a failed check prints in red as it happens, a finished step
as one dim line. PASS lines are not printed there.

Anywhere else (a log file, a pipe, CHECK_PLAIN=1) the plain lines, which
scripts/tags.py `failures` and the person's own greps read:

    [chromium] http://127.0.0.1:41573/
       [chromium 1/3] load and 10 s idle …
      PASS page reported ready
       [chromium 1/3] step 1/2 boot …
      PASS the title screen is up
       [chromium 1/3] step 1/2 boot PASS (0.8 s)
       [chromium 1/3] done: PASS (28 s)

Either way the run ends with the script's own PASSED/FAILED lines, and the
counts go to $CHECK_DETAIL (a file scripts/checkrun.py reads) when it is set.
"""
import json
import os
import sys
import time

try:
    from rich.console import Console
    from rich.progress import BarColumn, MofNCompleteColumn, Progress, SpinnerColumn, TextColumn, TimeElapsedColumn
except ImportError:  # the plain lines only
    Progress = None


def live_wanted():
    return Progress is not None and sys.stdout.isatty() and os.environ.get("CHECK_PLAIN") != "1"


def fit(text, n=20):
    """A bar's step text cut to `n` columns, so the bar stays one line."""
    return text if len(text) <= n else text[: n - 1] + "…"


def secs(s):
    return f"{s:.1f} s" if s < 10 else f"{s:.0f} s"


def emit(kind, **fields):
    """A progress event for the dashboard (scripts/check-dashboard.py), when
    $CHECK_EVENTS names its file."""
    path = os.environ.get("CHECK_EVENTS")
    if path:
        with open(path, "a") as f:
            f.write(json.dumps({"k": kind, "at": time.time(), **fields}) + "\n")


class Report:
    """`engines`: their names; `steps`: the step names each engine runs."""

    def __init__(self, engines, steps):
        self.engines, self.steps = engines, steps
        self.passed = self.failed = 0
        self.engine = self.tag = None
        self.live = live_wanted()
        if self.live:
            # One line per bar, at most 80 columns (scripts/checkrun.py runs
            # this on a terminal that wide): a bar line that wraps stacks a
            # new line on every redraw.
            self.progress = Progress(
                SpinnerColumn(),
                TextColumn("{task.description:<14}"),
                BarColumn(bar_width=18),
                MofNCompleteColumn(),
                TextColumn("{task.fields[step]:<20}"),
                TimeElapsedColumn(),
                TextColumn("{task.fields[fails]}"),
                console=Console(highlight=False),
            )
            self.tasks = {e: self.progress.add_task(e, total=len(steps), start=False, step="waiting", fails="") for e in engines}
            self.overall = self.progress.add_task("overall", total=len(steps) * len(engines), step="", fails="")
            self.progress.start()

    # ------------------------------------------------------------------ out

    def say(self, text, style=None):
        """A line above the bars, or a plain line."""
        if self.live:
            self.progress.console.print(text, style=style, markup=False)
        else:
            print(text, flush=True)

    def check(self, ok, what):
        if ok:
            self.passed += 1
            if self.live:
                self.progress.update(self.tasks[self.engine], step=fit(f"{self.cur} · {self.passed + self.failed - self.engine_checks} checks"))
            else:
                print("  PASS " + what, flush=True)
            return
        self.failed += 1
        if self.live:
            self.say(f"  FAIL [{self.engine}] {what}", "bold red")
            self.progress.update(self.tasks[self.engine], fails=f"{self.failed - self.engine_failed} failed")
        else:
            print("  FAIL " + what, flush=True)

    def note(self, text):
        """A script's own aside (an engine's URL, a NOTE line)."""
        if not self.live:
            print(text, flush=True)

    # --------------------------------------------------------------- stages

    def engine_start(self, i, engine):
        self.engine, self.tag = engine, f"[{engine} {i}/{len(self.engines)}]"
        self.engine_began, self.engine_failed = time.monotonic(), self.failed
        self.engine_checks, self.cur = self.passed + self.failed, "starting"
        self.done = 0
        emit("bar", id=engine, label=engine, done=0, total=len(self.steps), note="starting")
        if self.live:
            self.progress.start_task(self.tasks[engine])

    def step_start(self, k, name, label=None):
        self.step_began = time.monotonic()
        self.step_failed = self.failed
        self.head = f"   {self.tag} step {k}/{len(self.steps)} {label or name}"
        self.cur = name
        emit("bar", id=self.engine, label=self.engine, done=self.done, total=len(self.steps), note=name)
        if self.live:
            self.progress.update(self.tasks[self.engine], step=fit(name))
        else:
            print(f"{self.head} …", flush=True)

    def step_end(self, error=False):
        took = time.monotonic() - self.step_began
        bad = self.failed - self.step_failed
        verdict = "ERROR (exception, below)" if error else f"FAIL ({bad} failed)" if bad else "PASS"
        self.done += 1
        emit("bar", id=self.engine, label=self.engine, done=self.done, total=len(self.steps), note=self.cur, bad=bool(bad or error))
        if self.live:
            self.say(f"{self.head} {verdict} ({secs(took)})", "red" if bad or error else "dim")
            self.progress.advance(self.tasks[self.engine])
            self.progress.advance(self.overall)
        else:
            print(f"{self.head} {verdict} ({secs(took)})", flush=True)

    def engine_end(self):
        bad = self.failed - self.engine_failed
        line = f"   {self.tag} done: {'PASS' if not bad else f'FAIL ({bad} failed)'} ({secs(time.monotonic() - self.engine_began)})"
        if self.live:
            self.progress.update(self.tasks[self.engine], step="done")
            self.say(line, "red" if bad else "green")
        else:
            print(line, flush=True)

    def close(self):
        if self.live:
            self.progress.stop()
        path = os.environ.get("CHECK_DETAIL")
        if path:
            with open(path, "w") as f:
                f.write(f"{self.passed} checks passed, {self.failed} failed")
