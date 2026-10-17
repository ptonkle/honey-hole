#!/usr/bin/env python3
"""Rule 6 as a script: break each behaviour a test covers, run that test,
require it to fail, and put the code back. A test that still passes with its
behaviour broken does not cover it. (After Long Time Comin' and Lowballer.)

    python3 scripts/break_tests.py              every break in tests/breaks.json
    python3 scripts/break_tests.py title intro  the breaks whose names hold these words

tests/breaks.json is a list of objects:

    {"name": "an empty title is refused",
     "file": "crates/core/src/page.rs",
     "find": "if strings.title.trim().is_empty() {",
     "replace": "if false {",
     "test": "an_empty_title_is_rejected_by_name"}

The file is restored after each break, whatever happens. The script exits
non-zero when a break leaves its test passing, when its text is no longer in
its file (the break has rotted), or when the list is empty (a proof with
nothing in it proves nothing). It is slow on a large list, so it runs outside
the everyday check; run it after adding tests and before a handoff.
"""
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
LIST = ROOT / "tests" / "breaks.json"


def main(words):
    breaks = json.loads(LIST.read_text())
    if words:
        breaks = [b for b in breaks if any(w in b["name"] for w in words)]
    if not breaks:
        sys.exit("no breaks to run: the proof would prove nothing")
    bad = []
    for b in breaks:
        path = ROOT / b["file"]
        original = path.read_text()
        if original.count(b["find"]) != 1:
            print(f"  ROTTED  {b['name']}: the text to break appears {original.count(b['find'])} times in {b['file']}")
            bad.append(b["name"])
            continue
        try:
            path.write_text(original.replace(b["find"], b["replace"]))
            run = subprocess.run(
                ["cargo", "test", "--workspace", "--quiet", b["test"]],
                cwd=ROOT, capture_output=True, text=True,
            )
        finally:
            path.write_text(original)
        if run.returncode == 0:
            print(f"  MISSED  {b['name']}: {b['test']} still passes with the behaviour broken")
            bad.append(b["name"])
        else:
            print(f"  caught  {b['name']}")
    print(f"\n{len(breaks) - len(bad)} of {len(breaks)} breaks caught")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main(sys.argv[1:])
