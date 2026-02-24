//! Batch rendering and draw operations.

use std::sync::Mutex;
use std::time::Instant;

use crate::Context;
use crate::DrawCommand;
use crate::ShaderOpts;
use crate::image_raw::InstanceData;
use crate::pt::Pt;

use super::Graphics;
use super::core::ResolvedDraw;
use super::profile::{PROFILE_RENDER, PROFILE_STATS, RenderProfileStats};

impl Graphics {
    pub(super) fn resolve_drawables(
        &mut self,
        drawables: &[DrawCommand],
        logical_w: u32,
        logical_h: u32,
    ) {
        self.resolved_draws.clear();
        let viewport_rect = [0.0, 0.0, logical_w as f32, logical_h as f32];

        for drawable in drawables {
            match drawable {
                DrawCommand::Image(id, opts, shader_id, shader_opts, _) => {
                    if let Some(Some(entry)) = self.images.get(*id as usize) {
                        if !entry.visible {
                            continue;
                        }

                        self.resolved_draws.push(ResolvedDraw {
                            img_entry: entry.clone(),
                            opts: *opts,
                            shader_id: *shader_id,
                            shader_opts: *shader_opts,
                        });
                    }
                }
                DrawCommand::Text(text, opts) => {
                    if let Err(e) = self.layout_and_queue_text(text, opts, viewport_rect) {
                        eprintln!("[spot] Text layout error: {:?}", e);
                    }
                }
            }
        }
    }

    pub(super) fn render_batches<'a>(
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
            let resolved = &self.resolved_draws[i];
            let img_entry = &resolved.img_entry;
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

                if let Ok(range) = self
                    .image_renderer
                    .upload_instances(&self.queue, self.batch.as_slice())
                {
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
                current_engine_globals_offset = self
                    .image_renderer
                    .upload_engine_globals(&self.queue, &eg)
                    .unwrap_or(0);
            }

            if current_user_globals != effective_user_globals
                || (current_atlas_index.is_none() && self.batch.is_empty())
            {
                current_user_globals = effective_user_globals;
                if std::env::var("SPOT_DEBUG_SHADER").is_ok() {
                    let b = current_user_globals.as_bytes();
                    let x0 = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                    eprintln!(
                        "[spot][debug][shader] upload user_globals[0].x={:.3} shader_id={}",
                        x0, shader_id
                    );
                }
                current_user_globals_offset = self
                    .image_renderer
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
            if let Ok(range) = self
                .image_renderer
                .upload_instances(&self.queue, self.batch.as_slice())
            {
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
        self.draw_drawables_with_context(
            surface,
            context.draw_list(),
            context.scale_factor(),
            context,
        )
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
        let profile_enabled = *PROFILE_RENDER.get_or_init(|| {
            std::env::var("SPOT_PROFILE_RENDER")
                .ok()
                .map(|v| {
                    let v = v.trim().to_ascii_lowercase();
                    !v.is_empty() && v != "0" && v != "false" && v != "off"
                })
                .unwrap_or(false)
        });

        let mut t_prev = if profile_enabled {
            Some(Instant::now())
        } else {
            None
        };
        let frame = surface.get_current_texture()?;
        let dt_acquire_ms = if let Some(t0) = t_prev {
            t0.elapsed().as_secs_f64() * 1000.0
        } else {
            0.0
        };
        t_prev = if profile_enabled {
            Some(Instant::now())
        } else {
            None
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("graphics_encoder"),
            });
        let dt_encoder_ms = if let Some(t0) = t_prev {
            t0.elapsed().as_secs_f64() * 1000.0
        } else {
            0.0
        };
        t_prev = if profile_enabled {
            Some(Instant::now())
        } else {
            None
        };

        self.image_renderer.begin_frame();
        let sf = if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor
        } else {
            1.0
        };
        let logical_w = ((self.config.width as f64) / sf).round().max(1.0) as u32;
        let logical_h = ((self.config.height as f64) / sf).round().max(1.0) as u32;

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
        t_prev = if profile_enabled {
            Some(Instant::now())
        } else {
            None
        };

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
        t_prev = if profile_enabled {
            Some(Instant::now())
        } else {
            None
        };
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
}
