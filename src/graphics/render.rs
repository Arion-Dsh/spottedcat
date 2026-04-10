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

use super::core::{Graphics, ResolvedDraw};
use super::image_ops::resolve_image_uv;
use crate::image_raw::ImageRenderer;

pub(crate) struct RenderConfig<'a> {
    pub screen_size_data: [f32; 4],
    pub scale_factor: f32,
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

fn expect_resource_bind_group<'a>(
    ctx: &'a Context,
    texture_id: u32,
) -> &'a wgpu::BindGroup {
    ctx.registry
        .textures
        .get(texture_id as usize)
        .and_then(|v| v.as_ref())
        .map(|e| &e.runtime)
        .and_then(|d| d.bind_group.as_ref())
        .unwrap_or_else(|| panic!("[spot][render] missing bind group for texture {}", texture_id))
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

                        self.resolved_draws.push(ResolvedDraw {
                            texture_id: entry.texture_id,
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

        for resolved in resolved_draws.iter() {
            let opts = resolved.opts;
            let shader_id = resolved.shader_id;
            let shader_opts = resolved.shader_opts;
            let draw_opacity = opts.opacity();

            let effective_user_globals = shader_opts;

            let state_changed = current_texture_id != Some(resolved.texture_id)
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
                    let bind_group = expect_resource_bind_group(ctx, current_texture_id.unwrap());
                    image_renderer.draw_batch(
                        rpass,
                        pipeline,
                        bind_group,
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
            current_shader_id = shader_id;

            batch.push(InstanceData {
                pos: [opts.position()[0].as_f32(), opts.position()[1].as_f32()],
                rotation: opts.rotation(),
                size: [
                    resolved.bounds.width.as_f32() * opts.scale()[0],
                    resolved.bounds.height.as_f32() * opts.scale()[1],
                ],
                uv_rect: resolved.uv_rect,
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
            let bind_group = expect_resource_bind_group(ctx, current_texture_id.unwrap());
            image_renderer.draw_batch(
                rpass,
                pipeline,
                bind_group,
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

        #[cfg(feature = "model-3d")]
        if let Some(model_3d) = self.model_3d_mut() {
            model_3d.model_renderer.begin_frame();
        }
        self.image_renderer.begin_frame();

        self.render_all_targets(ctx, &draws);

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

        let width = self.config.width;
        let height = self.config.height;
        #[cfg(feature = "model-3d")]
        if !ctx.runtime.model_3d.draw_list.is_empty()
        {
            self.prepare_3d_command_order(ctx, 0);
        }

        #[cfg(feature = "model-3d")]
        if !ctx.runtime.model_3d.draw_list.is_empty()
        {
            self.render_shadow_pass(ctx, width, height, 0);
        }

        self.resolve_drawables(ctx, &draws, 0, width, height);

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

            if !ctx.runtime.model_3d.draw_list.is_empty()
            {
                self.render_main_3d_pass(ctx, width, height, &mut rpass, 0);
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
                    screen_size_data,
                    scale_factor: ctx.scale_factor() as f32,
                    image_pipelines: &self.image_pipelines,
                    default_pipeline: &self.default_pipeline,
                },
                ctx,
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
                    ctx,
                    drawables,
                    dependency,
                    encoder,
                    rendered,
                    visiting,
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
                DrawCommand::Image(cmd) if cmd.target_texture_id == target_texture_id => ctx
                    .registry
                    .images
                    .get(cmd.id as usize)
                    .and_then(|v| v.as_ref())
                    .map(|entry| entry.texture_id),
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
                crate::image::Bounds::new(crate::Pt(0.0), crate::Pt(0.0), entry.width, entry.height),
            )
        };

        #[cfg(feature = "model-3d")]
        if !ctx.runtime.model_3d.draw_list.is_empty() {
            self.prepare_3d_command_order(ctx, target_texture_id);
            if !self
                .model_3d()
                .map(|model_3d| {
                    !model_3d.opaque_draw_indices_3d.is_empty()
                        || !model_3d.transparent_draw_indices_3d.is_empty()
                })
                .unwrap_or(false)
            {
                // no-op
            } else {
                self.render_shadow_pass(ctx, width, height, target_texture_id);
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
                        target_texture_id,
                        runtime.generation,
                        self.gpu_generation
                    );
                }

                texture_obj.clone()
            };
            let view = &target_gpu_texture.0.view;

            #[cfg(feature = "model-3d")]
            {
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
            }

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("target_overlay_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
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
                    screen_size_data,
                    scale_factor: 1.0,
                    image_pipelines: &self.image_pipelines,
                    default_pipeline: &self.default_pipeline,
                },
                ctx,
            );
        }
    }
}
