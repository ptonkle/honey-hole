# Playtests

Headless AI-only matches, measured by `cargo run --release --example playtest -- [matches] [seed]` (run from the repository root). Each match is four AI players on a Burt Lake layout picked by the match seed, 8 minutes, with the shipped `data/tuning.json` and `data/fish.json`. Half the matches are four Normal players; the rest mix the difficulties (Hard, Normal, Normal, Easy; Hard, Hard, Easy, Easy; Easy, Normal, Hard, Normal). A third argument (a seat) prints that seat's events and goals, for tracing one player through a match.

The AI plays by a seat's own senses (`crates/core/src/ai/senses.rs`): the fish in sight, rivals in sight or earshot, its own pings, and the hook popups. It hunts a rival's hook when it can get there before the reel ends and the fish is worth more than the fishing it gives up, and it keeps reeling a fish worth more than a wreck costs (SECOND-ORDER R4.48 to R4.52).

**Since these results**, a legendary's hook popup comes 20 s late and a rammed-off rare or legendary swims 80 m (`data/tuning.json` `balance`); the numbers with those, and every other option measured, are in `notes/balance-options.md`. Setting `balance.announce_delay_ms.legendary` to 0 and `balance.rammed_swim_mm` back to 12000 gives the game these results describe, number for number. A tuning value can be changed for one run: `-- 48 1 balance.announce_delay_ms.legendary=0`.

## Results, 2026-10-18

48 matches from seed 1 (`-- 48 1`), after the tuning below. Branch `ai`.

### Catches and points per player, by difficulty (mean per match)

| Difficulty | Players | Common | Uncommon | Rare | Legendary | Points | Wins |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Easy | 32 | 2.9 (36 pts) | 2.3 (119 pts) | 1.0 (134 pts) | 0.1 (43 pts) | 333 | 4 |
| Normal | 128 | 3.2 (41 pts) | 2.1 (100 pts) | 1.1 (150 pts) | 0.2 (77 pts) | 368 | 33 |
| Hard | 32 | 3.5 (44 pts) | 2.2 (113 pts) | 1.2 (154 pts) | 0.2 (100 pts) | 411 | 11 |

### Time spent (share of each player's match)

| Difficulty | Fishing | Hunting | Driving | Idle |
| --- | --- | --- | --- | --- |
| Easy | 53% | 10% | 27% | 9% |
| Normal | 50% | 10% | 29% | 11% |
| Hard | 48% | 11% | 27% | 14% |

Fishing is from switching to fishing mode until the fish is landed; driving is looking for fish, going to one, and dodging; idle is wrecked, waiting to respawn, or coasting with the Fish-Boy up.

### Wrecks, hooks and rams (mean per match, all four players)

| Hooks | Rams that wrecked | Groundings | Hooks lost to rams | Points knocked off lines |
| --- | --- | --- | --- | --- |
| 36.3 | 9.6 | 0 | 9.0 | 2,735 |

Points per minute spent fishing: 92. Points knocked off a rival's line per minute spent hunting: 855. The rammer went on to land 1.0 of the knocked-off fish a match itself (190 points a match, about an eighth of all points scored).

### What became of each hook (48 matches)

| Tier | Hooks | Landed | Line broke | Rammed off | Dropped (fled) |
| --- | --- | --- | --- | --- | --- |
| Common | 620 | 617 | 0 | 0 | 1 |
| Uncommon | 424 | 407 | 1 | 10 | 0 |
| Rare | 363 | 215 | 7 | 137 | 0 |
| Legendary | 337 | 37 | 7 | 284 | 0 |

### Fish left at the end, and reel times

| Tier | Left at the end (of those stocked) | Mean reel to a catch |
| --- | --- | --- |
| Common | 17.0 of 29.9 (57%) | 3.0 s |
| Uncommon | 11.8 of 20.3 (58%) | 7.1 s |
| Rare | 5.8 of 10.2 (56%) | 14.8 s |
| Legendary | 4.2 of 5.0 (85%) | 32.0 s |

Winner's margin over second place: mean 34%, median 38%, closest 0% (a tie on points), widest 78%.

### The same matches without hunting

