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
