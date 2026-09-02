#!/usr/bin/env bash
# Regenerate the Rust -> WASM bindings used by the playground and extension.
#
# Requires:
#   - stable Rust with the `wasm32-unknown-unknown` target
#       rustup target add wasm32-unknown-unknown
#   - wasm-pack (0.15+)
#       cargo install wasm-pack
#
# Outputs:
#   wasm/pkg/            - fresh build artifacts
#   playground/src/pkg/  - COMMITTED copy the Vite app imports directly
#   extension/pkg/       - copy loaded by extension/wasm-worker.js
set -euo pipefail
cd "$(dirname "$0")"

wasm-pack build --target web --out-dir pkg --release

# Refresh the committed playground bindings (drop the generated `*` .gitignore
# so the refreshed files show up in `git status`).
rm -rf ../playground/src/pkg
mkdir -p ../playground/src/pkg
cp pkg/ghostmark_wasm.d.ts pkg/ghostmark_wasm.js \
   pkg/ghostmark_wasm_bg.wasm pkg/ghostmark_wasm_bg.wasm.d.ts \
   pkg/package.json ../playground/src/pkg/

# Refresh the extension bindings (packaged into ghostmark-release.zip).
rm -rf ../extension/pkg
mkdir -p ../extension/pkg
cp pkg/ghostmark_wasm.d.ts pkg/ghostmark_wasm.js \
   pkg/ghostmark_wasm_bg.wasm pkg/ghostmark_wasm_bg.wasm.d.ts \
   pkg/package.json ../extension/pkg/

echo ""
echo "WASM refreshed. Commit playground/src/pkg/ changes; re-run"
echo "\`npm run release\` in extension/ to rebuild the release zip."