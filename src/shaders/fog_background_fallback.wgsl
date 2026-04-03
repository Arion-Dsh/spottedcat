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

struct VsOut {
    @builtin(position) position: vec4<f32>,
};

@group(0) @binding(0) var<uniform> scene: SceneGlobals;
@group(0) @binding(1) var t_depth: texture_depth_2d;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(3.0, 1.0),
    );

    var out: VsOut;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    _ = scene.fog_params.x;
    _ = textureDimensions(t_depth);
    return vec4<f32>(0.0);
}
