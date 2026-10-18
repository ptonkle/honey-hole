#!/usr/bin/env python3
"""A full-screen view of a running check, for a person to watch.

    check-dashboard.py <events.jsonl> <log> [name]

scripts/checkrun.py and tests/browser/progress.py append progress events
to <events> ($CHECK_EVENTS); the check's plain output goes to <log>. Each
refresh draws the whole screen again from those two files at the window's
size now (a full-screen app, like htop), so moving the window, resizing it
or taking it to a monitor with other scaling only changes the next frame:
nothing is kept on screen to wrap or stack. The view only reads the files,
so closing it never stops the check. When the check ends it shows the
verdict, counts down 5 s and exits (the person, 2026-10-17).
"""

import json
import pathlib
import re
import sys
import time

from rich.console import Console, Group
from rich.live import Live
from rich.progress_bar import ProgressBar
from rich.table import Table
from rich.text import Text

CLOSE_AFTER = 5
ANSI = re.compile(r"\x1b\[[0-9;?]*[A-Za-z]|\x1b\][^\x07]*\x07")


def secs(s):
    return f"{s:.1f} s" if s < 10 else f"{s:.0f} s"


class State:
    def __init__(self, events):
        self.path, self.offset, self.part = pathlib.Path(events), 0, ""
        self.t0 = time.time()
        self.stages = []  # dicts: n, label, est, start, ok, secs, detail
        self.bars = {}  # id -> event
        self.end = None

    def read(self):
        try:
            with open(self.path) as f:
                f.seek(self.offset)
                chunk = f.read()
                self.offset = f.tell()
        except OSError:
            return
        lines = (self.part + chunk).split("\n")
        self.part = lines.pop()
        for line in lines:
            try:
                e = json.loads(line)
            except ValueError:
                continue
            self.apply(e)

    def apply(self, e):
        k = e.get("k")
        if k == "begin":
            self.t0 = e["at"]
        elif k == "stage":
            self.stages.append({"n": e["n"], "label": e["label"], "est": e.get("est"), "start": e["at"], "ok": None})
            self.bars = {}
        elif k == "done":
            for s in reversed(self.stages):
                if s["label"] == e["label"] and s["ok"] is None:
                    s.update(ok=e["ok"], secs=e["secs"], detail=e.get("detail", ""))
                    break
        elif k == "bar":
            self.bars[e["id"]] = e
        elif k == "end":
            self.end = e


def tail(log, n):
    try:
        lines = pathlib.Path(log).read_text(errors="replace").splitlines()
    except OSError:
        return []
    lines = (ANSI.sub("", l) for l in lines)
    return [l for l in lines if l.strip() and not l.startswith("EXIT ")][-n:]


def render(st, log, name, width, height, closing=None):
    now = time.time()
    rows = []
    title = Text(f" {name}", style="bold")
    title.append(f"   {secs(now - st.t0)}", style="dim")
    rows.append(title)
    rows.append(Text(""))

    # fixed widths: a ratio column took the time column's room and cut it
    # to "0.1…"; the label is what gets cut on a narrow window
    took_w = 16
    table = Table.grid(padding=(0, 1))
    table.add_column(width=2)
    table.add_column(width=max(10, width - took_w - 6), no_wrap=True, overflow="ellipsis")
    table.add_column(width=took_w, justify="right", no_wrap=True)
    for s in st.stages:
        if s["ok"] is None:
            mark, style = "▶", "bold yellow"
            took = secs(now - s["start"]) + (f" / ~{secs(s['est'])}" if s.get("est") else "")
        else:
            mark, style = ("✓", "green") if s["ok"] else ("✗", "bold red")
            took = secs(s["secs"])
        label = s["label"] + (f": {s['detail']}" if s.get("detail") and s["ok"] is not None else "")
        table.add_row(Text(mark, style=style), Text(label, style="" if s["ok"] is not False else "red"), Text(took, style="dim"))
    rows.append(table)

    bar_w = max(10, min(40, width - 40))
    if st.bars and not st.end:
        rows.append(Text(""))
        bars = Table.grid(padding=(0, 1))
        bars.add_column(width=10, no_wrap=True)
        bars.add_column(width=bar_w)
        bars.add_column(no_wrap=True, overflow="ellipsis", ratio=1)
        for b in st.bars.values():
            total = max(1, b.get("total") or 1)
            bar = ProgressBar(total=total, completed=b.get("done", 0), width=bar_w,
                              complete_style="red" if b.get("bad") else "green")
            note = f"{b.get('done', 0)}/{total} {b.get('note', '')}"
            bars.add_row(Text(b.get("label", b["id"]), style="bold"), bar, Text(note))
        rows.append(bars)

    if st.end:
        rows.append(Text(""))
        ok = st.end.get("code") == 0
        verdict = Text(" ✓ PASSED " if ok else f" ✗ FAILED (exit {st.end.get('code')}) ", style="bold white on green" if ok else "bold white on red")
        if closing is not None:
            verdict.append(f"   closing in {closing} s", style="dim")
        rows.append(verdict)

    used = sum(1 for _ in rows) + len(st.stages) + len(st.bars) + 3
    room = max(3, height - used)
    rows.append(Text(""))
    rows.append(Text(" recent output", style="dim"))
    for line in tail(log, room):
        style = "red" if "FAIL" in line or "✗" in line else "dim"
        rows.append(Text(line, style=style, no_wrap=True, overflow="ellipsis"))
    return Group(*rows)


def main(argv):
    if len(argv) < 2:
        sys.exit(__doc__)
    events, log = argv[0], argv[1]
    name = argv[2] if len(argv) > 2 else "check"
    console = Console()
    st = State(events)
    console.set_window_title(f"{name} — view only, safe to close")
    with Live(console=console, screen=True, auto_refresh=False, transient=True) as live:
        while True:
            st.read()
            w, h = console.size
            live.update(render(st, log, name, w, h), refresh=True)
            if st.end:
                break
            time.sleep(0.25)
        console.set_window_title(f"{name} — finished, closing")
        for left in range(CLOSE_AFTER, 0, -1):
            for _ in range(4):
                st.read()
                w, h = console.size
                live.update(render(st, log, name, w, h, closing=left), refresh=True)
                time.sleep(0.25)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
