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

- R5.78: the menus' look (cream cards, the drawn "Honey Hole" wordmark, the lake backdrop's camera and boats), in `notes/shots/menus/`.
- R1.63: the fog's sight distance (120 m assumed), which the map validator's hiding rule depends on.
- R2.100: the sound labels on the Sounds screen (`label` in `data/sounds.json`), a generated list a reader sees.
- R1.69: the fish roster and how the example counts split over it.
- R1.30: the game's name ("Fishing" is the working name), and a read of the menu strings and text size choices the agent wrote.
- R1.107: a read of the eleven lake prop names the Gallery shows (listed at the foot of `SECOND-ORDER.md`), and the props' look (`notes/shots/models/props/`).
- R1.48: the draft fish roster and their names (seven fish, Midwest lakes), shown in the Gallery.
- R1.54: the four player colours and the Gallery's light in `data/look.json`.
- R2.69: the fish's base lengths and values in `data/fish.json` (the 13 names and tiers are the person's).
- R2.75: the radar's brighter player colours in `data/ui2d.json` (`radar.players`).
- R1.149: the remodelled angler's look (tapered limbs, round head, no boxes), in `notes/shots/models/angler-v2/`.
- R2.7: the boat's handling numbers the brief does not name (reverse, braking, coasting, slip, turning, swivel, nitrous), in `data/tuning.json`; try them by driving.
- R4.45: the eased rare and legendary reel fights (`fishing.reel` in `data/tuning.json`), PLAN.md V38.
- R4.47: ramming pays several times better than fishing in AI playtests (`notes/playtests.md`); which rule to change, if any (PLAN.md V39).
- R4.49: whether B4's "leggendaries ar not announced for 20 seconds" means a legendary's hook popup comes 20 s late.
- R5.9: the ramming fix applied overnight (a legendary's popup 20 s late, a rammed-off rare or legendary swims 80 m); keep it, pick another option, or undo it with `balance.announce_delay_ms.legendary` 0 (`notes/balance-options.md`, PLAN.md V-balance).
- R5.5: whether a reeling boat should be harder to wreck and not pushed by bumps (a change to the 80% rule; measured only).
- R5.6: whether a ram should cost the rammer its held item (needs the items work; not built).
- R2.11: the twelve draft AI names in `data/names.json`.
- R2.21: the match setup's length choices (4, 8, 12 min) and buoy intervals (30 s to 2 min).
- R3.44: whether two players both behind strict NATs must be able to connect (a TURN server: a server or a paid service, and a new origin).
- R3.57: a try of online play by room code between two machines (the relay path no automatic check runs).
- R1.160: the fish's new shapes from above and their new base lengths (R1.153, R1.154), in `notes/shots/models/fish-v2/`.
- R2.116: the six more roster fish's look from the side and from above, in `notes/shots/models/fish-more/` (swims in `notes/shots/models/swims/`).
- R2.117: a read of their six names (listed at the foot of `SECOND-ORDER.md`).
- R2.136: all thirteen fish reshaped to their species from above, the side and 3/4, in `notes/shots/models/fish-v3/` (`before/` holds the same views before; side changes listed in R2.132).
- R3.93: the six item minigames' numbers and looks: play each from the Title, Minigames (shots in `notes/shots/minigames/`); numbers in `data/tuning.json` `minigames`, colours in `data/ui2d.json` `minigames`.
- R2.34: maples have no model; they draw as birches or tall spruces (`lake.props.maple` in `data/look.json`). A maple model, or keep it so.
- R3.20: the fishing's look (bobber, lure, line, underwater murk, picture-in-picture, HUD meters and popups), in `notes/shots/fishing/`.
- R4.97: the channel catfish v4 (broad flat head, thick body, short forward dorsal, forked tail), in `notes/shots/models/fish-v4/`.
- R4.80: the lower chase camera, in `notes/shots/polish/` (3 to 5c).
- R4.93: the underwater look, in `notes/shots/polish/` (1 to 2b).
- R3.123: the match flow's look (the HUD, popups, countdown, wreck burst, ghosted boat, postgame reveal and ranking), in `notes/shots/match-flow/`; every colour and timing is in `data/flow.json`.
- R5.31: R4.11's fix, the reel holds still while an item's minigame plays (`items.reel_holds_in_minigame`; PLAN.md V52).
- R5.33: the AI's hand on the minigames, by difficulty (`ai.items.hand`; PLAN.md V53).
- R5.40: items in AI playtests (camera strongest, Nitrous, Teleporter and Cloaking Tech worth about nothing, the winner's margin wider); whether to let the boat keep its throttle during a minigame, shorten Nitrous's, or lower the camera's weight (`notes/playtests.md`, "Results with items"; PLAN.md V54).
- R4.26: the items' look, words and numbers (`notes/shots/items/`; try them with `?debug=items,item.<key>`).
- R2.43: the boat camera's distance, height, pitch and field of view (`lake.camera` in `data/look.json`), and the lake's look in `notes/shots/world/`; the fog distance can be tried live in the Lake preview.
- R6.38: the line's new drag tick (hear it on the Sounds screen: Line tension on, slide the tension), its rate and loudness (`audio.line_tension` in `data/tuning.json`).
- R6.12: MSAA stays on; an edge-smoothing setting would help weaker machines (the frame-time measurements are in `MEASUREMENTS.md`, "A full match in the browser"; PLAN.md V-perf).

## Mistakes so far

By system, from `SECOND-ORDER.md`, with the rule each one produced.

## What passed

The last lines of `./scripts/check.sh` and `./scripts/count-tests.sh`, with the date and commit.

2026-10-18, the KISS pass, stage 2 (the line's drag tick, copy limits, the postgame's Catches; branch `worktree-agent-a9818ee19434f1aea`, merged with main at 552c22b): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `count-tests.sh`: 470 passed, 0 failed, 0 ignored; `break_tests.py` on the 11 new or changed rows: 11 of 11 caught.

2026-10-18, online: items shared, other players' lines drawn, hook popups at the host's delay, fish places packed (branch `online2`, worktree `.claude/worktrees/online2`, from main at 0389fa6): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `ONLINE CHECK PASSED` (chromium, firefox, webkit); `count-tests.sh`: 444 passed, 0 failed, 0 ignored; `scripts/break_tests.py "online items" "online fishing"`: every row caught.

2026-10-18, the balance knobs for ramming, with a legendary popup 20 s late and an 80 m swim applied (`notes/balance-options.md`), merged with main at dc644e7 (online play) (branch `worktree-agent-a0102cd6c05bc4610`): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `count-tests.sh`: 415 passed, 0 failed, 0 ignored; `break_tests.py` on the 10 new rows: 10 of 10 caught.

2026-10-18, online play, both halves (transport, lobby, boats, wrecks, timer; fishing shared by claims; branch `online`, worktree `.claude/worktrees/online-redo`, after d5a813b, merged with main at dad627e): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `./scripts/check-online.sh`: `ONLINE CHECK PASSED` (chromium, firefox, webkit); `count-tests.sh`: 369 passed, 0 failed, 0 ignored; `scripts/break_tests.py online boundary:`: every new break caught (24 + 5 for shared fishing).

2026-10-18, online play merged with the match flow and the AI (main at 3895a04; branch `online`): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `ONLINE CHECK PASSED` (chromium, firefox, webkit); `count-tests.sh`: 407 passed, 0 failed, 0 ignored; `scripts/break_tests.py "online flow"`: 3 of 3 caught.

2026-10-17, the menu machinery port (branch `worktree-agent-aaf1b7a85a5536b76`): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `count-tests.sh`: 58 passed, 0 failed, 0 ignored; `scripts/break_tests.py`: 25 of 25 breaks caught.

2026-10-17, the model format, renderer, Gallery and the seven draft fish, merged with main (maps, boat, angler, buoy) (branch `worktree-agent-ae7232570ef10b3c9`): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `count-tests.sh`: 129 passed, 0 failed, 0 ignored; `scripts/break_tests.py`: 82 of 82 breaks caught.

2026-10-17, the angler remodelled without boxes (branch `worktree-agent-a6cc1aec971e299a0`, after f39df0b): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `count-tests.sh`: 138 passed, 0 failed, 0 ignored; `scripts/break_tests.py`: 90 of 90 breaks caught.
2026-10-17, the item buoy as a giant bobber (branch `worktree-agent-aedf5b313381339fe`, after f08c629): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `count-tests.sh`: 136 passed, 0 failed, 0 ignored; `scripts/break_tests.py`: 88 of 88 breaks caught.
2026-10-17, the true-size line-up and the fish shapes from above (branch `worktree-agent-a28fb9a4d934aae73`, after 8a848c5): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit, 144 checks); `count-tests.sh`: 138 passed, 0 failed, 0 ignored; `scripts/break_tests.py line-up`: 4 of 4 new breaks caught.
2026-10-17, six more roster fish (branch `worktree-agent-ad8a32d5f83ca73c9`, after 443a6fd): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `count-tests.sh`: 173 passed, 0 failed, 0 ignored; `scripts/break_tests.py roster panic sampled`: 3 of 3 new breaks caught.
2026-10-18, all thirteen fish reshaped to their species (branch `worktree-agent-a224ab628fd9e4a46`, after 87c3d87, merged with main): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit, 222 checks); `count-tests.sh`: 231 passed, 0 failed, 0 ignored; no new behaviour tests (model data only).
2026-10-17, the match core (Match, boats, collisions, wrecks, respawn, timer, tuning.json, controls.json, New match, Play with a stand-in lake) (branch `worktree-agent-a0098b4853bad3d19`, after 650e51e): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `count-tests.sh`: 208 passed, 0 failed, 0 ignored; `scripts/break_tests.py`: 124 of 124 breaks caught before the merge with the sound engine.

