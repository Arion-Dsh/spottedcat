use crate::model::{Material, Model, ModelPart, Vertex};

/// Loads a GLB or glTF file from a byte slice into a Model.
pub fn load_gltf_from_bytes(ctx: &mut crate::Context, data: &[u8]) -> anyhow::Result<Model> {
    let (document, buffers, images) = gltf::import_slice(data)?;

    // 1. Process images (textures)
    let mut spot_images = Vec::new();
    for img in images {
        let width = img.width;
        let height = img.height;

        // Ensure RGBA8 format
        let rgba = match img.format {
            gltf::image::Format::R8G8B8A8 => img.pixels,
            gltf::image::Format::R8G8B8 => {
                let mut rgba = Vec::with_capacity(img.pixels.len() / 3 * 4);
                for rgb in img.pixels.chunks_exact(3) {
                    rgba.extend_from_slice(rgb);
                    rgba.push(255);
                }
                rgba
            }
            _ => {
                // Fallback attempt or error
                img.pixels
            }
        };

        let spot_img = crate::Image::new(ctx, width.into(), height.into(), &rgba)?;
        spot_images.push(spot_img);
    }

    let mut model_parts = Vec::new();

    // 2. Process meshes
    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            let pos_iter = reader
                .read_positions()
                .ok_or_else(|| anyhow::anyhow!("No positions in primitive"))?;
            let mut uv_iter = reader.read_tex_coords(0).map(|v| v.into_f32());
            let mut norm_iter = reader.read_normals();
            let mut joint_iter = reader.read_joints(0).map(|v| v.into_u16());
            let mut weight_iter = reader.read_weights(0).map(|v| v.into_f32());

            let mut primitive_vertices = Vec::new();
            for pos in pos_iter {
                let uv = uv_iter
                    .as_mut()
                    .and_then(|i| i.next())
                    .unwrap_or([0.0, 0.0]);
                let norm = norm_iter
                    .as_mut()
                    .and_then(|i| i.next())
                    .unwrap_or([0.0, 1.0, 0.0]);
                let joints = joint_iter
                    .as_mut()
                    .and_then(|i| i.next())
                    .unwrap_or([0, 0, 0, 0]);
                let weights = weight_iter
                    .as_mut()
                    .and_then(|i| i.next())
                    .unwrap_or([0.0, 0.0, 0.0, 0.0]);

                primitive_vertices.push(Vertex {
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

            let mut primitive_indices = Vec::new();
            if let Some(indices_reader) = reader.read_indices() {
                for idx in indices_reader.into_u32() {
                    primitive_indices.push(idx);
                }
            } else {
                // Generate sequential indices if missing
                for i in 0..primitive_vertices.len() {
                    primitive_indices.push(i as u32);
                }
            }

            // Check Material
            let mut material = Material::default();
            let g_mat = primitive.material();
            if let Some(tex) = g_mat.pbr_metallic_roughness().base_color_texture() {
                let img_idx = tex.texture().source().index();
                if let Some(spot_img) = spot_images.get(img_idx) {
                    material.albedo = Some(spot_img.id());
                }
            }

            // Create Mesh in Context for persistence
            let mesh_id = ctx.register_mesh(&primitive_vertices, &primitive_indices);

            model_parts.push(ModelPart {
                id: mesh_id,
                material,
            });
        }
    }

    Ok(Model {
        parts: model_parts.into(),
    })
}
