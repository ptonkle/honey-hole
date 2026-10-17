#!/usr/bin/env bash
# Build the release WebAssembly module and assemble the static site in dist/.
# Needs: Rust with the wasm32-unknown-unknown target, and the wasm-bindgen CLI
# at the version pinned in crates/shim/Cargo.toml.
#
# Besides building, this script:
# - refuses to build when a vendored file under web/ does not match the
#   SHA256SUMS beside it;
# - copies GLOSSARY.md to dist/data/glossary.md for the page's hover hints;
# - stamps a content hash on every internal URL (cache busting: GitHub Pages
#   serves files with a ten-minute cache, so a returning reader would keep the
#   old module after a deploy), and fails if one is left unstamped;
# - writes dist/build.txt (the hash and the commit) and dist/.nojekyll.
set -euo pipefail
cd "$(dirname "$0")/.."

PIN=$(sed -n 's/^wasm-bindgen = "=\(.*\)"/\1/p' crates/shim/Cargo.toml)
HAVE=$(wasm-bindgen --version 2>/dev/null | awk '{print $2}')
if [ "$HAVE" != "$PIN" ]; then
  echo "wasm-bindgen CLI is '${HAVE:-missing}', the shim pins $PIN." >&2
  echo "Install it with: cargo install wasm-bindgen-cli --version $PIN --locked" >&2
  exit 1
fi

sha256() { if command -v shasum >/dev/null; then shasum -a 256 "$@"; else sha256sum "$@"; fi }

# Vendored code is pinned by checksum, not scanned (boundary.rs skips vendor/).
while IFS= read -r sums; do
  if ! (cd "$(dirname "$sums")" && sha256 -c SHA256SUMS >/dev/null); then
    echo "a vendored file in $(dirname "$sums") does not match its SHA256SUMS" >&2
    exit 1
  fi
done < <(find web -name SHA256SUMS -path '*/vendor/*' | sort)

cargo build --release --target wasm32-unknown-unknown -p tool_shim

rm -rf dist
mkdir -p dist/pkg dist/data
wasm-bindgen --target web --no-typescript --out-dir dist/pkg \
  target/wasm32-unknown-unknown/release/tool_shim.wasm
cp -R web/. dist/
cp -R data/. dist/data/
cp GLOSSARY.md dist/data/glossary.md

# Cache busting. The hash covers every file, so any change gives new URLs.
BUILD=$(find dist -type f | LC_ALL=C sort | xargs cat | sha256 | cut -c1-12)
find dist -name '*.js' -not -path '*/vendor/*' -print0 | xargs -0 perl -0777 -pi -e "
  s{((?:from|import\\()\\s*\")(\\.{1,2}/[^\"?:]+\\.js)\"}{\$1\$2?v=$BUILD\"}g;
  s{(fetch\\(\")([^\"?:]+)\"}{\$1\$2?v=$BUILD\"}g;
  s{new URL\\('([A-Za-z0-9_]+_bg\\.wasm)', import\\.meta\\.url\\)}{new URL('\$1?v=$BUILD', import.meta.url)}g;
"
find dist -name '*.html' -print0 | xargs -0 perl -0777 -pi -e "
  s{((?:src|href)=\")([^\"?:#]+\\.(?:js|css))\"}{\$1\$2?v=$BUILD\"}g;
"
if grep -rnE --include='*.js' --exclude-dir=vendor '(from|import\()\s*"\.{1,2}/[^"?]+\.js"|fetch\("[^"?:]+"\)' dist \
   || grep -rnE --include='*.html' '(src|href)="[^"?:#]+\.(js|css)"' dist; then
  echo "the URL above is unstamped and would be served stale after a deploy" >&2
  exit 1
fi
grep -q "_bg.wasm?v=$BUILD" dist/pkg/tool_shim.js || { echo "the WebAssembly URL was not stamped" >&2; exit 1; }

echo "$BUILD $(git rev-parse --short HEAD 2>/dev/null || echo none)" > dist/build.txt
touch dist/.nojekyll

echo "Built dist/ (build $BUILD):"
find dist -type f | sort
wc -c < dist/pkg/tool_shim_bg.wasm | awk '{printf "wasm: %d KB\n", $1/1024}'
