//! Batch rendering and draw operations.

use crate::Context;
use crate::ShaderOpts;
use crate::drawable::DrawCommand;
use crate::image_raw::InstanceData;
use std::collections::HashMap;
use std::time::Instant;

use super::core::{AtlasSlot, Graphics, ResolvedDraw};
use crate::image_raw::ImageRenderer;

pub(crate) struct RenderConfig<'a> {
    pub screen_size_data: [f32; 4],
    pub scale_factor: f32,
    pub atlases: &'a [AtlasSlot],
    pub image_pipelines: &'a HashMap<u32, wgpu::RenderPipeline>,
    pub default_pipeline: &'a wgpu::RenderPipeline,
}

fn expect_image_pipeline<'a>(
    image_pipelines: &'a HashMap<u32, wgpu::RenderPipeline>,
    default_pipeline: &'a wgpu::RenderPipeline,
    shader_id: u32,
) -> &'a wgpu::RenderPipeline {
    if shader_id == 0 {
        default_pipeline
    } else {
        image_pipelines.get(&shader_id).unwrap_or_else(|| {
            panic!(
                "[spot][render] missing image pipeline for shader_id {}",
                shader_id
            )
        })
    }
}

fn expect_atlas_bind_group(atlases: &[AtlasSlot], atlas_index: Option<u32>) -> &wgpu::BindGroup {
    let atlas_index =
        atlas_index.unwrap_or_else(|| panic!("[spot][render] missing atlas index for image batch"));
    &atlases
        .get(atlas_index as usize)
        .unwrap_or_else(|| panic!("[spot][render] missing atlas {}", atlas_index))
        .bind_group
}

impl Graphics {
    fn process_image_commands(&mut self, ctx: &mut Context, drawables: &[DrawCommand]) {
        for command in drawables {
            match command {
                DrawCommand::ClearImage(target_id, color) => {
                    if let Err(e) = self.clear_image(ctx, *target_id, *color) {
                        eprintln!(
                            "[spot][graphics] clear_image failed for {}: {:?}",
                            target_id, e
                        );
                    }
                }
                DrawCommand::CopyImage(dst_id, src_id) => {
                    if let Err(e) = self.copy_image(ctx, *dst_id, *src_id) {
                        eprintln!(
                            "[spot][graphics] copy_image failed from {} to {}: {:?}",
                            src_id, dst_id, e
                        );
                    }
                }
                DrawCommand::Image(_) | DrawCommand::Text(_, _) => {}
            }
        }
    }

    pub(crate) fn resolve_drawables(
        &mut self,
        ctx: &mut Context,
        drawables: &[DrawCommand],
        logical_w: u32,
        logical_h: u32,
    ) {
        self.resolved_draws.clear();
        self.draw_index_counter = 0;
        let viewport_rect = [0.0, 0.0, logical_w as f32, logical_h as f32];

        for drawable in drawables {
            match drawable {
                DrawCommand::Image(cmd) => {
                    if let Some(Some(entry)) = ctx.registry.images.get(cmd.id as usize) {
                        if !entry.visible {
                            continue;
                        }

                        let Some(atlas_index) = entry.atlas_index else {
                            continue;
                        };
                        let Some(uv_rect) = entry.uv_rect else {
                            continue;
                        };

                        let draw_index = self.draw_index_counter;
                        self.draw_index_counter += 1;

                        self.resolved_draws.push(ResolvedDraw {
                            atlas_index,
                            bounds: entry.bounds,
                            uv_rect,
                            opts: cmd.opts,
                            shader_id: cmd.shader_id,
                            shader_opts: cmd.shader_opts,
                            layer: cmd.opts.layer(),
                            draw_index,
                        });
                    }
                }
                DrawCommand::Text(text, opts) => {
                    if let Err(e) = self.layout_and_queue_text(ctx, text, opts, viewport_rect) {
                        eprintln!("[spot] Text layout error: {:?}", e);
                    }
                }
                DrawCommand::ClearImage(_, _) | DrawCommand::CopyImage(_, _) => {}
            }
        }
    }

