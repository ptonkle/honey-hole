# Check times

One row per run of `scripts/timed-checks.sh`, newest last. Seconds per step; `load` is the 1-minute load average when the run started.

| When | Version | Commit | Load | Check | Steps (s) | Total (s) | Result |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-10-17 18:58 | 0.1.0 | 4a0c92a+ | 0.35 | check (chromium+firefox+webkit) | format=0.1 lint=1.8 core=0.1 tags=0.1 release=4.0 WebAssembly=0.1 art=0.1 notebook=0.0 browser=33.8 | 40.5 | pass |
| 2026-10-17 19:05 | 0.1.0 | b3847cc | 2.38 | check (chromium+firefox+webkit) | format=0.1 lint=0.2 core=0.1 tags=0.1 release=0.5 WebAssembly=0.1 art=0.1 notebook=0.0 browser=35.0 | 36.4 | pass |
| 2026-10-17 19:06 | 0.1.0 | c5d05ba+ | 1.20 | quick (chromium; tags boundary, notes, whatsnew) | format=0.1 lint=0.2 list=0.3 core=0.0 tags=0.1 release=0.5 WebAssembly=0.1 notebook=0.0 browser=11.7 | 12.9 | pass |

## The browser checks' fast mode, before and after (SECOND-ORDER R6.90)

2026-10-18, run back to back on the same build except for `?fast&mute` (`tests/browser/check.py`, then `tests/browser/online.py` and `online_match.py` as `scripts/check-online.sh` runs them). The machine was shared: the load average is at each run's start. Seconds per step as `tests/browser/progress.py` printed them.

| Step | Chromium before | after | Firefox before | after | WebKit before | after |
| --- | --- | --- | --- | --- | --- | --- |
| load and 10 s idle | 14 | 13 | 11 | 13 | 11 | 11 |
| boot, settings, notes | 1.9 | 2.0 | 1.5 | 2.3 | 2.4 | 4.4 |
| play | 6.2 | 3.2 | 6.2 | 3.6 | 6.9 | 4.9 |
| gallery, sounds, lake | 2.5 | 3.1 | 2.5 | 3.0 | 7.2 | 6.0 |
| fishboy | 7.5 | 4.6 | 10 | 7.2 | 12 | 9.0 |
| minigames | 3.0 | 2.4 | 8.7 | 10 | 5.7 | 11 |
| fishing | 16 | 15 | 18 | 16 | 15 | 18 |
| items | 18 | 13 | 18 | 14 | 20 | 14 |
| ai | 3.7 | 0.8 | 4.1 | 1.2 | 4.7 | 1.6 |
| ai_items | 9.2 | 6.4 | 10.0 | 9.2 | 10 | 7.5 |
| match_flow | 17 | 6.0 | 21 | 8.1 | 24 | 8.8 |
| **engine total** | **99** | **70** | **112** | **87** | **118** | **97** |

| Check | Before (load 26) | After (load 14 to 22) |
| --- | --- | --- |
| `tests/browser/check.py`, three engines | 330 s | 255 s |
| `online.py` (transport; opens nettest.html, no game) | 28 s | 34 s |
| `online_match.py` (a match between two tabs) | 44 s | 41 s |

What each saving is: 3 s of countdown in every match started (play, fishboy, fishing, items twice, ai, ai_items, match_flow), the 3 s "Lines up" hold and the reveal's 5 to 15 s in match_flow, the Fish-Boy's raise in fishboy. The Minigames screen's banner is 1.4 s; its step's time is the 73 pointer moves round the reel, which Firefox and WebKit make slowly and the load swings.

What is still slow, and why it stays:
- **load and 10 s idle** (11 to 14 s an engine): BUILDER-SPEC D7 sets the 10 s idle; it watches for stray requests, so no state can replace it.
- **fishing** (15 to 18 s): the bite's 2 to 6 s wait, the reel and the landed hold are rule timings (`?debug=fishing` already shortens the hook window and holds the fish still).
- **items** (13 to 14 s): two page loads and two 40 m drives to a buoy at the boat's real acceleration.
- **ai_items** (6 to 9 s): a Hard AI's real drive to the buoy.
- No fixed sleep was left in the steps but the idle; the online match's 1.5 s sleep behind the match menu now waits for the HUD's clock to change.
- `BUILDER_FAST=0` (Chromium, play, fishboy, minigames, match_flow): passes, with the real countdown, "Lines up", reveal and banner checked.
