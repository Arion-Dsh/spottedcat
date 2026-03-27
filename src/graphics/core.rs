//! Core Graphics structure and initialization.

use crate::DrawOption;
use crate::ShaderOpts;
use crate::glyph_cache::GlyphCache;
use crate::image::ImageEntry;
use crate::image_raw::{ImageRenderer, InstanceData};
use crate::model::Vertex;
use crate::graphics::model_raw::{ModelRenderer, MeshData};
use crate::packer::AtlasPacker;
use crate::texture::Texture;
use ab_glyph::FontArc;
use std::collections::HashMap;


pub(crate) struct AtlasSlot {
    pub packer: AtlasPacker,
    pub texture: Texture,
    pub bind_group: wgpu::BindGroup,
}

pub struct SkinData {
    pub bones: Vec<Bone>,
    pub bone_matrices: Vec<[[f32; 4]; 4]>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bone {
    pub parent_index: Option<usize>, // Index into 'bones' Vec
    pub inverse_bind_matrix: [[f32; 4]; 4],
}

#[derive(Clone)]
pub(crate) struct ResolvedDraw {
    pub img_entry: ImageEntry,
    pub opts: DrawOption,
    pub shader_id: u32,
    pub shader_opts: ShaderOpts,
}

pub struct Graphics {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) adapter: wgpu::Adapter,
    pub(crate) config: wgpu::SurfaceConfiguration,
    pub(crate) image_renderer: ImageRenderer,
    pub(crate) default_pipeline: wgpu::RenderPipeline,
    pub(crate) image_pipelines: HashMap<u32, wgpu::RenderPipeline>,
    pub(crate) model_pipelines: HashMap<u32, wgpu::RenderPipeline>,
    pub(crate) next_image_shader_id: u32,
    pub(crate) next_model_shader_id: u32,
    pub(crate) images: Vec<Option<ImageEntry>>,
    pub(crate) atlases: Vec<AtlasSlot>,
    pub(crate) batch: Vec<InstanceData>,
    pub(crate) font_cache: HashMap<u64, FontArc>,
    pub(crate) font_registry: HashMap<u32, Vec<u8>>,
    pub(crate) next_font_id: u32,
    pub(crate) glyph_cache: GlyphCache,
    pub(crate) resolved_draws: Vec<ResolvedDraw>,
    pub(crate) text_shader_id: u32,
    pub(crate) dirty_assets: bool,
    pub(crate) model_renderer: ModelRenderer,
    pub(crate) model_pipeline: wgpu::RenderPipeline,
    pub(crate) instanced_model_pipeline: wgpu::RenderPipeline,
    pub(crate) models: Vec<Option<MeshData>>,
    pub(crate) skins: Vec<Option<SkinData>>,
    pub(crate) depth_texture: wgpu::Texture,
    pub(crate) depth_view: wgpu::TextureView,
    pub(crate) white_image_id: u32,
    pub(crate) black_image_id: u32,
    pub(crate) normal_image_id: u32,
    #[allow(dead_code)]
    pub(crate) shadow_texture: wgpu::Texture,
    pub(crate) shadow_view: wgpu::TextureView,
    pub(crate) shadow_pipeline: wgpu::RenderPipeline,
    pub(crate) instanced_shadow_pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    pub(crate) irradiance_texture: wgpu::Texture,
    pub(crate) irradiance_view: wgpu::TextureView,
    #[allow(dead_code)]
    pub(crate) prefiltered_texture: wgpu::Texture,
    pub(crate) prefiltered_view: wgpu::TextureView,
    #[allow(dead_code)]
    pub(crate) brdf_lut_texture: wgpu::Texture,
    pub(crate) brdf_lut_view: wgpu::TextureView,
    pub(crate) scene_globals: crate::graphics::model_raw::SceneGlobals,
}

