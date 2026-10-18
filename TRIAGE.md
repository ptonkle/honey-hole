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