2026-10-17, the screen 2D layer and the live Fish-Boy (radar, Fishpedia, My catch), merged with main (branch `worktree-agent-a0af07e03570f7fa8`, after 843a370): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit, 207 checks); `count-tests.sh`: 203 passed, 0 failed, 0 ignored; `scripts/break_tests.py` on the 38 new breaks: 38 of 38 caught.
2026-10-17, the 3D world of Burt Lake (terrain, water, fog, props, instancing, cameras, the Lake preview, a match drawn on the lake, glowing colours), merged with main (branch `worktree-agent-a2ff84d9ac75f0c6e`): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `count-tests.sh`: 231 passed, 0 failed, 0 ignored; `scripts/break_tests.py` on the world's 18 breaks: all caught.

2026-10-18, the Fish-Boy integrated with the match core and the lake world, merged with main (branch `worktree-agent-a0af07e03570f7fa8`): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `count-tests.sh`: 265 passed, 0 failed, 0 ignored; `scripts/break_tests.py`: 218 of 218 breaks caught.

2026-10-18, fishing (fish in the lake, the cast, nibbles, the hook window, the reel, catches, the underwater camera and picture-in-picture, the fishing HUD), merged with main (branch `worktree-agent-a619c0b6bfe7ceb22`): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit, 291 checks, with the new `fishing` step); `count-tests.sh`: 300 passed, 0 failed, 0 ignored; `scripts/break_tests.py` on the 34 new breaks: 34 of 34 caught (and the map row "one schooling fish is refused" repaired).

