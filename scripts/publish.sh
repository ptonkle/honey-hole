#!/usr/bin/env bash
# Publish dist/ to the gh-pages branch of this repository's origin. A
# person's action (stage 8), or the agent's only when BRIEF.md's Autonomy
# paragraph says so. It refuses a tree with uncommitted changes, so the page
# always matches a commit, and the commit message says which one.
#
#   ./scripts/publish.sh
#   BUILDER_ORIGIN=https://<owner>.github.io/<repo> python3 tests/browser/check.py
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
if [ -n "$(git status --porcelain)" ]; then
  echo "uncommitted changes: commit first, so the published page matches a commit" >&2
  exit 1
fi
./scripts/check.sh
REMOTE="$(git remote get-url origin)"
COMMIT="$(git rev-parse --short HEAD)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/builder-pages.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT
cp -R dist/. "$WORK/"
cd "$WORK"
git init -q -b gh-pages
git add -A
git -c user.name="$(git -C "$ROOT" config user.name)" -c user.email="$(git -C "$ROOT" config user.email)" \
  commit -q -m "Built from $COMMIT"
git push -q -f "$REMOTE" gh-pages
echo "published dist/ built from $COMMIT to gh-pages"
echo "now run the browser check against the live address (BUILDER_ORIGIN=...)"
