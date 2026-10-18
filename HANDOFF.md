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
- R1.30: the game's name ("Fishing" is the working name), and a read of the menu strings and text size choices the agent wrote.

## Mistakes so far

By system, from `SECOND-ORDER.md`, with the rule each one produced.

## What passed

The last lines of `./scripts/check.sh` and `./scripts/count-tests.sh`, with the date and commit.

2026-10-17, the menu machinery port (branch `worktree-agent-aaf1b7a85a5536b76`): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `count-tests.sh`: 58 passed, 0 failed, 0 ignored; `scripts/break_tests.py`: 25 of 25 breaks caught.

## The next action

One action.
