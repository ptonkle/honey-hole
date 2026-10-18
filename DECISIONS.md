# Decisions

Dated entries. Each quotes the person's request word for word, then says what the agent decided and where it is recorded. Grants of autonomy go here too ("you may publish"), with their scope: this run only, or until withdrawn.

## 2026-10-17

> the builder project is a good core. There are a lot of good lessons from ape race too, but do not port over excessive recommendations or influence from ape race which would make the game feel less unique. Good, general things to port over are things like the idea of a design journal, the menu structure, the unit testing visibility and tagging enhancements that were worked on today, ask me any questions about things you should or orshouldn't learn from

The person's answers to the follow-up questions:

- Port from Ape Race: the design journal, test tags and the quick check, check visibility (nextest, progress bars, watch window, check times, a fast mode for browser checks), and the generic menu machinery. Port no Ape Race screens or content.
- Standard from the start: What's New (version numbers and patch notes). Replays and online play are left out until the game's own design calls for them.
- Glossary: keep `GLOSSARY.md` and its copy tests, but show no hover definitions on screen (Ape Race style).
- Project notes: lean. Start with the template notebooks and add checklists or archives only when the game needs them.

The agent decided: port the general tooling now, before the game is described, and keep every Ape Race name, screen and tuning choice out of it.

## 2026-10-17 (online)

> Online play will be a part of this but I want more care since this game will likely need different interactions between players compared to ape race

The agent decided: online play is in scope, but neither Ape Race's netcode (each browser owns its own ape and sends its state 20 times a second; the host runs the AI and results) nor Mandible's (lockstep over a full mesh) is copied by default. The sync model is chosen in `PLAN.md` from the game's own player interactions once the game is described, and agreed with the person before any transport code. Only the transport layer that is truly general (WebRTC introduced by room code over pinned Nostr relays, with its BUILDER-SPEC exemptions and `check-online.sh`) may be reused. Meanwhile the core is kept deterministic and driven by per-player inputs, so every sync model stays open.

## 2026-10-17 (design questions, round 1)

> b1 sure b2 that's what the sound warning is for, that's fine b3 yes good idea b4 all fish are announced, not just legendaries, and leggendaries ar not announced for 20 seconds, the popup should be like 2 seconds. Make legendaries worth 5x. and rare 3x. and uncommon 2x. b5 not sure what you mean. That's fine, they wouldn't have the necessary momentum yet and the person who hooked it has time to react. b6 yes measure with ai b7 that's part of teh risk of going fast
>
> f1 20 seconds f2 you should be able to see them below the water. Camera should move with mouse when you are in driving mode. f3 yes it helps a bit with attracting the fish as they swim at different depths (probably rarer fish are deeper). f4 yes o1 for now just ramming? but if it could be built beefier than that that's a good idea probably. r1 I used michigan as an example but that's just a place with a lot of great fishing lakes. Just make them all thigns that could live in a midwest like like a bass, sturgeon, etc.   p1 I will not be creating the models  p2 yes  p4 do not publish to github yet, you can publish to local though

The agent decided (questions in `brief/questions.md`):

- B1: a ram that wrecks a boat also stalls the rammer's motor for about 3 s.
- B2: no visual warning; the approaching motor's sound is the warning.
- B3: a few seconds of ghosting after a respawn.
- B4: every hook is announced in a popup of about 2 s. Tier multipliers: common 1×, uncommon 2×, rare 3×, legendary 5×.
- B5: no change; a teleporter arrives without momentum, so the angler has time to react.
- B6: AI playtests measure whether the lake runs dry before fish counts are tuned.
- B7: no change; grounding in fog is the risk of going fast.
- F1: reel times for a perfect reel are 20 / 10 / 5 / 3 s (legendary to common).
- F2: fish are visible below the water's surface. The mouse turns the camera in driving mode.
- F3: fish swim at depths, rarer ones deeper, and a bobber near a fish's depth attracts it a little better.
- F4: switching to driving while reeling drops the fish.
- O1: ramming is the only interaction for now; the online design is built so more can be added.
- R1: every fish is one that could live in a Midwest lake.
- P1: the agent creates all models.
- P2: rename the folder to `honey-hole/` (done once no agent is working in a worktree under it).
- P4: no GitHub yet; local deploys only.
- Unanswered questions keep the agent's suggestions (F5, R2, I1–I4, O2, O3, P3).