impl Graphics {
    pub async fn new(
        instance: &wgpu::Instance,
        surface: &wgpu::Surface<'_>,
        width: u32,
        height: u32,
    ) -> anyhow::Result<Self> {
        let width = width.max(1);
        let height = height.max(1);
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            })
            .await?;

        let info = adapter.get_info();
        eprintln!("[spot][init] Selected adapter: {:?} ({:?})", info.name, info.backend);

        let adapter_limits = adapter.limits();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: adapter_limits,
                experimental_features: wgpu::ExperimentalFeatures::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let adapter_clone = adapter.clone();

        let caps = surface.get_capabilities(&adapter);
        let mut config = surface
            .get_default_config(&adapter, width, height)
            .unwrap_or_else(|| wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: caps.formats[0],
                width: width.max(1),
                height: height.max(1),
                present_mode: caps.present_modes[0],
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            });

        config.present_mode = crate::graphics::profile::pick_present_mode(&caps);
        config.usage = crate::platform::surface_usage(&caps);

        surface.configure(&device, &config);

        let image_renderer = ImageRenderer::new(&device, config.format, 200000);

        let next_image_shader_id = 1;
        let image_pipelines = HashMap::new();

        let atlas_size = 2048;
        let packer = AtlasPacker::new(atlas_size, atlas_size, 2);
        let atlas_texture = Texture::create_empty(&device, atlas_size, atlas_size, config.format);
        let atlas_bind_group =
            image_renderer.create_texture_bind_group(&device, &atlas_texture.0.view);
        let atlases = vec![AtlasSlot {
            packer,
            texture: atlas_texture,
            bind_group: atlas_bind_group,
        }];

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("image_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/image.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("image_pipeline_layout"),
            bind_group_layouts: &[
                &image_renderer.texture_bind_group_layout,
                &image_renderer.user_globals_bind_group_layout,
                &image_renderer.engine_globals_bind_group_layout,
            ],
            immediate_size: 0,
        });

        let default_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("image_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
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
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        // Initialize 3D Renderer
        let model_renderer = ModelRenderer::new(&device);

        let shadow_size = 1024;
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow_texture"),
            size: wgpu::Extent3d { width: shadow_size, height: shadow_size, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[]
        });
        let shadow_view = shadow_texture.create_view(&Default::default());

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[]
        });
        let depth_view = depth_texture.create_view(&Default::default());

        let irradiance_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("irr_stub"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 6 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let irradiance_view = irradiance_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });

        let prefiltered_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pref_stub"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 6 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let prefiltered_view = prefiltered_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });

        let brdf_lut_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("brdf_stub"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let brdf_lut_view = brdf_lut_texture.create_view(&Default::default());

        let model_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("model_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/model.wgsl").into()),
        });

        let model_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("model_pipeline_layout"),
            bind_group_layouts: &[
                &model_renderer.globals_bind_group_layout,
                &model_renderer.texture_bind_group_layout,
                &model_renderer.bone_matrices_bind_group_layout,
                &model_renderer.environment_bind_group_layout,
            ],
            immediate_size: 0,
        });

        let model_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("model_pipeline"),
            layout: Some(&model_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &model_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Vertex::layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &model_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let instanced_model_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("model_instanced_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/model_instanced.wgsl").into()),
        });

        let instanced_model_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("instanced_model_pipeline"),
            layout: Some(&model_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &instanced_model_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    Vertex::layout(),
                    wgpu::VertexBufferLayout {
                        array_stride: 64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute { offset: 0, shader_location: 5, format: wgpu::VertexFormat::Float32x4 },
                            wgpu::VertexAttribute { offset: 16, shader_location: 6, format: wgpu::VertexFormat::Float32x4 },
                            wgpu::VertexAttribute { offset: 32, shader_location: 7, format: wgpu::VertexFormat::Float32x4 },
                            wgpu::VertexAttribute { offset: 48, shader_location: 8, format: wgpu::VertexFormat::Float32x4 },
                        ],
                    }
                ],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &instanced_model_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let shadow_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow_pipeline_layout"),
            bind_group_layouts: &[
                &model_renderer.globals_bind_group_layout,       // Group 0
                &model_renderer.bone_matrices_bind_group_layout, // Group 1 (mapped to Group 2 in full shader, but here bgls[1])
            ],
            immediate_size: 0,
        });

        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/shadow.wgsl").into()),
        });

        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow_pipeline"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shadow_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Vertex::layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });

        let instanced_shadow_pipeline = shadow_pipeline.clone(); // Stub for now

        let mut graphics = Self {
            device,
            queue,
            adapter: adapter_clone,
            config,
            image_renderer,
            default_pipeline,
            image_pipelines,
            model_pipelines: HashMap::new(),
            next_image_shader_id,
            next_model_shader_id: 1,
            images: Vec::new(),
            atlases,
            batch: Vec::with_capacity(10000),
            font_cache: HashMap::new(),
            font_registry: HashMap::new(),
            next_font_id: 1,
            glyph_cache: GlyphCache::new(),
            resolved_draws: Vec::with_capacity(10000),
            text_shader_id: 0,
            dirty_assets: false,
            model_renderer,
            model_pipeline,
            instanced_model_pipeline,
            models: Vec::new(),
            skins: Vec::new(),
            depth_texture,
            depth_view,
            shadow_texture,
            shadow_view,
            shadow_pipeline,
            instanced_shadow_pipeline,
            irradiance_texture,
            irradiance_view,
            prefiltered_texture,
            prefiltered_view,
            brdf_lut_texture,
            brdf_lut_view,
            white_image_id: 0,
            black_image_id: 0,
            normal_image_id: 0,
            scene_globals: crate::graphics::model_raw::SceneGlobals {
                camera_pos: [0.0, 0.0, 0.0, 0.0],
                ambient_color: [0.1, 0.1, 0.1, 1.0],
                lights: [crate::graphics::model_raw::Light {
                    position: [1.0, 1.0, 1.0, 0.0],
                    color: [1.0, 1.0, 1.0, 1.0],
                }; 4],
                light_view_proj: identity(),
            },
        };

        // Create default textures
        let white_id = graphics.create_image(1u32.into(), 1u32.into(), &[255, 255, 255, 255]).unwrap().id();
        graphics.white_image_id = white_id;

        let black_id = graphics.create_image(1u32.into(), 1u32.into(), &[0, 0, 0, 255]).unwrap().id();
        graphics.black_image_id = black_id;

        let normal_id = graphics.create_image(1u32.into(), 1u32.into(), &[128, 128, 255, 255]).unwrap().id();
        graphics.normal_image_id = normal_id;

        // Register default text shader for tinting
        let text_shader_src = r#"
            fn user_fs_hook() {
                let tint = user_globals[0];
                color = vec4<f32>(color.rgb * tint.rgb, color.a * tint.a);
            }
        "#;
        graphics.text_shader_id = graphics.register_image_shader(text_shader_src);

        Ok(graphics)
    }

    pub fn resize(&mut self, surface: &wgpu::Surface<'_>, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        
        let caps = surface.get_capabilities(&self.adapter);
        if caps.formats.is_empty() {
             eprintln!("[spot][graphics] surface has no supported formats!");
             return;
        }

        self.config.width = width;
        self.config.height = height;
        self.config.format = caps.formats[0]; // Ensure we use a format this surface supports
        self.config.present_mode = crate::graphics::profile::pick_present_mode(&caps);
        self.config.usage = crate::platform::surface_usage(&caps);
        
        surface.configure(&self.device, &self.config);

        self.depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size: wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        self.depth_view = self.depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
    }

    pub fn create_mesh(&mut self, vertices: &[Vertex], indices: &[u32]) -> anyhow::Result<u32> {
        let mesh = MeshData::new(&self.device, vertices, indices);
        mesh.upload(&self.queue, vertices, indices);
        let id = self.models.len() as u32;
        self.models.push(Some(mesh));
        Ok(id)
    }

    pub fn create_skin(&mut self, bones: Vec<Bone>, bone_matrices: Vec<[[f32; 4]; 4]>) -> u32 {
        let id = self.skins.len() as u32;
        self.skins.push(Some(SkinData { bones, bone_matrices }));
        id
    }

    pub fn update_bone_matrices(&mut self, skin_id: u32, matrices: &[[[f32; 4]; 4]]) {
        if let Some(Some(skin)) = self.skins.get_mut(skin_id as usize) {
            for (i, matrix) in matrices.iter().enumerate() {
                if i < skin.bone_matrices.len() {
                    skin.bone_matrices[i] = *matrix;
                }
            }
        }
    }
}

// Basic math helpers
pub fn identity() -> [[f32; 4]; 4] {
    [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]
}

pub fn create_scale(s: [f32; 3]) -> [[f32; 4]; 4] {
    [[s[0], 0.0, 0.0, 0.0], [0.0, s[1], 0.0, 0.0], [0.0, 0.0, s[2], 0.0], [0.0, 0.0, 0.0, 1.0]]
}

pub fn create_rotation_from_quat(q: [f32; 4]) -> [[f32; 4]; 4] {
    let x = q[0]; let y = q[1]; let z = q[2]; let w = q[3];
    let x2 = x + x; let y2 = y + y; let z2 = z + z;
    let xx = x * x2; let xy = x * y2; let xz = x * z2;
    let yy = y * y2; let yz = y * z2; let zz = z * z2;
    let wx = w * x2; let wy = w * y2; let wz = w * z2;

    [
        [1.0 - (yy + zz), xy + wz, xz - wy, 0.0],
        [xy - wz, 1.0 - (xx + zz), yz + wx, 0.0],
        [xz + wy, yz - wx, 1.0 - (xx + yy), 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}
