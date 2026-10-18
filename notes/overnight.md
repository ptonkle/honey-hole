# Overnight build: shared instructions for build agents

You are one of several agents building Honey Hole overnight while the person sleeps. It is a cartoony low-poly 3D online fishing game on the Builder template: Rust core in crates/core, wasm shim in crates/shim, static page in web/. You work in a git worktree of /home/peter/slop/honey-hole.

Read these first and follow every rule in them:
- CLAUDE.md, including rule 13 (no default boxes in models);
- BRIEF.md, all of it;
- PLAN.md: the architecture contracts are binding, and you must follow its units, tick, Match/SeatInput/events and tuning.json conventions;
- DECISIONS.md, BUILDER-SPEC.md, GLOSSARY.md, TONE.md and tests/tags.toml.

The person said: "Use your best judgement for any decisions you come across, and write the code in such a way that for the things that you think might need to be changed, they are easily updated. Do not stop working for anything. seek alternatives when you are blocked." So decide, record each notable decision as a SECOND-ORDER.md row in your assigned range, and add a line under PLAN.md "For the person to validate" for any decision the person should confirm.

Rules for working alongside other agents:
- Put new code in your own modules.
- Keep edits to shared files (lib.rs, app.rs, the shim, web/app.js, web/gl.js, tags.toml, breaks.json, GLOSSARY.md, strings.json) small and additive.
- Use a private subfolder of the scratchpad for any scripts, since agents share the scratchpad.
- Never open a visible window. Playwright python is at /home/peter/slop/honey-hole/.venv/bin/python, and the browsers are cached.
- Every new test, file and browser step needs a tests/tags.toml row (add tags as needed). Every new behaviour test needs a tests/breaks.json row that scripts/break_tests.py catches.
- New reader-visible terms need a GLOSSARY.md entry first. Strings go in data/strings.json, registered in crates/core/src/strings.rs KEYS.

Before you finish:
- merge the latest main (git merge main; the main checkout is /home/peter/slop/honey-hole) and resolve conflicts by keeping both sides;
- run ./scripts/check.sh until it passes (BUILDER_PYTHON=/home/peter/slop/honey-hole/.venv/bin/python if your worktree has no .venv);
- commit with messages ending "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>".

Report:
- the branch;
- what works, and how to see it in the game;
- the tuning keys you added;
- decisions to validate;
- anything left undone, with the reason.
Screenshots of anything visual go in notes/shots/<area>/ at 1920×1080; look at every one yourself.
