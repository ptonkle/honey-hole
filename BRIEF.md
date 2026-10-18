# Brief

Stage 1. The person's own words come first; the rest is written by the agent from them and read by the person. `C1 Frame the problem and set the rules`, `B1 Establish context`.

## The description, word for word

Given 2026-10-17. The two wireframes it names are `brief/fishpedia.png` and `brief/radar-minigame.png`. Later requests are numbered in `TRIAGE.md`.

```text
The new game is a fishing game. The game is called Honey Hole.


Style: the game is in 3d polygons like ape race. However, in this game I want to try to develop the fish and other models and save them off into their own files rather than them being dynamically created with Rust code as they are in ape race. maps are a bit different because they are semi-randomly generated. All elements should stay cartoony and while a fish should be able to be easily recognized by its size, shape, and markings, we are not going for realism here. 

Goal: The goal of the game is to catch the most value of fish.

However most of the gameplay is driven based on players trying to interfere with each other catching fish. When a player hooks a fish, it should alert all players in the game with a popup saying (PLAYERNAME has hooked a FISHTYPE!) The idea is if it's a sufficiently valuable fish, the other players should be incentivized to go mess with them. 

Map: we'll start with one map but we should keep the option open to create more in the future. The number of fish of each type should be set at the beginning of the match. The world should be medium foggy, you should not be able to see across the whole map to make it easier for players to hide and give value to the radar functionality. Map setting and environment is choosable but the map itself is randomly generated. So like the edges of the map are a bit random, placement of islands. This will work by pregenerating a bunch of maps and selecting one of them randomly. Give the option to see the map seed in the bottom right for debugging purposes in case we run into a map with issues.

First map: just a classic northern Michigan lake, trees, some small islands, sparse reeds, use your brain.

Powerups: there are a number of powerups players can obtain as items by driving their boat through a buoy that spawns once per minute. Give a notification when an item is spawning. That timing value should be easily updated. The powerup should attempt to be spawned a similar distance from all players to make it fair. You do not know what the item is until you pick it up. Each powerup also requires a small minigame to be played to use it. The minigame happens when you use it, NOT when you pick it up. The minigames are a bit like wario ware minigames, where there is a text popup that tells the player what they need to do to complete the minigame. There is no way to fail the minigame, but poor performance can make it take longer. Not all games are played on the pipboy but some are. you can only have 1 item at a time, picking up a new buoy tosses your old one and replaces it with the new one. Shift button to activate item (except items that are triggered by certain events like the camera).
- Cloaking tech: Hides the player from the radar for 1 minute.
  - Minigame: pull up the Fish-boy and there is a screen with an overhead view of a boat. You need to use your mouse to erase the boat like you would in Microsoft Paint, don't require 100% erasure as that could be frustrating, like 98% should be good.
- Power Reel: Gives the player 2x reeling power/speed for 20 seconds
  - a 2d reel (circle) appears and you need to make your mouse circle it clockwise 5 times to complete it 
- Nitrous: Allows the player to accelerate faster and reach a much higher top speed. You should also get more responsive controls so that it doesn't become hard to control with this on. 15 seconds.
  - Top-down, There are 8 "N" (nitrogen) balls (atoms) bouncing around the screen and 4 O (oxygen) ones bouncing around too. You need to click, drag, and unclick to drop them into a section labeled "Nitrogen" or "Oxygen". level finishes when you drop them all in 
- Teleporter: Allows you to teleport to somewhere else on the map after a 5 second charge. single use. Using it brings up the radar and lets you click somewhere on it to teleport there.
  - OSU-like game where there are 2d circles that appear around the screen. A ring circle appears around the and you need to click within the circle when the edge of the ring touches the edge of the circle. Circles keep spawning until you complete the game by getting 10 right. 
- High-Quality Camera: Can be used after catching a fish to increase its value by 1.5x. When you have it popup that asks the player if they want to use the camera once they get the fish with options "Use camera" vs "Save for later"
  - fish fly in from the edges of the screen at random intervals, and you need to click them 10x to clear the game. Would be good if the mouse could change to be like a camera with liek the corners of a box and then an x in the middle. kinda like fruit ninja but you just need to click them. 
- Enhanced Radar: brings up fishboy, makes you do a radar ping but this one also reveals the location of one fish of each tier. 
  - In this one, there is a blurry fish image and 3 vertical sliders. There is hidden, randomized sweetspot on each of the sliders that is the most in focus, moving towards that sweetspot should bring it more in focus, away makes it less in focus. Once you get within 10% of each of the sweetspots it snaps to being fully clear for a second then you clear the game. SEe radar minigame.png. 

Player kills: If you crash into another player going fast enough (let's say 80% of max speed without nitrous), they blow up and are sent back to the spawn point after a 2 second delay. They do not lose anything except the if they were reeling in a fish that is stopped.
Player also dies for running into land if they're going over 50% speed (again no nitrous), otherwise they just kinda bounce off.
All players are told with an onscreen popup when a player dies.

Player colors: Each player is assigned a color based on their slot in the lobby. Blue, red, green, yellow.

Fish-Boy:
The fishboy is modeled after the Pip-Boy from fallout new vegas. it contains tech functionality that the player uses, such as the radar and the fish log. It should be rendered as a 3d model with a screen in front of the player's face like how the pipboy is, not just a dialog the pops up. The radar should be like how a pip boy shows up in front of the player from fallout as like a little low tech screen that is actually held up to the player's face. The fish-boy screens should fill 80% of the screen, and the bezel around it should fill 10% and then you can see the live background rendered in the remaining bits around the end. you can pull up the fishboy at any time. Tabs at the top for accessing different functionalities. Default to last viewed screen when opening it.

Radar:  Pip-boy functionality. The radar shows a green radar-like, low fidelity, overhead view of the map, including the position of the buoy if it's present and each of the other players. It also needs to show the direction your own boat is pointing. The player color is also used on the radar view to identify them, but also put their username on the radar. You have to manually ping the radar though and the pings have a 5 second cooldown (per player). The ping should take about 3 seconds to fully scan the map, and it does so in a circle centered from the player. The item should be visible at all times without pinging.

Fishpedia:
Pip-Boy functionality. This is where you see what fish were spawned on the map, and how many of that type of fish is left. Remaining number of fish is public knowledge. The tier of each fish is shown too. orange theme for Legendary, Purple for Rare, Blue for Uncommon, Green for Common. See Fishpedia.png for a wireframe. Each blue thing with the brown circle shows the idle swimming animation of the fish. Also at the top of each box. put the name of the fish.

Boats: The player boats are like those medium-low profile fishing motorboats. You should be able to get going quite fast in them and you can get up to top speed quite quickly, like over the course of 5 seconds. crossing the full map maybe takes 30 seconds at full speed. Player boat color should be the color of the player.
When a player is driving the boat they are sitting on the back bench next to the outboard motor. When they are fishing they are standing at the prow. You can swap between modes instantly. momentum is conserved when switching modes. Boat can back up but is relatively slow. 

Time: A game of this lasts 8 minutes

Controls: 
- Boat:
  - W to go forward, s to back up, a to turn left, d to turn right. q to switch to fishing mode
- Fishing
  - q to switch to driving.
  - a to swivel left, d to swivel right. space to cast (detailed below)
- Tab brings up fish-boy
- Escape to cancel your current thing, like quit a minigame (wastes the item), stop looking at fishboy, drop the fish off the line, etc.
- The game needs to be able to be played with right and always on the mouse and left hand always positioned at WASD.

UI: 
  - When your line is in the water the screen should be all an underwater cam looking at the hook wher eyou can swivel the camera around with your mouse. There is a picture in picture of your guy on your boat in the upper left corner. 

Fishing Mechanics: hold space to start charging, release to cast based on how long you held it down, you see the bobber dropping, hit space to stop it dropping. Fish bites and backs off a number of times like animal crossing before it takes the bait, at which point you have a small window let's try .75 seconds to hit space to hook it, at which point it becomes the stardew valley minigame where you need to hold space to move a section inside another bar to keep the fish onthe line, otherwise the line breaks. WHile this is going on the fish is pulled to the surface and is in a panicked fast swim animation. once you catch a fish there is no way to lose it. 

Fish: Fish come in 4 tiers, Legendary, Rare, Uncommon, and COmmon. Fish value is determined by size and rarity and roughly corresponds with how difficult it is to reel them in and how long it takes. So for example a shark would put a big fight and take a long time to reel in and therefore we worth a lot, a clownfish would not be worth as much. See the ANimal Crossing games for good inspiration on a selection of fish sizes, rarities, and values. Fish should be able to vary about 1.5x in size (length), so for example you can catch a tuna that is 4 feet long to 6 feet long. don't feel the need to use those exact numbers for that fish. Measure size in feet down to a hundredth decimals. bringing in a legendary fish should take 20 minutes if you were to play the fishing minigame perfectly, 10s for legendary, 5 for uncommon and 3 for common. Example spread of fish, don't follow this exactly home some random variance on map start: 30 common, 20 uncommon, 10 rare, 5 legendary. if common is worth 1x, uncommon is 1.5x, rare is 2x and legendary is 3x. common and uncommon should spawn in schools of 2 to 4, rare and legendary do not spawn in schools and should intentionally be far apart. Use the colors from the fishboy as an additional visual reminder when displaying fish, such as in the postgame screen or the message that displays when a player hooks a fish. 

Camera: The camera is 3rd person. When in pip-boy it is effectively in first person though.

Sound: Sounds should accompany most actions:
- casting a line
- hooking a fish
- An important sound is the sound of an approaching player, you should be able to hear their motor getting louder, starting when they're about 5 seconds away (if they're going at full speed). Also attempt to do a doppler effect. Do not attempt to do directional sound.
- radar ping noise
- minigame sounds
- match start sounds
- explosion
- 5 sound countdown when an item is spawning
- etc.

Others:
- All players start at the same spawn points on one side of the map. You also respawn from this point. 
- Up to 4 players
- points are only tallied at the end a la mario party to keep players in suspense.
- post game screen shows each fish a player caught one after another and tallies up their points. 

Execution: 
- Start with some models of fish, boats, world, minigames, radar view, fishpedia, and put them in the development journal to get the early OK.
- Also start figuring out the map generation.
- Ask many questions about the game design as soon as possible

Implement later:  Give fish habitats that they are more commonly found in e.g. this fish is often found in reeds.
```

