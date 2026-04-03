use crate::model::Vertex;
use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct MaterialBindGroupKey {
    pub atlas_indices: [u32; 5],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct ModelGlobals {
    pub mvp: [[f32; 4]; 4],
    pub model: [[f32; 4]; 4],
    pub extra: [f32; 4], // [opacity, 0, 0, 0]
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
    pub camera_right: [f32; 4],
    pub camera_up: [f32; 4],
    pub camera_forward: [f32; 4],
    pub projection_params: [f32; 4], // [proj_x, proj_y, znear, zfar]
    pub ambient_color: [f32; 4],
    pub fog_color: [f32; 4],
    pub fog_distance: [f32; 4], // [start, end, exponent, density]
    pub fog_height: [f32; 4],   // [base, falloff, exponent, density]
    pub fog_params: [f32; 4],   // [strength, reserved, reserved, reserved]
    pub fog_background_zenith: [f32; 4], // [rgb, mix]
    pub fog_background_horizon: [f32; 4], // [rgb, mix]
    pub fog_background_nadir: [f32; 4], // [rgb, mix]
    pub fog_background_params: [f32; 4], // [horizon_glow, sky_fog_blend, geometry_fog_blend, reserved]
    pub fog_sampling: [f32; 4], // [min_height_samples, max_height_samples, height_sample_scale, reserved]
    pub lights: [Light; 4],
    pub light_view_proj: [[f32; 4]; 4],
}

pub struct ModelRenderer {
    pub(crate) globals_bind_group_layout: wgpu::BindGroupLayout, // Group 0: [Model, Scene, UserOpts]
    pub(crate) texture_bind_group_layout: wgpu::BindGroupLayout, // Group 1: Material textures
    pub(crate) bone_matrices_bind_group_layout: wgpu::BindGroupLayout, // Group 2: Skinning
    pub(crate) environment_bind_group_layout: wgpu::BindGroupLayout, // Group 3: Shadow + IBL

    pub(crate) model_globals_buffer: wgpu::Buffer,
    pub(crate) user_shader_opts_buffer: wgpu::Buffer,
    pub(crate) bone_matrices_buffer: wgpu::Buffer,
    pub(crate) instance_buffer: wgpu::Buffer,
    pub(crate) scene_globals_buffer: wgpu::Buffer,

    pub(crate) globals_bind_group: wgpu::BindGroup,
    pub(crate) bone_matrices_bind_group: wgpu::BindGroup,

    pub(crate) sampler: wgpu::Sampler,
    pub(crate) shadow_sampler: wgpu::Sampler,
    pub(crate) ibl_sampler: wgpu::Sampler,

    pub(crate) model_globals_stride: u32,
    pub(crate) user_opts_stride: u32,
    pub(crate) bone_matrices_stride: u32,
    pub(crate) next_model_globals: u32,
    pub(crate) next_user_shader_opts: u32,
    pub(crate) next_bone_batch: u32,
    pub(crate) max_model_globals: u32,
    pub(crate) max_user_shader_opts: u32,
    pub(crate) max_instances: u32,

    pub(crate) meshes: HashMap<u32, MeshData>,
    pub(crate) skins: HashMap<u32, crate::graphics::core::SkinData>,
    pub(crate) texture_bind_groups: HashMap<MaterialBindGroupKey, wgpu::BindGroup>,
    pub(crate) skin_bone_offsets: HashMap<u32, u32>,
}

impl ModelRenderer {
    pub const GLOBALS_SIZE_BYTES: usize = std::mem::size_of::<ModelGlobals>();
    pub const USER_SHADER_OPTS_SIZE: usize = 256;

    pub fn new(device: &wgpu::Device) -> Self {
        let align = device.limits().min_uniform_buffer_offset_alignment;
        let model_globals_stride = (Self::GLOBALS_SIZE_BYTES as u32).div_ceil(align) * align;
        let user_opts_stride = (Self::USER_SHADER_OPTS_SIZE as u32).div_ceil(align) * align;
        let max_model_globals = 4096u32;
        let max_user_shader_opts = 4096u32;
        let max_instances = 65536u32;

        let model_globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model_globals_ubo"),
            size: (model_globals_stride * max_model_globals) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let user_shader_opts_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model_user_shader_opts_ubo"),
            size: (user_opts_stride * max_user_shader_opts) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let scene_globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model_scene_globals_ubo"),
            size: std::mem::size_of::<SceneGlobals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model_instance_buffer"),
            size: (max_instances as usize * std::mem::size_of::<[[f32; 4]; 4]>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Group 0 Layout
        let globals_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("model_globals_bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        // 0: Model Globals (Dynamic)
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: std::num::NonZeroU64::new(
                                Self::GLOBALS_SIZE_BYTES as u64,
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        // 1: Scene Globals (Static)
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<
                                SceneGlobals,
                            >(
                            )
                                as u64),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        // 2: User Ops (Dynamic)
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: std::num::NonZeroU64::new(
                                Self::USER_SHADER_OPTS_SIZE as u64,
                            ),
                        },
                        count: None,
                    },
                ],
            });

        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model_globals_bg"),
            layout: &globals_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &model_globals_buffer,
                        offset: 0,
                        size: std::num::NonZeroU64::new(Self::GLOBALS_SIZE_BYTES as u64),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &scene_globals_buffer,
                        offset: 0,
                        size: std::num::NonZeroU64::new(std::mem::size_of::<SceneGlobals>() as u64),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &user_shader_opts_buffer,
                        offset: 0,
                        size: std::num::NonZeroU64::new(Self::USER_SHADER_OPTS_SIZE as u64),
                    }),
                },
            ],
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("model_texture_bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        // 0: Albedo
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        // 1: Sampler
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        // 2: PBR
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        // 3: Normal
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        // 4: AO
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        // 5: Emissive
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

        let bone_matrices_stride = 256 * 64;
        let bone_matrices_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("model_bone_matrices_bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: std::num::NonZeroU64::new(bone_matrices_stride as u64),
                    },
                    count: None,
                }],
            });

        let bone_matrices_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model_bone_matrices_uniform"),
            size: (bone_matrices_stride as u64 * max_model_globals as u64),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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

        let environment_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("model_env_bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        // 0: Shadow Map
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
                        // 1: Shadow Sampler
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        // 2: Irradiance
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        // 3: Prefiltered
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        // 4: BRDF LUT
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        // 5: IBL Sampler
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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
            ..Default::default()
        });

        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow_sampler"),
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let ibl_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ibl_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        Self {
            globals_bind_group_layout,
            texture_bind_group_layout,
            bone_matrices_bind_group_layout,
            environment_bind_group_layout,
            model_globals_buffer,
            user_shader_opts_buffer,
            bone_matrices_buffer,
            instance_buffer,
            scene_globals_buffer,

            globals_bind_group,
            bone_matrices_bind_group,
            sampler,
            shadow_sampler,
            ibl_sampler,
            model_globals_stride,
            user_opts_stride,
            bone_matrices_stride: bone_matrices_stride as u32,
            next_model_globals: 0,
            next_user_shader_opts: 0,
            next_bone_batch: 0,
            max_model_globals,
            max_user_shader_opts,
            max_instances,
            meshes: HashMap::new(),
            skins: HashMap::new(),
            texture_bind_groups: HashMap::new(),
            skin_bone_offsets: HashMap::new(),
        }
    }

    pub fn create_environment_bind_group(
        &self,
        device: &wgpu::Device,
        shadow_view: &wgpu::TextureView,
        irradiance_view: &wgpu::TextureView,
        prefiltered_view: &wgpu::TextureView,
        brdf_lut_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model_env_bg"),
            layout: &self.environment_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.shadow_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(irradiance_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(prefiltered_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(brdf_lut_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&self.ibl_sampler),
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

    pub fn texture_bind_group_for_atlases(
        &mut self,
        device: &wgpu::Device,
        key: MaterialBindGroupKey,
        views: [&wgpu::TextureView; 5],
    ) -> &wgpu::BindGroup {
        if !self.texture_bind_groups.contains_key(&key) {
            let bind_group = self.create_texture_bind_group(
                device, views[0], views[1], views[2], views[3], views[4],
            );
            self.texture_bind_groups.insert(key, bind_group);
        }

        self.texture_bind_groups
            .get(&key)
            .expect("texture bind group should be cached")
    }

    pub fn clear_texture_bind_group_cache(&mut self) {
        self.texture_bind_groups.clear();
    }

    pub fn begin_frame(&mut self) {
        self.next_model_globals = 0;
        self.next_user_shader_opts = 0;
        self.next_bone_batch = 0;
        self.skin_bone_offsets.clear();
    }

    pub fn bone_offset_for_skin(
        &mut self,
        queue: &wgpu::Queue,
        skin_id: u32,
        matrices: &[[[f32; 4]; 4]],
    ) -> anyhow::Result<u32> {
        if let Some(offset) = self.skin_bone_offsets.get(&skin_id) {
            return Ok(*offset);
        }

        let offset = self.upload_bone_matrices(queue, matrices)?;
        self.skin_bone_offsets.insert(skin_id, offset);
        Ok(offset)
    }

    pub fn upload_globals(
        &mut self,
        queue: &wgpu::Queue,
        globals: &ModelGlobals,
    ) -> anyhow::Result<u32> {
        if self.next_model_globals >= self.max_model_globals {
            return Err(anyhow::anyhow!("max model globals exceeded"));
        }
        let offset = self.next_model_globals * self.model_globals_stride;
        queue.write_buffer(
            &self.model_globals_buffer,
            offset as wgpu::BufferAddress,
            bytemuck::bytes_of(globals),
        );

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
        if self.next_user_shader_opts >= self.max_user_shader_opts {
            return Err(anyhow::anyhow!("max model shader opts exceeded"));
        }
        let slot = self.next_user_shader_opts;
        self.next_user_shader_opts += 1;
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

    pub fn upload_instances(
        &self,
        queue: &wgpu::Queue,
        instances: &[[[f32; 4]; 4]],
    ) -> anyhow::Result<()> {
        if instances.len() > self.max_instances as usize {
            return Err(anyhow::anyhow!(
                "too many instances! max is {}",
                self.max_instances
            ));
        }
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instances));
        Ok(())
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
            size: std::mem::size_of_val(vertices) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_index_buffer"),
            size: std::mem::size_of_val(indices) as u64,
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
    for i in 0..4 {
        // column
        for j in 0..4 {
            // row
            for k in 0..4 {
                // mid
                result[i][j] += a[k][j] * b[i][k];
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_point(matrix: [[f32; 4]; 4], point: [f32; 3]) -> [f32; 4] {
        let v = [point[0], point[1], point[2], 1.0];
        let mut out = [0.0; 4];
        for j in 0..4 {
            for i in 0..4 {
                out[j] += matrix[i][j] * v[i];
            }
        }
        out
    }

    #[test]
    fn test_projection_aspect_ratio() {
        let aspect = 2.0;
        let fovy = std::f32::consts::PI / 2.0; // 90 deg
        let proj = create_perspective(aspect, fovy, 0.1, 100.0);

        // f = 1 / tan(45 deg) = 1.0
        // x_scale = f / aspect = 0.5
        // y_scale = f = 1.0

        assert!((proj[0][0] - 0.5).abs() < 1e-6);
        assert!((proj[1][1] - 1.0).abs() < 1e-6);

        // Test mapping of a symmetric point e.g. [1, 1, -10]
        // Projected X should be half of Projected Y in NDC because aspect is 2.0 (wider)
        let v = [1.0, 1.0, -10.0, 1.0];
        let mut res = [0.0; 4];
        for j in 0..4 {
            for i in 0..4 {
                res[j] += proj[i][j] * v[i];
            }
        }
        let ndc_x = res[0] / res[3];
        let ndc_y = res[1] / res[3];

        assert!((ndc_x * 2.0 - ndc_y).abs() < 1e-6);
    }

    #[test]
    fn test_square_stays_square_on_wide_viewport() {
        let width = 800.0;
        let height = 600.0;
        let proj = create_perspective(width / height, std::f32::consts::PI / 4.0, 0.1, 1000.0);

        let corners = [
            [-1.0, -1.0, -5.0],
            [1.0, -1.0, -5.0],
            [1.0, 1.0, -5.0],
            [-1.0, 1.0, -5.0],
        ];

        let mut screen = [[0.0; 2]; 4];
        for (index, corner) in corners.iter().enumerate() {
            let projected = project_point(proj, *corner);
            let ndc_x = projected[0] / projected[3];
            let ndc_y = projected[1] / projected[3];
            screen[index] = [
                (ndc_x * 0.5 + 0.5) * width,
                (1.0 - (ndc_y * 0.5 + 0.5)) * height,
            ];
        }

        let screen_width = screen[1][0] - screen[0][0];
        let screen_height = screen[0][1] - screen[3][1];

        assert!(
            (screen_width - screen_height).abs() < 1e-4,
            "expected projected square to stay square, got width={} height={}",
            screen_width,
            screen_height
        );
    }
}
