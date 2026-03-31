#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WRAPPER_DIR="$OUT_DIR/spottedcat_ios_wrapper"
HEADER_DIR="$OUT_DIR/include"
XCFRAMEWORK_PATH="$OUT_DIR/Spottedcat.xcframework"

rm -rf "$XCFRAMEWORK_PATH"

rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios

cargo build --manifest-path "$WRAPPER_DIR/Cargo.toml" --release --target aarch64-apple-ios
cargo build --manifest-path "$WRAPPER_DIR/Cargo.toml" --release --target aarch64-apple-ios-sim
cargo build --manifest-path "$WRAPPER_DIR/Cargo.toml" --release --target x86_64-apple-ios

DEVICE_LIB="$WRAPPER_DIR/target/aarch64-apple-ios/release/libspottedcat_ios_wrapper.a"
SIM_ARM64_LIB="$WRAPPER_DIR/target/aarch64-apple-ios-sim/release/libspottedcat_ios_wrapper.a"
SIM_X64_LIB="$WRAPPER_DIR/target/x86_64-apple-ios/release/libspottedcat_ios_wrapper.a"

mkdir -p "$OUT_DIR/.tmp"

SIM_UNIVERSAL_LIB="$OUT_DIR/.tmp/libspottedcat_sim_universal.a"
lipo -create "$SIM_ARM64_LIB" "$SIM_X64_LIB" -output "$SIM_UNIVERSAL_LIB"

xcodebuild -create-xcframework \
  -library "$DEVICE_LIB" -headers "$HEADER_DIR" \
  -library "$SIM_UNIVERSAL_LIB" -headers "$HEADER_DIR" \
  -output "$XCFRAMEWORK_PATH"

echo "Created: $XCFRAMEWORK_PATH"
