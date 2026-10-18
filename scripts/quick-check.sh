#!/usr/bin/env bash
# The quick check: only the tests a change needs (tests/tags.toml).
#
#   ./scripts/quick-check.sh                 changes since the last passing full check
#   ./scripts/quick-check.sh <base>          changes since <base> (+ the working tree)
#   ./scripts/quick-check.sh --range A..B    the files of A..B (dry runs on history)
#   ./scripts/quick-check.sh --dry-run       print the plan only
#   ALL_BROWSERS=1 ./scripts/quick-check.sh  the browser check in all three engines
#                                            (a big CSS or page change)
#
# Always: format, lint, release build, WebAssembly import audit. Then the
# cargo tests, other checks and browser steps (Chromium) whose tags meet the
# changed files' tags. A file no glob matches runs everything. The full
# scripts/check.sh is still the check before publishing.
# Print each browser step as it finishes, not all at the end when logged to a file.
export PYTHONUNBUFFERED=1
set -euo pipefail
cd "$(dirname "$0")/.."
PY="${BUILDER_PYTHON:-}"
if [ -z "$PY" ]; then
  if [ -x .venv/bin/python ]; then PY=.venv/bin/python; else PY=python3; fi
fi
exec "$PY" scripts/tags.py quick "$@"
