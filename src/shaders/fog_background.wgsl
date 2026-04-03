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

fn remap01(value: f32, start: f32, end: f32) -> f32 {
    return clamp((value - start) / max(end - start, 0.0001), 0.0, 1.0);
}

fn linearize_depth(depth: f32) -> f32 {
    let near = scene.projection_params.z;
    let far = scene.projection_params.w;
    return (near * far) / max(far - depth * (far - near), 0.0001);
}

fn direction_to_sky_color(world_dir: vec3<f32>) -> vec3<f32> {
    let dir = normalize(world_dir);
    let up_amount = clamp(dir.y * 0.5 + 0.5, 0.0, 1.0);
    let zenith = mix(scene.fog_color.rgb, scene.fog_background_zenith.rgb, scene.fog_background_zenith.a);
    let horizon = mix(scene.fog_color.rgb, scene.fog_background_horizon.rgb, scene.fog_background_horizon.a);
    let nadir = mix(scene.fog_color.rgb, scene.fog_background_nadir.rgb, scene.fog_background_nadir.a);
    var sky = mix(nadir, horizon, smoothstep(0.08, 0.52, up_amount));
    sky = mix(sky, zenith, smoothstep(0.52, 1.0, up_amount));

    let horizon_band = 1.0 - abs(up_amount - 0.52) / 0.26;
    let horizon_glow = clamp(horizon_band, 0.0, 1.0);
    return sky + scene.fog_background_horizon.rgb * (scene.fog_background_params.x * horizon_glow);
}

fn compute_distance_fog_optical_depth(world_offset: vec3<f32>) -> f32 {
    let start = scene.fog_distance.x;
    let end = max(scene.fog_distance.y, start + 0.0001);
    let exponent = max(scene.fog_distance.z, 0.0001);
    let density = scene.fog_distance.w;

    if (density <= 0.0) {
        return 0.0;
    }

    let dist = length(world_offset);
    let fog_t = remap01(dist, start, end);
    let shaped = pow(fog_t * fog_t * (3.0 - 2.0 * fog_t), exponent);
    return shaped * density;
}

fn sample_height_fog_density(world_y: f32) -> f32 {
    let base = scene.fog_height.x;
    let falloff = max(scene.fog_height.y, 0.0001);
    let exponent = max(scene.fog_height.z, 0.0001);
    let height = 1.0 - remap01(max(world_y - base, 0.0), 0.0, falloff);
    return pow(clamp(height, 0.0, 1.0), exponent);
}

fn compute_height_fog_optical_depth(world_offset: vec3<f32>) -> f32 {
    let density = scene.fog_height.w;
    let falloff = max(scene.fog_height.y, 0.0001);

    if (density <= 0.0) {
        return 0.0;
    }

    let total_dist = length(world_offset);
    if (total_dist <= 0.0001) {
        return 0.0;
    }

    let min_samples = max(scene.fog_sampling.x, 1.0);
    let max_samples = max(scene.fog_sampling.y, min_samples);
    let sample_scale = max(scene.fog_sampling.z, 0.05);
    let desired_samples = clamp(ceil(total_dist / max(falloff * sample_scale, 1.0)), min_samples, max_samples);
    let sample_count = i32(desired_samples);
    let step_dist = total_dist / max(f32(sample_count), 1.0);
    var accumulated_density = 0.0;

    for (var i = 0; i < 10; i = i + 1) {
        if (i >= sample_count) {
            break;
        }
        let t = (f32(i) + 0.5) / f32(sample_count);
        let sample_pos = scene.camera_pos.xyz + world_offset * t;
        accumulated_density += sample_height_fog_density(sample_pos.y);
    }

    return density * accumulated_density * (step_dist / falloff);
}

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
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let strength = max(scene.fog_params.x, 0.0);
    if (strength <= 0.0) {
        return vec4<f32>(0.0);
    }

    let dims = textureDimensions(t_depth);
    let pixel = vec2<i32>(
        clamp(i32(frag_coord.x), 0, i32(dims.x) - 1),
        clamp(i32(frag_coord.y), 0, i32(dims.y) - 1),
    );
    let depth = textureLoad(t_depth, pixel, 0);
    let has_geometry = depth < 0.99999;

    let uv = vec2<f32>(frag_coord.x / f32(dims.x), frag_coord.y / f32(dims.y));
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let ray = vec3<f32>(
        ndc.x / max(scene.projection_params.x, 0.0001),
        ndc.y / max(scene.projection_params.y, 0.0001),
        1.0,
    );
    let world_dir =
        scene.camera_right.xyz * ray.x +
        scene.camera_up.xyz * ray.y +
        scene.camera_forward.xyz;
    let ray_distance = linearize_depth(select(1.0, depth, has_geometry));
    let world_offset = world_dir * ray_distance;
    let world_pos = scene.camera_pos.xyz + world_offset;

    let optical_depth =
        (compute_distance_fog_optical_depth(world_offset) +
        compute_height_fog_optical_depth(world_offset)) * strength;
    let fog_factor = 1.0 - exp(-optical_depth);
    let background_color = direction_to_sky_color(world_dir);

    if (!has_geometry) {
        let far_background = mix(background_color, scene.fog_color.rgb, scene.fog_background_params.y);
        return vec4<f32>(far_background, 1.0);
    }

    return vec4<f32>(mix(background_color, scene.fog_color.rgb, scene.fog_background_params.z), clamp(fog_factor, 0.0, 1.0));
}
