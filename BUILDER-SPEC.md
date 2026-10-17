# Builder specification

Builder is a reusable technical setting for small browser tools built with coding agents: application rules live in a fast-tested Rust core, a thin WebAssembly shim only moves values across the browser boundary, and a static page renders the result. The separation gives the agent fast feedback while it builds and leaves a finished artifact that can be checked at the core, the WebAssembly boundary and the browser page.

This file has three parts. **Builder requirements** define Builder; a project that drops one is no longer a Builder project, and the change should be stated as such. **Reference defaults** are what this template chose so that a copy works at once; change them by recording the change in `SECOND-ORDER.md` and in the table at the end of this file. **Project decisions** belong to the person building the tool, and they go in `BRIEF.md` or `PLAN.md`.

## Builder requirements

| # | Requirement | Enforced by |
| --- | --- | --- |
| R1 | The core (`crates/core`) holds every application rule and state transition. Scripts and tools that need a rule read the core's output; they do not recompute it. | review; `audit-boundary` prompt |
| R2 | The core performs no browser, filesystem, network, clock, random-device or UI input or output. `cargo test` reaches all of it without a browser. | `core_source_has_no_io_and_no_nondeterminism` in `crates/core/tests/boundary.rs` |
| R3 | The core's dependencies are on an explicit allowlist. | `core_dependencies_are_on_the_allowlist` |
| R4 | Public core functions take ordinary values and return ordinary values or serialisable structures. The template's convention is strings in, `{"ok": ...}` or `{"error": "..."}` out, in `crates/core/src/api.rs`: the core describes the screen as a view model (`ui`), and settings, screens and flags come back through one function, `act(name, value)`. | `api.rs` tests; review |
| R5 | The shim (`crates/shim`) uses `wasm-bindgen` and only translates values: it converts arguments, calls the core, and converts the result back. | `shim_decides_nothing` (no branch, loop or `?` in shim source) |
| R6 | The page (`web/`) is static HTML, CSS and JavaScript. It draws what the core returns and does not recompute an application rule. | `page_keeps_no_rule_and_names_no_other_origin`; review |
| R7 | The default build deploys as static files and makes no off-origin request on its default load-and-idle path, and the page does not navigate away on its own. A *declared off-origin feature* (online play, for example) is allowed when the brief names it, it loads only on a reader's action, its code is vendored and checksummed, and its origins are listed in one config file that a test matches against the CSP (`kits/online/`). | `tests/browser/check.py`; navigation scan in `boundary.rs`; the kit's fence test |
| R8 | `./scripts/check.sh` runs the complete pre-deploy check, and CI runs the same file. | `.github/workflows/checks.yml` in the reference repository |
| R9 | Each build follows the nine-stage procedure, and the agent does not deploy on its own judgement. It publishes only when the brief's Autonomy paragraph grants it. | `CLAUDE.md`, `AGENTS.md`, `HANDOFF.md` |

**When a requirement seems to block necessary work, stop and say so.** Write a worklist row in `SECOND-ORDER.md` naming the requirement, what the feature needs, and the options you can see. A rule in the shim or the page is a second rulebook that the fast suite cannot see, so the answer is almost always to move the rule into the core.

## Reference defaults