For comparison, every difficulty's `aggression_pct` set to 0 (no AI hunts), the same 48 seeds: 781 points a Normal player (698 Easy, 741 Hard); 150 of 155 legendary hooks landed; 31% to 40% of each tier left at the end; winner's margin mean 18%, median 17%; 0.5 wrecks a match (bumps on the way past) and one grounding in 48 matches. Hard trails Normal here: it pings every 15 s, and each ping leaves its boat coasting for 3 s.

## Against the person's balance concerns

**Ramming should not pay better than fishing: it does.** A minute of hunting knocks 855 points off rivals' lines; a minute of fishing lands 92. In a four-player race, taking points from one rival is worth about a third as much as scoring them, which still leaves hunting ahead (about 285 a minute against 92). And it is not only denial: the freed fish swims 12 m off, next to the rammer, who lands it once a match. Legendaries are the target: 284 of 337 legendary hooks were rammed off, because a legendary's reel (32 s for a Normal player) is long enough for a rival anywhere on the lake to arrive, and the hooked boat cannot move (its motor is off in fishing mode). With hunting, every player scores less than half of what it scores without (368 against 781 for Normal).

**The lake does not run dry.** More than half of every tier is left at the end with hunting (57% of commons, 85% of legendaries), and a third or more without it. The counts in `data/fish.json` are enough for four players for 8 minutes; they could come down a little if the person wants the Fishpedia's counts to matter by the end.

**Legendaries should be worth chasing: they are, but seldom landed.** A legendary is worth 340 to 470 points, about a Normal player's whole match of other fish. With hunting, 11% of legendary hooks are landed; without it, 97%. Chasing one is worth it; landing one, with three rivals in the lake, seldom happens.

**The match should end in a close race: not yet.** The median winner leads second place by 38% with hunting, and by 17% without. A landed legendary (37 in 48 matches with hunting) decides the match it is landed in, and so does a run of rams on one player. Without hunting the race is close: most matches end within a fifth.

## Tuning applied

Each is a validate item in `PLAN.md` with its numbers.

1. **Rare and legendary fights eased** (`fishing.reel` in `data/tuning.json`): the rare fish's marker 580 to 540 thousandths of the bar a second and its darts 15% to 12%; the legendary's 640 to 560, its course kept 800 to 900 ms, its darts 25% to 18%. Before, a Normal AI's reel (measured on 200 reels a tier) took 80 s for a legendary and 19 s for a rare, and in 24 matches no legendary was landed (0 of 158 hooks). After: 33 s and 14 s; in play 32 s and 15 s, and with hunting turned off 97% of legendary hooks are landed. Perfect reel times (3 / 5 / 10 / 20 s), the tier multipliers and the hook window are unchanged.
2. **The ram stall left at 3 s.** Tried 10 s on the same 24 seeds (with an earlier AI): rams stayed at 10.8 a match, and the rammer's own catches of knocked-off fish went from 1.3 to 1.0 a match. A longer stall does not stop a rammer, who then fishes where it stopped.

## For the person to decide

The numbers say hunting a hooked rival pays several times better than fishing, through rules the person set (the hook popup, the motionless angler, the freed fish nearby). The fix is a rule, so it is the person's. Options, each easy to try:

- **Delay or blur the popup for rarer fish.** DECISIONS.md, round 1 (B4), says "leggendaries ar not announced for 20 seconds". If that means a legendary's hook is announced 20 s late, most legendaries would be landed before anyone hears (a Normal reel is 32 s, a Hard one about 25 s).
- **A rammed-off fish dives deep and far** (say 40 m instead of 12 m), so the rammer cannot land it straight after.
- **Gentler AI**: `ai.skill.*.aggression_pct` is one number per difficulty. Normal at 45 hunts nearly every rare or legendary hook it can reach in time.

## Cost

The AI costs 0.9 µs per AI player per tick on average in a native release build (`cargo run --release --example playtest -- 48 1`). A path search is the costly part: 52 ticks in 1,152,000 took the four AI players over 1 ms together, the slowest 6 ms. The budget was 0.3 ms per AI per tick (MEASUREMENTS.md).
