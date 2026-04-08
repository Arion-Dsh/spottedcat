# Spot Repo Map

Use this reference when you need to choose a starting point, the right feature flags, or a local run command for `spottedcat`.

## Core files

- `README.md`
High-level engine overview, stable surfaces, feature list, and supported platforms.
- `AI_GAME_GENERATION_GUIDE.md`
Best first read for architecture, lifecycle, `Spot`, input, 2D, 3D, audio, and context rules.
- `Cargo.toml`
Feature flags and example declarations.
- `src/lib.rs`
Public API surface and top-level docs.

## Engine concepts to bias toward

- `Context` is the runtime hub for rendering, input, audio, and resources.
- `Spot` is the main lifecycle trait. Most game work starts here.
- `Image` and `Text` cover most 2D and HUD needs.
- `Model` and `DrawOption3D` cover 3D primitives and loaded geometry.
- `run::<T>(WindowConfig)` is the normal app entrypoint.

## Feature flags

- default: lightweight 2D-oriented baseline
- `model-3d`: enable 3D model APIs and helpers
- `effects`: enable fog-related workflows on top of `model-3d`
- `utils`: image/helper utilities
- `gltf`: glTF loading, also enables `model-3d` and `utils`
- `sensors`: motion and step APIs

Choose the smallest set that unlocks the requested work.

## Example chooser

- `examples/input_example.rs`
Start here for movement, keyboard handling, delta-time motion, and text overlays.
- `examples/audio_test.rs`
Start here for quick sound proof-of-life.
- `examples/fog_world.rs`
Start here for 3D camera setup, fog, lighting, procedural meshes, and FPS text.
- `examples/instancing_test.rs`
Start here when many repeated 3D props are needed.
- `examples/gltf_loader.rs`
Start here when the request depends on external model assets.
- `examples/billboard.rs`
Start here when 2D art must live inside a 3D scene.
- `examples/wasm/`
Read when the target is browser-hosted.
- `examples/android/`
Read when packaging or embedding for Android matters.
- `examples/ios/`
Read when packaging or embedding for iOS matters.

## Fast local commands

Run from the repo root unless a platform example says otherwise.

```bash
cargo check
cargo run --example input_example
cargo run --example audio_test
cargo run --example fog_world --features model-3d,effects
cargo run --example gltf_loader --features gltf
```

WASM flow:

```bash
cd examples/wasm/wasm_demo
wasm-pack build --target web
```

Android flow:

- inspect `examples/android/build_spottedcat_android_libs.sh`
- inspect `examples/android/GameActivityExample/`

iOS flow:

- inspect `examples/ios/build_spottedcat_xcframework.sh`
- inspect `examples/ios/SpottedcatIosSimulatorExample/`

## Practical implementation bias

- Prefer one small `Spot` scene before introducing scene switching.
- Prefer procedural placeholder art or generated textures before building an asset pipeline.
- Prefer desktop verification before browser or mobile deployment.
- Prefer adapting an existing example over inventing a new architecture.
- Prefer a vertical slice over a broad but incomplete framework.
