# Plan

Stage 2, written by the agent from `BRIEF.md`. The person granted autonomy on 2026-10-17: "continue to build the game until it is finished and playable. Use your best judgement..." (`DECISIONS.md`). Decisions marked **(V)** are ones the agent made alone and wants the person to validate; they are collected under "For the person to validate".

## Architecture (contracts every build agent follows)

| # | Decision | Reason | Test that shows it holds |
| --- | --- | --- | --- |
| 1 | **Units.** Positions in whole millimetres (x east, y north, z up in map space; models use glTF axes, converted at placement). Angles in 65536 units per turn. Speeds in mm/s. Time in ticks. | Integers only in the core (D2); the map module already uses these. | `map` tests; boundary scan |
| 2 | **Fixed tick: 50 per second (20 ms).** The page passes the clock in ms (`advance(now_ms)`); the core runs whole ticks and interpolates the drawing between them. | Boat physics and the 0.75 s hook window need a fine step; 20 ms is a whole number of ms. | `ticks_follow_the_clock_and_never_skip_inputs` |
| 3 | **One `Match` struct** (`crates/core/src/sim/`) holds the whole match: map, boats, fish, buoy, items, timer, scores, events. `Match::step(&mut self, inputs: &[SeatInput])` advances one tick. Same seed + same inputs give the same match. | Determinism serves tests, AI playtests, replays of bugs, and any online model. | `two_matches_with_the_same_inputs_stay_identical` |
| 4 | **`SeatInput`**: the held keys (W, A, S, D, Space, Shift), edge presses (Q, Tab, Esc, Shift, Space), mouse delta, mouse buttons and the pointer position in the current 2D layer (for minigames). People and AI make the same `SeatInput`. | One input path, so AI plays by the same rules. | AI tests drive `SeatInput` only |
| 5 | **Events.** Each tick appends `MatchEvent`s (Hooked, Caught, LineBroke, Wrecked, Respawned, BuoyWarning, BuoySpawned, ItemPicked, ItemUsed, Pinged, MatchEnded...). Popups, sounds, the AI and online messages all read events. | One place where things happen. | Event tests per system |
| 6 | **Tuning in data.** Every gameplay number the brief names, and every number likely to change, lives in `data/tuning.json`: match length, buoy interval, item durations, ram and grounding thresholds, reel times, tier multipliers, fish counts, ping cooldown and sweep, boat speeds. The core parses and validates it at start (a wrong key or value is refused by name). The fog distance stays the map's `sight_mm` (the person's call). | The person asked for things that may change to be easy to update. | `tuning_file_is_complete_and_valid`; each system's tests read the values from the parsed tuning, not literals |
| 7 | **Drawing.** The core returns three layers per frame: (a) **world**: static geometry (terrain, shore, trees, reeds, props from the map) uploaded once per match, plus per-frame model instances (boats, anglers, fish, buoy, bobber, line); (b) **screen 2D**: a draw list of rects, lines, circles, polygons, text and "model viewports" (a 3D model drawn in a rectangle, for the Fishpedia boxes), used for the HUD, popups, minigames, the postgame and the Fish-Boy screen; (c) **UI panels** for menus, as now. The Fish-Boy screen's 2D list is drawn onto the device model's `screen` part. | Keeps the page a replayer with no rules (R6, D3). | browser check steps per screen |
| 8 | **Cameras.** Driving: third person, the mouse orbits. Fishing with the line in: underwater camera on the hook, the mouse turns it, and a picture-in-picture of the boat in the top left. Fish-Boy: first person, device raised (screen 80% of the view, bezel 10%). | Brief. | camera tests |
| 9 | **Sound.** The core emits sound cues with parameters (and the motor loop's gain and pitch per rival, Doppler computed in the core). The page synthesises them with WebAudio from recipes in `data/sounds.json`. No audio files to license. | Sounds come from rules (distance, speed); synthesis keeps it all in the repo. | `a_rival_motor_is_heard_from_5_s_away`; `doppler_raises_pitch_when_closing` |
| 10 | **AI players** (`crates/core/src/ai/`) read the match through the same view a player has (no peeking at hidden fish unless a radar showed them) and produce `SeatInput`. Easy / Normal / Hard. A headless `examples/playtest.rs` runs AI-only matches and prints balance numbers. | Brief: AI opponents and playtests. | `ai_finishes_a_match_with_catches`; playtest report in `notes/` |
| 11 | **Online (V).** Host-authoritative world, client-owned boats. Each browser simulates its own boat and fishing locally (no input lag) and sends boat state 20 times a second. The host runs everything shared: fish, hooks (two players can't hook the same fish), the buoy and item pickups, wrecks (the host judges rams from both boats' reported states; the victim's browser applies the wreck), the timer, scores and the postgame. Clients send claims (hooked fish F, caught fish F, picked buoy, used item) and the host confirms or refuses. Transport: room codes over pinned Nostr relays + WebRTC (the same exemptions Mandible and Ape Race record). | Ramming needs lag-free steering, and the shared fish and buoy need one referee. Leaves room for more interactions (each is a new claim type). | `two_sessions_over_loopback_agree_on_scores`; `only_one_player_can_hook_a_fish`; online browser check over pasted codes |

## Build order

1. **Match core:** boats (physics, collisions, rams, grounding, wrecks, respawn, ghosting), the timer, the spawn, the event list, `tuning.json`. **World drawing:** terrain, water, fog, props, boats in the lake, cameras. **2D layer** and the Fish-Boy screen with the live radar and Fishpedia. **Sound engine.**
2. **Fishing:** fish swimming and depths, visibility under the water, casting, the bobber, nibbles, the hook window, the reel bar, landing, line breaks, hook popups.
3. **Items:** the buoy spawn, warning and fair point; pickup; Shift; the six items and their six minigames; the Camera prompt.
4. **Match flow:** the lobby (offline with AI seats), countdown, the HUD, popups, the end, the postgame reveal.
5. **AI players** and headless playtests; tune fish counts and values from them.
6. **Online:** lobby by room code, the host-authoritative sync, the online check.
7. **Polish:** sounds for everything, the journal, a full-match browser check.

## For the person to validate

Collected as the agent makes them; brought up in the morning. Each also has a row in `SECOND-ORDER.md`.

- V1. Online sync model (decision 11).
- V2. 50 ticks per second (decision 2).
- V3. The Fish-Boy's screen is drawn on a 2D layer over the view, fitted to where the device's screen lands, not as a texture on the model; the device slides straight up when raised (SECOND-ORDER R2.60).
- V4. Keys 1, 2 and 3 (and a click) switch the Fish-Boy's tabs, and Space pings with the radar up; A and D keep steering (R2.63).
- V5. A ping shows each rival where the sweep found them until the next sweep; a cloaked rival's old blip is wiped (R2.64). The cooldown runs from the ping's start (R2.65).
- V6. The fish's base lengths and values in `data/fish.json` (R2.69).
- V7. The radar shows the lake's square, not the whole map (R2.71); the radar's brighter player colours (R2.75).
- V8. A third Fish-Boy tab, My catch, lists your catches without values (R2.73).
- V9. Rams and grounding are judged on the speed *toward* the other boat or the land, not the boat's whole speed: a fast glancing scrape bounces (SECOND-ORDER R2.3).
- V10. The boat's handling numbers the brief leaves open: reverse 4 m/s, braking 2 s, coasting 6 s, turning 100°/s, the cast swivelling 135° either side, nitrous 200/150/150% (`data/tuning.json`, R2.7).
- V11. Two boats ramming each other head-on, both fast enough, are both wrecked (R2.4).
- V12. The draft AI names (`data/names.json`, R2.11).
- V13. The match menu has Resume as well as Settings and Leave match (R2.9).
- V14. Fish base lengths trimmed so a 1.5× catch is a trophy, not past a record (muskie 0.95 m, pike 0.65 m, walleye 0.43 m, smallmouth 0.35 m, perch 0.2 m, bluegill 0.17 m); the sturgeon stays 1.5 m rather than the 1.8 m first suggested (SECOND-ORDER R1.150–R1.154).
- V15. Angler brows made level and slightly raised (the person said they looked sad).
- V16. Sound: a rival's motor is heard from 5 s at top speed (80 m), rising with the square of the nearness to full within 6 m, and its Doppler shift is exaggerated 3 times (about +16% closing, -12% leaving) because the true shift of a boat is under a semitone (SECOND-ORDER R2.93, R2.94). Every sound's recipe is the agent's: audition them on the Sounds screen (Gallery, Sounds) and say which to change.
- V17. Sound: the 3-2-1 beeps are alike with a higher go, the buoy's five beats climb to a high last one, and an explosion and the wreck chime are heard by every player at full volume (SECOND-ORDER R2.96, R2.97, R2.106).
- V18. Six more roster fish modelled (pumpkinseed, rock bass, largemouth bass, black crappie, channel catfish, lake trout), bases 150 to 600 mm on the same 1.5× rule (SECOND-ORDER R2.110 to R2.117).
- V19. The lake's look: overcast grey-blue sky, fog thickening from 22% of the sight distance, water 30% see-through looking down, cool greens (`notes/shots/world/`, `data/look.json` "lake"; SECOND-ORDER R2.35, R2.37, R2.39).
- V20. Props the scene adds beyond the map: the dock behind the spawn, two cabins, lily pads, shore rocks, boulders, fallen logs; maples drawn as birches or tall spruces (R2.34, R2.40).
- V21. The boat camera: 9 m behind, 3.8 m up, 10° down, 55° field of view (R2.43).
- V22. Glowing colours take 45% of the fog on the lake, so the buoy's beacon and a cabin window read through it (R2.42).
- V23. Raising the Fish-Boy makes the boat coast (its keys do nothing), and Esc puts the Fish-Boy down before it opens the match menu (SECOND-ORDER R2.82).
- V24. Each fish's starting count is in `data/fish.json` (`count`: 30 common, 20 uncommon, 10 rare, 5 legendary over the 13 fish) until the fishing system adds its variance (R2.84).
- V25. With the Fish-Boy up, the world is seen first person from 1.5 m over the water, along the camera's heading (R2.85).
- V26. All thirteen fish checked against the real species from above, the side and 3/4 and reshaped: torpedo smallmouth, conical-snouted sturgeon, thin sunfish, smooth duck-bill pike and muskie, thin tails and fins, a flat-headed catfish with a short dorsal, a painted largemouth stripe, markings kept inside the outline from above; a few side views changed (SECOND-ORDER R2.130 to R2.136; shots in `notes/shots/models/fish-v3/`).
- V33. The six item minigames as built (SECOND-ORDER R3.80 to R3.94): one 1600 x 900 canvas for all; the camera's fish drawn flat, not as models (R3.84); each radar slider clearing its own fault (R3.86); anticlockwise winding back the turn in progress (R3.83); every number in `data/tuning.json` `minigames`. Try them from the Title, Minigames.
- V27. Fishing numbers (`data/tuning.json` `fish`, `fishing`): a cast only below 1.5 m/s; a full charge in 1.5 s casts 25 m, a tap 4 m; the lure sinks 1.5 m/s; a fish comes from 7 m at the lure's depth, 2.1 m when 3 m deeper or shallower; Space before the bite scares it; the hook window rounds to 760 ms (SECOND-ORDER R3.5, R3.13).
- V28. The reel: a 28% catch zone, the meter starting 30% full; perfect reels 3/5/10/20 s, but a steady player takes about 3, 5, 13 and 47 s, and the meter drains in 5/6/10/24 s so legendaries are long, landable fights that leave time for rivals to come (R3.12).
- V29. Each kind's count moves up to 20% either way per match; a freed fish swims 12 m off and roams alone from there; schools wander within 10 m of where they were placed (R3.2, R3.4).
- V30. Fish seen from the boat camera are drawn twice their size and at 45% of their depth so they read from 9 m back; under the water they are life size (R3.6).
- V31. The underwater camera's murk (green-blue, 16 m), the picture-in-picture (26% of the screen's width, top left, white border), the HUD moved to the top right, and the fishing HUD as panels until the 2D layer draws it (R3.7, R3.9, R3.10, R3.11).
- V32. The hook popup ends with a full stop, not "!", for the tone lint (R3.15).
- V34. Items (SECOND-ORDER R4.3 to R4.25): an unclaimed buoy moves to the new fair point when the next is due; equal item weights; a 3.5 m pickup; the first buoy a whole interval in.
- V35. Esc in a minigame, or while picking a teleport's spot, wastes the item; Tab cannot put the Fish-Boy down under its minigame (R4.9, R4.10).
- V36. The Teleporter: the boat can drive while it charges; a spot on land snaps to water 6 m from land; it lands at rest (R4.16).
- V37. The camera's offer: Shift uses it, Esc saves it, and after 10 s it is saved (R4.18).
- V38. Enhanced Radar pings whatever the cooldown and marks the nearest fish of each tier (R4.19).
- V39. Your cloaked boat is drawn pale to you until the see-through pass merges; the item slot, timers, icons and popups are the agent's (R4.12, R4.20 to R4.22; shots in `notes/shots/items/`).

## Status

- [x] Plan approved in advance by the person's autonomy grant, 2026-10-17.
