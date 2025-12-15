use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;


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
struct QuadVertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

impl QuadVertex {
    const ATTRS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }
}

fn align_up(value: u32, alignment: u32) -> u32 {
    debug_assert!(alignment.is_power_of_two());
    (value + alignment - 1) & !(alignment - 1)
}

pub struct ImageRenderer {
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) quad_vertex_buffer: wgpu::Buffer,
    pub(crate) quad_index_buffer: wgpu::Buffer,
    pub(crate) quad_index_count: u32,

    pub(crate) transform_buffer: wgpu::Buffer,
    pub(crate) transform_bind_group: wgpu::BindGroup,
    pub(crate) transform_stride: u32,

    pub(crate) sampler: wgpu::Sampler,
    pub(crate) texture_bind_group_layout: wgpu::BindGroupLayout,

    next_instance: u32,
    max_instances: u32,
}

impl ImageRenderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat, max_instances: u32) -> Self {
        let transform_size = std::mem::size_of::<ImageTransform>() as u32;
        let alignment = device.limits().min_uniform_buffer_offset_alignment;
        let transform_stride = align_up(transform_size, alignment);
        let transform_buffer_size = transform_stride as wgpu::BufferAddress * max_instances as wgpu::BufferAddress;

        let transform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image_transform_buffer"),
            size: transform_buffer_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let transform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("image_transform_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(
                        std::num::NonZeroU64::new(std::mem::size_of::<ImageTransform>() as u64).unwrap(),
                    ),
                },
                count: None,
            }],
        });

        let transform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image_transform_bg"),
            layout: &transform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &transform_buffer,
                    offset: 0,
                    size: Some(std::num::NonZeroU64::new(std::mem::size_of::<ImageTransform>() as u64).unwrap()),
                }),
            }],
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("image_pipeline_layout"),
            bind_group_layouts: &[&transform_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("image_shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
struct ImageTransform {
    mvp: mat4x4<f32>,
    uvp: mat4x4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: ImageTransform;

@group(1) @binding(0)
var tex: texture_2d<f32>;

@group(1) @binding(1)
var samp: sampler;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    out.clip_pos = u.mvp * vec4<f32>(in.pos, 0.0, 1.0);
    let uv4 = u.uvp * vec4<f32>(in.uv, 0.0, 1.0);
    out.uv = uv4.xy;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, in.uv);
    return c * u.color;
}
"#
                .into(),
            ),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("image_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[QuadVertex::layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
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
                module: &shader,
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

        let vertices: [QuadVertex; 4] = [
            QuadVertex { pos: [-1.0, -1.0], uv: [0.0, 1.0] },
            QuadVertex { pos: [1.0, -1.0], uv: [1.0, 1.0] },
            QuadVertex { pos: [1.0, 1.0], uv: [1.0, 0.0] },
            QuadVertex { pos: [-1.0, 1.0], uv: [0.0, 0.0] },
        ];
        let indices: [u16; 6] = [0, 1, 2, 2, 3, 0];

        let quad_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("image_quad_vb"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let quad_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("image_quad_ib"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            pipeline,
            quad_vertex_buffer,
            quad_index_buffer,
            quad_index_count: indices.len() as u32,
            transform_buffer,
            transform_bind_group,
            transform_stride,
            sampler,
            texture_bind_group_layout,
            next_instance: 0,
            max_instances,
        }
    }

    pub fn create_image(&mut self, device: &wgpu::Device, texture_view: &wgpu::TextureView) -> anyhow::Result<ImageRaw> {
        if self.next_instance >= self.max_instances {
            return Err(anyhow::anyhow!("max image instances exceeded"));
        }

        let transform_offset = self.next_instance * self.transform_stride;
        self.next_instance += 1;

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
        });

        Ok(ImageRaw {
            texture_bind_group,
            transform_offset,
            transform: ImageTransform::default(),
            dirty: true,
        })
    }


    pub fn flush_image(&self, queue: &wgpu::Queue, image: &mut ImageRaw) {
        if !image.dirty {
            return;
        }

        queue.write_buffer(
            &self.transform_buffer,
            image.transform_offset as wgpu::BufferAddress,
            bytemuck::bytes_of(&image.transform),
        );
        image.dirty = false;
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, image: &ImageRaw) {
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
        pass.set_index_buffer(self.quad_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.set_bind_group(0, &self.transform_bind_group, &[image.transform_offset]);
        pass.set_bind_group(1, &image.texture_bind_group, &[]);
        pass.draw_indexed(0..self.quad_index_count, 0, 0..1);
    }
}

pub struct ImageRaw {
    pub(crate) texture_bind_group: wgpu::BindGroup,
    pub(crate) transform_offset: u32,
    pub(crate) transform: ImageTransform,
    pub(crate) dirty: bool,
}

impl ImageRaw {
    pub fn set_transform(&mut self, t: ImageTransform) {
        self.transform = t;
        self.dirty = true;
    }
}