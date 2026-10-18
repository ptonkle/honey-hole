# Glossary

The source of truth for what each word in this game means. Strings, tests, the plan and the README use a term in the sense given here. The page shows no definitions on screen (the person's choice, `DECISIONS.md`, 2026-10-17); this file and its copy tests keep the words straight.

Write each entry as a `### Term` heading, then one or two plain sentences, then an optional `Not:` line listing words that must not be used for it. A number a rule decides is a `{placeholder}` the core fills, so the glossary cannot disagree with the rules. Keep entries in alphabetical order. `crates/core/tests/copy.rs` checks the format, the placeholders and the `Not:` words; `TONE.md` governs the prose.

<!--
An example entry. It sits inside this comment, so it does not count.

### Spout

Where slush comes out. Each line has up to {line.spouts} spouts.

Not: tap, dispenser, nozzle
-->

### AI player

A seat the computer plays, with a name of its own and a difficulty: Easy, Normal or Hard. It drives a boat by the same rules as a person.

Not: bot, CPU, computer player, NPC

### Animation

A model's movement, played in a loop: a fish's idle swim and panic swim, the angler's driving and fishing poses, a plant's sway. Still shows the model at rest.

Not: clip, motion, tween

### Angler

The figure each player plays: it sits on the boat's back bench to drive and stands at the bow to fish. Its shirt and hat band wear the player's colour.

Not: fisherman, avatar

### Back

The button at the foot of each screen opened from the Title. It goes back to the Title, as Esc does.

Not: return, exit

### Bite

The moment a fish takes the lure, after its nibbles. Space within {fishing.hook_window_ms} ms of it hooks the fish; later, the fish swims off.

Not: strike, take

### Boat

Each player's small aluminium fishing boat, painted in the player's colour, with an outboard motor at the stern and a raised casting deck at the bow.

Not: ship, vessel

### Bobber

The small float on the line that shows where the lure is. It dips when a fish nibbles or bites.

Not: indicator

### Burt Lake

The first map: a northern lake with pine and birch shores, small wooded islands and reeds in the coves, in medium fog. Each match on it plays one of its checked layouts.

Not: Michigan Lake, level, course

### Cast

Sending the line out in fishing mode: hold Space to charge and let go to cast, further the longer it was held. A boat moving too fast cannot cast.

Not: chuck, lob

### Catch meter

The bar beside the reel bar that fills while the fish is in the catch zone and drains while it is out. Full, the fish is landed; empty, the line breaks.

Not: progress bar, tension bar

### Catch zone

The part of the reel bar that Space lifts and gravity pulls down. Keeping the fish inside it fills the catch meter.

Not: green bar, box

### Cloaking Tech

An item that hides its player from every radar's ping for a while: blips of them already on a radar are wiped. The boat still shows in the lake, see-through to its own player.

Not: invisibility, stealth

### Controls

The keys the game uses, listed on the Title.

Not: keybindings, hotkeys, bindings

### Driving mode

The boat's mode for getting about: the angler sits on the back bench by the motor, W and S drive, and A and D turn. Q switches to fishing mode at once, keeping the boat's speed.

Not: boating mode, sailing

### Enhanced Radar

An item that pings the radar at once and marks the nearest fish of each tier on its player's radar until their next ping.

Not: sonar, fish finder

### Fish-Boy

The angler's computer, strapped to the left forearm. Raised to the face, its screen fills most of the view and shows the game's tabs, with the lake still in sight round its edge.

Not: Pip-Boy, pipboy, fishboy, PDA
### Fishpedia

The Fish-Boy's tab that lists each kind of fish in this match, in rows by tier, each with its name, a box of its idle swim and how many are left. The counts are known to everyone.

Not: encyclopedia, bestiary, fish log

### Fishing mode

The boat's mode for fishing: the angler stands at the bow, the motor is off so the boat coasts to a stop, and A and D swivel the cast. Q switches back to driving mode.

Not: angling mode, cast mode

### Fog distance

How far a player can see across the lake before the fog hides everything. The fog thickens from part of the way out and is whole at this distance.

Not: draw distance, visibility, view range

### Gallery

The screen, opened from the Title, that shows each model turning on a plain floor, in its animations and in each player colour. It can also show every model of one kind side by side at true size, with the boat for scale.

Not: showcase, viewer, model viewer

### High-Quality Camera

An item offered after a catch: Use camera photographs the catch, which makes it worth more; Save for later keeps the camera.

Not: photo, snapshot

### Hook

To catch a biting fish on the line by pressing Space in time. Every player sees a popup naming who hooked which fish.

Not: snag

### Item

A powerup a player holds, taken from an item buoy and hidden until then: Cloaking Tech, Power Reel, Nitrous, Teleporter, High-Quality Camera or Enhanced Radar. A player holds one at a time; a new one replaces it. Shift uses it, after its minigame.

Not: power-up, loot, perk

### Item buoy

The giant floating fishing bobber, glowing at its seam and topped by a beacon, that holds a hidden item. A player who drives through it takes the item.

Not: crate, loot box

### Lake preview

A debug screen, opened from the Title, that draws one checked map of the lake with a camera tour or a fixed view. Its fog distance can be changed live.

Not: level editor, map viewer, flyover

### Lure

The bait at the end of the line, under the bobber. It sinks until Space stops it or it reaches the lake bed, and fish swimming near its depth come to it more readily.

Not: bait, jig

### Map seed

The number that picks a map's layout. A match on the same map seed has the same shoreline and islands. Left empty on New match, the game picks a checked layout at random.

Not: map code, map ID

### Match

One game on the lake, from the start to the end of its time ({match.minutes} minutes unless New match says otherwise). The fish caught in it are added up at the end.

Not: round, session

### Match menu

The menu over Play that Esc opens when there is nothing else to cancel, and the Menu button too. It offers Resume, Settings and Leave match. A match against AI players only waits while it is open.

Not: options

### Minigame

A short game an item plays before it works, with a one-line instruction. It cannot be failed, only finished slowly; Esc quits it, and the item is lost.

Not: challenge, puzzle

### Minor version

A version whose last number is not zero. It holds fixes only, and What's New lists it when the reader asks.

Not: hotfix, point release

### Model

One fish, boat, figure, item or piece of scenery, kept as its own file of flat-coloured faces.

Not: mesh, asset, sprite

### My catch

The Fish-Boy's tab that lists your own catches this match, newest first: each fish's name, tier and length. It shows no points; scores stay hidden until the postgame.

Not: inventory, bag, haul

### New match

The screen Play opens, where an offline match is set up: how many AI players and how hard each plays, the match length, the item buoy interval and the map seed. Start match begins it.

Not: lobby screen, setup wizard

### Nibble

A fish tasting the lure before it bites. Space during a nibble scares the fish away.

Not: tug, peck

### Nitrous

An item that makes its player's boat faster to speed up, faster at the top and sharper in the turns for a while. Rams and grounding are judged on the plain top speed.

Not: boost, turbo, nitro

### Ping

A sweep of the radar outward from your boat that shows each other player where it passes them. Each player waits out a cooldown between pings.

Not: scan, pulse

### Play

The screen where a match runs: the lake, the boats, and the time left in the corner.

Not: level, stage, round

### Player colour

The colour a player's lobby slot gives them: blue, red, green or yellow. It marks their boat and their name, and the name is always shown with it.

Not: team colour

### Power Reel

An item that makes its player's reel stronger for a while, so a fish comes in faster.

Not: super reel

### Radar

The Fish-Boy's green overhead map of the lake. It always shows your boat and its heading and the item buoy, and shows other players only where the last ping found them.

Not: minimap, sonar

### Reel

The minigame after a hook: keep the fish inside the catch zone until the catch meter is full. A perfect reel takes {fishing.reel.perfect_ms} by tier.

Not: fight, tug of war

### Settings

The screen where the reader sets the volume, the text size and their name in matches. Settings last for this visit.

Not: options, preferences, config

### Sounds

The screen, opened from the Gallery, that plays each of the game's sounds on a click, and runs the motors and the line's whine at chosen settings, including another boat's motor driving past. The game makes every sound from a recipe as it plays; there are no recordings.

Not: soundboard, audio test, sound test

### Spawn point

The place at one end of the lake where every boat starts a match, side by side, and comes back after a wreck.

Not: start line, base

### Teleporter

An item that moves its player's boat to a point picked on the radar, after a charge every player can see. The boat arrives at rest.

Not: warp, portal

### Text size

How large the menus' text is drawn: small, medium or large.

Not: font size, zoom

### Tier

How rare a kind of fish is: Common, Uncommon, Rare or Legendary. Each tier has a colour and, in the Fishpedia, a frame shape of its own.

Not: rarity class, grade

### Title

The first screen: the game's name, its menu and its controls.

Not: home screen, main menu, start screen

### Underwater camera

The view while the line is in the water: it looks at the lure under the surface and the mouse turns it. A small picture of the boat sits in the top-left corner.

Not: fish cam, hook cam

### Version

The game's number, three numbers with dots between them, shown in the corner. Clicking it on a menu screen opens What's New.

Not: build, release

### Volume

How loud the game's sounds play, in percent. Sound starts after the reader's first click or key press on the page.

Not: loudness, sound level

### What's New

The screen listing what each version changed, newest first.

Not: changelog, release notes, patch notes, news

### Wreck

A boat blown up by hitting land at {wreck.land_pct}% of top speed or more, or by a boat ramming it at {wreck.ram_pct}% or more. It comes back at the spawn point after {wreck.respawn_s} seconds, ghosted for a few seconds: it passes through boats and cannot ram or be rammed.

Not: death, kill, crash, explode
