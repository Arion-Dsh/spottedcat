#!/usr/bin/env bash
set -euo pipefail

echo "Checking default-feature examples..."
cargo check --example input_example
cargo check --example async_loading_example
cargo check --example audio_test
cargo check --example centered_text_test
cargo check --example flip_test
cargo check --example one_shot_splash
cargo check --example seven_level_test
cargo check --example text_performance_test
cargo check --example touch_test

echo "Checking utils examples..."
cargo check --example happy_tree_desktop --features utils
cargo check --example atlas_subimage_move_test --features utils

echo "Checking model-3d examples..."
cargo check --example billboard --features model-3d
cargo check --example instancing_test --features model-3d
cargo check --example metal_sphere --features model-3d
cargo check --example model_test --features model-3d
cargo check --example render_state_stress --features model-3d

echo "Checking effects examples..."
cargo check --example fog_world --features effects

echo "Checking gltf examples..."
cargo check --example gltf_loader --features gltf
cargo check --example animated_gltf --features gltf

echo "All examples compiled successfully."
