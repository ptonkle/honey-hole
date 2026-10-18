# Morning briefing, 2026-10-18

What was built overnight, what needs your decision, and what to try. The full list of choices made alone is in `PLAN.md` ("For the person to validate"); this page is the short version.

## Try it

- Local game: http://127.0.0.1:8090/ (rebuilt after every merge).
- Design journal: https://claude.ai/artifact/TeFhUVC6nLvhasHW726N6U
- **Play → Offline:** a full 8-minute match against up to 3 AI players: countdown, fishing, items, wrecks, the postgame reveal.
- **Play → Online:** host a room code, join from a second machine.
- **Workshop** (on the title): Gallery (every model, true-size fish line-up), Lake preview (fog slider), Sounds (every sound and a rival motor passing), Minigames (all six).
- Debug addresses:
  - `?debug=watch`: a Hard AI plays your seat;
  - `?debug=fishing`: a fish by every bobber;
  - `?debug=items,item.nitrous`: a buoy ahead that gives that item;
  - `?debug=flow`: F8 jumps to the end, F9 wrecks you.

## Needs your decision

1. **Ramming balance.** In AI playtests ramming paid about nine times what fishing did, and 89% of legendary hooks were rammed off. I applied two reversible changes that contradict nothing you set:
   - a legendary's hook popup comes 20 s after the hook (my reading of your "leggendaries ar not announced for 20 seconds");
   - a rammed-off rare or legendary fish swims 80 m away.

   Result: 64% of legendaries are landed, fishing scores 146 a minute, there are about 5 rams a match, and the winner's lead is 21%.
   - Keep it, or undo it with `balance.announce_delay_ms.legendary` = 0.
   - Other options, with numbers, are in `notes/balance-options.md`. The next best would change your 80% rule: a boat that's reeling is harder to wreck.
2. **Items and the coasting rule.** You chose that the boat coasts during minigames. The playtests show this makes Nitrous, the Teleporter and Cloaking Tech nearly useless (the coast eats the gain), while the Camera is the strongest item (+78 points a use). Options:
   - the boat keeps its throttle and course during a minigame;
   - a shorter Nitrous minigame;
   - the Camera made rarer.

   Nothing is changed yet.
3. **The look of things you haven't seen:**
   - the title wordmark;
   - the menus;
   - the underwater lake;
   - the 3D lake and fog (pick the fog by eye in Lake preview, then the value goes in `sight_mm`);
   - the item effects;
   - the six minigames;
   - the six newer fish and the reworked catfish.

   All are in the journal.
4. **Sounds.** All 46 are synthesised and are my judgement. Audition them on the Sounds screen and say which to change. The rival motor's Doppler is exaggerated 3× so it's audible.
5. **The folder rename to `honey-hole/`.** It waits for you, so no helper is mid-work inside `fishing/` when it happens.

## Only you can do

- Try a room code between two real machines; no automatic check uses the public relays.
- Note that two players both behind strict home routers can't connect, since there's no relay (TURN) server.
- Publishing stays local until you say otherwise.

## What went wrong overnight (all fixed)

- The first online build was lost because I started its helper from the wrong folder; it was rebuilt.
- A usage limit stopped every helper around 1 a.m.; they resumed after it reset.
- An automatic merge broke the code once and was undone, and a planted test break slipped into one commit and was reversed. Main has passed the full check after every merge since.
