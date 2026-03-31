#!/usr/bin/env bash
set -euo pipefail

# Build wasm demo into ../pkg so web/ can import it.
# Requires: wasm-pack

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$SCRIPT_DIR/wasm_demo"
OUT_DIR="$SCRIPT_DIR/pkg"

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

wasm-pack build "$CRATE_DIR" --release --target web --out-dir "$OUT_DIR"

echo "[ok] wasm pkg generated at: $OUT_DIR"