Later the same day: "Also point out any things that you think would not be well balanced or would encourage anti-fun player behaviors such as players spending all game just destorying each other rather than fishing" and "oh also develop AI players to play against and also to be used for playtesting".

Earlier the same day the person also said: online play is part of the game, but its sync model must come from this game's own player interactions, not from Ape Race's (`DECISIONS.md`, "2026-10-17 (online)").

## The tool

**Honey Hole** is a 3D online fishing game in the browser for up to 4 players, played with mouse and keyboard. A match lasts 8 minutes, and the player whose catch is worth the most at the end wins. Most of the play is in the gaps between catches: a hooked fish is announced to everyone, and rivals race over to ram the angler before it's landed. Powerup items keep the fight going.

The game lives in `slop/honey-hole/` (renamed from the working name `fishing/` on 2026-10-18).

### Look

- Cartoony low-poly 3D, like Ape Race, but every model is its own file under `data/`, not built in Rust code. Maps are the exception: they are generated from a seed.
- A fish is recognisable by its size, shape and markings, without realism.
- The world is medium foggy: no one can see across the map, so players can hide and the radar matters.
- The camera is third person; with the Fish-Boy up it is effectively first person.

### Match

- 8 minutes. Up to 4 players, coloured by lobby slot: blue, red, green, yellow (boat, radar marker and name).
- Everyone starts and respawns at the same spawn point on one side of the map.
- Scores stay hidden until the end ("a la Mario Party"). The postgame screen shows each player's catches one after another and adds up their points.

