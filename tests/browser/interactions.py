"""The project's own browser interactions, run by check.py in every engine.

check.py has already loaded the page, waited for it to report ready, and let
it idle. Drive the tool here the way a person would, and assert what the page
shows with `check(condition, "what this shows")`. Assert the effect a person
would see (the text drawn, the file saved), not the markup that should cause it.

Expected values in these checks are worked out by hand from the brief (C4),
never copied from the tool's own output. Wait on a selector or a data-*
attribute the page sets, never on a fixed time: on CI the page may draw with
a CPU and run slowly.

The interactions are steps, `step_<name>(page, check)`, listed in STEPS. Each
starts and ends on the start screen, so any of them can run alone after
`boot`. tests/tags.toml tags each step ([browser]); scripts/quick-check.sh
runs only the steps a change needs, through BUILDER_STEPS. A new step goes in
STEPS and in [browser], or `scripts/tags.py guard` fails.
"""

import json
import os
import pathlib

STRINGS = json.loads((pathlib.Path(__file__).resolve().parents[2] / "data" / "strings.json").read_text())


def step_boot(page, check):
    check(page.inner_text("#title") == STRINGS["title"], "the title from data/strings.json is drawn")
    check(page.inner_text("#intro p") == STRINGS["intro"], "the intro from data/strings.json is drawn")
    check(page.get_attribute("body", "data-screen") == "start", "the core's start screen is shown")
    check(page.inner_text("#message") == "", "no error message is shown")


# The steps in order, by name; `boot` always runs.
STEPS = [("boot", step_boot)]


def chosen_steps():
    """The steps this run takes: all, or `boot` and those in BUILDER_STEPS."""
    wanted = os.environ.get("BUILDER_STEPS")
    chosen = None if wanted is None else set(wanted.split()) | {"boot"}
    return [(n, s) for n, s in STEPS if chosen is None or n in chosen]


def run(page, check, engine, report, first=1):
    """Runs the chosen steps; `report` (tests/browser/progress.py) shows each
    one start and end. `first` is the first step's number in the engine's
    count (the check's own load step comes before)."""
    steps = chosen_steps()
    skipped = [n for n, _ in STEPS if n not in dict(steps)]
    if skipped:
        report.note(f"  (skipping {len(skipped)} steps not picked: {', '.join(skipped)})")
    for k, (name, step) in enumerate(steps, first):
        report.step_start(k, name)
        try:
            step(page, check)
        except BaseException:
            report.step_end(error=True)
            raise
        report.step_end()
