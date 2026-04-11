use crate::model::Vertex;

/// Loads a simple OBJ file from a byte slice.
/// Supports only triangular faces (f v1/vt1/vn1 v2/vt2/vn2 v3/vt3/vn3).
/// Parses OBJ data into raw vertex and index buffers.
pub fn parse_obj_data(data: &[u8]) -> anyhow::Result<(Vec<Vertex>, Vec<u32>)> {
    let text = std::str::from_utf8(data)?;
    let mut positions = Vec::new();
    let mut uvs = Vec::new();
    let mut normals = Vec::new();
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut vertex_cache = std::collections::HashMap::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "v" => {
                if parts.len() >= 4 {
                    positions.push([
                        parts[1].parse::<f32>()?,
                        parts[2].parse::<f32>()?,
                        parts[3].parse::<f32>()?,
                    ]);
                }
            }
            "vt" => {
                if parts.len() >= 3 {
                    uvs.push([
                        parts[1].parse::<f32>()?,
                        1.0 - parts[2].parse::<f32>()?, // Flip Y for wgpu
                    ]);
                }
            }
            "vn" => {
                if parts.len() >= 4 {
                    normals.push([
                        parts[1].parse::<f32>()?,
                        parts[2].parse::<f32>()?,
                        parts[3].parse::<f32>()?,
                    ]);
                }
            }
            "f" => {
                // Support up to quads by triangulating (triangle fan)
                let face_indices: Vec<(usize, Option<usize>, Option<usize>)> = parts[1..]
                    .iter()
                    .map(|p| {
                        let sub: Vec<&str> = p.split('/').collect();
                        let v = sub[0].parse::<usize>().unwrap_or(1) - 1;
                        let vt = if sub.len() > 1 && !sub[1].is_empty() {
                            Some(sub[1].parse::<usize>().unwrap_or(1) - 1)
                        } else {
                            None
                        };
                        let vn = if sub.len() > 2 && !sub[2].is_empty() {
                            Some(sub[2].parse::<usize>().unwrap_or(1) - 1)
                        } else {
                            None
                        };
                        (v, vt, vn)
                    })
                    .collect();

                for i in 1..face_indices.len() - 1 {
                    let tris = [face_indices[0], face_indices[i], face_indices[i + 1]];
                    for key in tris {
                        if let Some(&idx) = vertex_cache.get(&key) {
                            indices.push(idx);
                        } else {
                            let idx = vertices.len() as u32;
                            vertices.push(Vertex {
                                pos: positions[key.0],
                                uv: key.1.map(|u| uvs[u]).unwrap_or([0.0, 0.0]),
                                normal: key.2.map(|n| normals[n]).unwrap_or([0.0, 1.0, 0.0]),
                                tangent: [0.0, 0.0, 0.0],
                                joint_indices: [0; 4],
                                joint_weights: [0.0; 4],
                            });
                            vertex_cache.insert(key, idx);
                            indices.push(idx);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok((vertices, indices))
}

/// Loads a simple OBJ file from a byte slice into a Model.
pub fn load_obj_from_bytes(
    ctx: &mut crate::Context,
    data: &[u8],
) -> anyhow::Result<crate::model::Model> {
    let (vertices, indices) = parse_obj_data(data)?;
    crate::model::Model::new(ctx, &vertices, &indices)
}
