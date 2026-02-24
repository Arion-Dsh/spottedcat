//! Core Graphics structure and initialization.

use crate::DrawOption;
use crate::ShaderOpts;
use crate::glyph_cache::GlyphCache;
use crate::image::ImageEntry;
use crate::image_raw::{ImageRenderer, InstanceData};
use crate::packer::AtlasPacker;
use crate::platform;
use crate::texture::Texture;
use ab_glyph::FontArc;
use std::collections::HashMap;

use super::profile::pick_present_mode;

pub(crate) struct AtlasSlot {
    pub packer: AtlasPacker,
    pub texture: Texture,
    pub bind_group: wgpu::BindGroup,
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
    pub(crate) config: wgpu::SurfaceConfiguration,
    pub(crate) image_renderer: ImageRenderer,
    pub(crate) default_pipeline: wgpu::RenderPipeline,
    pub(crate) image_pipelines: HashMap<u32, wgpu::RenderPipeline>,
    pub(crate) next_image_shader_id: u32,
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
}

impl Graphics {
    pub async fn new(
        instance: &wgpu::Instance,
        surface: &wgpu::Surface<'_>,
        width: u32,
        height: u32,
    ) -> anyhow::Result<Self> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            })
            .await?;

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

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let usage = platform::surface_usage(&surface_caps);
        let present_mode = pick_present_mode(&surface_caps);

        let config = wgpu::SurfaceConfiguration {
            usage,
            format: surface_format,
            width: width.max(1),
            height: height.max(1),
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let image_renderer = ImageRenderer::new(&device, config.format, 200000);
        let image_pipelines = HashMap::new();
        let next_image_shader_id = 1;

        let atlas_size = 4096;
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
            push_constant_ranges: &[],
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
            multiview: None,
            cache: None,
        });

        let mut graphics = Self {
            device,
            queue,
            config,
            image_renderer,
            default_pipeline,
            image_pipelines,
            next_image_shader_id,
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
        };

        // Register default text shader for tinting
        let text_shader_src = r#"
            fn user_fs_hook() {
                let tint = user_globals[0];
                color = vec4<f32>(tint.rgb, tint.a * color.a);
            }
        "#;
        graphics.text_shader_id = graphics.register_image_shader(text_shader_src);

        Ok(graphics)
    }

    pub fn resize(&mut self, surface: &wgpu::Surface<'_>, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        surface.configure(&self.device, &self.config);
    }
}
