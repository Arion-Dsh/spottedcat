struct ModelGlobals {
    mvp: mat4x4<f32>,
    model: mat4x4<f32>,
};

// Group 0: Globals (only ModelGlobals needed for shadow)
@group(0) @binding(0) var<uniform> model_globals: ModelGlobals;

// Group 1: Bones
@group(1) @binding(0) var<uniform> bone_matrices: array<mat4x4<f32>, 256>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(3) joint_indices: vec4<u32>,
    @location(4) joint_weights: vec4<f32>,
    @location(5) instance_mat_0: vec4<f32>,
    @location(6) instance_mat_1: vec4<f32>,
    @location(7) instance_mat_2: vec4<f32>,
    @location(8) instance_mat_3: vec4<f32>,
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

    let instance_mat = mat4x4<f32>(
        model.instance_mat_0,
        model.instance_mat_1,
        model.instance_mat_2,
        model.instance_mat_3
    );

    return model_globals.mvp * instance_mat * skin_mat * vec4<f32>(model.position, 1.0);
}
