//! Batch rendering and draw operations.

use crate::Context;
use crate::ShaderOpts;
use crate::drawable::DrawCommand;
use crate::image_raw::InstanceData;
use std::collections::{HashMap, HashSet};
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::time::Instant;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web_time::Instant;

use super::core::{Graphics, ResolvedDraw, ResolvedImageShaderInput};
use super::image_ops::resolve_image_uv;
use super::image_pipeline::ImagePipeline;
use crate::image_raw::ImageRenderer;
use crate::image_shader::ImageShaderInput;

pub(crate) struct RenderConfig<'a> {
    pub device: &'a wgpu::Device,
    pub screen_size_data: [f32; 4],
    pub scale_factor: f32,
    pub image_pipelines: &'a HashMap<u32, ImagePipeline>,
    pub default_pipeline: &'a wgpu::RenderPipeline,
    pub screen_snapshots: &'a HashMap<u32, crate::graphics::texture::GpuTexture>,
    pub history_snapshots: &'a HashMap<u32, crate::graphics::texture::GpuTexture>,
}

fn expect_image_pipeline<'a>(
    image_pipelines: &'a HashMap<u32, ImagePipeline>,
    default_pipeline: &'a wgpu::RenderPipeline,
    shader_id: u32,
) -> (&'a wgpu::RenderPipeline, bool) {
    if shader_id == 0 {
        (default_pipeline, false)
    } else {
        let pipeline = image_pipelines.get(&shader_id).unwrap_or_else(|| {
            panic!(
                "[spot][render] missing image pipeline for shader_id {}",
                shader_id
            )
        });
        (&pipeline.pipeline, pipeline.uses_extra_textures)
    }
}

fn expect_resource_bind_group<'a>(ctx: &'a Context, texture_id: u32) -> &'a wgpu::BindGroup {
    ctx.registry
        .textures
        .get(texture_id as usize)
        .and_then(|v| v.as_ref())
        .map(|e| &e.runtime)
        .and_then(|d| d.bind_group.as_ref())
        .unwrap_or_else(|| {
            panic!(
                "[spot][render] missing bind group for texture {}",
                texture_id
            )
        })
}

fn expect_texture_view<'a>(ctx: &'a Context, texture_id: u32) -> &'a wgpu::TextureView {
    ctx.registry
        .textures
        .get(texture_id as usize)
        .and_then(|v| v.as_ref())
        .and_then(|entry| entry.runtime.gpu_texture.as_ref())
        .map(|gpu| &gpu.0.view)
        .unwrap_or_else(|| {
            panic!(
                "[spot][render] missing texture view for texture {}",
                texture_id
            )
        })
}

fn resolve_shader_input_texture<'a>(
    ctx: &'a Context,
    screen_snapshots: &'a HashMap<u32, crate::graphics::texture::GpuTexture>,
    history_snapshots: &'a HashMap<u32, crate::graphics::texture::GpuTexture>,
    input: ResolvedImageShaderInput,
) -> &'a wgpu::TextureView {
    match input {
        ResolvedImageShaderInput::Texture(texture_id) => expect_texture_view(ctx, texture_id),
        ResolvedImageShaderInput::Screen(target_texture_id) => screen_snapshots
            .get(&target_texture_id)
            .map(|texture| &texture.0.view)
            .or_else(|| {
                history_snapshots
                    .get(&target_texture_id)
                    .map(|texture| &texture.0.view)
            })
            .unwrap_or_else(|| {
                panic!(
                    "[spot][render] missing screen snapshot for target {}",
                    target_texture_id
                )
            }),
        ResolvedImageShaderInput::History(target_texture_id) => history_snapshots
            .get(&target_texture_id)
            .map(|texture| &texture.0.view)
            .or_else(|| {
                screen_snapshots
                    .get(&target_texture_id)
                    .map(|texture| &texture.0.view)
            })
            .unwrap_or_else(|| {
                panic!(
                    "[spot][render] missing history snapshot for target {}",
                    target_texture_id
                )
            }),
    }
}