## 2026-10-17 (radar look, map name)

> let's do lower fidelity on the radar
> also call this map "Burt Lake" in the UI
> not that blocky, do like 80% the fidelity of the original
> yeah that's good

The agent decided: the radar keeps its first look (fine grid, thin ping rings, sweep, bright shoreline, two water shades, scanlines) at about 80% of the first preview's fidelity: the shoreline is judged in 2.5 px blocks where it used 2 (`tools/maps/preview.py`, `RADAR_CHUNK`). The in-game radar matches this. The first map's on-screen name is Burt Lake (`map.burt_lake`).

## 2026-10-17 (reeds, fog)

> reeds have no effect, I woul dneed to see the fog distance in game

The agent decided: boats pass through reeds with no slowdown or collision; reeds are scenery (and later a fish habitat). The fog distance stays at 120 m for now, and the map checker's cover rule uses that value. Once the lake is drawn in 3D, a debug control will set the fog distance live so the person can choose it in the game; the checker's sight distance then follows the chosen value. Boat top speed (16 m/s) and rare/legendary spacing (60 m) keep the map work's defaults until the person says otherwise.

> fog distance should be very easy to tune quickly too

The agent decided: the fog distance is one constant, `sight_mm` in the Burt Lake parameters (`crates/core/src/map/mod.rs`), which the 3D fog and the map checker both read. It can also be changed live in game from a debug control, for trying values without a rebuild. A changed value can make some pregenerated layouts too open or too cramped, so one command rescans the layout list; a test fails if the list was checked against a different fog distance than the file holds, so the two can't drift apart unnoticed.

> like surely it's just a single const right

Yes: `sight_mm: 120_000` (mm) in `crates/core/src/map/mod.rs`.

## 2026-10-17 (first model feedback)

> the fisherman is a bit too blocky is there any advantage to it being made out of cubes like this vs like triangles or trapezoids?
> boat looks good though

The agent decided: the boat is approved as is. The angler is remodelled with tapered, many-sided shapes (no plain boxes), keeping its poses, anchors and colours.

> Fish look pretty good tbh, I do'n thtink it needs any changes right now. The sizes don't look right now, isn't a sturgeon longer? they're all a bit the same shape from the top
> animations look pretty good
> lakeside props look good too

The agent decided: the fish's side views, the swims and the lakeside props are approved as they are. The model lengths already differ (bluegill 0.2 m to sturgeon 1.5 m), but the Gallery fits each model to the frame, so they look alike. The Gallery gets a to-scale line-up, base lengths are checked against real adults, and each fish gets its own exaggerated silhouette from above, with the side views and swims kept.

> boat colors are are all good

The agent decided: the four player colours in `data/look.json` stay as they are, yellow included (R1.54 answered).

> item buoy is too much like mario. Good idea but just too mario.

The agent decided: the item buoy keeps the idea (a floating mystery item that reads in fog and bobs) but drops the "?" block. Three fishing-themed alternatives (a giant bobber, a buoy with a glowing tackle box, a cluster of lures) for the person to choose from.

> 1

The person chose the giant bobber for the item buoy.

## 2026-10-17 (overnight autonomy)

> Ok. I'm going to go to bed, continue to build the game until it is finished and playable. Use your best judgement for any decisions you come across, and write the code in such a way that for the things that you think might need to be changed, they are easily updated. Do not stop working for anything. seek alternatives when you are blocked. Record decisions that you made and want me to validate and bring them up when I am able to inform you again in the morning. Any questions?

Grant: the plan is approved in advance; the agent builds until the game is finished and playable, decides by its own judgement, and records decisions to validate in `PLAN.md` ("For the person to validate") and `SECOND-ORDER.md`. Values likely to change go in `data/tuning.json`. Scope: this run, until the person is back. Publishing stays local only (no GitHub).

## 2026-10-17 (questions before the night)

The person's answers:

- **Line breaks:** a catch meter, as in Stardew Valley. It fills while the fish is in your zone and drains while it's out, and the line breaks only when it empties.
- **Casting:** only when the boat is nearly stopped. A moving boat drags the bobber, and too fast snaps the line or reels it in.
- **Minigames and the Fish-Boy:** the boat coasts (the throttle is released).
- **Your own score:** you can see your own catch list, but no point totals for anyone until the postgame.
- **Roster for Burt Lake:**
  - Common: bluegill, yellow perch, pumpkinseed, rock bass.
  - Uncommon: smallmouth bass, largemouth bass, walleye, black crappie.
  - Rare: northern pike, channel catfish, lake trout.
  - Legendary: muskellunge, lake sturgeon.
