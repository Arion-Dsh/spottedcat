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

@group(1) @binding(0) var t_albedo: texture_2d<f32>;
@group(1) @binding(1) var s_sampler: sampler;
@group(1) @binding(2) var t_pbr: texture_2d<f32>;
@group(1) @binding(3) var t_normal: texture_2d<f32>;
@group(1) @binding(4) var t_ao: texture_2d<f32>;
@group(1) @binding(5) var t_emissive: texture_2d<f32>;

@group(2) @binding(0) var<uniform> bone_matrices: array<mat4x4<f32>, 256>;

@group(3) @binding(0) var t_shadow: texture_depth_2d;
@group(3) @binding(1) var s_shadow: sampler_comparison;
@group(3) @binding(2) var t_irradiance: texture_cube<f32>;
@group(3) @binding(3) var t_prefiltered: texture_cube<f32>;
@group(3) @binding(4) var t_brdf_lut: texture_2d<f32>;
@group(3) @binding(5) var s_ibl: sampler;

// MODEL_SHARED_SLOT

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) joint_indices: vec4<u32>,
    @location(4) joint_weights: vec4<f32>,
    @location(9) tangent: vec3<f32>,
};

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

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) shadow_pos: vec3<f32>,
};

fn identity_mat4() -> mat4x4<f32> {
    return mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0),
    );
}

fn skin_matrix(joint_indices: vec4<u32>, joint_weights: vec4<f32>) -> mat4x4<f32> {
    let total_weight = joint_weights.x + joint_weights.y + joint_weights.z + joint_weights.w;
    if (total_weight <= 0.0) {
        return identity_mat4();
    }

    return joint_weights.x * bone_matrices[joint_indices.x]
        + joint_weights.y * bone_matrices[joint_indices.y]
        + joint_weights.z * bone_matrices[joint_indices.z]
        + joint_weights.w * bone_matrices[joint_indices.w];
}

fn build_vertex_output(
    position: vec3<f32>,
    uv: vec2<f32>,
    normal: vec3<f32>,
    tangent: vec3<f32>,
    skin_mat: mat4x4<f32>,
    instance_mat: mat4x4<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    let model_mat = model_globals.model * instance_mat * skin_mat;
    let world = model_mat * vec4<f32>(position, 1.0);
    let shadow = scene.light_view_proj * instance_mat * skin_mat * vec4<f32>(position, 1.0);

    out.world_pos = world.xyz;
    out.clip_position = model_globals.mvp * instance_mat * skin_mat * vec4<f32>(position, 1.0);
    out.uv = uv;
    out.normal = normalize((model_mat * vec4<f32>(normal, 0.0)).xyz);
    out.tangent = normalize((model_mat * vec4<f32>(tangent, 0.0)).xyz);
    out.shadow_pos = shadow.xyz / shadow.w;
    out.shadow_pos = vec3<f32>(
        out.shadow_pos.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5),
        out.shadow_pos.z,
    );
    return out;
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    return build_vertex_output(
        model.position,
        model.uv,
        model.normal,
        model.tangent,
        skin_matrix(model.joint_indices, model.joint_weights),
        identity_mat4(),
    );
}

@vertex
fn vs_main_instanced(model: VertexInputInstanced) -> VertexOutput {
    let instance_mat = mat4x4<f32>(
        model.instance_mat_0,
        model.instance_mat_1,
        model.instance_mat_2,
        model.instance_mat_3,
    );
    return build_vertex_output(
        model.position,
        model.uv,
        model.normal,
        model.tangent,
        skin_matrix(model.joint_indices, model.joint_weights),
        instance_mat,
    );
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let albedo_uv = model_globals.albedo_uv.xy + in.uv * model_globals.albedo_uv.zw;
    let src = textureSample(t_albedo, s_sampler, albedo_uv);

    // MODEL_FRAGMENT_BODY_SLOT
}
