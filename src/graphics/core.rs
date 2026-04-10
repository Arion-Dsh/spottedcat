//! Core Graphics structure and initialization.

use crate::DrawOption;
use crate::ShaderOpts;
use crate::drawable::DrawCommand;
use crate::glyph_cache::GlyphCache;
use crate::image_raw::{ImageRenderer, InstanceData};
use ab_glyph::FontArc;
use std::collections::HashMap;

#[cfg(feature = "model-3d")]
use super::core_3d::Graphics3D;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ResolvedDraw {
    pub texture_id: u32,
    pub bounds: crate::image::Bounds,
    pub uv_rect: [f32; 4],
    pub opts: DrawOption,
    pub shader_id: u32,
    pub shader_opts: ShaderOpts,
}

#[cfg(feature = "model-3d")]
type GraphicsModel3dState = Option<Graphics3D>;
#[cfg(not(feature = "model-3d"))]
#[derive(Debug, Default)]
pub(crate) struct GraphicsModel3dState;

pub(crate) struct Graphics {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) adapter: wgpu::Adapter,
    pub(crate) config: wgpu::SurfaceConfiguration,
    pub(crate) image_renderer: ImageRenderer,
    pub(crate) default_pipeline: wgpu::RenderPipeline,
    pub(crate) image_pipelines: HashMap<u32, wgpu::RenderPipeline>,
    pub(crate) batch: Vec<InstanceData>,
    pub(crate) font_cache: HashMap<u64, FontArc>,
    pub(crate) glyph_cache: GlyphCache,
    pub(crate) resolved_draws: Vec<ResolvedDraw>,
    pub(crate) text_shader_id: u32,
    pub(crate) dirty_assets: bool,
    pub(crate) pipelines_dirty: bool,
    pub(crate) gpu_generation: u32,
    #[cfg_attr(not(feature = "model-3d"), allow(dead_code))]
    pub(crate) model_3d: GraphicsModel3dState,
    pub(crate) transparent: bool,
}

impl std::fmt::Debug for Graphics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Graphics")
            .field("device", &self.device)
            .field("transparent", &self.transparent)
            .finish_non_exhaustive()
    }
}

impl Graphics {
    #[cfg(not(feature = "model-3d"))]
    fn sync_new_runtime_3d_assets(&mut self, _ctx: &mut crate::Context) -> anyhow::Result<()> {
        Ok(())
    }

    #[cfg(not(feature = "model-3d"))]
    fn prewarm_3d_materials(&mut self, _ctx: &mut crate::Context) -> anyhow::Result<()> {
        Ok(())
    }

    #[cfg(not(feature = "model-3d"))]
    fn restore_3d_assets(&mut self, _ctx: &mut crate::Context) {}

    #[cfg(not(feature = "model-3d"))]
    fn resize_3d_surface_resources(
        &mut self,
        _width: u32,
        _height: u32,
        _old_width: u32,
        _old_height: u32,
    ) {
    }

