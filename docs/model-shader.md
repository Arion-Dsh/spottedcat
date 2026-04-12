# Model Shader Layout

This document describes the public custom 3D model shader contract.

`spottedcat` now treats custom model shaders as full WGSL sources. There is no hook-injection path for `Image` or `Model` shaders.

## Registration

For the highest control, generate a starter WGSL with:

```rust
let wgsl = spottedcat::model_shader_template();
```

Edit that template, then register it with:

Register a custom model shader with:

```rust
let shader_id = spottedcat::register_model_shader(ctx, wgsl_source);
```

The recommended path is the limited template API:

```rust
let shader_id = spottedcat::register_model_shader_template(
    ctx,
    spottedcat::ModelShaderTemplate::new()
        .with_shared("fn tint(c: vec3<f32>) -> vec3<f32> { return c * vec3<f32>(0.8, 0.9, 1.0); }")
        .with_fragment_body("return vec4<f32>(tint(src.rgb), src.a * model_globals.extra.x);"),
);
```

Supported slots:

1. `shared`
   - helper functions, constants, and shared WGSL declarations
2. `fragment_body`
   - the body inserted into `fs_main` after `src` is prepared

The source must define these entry points:

1. `vs_main`
2. `vs_main_instanced`
3. `fs_main`

`vs_main` is used for normal model draws.

`vs_main_instanced` is used for instanced model draws.

`fs_main` is shared by both pipelines.

## Pipeline Contract

These parts remain engine-defined:

1. Pipeline layout and bind groups
2. Vertex buffer layout
3. Instanced vertex layout
4. Depth format and color target format
5. Primitive topology and culling
6. `ShaderOpts` payload shape

## Bind Groups

### `@group(0)`: model, scene, user globals

```wgsl
struct ModelGlobals {
    mvp: mat4x4<f32>,
    model: mat4x4<f32>,
    extra: vec4<f32>,
    albedo_uv: vec4<f32>,
    pbr_uv: vec4<f32>,
    normal_uv: vec4<f32>,
    ao_uv: vec4<f32>,
    emissive_uv: vec4<f32>,
};

struct Light {
    position: vec4<f32>,
    color: vec4<f32>,
};

struct SceneGlobals {
    camera_pos: vec4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
    camera_forward: vec4<f32>,
    projection_params: vec4<f32>,
    ambient_color: vec4<f32>,
    fog_color: vec4<f32>,
    fog_distance: vec4<f32>,
    fog_height: vec4<f32>,
    fog_params: vec4<f32>,
    fog_background_zenith: vec4<f32>,
    fog_background_horizon: vec4<f32>,
    fog_background_nadir: vec4<f32>,
    fog_background_params: vec4<f32>,
    fog_sampling: vec4<f32>,
    lights: array<Light, 4>,
    light_view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> model_globals: ModelGlobals;
@group(0) @binding(1) var<uniform> scene: SceneGlobals;
@group(0) @binding(2) var<uniform> user_globals: array<vec4<f32>, 16>;
```

`user_globals` is populated from `ShaderOpts`.

### `@group(1)`: material textures

```wgsl
@group(1) @binding(0) var t_albedo: texture_2d<f32>;
@group(1) @binding(1) var s_sampler: sampler;
@group(1) @binding(2) var t_pbr: texture_2d<f32>;
@group(1) @binding(3) var t_normal: texture_2d<f32>;
@group(1) @binding(4) var t_ao: texture_2d<f32>;
@group(1) @binding(5) var t_emissive: texture_2d<f32>;
```

### `@group(2)`: bones

```wgsl
@group(2) @binding(0) var<uniform> bone_matrices: array<mat4x4<f32>, 256>;
```

### `@group(3)`: environment and shadow resources

```wgsl
@group(3) @binding(0) var t_shadow: texture_depth_2d;
@group(3) @binding(1) var s_shadow: sampler_comparison;
@group(3) @binding(2) var t_irradiance: texture_cube<f32>;
@group(3) @binding(3) var t_prefiltered: texture_cube<f32>;
@group(3) @binding(4) var t_brdf_lut: texture_2d<f32>;
@group(3) @binding(5) var s_ibl: sampler;
```

## Vertex Inputs

### Standard draw vertex input

```wgsl
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) joint_indices: vec4<u32>,
    @location(4) joint_weights: vec4<f32>,
    @location(9) tangent: vec3<f32>,
};
```

### Instanced draw vertex input

```wgsl
struct VertexInputInstanced {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) joint_indices: vec4<u32>,
    @location(4) joint_weights: vec4<f32>,
    @location(9) tangent: vec3<f32>,
    @location(5) instance_mat_0: vec4<f32>,
    @location(6) instance_mat_1: vec4<f32>,
    @location(7) instance_mat_2: vec4<f32>,
    @location(8) instance_mat_3: vec4<f32>,
};
```

## Vertex Output

Both vertex entry points should return the same varyings expected by `fs_main`.

Typical shape:

```wgsl
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) shadow_pos: vec3<f32>,
};
```

## Notes

1. Standard and instanced shaders must be kept in the same WGSL source.
2. `spottedcat::model_shader_template()` returns a ready-to-edit starting point for the current contract.
3. `spottedcat::ModelShaderTemplate` is the recommended higher-level path for common material customizations.
4. `examples/metal_sphere.rs` shows the recommended template-based path.
5. `examples/advanced_model_shader_full.rs` shows the advanced full-WGSL path.
