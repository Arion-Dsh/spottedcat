use std::sync::Arc;

/// Structure representing skeletal animation skin data.
#[derive(Debug, Clone)]
pub struct SkinData {
    /// List of bones in the skin.
    pub bones: Vec<Bone>,
    /// Pre-calculated bone matrices for the current frame.
    pub bone_matrices: Vec<[[f32; 4]; 4]>,
}

/// A single bone in a skeleton.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bone {
    /// Index of the parent bone, or None if this is the root.
    pub parent_index: Option<usize>,
    /// Inverse bind matrix for skinning.
    pub inverse_bind_matrix: [[f32; 4]; 4],
}

/// A 3D camera with perspective projection settings.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    /// Eye position of the camera.
    pub eye: [f32; 3],
    /// Target point the camera is looking at.
    pub target: [f32; 3],
    /// Up vector of the camera.
    pub up: [f32; 3],
    /// Aspect ratio of the viewport (width / height).
    pub aspect: f32,
    /// Vertical field of view in degrees.
    pub fovy: f32,
    /// Near clipping plane distance.
    pub znear: f32,
    /// Far clipping plane distance.
    pub zfar: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            eye: [0.0, 0.0, 5.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            aspect: 1.0,
            fovy: 45.0,
            znear: 0.1,
            zfar: 1000.0,
        }
    }
}

impl Camera {
    /// Returns the view matrix (eye -> target).
    pub fn view_matrix(&self) -> [[f32; 4]; 4] {
        crate::math::mat4::look_at(self.eye, self.target, self.up)
    }

