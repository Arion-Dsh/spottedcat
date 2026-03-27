struct ModelGlobals {
    mvp: mat4x4<f32>,
    model: mat4x4<f32>,
};

// Group 0: Globals (only ModelGlobals needed for shadow)
@group(0) @binding(0) var<uniform> model_globals: ModelGlobals;

// Group 1: Bones (was Group 2 in full shader, but shadow pipeline only uses Groups 0,1)
@group(1) @binding(0) var<uniform> bone_matrices: array<mat4x4<f32>, 256>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(3) joint_indices: vec4<u32>,
    @location(4) joint_weights: vec4<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> @builtin(position) vec4<f32> {
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

    return model_globals.mvp * skin_mat * vec4<f32>(model.position, 1.0);
}
