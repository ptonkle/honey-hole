#!/usr/bin/env bash
# Runs a check where a person can watch it, and keeps a plain log to grep.
#
#   ./scripts/watch-check.sh notes/run.log -- ./scripts/timed-checks.sh quick
#   ./scripts/watch-check.sh /tmp/full.log --name "full check" -- ./scripts/check.sh
#
# The check runs detached (setsid) with plain output to <log> (a last line
# `EXIT <n>` when it ends) and progress events to <log>.events
# ($CHECK_EVENTS). A Windows Terminal window runs scripts/check-dashboard.py
# on those two files: a full-screen view drawn again from them on every
# refresh, so moving or resizing the window (or a monitor with other
# scaling) never garbles it, and closing it never stops the check. It closes
# itself 5 s after the check ends. Reopen it with
#   .venv/bin/python scripts/check-dashboard.py <log>.events <log> "<name>"
# Without wt.exe (not WSL), or with WATCH_NO_WINDOW=1, it opens no window.
set -euo pipefail
cd "$(dirname "$0")/.."
[ $# -ge 3 ] || { sed -n '2,15p' "$0"; exit 2; }
LOG=$(realpath -m "$1"); shift
NAME=""
if [ "$1" = --name ]; then NAME=$2; shift 2; fi
[ "$1" = -- ] || { echo "watch-check: expected -- before the command" >&2; exit 2; }
shift
NAME=${NAME:-$*}
EVENTS="$LOG.events"
mkdir -p "$(dirname "$LOG")"
q() { printf '%q' "$1"; }
PY=python3
[ -x .venv/bin/python ] && PY=$(pwd)/.venv/bin/python

# The check, detached; written to a file so the command's quoting holds.
RUNNER="$LOG.run"
{
  echo "cd $(q "$(pwd)")"
  echo "printf '{\"k\": \"begin\", \"at\": %s}\\n' \$(date +%s.%N) > $(q "$EVENTS")"
  echo "CHECK_EVENTS=$(q "$EVENTS") CHECK_PLAIN=1 $(printf '%q ' "$@") > $(q "$LOG") 2>&1 < /dev/null"
  echo 'code=$?'
  echo "echo \"EXIT \$code\" >> $(q "$LOG")"
  echo "printf '{\"k\": \"end\", \"code\": %s}\\n' \$code >> $(q "$EVENTS")"
  echo "rm -f $(q "$RUNNER")"
} > "$RUNNER"
: > "$LOG"; : > "$EVENTS"
setsid nohup bash "$RUNNER" >/dev/null 2>&1 < /dev/null &
echo "watch-check: '$NAME' running detached (pid $!)"
echo "  plain log: $LOG (ends with EXIT <n> when done)"

# The window: the dashboard. Its command goes in a file: Windows Terminal
# splits its command line at every ';' (a new tab each).
TITLE="$NAME — view only, safe to close anytime"
VIEW="$LOG.view"
{
  echo "$(q "$PY") $(q "$(pwd)/scripts/check-dashboard.py") $(q "$EVENTS") $(q "$LOG") $(q "$NAME")"
  echo "rm -f $(q "$VIEW")"
} > "$VIEW"
if [ -z "${WATCH_NO_WINDOW:-}" ] && command -v wt.exe >/dev/null && [ -n "${WSL_DISTRO_NAME:-}" ]; then
  wt.exe -w new --size 110,40 --title "$TITLE" wsl.exe -d "$WSL_DISTRO_NAME" -- bash "$VIEW" >/dev/null 2>&1 &
  echo "  window: opened ($TITLE)"
fi
