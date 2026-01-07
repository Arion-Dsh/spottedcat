use crate::{Context, DrawCommand, DrawOption};
use crate::image::{Bounds, Image, ImageEntry};
use crate::image_raw::{ImageRenderer, InstanceData};
use crate::packer::AtlasPacker;
use crate::platform;
use crate::texture::Texture;
use crate::text_renderer::TextRenderer;
use crate::pt::Pt;
use crate::ShaderOpts;
use crate::Text;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Instant;
use std::sync::Mutex;
use ab_glyph::FontArc;

static PROFILE_RENDER: OnceLock<bool> = OnceLock::new();

struct RenderProfileStats {
    frame: u64,
    sum_total_ms: f64,
    sum_wait_ms: f64,
    sum_work_ms: f64,
    min_total_ms: f64,
    max_total_ms: f64,
}

impl Default for RenderProfileStats {
    fn default() -> Self {
        Self {
            frame: 0,
            sum_total_ms: 0.0,
            sum_wait_ms: 0.0,
            sum_work_ms: 0.0,
            min_total_ms: f64::INFINITY,
            max_total_ms: 0.0,
        }
    }
}

fn parse_present_mode_from_env() -> Option<wgpu::PresentMode> {
    let v = std::env::var("SPOT_PRESENT_MODE").ok()?;
    let v = v.trim().to_ascii_lowercase();
    if v.is_empty() {
        return None;
    }
    match v.as_str() {
        "immediate" => Some(wgpu::PresentMode::Immediate),
        "mailbox" => Some(wgpu::PresentMode::Mailbox),
        "fifo" => Some(wgpu::PresentMode::Fifo),
        "auto" => Some(wgpu::PresentMode::AutoVsync),
        "auto_vsync" => Some(wgpu::PresentMode::AutoVsync),
        "auto_no_vsync" => Some(wgpu::PresentMode::AutoNoVsync),
        _ => None,
    }
}

fn pick_present_mode(surface_caps: &wgpu::SurfaceCapabilities) -> wgpu::PresentMode {
    if let Some(requested) = parse_present_mode_from_env() {
        if surface_caps.present_modes.iter().any(|m| *m == requested) {
            return requested;
        }
    }

    if surface_caps
        .present_modes
        .iter()
        .any(|m| *m == wgpu::PresentMode::Immediate)
    {
        wgpu::PresentMode::Immediate
    } else {
        surface_caps.present_modes[0]
    }
}

struct AtlasSlot {
    packer: AtlasPacker,
    texture: Texture,
    bind_group: wgpu::BindGroup,
}

pub struct Graphics {
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    image_renderer: ImageRenderer,
    default_pipeline: wgpu::RenderPipeline,
    image_pipelines: HashMap<u32, wgpu::RenderPipeline>,
    next_image_shader_id: u32,
    images: Vec<Option<ImageEntry>>,
    text_renderer: TextRenderer,
    atlases: Vec<AtlasSlot>,
    batch: Vec<InstanceData>,
    font_cache: HashMap<u64, FontArc>,
    resolved_draws: Vec<ResolvedDraw>,
    text_image_cache: HashMap<TextImageKey, ImageEntry>,
}

