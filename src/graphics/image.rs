use std::{borrow::Cow, sync::Arc};

use bytemuck::{Pod, Zeroable};

use super::{Texture, Vertex};



#[derive(Clone)]
pub struct ImageState {
    pub(crate) pipeline: Arc<wgpu::RenderPipeline>,
    pub(crate) texture_bind_group_layout: Arc<wgpu::BindGroupLayout>, 
    pub(crate) uniform_bind_group: Arc<wgpu::BindGroup>,
    pub(crate) uniform_buffer: Arc<wgpu::Buffer>,
    pub(crate) texture_uniform_buffer: Arc<wgpu::Buffer>,
    pub(crate) color_uniform_buffer: Arc<wgpu::Buffer>,
}

impl ImageState {
    pub fn new(device: &wgpu::Device,  config: &wgpu::SurfaceConfiguration) -> Self {


    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("uniform_buffer"),
        size: std::mem::size_of::<ImageBaseUniform>() as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let uniform_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("uniform_bind_group_layout"),
        });

    let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &uniform_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
        label: Some("uniform_bind_group"),
    });

    let texture_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("texture_uniform_buffer"),
        size: std::mem::size_of::<TextureUniform>() as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let color_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("color_uniform_buffer"),
        size: std::mem::size_of::<ColorUniform>() as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let texture_bind_group_layout =
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
        label: Some("texture_bind_group_layout"),
    });


        // Load the shaders from disk
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("image_shader.wgsl"))),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[ &uniform_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One, // 或者 wgpu::BlendFactor::SrcAlpha
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha, // 通常也使用 OneMinusSrcAlpha
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL, // 确保写入颜色和 alpha
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil:Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })  ,
            multisample: wgpu::MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
            // Useful for optimizing shader compilation on Android
            cache: None,
        });


        Self {
            pipeline: pipeline.into()   ,
            texture_bind_group_layout: texture_bind_group_layout.into(),
            uniform_bind_group: uniform_bind_group.into(),
            uniform_buffer: uniform_buffer.into(),
            texture_uniform_buffer: texture_uniform_buffer.into(),
            color_uniform_buffer: color_uniform_buffer.into(),
        }
    }
    pub fn texture_bind_group(&self, device: &wgpu::Device, texture_view: &wgpu::TextureView, sampler: &wgpu::Sampler) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.texture_uniform_buffer.as_entire_binding(),
                    },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.color_uniform_buffer.as_entire_binding(),
                },
            ],
            label: Some("diffuse_bind_group"),
        })
    }
    
    pub(crate) fn write_texture_uniform(&self, queue: &wgpu::Queue, tsize: [f32; 2], uv_offset: [f32; 2], uv_size: [f32; 2]) {
        let texture_uniform = TextureUniform::new(tsize, uv_offset, uv_size);
        queue.write_buffer(&self.texture_uniform_buffer, 0, bytemuck::cast_slice(&[texture_uniform]));
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub(crate) struct ImageBaseUniform {
    screen_size: [f32; 2], // 窗口的像素尺寸 (width, height)
    pos: [f32; 2],      // 位置 (几何空间像素坐标)
    size: [f32; 2],
    scale: [f32; 2],        // 缩放因子
    rotation_angle: f32, // 旋转角度
    opacity: f32, // 透明度
    z_index: f32, // z 索引
    use_color_uniform: f32, // 是否使用颜色变换
}
impl ImageBaseUniform {
    pub(crate) fn new(screen_size: [f32; 2], pos: [f32; 2], size: [f32; 2], scale: [f32; 2], rotation_angle: f32, opacity: f32, z_index: f32) -> Self {
        Self {
            screen_size,
            pos,
            size,
            scale,
            rotation_angle,
            opacity,
            z_index,
            use_color_uniform: 0.0,
            }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub(crate) struct ColorUniform {
    matrix: [[f32; 4]; 4],
    transform: [f32; 4],
    use_uniform: f32,
    _padding: [f32; 3], 
}
impl Default for ColorUniform {
    fn default() -> Self {
        Self {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            transform: [0.0, 0.0, 0.0, 1.0],
            use_uniform : 0.0,
            _padding: [0.0, 0.0, 0.0],
        }
    }
}

impl ColorUniform {
    pub(crate) fn new() -> Self {
        Self {
            matrix:[
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            transform: [0.0, 0.0, 0.0, 1.0],
            use_uniform : 0.0,
            _padding: [0.0, 0.0, 0.0],
        }
    }
    pub(crate) fn is_default(&self) -> bool {
        self.use_uniform == 0.0 && self.matrix == [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ] && self.transform == [0.0, 0.0, 0.0, 1.0] 
    }
}

// struct TextureUniform {
//     t_size: vec2<f32>,      // 纹理的原始尺寸 (像素)
//     uv_offset: vec2<f32>,   // 纹理 UV 坐标的偏移量
//     uv_size: vec2<f32>,     // 纹理 UV 坐标的有效区域大小
// };
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub(crate) struct TextureUniform  {
    t_size: [f32; 2],      // 纹理的原始尺寸 (像素)
    uv_offset: [f32; 2],   // 纹理 UV 坐标的偏移量
    uv_size: [f32; 2],     // 纹理 UV 坐标的有效区域大小
    _padding: [f32; 2],
}
impl TextureUniform {
    pub(crate) fn new(t_size: [f32; 2], uv_offset: [f32; 2], uv_size: [f32; 2]  ) -> Self {
        Self {
            t_size,
            uv_offset,
            uv_size,
            _padding: [0.0, 0.0],
            }
    }
}

    
