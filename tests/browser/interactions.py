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
starts and ends on the Title, so any of them can run alone after `boot`.
tests/tags.toml tags each step ([browser]); scripts/quick-check.sh runs only
the steps a change needs, through BUILDER_STEPS. A new step goes in STEPS and
in [browser], or `scripts/tags.py guard` fails.
"""

import json
import os
import pathlib

DATA = pathlib.Path(__file__).resolve().parents[2] / "data"
STRINGS = json.loads((DATA / "strings.json").read_text())


def by_id(action):
    """A selector for an element whose id is a dotted action name."""
    return f"[id='{action}']"


def screen(page, name):
    page.wait_for_selector(f"body[data-screen='{name}']", state="attached", timeout=5000)
    return page.get_attribute("body", "data-screen") == name


def focused(page):
    return page.evaluate("() => document.activeElement && document.activeElement.id")


def step_boot(page, check):
    check(screen(page, "title"), "the page opens on the Title")
    check(page.inner_text(".panel.title h2") == STRINGS["title"], "the Title shows the game's name")
    menu = page.locator(".panel.menu .button").all_inner_texts()
    check(
        menu == [STRINGS["menu.play"], STRINGS["menu.settings"], STRINGS["menu.notes"], STRINGS["menu.gallery"]],
        f"the Title's menu is Play, Settings, What's New, Gallery {menu}",
    )
    check(page.inner_text("#version") == "v0.1.0", "the version shows in the corner")
    check(page.inner_text("#message") == "", "no error message is shown")


def step_settings(page, check):
    # Settings: the volume slider, the text size select, Back
    page.click(by_id("menu.settings"))
    check(screen(page, "settings"), "Settings opens from the Title")
    slider = page.locator(by_id("settings.volume"))
    check(page.inner_text(by_id("settings.volume.text")) == "80%", "the volume starts at 80%")
    slider.focus()
    page.keyboard.press("ArrowLeft")
    page.wait_for_function("() => document.getElementById('settings.volume.text').textContent === '75%'", timeout=5000)
    check(page.inner_text(by_id("settings.volume.text")) == "75%", "one step down on the slider sets 75%")
    check(focused(page) == "settings.volume", "the slider keeps the focus while the menus redraw")
    page.select_option(by_id("settings.text_size"), "large")
    page.wait_for_selector("html[data-text-size='large']", state="attached", timeout=5000)
    check(page.get_attribute("html", "data-text-size") == "large", "Large text size reaches the page")
    buttons = page.locator(".panel .button").all_inner_texts()
    check(buttons[-1] == STRINGS["menu.back"], "Settings ends with Back")
    # by keyboard, so the button has the focus in every engine (WebKit does
    # not focus a clicked button)
    page.locator(by_id("menu.back")).focus()
    page.keyboard.press("Enter")
    check(screen(page, "title"), "Back goes to the Title")
    check(focused(page) == "menu.play", "the new screen's first button takes the focus")

    # Escape from a sub-screen
    page.click(by_id("menu.settings"))
    screen(page, "settings")
    check(page.inner_text(by_id("settings.volume.text")) == "75%", "the volume set earlier is kept")
    page.keyboard.press("Escape")
    check(screen(page, "title"), "Escape on Settings goes to the Title")


def step_notes(page, check):
    # What's New, from its button and from the version
    page.click(by_id("menu.notes"))
    check(screen(page, "notes"), "What's New opens from the Title")
    versions = page.locator(".note-version").all_inner_texts()
    check(versions == ["v0.1.0"], f"What's New lists version 0.1.0 {versions}")
    page.click(by_id("notes.minor"))
    page.wait_for_selector(f"{by_id('notes.minor')}:checked", state="attached", timeout=5000)
    check(page.is_checked(by_id("notes.minor")), "the minor versions toggle ticks")
    page.click(by_id("menu.back"))
    screen(page, "title")
    page.click("#version")
    check(screen(page, "notes"), "the version in the corner opens What's New")
    page.keyboard.press("Escape")
    check(screen(page, "title"), "Escape on What's New goes to the Title")


def step_play(page, check):
    # Play and its pause menu
    page.click(by_id("menu.play"))
    check(screen(page, "play"), "Play opens from the Title")
    check(page.is_disabled("#version"), "the version does not open What's New from Play")
    check(page.locator(".panel.pause").count() == 0, "Play opens with the pause menu closed")
    page.keyboard.press("Escape")
    page.wait_for_selector(".panel.pause", timeout=5000)
    check(page.inner_text(".panel.pause h2") == STRINGS["pause.heading"], "Escape in Play opens the pause menu")
    page.keyboard.press("Escape")
    page.wait_for_selector(".panel.pause", state="detached", timeout=5000)
    check(page.locator(".panel.pause").count() == 0, "Escape again closes it")
    page.click(by_id("menu.pause"))
    page.wait_for_selector(".panel.pause", timeout=5000)
    page.click(by_id("menu.resume"))
    page.wait_for_selector(".panel.pause", state="detached", timeout=5000)
    check(screen(page, "play"), "Resume closes the pause menu and stays in Play")
    page.keyboard.press("Escape")
    page.click(by_id("menu.quit"))
    check(screen(page, "title"), "Quit to Title goes to the Title")
    check(page.inner_text("#message") == "", "no error message after the round trip")


# The steps in order, by name; `boot` always runs.
STEPS = [("boot", step_boot), ("settings", step_settings), ("notes", step_notes), ("play", step_play)]


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