#[derive(Clone, Copy)]
struct ResolvedDraw {
    img_entry: ImageEntry,
    opts: DrawOption,
    shader_id: u32,
    shader_opts: ShaderOpts,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TextImageKey {
    content: String,
    font_hash: u64,
    font_size_bits: u32,
    color_bits: [u32; 4],
    max_width_bits: Option<u32>,
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
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: adapter_limits,
                    experimental_features: wgpu::ExperimentalFeatures::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                    trace: wgpu::Trace::Off,
                },
            )
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
        let text_renderer = TextRenderer::new(&device, config.format);

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
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/image.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("image_pipeline_layout"),
            bind_group_layouts: &[
                &image_renderer.texture_bind_group_layout, 
                &image_renderer.user_globals_bind_group_layout, 
                &image_renderer.engine_globals_bind_group_layout
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

        Ok(Self {
            device,
            queue,
            config,
            image_renderer,
            default_pipeline,
            image_pipelines,
            next_image_shader_id,
            images: Vec::new(),
            text_renderer,
            atlases,
            batch: Vec::with_capacity(10000),
            font_cache: HashMap::new(),
            resolved_draws: Vec::with_capacity(10000),
            text_image_cache: HashMap::new(),
        })
    }

    pub(crate) fn register_image_shader(&mut self, user_functions: &str) -> u32 {
        let shader_id = self.next_image_shader_id;
        self.next_image_shader_id = self.next_image_shader_id.saturating_add(1);

        // Hook-function injection.
        // User provides WGSL code snippets:
        // They will be inserted at markers in vs_main and fs_main.
        let base_template = include_str!("shaders/image.wgsl");
        let mut combined_shader = base_template.to_string();

        if let Some(vs_start) = user_functions.find("fn user_vs_hook") {
            let vs_body_start = user_functions[vs_start..].find('{').map(|i| vs_start + i + 1).unwrap_or(vs_start);
            let vs_end = user_functions[vs_body_start..]
                .find("fn user_fs_hook")
                .map(|rel| vs_body_start + rel)
                .unwrap_or(user_functions.len());
            let vs_body_end = user_functions[..vs_end].rfind('}').unwrap_or(vs_end);
            let vs_src = user_functions[vs_body_start..vs_body_end].trim();

            if !vs_src.is_empty() {
                let marker = "// USER_VS_HOOK";
                if let Some(pos) = combined_shader.rfind(marker) {
                    combined_shader.insert_str(pos + marker.len(), &format!("\n{}", vs_src));
                }
            }
        }

        if let Some(fs_start) = user_functions.find("fn user_fs_hook") {
            let fs_body_start = user_functions[fs_start..].find('{').map(|i| fs_start + i + 1).unwrap_or(fs_start);
            let fs_end = user_functions.len();
            let fs_body_end = user_functions[..fs_end].rfind('}').unwrap_or(fs_end);
            let fs_src = user_functions[fs_body_start..fs_body_end].trim();

            if !fs_src.is_empty() {
                let marker = "// USER_FS_HOOK";
                if let Some(pos) = combined_shader.rfind(marker) {
                    combined_shader.insert_str(pos + marker.len(), &format!("\n{}", fs_src));
                }
            }
        }

        if std::env::var("SPOT_DEBUG_SHADER").is_ok() {
            let vs_marker = "// USER_VS_HOOK";
            let fs_marker = "// USER_FS_HOOK";

            let vs_block = if let Some(pos) = combined_shader.find(vs_marker) {
                let end = combined_shader[pos..].find("return").map(|i| pos + i).unwrap_or(combined_shader.len());
                &combined_shader[pos..end]
            } else {
                "<missing vs hook marker>"
            };
            let fs_block = if let Some(pos) = combined_shader.find(fs_marker) {
                let end = combined_shader[pos..].find("return").map(|i| pos + i).unwrap_or(combined_shader.len());
                &combined_shader[pos..end]
            } else {
                "<missing fs hook marker>"
            };

            eprintln!("[spot][debug][shader] register_image_shader id={}\n{}\n{}", shader_id, vs_block, fs_block);
        }

        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("custom_image_shader"),
            source: wgpu::ShaderSource::Wgsl(combined_shader.into()),
        });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("custom_image_pipeline_layout"),
                bind_group_layouts: &[
                    &self.image_renderer.texture_bind_group_layout,
                    &self.image_renderer.user_globals_bind_group_layout,
                    &self.image_renderer.engine_globals_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let _pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("custom_image_pipeline"),
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
                    format: self.config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        self.image_pipelines.insert(shader_id, _pipeline);
        shader_id
    }

    fn ensure_atlas_for_image(&mut self, w: u32, h: u32) -> anyhow::Result<(u32, crate::packer::PackerRect)> {
        let Some(last) = self.atlases.last_mut() else {
            return Err(anyhow::anyhow!("no atlas"));
        };

        if let Some(rect) = last.packer.insert_raw(w, h) {
            let atlas_index = (self.atlases.len() - 1) as u32;
            return Ok((atlas_index, rect));
        }

        let atlas_size = 4096;
        let packer = AtlasPacker::new(atlas_size, atlas_size, 2);
        let texture = Texture::create_empty(&self.device, atlas_size, atlas_size, self.config.format);
        let bind_group = self
            .image_renderer
            .create_texture_bind_group(&self.device, &texture.0.view);
        self.atlases.push(AtlasSlot {
            packer,
            texture,
            bind_group,
        });

        let atlas_index = (self.atlases.len() - 1) as u32;
        let rect = self
            .atlases
            .last_mut()
            .expect("atlas")
            .packer
            .insert_raw(w, h)
            .ok_or_else(|| anyhow::anyhow!("image too large for atlas"))?;
        Ok((atlas_index, rect))
    }

    pub fn resize(&mut self, surface: &wgpu::Surface<'_>, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        surface.configure(&self.device, &self.config);
    }

    pub(crate) fn create_image(&mut self, width: Pt, height: Pt, rgba: &[u8]) -> anyhow::Result<Image> {
        let w = width.to_u32_clamped();
        let h = height.to_u32_clamped();

        let (atlas_index, rect) = self.ensure_atlas_for_image(w, h)?;
        let atlas = self
            .atlases
            .get(atlas_index as usize)
            .expect("atlas");

        let mut extruded_data = atlas.packer.extrude_rgba8(rgba, w, h);

        match atlas.texture.0.format {
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
                for p in extruded_data.chunks_exact_mut(4) {
                    p.swap(0, 2);
                }
            }
            _ => {}
        }

        let (tx, ty, tw, th) = atlas.packer.get_write_info(&rect);
        let bytes_per_row = 4 * tw;
        let (data, bytes_per_row) =
            platform::align_write_texture_bytes(bytes_per_row, th, extruded_data);

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas.texture.0.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: tx, y: ty, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(th),
            },
            wgpu::Extent3d {
                width: tw,
                height: th,
                depth_or_array_layers: 1,
            },
        );

        // Generate mipmaps for the atlas texture
        atlas.texture.generate_mipmaps(&self.device, &self.queue);

        let uv_param = atlas.packer.get_uv_param(&rect);
        let uv_rect = [uv_param[0], uv_param[1], uv_param[2], uv_param[3]];
        let bounds = Bounds::new(Pt(0.0), Pt(0.0), width, height);
        let entry = ImageEntry::new(atlas_index, bounds, uv_rect);
        Ok(self.insert_image_entry(entry))
    }
    
    pub(crate) fn create_sub_image(&mut self, image: Image, bounds: Bounds) -> anyhow::Result<Image> {
        let parent_entry = self.images.get(image.index())
            .and_then(|v| v.as_ref())
            .ok_or_else(|| anyhow::anyhow!("Invalid parent image"))?;
            
        let p_u0 = parent_entry.uv_rect[0];
        let p_v0 = parent_entry.uv_rect[1];
        let p_w = parent_entry.uv_rect[2];
        let p_h = parent_entry.uv_rect[3];
        
        let parent_w = parent_entry.bounds.width.as_f32();
        let parent_h = parent_entry.bounds.height.as_f32();
        
        let nx = bounds.x.as_f32() / parent_w;
        let ny = bounds.y.as_f32() / parent_h;
        let nw = bounds.width.as_f32() / parent_w;
        let nh = bounds.height.as_f32() / parent_h;
        
        let g_u0 = p_u0 + nx * p_w;
        let g_v0 = p_v0 + ny * p_h;
        let g_w = nw * p_w;
        let g_h = nh * p_h;
        
        let uv_rect = [g_u0, g_v0, g_w, g_h];
        let entry = ImageEntry::new(parent_entry.atlas_index, bounds, uv_rect);
        Ok(self.insert_image_entry(entry))
    }

    pub(crate) fn insert_image_entry(&mut self, entry: ImageEntry) -> Image {
        let id = self.images.len() as u32;
        let bounds = entry.bounds;
        self.images.push(Some(entry));
        Image {
            id,
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: bounds.height,
        }
    }

    pub(crate) fn take_image_entry(&mut self, image: Image) -> Option<ImageEntry> {
        self.images.get_mut(image.index())?.take()
    }

    pub(crate) fn image_bounds(&self, image: Image) -> anyhow::Result<Bounds> {
        self.images
            .get(image.index())
            .and_then(|v| v.as_ref())
            .map(|e| e.bounds)
            .ok_or_else(|| anyhow::anyhow!("invalid image"))
    }

    pub(crate) fn render_text_to_image(&mut self, text: &Text, _opts: &DrawOption) -> anyhow::Result<Option<ImageEntry>> {
        use ab_glyph::{Font as _, Glyph, PxScale, ScaleFont as _};
        
        let font_hash = text.font_hash;
        let font = if let Some(cached_font) = self.get_cached_font(font_hash) {
            cached_font
        } else {
            let font = FontArc::try_from_vec(text.font_data.clone())
                .map_err(|e| anyhow::anyhow!("Failed to parse font: {}", e))?;
            self.cache_font(font_hash, font.clone());
            font
        };

        let px_size = text.font_size.as_f32().max(1.0);
        let scale = PxScale::from(px_size);
        let scaled = font.as_scaled(scale);
        
        if text.max_width.is_some() {
            return self.render_text_to_image_with_wrapping(text, &font, &scaled);
        }
        
        let mut caret_x = 0.0f32;
        let mut max_width = 0.0f32;
        let mut min_y = scaled.ascent();
        let mut max_y = scaled.descent();
        
        for ch in text.content.chars() {
            let id = scaled.glyph_id(ch);
            caret_x += scaled.h_advance(id);
            max_width = max_width.max(caret_x);
            
            if let Some(glyph) = scaled.outline_glyph(Glyph { id, scale, position: ab_glyph::point(0.0, 0.0) }) {
                let bounds = glyph.px_bounds();
                min_y = min_y.min(bounds.min.y);
                max_y = max_y.max(bounds.max.y);
            }
        }
        
        let text_width = max_width.ceil().max(1.0) as u32;
        let text_height = (max_y - min_y).ceil().max(1.0) as u32;
        let mut rgba_data = vec![0u8; (text_width * text_height * 4) as usize];
        
        caret_x = 0.0f32;
        let baseline_y = 0.0f32;
        
        for ch in text.content.chars() {
            let id = scaled.glyph_id(ch);
            if let Some(glyph) = scaled.outline_glyph(Glyph { id, scale, position: ab_glyph::point(0.0, 0.0) }) {
                let bounds = glyph.px_bounds();
                let glyph_x = caret_x + bounds.min.x;
                let glyph_y = baseline_y + (bounds.min.y - min_y);
                
                glyph.draw(|x, y, v| {
                    let px = x as i32 + glyph_x as i32;
                    let py = y as i32 + glyph_y as i32;
                    if px >= 0 && py >= 0 && px < text_width as i32 && py < text_height as i32 {
                        let idx = ((py as u32 * text_width + px as u32) * 4) as usize;
                        let alpha = (v * 255.0).round().clamp(0.0, 255.0) as u8;
                        rgba_data[idx] = (text.color[2] * 255.0) as u8;
                        rgba_data[idx + 1] = (text.color[1] * 255.0) as u8;
                        rgba_data[idx + 2] = (text.color[0] * 255.0) as u8;
                        rgba_data[idx + 3] = alpha;
                    }
                });
            }
            caret_x += scaled.h_advance(id);
        }
        
        let image = self.create_image(Pt::from(text_width as f32), Pt::from(text_height as f32), &rgba_data)?;
        Ok(self.images.get(image.index()).and_then(|e| e.as_ref()).copied())
    }

    fn render_text_to_image_with_wrapping(&mut self, text: &Text, _font: &FontArc, scaled: &ab_glyph::PxScaleFont<&FontArc>) -> anyhow::Result<Option<ImageEntry>> {
        use ab_glyph::{Glyph, ScaleFont as _};
        
        let lines = text.get_wrapped_lines(&scaled);
        let line_height = scaled.ascent() - scaled.descent();
        let mut max_width = 0.0f32;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        
        for line in &lines {
            let mut line_width = 0.0f32;
            let mut prev: Option<ab_glyph::GlyphId> = None;
            for ch in line.chars() {
                let id = scaled.glyph_id(ch);
                if let Some(p) = prev { line_width += scaled.kern(p, id); }
                line_width += scaled.h_advance(id);
                prev = Some(id);
                if let Some(glyph) = scaled.outline_glyph(Glyph { id, scale: scaled.scale(), position: ab_glyph::point(0.0, 0.0) }) {
                    let bounds = glyph.px_bounds();
                    min_y = min_y.min(bounds.min.y);
                    max_y = max_y.max(bounds.max.y);
                }
            }
            max_width = max_width.max(line_width);
        }
        
        if min_y == f32::MAX { min_y = scaled.descent(); }
        let text_height = (lines.len() as f32 * line_height).ceil().max(1.0) as u32;
        let text_width = max_width.ceil().max(1.0) as u32;
        let mut rgba_data = vec![0u8; (text_width * text_height * 4) as usize];
        let baseline_y = scaled.ascent();
        
        for (line_index, line) in lines.iter().enumerate() {
            let mut caret_x = 0.0f32;
            let line_y = baseline_y + (line_index as f32 * line_height);
            let mut prev: Option<ab_glyph::GlyphId> = None;
            for ch in line.chars() {
                let id = scaled.glyph_id(ch);
                if let Some(p) = prev { caret_x += scaled.kern(p, id); }
                if let Some(glyph) = scaled.outline_glyph(Glyph { id, scale: scaled.scale(), position: ab_glyph::point(0.0, 0.0) }) {
                    let bounds = glyph.px_bounds();
                    let glyph_x = caret_x + bounds.min.x;
                    let glyph_y = line_y + (bounds.min.y - min_y);
                    glyph.draw(|x, y, v| {
                        let px = x as i32 + glyph_x as i32;
                        let py = y as i32 + glyph_y as i32;
                        if px >= 0 && py >= 0 && px < text_width as i32 && py < text_height as i32 {
                            let idx = ((py as u32 * text_width + px as u32) * 4) as usize;
                            let alpha = (v * 255.0).round().clamp(0.0, 255.0) as u8;
                            rgba_data[idx] = (text.color[2] * 255.0) as u8;
                            rgba_data[idx + 1] = (text.color[1] * 255.0) as u8;
                            rgba_data[idx + 2] = (text.color[0] * 255.0) as u8;
                            rgba_data[idx + 3] = alpha;
                        }
                    });
                }
                caret_x += scaled.h_advance(id);
                prev = Some(id);
            }
        }
        
        let image = self.create_image(Pt::from(text_width as f32), Pt::from(text_height as f32), &rgba_data)?;
        Ok(self.images.get(image.index()).and_then(|e| e.as_ref()).copied())
    }

    fn get_cached_font(&self, font_hash: u64) -> Option<FontArc> {
        self.font_cache.get(&font_hash).cloned()
    }

    fn cache_font(&mut self, font_hash: u64, font: FontArc) {
        self.font_cache.insert(font_hash, font);
    }

    fn resolve_drawables(
        &mut self,
        drawables: &[DrawCommand],
        logical_w: u32,
        logical_h: u32,
    ) {
        self.resolved_draws.clear();
        let viewport_rect = [0.0, 0.0, logical_w as f32, logical_h as f32];

        for drawable in drawables {
            match drawable {
                DrawCommand::Image(id, opts, shader_id, shader_opts) => {
                    if let Some(Some(entry)) = self.images.get(*id as usize) {
                        if !entry.visible {
                            continue;
                        }

                        let pos = opts.position();
                        let scale = opts.scale();
                        let w = entry.bounds.width.as_f32() * scale[0];
                        let h = entry.bounds.height.as_f32() * scale[1];

                        if pos[0].as_f32() + w < 0.0
                            || pos[0].as_f32() > viewport_rect[2]
                            || pos[1].as_f32() + h < 0.0
                            || pos[1].as_f32() > viewport_rect[3]
                        {
                            if opts.rotation() == 0.0 {
                                continue;
                            }
                        }

                        self.resolved_draws.push(ResolvedDraw {
                            img_entry: *entry,
                            opts: *opts,
                            shader_id: *shader_id,
                            shader_opts: *shader_opts,
                        });
                    }
                }
                DrawCommand::Text(text, opts) => {
                    let key = TextImageKey {
                        content: text.content.clone(),
                        font_hash: text.font_hash,
                        font_size_bits: text.font_size.as_f32().to_bits(),
                        color_bits: [
                            text.color[0].to_bits(),
                            text.color[1].to_bits(),
                            text.color[2].to_bits(),
                            text.color[3].to_bits(),
                        ],
                        max_width_bits: text.max_width.map(|w| w.as_f32().to_bits()),
                    };

                    let entry = if let Some(cached_entry) = self.text_image_cache.get(&key) {
                        *cached_entry
                    } else if let Ok(Some(new_entry)) = self.render_text_to_image(text, opts) {
                        self.text_image_cache.insert(key, new_entry);
                        new_entry
                    } else {
                        continue;
                    };

                    let pos = opts.position();
                    let scale = opts.scale();
                    let w = entry.bounds.width.as_f32() * scale[0];
                    let h = entry.bounds.height.as_f32() * scale[1];
                    if pos[0].as_f32() + w < 0.0
                        || pos[0].as_f32() > viewport_rect[2]
                        || pos[1].as_f32() + h < 0.0
                        || pos[1].as_f32() > viewport_rect[3]
                    {
                        if opts.rotation() == 0.0 {
                            continue;
                        }
                    }

                    self.resolved_draws.push(ResolvedDraw {
                        img_entry: entry,
                        opts: *opts,
                        shader_id: 0,
                        shader_opts: ShaderOpts::default(),
                    });
                }
            }
        }
    }

    fn render_batches<'a>(
        &'a mut self,
        rpass: &mut wgpu::RenderPass<'a>,
        screen_size_data: [f32; 4],
        sf: f64,
    ) {
        let mut current_opacity = 1.0f32;

        // Upload initial engine globals
        let engine_globals = crate::image_raw::EngineGlobals {
            screen: screen_size_data,
            opacity: current_opacity,
            _padding: [0.0; 3],
        };
        let mut current_engine_globals_offset = self
            .image_renderer
            .upload_engine_globals(&self.queue, &engine_globals)
            .unwrap_or(0);

        let mut default_user_globals = ShaderOpts::default();
        default_user_globals.set_opacity(1.0);
        let mut current_user_globals_offset = self
            .image_renderer
            .upload_user_globals_bytes(&self.queue, default_user_globals.as_bytes())
            .unwrap_or(0);

        self.batch.clear();
        let mut current_atlas_index: Option<u32> = None;
        let mut current_shader_id: u32 = 0;
        let mut current_user_globals = ShaderOpts::default();
        current_user_globals.set_opacity(1.0);
        let mut current_clip: Option<[Pt; 4]> = None;

        let config_width = self.config.width;
        let config_height = self.config.height;

        rpass.set_scissor_rect(0, 0, config_width.max(1), config_height.max(1));
        let mut last_set_scissor: Option<(u32, u32, u32, u32)> = None;

        for i in 0..self.resolved_draws.len() {
            let resolved = self.resolved_draws[i];
            let img_entry = resolved.img_entry;
            let opts = resolved.opts;
            let shader_id = resolved.shader_id;
            let shader_opts = resolved.shader_opts;

            let effective_user_globals = shader_opts;
            let draw_opacity = opts.opacity();

            let state_changed = current_atlas_index != Some(img_entry.atlas_index)
                || current_shader_id != shader_id
                || current_user_globals != effective_user_globals
                || current_clip != opts.get_clip()
                || current_opacity != draw_opacity;

            if state_changed && !self.batch.is_empty() {
                let ai = current_atlas_index.unwrap();
                let atlas_bg = &self.atlases.get(ai as usize).expect("atlas").bind_group;

                if let Ok(range) = self.image_renderer.upload_instances(&self.queue, self.batch.as_slice()) {
                    let pipeline = if current_shader_id == 0 {
                        &self.default_pipeline
                    } else {
                        self.image_pipelines.get(&current_shader_id).unwrap()
                    };
                    self.image_renderer.draw_batch(
                        rpass,
                        pipeline,
                        atlas_bg,
                        range,
                        current_user_globals_offset,
                        current_engine_globals_offset,
                    );
                }
                self.batch.clear();
            }

            if current_opacity != draw_opacity {
                current_opacity = draw_opacity;
                let eg = crate::image_raw::EngineGlobals {
                    screen: screen_size_data,
                    opacity: current_opacity,
                    _padding: [0.0; 3],
                };
                current_engine_globals_offset = self.image_renderer
                    .upload_engine_globals(&self.queue, &eg)
                    .unwrap_or(0);
            }

            if current_user_globals != effective_user_globals || (current_atlas_index.is_none() && self.batch.is_empty()) {
                current_user_globals = effective_user_globals;
                if std::env::var("SPOT_DEBUG_SHADER").is_ok() {
                    let b = current_user_globals.as_bytes();
                    let x0 = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                    eprintln!(
                        "[spot][debug][shader] upload user_globals[0].x={:.3} shader_id={}",
                        x0,
                        shader_id
                    );
                }
                current_user_globals_offset = self.image_renderer
                    .upload_user_globals_bytes(&self.queue, current_user_globals.as_bytes())
                    .unwrap_or(current_user_globals_offset);
            }

            if current_clip != opts.get_clip() {
                current_clip = opts.get_clip();
                let (sx, sy, sw, sh) = if let Some(clip) = current_clip {
                    let x0 = (clip[0].as_f32() * sf as f32).clamp(0.0, config_width as f32);
                    let y0 = (clip[1].as_f32() * sf as f32).clamp(0.0, config_height as f32);
                    let x1 = ((clip[0].as_f32() + clip[2].as_f32()) * sf as f32)
                        .clamp(0.0, config_width as f32);
                    let y1 = ((clip[1].as_f32() + clip[3].as_f32()) * sf as f32)
                        .clamp(0.0, config_height as f32);
                    let fw = (x1 - x0).max(0.0) as u32;
                    let fh = (y1 - y0).max(0.0) as u32;
                    if fw > 0 && fh > 0 {
                        (x0 as u32, y0 as u32, fw, fh)
                    } else {
                        (0, 0, 1, 1)
                    }
                } else {
                    (0, 0, config_width, config_height)
                };

                if last_set_scissor != Some((sx, sy, sw, sh)) {
                    rpass.set_scissor_rect(sx, sy, sw, sh);
                    last_set_scissor = Some((sx, sy, sw, sh));
                }
            }

            current_atlas_index = Some(img_entry.atlas_index);
            current_shader_id = shader_id;

            self.batch.push(InstanceData {
                pos: [opts.position()[0].as_f32(), opts.position()[1].as_f32()],
                rotation: opts.rotation(),
                size: [
                    img_entry.bounds.width.as_f32() * opts.scale()[0],
                    img_entry.bounds.height.as_f32() * opts.scale()[1],
                ],
                uv_rect: img_entry.uv_rect,
            });
        }

        if !self.batch.is_empty() {
            let ai = current_atlas_index.unwrap();
            let atlas_bg = &self.atlases.get(ai as usize).expect("atlas").bind_group;
            if let Ok(range) = self.image_renderer.upload_instances(&self.queue, self.batch.as_slice()) {
                let pipeline = if current_shader_id == 0 {
                    &self.default_pipeline
                } else {
                    self.image_pipelines.get(&current_shader_id).unwrap()
                };
                self.image_renderer.draw_batch(
                    rpass,
                    pipeline,
                    atlas_bg,
                    range,
                    current_user_globals_offset,
                    current_engine_globals_offset,
                );
            }
            self.batch.clear();
        }
    }

    pub fn draw_context(
        &mut self,
        surface: &wgpu::Surface<'_>,
        context: &Context,
    ) -> Result<(), wgpu::SurfaceError> {
        self.draw_drawables_with_context(surface, context.draw_list(), context.scale_factor(), context)
    }

    fn draw_drawables_with_context(
        &mut self,
        surface: &wgpu::Surface<'_>,
        drawables: &[DrawCommand],
        scale_factor: f64,
        context: &Context,
    ) -> Result<(), wgpu::SurfaceError> {
        let (lw, lh) = context.window_logical_size();
        let sf = if scale_factor.is_finite() && scale_factor > 0.0 { scale_factor } else { 1.0 };
        let expected_w = ((lw.as_f32() as f64) * sf).round().max(1.0) as u32;
        let expected_h = ((lh.as_f32() as f64) * sf).round().max(1.0) as u32;
        if expected_w != self.config.width || expected_h != self.config.height {
            self.resize(surface, expected_w, expected_h);
        }
        self.draw_drawables_internal(surface, drawables, sf, Some(context))
    }

    fn draw_drawables_internal(
        &mut self,
        surface: &wgpu::Surface<'_>,
        drawables: &[DrawCommand],
        scale_factor: f64,
        _context: Option<&Context>,
    ) -> Result<(), wgpu::SurfaceError> {
        let profile_enabled = *PROFILE_RENDER.get_or_init(|| {
            std::env::var("SPOT_PROFILE_RENDER")
                .ok()
                .map(|v| {
                    let v = v.trim().to_ascii_lowercase();
                    !v.is_empty() && v != "0" && v != "false" && v != "off"
                })
                .unwrap_or(false)
        });

        let mut t_prev = if profile_enabled { Some(Instant::now()) } else { None };
        let frame = surface.get_current_texture()?;
        let dt_acquire_ms = if let Some(t0) = t_prev {
            t0.elapsed().as_secs_f64() * 1000.0
        } else {
            0.0
        };
        t_prev = if profile_enabled { Some(Instant::now()) } else { None };

        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("graphics_encoder"),
        });
        let dt_encoder_ms = if let Some(t0) = t_prev {
            t0.elapsed().as_secs_f64() * 1000.0
        } else {
            0.0
        };
        t_prev = if profile_enabled { Some(Instant::now()) } else { None };

        self.image_renderer.begin_frame();
        let sf = if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor
        } else {
            1.0
        };
        let logical_w = ((self.config.width as f64) / sf).round().max(1.0) as u32;
        let logical_h = ((self.config.height as f64) / sf).round().max(1.0) as u32;
        self.text_renderer.begin_frame(logical_w, logical_h, &self.queue);

        let (sw, sh) = (logical_w as f32, logical_h as f32);
        let sw_inv = 1.0 / sw;
        let sh_inv = 1.0 / sh;
        let screen_size_data = [sw_inv * 2.0, sh_inv * 2.0, sw_inv, sh_inv];

        self.resolve_drawables(drawables, logical_w, logical_h);

        let dt_setup_ms = if let Some(t0) = t_prev {
            t0.elapsed().as_secs_f64() * 1000.0
        } else {
            0.0
        };
        t_prev = if profile_enabled { Some(Instant::now()) } else { None };

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("graphics_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.render_batches(&mut rpass, screen_size_data, sf);
        }

        let dt_renderpass_ms = if let Some(t0) = t_prev {
            t0.elapsed().as_secs_f64() * 1000.0
        } else {
            0.0
        };
        t_prev = if profile_enabled { Some(Instant::now()) } else { None };
        self.queue.submit(Some(encoder.finish()));
        let dt_submit_ms = if let Some(t0) = t_prev {
            t0.elapsed().as_secs_f64() * 1000.0
        } else {
            0.0
        };
        frame.present();

        if profile_enabled {
            let total_ms =
                dt_acquire_ms + dt_encoder_ms + dt_setup_ms + dt_renderpass_ms + dt_submit_ms;
            let wait_ms = dt_acquire_ms;
            let work_ms = total_ms - wait_ms;

            static PROFILE_STATS: OnceLock<Mutex<RenderProfileStats>> = OnceLock::new();
            let stats_lock =
                PROFILE_STATS.get_or_init(|| Mutex::new(RenderProfileStats::default()));
            if let Ok(mut s) = stats_lock.lock() {
                s.frame = s.frame.saturating_add(1);
                s.sum_total_ms += total_ms;
                s.sum_wait_ms += wait_ms;
                s.sum_work_ms += work_ms;
                s.min_total_ms = s.min_total_ms.min(total_ms);
                s.max_total_ms = s.max_total_ms.max(total_ms);

                if s.frame % 30 == 0 {
                    let n = s.frame as f64;
                    eprintln!(
                        "[spot][render][avg@{}] total={:.3}ms work={:.3} wait={:.3} min={:.3} max={:.3}",
                        s.frame,
                        s.sum_total_ms / n,
                        s.sum_work_ms / n,
                        s.sum_wait_ms / n,
                        s.min_total_ms,
                        s.max_total_ms
                    );
                }
            }
        }
        Ok(())
    }

    pub(crate) fn copy_image(&mut self, dst: Image, src: Image) -> anyhow::Result<()> {
        let (dst_atlas_index, dst_rect, dst_uv_rect) = {
             let Some(Some(d)) = self.images.get(dst.index()) else { return Err(anyhow::anyhow!("invalid dst image")); };
            (d.atlas_index, d.bounds, d.uv_rect)
        };
         let (src_atlas_index, src_rect, src_uv_rect) = {
             let Some(Some(s)) = self.images.get(src.index()) else { return Err(anyhow::anyhow!("invalid src image")); };
            (s.atlas_index, s.bounds, s.uv_rect)
        };
        if dst_atlas_index != src_atlas_index { return Err(anyhow::anyhow!("copy_image across atlases is not supported")); }
        if dst_rect.width != src_rect.width || dst_rect.height != src_rect.height { return Err(anyhow::anyhow!("size mismatch")); }
        
        let atlas = self.atlases.get(dst_atlas_index as usize).expect("atlas");
        let aw = atlas.packer.width() as f32;
        let ah = atlas.packer.height() as f32;
        
        let src_x = (src_uv_rect[0] * aw).round() as u32;
        let src_y = (src_uv_rect[1] * ah).round() as u32;
        let dst_x = (dst_uv_rect[0] * aw).round() as u32;
        let dst_y = (dst_uv_rect[1] * ah).round() as u32;
        let w = dst_rect.width.to_u32_clamped();
        let h = dst_rect.height.to_u32_clamped();

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("graphics_copy_image_encoder") });
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo { texture: &atlas.texture.0.texture, mip_level: 0, origin: wgpu::Origin3d { x: src_x, y: src_y, z: 0 }, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyTextureInfo { texture: &atlas.texture.0.texture, mip_level: 0, origin: wgpu::Origin3d { x: dst_x, y: dst_y, z: 0 }, aspect: wgpu::TextureAspect::All },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 }
        );
        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }

    pub(crate) fn clear_image(&mut self, target: Image, color: [f32; 4]) -> anyhow::Result<()> {
        let bounds = self.image_bounds(target)?;
        let w = bounds.width.to_u32_clamped();
        let h = bounds.height.to_u32_clamped();
        let pixel = [(color[0] * 255.0) as u8, (color[1] * 255.0) as u8, (color[2] * 255.0) as u8, (color[3] * 255.0) as u8];
        let data: Vec<u8> = pixel.repeat((w * h) as usize);
        let entry = self.images.get(target.index()).unwrap().as_ref().unwrap();
        let atlas = self.atlases.get(entry.atlas_index as usize).expect("atlas");
        let aw = atlas.packer.width() as f32;
        let ah = atlas.packer.height() as f32;
        let x = (entry.uv_rect[0] * aw).round() as u32;
        let y = (entry.uv_rect[1] * ah).round() as u32;
        let bytes_per_row = 4 * w;
        let (data, bytes_per_row) = platform::align_write_texture_bytes(bytes_per_row, h, data);

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &atlas.texture.0.texture, mip_level: 0, origin: wgpu::Origin3d { x: x, y: y, z: 0 }, aspect: wgpu::TextureAspect::All },
            &data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bytes_per_row), rows_per_image: Some(h) },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 }
        );
        Ok(())
    }
}
