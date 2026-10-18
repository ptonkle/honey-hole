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

## Results with items, 2026-10-18

48 matches from seed 1 (`-- 48 1`), each checked again on seeds 1001 to 1048, with the balance settings as shipped (a legendary's hook popup 20 s late, a rammed-off rare or legendary swims 80 m) and the AI now using items (`crates/core/src/ai/items.rs`, its hand on the minigames in `ai/hand.rs`; SECOND-ORDER R5.30 to R5.46). Items come as before: a buoy a minute from the first minute, each item equally likely. Two switches in `ai.items` give the comparisons: `use_items` false (no AI goes for a buoy or uses an item: the game of `notes/balance-options.md`, number for number), and `use_held` false (it drives to buoys but never uses what it picks up: what the trips cost). A run with one item alone sets the other five weights to 0, for example `-- 48 1 items.weights.cloak=0 items.weights.power_reel=0 items.weights.nitrous=0 items.weights.teleporter=0 items.weights.enhanced_radar=0` for the camera alone.

### The balance table (as in `notes/balance-options.md`), seeds 1 to 48

| Option | Fishing / min | Knocked off / hunting min | Rammer lands / hunting min | Landed / player | Knocked off / player | Legendaries landed | Margin | Hunting | Fish left | Rams |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| No items (`ai.items.use_items` false) | 146 | 848 | 150 | 611 | 211 | 119 of 187 (64%) | 22% / 21% | 3% | 48% (51%) | 4.8 |
| Buoy trips, items never used (`use_held` false) | 145 | 847 | 115 | 596 | 203 | 116 of 183 (63%) | 20% / 20% | 3% | 48% (52%) | 4.6 |
| **Items, as shipped** | **152** | **820** | **125** | **632** | **210** | **120 of 188 (64%)** | **29% / 31%** | **3%** | **48% (50%)** | **4.8** |
| Cloaking Tech alone | 147 | 742 | 151 | 598 | 191 | 118 of 174 (68%) | 27% / 24% | 3% | 49% (51%) | 4.8 |
| Power Reel alone | 156 | 659 | 106 | 631 | 167 | 125 of 164 (76%) | 24% / 26% | 3% | 46% (48%) | 4.9 |
| Nitrous alone | 145 | 843 | 115 | 594 | 201 | 114 of 180 (63%) | 20% / 20% | 3% | 48% (53%) | 4.6 |
| Teleporter alone | 145 | 777 | 130 | 604 | 185 | 114 of 171 (67%) | 24% / 24% | 3% | 46% (53%) | 4.3 |
| High-Quality Camera alone | 160 | 841 | 157 | 666 | 212 | 103 of 167 (62%) | 26% / 25% | 3% | 49% (57%) | 4.9 |
| Enhanced Radar alone | 148 | 836 | 145 | 606 | 218 | 126 of 198 (64%) | 20% / 18% | 3% | 48% (48%) | 4.8 |

### The same on seeds 1001 to 1048

| Option | Landed / player | Knocked off / player | Legendaries landed | Margin | Rams |
| --- | --- | --- | --- | --- | --- |
| No items | 619 | 188 | 122 of 183 (67%) | 21% / 22% | 4.5 |
| Buoy trips, items never used | 602 | 215 | 120 of 188 (64%) | 22% / 18% | 4.9 |
| **Items, as shipped** | **589** | **236** | **107 of 183 (58%)** | **31% / 31%** | **5.2** |
| Cloaking Tech alone | 582 | 217 | 117 of 183 (64%) | 25% / 21% | 5.3 |
| Power Reel alone | 658 | 163 | 133 of 172 (77%) | 20% / 20% | 4.3 |
| Nitrous alone | 603 | 214 | 118 of 187 (63%) | 22% / 18% | 4.9 |
| Teleporter alone | 594 | 197 | 116 of 182 (64%) | 25% / 24% | 4.8 |
| High-Quality Camera alone | 694 | 196 | 118 of 180 (66%) | 26% / 22% | 4.6 |
| Enhanced Radar alone | 630 | 229 | 129 of 211 (61%) | 24% / 23% | 4.9 |

The same setting differs by up to about 25 points a player between the two runs of 48 (632 against 589 with items as shipped), so a difference under about 20 points is noise.

### Items: pickups, uses and minigames (seeds 1 to 48, as shipped)

| Item | Picked up / match | Used / match | Minigame to effect, Easy / Normal / Hard |
| --- | --- | --- | --- |
| Cloaking Tech | 1.29 | 1.04 | 6.4 s / 3.9 s / 3.7 s |
| Power Reel | 1.17 | 0.85 | 7.2 s / 4.6 s / 3.4 s |
| Nitrous | 0.92 | 0.04 | 14.3 s / 8.8 s / 5.9 s |
| Teleporter | 0.88 | 0.62 | 11.4 s / 9.4 s / 9.4 s |
| High-Quality Camera | 0.85 | 0.65 | 9.5 s / 8.9 s / 8.8 s |
| Enhanced Radar | 0.94 | 0.90 | 5.0 s / 4.0 s / 3.4 s |
| All | 6.04 | 4.10 | |

A match has 7 buoys (one a minute from the first minute); the AI took 6.0 of them. An item not used was held to the end or replaced by the next pickup. Nitrous is seldom used: the AI keeps it for drives of 400 m or more and hunts 120 m or more (below). The minigame times are from the runs with that item alone (many more games), from Shift to the effect; the Teleporter's and the camera's are paced by their circles and fish (a perfect Teleporter run is about 9.2 s), the rest by the hand. None was wasted except one Teleporter whose charge a wreck lost.

### What each item adds a use

Points landed per player against the buoy-trips run (the same trips, no item used), mean of the two runs of 48, divided by the uses per player.

| Item | Points a player, alone (two runs) | Against trips only (599) | Uses a player | Points a use |
| --- | --- | --- | --- | --- |
| High-Quality Camera | 680 | +81 | 1.04 | **+78** (the photos themselves add 94 a photo on seed 1: half a rare or legendary catch) |
| Power Reel | 645 | +46 | 1.13 | **+40** (and legendaries landed rise from 64% to 76%) |
| Enhanced Radar | 618 | +19 | 1.41 | +13 (within the noise) |
| Teleporter | 599 | 0 | 1.15 | 0 |
| Cloaking Tech | 590 | -9 | 1.28 | -7 (within the noise) |
| Nitrous | 599 | 0 | 0.07 | none measurable; used on every drive of 200 m or more it cost 12 points a use |

The buoy trips themselves cost 16 points a player a match (615 with no items, 599 with trips and no use). With all six items in the mix, the points landed come out level with no items (611 and 619 without; 632 and 589 with), but **the winner's margin widens from 21% to 30%**: the camera and the Power Reel go to whoever lands the rare fish, and Hard players, who reach the buoys and finish the minigames quicker, gain the most (Hard 705 against Easy 502 on seed 1, from 671 against 486 without items).

### Is any item too strong or useless?

- **The High-Quality Camera is clearly the strongest:** about 78 points a use, an eighth of a player's match, and never wasted (the AI saves it for a rare or legendary catch, or any catch in the last 45 s). It is the main reason items widen the winner's margin.
- **The Power Reel is good:** about 40 points a use, mostly by landing more legendaries before a rival arrives. Using it at the hook is now safe, because the reel holds while its minigame plays (R4.11, below).
- **Nitrous, the Teleporter and Cloaking Tech are close to useless for AI players.** Each one's minigame leaves the boat coasting for 4 to 15 s, which costs about as much as the effect gains: 15 s of nitrous at 150% saves at most 7.5 s on a straight run, but its minigame takes 6 to 14 s. The Teleporter never reaches a hunt in time (its 9 s minigame and 5 s charge are longer than any announced reel has left), so the AI teleports only to far water. Cloaking Tech hides a boat from the radar only; a hunter that can see or hear its target still finds it, and the AI hunters mostly do. People may do better with all three, since the radar matters more to people.
- **Enhanced Radar** is a small plus.

**Tuning proposal** (not applied; each is a validate item): the brief names every one of these items' durations, so the only weight or duration changes allowed would be the item weights. Lowering the camera's weight (10 to 6) would narrow the margin a little, but makes the other, weaker items more common, so I left the weights equal. The fix that would make Nitrous and the Teleporter worth their minigames is a rule, so it is the person's: let the boat keep its throttle and course while a full-screen minigame plays, instead of coasting (`Boat::captured`). A shorter Nitrous minigame (slower atoms) would help too; the brief names its 8 N and 4 O atoms.

### R4.11: the Power Reel during a reel

A minigame takes the keys, so before this change a Power Reel used with a fish on the line left the reel bar fighting with no Space, and the fish could break the line (SECOND-ORDER R4.11). Now **the reel holds still while any item's minigame plays, or a teleport's spot is picked** (`items.reel_holds_in_minigame`, true): the catch zone, the fish's marker and the meter wait, and pick up where they were. A ram still snaps the line. The other option, refusing the Power Reel while reeling with a "not now" cue, would push it before the cast, where its 20 s run out during the cast, the sink and the nibbles; holding the reel lets it be used when it is wanted, at the cost of the seconds its minigame takes (SECOND-ORDER R5.31). With the hold off, the game is as before.

### The AI's item tuning (`ai.items`)

Set from these runs: the buoy is worth a trip of 15 s with the slot empty (25 s cost 15 more points a player than 15 s, and 10 s took fewer buoys for the same points) and 6 s with an item held; Nitrous for drives of 400 m or more and hunts of 120 m or more (200 m cost 15 points a player); Cloaking Tech before the cast at a rare or legendary fish, and on hunts 100 m or more off; the Power Reel at the hook of a rare or legendary (before the cast it gave about half as much). Each difficulty's hand (`ai.items.hand`): the pointer's speed 1400 / 2400 / 3800 canvas units a second, a pause of 450 / 250 / 120 ms after each grab, drop or click, its aim off by up to 26 / 14 / 6 units, its click on a closing ring off by up to 240 / 130 / 60 ms, and the Power Reel circled at 0.9 / 1.6 / 2.6 turns a second.

AI cost with items: 1.6 µs per AI player per tick on average (1.2 before); 430 ticks in 1,152,000 took the four AI players over 1 ms together, the slowest 22 ms (a path search).
