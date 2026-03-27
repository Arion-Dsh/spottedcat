struct ModelGlobals {
    mvp: mat4x4<f32>,
    model: mat4x4<f32>,
    extra: vec4<f32>, // x: opacity
    albedo_uv: vec4<f32>,
    pbr_uv: vec4<f32>,
    normal_uv: vec4<f32>,
    ao_uv: vec4<f32>,
    emissive_uv: vec4<f32>,
};

struct Light {
    position: vec4<f32>, // w=1.0 point, w=0.0 directional
    color: vec4<f32>,    // rgb: color, a: intensity
};

struct SceneGlobals {
    camera_pos: vec4<f32>,
    ambient_color: vec4<f32>,
    lights: array<Light, 4>,
    light_view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> model_globals: ModelGlobals;

@group(1) @binding(0) var t_albedo: texture_2d<f32>;
@group(1) @binding(1) var s_sampler: sampler;
@group(1) @binding(2) var t_pbr: texture_2d<f32>;
@group(1) @binding(3) var t_normal: texture_2d<f32>;
@group(1) @binding(4) var t_ao: texture_2d<f32>;
@group(1) @binding(5) var t_emissive: texture_2d<f32>;

@group(2) @binding(0) var<uniform> user_globals: array<vec4<f32>, 16>;
@group(3) @binding(0) var<uniform> bone_matrices: array<mat4x4<f32>, 256>;
@group(4) @binding(0) var<uniform> scene: SceneGlobals;
@group(5) @binding(0) var t_shadow: texture_depth_2d;
@group(5) @binding(1) var s_shadow: sampler_comparison;
@group(6) @binding(0) var t_irradiance: texture_cube<f32>;
@group(6) @binding(1) var t_prefiltered: texture_cube<f32>;
@group(6) @binding(2) var t_brdf_lut: texture_2d<f32>;
@group(6) @binding(3) var s_ibl: sampler;

struct VertexInput {
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

const PI: f32 = 3.14159265359;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var skin_mat = mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0)
    );
    
    let total_weight = model.joint_weights.x + model.joint_weights.y + model.joint_weights.z + model.joint_weights.w;
    if (total_weight > 0.0) {
        skin_mat = 
            model.joint_weights.x * bone_matrices[model.joint_indices.x] +
            model.joint_weights.y * bone_matrices[model.joint_indices.y] +
            model.joint_weights.z * bone_matrices[model.joint_indices.z] +
            model.joint_weights.w * bone_matrices[model.joint_indices.w];
    }

    let instance_mat = mat4x4<f32>(
        model.instance_mat_0,
        model.instance_mat_1,
        model.instance_mat_2,
        model.instance_mat_3
    );

    var out: VertexOutput;
    let world_pos = (model_globals.model * instance_mat * skin_mat * vec4<f32>(model.position, 1.0)).xyz;
    out.world_pos = world_pos;
    out.clip_position = model_globals.mvp * instance_mat * skin_mat * vec4<f32>(model.position, 1.0);
    out.uv = model.uv; // Pass raw UV through
    out.normal = normalize((model_globals.model * instance_mat * skin_mat * vec4<f32>(model.normal, 0.0)).xyz);
    out.tangent = normalize((model_globals.model * instance_mat * skin_mat * vec4<f32>(model.tangent, 0.0)).xyz);
    
    let shadow_pos = scene.light_view_proj * instance_mat * skin_mat * vec4<f32>(model.position, 1.0);
    out.shadow_pos = shadow_pos.xyz / shadow_pos.w;
    out.shadow_pos = vec3<f32>(out.shadow_pos.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5), out.shadow_pos.z);

    return out;
}

fn DistributionGGX(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let NdotH = max(dot(N, H), 0.0);
    let NdotH2 = NdotH * NdotH;
    let num = a2;
    var denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom + 0.0000001;
    return num / denom;
}

fn GeometrySchlickGGX(NdotV: f32, roughness: f32) -> f32 {
    let r = (roughness + 1.0);
    let k = (r * r) / 8.0;
    let num = NdotV;
    let denom = NdotV * (1.0 - k) + k;
    return num / denom;
}

fn GeometrySmith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32 {
    let NdotV = max(dot(N, V), 0.0);
    let NdotL = max(dot(N, L), 0.0);
    let ggx2 = GeometrySchlickGGX(NdotV, roughness);
    let ggx1 = GeometrySchlickGGX(NdotL, roughness);
    return ggx1 * ggx2;
}

