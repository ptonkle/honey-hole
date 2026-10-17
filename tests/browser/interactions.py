"""The project's own browser interactions, run by check.py in every engine.

check.py has already loaded the page, waited for it to report ready, and let
it idle. Drive the tool here the way a person would, and assert what the page
shows with `check(condition, "what this shows")`. Assert the effect a person
would see (the text drawn, the file saved), not the markup that should cause it.

Expected values in these checks are worked out by hand from the brief (C4),
never copied from the tool's own output. Wait on a selector or a data-*
attribute the page sets, never on a fixed time: on CI the page may draw with
a CPU and run slowly.
"""

import json
import pathlib

STRINGS = json.loads((pathlib.Path(__file__).resolve().parents[2] / "data" / "strings.json").read_text())


def run(page, check, engine):
    check(page.inner_text("#title") == STRINGS["title"], "the title from data/strings.json is drawn")
    check(page.inner_text("#intro p") == STRINGS["intro"], "the intro from data/strings.json is drawn")
    check(page.get_attribute("body", "data-screen") == "start", "the core's start screen is shown")
    check(page.inner_text("#message") == "", "no error message is shown")