fn resolve_extra_texture_ids(inputs: [ResolvedImageShaderInput; 4]) -> [u32; 4] {
    let mut ids = [0u32; 4];
    for (index, input) in inputs.iter().enumerate() {
        ids[index] = match input {
            ResolvedImageShaderInput::Texture(texture_id)
            | ResolvedImageShaderInput::Screen(texture_id)
            | ResolvedImageShaderInput::History(texture_id) => *texture_id,
        };
    }
    ids
}

impl Graphics {
    pub(crate) fn resolve_drawables(
        &mut self,
        ctx: &mut Context,
        drawables: &[DrawCommand],
        target_texture_id: u32,
        logical_w: u32,
        logical_h: u32,
    ) {
        self.resolved_draws.clear();
        let viewport_rect = [0.0, 0.0, logical_w as f32, logical_h as f32];

        for drawable in drawables {
            match drawable {
                DrawCommand::Image(cmd) => {
                    if cmd.target_texture_id != target_texture_id {
                        continue;
                    }
                    if let Some(Some(entry)) = ctx.registry.images.get(cmd.id as usize) {
                        let Some(texture_entry) = ctx
                            .registry
                            .textures
                            .get(entry.texture_id as usize)
                            .and_then(|v| v.as_ref())
                        else {
                            continue;
                        };
                        if !entry.visible || !texture_entry.is_ready(self.gpu_generation) {
                            continue;
                        }

                        let mut extra_inputs =
                            [ResolvedImageShaderInput::Texture(entry.texture_id); 4];
                        
                        let shader_desc = ctx.registry.image_shaders.get(&cmd.shader_id);

                        // Helper to resolve ImageShaderInput to ResolvedImageShaderInput
                        let resolve_input = |input: &ImageShaderInput| -> ResolvedImageShaderInput {
                            match input {
                                ImageShaderInput::None => {
                                    ResolvedImageShaderInput::Texture(entry.texture_id)
                                }
                                ImageShaderInput::Image(extra_image) => ctx
                                    .registry
                                    .images
                                    .get(extra_image.index())
                                    .and_then(|v| v.as_ref())
                                    .map(|extra_entry| {
                                        ResolvedImageShaderInput::Texture(extra_entry.texture_id)
                                    })
                                    .unwrap_or(ResolvedImageShaderInput::Texture(entry.texture_id)),
                                ImageShaderInput::Screen => {
                                    ResolvedImageShaderInput::Screen(cmd.target_texture_id)
                                }
                                ImageShaderInput::History => {
                                    ResolvedImageShaderInput::History(cmd.target_texture_id)
                                }
                            }
                        };

                        // 1. Apply legacy index-based slots
                        for (index, input) in cmd.shader_bindings.extra_inputs.iter().enumerate() {
                            if *input != ImageShaderInput::None {
                                extra_inputs[index] = resolve_input(input);
                            }
                        }

                        // 2. Apply semantic intents (ignoring None)
                        if let Some(desc) = shader_desc {
                            if cmd.shader_bindings.history {
                                if let Some(slot) = desc.history_slot {
                                    extra_inputs[slot] = ResolvedImageShaderInput::History(cmd.target_texture_id);
                                }
                            }
                            if cmd.shader_bindings.screen {
                                if let Some(slot) = desc.screen_slot {
                                    extra_inputs[slot] = ResolvedImageShaderInput::Screen(cmd.target_texture_id);
                                }
                            }
                            for (name, input) in &cmd.shader_bindings.named_inputs {
                                for i in 0..4 {
                                    if desc.extra_texture_names[i].as_deref() == Some(name) {
                                        extra_inputs[i] = resolve_input(input);
                                        break;
                                    }
                                }
                            }
                        }

                        self.resolved_draws.push(ResolvedDraw {
                            texture_id: entry.texture_id,
                            extra_inputs,
                            bounds: entry.bounds,
                            uv_rect: resolve_image_uv(entry, texture_entry),
                            opts: cmd.opts,
                            shader_id: cmd.shader_id,
                            shader_opts: cmd.shader_opts,
                        });
                    }
                }
                DrawCommand::Text(cmd) => {
                    if cmd.target_texture_id != target_texture_id {
                        continue;
                    }
                    if let Err(e) =
                        self.layout_and_queue_text(ctx, &cmd.text, &cmd.opts, viewport_rect)
                    {
                        eprintln!("[spot] Text layout error: {:?}", e);
                    }
                }
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
        ctx: &'a Context,
    ) {
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
        let mut current_texture_id: Option<u32> = None;
        let mut current_shader_id: u32 = 0;
        let mut current_user_globals = ShaderOpts::default();
        let mut current_extra_inputs = [ResolvedImageShaderInput::Texture(0); 4];

        for resolved in resolved_draws.iter() {
            let opts = resolved.opts;
            let shader_id = resolved.shader_id;
            let shader_opts = resolved.shader_opts;
            let draw_opacity = opts.opacity();

            let effective_user_globals = shader_opts;

            let state_changed = current_texture_id != Some(resolved.texture_id)
                || current_extra_inputs != resolved.extra_inputs
                || current_shader_id != shader_id
                || current_user_globals != effective_user_globals
                || current_opacity != draw_opacity;

            if state_changed && !batch.is_empty() {
                if let Ok(range) = image_renderer.upload_instances(queue, batch.as_slice()) {
                    let (pipeline, uses_extra_textures) = expect_image_pipeline(
                        config.image_pipelines,
                        config.default_pipeline,
                        current_shader_id,
                    );
                    let bind_group = expect_resource_bind_group(ctx, current_texture_id.unwrap());
                    let extra_bind_group = if uses_extra_textures {
                        let texture_ids = resolve_extra_texture_ids(current_extra_inputs);
                        Some(image_renderer.extra_texture_bind_group(
                            config.device,
                            texture_ids,
                            [
                                resolve_shader_input_texture(
                                    ctx,
                                    config.screen_snapshots,
                                    config.history_snapshots,
                                    current_extra_inputs[0],
                                ),
                                resolve_shader_input_texture(
                                    ctx,
                                    config.screen_snapshots,
                                    config.history_snapshots,
                                    current_extra_inputs[1],
                                ),
                                resolve_shader_input_texture(
                                    ctx,
                                    config.screen_snapshots,
                                    config.history_snapshots,
                                    current_extra_inputs[2],
                                ),
                                resolve_shader_input_texture(
                                    ctx,
                                    config.screen_snapshots,
                                    config.history_snapshots,
                                    current_extra_inputs[3],
                                ),
                            ],
                        ))
                    } else {
                        None
                    };
                    image_renderer.draw_batch(
                        rpass,
                        pipeline,
                        bind_group,
                        extra_bind_group.as_ref(),
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

            if current_user_globals != effective_user_globals || batch.is_empty() {
                current_user_globals = effective_user_globals;
                current_user_globals_offset = image_renderer
                    .upload_user_globals_bytes(queue, current_user_globals.as_bytes())
                    .unwrap_or(current_user_globals_offset);
            }

            current_texture_id = Some(resolved.texture_id);
            current_extra_inputs = resolved.extra_inputs;
            current_shader_id = shader_id;

            batch.push(InstanceData {
                pos: [opts.position()[0].as_f32(), opts.position()[1].as_f32()],
                rotation: opts.rotation(),
                size: [
                    resolved.bounds.width.as_f32() * opts.scale()[0],
                    resolved.bounds.height.as_f32() * opts.scale()[1],
                ],
                uv_rect: resolved.uv_rect,
                ..Default::default()
            });
        }

        // Final Batch
        if !batch.is_empty()
            && let Ok(range) = image_renderer.upload_instances(queue, batch.as_slice())
        {
            let (pipeline, uses_extra_textures) = expect_image_pipeline(
                config.image_pipelines,
                config.default_pipeline,
                current_shader_id,
            );
            let bind_group = expect_resource_bind_group(ctx, current_texture_id.unwrap());
            let extra_bind_group = if uses_extra_textures {
                let texture_ids = resolve_extra_texture_ids(current_extra_inputs);
                Some(image_renderer.extra_texture_bind_group(
                    config.device,
                    texture_ids,
                    [
                        resolve_shader_input_texture(
                            ctx,
                            config.screen_snapshots,
                            config.history_snapshots,
                            current_extra_inputs[0],
                        ),
                        resolve_shader_input_texture(
                            ctx,
                            config.screen_snapshots,
                            config.history_snapshots,
                            current_extra_inputs[1],
                        ),
                        resolve_shader_input_texture(
                            ctx,
                            config.screen_snapshots,
                            config.history_snapshots,
                            current_extra_inputs[2],
                        ),
                        resolve_shader_input_texture(
                            ctx,
                            config.screen_snapshots,
                            config.history_snapshots,
                            current_extra_inputs[3],
                        ),
                    ],
                ))
            } else {
                None
            };
            image_renderer.draw_batch(
                rpass,
                pipeline,
                bind_group,
                extra_bind_group.as_ref(),
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
        self.prepare_frame_resources(ctx, &draws).map_err(|e| {
            eprintln!("[spot][graphics] prepare_frame_resources failed: {:?}", e);
            wgpu::SurfaceError::Lost
        })?;

        let targets_ms;
        #[allow(unused_mut, unused_assignments)]
        let mut shadow_ms = 0.0;
        #[allow(unused_mut, unused_assignments)]
        let mut main_3d_ms = 0.0;
        let overlay_ms;
        let present_ms;

        #[cfg(feature = "model-3d")]
        if let Some(model_3d) = self.model_3d_mut() {
            model_3d.model_renderer.begin_frame();
        }
        self.image_renderer.begin_frame();

        let targets_started_at = Instant::now();
        self.render_all_targets(ctx, &draws);
        targets_ms = targets_started_at.elapsed().as_secs_f64() * 1000.0;

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
        let surface_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("command_encoder"),
            });

        let width = self.config.width;
        let height = self.config.height;
        let final_screen_texture = self.ensure_final_screen_texture(width, height);
        let render_view = &final_screen_texture.0.view;
        #[cfg(feature = "model-3d")]
        if ctx.runtime.model_3d.draw_list.is_empty() {
            self.clear_3d_command_order();
        } else {
            self.prepare_3d_command_order(ctx, 0);
        }

        #[cfg(feature = "model-3d")]
        if !ctx.runtime.model_3d.draw_list.is_empty()
            && self
                .model_3d()
                .map(|model_3d| !model_3d.shadow_draw_indices_3d.is_empty())
                .unwrap_or(false)
        {
            let shadow_started_at = Instant::now();
            self.render_shadow_pass(&mut encoder, ctx, width, height, 0);
            shadow_ms = shadow_started_at.elapsed().as_secs_f64() * 1000.0;
        }

        self.resolve_drawables(ctx, &draws, 0, width, height);

        #[cfg(feature = "model-3d")]
        {
            let main_3d_started_at = Instant::now();
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
                    view: render_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
                            r: 0.1,
                            #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
                            r: 0.0,
                            #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
                            g: 0.1,
                            #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
                            g: 0.0,
                            #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
                            b: 0.2,
                            #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
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

            if !ctx.runtime.model_3d.draw_list.is_empty() {
                self.render_main_3d_pass(ctx, width, height, &mut rpass, 0);
            }
            main_3d_ms = main_3d_started_at.elapsed().as_secs_f64() * 1000.0;
        }

        Self::update_shader_snapshot(
            &mut self.shader_screen_snapshots,
            &self.device,
            &mut encoder,
            0,
            &final_screen_texture.0.texture,
            width,
            height,
            self.config.format,
        );

        {
            let overlay_started_at = Instant::now();
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_overlay_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: render_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        #[cfg(feature = "model-3d")]
                        load: wgpu::LoadOp::Load,
                        #[cfg(not(feature = "model-3d"))]
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
                            r: 0.1,
                            #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
                            r: 0.0,
                            #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
                            g: 0.1,
                            #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
                            g: 0.0,
                            #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
                            b: 0.2,
                            #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
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

            let screen_scale_factor = ctx.scale_factor() as f32;
            let lw = width as f32 / screen_scale_factor;
            let lh = height as f32 / screen_scale_factor;
            let screen_size_data = [2.0 / lw, 2.0 / lh, 1.0 / lw, 1.0 / lh];

            Self::render_batches_internal(
                &mut self.image_renderer,
                &self.queue,
                &mut self.batch,
                &mut self.resolved_draws,
                &mut rpass,
                RenderConfig {
                    device: &self.device,
                    screen_size_data,
                    scale_factor: ctx.scale_factor() as f32,
                    image_pipelines: &self.image_pipelines,
                    default_pipeline: &self.default_pipeline,
                    screen_snapshots: &self.shader_screen_snapshots,
                    history_snapshots: &self.shader_history_snapshots,
                },
                ctx,
            );
            overlay_ms = overlay_started_at.elapsed().as_secs_f64() * 1000.0;
        }

        Self::update_shader_snapshot(
            &mut self.shader_history_snapshots,
            &self.device,
            &mut encoder,
            0,
            &final_screen_texture.0.texture,
            width,
            height,
            self.config.format,
        );

        {
            let present_started_at = Instant::now();
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("present_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            let screen_scale_factor = ctx.scale_factor() as f32;
            let lw = width as f32 / screen_scale_factor;
            let lh = height as f32 / screen_scale_factor;
            let screen_size_data = [2.0 / lw, 2.0 / lh, 1.0 / lw, 1.0 / lh];
            let engine_globals = crate::image_raw::EngineGlobals {
                screen: screen_size_data,
                opacity: 1.0,
                shader_opacity: 1.0,
                scale_factor: screen_scale_factor,
                _padding: [0.0; 1],
            };
            let engine_offset = self
                .image_renderer
                .upload_engine_globals(&self.queue, &engine_globals)
                .unwrap_or(0);
            let user_offset = self
                .image_renderer
                .upload_user_globals_bytes(&self.queue, ShaderOpts::default().as_bytes())
                .unwrap_or(0);
            let range = self
                .image_renderer
                .upload_instances(
                    &self.queue,
                    &[InstanceData {
                        pos: [0.0, 0.0],
                        rotation: 0.0,
                        size: [lw, lh],
                        uv_rect: [0.0, 0.0, 1.0, 1.0],
                        ..Default::default()
                    }],
                )
                .unwrap_or(0..0);
            let bind_group = self
                .image_renderer
                .create_texture_bind_group(&self.device, render_view);
            self.image_renderer.draw_batch(
                &mut rpass,
                &self.default_pipeline,
                &bind_group,
                None,
                range,
                user_offset,
                engine_offset,
            );
            present_ms = present_started_at.elapsed().as_secs_f64() * 1000.0;
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        crate::graphics::profile::record_render_frame(
            wait_ms,
            frame_started_at.elapsed().as_secs_f64() * 1000.0,
            targets_ms,
            shadow_ms,
            main_3d_ms,
            overlay_ms,
            present_ms,
        );

        Ok(())
    }

    fn render_all_targets(&mut self, ctx: &mut Context, drawables: &[DrawCommand]) {
        let target_ids = self.collect_target_ids(ctx, drawables);
        if target_ids.is_empty() {
            return;
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("target_encoder"),
            });
        let mut rendered = HashSet::new();
        let mut visiting = HashSet::new();

        for target_texture_id in target_ids {
            self.render_target_recursive(
                ctx,
                drawables,
                target_texture_id,
                &mut encoder,
                &mut rendered,
                &mut visiting,
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    fn collect_target_ids(&self, ctx: &Context, drawables: &[DrawCommand]) -> Vec<u32> {
        let mut target_ids = Vec::new();
        let mut seen = HashSet::new();

        for drawable in drawables {
            let target_texture_id = match drawable {
                DrawCommand::Image(cmd) => cmd.target_texture_id,
                DrawCommand::Text(cmd) => cmd.target_texture_id,
            };
            if target_texture_id != 0
                && self.target_is_live(ctx, target_texture_id)
                && seen.insert(target_texture_id)
            {
                target_ids.push(target_texture_id);
            }
        }

        #[cfg(feature = "model-3d")]
        for command in &ctx.runtime.model_3d.draw_list {
            let target_texture_id = match command {
                crate::drawable::DrawCommand3D::Model(target, ..)
                | crate::drawable::DrawCommand3D::ModelInstanced(target, ..) => *target,
            };
            if target_texture_id != 0
                && self.target_is_live(ctx, target_texture_id)
                && seen.insert(target_texture_id)
            {
                target_ids.push(target_texture_id);
            }
        }

        target_ids
    }

    fn target_is_live(&self, ctx: &Context, target_texture_id: u32) -> bool {
        ctx.registry
            .textures
            .get(target_texture_id as usize)
            .and_then(|v| v.as_ref())
            .map(|entry| entry.is_ready(self.gpu_generation))
            .unwrap_or(false)
    }

    fn render_target_recursive(
        &mut self,
        ctx: &mut Context,
        drawables: &[DrawCommand],
        target_texture_id: u32,
        encoder: &mut wgpu::CommandEncoder,
        rendered: &mut HashSet<u32>,
        visiting: &mut HashSet<u32>,
    ) {
        if rendered.contains(&target_texture_id) {
            return;
        }
        if !visiting.insert(target_texture_id) {
            panic!(
                "[spot][render] render target cycle detected while resolving texture {}",
                target_texture_id
            );
        }

        for dependency in self.target_dependencies(ctx, drawables, target_texture_id) {
            if dependency != target_texture_id {
                self.render_target_recursive(
                    ctx, drawables, dependency, encoder, rendered, visiting,
                );
            }
        }

        self.render_target_pass(ctx, drawables, target_texture_id, encoder);

        visiting.remove(&target_texture_id);
        rendered.insert(target_texture_id);
    }

    fn target_dependencies(
        &self,
        ctx: &Context,
        drawables: &[DrawCommand],
        target_texture_id: u32,
    ) -> Vec<u32> {
        let mut deps = Vec::new();
        let mut seen = HashSet::new();

        for drawable in drawables {
            let Some(dep_texture_id) = (match drawable {
                DrawCommand::Image(cmd) if cmd.target_texture_id == target_texture_id => {
                    let mut dep_texture_id = ctx
                        .registry
                        .images
                        .get(cmd.id as usize)
                        .and_then(|v| v.as_ref())
                        .map(|entry| entry.texture_id);
                    if dep_texture_id.is_none() {
                        for input in &cmd.shader_bindings.extra_inputs {
                            if let ImageShaderInput::Image(image) = input {
                                dep_texture_id = ctx
                                    .registry
                                    .images
                                    .get(image.id() as usize)
                                    .and_then(|v| v.as_ref())
                                    .map(|entry| entry.texture_id);
                                if dep_texture_id.is_some() {
                                    break;
                                }
                            }
                        }
                    }
                    dep_texture_id
                }
                DrawCommand::Text(cmd) if cmd.target_texture_id == target_texture_id => None,
                DrawCommand::Image(_) | DrawCommand::Text(_) => None,
            }) else {
                continue;
            };

            if dep_texture_id != 0
                && dep_texture_id != target_texture_id
                && ctx
                    .registry
                    .textures
                    .get(dep_texture_id as usize)
                    .and_then(|v| v.as_ref())
                    .map(|entry| entry.is_render_target())
                    .unwrap_or(false)
                && seen.insert(dep_texture_id)
            {
                deps.push(dep_texture_id);
            }
        }

        #[cfg(feature = "model-3d")]
        for command in &ctx.runtime.model_3d.draw_list {
            let matches_target = match command {
                crate::drawable::DrawCommand3D::Model(target, ..)
                | crate::drawable::DrawCommand3D::ModelInstanced(target, ..) => {
                    *target == target_texture_id
                }
            };
            if !matches_target {
                continue;
            }

            let model = match command {
                crate::drawable::DrawCommand3D::Model(_, model, ..)
                | crate::drawable::DrawCommand3D::ModelInstanced(_, model, ..) => model,
            };

            for part in model.parts.iter() {
                for image_id in [
                    part.material.albedo,
                    part.material.pbr,
                    part.material.normal,
                    part.material.occlusion,
                    part.material.emissive,
                ]
                .into_iter()
                .flatten()
                {
                    let Some(dep_texture_id) = ctx
                        .registry
                        .images
                        .get(image_id as usize)
                        .and_then(|v| v.as_ref())
                        .map(|entry| entry.texture_id)
                    else {
                        continue;
                    };
                    if dep_texture_id != target_texture_id
                        && ctx
                            .registry
                            .textures
                            .get(dep_texture_id as usize)
                            .and_then(|v| v.as_ref())
                            .map(|entry| entry.is_render_target())
                            .unwrap_or(false)
                        && seen.insert(dep_texture_id)
                    {
                        deps.push(dep_texture_id);
                    }
                }
            }
        }

        deps
    }

    fn ensure_shader_snapshot_texture(
        cache: &mut HashMap<u32, crate::graphics::texture::GpuTexture>,
        device: &wgpu::Device,
        target_texture_id: u32,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) {
        let needs_recreate = cache
            .get(&target_texture_id)
            .map(|texture| {
                texture.0.texture.width() != width || texture.0.texture.height() != height
            })
            .unwrap_or(true);
        if needs_recreate {
            cache.insert(
                target_texture_id,
                crate::graphics::texture::GpuTexture::create_empty_with_usage_and_mips(
                    device,
                    width.max(1),
                    height.max(1),
                    format,
                    wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_DST
                        | wgpu::TextureUsages::COPY_SRC,
                    1,
                ),
            );
        }
    }

    fn update_shader_snapshot(
        cache: &mut HashMap<u32, crate::graphics::texture::GpuTexture>,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target_texture_id: u32,
        src_texture: &wgpu::Texture,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) {
        Self::ensure_shader_snapshot_texture(
            cache,
            device,
            target_texture_id,
            width,
            height,
            format,
        );
        let dst = cache
            .get(&target_texture_id)
            .expect("snapshot texture must exist after ensure");
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: src_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &dst.0.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
        );
    }

    fn ensure_final_screen_texture(
        &mut self,
        width: u32,
        height: u32,
    ) -> crate::graphics::texture::GpuTexture {
        let needs_recreate = self
            .final_screen_texture
            .as_ref()
            .map(|texture| {
                texture.0.texture.width() != width || texture.0.texture.height() != height
            })
            .unwrap_or(true);
        if needs_recreate {
            self.final_screen_texture = Some(
                crate::graphics::texture::GpuTexture::create_empty_with_usage_and_mips(
                    &self.device,
                    width.max(1),
                    height.max(1),
                    self.config.format,
                    wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC,
                    1,
                ),
            );
        }
        self.final_screen_texture
            .as_ref()
            .expect("final screen texture must exist after ensure")
            .clone()
    }

    fn render_target_pass(
        &mut self,
        ctx: &mut Context,
        drawables: &[DrawCommand],
        target_texture_id: u32,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let (width, height, bounds) = {
            let entry = ctx
                .registry
                .textures
                .get(target_texture_id as usize)
                .and_then(|v| v.as_ref())
                .expect("[spot][render] render target missing during target pass");
            (
                entry.pixel_width,
                entry.pixel_height,
                crate::image::Bounds::new(
                    crate::Pt(0.0),
                    crate::Pt(0.0),
                    entry.width,
                    entry.height,
                ),
            )
        };

        #[cfg(feature = "model-3d")]
        if ctx.runtime.model_3d.draw_list.is_empty() {
            self.clear_3d_command_order();
        } else {
            self.prepare_3d_command_order(ctx, target_texture_id);
            if self
                .model_3d()
                .map(|model_3d| !model_3d.shadow_draw_indices_3d.is_empty())
                .unwrap_or(false)
            {
                self.render_shadow_pass(encoder, ctx, width, height, target_texture_id);
            }
        }

        self.resolve_drawables(ctx, drawables, target_texture_id, width, height);
        let mut target_resolved = std::mem::take(&mut self.resolved_draws);

        {
            let target_gpu_texture = {
                let entry = ctx
                    .registry
                    .textures
                    .get(target_texture_id as usize)
                    .and_then(|v| v.as_ref())
                    .unwrap();
                let runtime = &entry.runtime;
                let texture_obj = runtime.gpu_texture.as_ref().unwrap();

                if runtime.generation != self.gpu_generation {
                    eprintln!(
                        "[spot][render] WARNING: Rendering to texture {} with generation mismatch ({} vs {})",
                        target_texture_id, runtime.generation, self.gpu_generation
                    );
                }

                texture_obj.clone()
            };
            let view = &target_gpu_texture.0.view;

            #[cfg(feature = "model-3d")]
            let has_3d = {
                let has_3d = self
                    .model_3d()
                    .map(|model_3d| {
                        !model_3d.opaque_draw_indices_3d.is_empty()
                            || !model_3d.transparent_draw_indices_3d.is_empty()
                    })
                    .unwrap_or(false);

                if has_3d {
                    let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("spot_offscreen_depth"),
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
                        view_formats: &[],
                    });
                    let depth_view =
                        depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("target_3d_render_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view,
                            resolve_target: None,
                            depth_slice: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &depth_view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    self.render_main_3d_pass(ctx, width, height, &mut rpass, target_texture_id);
                }
                has_3d
            };
            #[cfg(not(feature = "model-3d"))]
            let has_3d = false;

            Self::update_shader_snapshot(
                &mut self.shader_screen_snapshots,
                &self.device,
                encoder,
                target_texture_id,
                &target_gpu_texture.0.texture,
                width,
                height,
                self.config.format,
            );

            {
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("target_overlay_render_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: if has_3d {
                                wgpu::LoadOp::Load
                            } else {
                                wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
                            },
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

                let lw = bounds.width.as_f32();
                let lh = bounds.height.as_f32();
                let screen_size_data = [2.0 / lw, 2.0 / lh, 1.0 / lw, 1.0 / lh];

                Self::render_batches_internal(
                    &mut self.image_renderer,
                    &self.queue,
                    &mut self.batch,
                    &mut target_resolved,
                    &mut rpass,
                    RenderConfig {
                        device: &self.device,
                        screen_size_data,
                        scale_factor: 1.0,
                        image_pipelines: &self.image_pipelines,
                        default_pipeline: &self.default_pipeline,
                        screen_snapshots: &self.shader_screen_snapshots,
                        history_snapshots: &self.shader_history_snapshots,
                    },
                    ctx,
                );
            }

            Self::update_shader_snapshot(
                &mut self.shader_history_snapshots,
                &self.device,
                encoder,
                target_texture_id,
                &target_gpu_texture.0.texture,
                width,
                height,
                self.config.format,
            );
        }
    }
}