| # | Default | Where | Why this default |
| --- | --- | --- | --- |
| D1 | Core allowlist: `serde`, `serde_json`. | `ALLOWED_DEPENDENCIES` in `boundary.rs` | enough for JSON in and out |
| D2 | No floats, hash maps, clocks or randomness in the core. The banned names include `f32`, `f64`, `HashMap`, `HashSet`, `Instant`, `SystemTime`, `time`, `rand`, and, as input/output, `fs`, `io`, `net`, `env`, `process` and `thread`; name modules accordingly (`online`, not `net`). For geometry and physics use `fx.rs` (Q16.16, products through `fx::mul`); for chance, a seed passed in from the page and `rng.rs` (`kits/realtime/`). | `NONDETERMINISM_IDENTIFIERS` in `boundary.rs` | the same input gives the same output on every machine, so tests stay stable and replays, ghosts and lockstep play work. A project may allow `f32` instead if it bans platform maths (`sin`, `powf`, `mul_add`) and keeps a bit-exact replay test; record it as a change |
| D3 | Page JavaScript holds no numeric literal other than 0 and 1, and no `Math`, `parseInt` or rounding. Presentation numbers (sizes, durations, audio) go in `data/*.json`; a canvas or WebGL page replays a draw list and assets the core sends (`kits/canvas/`); shaders live in `data/shaders/` and hold no number other than 0 and 1, taking rule values as uniforms. | `PAGE_FORBIDDEN_IDENTIFIERS`, `shaders_hold_no_numbers_of_their_own` | a number in the page is usually a copy of a rule |
| D4 | Every HTML page, at any depth under `web/`, carries a Content-Security-Policy meta element with `default-src 'self'`, `connect-src 'self'`, `'wasm-unsafe-eval'` and `form-action 'none'`. | `every_page_sets_the_content_security_policy` | a second, browser-enforced limit on where the page can connect; it cannot stop navigation, so the browser check still watches requests and the scan watches navigation |
| D5 | Reader-visible text lives in `data/`, and the page fetches it from its own origin. | `copy.rs` | content changes without touching rules |
| D6 | `wasm-bindgen` pinned to `=0.2.127`; the build refuses a different CLI version. | `crates/shim/Cargo.toml`, `scripts/build.sh` | the generated glue must match the crate |
| D7 | Browser check: Playwright 1.63.0 in Chromium, Firefox and WebKit, 10 s idle, then `tests/browser/interactions.py`. CI may run Chromium alone (`BUILDER_ENGINES=chromium`) when all three run locally before a push. `webkit-iphone` adds a phone profile. Interactions wait on selectors and `data-*` attributes, never on a fixed time. | `tests/browser/check.py` | the engines classroom machines run; a three-engine install on CI has hung for over fifteen minutes |
| D8 | Mutation testing with `cargo-mutants`, and `scripts/break_tests.py`, as audits outside `check.sh`. | `scripts/mutants.sh`, `scripts/break_tests.py` | they take minutes; the everyday check must take seconds |
| D9 | `cargo fmt --check` and `cargo clippy -D warnings`. | `scripts/check.sh` | |
| D10 | The build stamps a content hash on every internal URL, fails on an unstamped one, verifies vendored files against `SHA256SUMS`, and writes `dist/build.txt` and `.nojekyll`. | `scripts/build.sh` | GitHub Pages caches files for ten minutes, so a returning reader would keep the old module after a deploy |
| D11 | Release profile for size; a test profile at `opt-level = 2` with overflow checks on. | `Cargo.toml` | whole-run tests in unoptimised integer maths get slow enough to stop being run |
| D12 | `rustfmt.toml` with `max_width = 110`. | `rustfmt.toml` | fewer rewrapped lines between scripted edits |
| D13 | A pinned Rust toolchain. | `rust-toolchain.toml` | a newer Clippy on CI failed a build that passed locally |
| D14 | `LICENSES.md` lists every asset, vendored file and borrowed piece of code with its terms. | review | a rights question is the person's, and needs a place to be asked |
| D15 | `GLOSSARY.md` defines every term: a plain definition, numbers as `{placeholders}` the core fills, and a `Not:` line of words never used for it. The page shows definitions on hover. | `copy.rs`; `glossary.rs` tests | one name for each thing, kept by a test |
| D16 | Reader-visible strings, glossary definitions and the README follow `TONE.md`; the phrases between its lint markers fail a test. | `no_text_uses_a_phrase_the_tone_file_rules_out` | stock phrasing is the easiest drift to catch mechanically |
| D17 | The page uses no browser storage. | `no_storage.rs` | storage is a decision about a person's data |
| D18 | When the brief names no style, 2D art uses the default style pack in `styles/default/`. The pack's colours and name appear nowhere else in Builder. | `scripts/check-style-leak.py` | a fallback that does not become Builder's identity |

## Project decisions

These stay with the person and go in `BRIEF.md` or `PLAN.md`:

- what the tool does, for whom, and what "done" means, starting from the person's own description;
- the content and its source, and who checks it;
- any data a person enters, where it stays, and whether the page keeps it between visits (the reference default, D17, is no browser storage; the brief may allow it);
- any crate added to the allowlist;
- accessibility: how information is carried besides colour, and which input devices must work;
- any third-party asset, likeness or recording, and any code borrowed without a licence;
- any list an agent generated that a reader will see (names, labels, word lists);
- any study of the people who use the tool, which needs approval before any data is kept;
- the visual style, or `default`;
- any feature that needs a server, a live model, an account, or a request to another origin. These fall outside the default build (R7). Say so in the brief and give the feature its own checks and approvals.

## Changes to the defaults in this project

| Default | Changed to | Reason | Recorded in |
| --- | --- | --- | --- |
| | | | |
