#!/usr/bin/env bash
# Optional audit, kept out of check.sh because it takes minutes, not seconds.
#
# Mutation testing deliberately makes small changes to the code and reruns the
# tests. A planted change is caught when those tests fail. The share caught
# says how much current behaviour the tests pin down. It is not a bug rate, and
# it says nothing about whether the behaviour pinned down is the intended one.
#
#   ./scripts/mutants.sh            every mutant in the core
#   ./scripts/mutants.sh --shard 1/4   a quarter of them
#
# Needs: cargo install cargo-mutants --locked
set -euo pipefail
cd "$(dirname "$0")/.."
cargo mutants --package tool_core --jobs 4 "$@"
echo "Missed mutants, if any, are listed in mutants.out/missed.txt."
echo "Each one is either a missing test or a boundary the brief has not decided."
