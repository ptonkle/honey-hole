# Triage

Stage 7. One row per request after the person has used the build: a bug, a change, or a new idea, in the person's own words. Rows are numbered T1, T2 and so on, and each commit that answers one names it ("Fix the timer reset (T4)"). `C4 Verify against hand-computed cases`.

Compare at least one result with a case worked out by hand. **Severity** is *blocks use*, *wrong*, or *wanted*. **Cost** is the agent's estimate. **Disposition** is *done*, *open*, *declined* (with the reason), or *try* (built on a branch and published as a TRY commit that says how to undo it).

| # | Request, in the person's words | Expected | What happened | Severity | Cost | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| T1 | | | | | | |

## Future ideas (the person's todos, for a later session)

Not built in the session they were raised in. Pick them up from here.

| # | Idea | Raised | Notes | Status |
| --- | --- | --- | --- | --- |
| T1 | Fish should hit and back off the bait when biting. | 2026-10-18 | The person's words: "fish should hit and back off of bait when biting". Nibbles exist in `crates/core/src/sim/fishing/` (1–3 nibbles, then the bite, Animal Crossing style), but the fish may not visibly dart in to the lure and back off on each nibble. Check the underwater view and the fish's motion per nibble. | open |
| T2 | The reel minigame (Stardew Valley style) is far too hard. | 2026-10-18 | The person's words: "stardew valley game is wayyyyy too hard". Knobs are in `data/tuning.json` `fishing.reel`: `zone_permille` (28%), `gravity_per_s2`, `lift_per_s2`, `start_pct`, per tier `fish_speed_per_s`, `fish_dart_pct`, `drain_ms`. The AI agent already eased rare and legendary once (PLAN V38). Consider a bigger zone, a slower drain, a gentler fish and a fuller starting meter, and re-check AI reel times and the playtests afterwards. | open |
| T3 | Engine noise in general is far too loud. | 2026-10-18 | The person's words: "engine noise in general is wayy too loud". The levels are in `data/tuning.json` `audio.own_motor` (`idle_gain`, `full_gain`) and `audio.rival_motor` (`loudest`), plus the `motor` and `rival_motor` loop recipes in `data/sounds.json`. Lower both, keeping a rival's approach audible against the quieter mix. | open |
| T4 | A visual cue when the fish takes the bait, not just the sound. | 2026-10-18 | The person's words: "need a queue when the fish takes the bait other that sound" (cue). The KISS pass cut the fishing text to a bare "Bite". Consider the bobber plunging, a splash or ring on the water, the fish visibly lunging at the lure underwater, a quick screen pulse, or a rod-tip bend, so the 0.75 s hook window reads without sound (also an accessibility need: important sounds should have an on-screen cue, per BRIEF). Pairs with T1 (fish hit and back off the bait). | open |
| T5 | The fog doesn't show in single player (probably online too). | 2026-10-18 | The person's words: "I can't see the fog in single player for some reason. probably also an issue in multiplayer". Leads: the Lake preview has a fog toggle and slider (`act("lake.fog")`) whose setting "carries into matches", so a match may inherit fog-off from the preview or a stale default; the perf pass changed the fog shaders ("skip the murk where the fog hasn't started") and draws the sky last on the far plane; the fog distance is `sight_mm` (120 m) with fog from 22% of it (`data/look.json` `lake.fog_start_pct`). Check a fresh match with no preview visit, and online. | open |