fn fresnelSchlick(cosTheta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

fn fresnelSchlickRoughness(cosTheta: f32, F0: vec3<f32>, roughness: f32) -> vec3<f32> {
    return F0 + (max(vec3<f32>(1.0 - roughness), F0) - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

fn fetch_shadow(shadow_pos: vec3<f32>) -> f32 {
    if (shadow_pos.x < 0.0 || shadow_pos.x > 1.0 || shadow_pos.y < 0.0 || shadow_pos.y > 1.0) {
        return 1.0;
    }
    
    var shadow: f32 = 0.0;
    let texel_size = 1.0 / 2048.0;
    for (var y: i32 = -1; y <= 1; y = y + 1) {
        for (var x: i32 = -1; x <= 1; x = x + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            shadow += textureSampleCompare(t_shadow, s_shadow, shadow_pos.xy + offset, shadow_pos.z - 0.005);
        }
    }
    return shadow / 9.0;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let albedo_uv = model_globals.albedo_uv.xy + in.uv * model_globals.albedo_uv.zw;
    let pbr_uv = model_globals.pbr_uv.xy + in.uv * model_globals.pbr_uv.zw;
    let normal_uv = model_globals.normal_uv.xy + in.uv * model_globals.normal_uv.zw;
    let ao_uv = model_globals.ao_uv.xy + in.uv * model_globals.ao_uv.zw;
    let emissive_uv = model_globals.emissive_uv.xy + in.uv * model_globals.emissive_uv.zw;

    let albedo = textureSample(t_albedo, s_sampler, albedo_uv).rgb;
    let pbr_data = textureSample(t_pbr, s_sampler, pbr_uv);
    let roughness = max(pbr_data.g, 0.05);
    let metallic = pbr_data.b;
    let ao = textureSample(t_ao, s_sampler, ao_uv).r;
    let emissive = textureSample(t_emissive, s_sampler, emissive_uv).rgb;
    
    let normal_map = textureSample(t_normal, s_sampler, normal_uv).rgb * 2.0 - 1.0;
    let world_N = normalize(in.normal);
    let world_T = normalize(in.tangent);
    let world_B = cross(world_N, world_T);
    let TBN = mat3x3<f32>(world_T, world_B, world_N);
    let N = normalize(TBN * normal_map);
    
    let V = normalize(scene.camera_pos.xyz - in.world_pos);

    var F0 = vec3<f32>(0.04);
    F0 = mix(F0, albedo, metallic);

    var Lo = vec3<f32>(0.0);
    for (var i = 0; i < 4; i = i + 1) {
        let light = scene.lights[i];
        var L: vec3<f32>;
        var attenuation: f32;
        
        if (light.position.w == 0.0) { // Directional
            L = normalize(light.position.xyz);
            attenuation = 1.0;
        } else { // Point
            let diff = light.position.xyz - in.world_pos;
            let distance = length(diff);
            L = normalize(diff);
            attenuation = 1.0 / (distance * distance + 0.0001);
        }

        let H = normalize(V + L);
        let radiance = light.color.rgb * light.color.a * attenuation;

        let NDF = DistributionGGX(N, H, roughness);
        let G = GeometrySmith(N, V, L, roughness);
        let F = fresnelSchlick(max(dot(H, V), 0.0), F0);

        let kS = F;
        var kD = vec3<f32>(1.0) - kS;
        kD *= 1.0 - metallic;

        let numerator = NDF * G * F;
        let denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.0001;
        let specular = numerator / denominator;

        let NdotL = max(dot(N, L), 0.0);
        let shadow = select(1.0, fetch_shadow(in.shadow_pos), i == 0); // Only first light casts shadows
        Lo += (kD * albedo / PI + specular) * radiance * NdotL * shadow;
    }

    var ambient = scene.ambient_color.rgb * albedo * ao;
    
    // IBL
    let R = reflect(-V, N);
    let F_ibl = fresnelSchlickRoughness(max(dot(N, V), 0.0), F0, roughness);
    let kS_ibl = F_ibl;
    let kD_ibl = (1.0 - kS_ibl) * (1.0 - metallic);
    
    let irradiance = textureSample(t_irradiance, s_ibl, N).rgb;
    let diffuse_ibl = irradiance * albedo;
    
    let prefiltered_color = textureSampleLevel(t_prefiltered, s_ibl, R, roughness * 4.0).rgb;
    let env_brdf = textureSample(t_brdf_lut, s_ibl, vec2<f32>(max(dot(N, V), 0.0), roughness)).rg;
    let specular_ibl = prefiltered_color * (F_ibl * env_brdf.x + env_brdf.y);
    
    ambient = ambient + (kD_ibl * diffuse_ibl + specular_ibl) * ao;

    var color = ambient + Lo + emissive;
    
    // Simple HDR tone mapping
    color = color / (color + vec3<f32>(1.0));
    // Linear to Srgb
    color = pow(color, vec3<f32>(1.0/2.2));
    
    var final_color = vec4<f32>(color, model_globals.extra.x);
    
    // USER_FS_HOOK
    
    return final_color;
}
