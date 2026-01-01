use crate::{Context, DrawCommand, DrawOption};
use crate::image::{Bounds, Image, ImageEntry};
use crate::image_raw::{ImageRenderer, InstanceData};
use crate::packer::AtlasPacker;
use crate::platform;
use crate::texture::Texture;
use crate::text_renderer::TextRenderer;
use crate::pt::Pt;
use crate::ShaderOpts;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Instant;
use std::sync::Mutex;

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

#[inline]
fn mvp_from_draw_options(
    sw_inv: f32,
    sh_inv: f32,
    sw_inv_2: f32,
    sh_inv_2: f32,
    base_w_px: f32,
    base_h_px: f32,
    opts: DrawOption,
) -> ([f32; 2], [f32; 2], [f32; 2]) {
    // `position` is the desired top-left corner in screen pixels (origin at top-left).
    let pos = opts.position();
    let (px, py) = (pos[0].as_f32(), pos[1].as_f32());
    let (w_px, h_px) = (
        base_w_px * opts.scale()[0],
        base_h_px * opts.scale()[1],
    );

    // Target top-left in clip-space.
    let tx = px * sw_inv_2 - 1.0;
    let ty = 1.0 - py * sh_inv_2;

    // Our quad is in local space [-1, 1]. Width/height = 2.
    // To get a clip-space width of (w_px / sw) * 2, we need sx = w_px * sw_inv.
    let sx = w_px * sw_inv;
    let sy = h_px * sh_inv;

    if opts.rotation() == 0.0 {
        // Fast path for no rotation.
        // With rotation=0, c=1, s=0.
        return ([sx, 0.0], [0.0, sy], [tx + sx, ty - sy]);
    }

    let r = opts.rotation();
    let (c, s) = (r.cos(), r.sin());

    // Anchor is local top-left corner of the quad.
    // With our vertex layout, top-left is (-1, +1).
    let p_tl_x = -1.0;
    let p_tl_y = 1.0;

    // Compute where the local top-left ends up after R*S, then choose translation so it lands on (tx,ty).
    // v = R * (S * p)
    let spx = sx * p_tl_x;
    let spy = sy * p_tl_y;
    let v_tl_x = c * spx - s * spy;
    let v_tl_y = s * spx + c * spy;

    let dx = tx - v_tl_x;
    let dy = ty - v_tl_y;

    // Column-major affine matrix used by WGSL (mvp * vec4(pos,0,1)).
    // MVP = T * R * S with T chosen so that (-1,+1) maps to (tx,ty).
    ([c * sx, s * sx], [-s * sy, c * sy], [dx, dy])
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

fn flush_image_batch<'a>(
    batch: &mut Vec<InstanceData>,
    rpass: &mut wgpu::RenderPass<'a>,
    image_renderer: &mut ImageRenderer,
    queue: &wgpu::Queue,
    pipeline: &'a wgpu::RenderPipeline,
    atlas_bg: &wgpu::BindGroup,
    globals_offset: u32,
) {
    if batch.is_empty() {
        return;
    }
    let range_opt = match image_renderer.upload_instances(queue, batch.as_slice()) {
        Ok(r) => Some(r),
        Err(_) => None,
    };
    if let Some(range) = range_opt {
        image_renderer.draw_batch(rpass, pipeline, atlas_bg, range, globals_offset);
    }
    batch.clear();
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
    image_pipelines: HashMap<u32, wgpu::RenderPipeline>,
    next_image_shader_id: u32,
    images: Vec<Option<ImageEntry>>,
    text_renderer: TextRenderer,

    atlases: Vec<AtlasSlot>,

    // Reuse vector to avoid allocation
    batch: Vec<InstanceData>,
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
        let profile_enabled = *PROFILE_RENDER.get_or_init(|| {
            std::env::var("SPOT_PROFILE_RENDER")
                .ok()
                .map(|v| {
                    let v = v.trim().to_ascii_lowercase();
                    !v.is_empty() && v != "0" && v != "false" && v != "off"
                })
                .unwrap_or(false)
        });
        if profile_enabled {
            eprintln!(
                "[spot][present] supported={:?} chosen={:?}",
                surface_caps.present_modes,
                present_mode
            );
        }

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

        Ok(Self {
            device,
            queue,
            config,
            image_renderer,
            image_pipelines,
            next_image_shader_id,
            images: Vec::new(),
            text_renderer,
            atlases,
            batch: Vec::with_capacity(10000),
        })
    }

    pub(crate) fn register_image_shader(&mut self, wgsl_source: &str) -> u32 {
        let shader_id = self.next_image_shader_id;
        self.next_image_shader_id = self.next_image_shader_id.saturating_add(1);

        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("custom_image_shader"),
            source: wgpu::ShaderSource::Wgsl(wgsl_source.into()),
        });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("custom_image_pipeline_layout"),
                bind_group_layouts: &[
                    &self.image_renderer.texture_bind_group_layout,
                    &self.image_renderer.globals_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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

        self.image_pipelines.insert(shader_id, pipeline);
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

        let uv_param = atlas.packer.get_uv_param(&rect);
        // uv_param: [u0, v0, u_width, v_height]

        let uv_rect = [uv_param[0], uv_param[1], uv_param[2], uv_param[3]];

        let bounds = Bounds::new(Pt(0.0), Pt(0.0), width, height);

        let entry = ImageEntry::new(atlas_index, bounds, uv_rect);
        Ok(self.insert_image_entry(entry))
    }
    
    pub(crate) fn create_sub_image(&mut self, image: Image, bounds: Bounds) -> anyhow::Result<Image> {
        let parent_entry = self.images.get(image.index())
            .and_then(|v| v.as_ref())
            .ok_or_else(|| anyhow::anyhow!("Invalid parent image"))?;
            
        // Parent uv_rect: [u0, v0, w, h]
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
        let sf = if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor
        } else {
            1.0
        };
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
        static PROFILE_STATS: OnceLock<Mutex<RenderProfileStats>> = OnceLock::new();

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
        let mut dt_acquire_ms: f64 = 0.0;
        let mut dt_encoder_ms: f64 = 0.0;
        let mut dt_setup_ms: f64 = 0.0;
        let mut dt_renderpass_ms: f64 = 0.0;
        let mut dt_submit_ms: f64 = 0.0;
        let mut dt_present_ms: f64 = 0.0;

        let frame = surface.get_current_texture()?;
        if let Some(t0) = t_prev {
            dt_acquire_ms = t0.elapsed().as_secs_f64() * 1000.0;
            t_prev = Some(Instant::now());
        }
        let view: wgpu::TextureView = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("graphics_encoder"),
            });
        if let Some(t0) = t_prev {
            dt_encoder_ms = t0.elapsed().as_secs_f64() * 1000.0;
            t_prev = Some(Instant::now());
        }

        self.image_renderer.begin_frame();
        let scale_factor = if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor
        } else {
            1.0
        };

        let logical_w = ((self.config.width as f64) / scale_factor).round().max(1.0) as u32;
        let logical_h = ((self.config.height as f64) / scale_factor).round().max(1.0) as u32;

        self.text_renderer.begin_frame(logical_w, logical_h, &self.queue);
        let (sw, sh) = (logical_w as f32, logical_h as f32);
        let sw_inv = 1.0 / sw;
        let sh_inv = 1.0 / sh;
        let sw_inv_2 = sw_inv * 2.0;
        let sh_inv_2 = sh_inv * 2.0;

        if let Some(t0) = t_prev {
            dt_setup_ms = t0.elapsed().as_secs_f64() * 1000.0;
            t_prev = Some(Instant::now());
        }

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

            self.batch.clear();
            self.batch.reserve(drawables.len());
            // Set initial scissor to full screen
            rpass.set_scissor_rect(0, 0, self.config.width.max(1), self.config.height.max(1));
            let mut current_clip: Option<[Pt; 4]> = None;

            let mut batch = std::mem::take(&mut self.batch);
            let (image_renderer, queue, atlases) = (&mut self.image_renderer, &self.queue, &self.atlases);

            let mut current_atlas_index: Option<u32> = None;
            let mut current_pipeline: Option<*const wgpu::RenderPipeline> = None;
            let mut current_globals: ShaderOpts = ShaderOpts::default();
            let mut current_globals_offset: u32 = image_renderer
                .upload_globals_bytes(queue, current_globals.as_bytes())
                .unwrap_or(0);

            for drawable in drawables {
                match drawable {
                    DrawCommand::Image(id, opts, shader_id, shader_opts) => {
                        let Some(Some(img)) = self.images.get(*id as usize) else {
                            continue;
                        };
                        if !img.visible {
                            continue;
                        }

                        let effective_globals = *shader_opts;

                        let pipeline_ptr: *const wgpu::RenderPipeline = if *shader_id == 0 {
                            std::ptr::addr_of!(image_renderer.pipeline)
                        } else {
                            let Some(p) = self.image_pipelines.get(shader_id) else {
                                continue;
                            };
                            p as *const wgpu::RenderPipeline
                        };

                        if current_clip != opts.clip() 
                            || (current_atlas_index.is_some() && current_atlas_index != Some(img.atlas_index))
                            || (current_pipeline.is_some() && current_pipeline != Some(pipeline_ptr))
                            || current_globals != effective_globals
                        {
                            if let Some(ai) = current_atlas_index {
                                let atlas_bg = &atlases.get(ai as usize).expect("atlas").bind_group;
                                let prev_pipeline_ptr = current_pipeline
                                    .unwrap_or_else(|| std::ptr::addr_of!(image_renderer.pipeline));
                                flush_image_batch(
                                    &mut batch,
                                    &mut rpass,
                                    image_renderer,
                                    queue,
                                    unsafe { &*prev_pipeline_ptr },
                                    atlas_bg,
                                    current_globals_offset,
                                );
                            }

                            if current_globals != effective_globals {
                                current_globals = effective_globals;
                                current_globals_offset = image_renderer
                                    .upload_globals_bytes(queue, current_globals.as_bytes())
                                    .unwrap_or(current_globals_offset);
                            }

                            if current_clip != opts.clip() {
                                current_clip = opts.clip();
                                if let Some(clip) = current_clip {
                                    let cx = (clip[0].as_f32() * scale_factor as f32).max(0.0) as u32;
                                    let cy = (clip[1].as_f32() * scale_factor as f32).max(0.0) as u32;
                                    let cw = (clip[2].as_f32() * scale_factor as f32).max(0.0) as u32;
                                    let ch = (clip[3].as_f32() * scale_factor as f32).max(0.0) as u32;
                                    
                                    let sw = self.config.width;
                                    let sh = self.config.height;
                                    
                                    let final_x = cx.min(sw);
                                    let final_y = cy.min(sh);
                                    let final_w = cw.min(sw - final_x);
                                    let final_h = ch.min(sh - final_y);
                                    
                                    if final_w > 0 && final_h > 0 {
                                        rpass.set_scissor_rect(final_x, final_y, final_w, final_h);
                                    } else {
                                        rpass.set_scissor_rect(0, 0, 1, 1); 
                                    }
                                } else {
                                    rpass.set_scissor_rect(0, 0, self.config.width, self.config.height);
                                }
                            }
                        }
                        current_atlas_index = Some(img.atlas_index);
                        current_pipeline = Some(pipeline_ptr);

                        let base_w_px = img.bounds.width.0.max(0.0);
                        let base_h_px = img.bounds.height.0.max(0.0);
                        let (mvp_col0, mvp_col1, mvp_col3) = mvp_from_draw_options(sw_inv, sh_inv, sw_inv_2, sh_inv_2, base_w_px, base_h_px, *opts);
                        batch.push(InstanceData {
                            mvp_col0,
                            mvp_col1,
                            mvp_col3,
                            uv_rect: img.uv_rect,
                        });
                    }
                    DrawCommand::Text(text, opts) => {
                        if current_clip != opts.clip() || current_atlas_index.is_some() {
                            if let Some(ai) = current_atlas_index {
                                let atlas_bg = &atlases.get(ai as usize).expect("atlas").bind_group;
                                let prev_pipeline_ptr = current_pipeline
                                    .unwrap_or_else(|| std::ptr::addr_of!(image_renderer.pipeline));
                                flush_image_batch(
                                    &mut batch,
                                    &mut rpass,
                                    image_renderer,
                                    queue,
                                    unsafe { &*prev_pipeline_ptr },
                                    atlas_bg,
                                    current_globals_offset,
                                );
                            }
                            current_atlas_index = None;
                            current_pipeline = None;

                            if current_clip != opts.clip() {
                                current_clip = opts.clip();
                                if let Some(clip) = current_clip {
                                    let cx = (clip[0].as_f32() * scale_factor as f32).max(0.0) as u32;
                                    let cy = (clip[1].as_f32() * scale_factor as f32).max(0.0) as u32;
                                    let cw = (clip[2].as_f32() * scale_factor as f32).max(0.0) as u32;
                                    let ch = (clip[3].as_f32() * scale_factor as f32).max(0.0) as u32;
                                    
                                    let sw = self.config.width;
                                    let sh = self.config.height;
                                    
                                    let final_x = cx.min(sw);
                                    let final_y = cy.min(sh);
                                    let final_w = cw.min(sw - final_x);
                                    let final_h = ch.min(sh - final_y);
                                    
                                    if final_w > 0 && final_h > 0 {
                                        rpass.set_scissor_rect(final_x, final_y, final_w, final_h);
                                    } else {
                                        rpass.set_scissor_rect(0, 0, 1, 1);
                                    }
                                } else {
                                    rpass.set_scissor_rect(0, 0, self.config.width, self.config.height);
                                }
                            }
                        }

                        self.text_renderer
                            .queue_text(text, opts, &self.queue)
                            .expect("Text draw requires valid font_data");
                    }
                }
            }

            if let Some(ai) = current_atlas_index {
                let atlas_bg = &atlases.get(ai as usize).expect("atlas").bind_group;
                let prev_pipeline_ptr = current_pipeline
                    .unwrap_or_else(|| std::ptr::addr_of!(image_renderer.pipeline));
                flush_image_batch(
                    &mut batch,
                    &mut rpass,
                    image_renderer,
                    queue,
                    unsafe { &*prev_pipeline_ptr },
                    atlas_bg,
                    current_globals_offset,
                );
            }

            self.batch = batch;

            self.text_renderer
                .flush(&self.device, &mut rpass, &self.queue);
        }

        if let Some(t0) = t_prev {
            dt_renderpass_ms = t0.elapsed().as_secs_f64() * 1000.0;
            t_prev = Some(Instant::now());
        }

        self.queue.submit(Some(encoder.finish()));

        if let Some(t0) = t_prev {
            dt_submit_ms = t0.elapsed().as_secs_f64() * 1000.0;
            t_prev = Some(Instant::now());
        }

        frame.present();

        if let Some(t0) = t_prev {
            dt_present_ms = t0.elapsed().as_secs_f64() * 1000.0;
        }

        if profile_enabled {
            let total_ms = dt_acquire_ms
                + dt_encoder_ms
                + dt_setup_ms
                + dt_renderpass_ms
                + dt_submit_ms
                + dt_present_ms;
            let wait_ms = dt_acquire_ms;
            let work_ms = total_ms - wait_ms;
            eprintln!(
                "[spot][render] total={:.3}ms work={:.3} wait={:.3} acquire={:.3} encoder={:.3} setup={:.3} renderpass={:.3} submit={:.3} present={:.3}",
                total_ms,
                work_ms,
                wait_ms,
                dt_acquire_ms,
                dt_encoder_ms,
                dt_setup_ms,
                dt_renderpass_ms,
                dt_submit_ms,
                dt_present_ms
            );

            let stats_lock = PROFILE_STATS.get_or_init(|| Mutex::new(RenderProfileStats::default()));
            if let Ok(mut s) = stats_lock.lock() {
                s.frame = s.frame.saturating_add(1);
                s.sum_total_ms += total_ms;
                s.sum_wait_ms += wait_ms;
                s.sum_work_ms += work_ms;
                if total_ms < s.min_total_ms {
                    s.min_total_ms = total_ms;
                }
                if total_ms > s.max_total_ms {
                    s.max_total_ms = total_ms;
                }

                if s.frame % 60 == 0 {
                    let n = s.frame as f64;
                    let avg_total = s.sum_total_ms / n;
                    let avg_wait = s.sum_wait_ms / n;
                    let avg_work = s.sum_work_ms / n;
                    eprintln!(
                        "[spot][render][avg@{}] total={:.3}ms work={:.3} wait={:.3} min={:.3} max={:.3}",
                        s.frame,
                        avg_total,
                        avg_work,
                        avg_wait,
                        s.min_total_ms,
                        s.max_total_ms
                    );
                }
            }
        }
        Ok(())
    }

    pub(crate) fn copy_image(&mut self, dst: Image, src: Image) -> anyhow::Result<()> {
        // Copying within the atlas is more complex if we want to copy pixel data.
        // For now, if "copy_image" implies copying pixel data on GPU, we can use copy_texture_to_texture
        // but we need to know the locations in the atlas.
        
        let (dst_atlas_index, dst_rect, dst_uv_rect) = {
             let Some(Some(d)) = self.images.get(dst.index()) else {
                return Err(anyhow::anyhow!("invalid dst image"));
            };
            (d.atlas_index, d.bounds, d.uv_rect)
        };
        
         let (src_atlas_index, src_rect, src_uv_rect) = {
             let Some(Some(s)) = self.images.get(src.index()) else {
                return Err(anyhow::anyhow!("invalid src image"));
            };
            (s.atlas_index, s.bounds, s.uv_rect)
        };

        if dst_atlas_index != src_atlas_index {
            return Err(anyhow::anyhow!("copy_image across atlases is not supported"));
        }

        if dst_rect.width != src_rect.width || dst_rect.height != src_rect.height {
             return Err(anyhow::anyhow!("size mismatch"));
        }
        
        // Calculate offsets in atlas from UVP
        // u0 = uv_rect[0], v0 = uv_rect[1]
        // atlas_x = u0 * atlas_width
        // atlas_y = v0 * atlas_height
        
        let atlas = self
            .atlases
            .get(dst_atlas_index as usize)
            .expect("atlas");
        let aw = atlas.packer.width() as f32;
        let ah = atlas.packer.height() as f32;
        
        // Wait, AtlasPacker struct has width/height but maybe not public?
        // Let's assume we can get it or compute it.
        // Actually, we can just use the uvp to find the texel coordinates.
        // But wait, uvp includes padding if we set it up that way?
        // Our create_image sets up UVP based on get_uv_param which handles padding.
        // So the UVP points to the content.
        
        let src_u0 = src_uv_rect[0];
        let src_v0 = src_uv_rect[1];
        let dst_u0 = dst_uv_rect[0];
        let dst_v0 = dst_uv_rect[1];
        
        let src_x = (src_u0 * aw).round() as u32;
        let src_y = (src_v0 * ah).round() as u32;
        let dst_x = (dst_u0 * aw).round() as u32;
        let dst_y = (dst_v0 * ah).round() as u32;
        let w = dst_rect.width.to_u32_clamped();
        let h = dst_rect.height.to_u32_clamped();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("graphics_copy_image_encoder"),
            });

        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas.texture.0.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: src_x,
                    y: src_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &atlas.texture.0.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: dst_x,
                    y: dst_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }

    pub(crate) fn clear_image(&mut self, target: Image, color: [f32; 4]) -> anyhow::Result<()> {
        // Clearing a region in the atlas is hard with render pass loadOp::Clear because it clears the whole attachment.
        // We must use a draw call to clear a specific region, or write_texture with a solid color buffer.
        // For simplicity and performance of small updates, write_texture might be okay, or a draw call with a solid color shader.
        // However, we are in `clear_image` which might be used as "fill with color".
        
        // For now, let's implement via write_texture (CPU -> GPU) which is slow but simple,
        // OR we can't easily do it efficiently without a "clear" shader or "fill rect" capability.
        // Given constraints, maybe just write texture.
        
        let bounds = self.image_bounds(target)?;
        let w = bounds.width.to_u32_clamped();
        let h = bounds.height.to_u32_clamped();
        
        let r = (color[0] * 255.0) as u8;
        let g = (color[1] * 255.0) as u8;
        let b = (color[2] * 255.0) as u8;
        let a = (color[3] * 255.0) as u8;
        let pixel = [r, g, b, a];
        let data: Vec<u8> = pixel.repeat((w * h) as usize);
        
        // Find position
        let entry = self.images.get(target.index()).unwrap().as_ref().unwrap();
        let u0 = entry.uv_rect[0];
        let v0 = entry.uv_rect[1];
        let atlas = self
            .atlases
            .get(entry.atlas_index as usize)
            .expect("atlas");
        let aw = atlas.packer.width() as f32;
        let ah = atlas.packer.height() as f32;
        let x = (u0 * aw).round() as u32;
        let y = (v0 * ah).round() as u32;

        let bytes_per_row = 4 * w;
        #[cfg(target_arch = "wasm32")]
        let (data, bytes_per_row) = {
            let align = 256u32;
            let padded = ((bytes_per_row + align - 1) / align) * align;
            if padded == bytes_per_row {
                (data, bytes_per_row)
            } else {
                let mut out = vec![0u8; (padded * h) as usize];
                for row in 0..h {
                    let src_off = (row * bytes_per_row) as usize;
                    let dst_off = (row * padded) as usize;
                    out[dst_off..dst_off + bytes_per_row as usize]
                        .copy_from_slice(&data[src_off..src_off + bytes_per_row as usize]);
                }
                (out, padded)
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        let (data, bytes_per_row) = (data, bytes_per_row);

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas.texture.0.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        Ok(())
    }

}