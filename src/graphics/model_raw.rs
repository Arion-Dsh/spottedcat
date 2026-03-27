use bytemuck::{Pod, Zeroable};
use crate::model::Vertex;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct ModelGlobals {
    pub mvp: [[f32; 4]; 4],
    pub model: [[f32; 4]; 4],
    pub extra: [f32; 4],   // [opacity, 0, 0, 0]
    pub albedo_uv: [f32; 4],
    pub pbr_uv: [f32; 4],
    pub normal_uv: [f32; 4],
    pub ao_uv: [f32; 4],
    pub emissive_uv: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct Light {
    pub position: [f32; 4], // [x, y, z, 1.0 = point, 0.0 = directional]
    pub color: [f32; 4],    // [r, g, b, intensity]
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct SceneGlobals {
    pub camera_pos: [f32; 4],
    pub ambient_color: [f32; 4],
    pub lights: [Light; 4],
    pub light_view_proj: [[f32; 4]; 4],
}

pub struct ModelRenderer {
    pub(crate) user_globals_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) texture_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) user_shader_opts_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) bone_matrices_bind_group_layout: wgpu::BindGroupLayout, // Group 3
    pub(crate) scene_globals_bind_group_layout: wgpu::BindGroupLayout, // Group 4
    pub(crate) shadow_bind_group_layout: wgpu::BindGroupLayout,        // Group 5
    pub(crate) ibl_bind_group_layout: wgpu::BindGroupLayout,           // Group 6
    pub(crate) model_globals_buffer: wgpu::Buffer,
    pub(crate) model_globals_bind_group: wgpu::BindGroup,
    pub(crate) user_shader_opts_buffer: wgpu::Buffer,
    pub(crate) user_shader_opts_bind_group: wgpu::BindGroup,
    pub(crate) bone_matrices_buffer: wgpu::Buffer,
    pub(crate) bone_matrices_bind_group: wgpu::BindGroup,
    pub(crate) scene_globals_buffer: wgpu::Buffer,
    pub(crate) scene_globals_bind_group: wgpu::BindGroup,
    pub(crate) sampler: wgpu::Sampler,
    pub(crate) shadow_sampler: wgpu::Sampler,
    pub(crate) ibl_sampler: wgpu::Sampler,
    model_globals_stride: u32,
    user_opts_stride: u32,
    bone_matrices_stride: u32,
    next_model_globals: u32,
    next_bone_batch: u32,
    max_model_globals: u32,
}

impl ModelRenderer {
    pub const GLOBALS_SIZE_BYTES: usize = std::mem::size_of::<ModelGlobals>();
    pub const USER_SHADER_OPTS_SIZE: usize = 256; // Matching ImageRenderer/ShaderOpts