    pub async fn new(
        instance: &wgpu::Instance,
        surface: &wgpu::Surface<'_>,
        width: u32,
        height: u32,
        transparent: bool,
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
        eprintln!(
            "[spot][init] Selected adapter: {:?} ({:?})",
            info.name, info.backend
        );

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
                format: pick_surface_format(&caps),
                width: width.max(1),
                height: height.max(1),
                present_mode: caps.present_modes[0],
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 1,
            });

        config.alpha_mode = pick_alpha_mode(&caps, transparent);

        config.present_mode = crate::graphics::profile::pick_present_mode(&caps);
        config.usage = crate::platform::surface_usage(&caps);

        surface.configure(&device, &config);

        let image_renderer = ImageRenderer::new(&device, config.format, 200000);

        let image_pipelines = HashMap::new();

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

        let graphics = Self {
            device,
            queue,
            adapter: adapter_clone,
            config,
            image_renderer,
            default_pipeline,
            image_pipelines,
            batch: Vec::with_capacity(10000),
            font_cache: HashMap::new(),
            glyph_cache: GlyphCache::new(),
            resolved_draws: Vec::with_capacity(10000),
            text_shader_id: 0,
            dirty_assets: true,
            pipelines_dirty: false,
            gpu_generation: 0, // This will be set by the platform/app
            model_3d: GraphicsModel3dState::default(),
            transparent,
        };

        // Default resources will be registered via the Context in App initialization
        Ok(graphics)
    }

    fn sync_new_runtime_assets(&mut self, ctx: &mut crate::Context) -> anyhow::Result<()> {
        for (&id, source) in &ctx.registry.image_shaders {
            if id != 0 && !self.image_pipelines.contains_key(&id) {
                self.restore_image_shader(id, source);
            }
        }
        self.sync_new_runtime_3d_assets(ctx)?;

        for (&id, data) in &ctx.registry.fonts {
            if let std::collections::hash_map::Entry::Vacant(entry) =
                self.font_cache.entry(id as u64)
            {
                let font = FontArc::try_from_vec(data.clone()).unwrap_or_else(|e| {
                    panic!("[spot][graphics] Failed to sync font with ID {}: {}", id, e)
                });
                entry.insert(font);
            }
        }

        self.process_registrations(ctx)?;
        ctx.registry.dirty_assets = false;
        Ok(())
    }

    pub(crate) fn prepare_frame_resources(
        &mut self,
        ctx: &mut crate::Context,
        drawables: &[DrawCommand],
    ) -> anyhow::Result<()> {
        for drawable in drawables {
            if let DrawCommand::Text(cmd) = drawable {
                self.ensure_text_layout(ctx, &cmd.text, cmd.opts.scale())?;
            }
        }

        if ctx.registry.dirty_assets {
            self.process_registrations(ctx)?;
        }

        self.prewarm_3d_materials(ctx)?;

        Ok(())
    }

    pub(crate) fn sync_assets(&mut self, ctx: &mut crate::Context) -> anyhow::Result<()> {
        if self.pipelines_dirty {
            self.rebuild_surface_format_dependent_pipelines(ctx);
        }

        if self.gpu_generation == ctx.registry.gpu_generation {
            if ctx.registry.dirty_assets {
                self.sync_new_runtime_assets(ctx)?;
            }
            return Ok(());
        }

        eprintln!(
            "[spot][graphics] GPU generation mismatch ({} vs {}). Restoring assets...",
            self.gpu_generation, ctx.registry.gpu_generation
        );

        // Reset transient caches
        self.font_cache.clear();
        self.glyph_cache.clear();

        // 1. Restore Shaders
        self.image_pipelines.clear();
        for (&id, source) in &ctx.registry.image_shaders {
            self.restore_image_shader(id, source);
        }

        // 2. Restore Fonts
        for (&id, data) in &ctx.registry.fonts {
            let font = ab_glyph::FontArc::try_from_vec(data.clone()).unwrap_or_else(|e| {
                panic!(
                    "[spot][graphics] Failed to restore font with ID {}: {}",
                    id, e
                )
            });
            self.font_cache.insert(id as u64, font);
        }

        // 3. Restore texture resources
        self.gpu_generation = ctx.registry.gpu_generation;
        self.text_shader_id = 1;

        self.dirty_assets = true;
        self.rebuild_textures(ctx)?;
        self.restore_3d_assets(ctx);

        // CRITICAL: Immediately process registrations to recreate all assets (including Canvases) for the new device
        self.process_registrations(ctx)?;

        eprintln!(
            "[spot][graphics] Asset restoration complete. generation={}",
            self.gpu_generation
        );

        Ok(())
    }

    pub fn resize(&mut self, surface: &wgpu::Surface<'_>, width: u32, height: u32) {
        if width == 0 || height == 0 {
            eprintln!(
                "[spot][graphics] Warning: Attempted resize with zero dimension: {}x{}",
                width, height
            );
            return;
        }

        let caps = surface.get_capabilities(&self.adapter);
        if caps.formats.is_empty() {
            eprintln!("[spot][graphics] Surface has no supported formats on resize!");
            return;
        }

        let old_width = self.config.width;
        let old_height = self.config.height;
        let old_format = self.config.format;

        self.config.width = width;
        self.config.height = height;

        // Try to keep the same format if possible to avoid pipeline incompatibility
        if !caps.formats.contains(&old_format) {
            let new_fmt = pick_surface_format(&caps);
            eprintln!(
                "[spot][graphics] Warning: Original surface format {:?} not supported by new surface. Switching to {:?}. Pipelines may become invalid!",
                old_format, new_fmt
            );
            self.config.format = new_fmt;
            self.pipelines_dirty = true;
        } else {
            self.config.format = old_format;
        }

        self.config.present_mode = crate::graphics::profile::pick_present_mode(&caps);
        self.config.usage = crate::platform::surface_usage(&caps);
        self.config.alpha_mode = pick_alpha_mode(&caps, self.transparent);
        surface.configure(&self.device, &self.config);

        self.resize_3d_surface_resources(width, height, old_width, old_height);
    }

    pub fn set_transparent(&mut self, transparent: bool) {
        self.transparent = transparent;
    }

    #[cfg(target_os = "android")]
    pub fn poll_device(&self, force_wait: bool) {
        let _ = self.device.poll(if force_wait {
            wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_millis(1)),
            }
        } else {
            wgpu::PollType::Poll
        });
    }

    pub fn transparent(&self) -> bool {
        self.transparent
    }
}

// Basic math helpers - removed and consolidated in crate::math.

fn pick_surface_format(caps: &wgpu::SurfaceCapabilities) -> wgpu::TextureFormat {
    // Prefer Srgb formats with alpha
    let preferred = [
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8Unorm,
    ];

    for &fmt in &preferred {
        if caps.formats.contains(&fmt) {
            return fmt;
        }
    }
    caps.formats[0]
}

fn pick_alpha_mode(
    caps: &wgpu::SurfaceCapabilities,
    requested_transparent: bool,
) -> wgpu::CompositeAlphaMode {
    if !requested_transparent && caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
        return wgpu::CompositeAlphaMode::Opaque;
    }

    // If transparent is requested, try to find a transparent-capable mode.
    // Even if not requested, we might want to use a transparent-capable mode
    // to allow dynamic toggling later.
    let transparent_modes = [
        #[cfg(target_os = "android")]
        wgpu::CompositeAlphaMode::Inherit, // Android GLES usually needs Inherit for transparency
        wgpu::CompositeAlphaMode::PostMultiplied,
        wgpu::CompositeAlphaMode::PreMultiplied,
        #[cfg(not(target_os = "android"))]
        wgpu::CompositeAlphaMode::Inherit,
    ];

    for mode in transparent_modes {
        if caps.alpha_modes.contains(&mode) {
            return mode;
        }
    }

    caps.alpha_modes[0]
}
