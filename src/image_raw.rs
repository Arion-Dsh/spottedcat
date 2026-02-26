use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct ImageTransform {
    pub mvp: [[f32; 4]; 4],
    pub uvp: [[f32; 4]; 4],
    pub color: [f32; 4],
}

impl Default for ImageTransform {
    fn default() -> Self {
        Self {
            mvp: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            uvp: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct InstanceData {
    pub pos: [f32; 2],
    pub rotation: f32,
    pub size: [f32; 2],
    pub uv_rect: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct EngineGlobals {
    // screen[0].xy = [2.0/logical_w, 2.0/logical_h] (sw_inv_2, sh_inv_2)
    // screen[0].zw = [1.0/logical_w, 1.0/logical_h] (sw_inv, sh_inv)
    pub screen: [f32; 4],
    pub opacity: f32,
    pub _padding: [f32; 3],
}

impl InstanceData {
    const ATTRS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Float32x2, // pos
        1 => Float32,   // rotation
        2 => Float32x2, // size
        3 => Float32x4, // uv_rect
    ];

    pub(crate) fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceData>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRS,
        }
    }
}

pub struct ImageRenderer {
    pub(crate) sampler: wgpu::Sampler,
    pub(crate) texture_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) user_globals_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) engine_globals_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) user_globals_bind_group: wgpu::BindGroup,
    pub(crate) user_globals_buffer: wgpu::Buffer,
    pub(crate) engine_globals_bind_group: wgpu::BindGroup,
    pub(crate) engine_globals_buffer: wgpu::Buffer,
    globals_stride: u32,
    engine_globals_stride: u32,
    next_user_globals: u32,
    max_user_globals: u32,
    next_engine_globals: u32,
    max_engine_globals: u32,
    pub(crate) instance_buffer: wgpu::Buffer,
    pub(crate) instance_stride: u32,
    next_instance: u32,
    max_instances: u32,
}

impl ImageRenderer {
    pub const GLOBALS_SIZE_BYTES: usize = 256;
    pub const ENGINE_GLOBALS_SIZE_BYTES: usize = std::mem::size_of::<EngineGlobals>();

