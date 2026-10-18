# Second-order notebook

Stage 6, kept by the agent as the work happens, not afterwards. `B3 Build in self-questioning`.

The code and tests are the work; this file is the record about the work. It is where a decision the agent made alone, an assumption that turned out wrong, or something only the person can do becomes visible to the person. With the plan approved in advance, it is the main place those decisions surface before the person uses the tool. [`docs/procedure.md`](https://github.com/sgilson7/builder-setting/blob/main/docs/procedure.md#the-second-order-notebook) says why.

One row per assumption, surprise or open item. **Kind** is *divergence* (what was assumed, what turned out to be true, what changed), *finding*, or *worklist* (the person's to do). Number rows by round (R1.1, R1.2, then R2.1 after the first triage) so a later round does not renumber an earlier one. **Rule** is the rule the row produced for later sessions, and where it now lives (`CLAUDE.md`, a test, `tests/breaks.json`), or blank. **Status** is *done*, *open*, or *person*; every *person* row is named in `HANDOFF.md`. `scripts/notebook_check.py` checks the form of each row and that the notebook is not empty once `PLAN.md` holds a decision, not what the rows say.

| # | Kind | Assumed or found | What is true | What changed | Rule | Status |
| --- | --- | --- | --- | --- | --- | --- |
| R1.1 | | | | | | |
| R1.20 | divergence | The template's page holds a `State` JSON and hands it back on every `act`. | The game needs a stateful app, as Ape Race has; a state round-trip through the page is page-held state. | The shim holds one `Game` (an `api::Session` around `app::App`) with `new(strings, notes)`, `ui()` and `act(name, value)`; `act` returns the next view model. | `crates/core/src/api.rs`; `shim_decides_nothing` | done |
| R1.21 | divergence | The template sends glossary definitions to the page as hover hints (D15). | The person chose no hover definitions on screen (DECISIONS.md, 2026-10-17); `GLOSSARY.md` and its copy tests stay. | Dropped `Hint` from the view model, the page's `mark()`, and the glossary copy in `scripts/build.sh`; rewrote the `GLOSSARY.md` intro; recorded the change in `BUILDER-SPEC.md`'s change table. | `BUILDER-SPEC.md` changes table | done |
| R1.22 | divergence | The template's strings file is a fixed struct (`title`, `intro`). | The menus need many strings by key, as Ape Race's `self.t(id)`. | `strings.rs`: a key-to-text map. The core lists every key it reads in `KEYS`; a missing, empty or unused key is refused by name before any screen draws. Keys starting `_` are notes for people. | `crates/core/src/strings.rs` tests; `tests/breaks.json` | done |
| R1.23 | worklist | Ape Race keeps settings in browser storage. | Builder's D17 bans browser storage unless the brief allows it; keeping data between visits is the person's decision. | Settings (volume, text size, minor versions in What's New) last for this visit. The Settings glossary entry says so. | `GLOSSARY.md` (Settings) | person |
| R1.24 | divergence | Ape Race marks What's New "New" until this version's notes are opened. | The mark needs the seen version kept between visits, which needs storage (R1.23). | Left the mark out. The button item still carries `badge`, so it can return once storage is decided. | | open |
| R1.25 | finding | Ape Race builds its patch notes into the core with `include_str!` and writes a site page with `scripts/patch_notes_html.py`. | D5 puts reader-visible text in `data/`, fetched by the page; there is no published site yet. | The page fetches `data/patch-notes.json` and hands it to the core, which checks each version (three numbers), date (YYYY-MM-DD), lines and newest-first order. `tests/data_files.rs` fails when the newest entry is not `app::VERSION` (0.1.0). The site page script is not ported. | `crates/core/src/patch_notes.rs`; `crates/core/tests/data_files.rs` | done |
| R1.26 | divergence | Ape Race sends keys through its own `key(code, down)` shim function. | The template's convention is one door, `act`. | The page sends each key-down as `act("key", code)`, except keys typed into a select or text control; the core decides what a key does (Escape only, so far). | `App::key` in `crates/core/src/app.rs` | done |
| R1.27 | finding | Ape Race has thirteen item kinds. | Tiles, picker, stats, checks, field and copy are tied to its maps, apes, stats and online codes. | Ported button (quiet, badge, disabled), slider, toggle, select, list, table and notes. Table, badge and disabled have no screen yet: a core test covers their JSON, but no browser check reaches their builders until a screen uses them. | `crates/core/src/view.rs` | open |
| R1.28 | divergence | Ape Race's view model carries `sounds` (a click on every action). | The person asked for none of Ape Race's sounds; fishing has no sound yet. | No `sounds` field. Volume is kept and sent in the view model (`volume`) for the sounds to come; the Volume glossary entry says no sound plays yet. | | done |
| R1.29 | finding | The coordinator asked that the app stay open to any sync model for online play (DECISIONS.md, 2026-10-17 (online)). | Ape Race's pause stops the race offline and only opens the menu online. | Every change goes through `act`, the page keeps no state, and the core reads no clock or random number. The pause menu is a flag for the person at this screen (`App::paused`); whether the game stops under it is for the sync model to decide. Play holds no player of its own, so nothing assumes one local player. | doc comment of `crates/core/src/app.rs` | done |
| R1.30 | worklist | The game has no name yet. | "Fishing" is the working name. | It is the Title's heading and the page title until the person names the game. The agent's strings and the text size choices (Small, Medium, Large) are listed below for the person to read. | | person |
| R1.31 | finding | Ape Race gives the focus back only to a replaced control that is not a button. | A keyboard reader whose button opened a new screen was left with the focus on the page. | When the focused control is gone after a redraw, the new screen's first control takes the focus. WebKit does not focus a clicked button, so the browser check presses Back by keyboard. | `tests/browser/interactions.py` | done |
| R1.32 | finding | Rule 6: each new behaviour test must fail with its behaviour broken. | `scripts/break_tests.py`: 25 of 25 breaks caught, each by an assertion, not a build error (strings, unknown actions, Escape in each screen, Back, the pause menu, the version's reach, the volume's range and steps, text sizes, minor versions, the patch notes' checks). | Added 24 rows to `tests/breaks.json` and moved the template's two to the new modules. | `tests/breaks.json` | done |
| R1.33 | divergence | Ape Race ignores an action it does not know. | The template refuses it by name. | Kept the template's refusal: `act` returns `{"error": ...}` and the page shows the message; a refused setting value (a volume that is not a number, an unlisted text size) is refused the same way and leaves the setting as it was. | `an_unknown_action_is_refused_by_name` | done |
| R1.34 | divergence | Ape Race's menu look: jungle colours, its fonts, a menu dock and a flyover behind the Title. | The person asked that fishing feel like its own game. | A neutral stylesheet (system font, greys, one blue accent, light and dark) until the game has a style. The panel `kind` names a panel for the stylesheet only. | `web/style.css` | done |

## Strings written by the agent

Each reader-visible string the agent wrote, with its key, for the person to read.

| Key | String |
| --- | --- |
| `title` | Fishing |
| `menu.play` | Play |
| `menu.settings` | Settings |
| `menu.notes` | What's New |
| `menu.back` | Back |
| `menu.pause` | Pause |
| `menu.resume` | Resume |
| `menu.quit` | Quit to Title |
| `controls.heading` | Controls |
| `controls.pause` | Pause |
| `controls.pause_keys` | Esc |
| `settings.heading` | Settings |
| `settings.volume` | Volume |
| `settings.text_size` | Text size |
| `settings.text_size.small` | Small |
| `settings.text_size.medium` | Medium |
| `settings.text_size.large` | Large |
| `notes.heading` | What's New |
| `notes.minor` | Show minor versions |
| `play.placeholder` | The game goes here once it is designed. Press Esc to pause. |
| `pause.heading` | Paused |
| `data/patch-notes.json` 0.1.0 | First version: the Title, Settings, What's New and a stand-in Play screen with a pause menu. |
| core messages | "Unknown action \"…\".", "The volume \"…\" is not a whole number.", "There is no text size \"…\".", the strings file and patch notes refusals in `strings.rs` and `patch_notes.rs` |
