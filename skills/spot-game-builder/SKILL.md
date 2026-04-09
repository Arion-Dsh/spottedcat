---
name: spot-game-builder
description: Plan and implement games quickly with the spottedcat (`spot`) Rust engine. Use when Codex needs to turn a game idea into working code in a repo that uses `spottedcat`, especially for scaffolding a new `Spot` scene, choosing 2D vs 3D engine features, mapping gameplay into `Context`/`Spot`/draw calls, reusing the repo's examples, wiring input, text, audio, fog, models, or preparing desktop, WASM, Android, or iOS game builds. Also use when the request is phrased in Chinese, such as “用 spot 快速做游戏”, “帮我用 spottedcat 做一个小游戏”, “把这个玩法实现成 Spot 场景”, “基于这个仓库做游戏原型”, or in English, such as “build a game with spottedcat”, “prototype a game in spot”, “turn this game idea into a Spot scene”, “add gameplay/HUD/audio to this spottedcat project”, or “use the repo examples to make a playable demo”.
---

# Spot Game Builder

## Overview

Use this skill to move from a game idea to a playable `spottedcat` prototype with as little ceremony as possible. Favor a small vertical slice first, reuse existing engine examples aggressively, and keep the implementation aligned with the engine's stable surfaces: `Context`, `Spot`, `Image`, `Model`, `Text`, and `run`.

## Workflow

### 1. Confirm the engine root and existing baseline

Look for:

- `Cargo.toml` with package name `spottedcat`
- `README.md`
- `AI_GAME_GENERATION_GUIDE.md`
- `examples/`

If those files exist, treat that repo as the primary engine source of truth. Read `AI_GAME_GENERATION_GUIDE.md` first for architecture and lifecycle expectations. Read [references/repo-map.md](references/repo-map.md) when you need example selection, feature flags, or build commands.

### 2. Turn the request into a tiny playable slice

Before writing code, reduce the request into:

- player fantasy: what the player does repeatedly
- camera/render mode: 2D, 3D, or mixed UI + 3D
- inputs: keyboard, mouse, touch, sensors
- renderables: image, text, model, instancing, fog, shader
- win loop: survive, dodge, collect, score, explore

Start with the smallest playable version that proves the loop. Examples:

- "做一个俯视角射击原型" -> move, aim, spawn bullets, one enemy type, score text
- "做一个 3D 雾气场景漫游" -> camera, environment models, movement, fog tuning, FPS overlay
- "做一个手机计步小游戏" -> sensor input, simple HUD, daily step readout, one interaction loop

Avoid building menus, save systems, and content pipelines before the core loop feels real.

### 3. Choose the engine path

Use this decision rule:

- Choose pure 2D when the game can be expressed with `Image`, `Text`, `DrawOption`, and screen-space coordinates.
- Choose 3D when the request needs `Model`, camera placement, lighting, fog, billboards, or instancing.
- Choose mixed 2D + 3D when the world is 3D but HUD or overlays should remain 2D text/images.

Choose features conservatively:

- no extra features for minimal 2D
- `model-3d` for procedural 3D primitives and model drawing
- `effects` when using fog or effect helpers
- `utils` when loading PNGs and similar helper-driven assets
- `gltf` when loading glTF models
- `sensors` for motion or step APIs on supported platforms

### 4. Reuse the nearest example instead of inventing structure

Pick the closest baseline from the repo and adapt it:

- `examples/input_example.rs` for movement, input polling, and text HUD
- `examples/audio_test.rs` for quick sound verification
- `examples/fog_world.rs` for 3D scene setup, camera, fog, and overlay text
- `examples/instancing_test.rs` for many repeated 3D objects
- `examples/gltf_loader.rs` for asset-loaded 3D content
- `examples/wasm/` or mobile wrappers when the target is browser or device-specific

Do not rewrite engine conventions from scratch when an example already demonstrates the needed pattern.

### 5. Implement around `Spot`

Structure game code around one `Spot` implementation per scene or prototype state.

Typical shape:

1. `initialize`
Load or register assets, set camera/light/fog defaults, and create the initial state.
2. `update`
Read input, advance simulation with `dt.as_secs_f32()`, and queue scene changes if needed.
3. `draw`
Issue all draw calls for the frame. Keep gameplay mutation out of rendering unless the pattern is trivial and already established in the repo.
4. `resumed` / `suspended`
Use when platform lifecycle matters, especially on mobile.
5. `remove`
Clean up scene-specific state and clear temporary global scene configuration like fog when needed.

Honor these rules:

- Always use the lifecycle `ctx` that the engine provides.
- Multiply motion and time-based effects by delta time.
- Treat 2D origin as top-left.
- Use `Pt` for logical 2D units and viewport-relative helpers when layout should scale.
- For encoded PNG/JPEG/WebP bytes, use `Image::from_bytes(ctx, data)`.
- Keep the first implementation simple enough to run immediately.

### 6. Map common game needs to `spottedcat`

- sprite or HUD element -> `Image::new` plus `image.draw`
- on-screen text -> `register_font`, `Text::new`, `spottedcat::text::draw`
- player movement -> `key_down`, `key_pressed`, `mouse_down`, `mouse_pos`, `touches`
- simple sound feedback -> `register_sound` + `play_sound`, or `play_sine` for fast smoke tests
- 3D blockout -> `model::create_cube`, `create_plane`, `create_sphere`
- large repeated props -> `model::draw_instanced`
- scene transitions -> `switch_scene` or `switch_scene_with`
- shared state -> `insert_resource` and `get_resource`
- atmosphere -> `set_fog`, light helpers, camera helpers

### 7. Verify in the cheapest environment first

Prefer this validation order unless the request is platform-specific:

1. compile or run on desktop first
2. verify the closest example or new prototype locally
3. move to WASM or mobile only after the core loop behaves correctly

When debugging:

- start with `cargo check`
- then run the narrowest example or target
- separate engine misuse from game logic mistakes
- read existing examples before adding new abstractions

### 8. Escalate complexity only after the slice works

After the first playable slice works, then add:

- more scenes
- asset loading
- custom shaders
- better camera behavior
- platform packaging
- content polish

When the request is broad, prefer shipping a playable prototype plus a clear next-step list instead of an unfinished full game architecture.

## Prompt Patterns

These are good triggers for this skill:

- "用 spot 快速做一个 2D 平台跳跃原型"
- "帮我在 spottedcat 里搭一个 3D 雾景漫游 demo"
- "把这个游戏想法映射成 `Spot` 场景和实现步骤"
- "基于这个 repo 的 examples 做一个可玩的小游戏"
- "帮我给这个 spottedcat 项目加输入、HUD 和音效"
- "Use spottedcat to build a small 2D platformer prototype"
- "Help me create a 3D foggy exploration demo in spot"
- "Turn this game idea into a `Spot` scene and implementation plan"
- "Use this repo's examples to build a playable mini-game"
- "Add input, HUD, and audio to this spottedcat game project"

## Reference Use

Read [references/repo-map.md](references/repo-map.md) when you need:

- the recommended example to start from
- `Cargo.toml` feature selection
- common local run commands
- a quick map of repo files that matter for game work