    pub fn new(device: &wgpu::Device) -> Self {
        let model_globals_stride = {
            let align = device.limits().min_uniform_buffer_offset_alignment;
            let size = Self::GLOBALS_SIZE_BYTES as u32;
            ((size + align - 1) / align) * align
        };
        let max_model_globals = 4096u32;
        let buffer_size = model_globals_stride as wgpu::BufferAddress * max_model_globals as wgpu::BufferAddress;

        let model_globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model_globals_ubo"),
            size: buffer_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let user_globals_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model_globals_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: std::num::NonZeroU64::new(Self::GLOBALS_SIZE_BYTES as u64),
                },
                count: None,
            }],
        });

        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model_texture_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { // 0: Albedo
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { // 1: Sampler
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { // 2: PBR (Metallic/Roughness)
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { // 3: Normal
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { // 4: AO
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { // 5: Emissive
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("model_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let ibl_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ibl_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        let model_globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model_globals_bg"),
            layout: &user_globals_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &model_globals_buffer,
                    offset: 0,
                    size: std::num::NonZeroU64::new(Self::GLOBALS_SIZE_BYTES as u64),
                }),
            }],
        });

        let user_shader_opts_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("model_user_shader_opts_bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: std::num::NonZeroU64::new(Self::USER_SHADER_OPTS_SIZE as u64),
                    },
                    count: None,
                }],
            });

        let user_opts_stride = {
            let align = device.limits().min_uniform_buffer_offset_alignment;
            let size = Self::USER_SHADER_OPTS_SIZE as u32;
            ((size + align - 1) / align) * align
        };

        let user_shader_opts_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model_user_shader_opts_ubo"),
            size: (user_opts_stride * max_model_globals) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let user_shader_opts_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model_user_shader_opts_bg"),
            layout: &user_shader_opts_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &user_shader_opts_buffer,
                    offset: 0,
                    size: std::num::NonZeroU64::new(Self::USER_SHADER_OPTS_SIZE as u64),
                }),
            }],
        });

        let bone_matrices_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("model_bone_matrices_bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: true,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let bone_matrices_stride = 256 * 64; // Support up to 256 bones per model, mat4 is 64 bytes
        let bone_matrices_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model_bone_matrices_storage"),
            size: (bone_matrices_stride as u64 * max_model_globals as u64),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bone_matrices_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model_bone_matrices_bg"),
            layout: &bone_matrices_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &bone_matrices_buffer,
                    offset: 0,
                    size: std::num::NonZeroU64::new(bone_matrices_stride as u64),
                }),
            }],
        });

        let scene_globals_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model_scene_globals_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<SceneGlobals>() as u64),
                },
                count: None,
            }],
        });

        let scene_globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model_scene_globals_ubo"),
            size: std::mem::size_of::<SceneGlobals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let scene_globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model_scene_globals_bg"),
            layout: &scene_globals_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &scene_globals_buffer,
                    offset: 0,
                    size: std::num::NonZeroU64::new(std::mem::size_of::<SceneGlobals>() as u64),
                }),
            }],
        });

        let shadow_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model_shadow_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });

        let ibl_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model_ibl_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        Self {
            user_globals_bind_group_layout,
            texture_bind_group_layout,
            user_shader_opts_bind_group_layout,
            bone_matrices_bind_group_layout,
            scene_globals_bind_group_layout,
            shadow_bind_group_layout,
            ibl_bind_group_layout,
            model_globals_buffer,
            model_globals_bind_group,
            user_shader_opts_buffer,
            user_shader_opts_bind_group,
            bone_matrices_buffer,
            bone_matrices_bind_group,
            scene_globals_buffer,
            scene_globals_bind_group,
            sampler,
            shadow_sampler,
            ibl_sampler,
            model_globals_stride,
            user_opts_stride,
            bone_matrices_stride: bone_matrices_stride as u32,
            next_model_globals: 0,
            next_bone_batch: 0,
            max_model_globals,
        }
    }

    pub fn create_ibl_bind_group(
        &self,
        device: &wgpu::Device,
        irradiance_view: &wgpu::TextureView,
        prefiltered_view: &wgpu::TextureView,
        brdf_lut_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model_ibl_bg"),
            layout: &self.ibl_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(irradiance_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(prefiltered_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(brdf_lut_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.ibl_sampler),
                },
            ],
        })
    }

    pub fn create_shadow_bind_group(
        &self,
        device: &wgpu::Device,
        shadow_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model_shadow_bg"),
            layout: &self.shadow_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.shadow_sampler),
                },
            ],
        })
    }

    pub fn create_texture_bind_group(
        &self,
        device: &wgpu::Device,
        albedo: &wgpu::TextureView,
        pbr: &wgpu::TextureView,
        normal: &wgpu::TextureView,
        ao: &wgpu::TextureView,
        emissive: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model_texture_bg"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(albedo),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(pbr),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(normal),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(ao),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(emissive),
                },
            ],
        })
    }

    pub fn begin_frame(&mut self) {
        self.next_model_globals = 0;
        self.next_bone_batch = 0;
    }

    pub fn upload_globals(&mut self, queue: &wgpu::Queue, globals: &ModelGlobals) -> anyhow::Result<u32> {
        if self.next_model_globals >= self.max_model_globals {
            return Err(anyhow::anyhow!("max model globals exceeded"));
        }
        let offset = self.next_model_globals * self.model_globals_stride;
        queue.write_buffer(&self.model_globals_buffer, offset as wgpu::BufferAddress, bytemuck::bytes_of(globals));
        
        let dyn_offset = offset;
        self.next_model_globals += 1;
        Ok(dyn_offset)
    }

    pub fn upload_shader_opts_bytes(
        &mut self,
        queue: &wgpu::Queue,
        bytes: &[u8],
    ) -> anyhow::Result<u32> {
        if bytes.len() != Self::USER_SHADER_OPTS_SIZE {
            return Err(anyhow::anyhow!(
                "model shader opts must be exactly {} bytes",
                Self::USER_SHADER_OPTS_SIZE
            ));
        }
        let slot = self.next_model_globals - 1; 
        let offset = slot as wgpu::BufferAddress * self.user_opts_stride as wgpu::BufferAddress;
        queue.write_buffer(&self.user_shader_opts_buffer, offset, bytes);
        Ok(offset as u32)
    }

    pub fn upload_bone_matrices(
        &mut self,
        queue: &wgpu::Queue,
        matrices: &[[[f32; 4]; 4]],
    ) -> anyhow::Result<u32> {
        if matrices.is_empty() {
             return Ok(0);
        }
        if self.next_bone_batch >= self.max_model_globals {
            return Err(anyhow::anyhow!("max model bone batches exceeded"));
        }
        let slot = self.next_bone_batch;
        self.next_bone_batch += 1;
        let offset = slot as wgpu::BufferAddress * self.bone_matrices_stride as wgpu::BufferAddress;
        
        let bytes = bytemuck::cast_slice(matrices);
        if bytes.len() > self.bone_matrices_stride as usize {
             return Err(anyhow::anyhow!("too many bones! max is 256"));
        }
        
        queue.write_buffer(&self.bone_matrices_buffer, offset, bytes);
        Ok(offset as u32)
    }

    pub fn upload_scene_globals(&self, queue: &wgpu::Queue, scene: &SceneGlobals) {
        queue.write_buffer(&self.scene_globals_buffer, 0, bytemuck::bytes_of(scene));
    }
}

