# Quick check and check visibility

Ported from Ape Race on 2026-10-17 (DECISIONS.md), with its maps, online play and AI left out. SECOND-ORDER.md R1.1 to R1.14 record each choice.

## The quick check

- `tests/tags.toml` holds the tags, each with a one-line meaning, and maps changed files to tags (`[sources]`), cargo tests to tags (`[tests]`, by `lib:<module path>` or `test:<file>[:<name prefix>]`, the longest key wins) and browser steps to tags (`[browser]`). `[lines]`, `[functions]` and `[steps]` place single changed lines more narrowly; `[implies]` widens a tag.
- `./scripts/quick-check.sh [base]` diffs `base..HEAD`, the working tree and untracked files. The base defaults to the commit of the newest passing full `check` row in `notes/check-times.md`, so a quick check only ever skips relative to a full pass. It always runs format, lint, the tag guard, the release build and the import audit; then the cargo tests whose tags meet the changed files' tags (one `cargo test -- --exact <names>`), the art and notebook checks on their tags, and the browser check in Chromium with only the picked steps (`BUILDER_STEPS`; `boot`, the load and idle watch and the closing error and request checks always run; a `page` change runs every step).
- Fail safe: a changed file that no glob matches runs everything, and so does a foundation (`all`). The `boundary` tests always run. A test file runs only its own tests; a line inside a `#[cfg(test)] mod` runs only the test it sits in.
- `--dry-run` prints the plan; `--range A..B --dry-run` shows what a past change would pick; `ALL_BROWSERS=1` adds Firefox and WebKit.
- The guard, `scripts/tags.py guard` (stage 4 of `check.sh`, and in every quick check), fails on an untagged cargo test or browser step, a stale key, an unknown tag, or a tracked file no glob matches. A new core module or test file needs its row before the check passes.

What Ape Race learned that holds here: a test is only as safe as its tags, so when unsure give the wider tag; the full `check.sh` in three engines stays the check before publishing; cheap tests that never fail cost nothing and are not worth merging or deleting (Ape Race's `notes/test-consolidation.md`: the time was in a few slow tests and in running files and engines one after another).

## Seeing a check run

- Every stage of `check.sh` and the quick check runs through `scripts/checkrun.py`: a header with the stage's usual time (the median of the last five passing rows in `notes/check-times.md`), a ✓ or ✗ line with its seconds, a summary table on failure and at the end, and a `TIMES` line.
- In a terminal the core tests run on `cargo-nextest` with its progress bar when it is installed (`CHECK_NEXTEST=0` skips it), and the browser check draws one `rich` bar per engine. Bars are one line and at most 80 columns, on a terminal of their own: a bar line that wraps stacks a new line on each redraw. In a log (or with `CHECK_PLAIN=1`) the plain lines, and plain `cargo test`.
- `./scripts/timed-checks.sh [check|quick]` adds a row to `notes/check-times.md`; a failing run adds the failing tests and steps to `notes/test-failures.md` (`scripts/tags.py failures`).
- `./scripts/watch-check.sh <log> [--name N] -- <check>` runs the check detached (`setsid`) with plain output to `<log>` and progress events to `<log>.events`. Under WSL it opens a Windows Terminal window running `scripts/check-dashboard.py`, a full-screen view drawn again from those files on each refresh, so moving or resizing the window never garbles it and closing it never stops the check. It closes itself 5 s after the check ends. The window's command goes through a file because Windows Terminal splits its command line at every `;`. `WATCH_NO_WINDOW=1` opens no window.
- Chromium draws on the GPU in an invisible Xvfb window where `/dev/dxg` and `Xvfb` exist (WSL), and headless elsewhere or with `BUILDER_HEADLESS=1` (`tests/browser/engines.py`).
