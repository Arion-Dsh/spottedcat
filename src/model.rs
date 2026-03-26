use crate::with_graphics;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Model {
    pub(crate) id: u32,
    pub(crate) material: Material,
}

impl Model {
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Creates a new model from vertex and index data.
    pub fn new(vertices: &[Vertex], indices: &[u32]) -> anyhow::Result<Self> {
        with_graphics(|g| g.create_model(vertices, indices))
            .ok_or_else(|| anyhow::anyhow!("Graphics not initialized"))?
    }

    /// Sets the albedo material (texture) for this model.
    pub fn with_material(mut self, image: crate::Image) -> Self {
        self.material.albedo = Some(image.id());
        self
    }

    pub fn with_albedo(mut self, image: crate::Image) -> Self {
        self.material.albedo = Some(image.id());
        self
    }

    pub fn with_normal_map(mut self, image: crate::Image) -> Self {
        self.material.normal = Some(image.id());
        self
    }

    pub fn with_pbr_map(mut self, image: crate::Image) -> Self {
        self.material.pbr = Some(image.id());
        self
    }

    pub fn with_ao_map(mut self, image: crate::Image) -> Self {
        self.material.occlusion = Some(image.id());
        self
    }

    pub fn with_emissive_map(mut self, image: crate::Image) -> Self {
        self.material.emissive = Some(image.id());
        self
    }

    /// Creates a simple cube model with the specified size.
    pub fn cube(size: f32) -> anyhow::Result<Self> {
        let s = size / 2.0;
        let vertices = vec![
            // Front face
            Vertex { pos: [-s, -s,  s], uv: [0.0, 1.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [ s, -s,  s], uv: [1.0, 1.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [ s,  s,  s], uv: [1.0, 0.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [-s,  s,  s], uv: [0.0, 0.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [-s, -s, -s], uv: [1.0, 1.0], normal: [0.0, 0.0, -1.0], tangent: [-1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [-s,  s, -s], uv: [1.0, 0.0], normal: [0.0, 0.0, -1.0], tangent: [-1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [ s,  s, -s], uv: [0.0, 0.0], normal: [0.0, 0.0, -1.0], tangent: [-1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [ s, -s, -s], uv: [0.0, 1.0], normal: [0.0, 0.0, -1.0], tangent: [-1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [-s,  s, -s], uv: [0.0, 0.0], normal: [0.0, 1.0, 0.0], tangent: [1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [-s,  s,  s], uv: [0.0, 1.0], normal: [0.0, 1.0, 0.0], tangent: [1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [ s,  s,  s], uv: [1.0, 1.0], normal: [0.0, 1.0, 0.0], tangent: [1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [ s,  s, -s], uv: [1.0, 0.0], normal: [0.0, 1.0, 0.0], tangent: [1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [-s, -s, -s], uv: [1.0, 1.0], normal: [0.0, -1.0, 0.0], tangent: [1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [ s, -s, -s], uv: [0.0, 1.0], normal: [0.0, -1.0, 0.0], tangent: [1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [ s, -s,  s], uv: [0.0, 0.0], normal: [0.0, -1.0, 0.0], tangent: [1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [-s, -s,  s], uv: [1.0, 0.0], normal: [0.0, -1.0, 0.0], tangent: [1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [ s, -s, -s], uv: [1.0, 1.0], normal: [1.0, 0.0, 0.0], tangent: [0.0, 0.0, -1.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [ s,  s, -s], uv: [1.0, 0.0], normal: [1.0, 0.0, 0.0], tangent: [0.0, 0.0, -1.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [ s,  s,  s], uv: [0.0, 0.0], normal: [1.0, 0.0, 0.0], tangent: [0.0, 0.0, -1.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [ s, -s,  s], uv: [0.0, 1.0], normal: [1.0, 0.0, 0.0], tangent: [0.0, 0.0, -1.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [-s, -s, -s], uv: [0.0, 1.0], normal: [-1.0, 0.0, 0.0], tangent: [0.0, 0.0, 1.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [-s, -s,  s], uv: [1.0, 1.0], normal: [-1.0, 0.0, 0.0], tangent: [0.0, 0.0, 1.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [-s,  s,  s], uv: [1.0, 0.0], normal: [-1.0, 0.0, 0.0], tangent: [0.0, 0.0, 1.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [-s,  s, -s], uv: [0.0, 0.0], normal: [-1.0, 0.0, 0.0], tangent: [0.0, 0.0, 1.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
        ];

        let indices = vec![
            0, 1, 2,  0, 2, 3,    // Front
            4, 5, 6,  4, 6, 7,    // Back
            8, 9, 10, 8, 10, 11,  // Top
            12, 13, 14, 12, 14, 15, // Bottom
            16, 17, 18, 16, 18, 19, // Right
            20, 21, 22, 20, 22, 23, // Left
        ];

        let mut model = Self::new(&vertices, &indices)?;
        model.material = Material::default();
        Ok(model)
    }

    /// Creates a 2D plane model in 3D space, facing +Z. Good for billboards or ground planes.
    pub fn plane(width: f32, height: f32) -> anyhow::Result<Self> {
        let hw = width / 2.0;
        let hh = height / 2.0;
        
        // Vertices face +Z direction
        let vertices = vec![
            Vertex { pos: [-hw, -hh, 0.0], uv: [0.0, 1.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [ hw, -hh, 0.0], uv: [1.0, 1.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [ hw,  hh, 0.0], uv: [1.0, 0.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
            Vertex { pos: [-hw,  hh, 0.0], uv: [0.0, 0.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0], joint_indices: [0; 4], joint_weights: [0.0; 4] },
        ];

        let indices = vec![0, 1, 2, 0, 2, 3];

        let mut model = Self::new(&vertices, &indices)?;
        model.material = Material::default();
        Ok(model)
    }

    /// Creates a UV sphere model with the specified radius.
    pub fn sphere(radius: f32) -> anyhow::Result<Self> {
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

        let mut model = Self::new(&vertices, &indices)?;
        model.material = Material::default();
        Ok(model)
    }

    pub fn draw(&self, context: &mut crate::Context, options: crate::DrawOption3D) {
        context.push_3d(crate::drawable::DrawCommand3D::Model(
            *self,
            options,
            0,
            crate::ShaderOpts::default(),
            None,
        ));
    }

    pub fn draw_skinned(
        &self,
        context: &mut crate::Context,
        options: crate::DrawOption3D,
        skin_id: u32,
    ) {
        context.push_3d(crate::drawable::DrawCommand3D::Model(
            *self,
            options,
            0,
            crate::ShaderOpts::default(),
            Some(skin_id),
        ));
    }

    pub fn draw_with_shader(
        &self,
        context: &mut crate::Context,
        shader_id: u32,
        options: crate::DrawOption3D,
        shader_opts: crate::ShaderOpts,
        skin_id: Option<u32>,
    ) {
        context.push_3d(crate::drawable::DrawCommand3D::Model(
            *self,
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
        context: &mut crate::Context,
        options: crate::DrawOption3D,
        transforms: &[[[f32; 4]; 4]],
    ) {
        if transforms.is_empty() { return; }
        context.push_3d(crate::drawable::DrawCommand3D::ModelInstanced(
            *self,
            options,
            0,
            crate::ShaderOpts::default(),
            None,
            transforms.to_vec(),
        ));
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
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