### Maps

- One map at first (a northern Michigan lake: trees, small islands, sparse reeds), with room for more settings later.
- The setting is chosen; the layout (shoreline, islands) comes at random from a set of pregenerated maps.
- An option shows the map seed in the bottom-right corner for debugging.
- The number of fish of each kind is fixed at the start of the match, with some random variance around a spread such as 30 common, 20 uncommon, 10 rare, 5 legendary.
- Common and uncommon fish swim in schools of 2–4. Rare and legendary fish are single and deliberately far apart.

### Boats

- Medium-low-profile fishing motorboats in the player's colour. Top speed in about 5 s; crossing the map takes about 30 s at top speed. Reversing is slow.
- **Driving:** the player sits on the back bench by the outboard. **Fishing:** the player stands at the bow. Switching modes is instant and keeps momentum.
- **Ramming:** a boat that hits another at 80% or more of top speed (without nitrous) blows it up. The wrecked player respawns after 2 s, keeps every fish already caught, and loses only a fish being reeled in.
- **Grounding:** hitting land above 50% of top speed (without nitrous) blows the boat up; slower, it bounces off.
- Every player gets a popup when any player is wrecked.

### Fishing

1. Hold Space to charge the cast and release to cast; the bobber drops, and Space stops it at that depth.
2. A fish nibbles and backs off a few times (as in Animal Crossing), then takes the bait. The player then has a 0.75 s window to press Space and hook it.
3. On a hook, every player gets a popup: "{player} has hooked a {fish}!", tinted with the fish's tier colour.
4. The reel: a Stardew Valley style bar. Space moves a zone inside a bar to keep the fish on the line; if it slips out, the line breaks. Meanwhile the fish is pulled to the surface in a fast, panicked swim.
5. A landed fish can't be lost.

