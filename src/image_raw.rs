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
    // mvp columns 0, 1, 3 (only xy needed for 2D)
    pub mvp_col0: [f32; 2], // c*sx, s*sx
    pub mvp_col1: [f32; 2], // -s*sy, c*sy
    pub mvp_col3: [f32; 2], // dx, dy

    // uvp: u, v, w, h
    pub uv_rect: [f32; 4],
}

impl From<ImageTransform> for InstanceData {
    fn from(t: ImageTransform) -> Self {
        Self {
            mvp_col0: [t.mvp[0][0], t.mvp[0][1]],
            mvp_col1: [t.mvp[1][0], t.mvp[1][1]],
            mvp_col3: [t.mvp[3][0], t.mvp[3][1]],
            uv_rect: [t.uvp[3][0], t.uvp[3][1], t.uvp[0][0], t.uvp[1][1]],
        }
    }
}

impl InstanceData {
    const ATTRS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Float32x2, // mvp_col0
        1 => Float32x2, // mvp_col1
        2 => Float32x2, // mvp_col3
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
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) sampler: wgpu::Sampler,
    pub(crate) texture_bind_group_layout: wgpu::BindGroupLayout,

    pub(crate) globals_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) globals_bind_group: wgpu::BindGroup,
    pub(crate) globals_buffer: wgpu::Buffer,
    pub(crate) globals_stride: u32,
    next_globals: u32,
    max_globals: u32,

    pub(crate) instance_buffer: wgpu::Buffer,
    pub(crate) instance_stride: u32,

    next_instance: u32,
    max_instances: u32,
}

impl ImageRenderer {
    pub const GLOBALS_SIZE_BYTES: usize = 256;

    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat, max_instances: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("image_shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
@group(0) @binding(0)
var tex: texture_2d<f32>;

@group(0) @binding(1)
var samp: sampler;

// 256 bytes = 16 * vec4<f32>. The last vec4's .w is reserved for DrawOption opacity.
@group(1) @binding(0)
var<uniform> globals: array<vec4<f32>, 16>;

struct VsIn {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) mvp_col0: vec2<f32>,
    @location(1) mvp_col1: vec2<f32>,
    @location(2) mvp_col3: vec2<f32>,
    @location(3) uv_rect: vec4<f32>,
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    
    // Triangle Strip Quad: BL, BR, TL, TR
    var pos_arr = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0,  1.0)
    );
    // UVs follow pos: (0,1), (1,1), (0,0), (1,0)
    var uv_arr = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0)
    );
    
    let pos = pos_arr[in.vertex_index];
    let uv = uv_arr[in.vertex_index];

    // Reconstruct position from compressed MVP columns
    // x = pos.x * col0.x + pos.y * col1.x + col3.x
    // y = pos.x * col0.y + pos.y * col1.y + col3.y
    let x = pos.x * in.mvp_col0.x + pos.y * in.mvp_col1.x + in.mvp_col3.x;
    let y = pos.x * in.mvp_col0.y + pos.y * in.mvp_col1.y + in.mvp_col3.y;
    out.clip_pos = vec4<f32>(x, y, 0.0, 1.0);
    
    // UVs: u = u0 + uv.x * w, v = v0 + uv.y * h
    // uv_rect is [u0, v0, w, h]
    out.uv = vec2<f32>(
        in.uv_rect.x + uv.x * in.uv_rect.z,
        in.uv_rect.y + uv.y * in.uv_rect.w
    );
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, in.uv);
    let opacity = clamp(globals[15].w, 0.0, 1.0);
    return vec4<f32>(c.rgb, c.a * opacity);
}
"#
                .into(),
            ),
        });

        Self::new_with_shader(device, surface_format, max_instances, &shader)
    }

    pub fn new_with_shader(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        max_instances: u32,
        shader: &wgpu::ShaderModule,
    ) -> Self {
        let globals_stride = {
            let align = device.limits().min_uniform_buffer_offset_alignment;
            let size = Self::GLOBALS_SIZE_BYTES as u32;
            ((size + align - 1) / align) * align
        };
        let max_globals = 4096u32;
        let instance_stride = std::mem::size_of::<InstanceData>() as u32;
        let instance_buffer_size =
            instance_stride as wgpu::BufferAddress * max_instances as wgpu::BufferAddress;

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image_instance_buffer"),
            size: instance_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            ..Default::default()
        });

        let globals_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("image_globals_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
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
            ],
        });

        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image_globals_ubo"),
            size: globals_stride as wgpu::BufferAddress * max_globals as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image_globals_bg"),
            layout: &globals_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &globals_buffer,
                        offset: 0,
                        size: std::num::NonZeroU64::new(Self::GLOBALS_SIZE_BYTES as u64),
                    }),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("image_pipeline_layout"),
            bind_group_layouts: &[&texture_bind_group_layout, &globals_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("image_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[InstanceData::layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            sampler,
            texture_bind_group_layout,
            globals_bind_group_layout,
            globals_bind_group,
            globals_buffer,
            globals_stride,
            next_globals: 0,
            max_globals,
            instance_buffer,
            instance_stride,
            next_instance: 0,
            max_instances,
        }
    }

    pub fn upload_globals_bytes(&mut self, queue: &wgpu::Queue, bytes: &[u8]) -> anyhow::Result<u32> {
        if bytes.len() != Self::GLOBALS_SIZE_BYTES {
            return Err(anyhow::anyhow!("image globals must be exactly {} bytes", Self::GLOBALS_SIZE_BYTES));
        }
        if self.next_globals >= self.max_globals {
            return Err(anyhow::anyhow!("max image globals exceeded"));
        }
        let slot = self.next_globals;
        self.next_globals = self.next_globals.saturating_add(1);
        let offset = slot as wgpu::BufferAddress * self.globals_stride as wgpu::BufferAddress;
        queue.write_buffer(&self.globals_buffer, offset, bytes);
        Ok((slot * self.globals_stride) as u32)
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
        self.next_globals = 0;
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
        let offset_bytes = start as wgpu::BufferAddress * self.instance_stride as wgpu::BufferAddress;
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
        globals_offset: u32,
    ) {
        if instance_range.start == instance_range.end {
            return;
        }
        pass.set_pipeline(pipeline);
        let start =
            instance_range.start as wgpu::BufferAddress * self.instance_stride as wgpu::BufferAddress;
        let end =
            instance_range.end as wgpu::BufferAddress * self.instance_stride as wgpu::BufferAddress;
        pass.set_vertex_buffer(0, self.instance_buffer.slice(start..end));
        pass.set_bind_group(0, texture_bind_group, &[]);
        pass.set_bind_group(1, &self.globals_bind_group, &[globals_offset]);
        let instance_count = instance_range.end - instance_range.start;
        pass.draw(0..4, 0..instance_count);
    }
}