    pub fn new(
        device: &wgpu::Device,
        _surface_format: wgpu::TextureFormat,
        max_instances: u32,
    ) -> Self {
        let globals_stride = {
            let align = device.limits().min_uniform_buffer_offset_alignment;
            let size = Self::GLOBALS_SIZE_BYTES as u32;
            ((size + align - 1) / align) * align
        };
        let engine_globals_stride = {
            let align = device.limits().min_uniform_buffer_offset_alignment;
            let size = Self::ENGINE_GLOBALS_SIZE_BYTES as u32;
            ((size + align - 1) / align) * align
        };
        let max_user_globals = 4096u32;
        let max_engine_globals = 4096u32;
        let user_globals_buffer_size =
            globals_stride as wgpu::BufferAddress * max_user_globals as wgpu::BufferAddress;
        let engine_globals_buffer_size = engine_globals_stride as wgpu::BufferAddress
            * max_engine_globals as wgpu::BufferAddress;

        let user_globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image_user_globals_ubo"),
            size: user_globals_buffer_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let engine_globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image_engine_globals_ubo"),
            size: engine_globals_buffer_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let instance_stride = std::mem::size_of::<InstanceData>() as u32;
        let instance_buffer_size =
            instance_stride as wgpu::BufferAddress * max_instances as wgpu::BufferAddress;

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image_instance_buffer"),
            size: instance_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("image_texture_bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
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
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("image_sampler"),
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

        let user_globals_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("image_user_globals_bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
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
                }],
            });

        let engine_globals_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("image_engine_globals_bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: std::num::NonZeroU64::new(
                            Self::ENGINE_GLOBALS_SIZE_BYTES as u64,
                        ),
                    },
                    count: None,
                }],
            });

        let user_globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image_user_globals_bg"),
            layout: &user_globals_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &user_globals_buffer,
                    offset: 0,
                    size: std::num::NonZeroU64::new(Self::GLOBALS_SIZE_BYTES as u64),
                }),
            }],
        });

        let engine_globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image_engine_globals_bg"),
            layout: &engine_globals_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &engine_globals_buffer,
                    offset: 0,
                    size: std::num::NonZeroU64::new(Self::ENGINE_GLOBALS_SIZE_BYTES as u64),
                }),
            }],
        });

        Self {
            sampler,
            texture_bind_group_layout,
            user_globals_bind_group_layout,
            engine_globals_bind_group_layout,
            user_globals_bind_group,
            user_globals_buffer,
            engine_globals_bind_group,
            engine_globals_buffer,
            globals_stride,
            engine_globals_stride,
            next_user_globals: 0,
            max_user_globals,
            next_engine_globals: 0,
            max_engine_globals,
            instance_buffer,
            instance_stride,
            next_instance: 0,
            max_instances,
        }
    }

    pub fn upload_user_globals_bytes(
        &mut self,
        queue: &wgpu::Queue,
        bytes: &[u8],
    ) -> anyhow::Result<u32> {
        if bytes.len() != Self::GLOBALS_SIZE_BYTES {
            return Err(anyhow::anyhow!(
                "image user globals must be exactly {} bytes",
                Self::GLOBALS_SIZE_BYTES
            ));
        }
        if self.next_user_globals >= self.max_user_globals {
            return Err(anyhow::anyhow!("max image user globals exceeded"));
        }
        let slot = self.next_user_globals;
        self.next_user_globals = self.next_user_globals.saturating_add(1);
        let offset = slot as wgpu::BufferAddress * self.globals_stride as wgpu::BufferAddress;
        queue.write_buffer(&self.user_globals_buffer, offset, bytes);
        Ok((slot * self.globals_stride) as u32)
    }

    pub fn upload_engine_globals_bytes(
        &mut self,
        queue: &wgpu::Queue,
        bytes: &[u8],
    ) -> anyhow::Result<u32> {
        if bytes.len() != Self::ENGINE_GLOBALS_SIZE_BYTES {
            return Err(anyhow::anyhow!(
                "image engine globals must be exactly {} bytes",
                Self::ENGINE_GLOBALS_SIZE_BYTES
            ));
        }
        if self.next_engine_globals >= self.max_engine_globals {
            return Err(anyhow::anyhow!("max image engine globals exceeded"));
        }

        let offset = self.next_engine_globals * self.engine_globals_stride;
        queue.write_buffer(
            &self.engine_globals_buffer,
            offset as wgpu::BufferAddress,
            bytes,
        );

        let dyn_offset = offset;
        self.next_engine_globals += 1;
        Ok(dyn_offset)
    }

    pub fn upload_engine_globals(
        &mut self,
        queue: &wgpu::Queue,
        globals: &EngineGlobals,
    ) -> anyhow::Result<u32> {
        self.upload_engine_globals_bytes(queue, bytemuck::bytes_of(globals))
    }

    pub fn create_texture_bind_group(
        &self,
        device: &wgpu::Device,
        texture_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image_texture_bg"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    pub fn begin_frame(&mut self) {
        self.next_instance = 0;
        self.next_user_globals = 0;
        self.next_engine_globals = 0;
    }

    pub fn upload_instances(
        &mut self,
        queue: &wgpu::Queue,
        instances: &[InstanceData],
    ) -> anyhow::Result<std::ops::Range<u32>> {
        let count = instances.len() as u32;
        if count == 0 {
            return Ok(0..0);
        }
        if self.next_instance.saturating_add(count) > self.max_instances {
            return Err(anyhow::anyhow!("max image instances exceeded"));
        }

        let start = self.next_instance;
        let offset_bytes =
            start as wgpu::BufferAddress * self.instance_stride as wgpu::BufferAddress;
        queue.write_buffer(
            &self.instance_buffer,
            offset_bytes,
            bytemuck::cast_slice(instances),
        );
        self.next_instance += count;
        Ok(start..(start + count))
    }

    pub fn draw_batch<'rp>(
        &self,
        pass: &mut wgpu::RenderPass<'rp>,
        pipeline: &'rp wgpu::RenderPipeline,
        texture_bind_group: &wgpu::BindGroup,
        instance_range: std::ops::Range<u32>,
        user_globals_offset: u32,
        engine_globals_offset: u32,
    ) {
        if instance_range.start == instance_range.end {
            return;
        }
        pass.set_pipeline(pipeline);
        let start = instance_range.start as wgpu::BufferAddress
            * self.instance_stride as wgpu::BufferAddress;
        let end =
            instance_range.end as wgpu::BufferAddress * self.instance_stride as wgpu::BufferAddress;
        pass.set_vertex_buffer(0, self.instance_buffer.slice(start..end));
        pass.set_bind_group(0, texture_bind_group, &[]);
        pass.set_bind_group(1, &self.user_globals_bind_group, &[user_globals_offset]);
        pass.set_bind_group(2, &self.engine_globals_bind_group, &[engine_globals_offset]);
        let instance_count = instance_range.end - instance_range.start;
        pass.draw(0..4, 0..instance_count);
    }
}
