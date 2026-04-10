use spottedcat::math::mat4::identity;
use spottedcat::model::Bone;
use spottedcat::model::{Model, Vertex};
use spottedcat::{Context, DrawOption3D, Spot, WindowConfig};

struct GltfApp {
    model: Model,
    rotation: f32,
}

impl Spot for GltfApp {
    fn initialize(ctx: &mut Context) -> Self {
        // 1. Setup scene-wide PBR lighting
        spottedcat::set_ambient_light(ctx, [0.2, 0.2, 0.2, 1.0]);
        // A bright directional light from the top-right
        spottedcat::set_light(ctx, 0, [10.0, 10.0, 10.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        // Set camera position (matching the hardcoded view matrix in render.rs)
        spottedcat::set_camera_pos(ctx, [0.0, 0.0, 5.0]);

        // 2. Create a model (using a sphere for PBR demonstration)
        let model = spottedcat::model::create_sphere(ctx, 1.0).unwrap();

        Self {
            model,
            rotation: 0.0,
        }
    }

    fn update(&mut self, _ctx: &mut Context, dt: std::time::Duration) {
        self.rotation += dt.as_secs_f32() * 0.5;
    }

    fn draw(&mut self, ctx: &mut Context, screen: spottedcat::Image) {
        let opts = DrawOption3D::default()
            .with_position([0.0, 0.0, 0.0]) // Already at -5 in view space
            .with_rotation([0.0, self.rotation, 0.0]);

        screen.draw(ctx, &self.model, opts);
    }

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    spottedcat::run::<GltfApp>(WindowConfig::default());
}

/// A reference implementation of a glTF loader using the `gltf` crate.
/// This would live in your application layer.
pub fn load_gltf(ctx: &mut Context, path: &str) -> anyhow::Result<(Model, u32)> {
    let (document, buffers, _) = gltf::import(path)?;

    let mut all_vertices = Vec::new();
    let mut all_indices = Vec::new();

    // 1. Extract Mesh Data
    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            let pos_iter = reader
                .read_positions()
                .ok_or_else(|| anyhow::anyhow!("No positions"))?;
            let mut uv_iter = reader.read_tex_coords(0).map(|v| v.into_f32());
            let mut norm_iter = reader.read_normals();
            let mut joint_iter = reader.read_joints(0).map(|v| v.into_u16());
            let mut weight_iter = reader.read_weights(0).map(|v| v.into_f32());

            let base_idx = all_vertices.len() as u32;

            for pos in pos_iter {
                let uv = uv_iter
                    .as_mut()
                    .and_then(|i| i.next())
                    .unwrap_or([0.0, 0.0]);
                let norm = norm_iter
                    .as_mut()
                    .and_then(|i| i.next())
                    .unwrap_or([0.0, 0.0, 1.0]);
                let joints = joint_iter
                    .as_mut()
                    .and_then(|i| i.next())
                    .unwrap_or([0, 0, 0, 0]);
                let weights = weight_iter
                    .as_mut()
                    .and_then(|i| i.next())
                    .unwrap_or([0.0, 0.0, 0.0, 0.0]);

                all_vertices.push(Vertex {
                    pos,
                    uv,
                    normal: norm,
                    tangent: [1.0, 0.0, 0.0], // Default tangent
                    joint_indices: [
                        joints[0] as u32,
                        joints[1] as u32,
                        joints[2] as u32,
                        joints[3] as u32,
                    ],
                    joint_weights: weights,
                });
            }

            if let Some(indices_reader) = reader.read_indices() {
                for idx in indices_reader.into_u32() {
                    all_indices.push(base_idx + idx);
                }
            }
        }
    }

    let model = spottedcat::model::create(ctx, &all_vertices, &all_indices)?;

    // 2. Extract Skin Data
    let mut skin_id = 0;
    if let Some(skin) = document.skins().next() {
        let reader = skin.reader(|buffer| Some(&buffers[buffer.index()]));
        let ibms: Vec<[[f32; 4]; 4]> = reader
            .read_inverse_bind_matrices()
            .map(|i| i.collect())
            .unwrap_or_default();

        // Build hierarchy
        let mut node_parents = std::collections::HashMap::new();
        for node in document.nodes() {
            for child in node.children() {
                node_parents.insert(child.index(), node.index());
            }
        }

        let mut bones = Vec::new();
        let skin_joints: Vec<_> = skin.joints().collect();
        for (i, joint_node) in skin_joints.iter().enumerate() {
            let ibm = ibms.get(i).copied().unwrap_or(identity());
            let parent_index = node_parents
                .get(&joint_node.index())
                .and_then(|p_idx| skin_joints.iter().position(|node| node.index() == *p_idx));

            bones.push(Bone {
                parent_index,
                inverse_bind_matrix: ibm,
            });
        }

        let initial_matrices = vec![identity(); bones.len()];
        skin_id = ctx.create_skin(bones, initial_matrices);
    }

    Ok((model, skin_id))
}
