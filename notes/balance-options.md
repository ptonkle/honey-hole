# Balance options: the ramming fix

For the person to choose (PLAN.md, "V-balance: choose a ramming fix"). Written 2026-10-18 by the overnight balance agent.

## The question

In AI playtests, ramming paid far better than fishing (`notes/playtests.md`, PLAN.md V39): a minute of hunting knocked about 855 points off rivals' lines, against 92 points landed a minute of fishing, and 284 of 337 legendary hooks were rammed off. The brief wants interference at the centre ("most of the gameplay is driven based on players trying to interfere with each other catching fish") but not "players spending all game just destroying each other rather than fishing".

Five knobs were added to `data/tuning.json` (the new `balance` section, read by `crates/core/src/sim/balance.rs`), each one off by default, so that turning a knob off gives back today's game exactly. With every knob off, the playtest's numbers matched the earlier report to the digit. Then each option was measured.

| Knob | `data/tuning.json` key | Off (today's game) | What it does |
| --- | --- | --- | --- |
| 1. Hook popup delay | `balance.announce_delay_ms` (per tier) | 0 | The hook popup, and the other players' hook sound, come this long after the hook. A fish that is landed or lost before then is never announced (its landing has its own popup). The AI hears only the popup (the new `Announced` event) and knows the delay rule, as a person would. |
| 2. A rammed-off fish swims far | `balance.rammed_swim_mm` (per tier) | 12000 (same as `fish.freed_swim_mm`) | A fish knocked off a line by a ram swims this far away. Other lost fish still swim 12 m. |
| 3. Protection while reeling | `balance.reeling_ram_wreck_pct`, `balance.reeling_anchored` | 80, false | A boat reeling a fish in is wrecked only at this share of top speed. When anchored, a bump doesn't push it (the boat that bumps it takes the whole bump). **This changes a rule you set (BRIEF: a ram at 80% wrecks), so I only measured it.** |
| 4. AI aggression | `ai.skill.*.aggression_pct` (already there) | 15 / 45 / 75 | How readily each difficulty hunts. |
| 5. A cost for the rammer | `balance.rammer_wreck_pct` | 0 (never) | A rammer driving at least this fast toward its victim is wrecked too. **This changes a rule you set (round 1, B1: the rammer only stalls), so I only measured it.** Making the rammer lose its held item could not be built or measured: items are not on main yet, and the AI does not use items (SECOND-ORDER R5.6). |

## How it was measured

`cargo run --release --example playtest -- 48 1 <key>=<value> ...` runs 48 seeded 4-AI matches of 8 minutes on Burt Lake (24 with four Normal players, 24 with mixed difficulties), from seed 1. A setting given as `key=value` changes one tuning value for that run only. Every option was also run again on seeds 1001 to 1048. The two runs agree within a few points on every measure, and the ranking of the options is the same. The table shows seeds 1 to 48; the second run's numbers for the leading options are below it.

How to read the columns:
- **Fishing / min**: all points landed, divided by the minutes spent fishing.
- **Knocked off / hunting min**: points of fish rammed off rivals' lines, divided by the minutes spent hunting.
- **Rammer lands / hunting min**: points of knocked-off fish that the rammer then landed itself, divided by the same minutes. This is the part of a hunt that scores for the hunter.
- **Landed / player** and **Knocked off / player**: per player per match.
- **Legendaries landed**: of all legendary hooks.
- **Margin**: the winner's lead over second place, as mean / median.
- **Hunting**: the share of every player's time spent hunting.
- **Fish left**: the share of stocked fish still in the lake at the end, all tiers (legendaries alone in brackets).
- **Rams**: rams that wrecked a boat, per match.

### Results, seeds 1 to 48

| Option | Setting | Fishing / min | Knocked off / hunting min | Rammer lands / hunting min | Landed / player | Knocked off / player | Legendaries landed | Margin | Hunting | Fish left | Rams |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **Today** | all off | 92 | 855 | 59 | 369 | 684 | 37 of 337 (11%) | 34% / 38% | 10% | 59% (85%) | 9.6 |
| 1 | legendary popup 15 s late | 128 | 908 | 223 | 548 | 374 | 94 of 228 (41%) | 26% / 24% | 5% | 51% (61%) | 7.0 |
| 1 | legendary 20 s | 146 | 793 | 255 | 628 | 236 | 124 of 188 (66%) | 23% / 18% | 4% | 46% (49%) | 5.5 |
| 1 | legendary 10 s, rare 5 s | 130 | 1088 | 166 | 562 | 422 | 85 of 305 (28%) | 26% / 27% | 5% | 48% (65%) | 5.4 |
| 1 | legendary 20 s, rare 10 s | 173 | 1083 | 360 | 732 | 100 | 144 of 207 (70%) | 21% / 21% | 1% | 39% (40%) | 1.8 |
| 1 | legendary 30 s, rare 15 s | 183 | 178 | 96 | 765 | 6 | 153 of 160 (96%) | 21% / 19% | 0% | 36% (37%) | 0.8 |
| 2 | rare and legendary swim 40 m | 97 | 863 | 69 | 383 | 553 | 37 of 271 (14%) | 24% / 23% | 8% | 58% (85%) | 8.3 |
| 2 | rare and legendary swim 80 m | 96 | 762 | 32 | 369 | 490 | 26 of 245 (11%) | 28% / 27% | 8% | 57% (89%) | 7.5 |
| 4 | aggression 10 / 30 / 50 | 92 | 870 | 58 | 368 | 683 | 35 of 336 (10%) | 34% / 38% | 10% | 59% (85%) | 9.6 |
| 4 | aggression 5 / 20 / 35 | 93 | 912 | 70 | 372 | 690 | 36 of 354 (10%) | 32% / 35% | 9% | 60% (85%) | 9.5 |
| 4 | aggression 0 / 10 / 20 | 101 | 895 | 110 | 412 | 607 | 47 of 314 (15%) | 27% / 24% | 8% | 58% (80%) | 8.7 |
| 4 | aggression 0 (no hunting) | 186 | 0 | 0 | 761 | 1 | 150 of 155 (97%) | 18% / 17% | 0% | 37% (38%) | 0.5 |
| 3 (rule) | reeling boat wrecks at 90% | 92 | 516 | 41 | 374 | 455 | 38 of 315 (12%) | 27% / 28% | 11% | 58% (84%) | 6.5 |
| 3 (rule) | reeling boat anchored | 99 | 815 | 71 | 394 | 642 | 45 of 319 (14%) | 31% / 33% | 10% | 59% (81%) | 9.1 |
| 3 (rule) | 95% and anchored | 150 | 267 | 72 | 584 | 242 | 103 of 209 (49%) | 21% / 18% | 11% | 49% (57%) | 3.8 |
| 5 (rule) | rammer wrecked at 90% | 96 | 983 | 30 | 361 | 680 | 24 of 332 (7%) | 23% / 22% | 9% | 59% (90%) | 16.2 |
| 5 (rule) | rammer wrecked at 80% | 99 | 880 | 24 | 367 | 630 | 32 of 311 (10%) | 25% / 22% | 9% | 59% (87%) | 18.5 |
| 1 + 2 | legendary 20 s, swim 40 m | 144 | 755 | 208 | 606 | 208 | 118 of 172 (69%) | 23% / 22% | 3% | 47% (51%) | 5.1 |
| **1 + 2 (applied)** | **legendary 20 s, rare and legendary swim 80 m** | **146** | **848** | **150** | **611** | **211** | **119 of 187 (64%)** | **22% / 21%** | **3%** | **48% (51%)** | **4.8** |
| 1 + 2 | legendary 15 s, swim 80 m | 128 | 837 | 135 | 535 | 284 | 88 of 202 (44%) | 26% / 26% | 4% | 50% (63%) | 5.5 |
| 1 + 2 + 4 | legendary 20 s, swim 80 m, aggression 5 / 20 / 35 | 151 | 863 | 170 | 630 | 198 | 127 of 190 (67%) | 22% / 20% | 3% | 47% (47%) | 4.3 |
| 1 + 3 (rule) | legendary 20 s, 95% and anchored | 166 | 253 | 102 | 689 | 92 | 132 of 162 (81%) | 24% / 22% | 5% | 41% (45%) | 2.3 |
| 1 + 5 (rule) | legendary 20 s, rammer wrecked at 90% | 143 | 873 | 187 | 586 | 263 | 112 of 186 (60%) | 26% / 24% | 4% | 48% (54%) | 10.6 |

Each "Rams" count includes the rammer's own wreck when knob 5 is on: a rammer that wrecks itself counts as a second ram.

### The same check on seeds 1001 to 1048

| Option | Fishing / min | Rammer lands / hunting min | Landed / player | Knocked off / player | Legendaries landed | Margin | Hunting | Rams |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Today | 85 | 52 | 340 | 712 | 29 of 355 (8%) | 25% / 23% | 11% | 10.1 |
| Legendary 20 s | 151 | 372 | 640 | 244 | 135 of 203 (67%) | 23% / 18% | 4% | 5.5 |
| **Legendary 20 s, swim 80 m (applied)** | **150** | **157** | **619** | **188** | **122 of 183 (67%)** | **21% / 22%** | **3%** | **4.5** |
| Legendary 15 s, swim 80 m | 125 | 84 | 521 | 329 | 90 of 213 (42%) | 24% / 17% | 4% | 6.0 |
| Aggression 5 / 20 / 35 | 93 | 65 | 377 | 661 | 37 of 349 (11%) | 30% / 28% | 10% | 9.0 |

## What the numbers say

- **The popup delay (knob 1) is the knob that works.** A legendary announced 20 s after its hook gives a Normal player most of its 32 s reel before anyone hears about it, so two in three legendary hooks are landed instead of one in nine. Points knocked off fall from 684 a player to 236, and points landed rise from 369 to 628. Hunting is still there: about 5 rams a match, and rare hooks are still announced at once and hunted. The race gets closer too: the median margin falls from 38% to 18%, close to the 17% of a game with no hunting at all. This is also the plainest reading of your round-1 answer, "leggendaries ar not announced for 20 seconds" (SECOND-ORDER R4.49).
- **A delay on rares as well goes too far.** Legendary 20 s with rare 10 s brings rams down to 1.8 a match and hunting to 1% of the time, and at 30 s / 15 s nobody hunts at all. Interference stops being part of the game.
- **On its own, the long swim (knob 2) does little, but it closes a gap the delay leaves.** With the delay alone, a hunt that does arrive still pays the hunter well: the rammer lands the knocked-off fish itself (255 points per hunting minute, against 146 per fishing minute; 372 against 151 on the second run). An 80 m swim cuts that to 150 (157), about what fishing pays, and the steals fall from 1.6 to 0.9 a match. Without the delay, 80 m changes almost nothing else, because the fish is still knocked off.
- **Gentler AI (knob 4) barely moves anything until it reaches 0.** A legendary is worth so much (about 340 points) that even 10% aggression finds a hunt worthwhile. The knob only tunes how often the AI plays the hunter; it doesn't change what hunting pays. It is not a fix for people.
- **Protecting the reeling boat (knob 3, a rule change) works only if both parts are on.** With a 90% threshold alone, the ram still shoves the angler past the line's 5 m/s snapping speed, so the fish is lost to a broken line instead of a wreck (the test `a_reeling_boat_is_wrecked_only_at_the_reeling_ram_speed` shows it). With 95% and anchored, about half of the legendaries are landed. That is less than the delay achieves, and it needs your 80% rule to change.
- **Wrecking the rammer too (knob 5, a rule change) makes things worse.** Hunters still knock off just as much, legendaries are landed less often (7%), and the wrecks double. The AI doesn't weigh its own wreck, but a person who doesn't mind a 2 s respawn would play the same way.

## Recommendation, and what was applied

**Applied:** knobs 1 and 2, neither of which contradicts anything you set (`data/tuning.json`, `balance`):
- `announce_delay_ms.legendary`: 0 → **20000** (a legendary's hook popup comes 20 s after the hook);
- `rammed_swim_mm.rare` and `rammed_swim_mm.legendary`: 12000 → **80000** (a rare or legendary fish knocked off by a ram swims 80 m away).

Commons, uncommons and rares are still announced at once, and rares are still hunted. This keeps interference central (about 5 rams a match, a fifth of the old knocked-off points), while fishing becomes the main way to score (611 landed per player against 211 knocked off, and a hunt pays the hunter about what fishing pays). Two in three legendary hooks are landed, half of the legendaries are left in the lake at the end, and the median margin is 21%.

**To undo it:**
- To remove the main change with one value: set `balance.announce_delay_ms.legendary` back to `0`. The 80 m swim that stays is close to today's game on its own (row "rare and legendary swim 80 m": 11% of legendaries landed, 7.5 rams a match).
- To get today's game exactly: also set `balance.rammed_swim_mm.rare` and `.legendary` back to `12000`.

**If you want more hunting**, use a 15 s legendary delay (the "legendary 15 s, swim 80 m" row): 42% to 44% of legendaries landed and 5.5 to 6 rams a match. **If you want less**, give rares a delay too (5 s to 10 s).

Rule changes I did not apply: knob 3 (95% and anchored) is the next best fix if you'd rather keep every popup instant. Knob 5 should not be used.

## Not measured

- Making the rammer drop its own held item: items are not on main, and the AI does not pick up or use items yet (SECOND-ORDER R4.51).
- Real players. The AI hunts by an estimate (`ai.rival_reel_ms`, `ai.fishing_rate_per_min`). It doesn't bluff, doesn't guess a hook from a boat sitting still, and doesn't hunt on the landing popup, so people may hunt in different ways.