    pub(crate) fn render_batches_internal<'a>(
        image_renderer: &mut ImageRenderer,
        queue: &wgpu::Queue,
        batch: &mut Vec<InstanceData>,
        resolved_draws: &mut [ResolvedDraw],
        rpass: &mut wgpu::RenderPass<'a>,
        config: RenderConfig<'a>,
    ) {
        // Sort by layer first, then atlas and shader to maximize batching, 
        // finally by draw_index to preserve submission order within the same state.
        resolved_draws.sort_by(|a, b| {
            a.layer
                .cmp(&b.layer)
                .then(a.atlas_index.cmp(&b.atlas_index))
                .then(a.shader_id.cmp(&b.shader_id))
                .then(a.draw_index.cmp(&b.draw_index))
        });

        let mut current_opacity = 1.0f32;

        // Upload initial engine globals
        let engine_globals = crate::image_raw::EngineGlobals {
            screen: config.screen_size_data,
            opacity: current_opacity,
            shader_opacity: 1.0,
            scale_factor: config.scale_factor,
            _padding: [0.0; 1],
        };
        let mut current_engine_globals_offset = image_renderer
            .upload_engine_globals(queue, &engine_globals)
            .unwrap_or(0);

        let default_user_globals = ShaderOpts::default();
        let mut current_user_globals_offset = image_renderer
            .upload_user_globals_bytes(queue, default_user_globals.as_bytes())
            .unwrap_or(0);

        batch.clear();
        let mut current_atlas_index: Option<u32> = None;
        let mut current_shader_id: u32 = 0;
        let mut current_user_globals = ShaderOpts::default();

        for resolved in resolved_draws.iter() {
            let opts = resolved.opts;
            let shader_id = resolved.shader_id;
            let shader_opts = resolved.shader_opts;
            let draw_opacity = opts.opacity();

            let effective_user_globals = shader_opts;

            let state_changed = current_atlas_index != Some(resolved.atlas_index)
                || current_shader_id != shader_id
                || current_user_globals != effective_user_globals
                || current_opacity != draw_opacity;

            if state_changed && !batch.is_empty() {
                if let Ok(range) = image_renderer.upload_instances(queue, batch.as_slice()) {
                    let pipeline = expect_image_pipeline(
                        config.image_pipelines,
                        config.default_pipeline,
                        current_shader_id,
                    );
                    let atlas_bg = expect_atlas_bind_group(config.atlases, current_atlas_index);
                    image_renderer.draw_batch(
                        rpass,
                        pipeline,
                        atlas_bg,
                        range,
                        current_user_globals_offset,
                        current_engine_globals_offset,
                    );
                }
                batch.clear();
            }

            if current_opacity != draw_opacity
                || current_user_globals.opacity != resolved.shader_opts.opacity
            {
                current_opacity = draw_opacity;
                let eg = crate::image_raw::EngineGlobals {
                    screen: config.screen_size_data,
                    opacity: current_opacity,
                    shader_opacity: resolved.shader_opts.opacity,
                    scale_factor: config.scale_factor,
                    _padding: [0.0; 1],
                };
                current_engine_globals_offset = image_renderer
                    .upload_engine_globals(queue, &eg)
                    .unwrap_or(0);
            }

            if current_user_globals != effective_user_globals
                || (current_atlas_index.is_none() && batch.is_empty())
            {
                current_user_globals = effective_user_globals;
                current_user_globals_offset = image_renderer
                    .upload_user_globals_bytes(queue, current_user_globals.as_bytes())
                    .unwrap_or(current_user_globals_offset);
            }

            current_atlas_index = Some(resolved.atlas_index);
            current_shader_id = shader_id;

            // Encode clip rect into instance data
            // clip_rect[2] (width) < 0 means no clipping
            let clip_rect = if let Some(clip) = opts.get_clip() {
                [
                    clip[0].as_f32(),
                    clip[1].as_f32(),
                    clip[2].as_f32(),
                    clip[3].as_f32(),
                ]
            } else {
                [0.0, 0.0, -1.0, 0.0]
            };

            batch.push(InstanceData {
                pos: [opts.position()[0].as_f32(), opts.position()[1].as_f32()],
                rotation: opts.rotation(),
                size: [
                    resolved.bounds.width.as_f32() * opts.scale()[0],
                    resolved.bounds.height.as_f32() * opts.scale()[1],
                ],
                uv_rect: resolved.uv_rect,
                clip_rect,
            });
        }

        // Final Batch
        if !batch.is_empty()
            && let Ok(range) = image_renderer.upload_instances(queue, batch.as_slice())
        {
            let pipeline = expect_image_pipeline(
                config.image_pipelines,
                config.default_pipeline,
                current_shader_id,
            );
            let atlas_bg = expect_atlas_bind_group(config.atlases, current_atlas_index);
            image_renderer.draw_batch(
                rpass,
                pipeline,
                atlas_bg,
                range,
                current_user_globals_offset,
                current_engine_globals_offset,
            );
        }
    }

    pub fn draw_context(
        &mut self,
        surface: &wgpu::Surface<'_>,
        ctx: &mut Context,
    ) -> Result<(), wgpu::SurfaceError> {
        #[cfg(feature = "model-3d")]
        if !ctx.runtime.model_3d.draw_list.is_empty() {
            self.ensure_model_3d();
        }
        self.sync_assets(ctx).map_err(|e| {
            eprintln!("[spot][graphics] sync_assets failed: {:?}", e);
            wgpu::SurfaceError::Lost
        })?;
        let _ = self.process_registrations(ctx);
        let draws = std::mem::take(&mut ctx.runtime.draw_list);
        self.process_image_commands(ctx, &draws);
        self.prepare_frame_resources(ctx, &draws).map_err(|e| {
            eprintln!("[spot][graphics] prepare_frame_resources failed: {:?}", e);
            wgpu::SurfaceError::Lost
        })?;
        let sf = ctx.scale_factor();
        self.draw_drawables_with_context(surface, &draws, sf, ctx)
    }

    fn draw_drawables_with_context(
        &mut self,
        surface: &wgpu::Surface<'_>,
        drawables: &[DrawCommand],
        scale_factor: f64,
        ctx: &mut Context,
    ) -> Result<(), wgpu::SurfaceError> {
        let (_lw, _lh) = ctx.window_logical_size();
        let sf = if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor
        } else {
            1.0
        };
        // No need to resize here anymore, we'll do it in draw_drawables_internal after getting the texture
        self.draw_drawables_internal(surface, drawables, sf, Some(ctx))
    }

    pub(crate) fn draw_drawables_internal(
        &mut self,
        surface: &wgpu::Surface<'_>,
        drawables: &[DrawCommand],
        scale_factor: f64,
        mut ctx: Option<&mut Context>,
    ) -> Result<(), wgpu::SurfaceError> {
        let frame_started_at = Instant::now();
        let wait_started_at = Instant::now();
        let frame = match surface.get_current_texture() {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[spot][graphics] get_current_texture failed: {:?}", e);
                return Err(e);
            }
        };
        let wait_ms = wait_started_at.elapsed().as_secs_f64() * 1000.0;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("command_encoder"),
            });

        #[cfg(feature = "model-3d")]
        if let Some(model_3d) = self.model_3d_mut() {
            model_3d.model_renderer.begin_frame();
        }
        self.image_renderer.begin_frame();

        let width = self.config.width;
        let height = self.config.height;
        #[cfg(feature = "model-3d")]
        if let Some(ctx_ref) = ctx.as_deref()
            && !ctx_ref.runtime.model_3d.draw_list.is_empty()
        {
            self.prepare_3d_command_order(ctx_ref);
        }

        // 1. Shadow Pass (3D)
        #[cfg(feature = "model-3d")]
        if let Some(ref mut ctx) = ctx
            && !ctx.runtime.model_3d.draw_list.is_empty()
        {
            self.render_shadow_pass(ctx, width, height);
        }

        if let Some(ref mut ctx) = ctx {
            self.resolve_drawables(ctx, drawables, width, height);
        }

        #[cfg(feature = "model-3d")]
        {
            let depth_stencil_attachment =
                self.model_3d()
                    .map(|model_3d| wgpu::RenderPassDepthStencilAttachment {
                        view: &model_3d.depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    });

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_3d_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: if self.transparent { 0.0 } else { 1.0 },
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            if let Some(ref mut ctx) = ctx
                && !ctx.runtime.model_3d.draw_list.is_empty()
            {
                self.render_main_3d_pass(ctx, width, height, &mut rpass);
            }
        }

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_overlay_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        #[cfg(feature = "model-3d")]
                        load: wgpu::LoadOp::Load,
                        #[cfg(not(feature = "model-3d"))]
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: if self.transparent { 0.0 } else { 1.0 },
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            #[cfg(all(feature = "model-3d", feature = "effects"))]
            if !self.transparent
                && let Some(model_3d) = self.model_3d()
                && model_3d.scene_globals.fog_params[0] > 0.0
            {
                rpass.set_pipeline(&model_3d.fog_background_pipeline);
                rpass.set_bind_group(0, &model_3d.fog_background_bind_group, &[]);
                rpass.draw(0..3, 0..1);
            }

            let lw = width as f32 / scale_factor as f32;
            let lh = height as f32 / scale_factor as f32;
            let screen_size_data = [2.0 / lw, 2.0 / lh, 1.0 / lw, 1.0 / lh];

            Self::render_batches_internal(
                &mut self.image_renderer,
                &self.queue,
                &mut self.batch,
                &mut self.resolved_draws,
                &mut rpass,
                RenderConfig {
                    screen_size_data,
                    scale_factor: scale_factor as f32,
                    atlases: &self.atlases,
                    image_pipelines: &self.image_pipelines,
                    default_pipeline: &self.default_pipeline,
                },
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        crate::graphics::profile::record_render_frame(
            wait_ms,
            frame_started_at.elapsed().as_secs_f64() * 1000.0,
        );

        Ok(())
    }
}
