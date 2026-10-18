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
- V3. Sound: a rival's motor is heard from 5 s at top speed (80 m), rising with the square of the nearness to full within 6 m, and its Doppler shift is exaggerated 3 times (about +16% closing, -12% leaving) because the true shift of a boat is under a semitone (SECOND-ORDER R2.93, R2.94). Every sound's recipe is the agent's: audition them on the Sounds screen (Gallery, Sounds) and say which to change.
- V4. Sound: the 3-2-1 beeps are alike with a higher go, the buoy's five beats climb to a high last one, and an explosion and the wreck chime are heard by every player at full volume (SECOND-ORDER R2.96, R2.97, R2.106).

## Status

- [x] Plan approved in advance by the person's autonomy grant, 2026-10-17.