While the line is in the water, the whole screen is an underwater camera on the hook, swivelled with the mouse. A picture-in-picture of the player on the boat sits in the top-left corner.

### Fish

| Tier | Colour | Value multiplier | Perfect reel time | Schools |
| --- | --- | --- | --- | --- |
| Legendary | orange | 5× | 20 s | single, far apart |
| Rare | purple | 3× | 10 s | single, far apart |
| Uncommon | blue | 2× | 5 s | 2–4 |
| Common | green | 1× | 3 s | 2–4 |

- A fish's value comes from its kind (tier) and its length. Lengths vary about 1.5× within a kind and are shown in feet to two decimals.
- Bigger and rarer fish fight harder and take longer to reel in. Animal Crossing's sizes, rarities and values are the inspiration.

### Items

- **Buoys:** an item buoy spawns once a minute, at a similar distance from every player. The timing is one easily changed value.
- **Spawn warning:** a notification and a 5-beat countdown sound warn that a buoy is about to spawn.
- **Radar:** the buoy shows on the radar at all times.
- **Picking up:** driving through the buoy gives a hidden item. A player holds at most one; a new buoy replaces the old item.
- **Using:** Shift uses the item, except the Camera, which offers itself after a catch.
- **Minigames:** using an item first plays a short WarioWare-style minigame with a one-line instruction. It can't be failed, only finished slowly. Some minigames are on the Fish-Boy, some aren't. Esc quits the minigame and wastes the item.

| Item | Effect | Minigame |
| --- | --- | --- |
| Cloaking Tech | hidden from the radar for 1 min | on the Fish-Boy: rub out an overhead picture of a boat, Paint-eraser style, to about 98% |
| Power Reel | 2× reeling power and speed for 20 s | circle a 2D reel clockwise 5 times with the mouse |
| Nitrous | faster acceleration, much higher top speed and sharper steering for 15 s | drag 8 bouncing N atoms and 4 O atoms into "Nitrogen" and "Oxygen" bins |
| Teleporter | single use: pick a point on the radar, teleport there after a 5 s charge | osu!-style: click circles as the closing ring meets their edge, 10 hits |
| High-Quality Camera | a landed fish is worth 1.5×; offered after a catch ("Use camera" / "Save for later") | click 10 fish flying in from the screen edges, with a viewfinder cursor |
| Enhanced Radar | a radar ping that also reveals one fish of each tier | on the Fish-Boy: bring a blurred fish into focus with 3 sliders, each with a hidden sweet spot; within 10% of all three it snaps sharp for a second (`brief/radar-minigame.png`) |

### Fish-Boy

- A wrist computer modelled on Fallout: New Vegas's Pip-Boy, drawn as a 3D model raised to the player's face, not a dialog.
- The screen fills 80% of the view, the bezel 10%, and the live world shows in the rest.
- Tab opens it at any time, with tabs along the top. It opens on the last tab viewed.
- **Radar:**
  - A green, low-fidelity overhead view showing the map, your own heading, the buoy (always), and other players in their colour with their name.
  - A manual ping (5 s cooldown per player) sweeps out in a circle from the player and takes about 3 s to cover the map.
- **Fishpedia:**
  - Each kind of fish spawned on this map, grouped by tier (orange, purple, blue, green frames), with its name, how many are left (public) and a box of its idle swim (`brief/fishpedia.png`).

### Sound

- **Actions:** the cast, hooking a fish, the radar ping, the minigames, the match start, explosions, the 5-beat buoy countdown, and more.
- **Approaching boats:** an approaching rival's motor grows louder from about 5 s away at top speed, with a Doppler shift. There is no directional sound.

### Controls

Left hand on WASD, right hand on the mouse, always.

| Key | Driving | Fishing |
| --- | --- | --- |
| W / S | forward / slow reverse | — |
| A / D | turn left / right | swivel left / right |
| Q | switch to fishing | switch to driving |
| Space | — | charge and cast; stop the bobber; hook; reel |
| Shift | use item | use item |
| Tab | Fish-Boy | Fish-Boy |
| Esc | cancel the current thing: quit a minigame (item wasted), close the Fish-Boy, drop the fish off the line | same |

### AI players

- AI players fill seats to play against, offline or online, and run whole matches headless in the core for playtesting.
- Playtests measure balance: catches and value per player, time spent fishing against hunting, wrecks, and item use. This shows whether ramming pays better than fishing.
- The AI is a core module, deterministic from the match seed, and drives a boat through the same inputs a person does.

### Answers to round 1

