# Glossary

The source of truth for what each word in this game means. Strings, tests, the plan and the README use a term in the sense given here. The page shows no definitions on screen (the person's choice, `DECISIONS.md`, 2026-10-17); this file and its copy tests keep the words straight.

Write each entry as a `### Term` heading, then one or two plain sentences, then an optional `Not:` line listing words that must not be used for it. A number a rule decides is a `{placeholder}` the core fills, so the glossary cannot disagree with the rules. Keep entries in alphabetical order. `crates/core/tests/copy.rs` checks the format, the placeholders and the `Not:` words; `TONE.md` governs the prose.

<!--
An example entry. It sits inside this comment, so it does not count.

### Spout

Where slush comes out. Each line has up to {line.spouts} spouts.

Not: tap, dispenser, nozzle
-->

### Back

The button at the foot of each screen opened from the Title. It goes back to the Title, as Esc does.

Not: return, exit

### Controls

The keys the game uses, listed on the Title.

Not: keybindings, hotkeys, bindings

### Minor version

A version whose last number is not zero. It holds fixes only, and What's New lists it when the reader asks.

Not: hotfix, point release

### Pause menu

The menu over Play that Esc opens and closes. It offers Resume and Quit to Title.

Not: freeze, halt

### Play

The screen where the game runs. It is a stand-in until the game is designed.

Not: level, stage, round

### Settings

The screen where the reader sets the volume and the text size. Settings last for this visit.

Not: options, preferences, config

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
