# Glossary

The source of truth for what each word in this game means. Strings, tests, the plan and the README use a term in the sense given here. The page shows no definitions on screen (the person's choice, `DECISIONS.md`, 2026-10-17); this file and its copy tests keep the words straight.

Write each entry as a `### Term` heading, then one or two plain sentences, then an optional `Not:` line listing words that must not be used for it. A number a rule decides is a `{placeholder}` the core fills, so the glossary cannot disagree with the rules. Keep entries in alphabetical order. `crates/core/tests/copy.rs` checks the format, the placeholders and the `Not:` words; `TONE.md` governs the prose.

<!--
An example entry. It sits inside this comment, so it does not count.

### Spout

Where slush comes out. Each line has up to {line.spouts} spouts.

Not: tap, dispenser, nozzle
-->

### Animation

A model's movement, played in a loop: a fish's idle swim and panic swim, the angler's driving and fishing poses, a plant's sway. Still shows the model at rest.

Not: clip, motion, tween

### Angler

The figure each player plays: it sits on the boat's back bench to drive and stands at the bow to fish. Its shirt and hat band wear the player's colour.

Not: fisherman, avatar

### Back

The button at the foot of each screen opened from the Title. It goes back to the Title, as Esc does.

Not: return, exit

### Boat

Each player's small aluminium fishing boat, painted in the player's colour, with an outboard motor at the stern and a raised casting deck at the bow.

Not: ship, vessel

### Burt Lake

The first map: a northern lake with pine and birch shores, small wooded islands and reeds in the coves, in medium fog. Each match on it plays one of its checked layouts.

Not: Michigan Lake, level, course

### Controls

The keys the game uses, listed on the Title.

Not: keybindings, hotkeys, bindings

### Fish-Boy

The angler's computer, strapped to the left forearm. Raised to the face, its screen fills most of the view and shows the game's tabs, with the lake still in sight round its edge.

Not: Pip-Boy, pipboy, fishboy, PDA
### Fog distance

How far a player can see across the lake before the fog hides everything. The fog thickens from part of the way out and is whole at this distance.

Not: draw distance, visibility, view range

### Gallery

The screen, opened from the Title, that shows each model turning on a plain floor, in its animations and in each player colour. It can also show every model of one kind side by side at true size, with the boat for scale.

Not: showcase, viewer, model viewer

### Item buoy

The giant floating fishing bobber, glowing at its seam and topped by a beacon, that holds a hidden item. A player who drives through it takes the item.

Not: crate, loot box

### Lake preview

A debug screen, opened from the Title, that draws one checked map of the lake with a camera tour or a fixed view. Its fog distance can be changed live.

Not: level editor, map viewer, flyover

### Map seed

The number a map's layout is made from. The same seed always gives the same shoreline, islands, trees and reeds.

Not: map ID, level number

### Minor version

A version whose last number is not zero. It holds fixes only, and What's New lists it when the reader asks.

Not: hotfix, point release

### Model

One fish, boat, figure, item or piece of scenery, kept as its own file of flat-coloured faces.

Not: mesh, asset, sprite

### Pause menu

The menu over Play that Esc opens and closes. It offers Resume and Quit to Title.

Not: freeze, halt

### Play

The screen where the game runs. It is a stand-in until the game is designed.

Not: level, stage, round

### Player colour

The colour a player's lobby slot gives them: blue, red, green or yellow. It marks their boat and their name, and the name is always shown with it.

Not: team colour

### Settings

The screen where the reader sets the volume and the text size. Settings last for this visit.

Not: options, preferences, config

### Spawn point

Where every boat starts a match and comes back after a wreck: open water near the dock at one end of the lake.

Not: start line, base, home

### Text size

How large the menus' text is drawn: small, medium or large.

Not: font size, zoom

### Title

The first screen: the game's name, its menu and its controls.

Not: home screen, main menu, start screen

### Version

The game's number, three numbers with dots between them, shown in the corner. Clicking it on a menu screen opens What's New.

Not: build, release

### Volume

How loud the game's sounds play, in percent. The game plays no sound yet.

Not: loudness, sound level

### What's New

The screen listing what each version changed, newest first.

Not: changelog, release notes, patch notes, news
