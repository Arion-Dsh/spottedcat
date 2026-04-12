# Image Shader Layout

This document describes the public `Image` shader contract for `spottedcat`.

The runtime does not expose `wgpu`, but custom image shaders still run against a fixed bind-group layout and a fixed vertex buffer layout.

## Modes

Generate a starter WGSL with:

```rust
let wgsl = spottedcat::image_shader_template();
```

If you want the generated template to include the extra-texture bind group, use the template API:

```rust
let wgsl = spottedcat::ImageShaderTemplate::new()
    .with_extra_textures(true)
    .build();
```

For the highest control, register the edited shader with:

1. `register_image_shader_desc(ctx, ImageShaderDesc::from_wgsl(source))`
   - Full WGSL mode.
   - You provide the full shader source.
   - This is the only supported custom `Image` shader path.

The recommended path is the limited template API:

```rust
let shader_id = spottedcat::register_image_shader_template(
    ctx,
    spottedcat::ImageShaderTemplate::new()
    .with_extra_textures(true)
    .with_shared("fn tint(c: vec3<f32>) -> vec3<f32> { return c * vec3<f32>(1.0, 0.5, 0.8); }")
    .with_vertex_body("out.local_uv = out.local_uv * 0.9 + vec2<f32>(0.05, 0.05);")
    .with_fragment_body("return vec4<f32>(tint(src.rgb), src.a * opacity);")
);
```

Supported slots:

1. `shared`
   - helper functions, constants, and shared WGSL declarations
2. `vertex_body`
   - a small vertex-stage customization block inserted before `return out`
3. `fragment_body`
   - the body inserted into `fs_main` after `src` and `opacity` are prepared

## Pipeline Contract

These parts stay engine-defined even in full WGSL mode:

1. Render pipeline topology: triangle strip quad rendering.
2. Draw model: one quad per image draw.
3. Vertex buffer layout: the engine writes one instance record per draw.
4. Uniform payload for custom data: `ShaderOpts` as `array<vec4<f32>, 16>`.
5. Extra texture count limit: up to 4.

The blend mode is configurable through `ImageShaderBlendMode`.

## Bind Group Layout

### Without extra textures

1. `@group(0)` source image texture and sampler
2. `@group(1)` user globals
3. `@group(2)` engine globals

### With extra textures

If `ImageShaderDesc::with_extra_textures(true)` is used:

1. `@group(0)` source image texture and sampler
2. `@group(1)` extra textures and sampler
3. `@group(2)` user globals
4. `@group(3)` engine globals

The extra-texture bind group always exposes 4 texture bindings plus 1 sampler. If your shader only needs 1 or 2 extra textures, you can ignore the unused bindings.

## Binding Details

### `@group(0)`: source image

```wgsl
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
```

This is the image passed as the `source` argument to `draw_with_shader` or `draw_with_shader_bindings`.

### `@group(1)`: extra textures

Present only when `with_extra_textures(true)` is enabled.

```wgsl
@group(1) @binding(0) var t0: texture_2d<f32>;
@group(1) @binding(1) var t1: texture_2d<f32>;
@group(1) @binding(2) var t2: texture_2d<f32>;
@group(1) @binding(3) var t3: texture_2d<f32>;
@group(1) @binding(4) var extra_samp: sampler;
```

These slots are populated from `ImageShaderBindings`.

```rust
let bindings = ImageShaderBindings::new()
    .with_extra_image(0, noise)
    .with_screen(1)
    .with_history(2);
```

Slot mapping:

1. `with_extra_image(slot, image)` -> samples another `Image`
2. `with_screen(slot)` -> samples the current target snapshot
3. `with_history(slot)` -> samples the previous-frame target snapshot

Unused slots fall back to the source image texture.

## `screen` And `history`

`screen` and `history` are target-relative, not global.

For a draw into some target texture `T`:

1. `screen`
   - A snapshot of `T` taken before the target's image batch begins.
   - For the main screen, this means the state after the 3D/base pass and before the 2D image batch.

2. `history`
   - A snapshot of `T` from the end of the previous frame.
   - On the first frame, it falls back to `screen`.

Important: `screen` is not a live read of the current attachment during the same pass.

## User Globals Layout

Custom per-draw data comes from `ShaderOpts`.

WGSL declaration:

```wgsl
@group(N) @binding(0) var<uniform> user_globals: array<vec4<f32>, 16>;
```

`N` is:

1. `1` when extra textures are disabled
2. `2` when extra textures are enabled

Rust side example:

```rust
let mut opts = ShaderOpts::default();
opts.set_vec4(0, [time, 0.0, 0.0, 0.0]);
opts.set_vec4(1, [1.0, 0.5, 0.8, 1.0]);
```

## Engine Globals Layout

WGSL declaration:

```wgsl
struct EngineGlobals {
    screen: vec4<f32>,
    opacity: f32,
    shader_opacity: f32,
    scale_factor: f32,
    _padding: f32,
};

@group(N) @binding(0) var<uniform> _sp_internal: EngineGlobals;
```

`N` is:

1. `2` when extra textures are disabled
2. `3` when extra textures are enabled

Field meanings:

1. `screen.xy`
   - `2.0 / logical_width`, `2.0 / logical_height`
2. `screen.zw`
   - `1.0 / logical_width`, `1.0 / logical_height`
3. `opacity`
   - draw opacity from `DrawOption`
4. `shader_opacity`
   - `ShaderOpts::opacity`
5. `scale_factor`
   - current window/device scale factor

## Vertex Layout

The engine provides one instanced vertex record per image draw.

WGSL input shape:

```wgsl
struct VsIn {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) pos: vec2<f32>,
    @location(1) rotation: f32,
    @location(2) size: vec2<f32>,
    @location(3) uv_rect: vec4<f32>,
};
```

Field meanings:

1. `pos`
   - logical draw position
2. `rotation`
   - per-draw rotation in radians
3. `size`
   - final logical draw size after scaling
4. `uv_rect`
   - source UV rectangle in atlas space

## Entry Points

In full WGSL mode, entry point names are currently fixed:

1. Vertex: `vs_main`
2. Fragment: `fs_main`

## Example

See:

1. `examples/advanced_image_shader_full.rs`

That example demonstrates:

1. full WGSL registration
2. extra image textures
3. `screen`
4. `history`
5. additive blending
