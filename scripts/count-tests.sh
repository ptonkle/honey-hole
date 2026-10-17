#!/usr/bin/env bash
# One source for every test count a document states. Summing cargo's output
# by hand gave three different totals off one run in Slushline; quote this.
#
#   ./scripts/count-tests.sh
set -euo pipefail
cd "$(dirname "$0")/.."
cargo test --workspace 2>&1 \
  | awk '/^test result:/ { pass += $4; fail += $6; ignored += $8 }
         END { printf "%d passed, %d failed, %d ignored\n", pass, fail, ignored; exit (fail > 0) }'
