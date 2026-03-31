/// Handle to a 3D model resource.
///
/// Models are collection of meshes and materials that can be rendered in 3D space.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Material {
    pub albedo: Option<u32>,
    pub pbr: Option<u32>, // Metallic (B) / Roughness (G)
    pub normal: Option<u32>,
    pub occlusion: Option<u32>,
    pub emissive: Option<u32>,
}

impl Material {
    pub fn with_albedo(mut self, image: crate::Image) -> Self {
        self.albedo = Some(image.id());
        self
    }
    pub fn with_pbr(mut self, image: crate::Image) -> Self {
        self.pbr = Some(image.id());
        self
    }
    pub fn with_normal(mut self, image: crate::Image) -> Self {
        self.normal = Some(image.id());
        self
    }
    pub fn with_occlusion(mut self, image: crate::Image) -> Self {
        self.occlusion = Some(image.id());
        self
    }
    pub fn with_emissive(mut self, image: crate::Image) -> Self {
        self.emissive = Some(image.id());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModelPart {
    pub(crate) id: u32,
    pub(crate) material: Material,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Model {
    pub(crate) parts: Vec<ModelPart>,
}

impl Model {
    pub fn first_id(&self) -> u32 {
        self.parts.first().map(|p| p.id).unwrap_or(0)
    }

    /// Creates a new model from vertex and index data.
    pub fn new(
        ctx: &mut crate::Context,
        vertices: &[Vertex],
        indices: &[u32],
    ) -> anyhow::Result<Self> {
        let mesh_id = ctx.register_mesh(vertices, indices);
        Ok(Self {
            parts: vec![ModelPart {
                id: mesh_id,
                material: Material::default(),
            }],
        })
    }

    /// Sets the albedo material (texture) for this model.
    pub fn with_material(mut self, image: crate::Image) -> Self {
        for part in &mut self.parts {
            part.material.albedo = Some(image.id());
        }
        self
    }

    pub fn with_albedo(mut self, image: crate::Image) -> Self {
        for part in &mut self.parts {
            part.material.albedo = Some(image.id());
        }
        self
    }

    pub fn with_normal_map(mut self, image: crate::Image) -> Self {
        for part in &mut self.parts {
            part.material.normal = Some(image.id());
        }
        self
    }

    pub fn with_pbr_map(mut self, image: crate::Image) -> Self {
        for part in &mut self.parts {
            part.material.pbr = Some(image.id());
        }
        self
    }

    pub fn with_ao_map(mut self, image: crate::Image) -> Self {
        for part in &mut self.parts {
            part.material.occlusion = Some(image.id());
        }
        self
    }

    pub fn with_emissive_map(mut self, image: crate::Image) -> Self {
        for part in &mut self.parts {
            part.material.emissive = Some(image.id());
        }
        self
    }

    pub fn with_part_material(mut self, index: usize, material: Material) -> Self {
        if let Some(part) = self.parts.get_mut(index) {
            part.material = material;
        }
        self
    }

    /// Appends a new sub-mesh part to the model.
    pub fn add_part(
        &mut self,
        ctx: &mut crate::Context,
        vertices: &[Vertex],
        indices: &[u32],
        material: Material,
    ) -> anyhow::Result<&mut Self> {
        let mesh_id = ctx.register_mesh(vertices, indices);
        self.parts.push(ModelPart {
            id: mesh_id,
            material,
        });
        Ok(self)
    }

    /// Chaining version of add_part.
    pub fn with_part(
        mut self,
        ctx: &mut crate::Context,
        vertices: &[Vertex],
        indices: &[u32],
        material: Material,
    ) -> anyhow::Result<Self> {
        self.add_part(ctx, vertices, indices, material)?;
        Ok(self)
    }

    /// Creates a simple cube model with the specified size.
    pub fn cube(ctx: &mut crate::Context, size: f32) -> anyhow::Result<Self> {
        let s = size / 2.0;
        let vertices = vec![
            // Front face
            Vertex {
                pos: [-s, -s, s],
                uv: [0.0, 1.0],
                normal: [0.0, 0.0, 1.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [s, -s, s],
                uv: [1.0, 1.0],
                normal: [0.0, 0.0, 1.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [s, s, s],
                uv: [1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [-s, s, s],
                uv: [0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [-s, -s, -s],
                uv: [1.0, 1.0],
                normal: [0.0, 0.0, -1.0],
                tangent: [-1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [-s, s, -s],
                uv: [1.0, 0.0],
                normal: [0.0, 0.0, -1.0],
                tangent: [-1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [s, s, -s],
                uv: [0.0, 0.0],
                normal: [0.0, 0.0, -1.0],
                tangent: [-1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [s, -s, -s],
                uv: [0.0, 1.0],
                normal: [0.0, 0.0, -1.0],
                tangent: [-1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [-s, s, -s],
                uv: [0.0, 0.0],
                normal: [0.0, 1.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [-s, s, s],
                uv: [0.0, 1.0],
                normal: [0.0, 1.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [s, s, s],
                uv: [1.0, 1.0],
                normal: [0.0, 1.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [s, s, -s],
                uv: [1.0, 0.0],
                normal: [0.0, 1.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [-s, -s, -s],
                uv: [1.0, 1.0],
                normal: [0.0, -1.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [s, -s, -s],
                uv: [0.0, 1.0],
                normal: [0.0, -1.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [s, -s, s],
                uv: [0.0, 0.0],
                normal: [0.0, -1.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [-s, -s, s],
                uv: [1.0, 0.0],
                normal: [0.0, -1.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [s, -s, -s],
                uv: [1.0, 1.0],
                normal: [1.0, 0.0, 0.0],
                tangent: [0.0, 0.0, -1.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [s, s, -s],
                uv: [1.0, 0.0],
                normal: [1.0, 0.0, 0.0],
                tangent: [0.0, 0.0, -1.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [s, s, s],
                uv: [0.0, 0.0],
                normal: [1.0, 0.0, 0.0],
                tangent: [0.0, 0.0, -1.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [s, -s, s],
                uv: [0.0, 1.0],
                normal: [1.0, 0.0, 0.0],
                tangent: [0.0, 0.0, -1.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [-s, -s, -s],
                uv: [0.0, 1.0],
                normal: [-1.0, 0.0, 0.0],
                tangent: [0.0, 0.0, 1.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [-s, -s, s],
                uv: [1.0, 1.0],
                normal: [-1.0, 0.0, 0.0],
                tangent: [0.0, 0.0, 1.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [-s, s, s],
                uv: [1.0, 0.0],
                normal: [-1.0, 0.0, 0.0],
                tangent: [0.0, 0.0, 1.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [-s, s, -s],
                uv: [0.0, 0.0],
                normal: [-1.0, 0.0, 0.0],
                tangent: [0.0, 0.0, 1.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
        ];

        let indices = vec![
            0, 1, 2, 0, 2, 3, // Front
            4, 5, 6, 4, 6, 7, // Back
            8, 9, 10, 8, 10, 11, // Top
            12, 13, 14, 12, 14, 15, // Bottom
            16, 17, 18, 16, 18, 19, // Right
            20, 21, 22, 20, 22, 23, // Left
        ];

        Self::new(ctx, &vertices, &indices)
    }

    /// Creates a 2D plane model in 3D space, facing +Z. Good for billboards or ground planes.
    pub fn plane(ctx: &mut crate::Context, width: f32, height: f32) -> anyhow::Result<Self> {
        let hw = width / 2.0;
        let hh = height / 2.0;

        // Vertices face +Z direction
        let vertices = vec![
            Vertex {
                pos: [-hw, -hh, 0.0],
                uv: [0.0, 1.0],
                normal: [0.0, 0.0, 1.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [hw, -hh, 0.0],
                uv: [1.0, 1.0],
                normal: [0.0, 0.0, 1.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [hw, hh, 0.0],
                uv: [1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
            Vertex {
                pos: [-hw, hh, 0.0],
                uv: [0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
            },
        ];

        let indices = vec![0, 1, 2, 0, 2, 3];

        Self::new(ctx, &vertices, &indices)
    }

    /// Creates a UV sphere model with the specified radius.
    pub fn sphere(ctx: &mut crate::Context, radius: f32) -> anyhow::Result<Self> {
        let segments = 32;
        let rings = 16;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for y in 0..=rings {
            let phi = y as f32 * std::f32::consts::PI / rings as f32;
            for x in 0..=segments {
                let theta = x as f32 * 2.0 * std::f32::consts::PI / segments as f32;

                let nx = phi.sin() * theta.cos();
                let ny = phi.cos();
                let nz = phi.sin() * theta.sin();

                let tx = -phi.sin() * theta.sin();
                let tz = phi.sin() * theta.cos();
                let tangent = if phi.sin().abs() < 1e-6 {
                    [1.0, 0.0, 0.0]
                } else {
                    let len = (tx * tx + tz * tz).sqrt();
                    [tx / len, 0.0, tz / len]
                };

                vertices.push(Vertex {
                    pos: [radius * nx, radius * ny, radius * nz],
                    uv: [x as f32 / segments as f32, y as f32 / rings as f32],
                    normal: [nx, ny, nz],
                    tangent,
                    joint_indices: [0; 4],
                    joint_weights: [0.0; 4],
                });
            }
        }

        for y in 0..rings {
            for x in 0..segments {
                let first = y * (segments + 1) + x;
                let second = first + segments + 1;

                indices.push(first);
                indices.push(first + 1);
                indices.push(second);

                indices.push(second);
                indices.push(first + 1);
                indices.push(second + 1);
            }
        }

        Self::new(ctx, &vertices, &indices)
    }

    pub fn draw(&self, ctx: &mut crate::Context, options: crate::DrawOption3D) {
        ctx.push_3d(crate::drawable::DrawCommand3D::Model(
            self.clone(),
            options,
            0,
            crate::ShaderOpts::default(),
            None,
        ));
    }

    pub fn draw_skinned(
        &self,
        ctx: &mut crate::Context,
        options: crate::DrawOption3D,
        skin_id: u32,
    ) {
        ctx.push_3d(crate::drawable::DrawCommand3D::Model(
            self.clone(),
            options,
            0,
            crate::ShaderOpts::default(),
            Some(skin_id),
        ));
    }

    pub fn draw_with_shader(
        &self,
        ctx: &mut crate::Context,
        shader_id: u32,
        options: crate::DrawOption3D,
        shader_opts: crate::ShaderOpts,
        skin_id: Option<u32>,
    ) {
        ctx.push_3d(crate::drawable::DrawCommand3D::Model(
            self.clone(),
            options,
            shader_id,
            shader_opts,
            skin_id,
        ));
    }

    /// Renders thousands of instances of this model with a single draw call.
    ///
    /// `transforms` should be an array of 4x4 matrices representing the View/Model transformations
    /// for each instance. This achieves massive performance improvements for identical meshes.
    pub fn draw_instanced(
        &self,
        ctx: &mut crate::Context,
        options: crate::DrawOption3D,
        transforms: &[[[f32; 4]; 4]],
    ) {
        if transforms.is_empty() {
            return;
        }
        self.draw_instanced_shared(ctx, options, std::sync::Arc::from(transforms));
    }

    /// Renders instances using a caller-owned transform buffer without making an extra copy.
    pub fn draw_instanced_owned(
        &self,
        ctx: &mut crate::Context,
        options: crate::DrawOption3D,
        transforms: Vec<[[f32; 4]; 4]>,
    ) {
        if transforms.is_empty() {
            return;
        }
        self.draw_instanced_shared(ctx, options, std::sync::Arc::from(transforms));
    }

    /// Renders instances backed by shared transform data.
    pub fn draw_instanced_shared(
        &self,
        ctx: &mut crate::Context,
        options: crate::DrawOption3D,
        transforms: std::sync::Arc<[[[f32; 4]; 4]]>,
    ) {
        if transforms.is_empty() {
            return;
        }
        ctx.push_3d(crate::drawable::DrawCommand3D::ModelInstanced(
            self.clone(),
            options,
            0,
            crate::ShaderOpts::default(),
            None,
            transforms,
        ));
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
    pub joint_indices: [u32; 4],
    pub joint_weights: [f32; 4],
}

impl Vertex {
    pub(crate) fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12, // 3 * 4
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 20, // (3 + 2) * 4
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 32, // (3 + 2 + 3) * 4
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 44, // (3 + 2 + 3 + 3) * 4
                    shader_location: 3,
                    format: wgpu::VertexFormat::Uint32x4,
                },
                wgpu::VertexAttribute {
                    offset: 60, // (3 + 2 + 3 + 3 + 4) * 4
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshDataPersistent {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}
