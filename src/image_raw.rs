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
pub struct InstanceData {
    pub mvp: [[f32; 4]; 4],
    pub uvp: [[f32; 4]; 4],
    pub color: [f32; 4],
}

impl From<ImageTransform> for InstanceData {
    fn from(t: ImageTransform) -> Self {
        Self {
            mvp: t.mvp,
            uvp: t.uvp,
            color: t.color,
        }
    }
}

impl InstanceData {
    const ATTRS: [wgpu::VertexAttribute; 9] = wgpu::vertex_attr_array![
        2 => Float32x4,
        3 => Float32x4,
        4 => Float32x4,
        5 => Float32x4,
        6 => Float32x4,
        7 => Float32x4,
        8 => Float32x4,
        9 => Float32x4,
        10 => Float32x4
    ];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceData>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRS,
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

    pub(crate) sampler: wgpu::Sampler,
    pub(crate) texture_bind_group_layout: wgpu::BindGroupLayout,

    pub(crate) instance_buffer: wgpu::Buffer,
    pub(crate) instance_stride: u32,

    next_instance: u32,
    max_instances: u32,
}

impl ImageRenderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat, max_instances: u32) -> Self {
        let instance_size = std::mem::size_of::<InstanceData>() as u32;
        let alignment = 16;
        let instance_stride = align_up(instance_size, alignment);
        let instance_buffer_size = instance_stride as wgpu::BufferAddress * max_instances as wgpu::BufferAddress;

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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("image_pipeline_layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("image_shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
@group(0) @binding(0)
var tex: texture_2d<f32>;

@group(0) @binding(1)
var samp: sampler;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) mvp0: vec4<f32>,
    @location(3) mvp1: vec4<f32>,
    @location(4) mvp2: vec4<f32>,
    @location(5) mvp3: vec4<f32>,
    @location(6) uvp0: vec4<f32>,
    @location(7) uvp1: vec4<f32>,
    @location(8) uvp2: vec4<f32>,
    @location(9) uvp3: vec4<f32>,
    @location(10) color: vec4<f32>,
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    let mvp = mat4x4<f32>(in.mvp0, in.mvp1, in.mvp2, in.mvp3);
    let uvp = mat4x4<f32>(in.uvp0, in.uvp1, in.uvp2, in.uvp3);
    out.clip_pos = mvp * vec4<f32>(in.pos, 0.0, 1.0);
    let uv4 = uvp * vec4<f32>(in.uv, 0.0, 1.0);
    out.uv = uv4.xy;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, in.uv);
    return c * in.color;
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
                buffers: &[QuadVertex::layout(), InstanceData::layout()],
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
            sampler,
            texture_bind_group_layout,
            instance_buffer,
            instance_stride,
            next_instance: 0,
            max_instances,
        }
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
        texture_bind_group: &wgpu::BindGroup,
        instance_range: std::ops::Range<u32>,
    ) {
        if instance_range.start == instance_range.end {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
        let start = instance_range.start as wgpu::BufferAddress * self.instance_stride as wgpu::BufferAddress;
        let end = instance_range.end as wgpu::BufferAddress * self.instance_stride as wgpu::BufferAddress;
        pass.set_vertex_buffer(1, self.instance_buffer.slice(start..end));
        pass.set_index_buffer(self.quad_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.set_bind_group(0, texture_bind_group, &[]);
        let instance_count = instance_range.end - instance_range.start;
        pass.draw_indexed(0..self.quad_index_count, 0, 0..instance_count);
    }
}