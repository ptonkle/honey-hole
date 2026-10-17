#!/usr/bin/env python3
"""Check that SECOND-ORDER.md can be read the way the procedure reads it.

The notebook is read by the person, not by a test, so this checks its form
and nothing about its content:

- every row in the notebook's table has a round number (R1.1, R2.3), a kind
  (divergence, finding or worklist) and a status (done, open or person);
- no number is used twice, so a commit or a triage answer that cites one
  means one row;
- every row whose status is *person* is named in HANDOFF.md, so an item the
  person must do reaches the file they are told to read;
- once PLAN.md holds a decision, the notebook holds at least one row. A plan
  is made of choices the brief did not fix, so a started build with an empty
  notebook has decided things the person cannot see.

A row whose cells are all empty after the number is the template's blank
row and is skipped.

It also prints, without failing, how many of the project's commits that
changed code also changed the notebook. A notebook that changes only in the
last commit was probably written afterwards; that is for the person to read,
not for a check to decide.

    python3 scripts/notebook_check.py
"""

import re
import subprocess
import sys
from pathlib import Path

KINDS = {"divergence", "finding", "worklist"}
STATUSES = {"done", "open", "person"}
NUMBER = re.compile(r"R[1-9]\d*\.[1-9]\d*")
CODE = ["crates", "web", "data", "tests"]


def cells(line):
    return [c.strip() for c in line.strip().strip("|").split("|")]


def rows(text):
    """Yield (number, kind, status) for each row of the notebook's table."""
    header = None
    for line in text.splitlines():
        if not line.lstrip().startswith("|"):
            header = None
            continue
        c = cells(line)
        if header is None:
            if "Kind" in c and "Status" in c:
                header = c
            continue
        if set("".join(c)) <= set("-: "):
            continue
        c += [""] * (len(header) - len(c))
        if not any(c[1:]):
            continue
        yield (
            c[0],
            c[header.index("Kind")].strip("*_ ").lower(),
            c[header.index("Status")].strip("*_ ").lower(),
        )


def plan_has_a_decision(root):
    plan = root / "PLAN.md"
    if not plan.exists():
        return False
    return any(decision for _, decision in table(plan.read_text(), "Decision"))


def table(text, column):
    """Yield (first cell, named cell) for each filled row of the table with that column."""
    header = None
    for line in text.splitlines():
        if not line.lstrip().startswith("|"):
            header = None
            continue
        c = cells(line)
        if header is None:
            header = c if column in c else None
            continue
        if set("".join(c)) <= set("-: "):
            continue
        c += [""] * (len(header) - len(c))
        yield c[0], c[header.index(column)]


def history(root):
    """How many commits that changed code also changed the notebook, or None."""
    def git(*args):
        return subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True)
    top = git("rev-parse", "--show-toplevel")
    if top.returncode != 0 or Path(top.stdout.strip()).resolve() != root.resolve():
        return None
    code = set(git("log", "--format=%H", "--", *CODE).stdout.split())
    notes = set(git("log", "--format=%H", "--", "SECOND-ORDER.md").stdout.split())
    return len(code & notes), len(code)


def main(root):
    root = Path(root)
    notebook = (root / "SECOND-ORDER.md").read_text()
    handoff = (root / "HANDOFF.md").read_text()
    problems, seen = [], set()
    for number, kind, status in rows(notebook):
        name = number or "a row with no number"
        if not number:
            problems.append("SECOND-ORDER.md: a row has no number")
        elif not NUMBER.fullmatch(number):
            problems.append(f"SECOND-ORDER.md: {number} is not a round number; use R1.1, R1.2, then R2.1 after the first triage")
        elif number in seen:
            problems.append(f"SECOND-ORDER.md: {number} is used for two rows")
        seen.add(number)
        if kind not in KINDS:
            problems.append(f"SECOND-ORDER.md: {name} has kind {kind!r}; use divergence, finding or worklist")
        if status not in STATUSES:
            problems.append(f"SECOND-ORDER.md: {name} has status {status!r}; use done, open or person")
        if status == "person" and number and not re.search(rf"(?<![\w.]){re.escape(number)}(?![\w]|\.\d)", handoff):
            problems.append(f"SECOND-ORDER.md: {number} is the person's to do and HANDOFF.md does not name it")
    if not seen and plan_has_a_decision(root):
        problems.append("SECOND-ORDER.md has no rows, but PLAN.md holds decisions: record each choice the brief did not fix, as it is made")
    for p in problems:
        print(f"FAIL {p}")
    if problems:
        return 1
    print(f"notebook rows: {len(seen)}, each with a kind and a status; person rows named in HANDOFF.md")
    h = history(root)
    if h and h[1]:
        print(f"notebook: changed in {h[0]} of the {h[1]} commits that changed code (for the person to read, not a pass mark)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent))