    /// Returns the perspective projection matrix for this camera.
    pub fn projection_matrix(&self) -> [[f32; 4]; 4] {
        crate::math::projection::perspective_degrees(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

/// A 3D light source (Point or Directional).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default, Debug)]
pub struct Light {
    /// Position or direction of the light. W component: 1.0 = point, 0.0 = directional.
    pub position: [f32; 4],
    /// Color and intensity. [R, G, B, Intensity].
    pub color: [f32; 4],
}

/// Global scene-level environment settings.
///
/// This structure matches the GPU uniform buffer layout for scene globals.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default, Debug)]
pub struct SceneGlobals {
    /// Camera position for lighting calculations.
    pub camera_pos: [f32; 4],
    /// View-space right vector.
    pub camera_right: [f32; 4],
    /// View-space up vector.
    pub camera_up: [f32; 4],
    /// View-space forward vector.
    pub camera_forward: [f32; 4],
    /// Projection parameters: [proj_x, proj_y, znear, zfar].
    pub projection_params: [f32; 4],
    /// Ambient light color.
    pub ambient_color: [f32; 4],
    /// Fog color.
    pub fog_color: [f32; 4],
    /// Fog distance settings: [start, end, exponent, density].
    pub fog_distance: [f32; 4],
    /// Fog height settings: [base, falloff, exponent, density].
    pub fog_height: [f32; 4],
    /// Fog general parameters.
    pub fog_params: [f32; 4],
    /// Sky-based fog zenith color.
    pub fog_background_zenith: [f32; 4],
    /// Sky-based fog horizon color.
    pub fog_background_horizon: [f32; 4],
    /// Sky-based fog nadir color.
    pub fog_background_nadir: [f32; 4],
    /// Sky-based fog general parameters.
    pub fog_background_params: [f32; 4],
    /// Fog sampling parameters.
    pub fog_sampling: [f32; 4],
    /// Scene lights (up to 4 supported).
    pub lights: [Light; 4],
    /// Light view-projection matrix for shadow mapping.
    pub light_view_proj: [[f32; 4]; 4],
}

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
    pub(crate) parts: Arc<Vec<ModelPart>>,
}

impl Model {
    /// Creates an empty model with no parts.
    pub fn empty(_ctx: &mut crate::Context) -> Self {
        Self {
            parts: Arc::new(Vec::new()),
        }
    }
    pub fn first_id(&self) -> u32 {
        self.parts.first().map(|p| p.id).unwrap_or(0)
    }

    /// Creates a new model from vertex and index data.
    pub(crate) fn new(
        ctx: &mut crate::Context,
        vertices: &[Vertex],
        indices: &[u32],
    ) -> anyhow::Result<Self> {
        let mesh_id = ctx.register_mesh(vertices, indices);
        Ok(Self {
            parts: Arc::new(vec![ModelPart {
                id: mesh_id,
                material: Material::default(),
            }]),
        })
    }

    /// Sets the albedo material (texture) for this model.
    pub fn with_material(mut self, image: crate::Image) -> Self {
        for part in Arc::make_mut(&mut self.parts).iter_mut() {
            part.material.albedo = Some(image.id());
        }
        self
    }

    pub fn with_albedo(mut self, image: crate::Image) -> Self {
        for part in Arc::make_mut(&mut self.parts).iter_mut() {
            part.material.albedo = Some(image.id());
        }
        self
    }

    pub fn with_normal_map(mut self, image: crate::Image) -> Self {
        for part in Arc::make_mut(&mut self.parts).iter_mut() {
            part.material.normal = Some(image.id());
        }
        self
    }

    pub fn with_pbr_map(mut self, image: crate::Image) -> Self {
        for part in Arc::make_mut(&mut self.parts).iter_mut() {
            part.material.pbr = Some(image.id());
        }
        self
    }

    pub fn with_ao_map(mut self, image: crate::Image) -> Self {
        for part in Arc::make_mut(&mut self.parts).iter_mut() {
            part.material.occlusion = Some(image.id());
        }
        self
    }

    pub fn with_emissive_map(mut self, image: crate::Image) -> Self {
        for part in Arc::make_mut(&mut self.parts).iter_mut() {
            part.material.emissive = Some(image.id());
        }
        self
    }

    pub fn with_part_material(mut self, index: usize, material: Material) -> Self {
        if let Some(part) = Arc::make_mut(&mut self.parts).get_mut(index) {
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
        Arc::make_mut(&mut self.parts).push(ModelPart {
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
    pub(crate) fn cube(ctx: &mut crate::Context, size: f32) -> anyhow::Result<Self> {
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
                ..Default::default()
            },
            Vertex {
                pos: [s, -s, s],
                uv: [1.0, 1.0],
                normal: [0.0, 0.0, 1.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [s, s, s],
                uv: [1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [-s, s, s],
                uv: [0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [-s, -s, -s],
                uv: [1.0, 1.0],
                normal: [0.0, 0.0, -1.0],
                tangent: [-1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [-s, s, -s],
                uv: [1.0, 0.0],
                normal: [0.0, 0.0, -1.0],
                tangent: [-1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [s, s, -s],
                uv: [0.0, 0.0],
                normal: [0.0, 0.0, -1.0],
                tangent: [-1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [s, -s, -s],
                uv: [0.0, 1.0],
                normal: [0.0, 0.0, -1.0],
                tangent: [-1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [-s, s, -s],
                uv: [0.0, 0.0],
                normal: [0.0, 1.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [-s, s, s],
                uv: [0.0, 1.0],
                normal: [0.0, 1.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [s, s, s],
                uv: [1.0, 1.0],
                normal: [0.0, 1.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [s, s, -s],
                uv: [1.0, 0.0],
                normal: [0.0, 1.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [-s, -s, -s],
                uv: [1.0, 1.0],
                normal: [0.0, -1.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [s, -s, -s],
                uv: [0.0, 1.0],
                normal: [0.0, -1.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [s, -s, s],
                uv: [0.0, 0.0],
                normal: [0.0, -1.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [-s, -s, s],
                uv: [1.0, 0.0],
                normal: [0.0, -1.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [s, -s, -s],
                uv: [1.0, 1.0],
                normal: [1.0, 0.0, 0.0],
                tangent: [0.0, 0.0, -1.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [s, s, -s],
                uv: [1.0, 0.0],
                normal: [1.0, 0.0, 0.0],
                tangent: [0.0, 0.0, -1.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [s, s, s],
                uv: [0.0, 0.0],
                normal: [1.0, 0.0, 0.0],
                tangent: [0.0, 0.0, -1.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [s, -s, s],
                uv: [0.0, 1.0],
                normal: [1.0, 0.0, 0.0],
                tangent: [0.0, 0.0, -1.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [-s, -s, -s],
                uv: [0.0, 1.0],
                normal: [-1.0, 0.0, 0.0],
                tangent: [0.0, 0.0, 1.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [-s, -s, s],
                uv: [1.0, 1.0],
                normal: [-1.0, 0.0, 0.0],
                tangent: [0.0, 0.0, 1.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [-s, s, s],
                uv: [1.0, 0.0],
                normal: [-1.0, 0.0, 0.0],
                tangent: [0.0, 0.0, 1.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [-s, s, -s],
                uv: [0.0, 0.0],
                normal: [-1.0, 0.0, 0.0],
                tangent: [0.0, 0.0, 1.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
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
    pub(crate) fn plane(ctx: &mut crate::Context, width: f32, height: f32) -> anyhow::Result<Self> {
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
                ..Default::default()
            },
            Vertex {
                pos: [hw, -hh, 0.0],
                uv: [1.0, 1.0],
                normal: [0.0, 0.0, 1.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [hw, hh, 0.0],
                uv: [1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
            Vertex {
                pos: [-hw, hh, 0.0],
                uv: [0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                tangent: [1.0, 0.0, 0.0],
                joint_indices: [0; 4],
                joint_weights: [0.0; 4],
                ..Default::default()
            },
        ];

        let indices = vec![0, 1, 2, 0, 2, 3];

        Self::new(ctx, &vertices, &indices)
    }

    /// Creates a UV sphere model with the specified radius.
    pub(crate) fn sphere(ctx: &mut crate::Context, radius: f32) -> anyhow::Result<Self> {
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
                    ..Default::default()
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

    /// Renders the model using skeletal animation.
    pub fn draw_skinned(
        &self,
        ctx: &mut crate::Context,
        target: crate::Image,
        options: crate::DrawOption3D,
        skin_id: u32,
    ) {
        let target_texture_id = ctx.resolve_target_texture_id(target);
        ctx.push_3d(crate::drawable::DrawCommand3D::Model(
            target_texture_id,
            self.clone(),
            options,
            0,
            crate::ShaderOpts::default(),
            Some(skin_id),
        ));
    }

    pub(crate) fn draw_with_shader(
        &self,
        ctx: &mut crate::Context,
        target: crate::Image,
        shader_id: u32,
        options: crate::DrawOption3D,
        shader_opts: crate::ShaderOpts,
        skin_id: Option<u32>,
    ) {
        let target_texture_id = ctx.resolve_target_texture_id(target);
        ctx.push_3d(crate::drawable::DrawCommand3D::Model(
            target_texture_id,
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
    ///
    /// The `target` image is used as the render target (e.g., `screen`).
    pub(crate) fn draw_instanced(
        &self,
        ctx: &mut crate::Context,
        target: crate::Image,
        options: crate::DrawOption3D,
        transforms: &[[[f32; 4]; 4]],
    ) {
        if transforms.is_empty() {
            return;
        }
        self.draw_instanced_shared(ctx, target, options, std::sync::Arc::from(transforms));
    }

    /// Renders instances using a caller-owned transform buffer without making an extra copy.
    pub fn draw_instanced_owned(
        &self,
        ctx: &mut crate::Context,
        target: crate::Image,
        options: crate::DrawOption3D,
        transforms: Vec<[[f32; 4]; 4]>,
    ) {
        if transforms.is_empty() {
            return;
        }
        self.draw_instanced_shared(ctx, target, options, std::sync::Arc::from(transforms));
    }

    /// Renders instances backed by shared transform data.
    pub(crate) fn draw_instanced_shared(
        &self,
        ctx: &mut crate::Context,
        target: crate::Image,
        options: crate::DrawOption3D,
        transforms: std::sync::Arc<[[[f32; 4]; 4]]>,
    ) {
        if transforms.is_empty() {
            return;
        }
        let target_texture_id = ctx.resolve_target_texture_id(target);
        ctx.push_3d(crate::drawable::DrawCommand3D::ModelInstanced(
            target_texture_id,
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
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, PartialEq, Default)]
pub struct Vertex {
    pub pos: [f32; 3],          // 0..12
    pub _pad1: f32,             // 12..16
    pub uv: [f32; 2],           // 16..24
    pub _pad2: [f32; 2],        // 24..32
    pub normal: [f32; 3],       // 32..44
    pub _pad3: f32,             // 44..48
    pub tangent: [f32; 3],      // 48..60
    pub _pad4: f32,             // 60..64
    pub joint_indices: [u32; 4], // 64..80
    pub joint_weights: [f32; 4], // 80..96
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
                    offset: 16,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 48,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 64,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Uint32x4,
                },
                wgpu::VertexAttribute {
                    offset: 80,
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

/// Creates a model from vertex/index data.
pub fn create(
    ctx: &mut crate::Context,
    vertices: &[Vertex],
    indices: &[u32],
) -> anyhow::Result<Model> {
    Model::new(ctx, vertices, indices)
}

/// Creates a cube model.
pub fn create_cube(ctx: &mut crate::Context, size: f32) -> anyhow::Result<Model> {
    Model::cube(ctx, size)
}

/// Creates a plane model.
pub fn create_plane(ctx: &mut crate::Context, width: f32, height: f32) -> anyhow::Result<Model> {
    Model::plane(ctx, width, height)
}

/// Creates a sphere model.
pub fn create_sphere(ctx: &mut crate::Context, radius: f32) -> anyhow::Result<Model> {
    Model::sphere(ctx, radius)
}

/// Creates an empty model.
pub fn create_empty(ctx: &mut crate::Context) -> Model {
    Model::empty(ctx)
}

/// Draws a model with a custom shader into a specific target.
pub fn draw_with_shader(
    ctx: &mut crate::Context,
    target: crate::Image,
    model: &Model,
    shader_id: u32,
    options: crate::DrawOption3D,
    shader_opts: crate::ShaderOpts,
    skin_id: Option<u32>,
) {
    model.draw_with_shader(ctx, target, shader_id, options, shader_opts, skin_id);
}

/// Draws instanced models from borrowed transform data.
pub fn draw_instanced(
    ctx: &mut crate::Context,
    target: crate::Image,
    model: &Model,
    options: crate::DrawOption3D,
    transforms: &[[[f32; 4]; 4]],
) {
    model.draw_instanced(ctx, target, options, transforms);
}

/// Draws instanced models from Arc-backed shared transform data.
pub fn draw_instanced_shared(
    ctx: &mut crate::Context,
    target: crate::Image,
    model: &Model,
    options: crate::DrawOption3D,
    transforms: std::sync::Arc<[[[f32; 4]; 4]]>,
) {
    model.draw_instanced_shared(ctx, target, options, transforms);
}

impl crate::Drawable for &Model {
    type Options = crate::DrawOption3D;

    fn draw_to(self, ctx: &mut crate::Context, target: crate::Image, options: Self::Options) {
        let target_texture_id = ctx.resolve_target_texture_id(target);
        ctx.push_3d(crate::drawable::DrawCommand3D::Model(
            target_texture_id,
            self.clone(),
            options,
            0,
            crate::ShaderOpts::default(),
            None,
        ));
    }
}
