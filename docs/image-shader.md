# Image Shader Guide

This document describes the custom image shader system in `spottedcat`.

## Overview

Custom shaders in `spottedcat` are written in **WGSL**. While the engine handles the underlying `wgpu` state, you have full control over the vertex and fragment stages.

There are two main ways to create an image shader:

1.  **`ImageShaderTemplate`** (Recommended): A slot-based API where you only provide the shader body. The engine handles all standard boilerplate (structs, uniforms, vertex logic).
2.  **`ImageShaderDesc` with `internal_prelude`**: Full control over `vs_main` and `fs_main`, but with engine-standard structs and bindings automatically injected.

---

## The Recommended Approach: Templates

Using `ImageShaderTemplate` avoids repetitive boilerplate and makes your shaders more resilient to engine updates.

### 1. Registration

```rust
let shader_id = spottedcat::register_image_shader_template(
    ctx,
    spottedcat::ImageShaderTemplate::new()
        .with_extra_textures(true)
        .with_history_at(0)   // Map History semantic to slot 0
        .with_screen_at(1)    // Map Screen semantic to slot 1
        .with_texture_alias(2, "t_noise") // Custom name for slot 2
        .with_fragment_body(r#"
            // 't_history', 't_screen', 't_noise' are auto-injected
            let history = textureSample(t_history, extra_samp, in.uv);
            let screen_bg = textureSample(t_screen, extra_samp, in.uv);
            let noise = textureSample(t_noise, extra_samp, in.local_uv).r;

            // 'src', 'opacity', 'screen', and 'scale_factor' are also auto-injected
            let final_color = mix(src.rgb, history.rgb, noise) * opacity;
            return vec4<f32>(final_color, src.a);
        "#),
);
```

### 2. Semantic Binding (Draw Time)

When drawing with `draw_with_shader_bindings`, you bind textures by **intent** rather than index:

```rust
let bindings = ImageShaderBindings::new()
    .with_history()                // Engine knows this belongs in slot 0
    .with_screen()                 // Engine knows this belongs in slot 1
    .with_image("t_noise", my_noise); // Engine knows this belongs in slot 2

screen.draw_with_shader_bindings(ctx, sprite, shader_id, opts, shader_opts, bindings);
```

---

## Automatic Injection (Prelude)

When using `ImageShaderTemplate` or `ImageShaderDesc::with_internal_prelude(true)`, the engine injects a standard set of WGSL definitions into your shader.

### Injected Structs

-   `VsIn`: Vertex input (position, rotation, size, uv_rect).
-   `VsOut`: Fragment input (clip_pos, uv, local_uv, uv_scale).
-   `EngineGlobals`: Screen size and global opacity info.

### Injected Variables

In your vertex/fragment bodies, you can directly access:

-   `src`: (Fragment only) The sampled color of the main image.
-   `opacity`: The combined effect of `DrawOption` and `ShaderOpts` opacity.
-   `screen`: A `vec4<f32>` containing `[2/w, 2/h, 1/w, 1/h]`.
-   `scale_factor`: The current device pixel ratio.
-   `user_globals`: An array of 16 `vec4<f32>` containing your `ShaderOpts` data.

### Injected Bindings

-   `tex`: The main source image texture.
-   `samp`: The primary linear sampler.
-   `extra_samp`: A linear sampler for all extra textures.
-   `t_history`, `t_screen`: (If slots are assigned) Semantic texture aliases.
-   `t0` to `t3`: Generic names for extra textures (fallback).

---

## Semantic Descriptions

### `screen` snapshot
A snapshot of the current render target taken *before* the current draw batch began. Useful for post-processing effects and localized distortion.

### `history` snapshot
A snapshot of the render target from the **end of the previous frame**. Essential for temporal effects like trails, motion blur, and accumulation buffers.

---

## Low-Level: Full WGSL Contract

If you disable `internal_prelude`, you must handle all bindings and structs manually.

### Bind Group Layout

-   `@group(0)`: Main texture (`binding(0)`) and sampler (`binding(1)`).
-   `@group(1)`: Extra textures (`binding(0-3)`) and extra sampler (`binding(4)`).
-   `@group(2)`: User globals (`ShaderOpts`).
-   `@group(3)`: Engine globals (Internal state).

*Note: Group indices shift down if extra textures are disabled.*

### Vertex Input Layout

```wgsl
struct VsIn {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) pos: vec2<f32>,
    @location(1) rotation: f32,
    @location(2) size: vec2<f32>,
    @location(3) uv_rect: vec4<f32>,
};
```

---

## Examples

-   Template mode: `examples/image_shader_template.rs`
-   Full WGSL mode: `examples/advanced_image_shader_full.rs`
