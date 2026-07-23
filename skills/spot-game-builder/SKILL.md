---
name: spot-game-builder
description: Build and modify playable Rust games that depend on the Spottedcat (`spottedcat` or `spot`) engine. Use when an AI coding agent needs to scaffold or extend a `Spot` scene, choose 2D/3D features, implement gameplay, input, images, text, audio, cameras, models, fog, shaders, scene transitions, or prepare desktop, WebAssembly, Android, or iOS builds in a consumer game project. Trigger for requests such as “用 spot 做游戏”, “用 spottedcat 实现这个玩法”, “build a game with spottedcat”, or “add gameplay/HUD/audio to this Spot scene”.
---

# Spot Game Builder

Build the smallest playable slice that fits the existing game project. Treat the current repository as the application being developed, not as the Spottedcat engine source.

## Establish the project boundary

Inspect before editing:

- `Cargo.toml` and `Cargo.lock` for the actual `spottedcat` version, source, and enabled features
- existing `src/`, assets, scenes, platform wrappers, and project commands
- the current `Spot` implementation and local code conventions
- existing user changes that must be preserved

If the dependency is a local `path`, inspect that checkout for exact APIs and examples. If it is a registry or Git dependency, prefer the version already selected by the project. Do not upgrade Spottedcat, edit the engine dependency, or assume unreleased APIs unless the user asks.

Read [references/api-patterns.md](references/api-patterns.md) before adding or changing Spottedcat code. Read [references/repo-map.md](references/repo-map.md) only when choosing an upstream example, locating deeper engine documentation, or handling a platform target.

## Build a vertical slice

Reduce the request to:

- the repeated player action
- the visible response
- the input methods
- 2D, 3D, or mixed rendering
- one goal, hazard, score, timer, or restart loop

Implement that loop before menus, save systems, generalized frameworks, or content pipelines. Extend existing scenes and patterns when possible.

## Choose the smallest engine surface

- Use the default feature set for 2D, text, input, and audio.
- Add `utils` for encoded image decoding and asynchronous image helpers.
- Add `model-3d` for models, cameras, lights, billboards, or instancing.
- Add `effects` for fog; it includes `model-3d`.
- Add `gltf` for glTF loading; it includes `model-3d` and `utils`.
- Add `sensors` only for supported mobile motion or step APIs.

Do not enable every feature preemptively. Match the dependency version and feature style already used by the project.

## Implement around `Spot`

Keep responsibilities explicit:

1. `initialize`: register fonts, images, sounds, shaders, and models; create scene state.
2. `update`: read input and advance gameplay using the supplied fixed `dt`.
3. `draw`: submit rendering using the supplied `screen`; avoid gameplay mutation and repeated asset creation.
4. `resumed` / `suspended`: handle platform lifecycle only when needed.
5. `remove`: clear scene-specific global state or other explicit cleanup.

Honor these invariants:

- Always use the callback's `&mut Context`.
- Treat 2D origin as top-left and use `Pt` for logical coordinates.
- Multiply rates by `dt.as_secs_f32()` in `update`.
- Treat `WindowConfig::update_hz` as simulation frequency, not render FPS; it must be greater than zero.
- Use `Interpolated<T>` or `ctx.draw_interpolation()` when fixed updates need smooth variable-rate presentation.
- Keep reusable `Image`, `Text`, `Model`, and shader handles in scene state.
- Update existing `Text` with setters when content changes; do not rebuild stable text every draw.
- Decode or load expensive assets outside the render loop. Register GPU resources through the lifecycle context.
- Import only public `spottedcat` APIs. Do not depend on engine internals.

## Reuse upstream examples carefully

Use the closest upstream example to confirm API shape, then adapt it to the consumer project's structure and dependency version. Do not copy engine-repository commands, paths, assets, or platform wrappers into the game project without checking that they apply.

When an example conflicts with the checked-out dependency source or compiler diagnostics, follow the dependency version in the game project.

## Validate proportionally

Start with the cheapest relevant check:

1. format touched Rust files
2. run `cargo check` for the consumer project
3. run its focused tests or target-specific check
4. run the game when visual or interactive behavior must be verified
5. build WASM/mobile only when that platform is in scope

Fix API and feature errors before changing gameplay architecture. If a native dependency or platform toolchain blocks validation, report the exact command and error while preserving the working desktop/core result.

## Finish the task

Deliver runnable code, not only a plan, when implementation was requested. Summarize:

- the playable behavior added
- required features or assets
- commands that passed
- platform-specific work that remains unverified

Avoid claiming visual quality, frame rate, or device support that was not actually tested.