- **Respawn:** caught fish don't respawn; the count is fixed per match.
- **Names:** "Fishy names, but all conceivably real names, no nicknames like "reel deal ray"". You type your own name once, and it's kept in Settings.
- **Fish sight:** "Close only, but you shouldn't need to get right over the legnedaries be a little more generous than that". The agent set it to about 40 m for shallow fish, fading with depth, so deep legendaries show from about 25 m. These are tuning values.

> fish do not disappear if you fail the fishing game btw

A fish that breaks the line (or is dropped with Esc, Q or a wreck) swims free and stays in the lake; the Fishpedia count is unchanged (F5 confirmed).

> maybe they swim a bit away

A fish that gets free swims a short way off (a tuning distance, about 10–15 m) before it settles back to its usual roaming.

More answers:
- **Esc with nothing to cancel:** opens a small menu (Settings, Leave match) while the match keeps running. Offline against AI it also pauses.
- **Bite wait:** 2–6 s once the bobber is near fish, then 1–3 nibbles before the real bite (tuning values).
- **Offline match setup:** AI count (0–3) and each AI's difficulty (Easy/Normal/Hard), match length (default 8 min), map seed (random, or typed), and the item buoy interval (default 1 min).

> eyebrows on the fisherman look sad. otherwise looks good

The agent levelled the brows and raised them slightly; the rest of the angler is approved.

## 2026-10-18 (morning feedback)

> The UI is overexplaining right now. YOu do not need a speed meter, you do not need a thing that tells you when you're driving (you can clearly tell based on where the guy is sitting). you do not need a popup that tells you when that you lose your item when you pick up a second one.
> Also I don't like the color scheme and other things baout the menus. where did you get this style from? Did you fall back to teh guidance in builder to use tikz with the colorblind-friendly color scheme? This makes it look very samesy. It should be quick to fix though I hope.
> there is no reason they wouyld want to invert mouse up and down, remove this.
> another overexplaination "the item buoy is on the radar", why not just say "an item has spawned". This is a game, there should be skill expression, you do'nt need to tell the players exactly what is happening and what to do at all times.
> you don't ned all screens to say that you can raise the fish-boy
> minigames are oberexplained too, "wind it clockwise 5 times" should just be "wind the reel"
> don't need to tell them "esc quits and the item is lost"
> there's also effectively 2 progress bars in the reel winding mini game too
> tleeport should be "sync the teleporter!", nitrous should be "create nitrous!". photopgraph one doesn't need to tell you how many photographs you need to take.
> radar one should just say "bring into focus"
> line tesnsion sound isa bit much, maybe it's actually quieter in the action game. Could it be like the ratcheting sound instead? Right now it sound slike I'm being abduected by an alient
> otherwise sounds seem good
> generally about the on screen language, it seems like you are putting in user-facing string a lot of information from the spec I gave you. ... Keep it simple, stupid - KISS. You can leave this information in code comments all you want but exposing this to end users is not the corect thing to do
> also we probably need to talk about boat speed and map size. I haven't played yet but I think I was imagining a higher top speed and larger map. We can adjust later though

Answers to follow-up questions: menu and HUD style "Retro outdoors magazine"; boat speed and map size "Decide after I play".

The agent decided: a KISS pass over every player-facing string and HUD element (removing the speed gauge, the mode badge, the item-replaced popup, the Fish-Boy hints, minigame rule text and "Esc quits" lines, the second progress bar in Power Reel, and invert-Y; using the person's wording for the minigame calls); a restyle of menus, HUD, popups and postgame as a retro 1970s outdoors magazine; the line-tension sound replaced by a quieter ratchet-like tick. Sounds otherwise approved. The menus' current look came from Builder's default style pack palette.

> make sure that after it announces the winners "Herb fisherman wins" popup, thre is still an option to close that popup and see what summed up to that total

The agent decided: the postgame winner announcement can be closed to show each player's catches and points adding up to their total, with a way back to the ranking.