2026-10-18, the match flow (countdown, the 2D HUD with the fishing's meters, data-driven popups, wreck burst and shake, ghosting, the end, the postgame reveal and ranking, Rematch), merged with main (fishing, item minigames) (branch `worktree-agent-a9be3869944870630`): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit, 357 checks, with the new `match_flow` step); `count-tests.sh`: 367 passed, 0 failed, 0 ignored; `scripts/break_tests.py` on the 23 new and changed breaks: 23 of 23 caught.

2026-10-18, the menus in the HUD's family (wordmark, the lake backdrop, the Workshop, cream cards, settings kept between visits, mouse sensitivity and invert-Y, What's New 0.2.0), merged with main through 19e263d (branch `worktree-agent-acc344f60edb47916`): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit, 420 checks); `count-tests.sh`: 454 passed, 0 failed, 0 ignored; `scripts/break_tests.py` on the 9 new breaks: 9 of 9 caught.

2026-10-18, the AI players and headless playtests (`crates/core/src/ai/`, `examples/playtest.rs`, `notes/playtests.md`), merged with main (match flow, minigames, polish) (branch `ai`): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit, 372 checks, with the new `ai` step); `count-tests.sh`: 380 passed, 0 failed, 0 ignored; `scripts/break_tests.py` on the 10 new breaks and the restored cue delay: 11 of 11 caught.

2026-10-18, items (the buoy's clock, warning, beats and fair point; pickups; Shift and the six minigames; the six effects; claim functions; HUD, icons, popups, world effects; `?debug=items`), merged with main through dc644e7 (online play) (branch `items`): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit, 405 checks, with the new `items` step); `count-tests.sh`: 431 passed, 0 failed, 0 ignored; `scripts/break_tests.py items:`: 29 of 29 caught.

2026-10-18, the AI uses items (the buoy, each item's moment, a hand that plays the six minigames through the pointer), R4.11's fix (the reel holds while a minigame plays) and the playtests with items, merged with main's online items (branch `worktree-agent-abdb694ecf26104d8`): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit, with the new `ai_items` step); `count-tests.sh`: 455 passed, 0 failed, 0 ignored; `break_tests.py` caught all 11 new breaks.

2026-10-18, the performance pass (one AI path search a tick, models out of view not posed, the frame's lists read in place, unchanged view model and 2D list not redrawn, the sky drawn last, no stray 404s), merged with main (branch `worktree-agent-a9dc849f7fd273052`): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit, now failing on any 404); `ONLINE CHECK PASSED`; `count-tests.sh`: 467 passed, 0 failed, 0 ignored; `scripts/break_tests.py R6.4 R6.5`: 2 of 2 breaks caught.

2026-10-18, the retro outdoors magazine look (menus, masthead, HUD cards, popups, postgame, minigame frames, tab icon; the KISS pass's unused look keys deleted), merged with main's KISS stage 2 (branch `worktree-agent-a1802f7afa7098030`): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `count-tests.sh`: 471 passed, 0 failed, 0 ignored; `scripts/break_tests.py "dot corner"`: 1 of 1 caught. Person: R6.69 (the look, `notes/shots/retro/`).

## The next action

One action.

2026-10-17, map generation (branch `worktree-agent-a13a60c04e6220e7c`, after df1b4b7): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `count-tests.sh`: 82 passed, 0 failed, 0 ignored; `scripts/break_tests.py`: 53 of 53 breaks caught.
2026-10-18, the six item minigames and the Minigames screen (branch `worktree-agent-a8a336c7d80722fb8`, merged with main's fishing): `BUILDER CHECK PASSED` (browser check in chromium, firefox, webkit); `count-tests.sh`: 342 passed, 0 failed, 0 ignored; `scripts/break_tests.py`: 22 of 22 new breaks caught.
