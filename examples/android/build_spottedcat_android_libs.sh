#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WRAPPER_DIR="$OUT_DIR/spottedcat_android_wrapper"
GAME_ACTIVITY_EXAMPLE_DIR="$OUT_DIR/GameActivityExample"
JNI_LIBS_DIR="$GAME_ACTIVITY_EXAMPLE_DIR/app/src/main/jniLibs"

mkdir -p "$OUT_DIR/.tmp"

rustup target add \
  aarch64-linux-android \
  armv7-linux-androideabi \
  i686-linux-android \
  x86_64-linux-android

echo "Building spottedcat Android shared libs (.so)"

if ! command -v cargo-ndk >/dev/null 2>&1; then
  echo "error: cargo-ndk not found. Install it with: cargo install cargo-ndk" >&2
  exit 1
fi

if [[ -z "${ANDROID_NDK_HOME:-}" && -z "${ANDROID_NDK_ROOT:-}" ]]; then
  DEFAULT_NDK_DIR="$HOME/Library/Android/sdk/ndk"
  if [[ -d "$DEFAULT_NDK_DIR" ]]; then
    DETECTED_NDK="$(ls -1d "$DEFAULT_NDK_DIR"/* 2>/dev/null | sort | tail -n 1)"
    if [[ -n "${DETECTED_NDK:-}" && -d "$DETECTED_NDK" ]]; then
      export ANDROID_NDK_HOME="$DETECTED_NDK"
      echo "Detected ANDROID_NDK_HOME=$ANDROID_NDK_HOME"
    fi
  fi
fi

if [[ -z "${ANDROID_NDK_HOME:-}" && -z "${ANDROID_NDK_ROOT:-}" ]]; then
  echo "error: ANDROID_NDK_HOME (or ANDROID_NDK_ROOT) is not set." >&2
  echo "Please set it to your Android NDK directory (e.g. .../Android/sdk/ndk/<version>)." >&2
  exit 1
fi

mkdir -p "$JNI_LIBS_DIR"

cargo ndk -o "$JNI_LIBS_DIR" -t arm64-v8a -t armeabi-v7a -t x86 -t x86_64 -P 26 \
  --manifest-path "$WRAPPER_DIR/Cargo.toml" \
  build --release

echo "Built and copied .so files into: $JNI_LIBS_DIR/<abi>/"
