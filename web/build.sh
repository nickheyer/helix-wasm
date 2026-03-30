#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
TARGET_DIR="$ROOT_DIR/target/wasm32-unknown-unknown/release"
OUT_DIR="$SCRIPT_DIR/pkg"
SYSROOT="$ROOT_DIR/patches/wasm-sysroot/include"

# Find clang builtin headers
CLANG_BUILTIN=$(find /nix/store -maxdepth 6 -name 'stdbool.h' -path '*clang*include*' 2>/dev/null | head -1 | xargs dirname 2>/dev/null || true)
if [ -z "$CLANG_BUILTIN" ]; then
    echo "Error: Could not find clang builtin headers. Make sure clang is available."
    exit 1
fi
echo "Using clang builtin headers from: $CLANG_BUILTIN"

echo "Building helix-wasm (release)..."
CC_wasm32_unknown_unknown="clang-19" \
CFLAGS_wasm32_unknown_unknown="--target=wasm32-unknown-unknown -Os -nostdinc -isystem $CLANG_BUILTIN -isystem $SYSROOT" \
RUSTFLAGS="--cfg tokio_unstable --cfg getrandom_backend=\"wasm_js\"" \
    cargo build --target wasm32-unknown-unknown -p helix-wasm --release

echo "Running wasm-bindgen..."
mkdir -p "$OUT_DIR"
wasm-bindgen \
    --target web \
    --out-dir "$OUT_DIR" \
    "$TARGET_DIR/helix_wasm.wasm"

echo "Optimizing with wasm-opt..."
if command -v wasm-opt &> /dev/null; then
    wasm-opt -Oz "$OUT_DIR/helix_wasm_bg.wasm" -o "$OUT_DIR/helix_wasm_bg.wasm"
    echo "Optimized."
else
    echo "wasm-opt not found, skipping optimization."
fi

WASM_SIZE=$(du -h "$OUT_DIR/helix_wasm_bg.wasm" | cut -f1)
echo ""
echo "Build complete!"
echo "  WASM binary: $OUT_DIR/helix_wasm_bg.wasm ($WASM_SIZE)"
echo "  JS bindings: $OUT_DIR/helix_wasm.js"
echo ""
echo "To serve: cd $SCRIPT_DIR && python3 -m http.server 8080"
