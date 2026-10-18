# Handoff

Stage 9, written by the agent. `C5 Consolidate in your own words`. A fresh session starts from this file, not from memory.

## What this is

Five sentences at most: what the tool does, for whom, and where it lives.

## Rules that hold it up

The rules that break silently if a later change ignores them, and the test that guards each.

## Shape of the code

Where each kind of thing lives, in a short table.

## Commands

Build, test, check, publish, and the probes, with what each prints when it passes.

## What will bite within the hour

What a new session gets wrong first.

## Open for the person

Every `SECOND-ORDER.md` row with status *person*, by number, with what the person has to decide or supply. `scripts/notebook_check.py` fails while one is missing.

- R1.23: whether settings are kept between visits (browser storage, D17). Today they last for the visit.
- R1.63: the fog's sight distance (120 m assumed), which the map validator's hiding rule depends on.
- R1.69: the fish roster and how the example counts split over it.
- R1.30: the game's name ("Fishing" is the working name), and a read of the menu strings and text size choices the agent wrote.
- R1.107: a read of the eleven lake prop names the Gallery shows (listed at the foot of `SECOND-ORDER.md`), and the props' look (`notes/shots/models/props/`).
- R1.48: the draft fish roster and their names (seven fish, Midwest lakes), shown in the Gallery.
- R1.54: the four player colours and the Gallery's light in `data/look.json`.
- R1.160: the fish's new shapes from above and their new base lengths (R1.153, R1.154), in `notes/shots/models/fish-v2/`.

## Mistakes so far

By system, from `SECOND-ORDER.md`, with the rule each one produced.

## What passed

The last lines of `./scripts/check.sh` and `./scripts/count-tests.sh`, with the date and commit.

2026-10-17, the menu machinery port (branch `worktree-agent-aaf1b7a85a5536b76`): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `count-tests.sh`: 58 passed, 0 failed, 0 ignored; `scripts/break_tests.py`: 25 of 25 breaks caught.

2026-10-17, the model format, renderer, Gallery and the seven draft fish, merged with main (maps, boat, angler, buoy) (branch `worktree-agent-ae7232570ef10b3c9`): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `count-tests.sh`: 129 passed, 0 failed, 0 ignored; `scripts/break_tests.py`: 82 of 82 breaks caught.

## The next action

One action.

2026-10-17, map generation (branch `worktree-agent-a13a60c04e6220e7c`, after df1b4b7): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `count-tests.sh`: 82 passed, 0 failed, 0 ignored; `scripts/break_tests.py`: 53 of 53 breaks caught.