pub struct MeshData {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

impl MeshData {
    pub fn new(device: &wgpu::Device, vertices: &[Vertex], indices: &[u32]) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_vertex_buffer"),
            size: (vertices.len() * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_index_buffer"),
            size: (indices.len() * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }

    pub fn upload(&self, queue: &wgpu::Queue, vertices: &[Vertex], indices: &[u32]) {
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(vertices));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(indices));
    }
}

pub fn create_perspective(aspect: f32, fov_y: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (fov_y / 2.0).tan();
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), -1.0],
        [0.0, 0.0, (far * near) / (near - far), 0.0],
    ]
}

pub fn create_scale(scale: [f32; 3]) -> [[f32; 4]; 4] {
    [
        [scale[0], 0.0, 0.0, 0.0],
        [0.0, scale[1], 0.0, 0.0],
        [0.0, 0.0, scale[2], 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

pub fn create_translation(pos: [f32; 3]) -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [pos[0], pos[1], pos[2], 1.0],
    ]
}

pub fn create_rotation(rot: [f32; 3]) -> [[f32; 4]; 4] {
    let (cx, sx) = (rot[0].cos(), rot[0].sin());
    let (cy, sy) = (rot[1].cos(), rot[1].sin());
    let (cz, sz) = (rot[2].cos(), rot[2].sin());

    let rx = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, cx, sx, 0.0],
        [0.0, -sx, cx, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    let ry = [
        [cy, 0.0, -sy, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [sy, 0.0, cy, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    let rz = [
        [cz, sz, 0.0, 0.0],
        [-sz, cz, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    multiply(multiply(rx, ry), rz)
}

pub fn multiply(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut result = [[0.0; 4]; 4];
    for i in 0..4 { // column
        for j in 0..4 { // row
            for k in 0..4 { // mid
                result[i][j] += a[k][j] * b[i][k];
            }
        }
    }
    result
}
