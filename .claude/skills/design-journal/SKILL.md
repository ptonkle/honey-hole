---
name: design-journal
description: Keep a design journal for a video game: a dated screenshot history with one page per level, published as a Claude artifact so the person can review how the game looks and give feedback. Use when asked to start, update, add to, publish or maintain a design journal, or when visual work (levels, models, characters, UI) should be shown to the person.
---

# Design journal for a game

The person reviews the game's look from this journal, not from the game: screenshots you open yourself are invisible to them. So every visual change gets a journal entry, published, with the link.

## The journal folder

Keep it in one lasting folder, called JOURNAL here (ask the person where it lives, or create `<game>-journal/` next to the game's repo). Never in a temporary or scratch folder: it is the source of truth.

- `index.html`: the whole journal. Start a new one from `template/index.html` beside this skill (it has the layout, the lightbox and the page routing).
- `img/<level>/<topic>/<shot>.png`: the screenshots.
- A git repo: commit after every change.

## Structure

- `#home`: a card per level (picture, entry count, latest date, status) and a timeline of every entry, newest first.
- One `<section class="page" id="<level-key>" data-page>` per level, plus pages for things that span levels (characters, menus, extras) as needed. Add each to the top nav.
- Each page holds `<article class="entry">` blocks, newest first: a date and status stamp, a short title, a plain-words lede (what changed, what still looks wrong), and a grid of figures, each with a one-line caption. Clicking a figure opens it full-window; the arrow keys step through the entry.
- Never delete or rewrite old entries: the journal is a history of how each level evolved.

## Adding an entry

1. **Shots.** Use the game's own screenshot tooling if it has any (look for scripts, a URL or command-line option that places the camera, a debug or photo mode). Otherwise drive the built game headless (e.g. Playwright for a web game) and capture at **1920×1080**. Frame each shot from the most interesting vista, not wherever the default camera sits. Never open a visible window on the person's screen. Look at every shot yourself and retake any that read badly (too dark, cluttered, cropped, or not showing the change).
2. **Copy** the shots to `JOURNAL/img/<level>/<topic>/`.
3. **Write** the entry at the top of the level's page in the same format as the ones already there. Add the `#home` timeline row and update the level's card. A new level gets a page, a nav link and a card.
4. **Publish** with the Artifact tool: `file_path` = `JOURNAL/index.html`, `root` = JOURNAL, `files` = the new image paths (published files you don't send are kept). Keep the same URL: from a session that didn't publish it, read the artifact once, then publish with `url`. More than 255 files go up over several publishes to the same URL.
5. **Commit** in JOURNAL.
6. **Reply** with the link and a few plain lines on what's new and what still looks off.

## Style

- Captions and ledes in everyday words, not code terms or internal names.
- Say honestly what still looks wrong; the person uses the journal to give feedback.
- Lay pages out for a 1920-wide window with big shots.

## In this project (Honey Hole)

- **The journal:** the journal is `../honey-hole-journal/` beside this repo (its own git repo, never pushed), published at https://claude.ai/artifact/TeFhUVC6nLvhasHW726N6U. Pages: one per area of the game (models, maps, Fish-Boy and minigames).