The person's answers to `brief/questions.md` are recorded in `DECISIONS.md` (2026-10-17, round 1) and override the description where they differ. Every fish could live in a Midwest lake. Fish are visible below the surface. Rarer fish swim deeper, and bobber depth matters. Every hook is announced in a popup of about 2 s. Ramming stalls the rammer, and a respawn ghosts the player for a few seconds.

### Later

- Fish habitats: some fish found mostly in, say, reeds.
- Player interactions beyond ramming, which the online design leaves room for.
- More map settings.

## Done means

The person checks these before a first public release. Each is checkable by playing or by a named test.

- Four players join one match by room code and play a full 8-minute match to the postgame screen in Chromium, Firefox and WebKit.
- Every fish, boat, buoy, Fish-Boy and world prop model is loaded from its own file under `data/`, and none is built in Rust code.
- The map comes from a set of pregenerated seeds, and its seed shows in the bottom right when the option is on. Two matches on the same seed have the same shoreline and islands.
- Fish counts per kind are fixed at match start, schools of common and uncommon are 2–4, and rare and legendary fish are spaced apart (core tests).
- The cast, bite, 0.75 s hook window and reel bar play as described, and a perfect reel takes each tier's reel time (core tests, by tier).
- A hook pops up for every player with the hooker's name, the fish and its tier colour.
- A ram at 80% of top speed wrecks the other boat; 79% does not. Land at over 50% wrecks; under that, the boat bounces (core tests). Every player sees the wreck popup.
- An item buoy spawns every minute (one value in `data/`), after a warning and countdown, at a similar distance from every player (core test with stated tolerance).
- All six items work, each after its minigame, and Esc in a minigame wastes the item.
- The Fish-Boy opens with Tab on its last tab and fills 80% / 10% of the view. The radar ping sweeps for about 3 s, with a 5 s cooldown. The Fishpedia shows counts left by tier.
- The postgame screen reveals each player's catches one by one and totals them.
- The sounds listed above play, including a rival's motor with a Doppler shift from about 5 s away.
- AI players fill empty seats and play a full match; a headless batch of AI-only matches reports the balance numbers above.
- `./scripts/check.sh` passes, and the online check passes.

## Decisions that stay with the person

- The fish roster: kinds, tiers, lengths and values.
- How players interact online, and so the sync model (`DECISIONS.md`).
- The look of every model, approved through the design journal before it ships.
- Map settings beyond the first lake.
- Any balance number the brief names: match length, item durations, ram and grounding speeds, buoy timing.
- The name of the folder and repository.
- Whether and where to publish.

## Accessibility

- Tiers are shown by colour and always also by their name (and frame shape in the Fishpedia), never by colour alone.
- Players are shown by colour and always also by their name.
- Popups carry text; important sounds also have an on-screen cue.
- Input is keyboard and mouse only, with one-handed reach: left hand on WASD (plus Q, Shift, Tab, Space, Esc), right hand on the mouse.
- No gamepad or touch at first.

## Style

Not the template's `default` look. Cartoony low-poly 3D with flat colours and fog. The Fish-Boy screens are green-phosphor, low-tech (Pip-Boy). The look is approved per model and screen through the design journal (`.claude/skills/design-journal`).

## Words

Terms the person uses, to go into `GLOSSARY.md` before any string uses them:

- Fish-Boy (the person also writes "fishboy" and "pip-boy")
- Radar, ping
- Fishpedia
- tier (Common, Uncommon, Rare, Legendary)
- item, powerup, buoy
- minigame
- hook, cast, bobber, reel
- school (a group of fish)
- driving mode, fishing mode
- wreck (the person says "blow up" and "dies")
- spawn point
- match
- postgame

## Autonomy

Since 2026-10-17 (overnight) the plan is approved in advance: the agent builds until the game is finished and playable, decides by its own judgement, and records decisions for the person to validate (`DECISIONS.md`). Before that grant: the plan was not approved in advance. The agent writes `PLAN.md` and agrees with the person on design questions before building the game itself. The agent may start, before that, on the person's execution list:

- models of fish, boats, the world, the minigames, the radar view and the Fishpedia, shown in the design journal for an early OK;
- map generation.

Where this brief is silent, the agent asks if the answer changes the design; otherwise it decides, records the decision in `SECOND-ORDER.md`, and continues. The agent may not loosen a Builder requirement or a boundary test. It may not add a server, an account or analytics, and may make no off-origin request except the pinned relays the online design will need, once agreed. Since 2026-10-18 the agent may publish to the public repository ptonkle/honey-hole and its GitHub Pages site with `./scripts/publish.sh` once `./scripts/check.sh` passes (`DECISIONS.md`). Before that, local deploys only.

## Allowed core dependencies

`serde`, `serde_json` (reference default). Add others only here, with the reason.
