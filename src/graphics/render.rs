//! Batch rendering and draw operations.


use crate::Context;
use crate::drawable::{DrawCommand, DrawCommand3D};
use crate::ShaderOpts;
use crate::image_raw::InstanceData;
use crate::pt::Pt;
use std::collections::HashMap;

use crate::image::ImageEntry;
use crate::image_raw::ImageRenderer;
use crate::graphics::model_raw::{ModelRenderer, MeshData};
use super::core::{Graphics, ResolvedDraw, AtlasSlot, SkinData};

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
                        if !entry.visible || !entry.is_ready() {
                            continue;
                        }

                        self.resolved_draws.push(ResolvedDraw {
                            img_entry: entry.clone(),
                            opts: *opts,
                            shader_id: *shader_id,
                            shader_opts: *shader_opts,
                            layer: opts.layer(),
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

    pub(super) fn render_batches_internal<'a>(
        image_renderer: &mut ImageRenderer,
        queue: &wgpu::Queue,
        atlases: &[AtlasSlot],
        image_pipelines: &'a HashMap<u32, wgpu::RenderPipeline>,
        default_pipeline: &'a wgpu::RenderPipeline,
        batch: &mut Vec<InstanceData>,
        resolved_draws: &mut Vec<ResolvedDraw>,
        rpass: &mut wgpu::RenderPass<'a>,
        screen_size_data: [f32; 4],
        config_width: u32,
        config_height: u32,
        sf: f64,
    ) {
        // Sort by layer first, then atlas and shader to maximize batching
        resolved_draws.sort_by(|a, b| {
            a.layer.cmp(&b.layer)
                .then(a.img_entry.atlas_index.cmp(&b.img_entry.atlas_index))
                .then(a.shader_id.cmp(&b.shader_id))
        });

        let mut current_opacity = 1.0f32;

        // Upload initial engine globals
        let engine_globals = crate::image_raw::EngineGlobals {
            screen: screen_size_data,
            opacity: current_opacity,
            shader_opacity: 1.0,
            _padding: [0.0; 2],
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
        let mut current_clip: Option<[Pt; 4]> = None;

        rpass.set_scissor_rect(0, 0, config_width.max(1), config_height.max(1));
        let mut last_set_scissor: Option<(u32, u32, u32, u32)> = None;

        for resolved in resolved_draws.iter() {
            let img_entry = &resolved.img_entry;
            let opts = resolved.opts;
            let shader_id = resolved.shader_id;
            let shader_opts = resolved.shader_opts;
            let draw_opacity = opts.opacity();

            let uv_rect = match img_entry.uv_rect {
                Some(uv) => uv,
                None => continue,
            };

            let effective_user_globals = shader_opts;

            let state_changed = current_atlas_index != img_entry.atlas_index
                || current_shader_id != shader_id
                || current_user_globals != effective_user_globals
                || current_clip != opts.get_clip()
                || current_opacity != draw_opacity;

            if state_changed && !batch.is_empty() {
                let ai = current_atlas_index
                    .expect("current_atlas_index should be Some if batch is not empty");
                let atlas_bg = &atlases.get(ai as usize).expect("atlas").bind_group;

                if let Ok(range) = image_renderer.upload_instances(queue, batch.as_slice())
                {
                    let pipeline = if current_shader_id == 0 {
                        default_pipeline
                    } else {
                        image_pipelines.get(&current_shader_id).unwrap()
                    };
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
                    screen: screen_size_data,
                    opacity: current_opacity,
                    shader_opacity: resolved.shader_opts.opacity,
                    _padding: [0.0; 2],
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

            current_atlas_index = img_entry.atlas_index;
            current_shader_id = shader_id;

            batch.push(InstanceData {
                pos: [opts.position()[0].as_f32(), opts.position()[1].as_f32()],
                rotation: opts.rotation(),
                size: [
                    img_entry.bounds.width.as_f32() * opts.scale()[0],
                    img_entry.bounds.height.as_f32() * opts.scale()[1],
                ],
                uv_rect,
            });
        }

        if !batch.is_empty() {
            let ai = current_atlas_index
                .expect("current_atlas_index should be Some if batch is not empty");
            let atlas_bg = &atlases.get(ai as usize).expect("atlas").bind_group;
            if let Ok(range) = image_renderer.upload_instances(queue, batch.as_slice())
            {
                let pipeline = if current_shader_id == 0 {
                    default_pipeline
                } else {
                    image_pipelines.get(&current_shader_id).unwrap()
                };
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
    }


    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_3d_internal<'a>(
        model_renderer: &mut ModelRenderer,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        scene_globals: &mut crate::graphics::model_raw::SceneGlobals,
        models: &[Option<MeshData>],
        skins: &[Option<SkinData>],
        images: &[Option<ImageEntry>],
        atlases: &[AtlasSlot],
        model_pipeline: &wgpu::RenderPipeline,
        instanced_model_pipeline: &wgpu::RenderPipeline,
        shadow_pipeline: &wgpu::RenderPipeline,
        instanced_shadow_pipeline: &wgpu::RenderPipeline,
        model_pipelines: &HashMap<u32, wgpu::RenderPipeline>,
        white_image_id: u32,
        black_image_id: u32,
        normal_image_id: u32,
        shadow_view: &wgpu::TextureView,
        irradiance_view: &wgpu::TextureView,
        prefiltered_view: &wgpu::TextureView,
        brdf_lut_view: &wgpu::TextureView,
        rpass: &mut wgpu::RenderPass<'a>,
        context: &Context,
        is_shadow_pass: bool,
        config_width: u32,
        config_height: u32,
    ) {
        let aspect = config_width as f32 / config_height as f32;
        let proj = crate::graphics::model_raw::create_perspective(aspect, std::f32::consts::PI / 4.0, 0.1, 1000.0);
        
        let view_mat = crate::graphics::model_raw::create_translation([0.0, 0.0, -5.0]); // Fallback view

        scene_globals.light_view_proj = [
            [0.1, 0.0, 0.0, 0.0],
            [0.0, 0.1, 0.0, 0.0],
            [0.0, 0.0, 0.05, 0.0],
            [0.0, 0.0, 0.5, 1.0],
        ];

        if !is_shadow_pass {
            model_renderer.upload_scene_globals(queue, scene_globals);
        }

        let lvp = scene_globals.light_view_proj;

        for command in &context.draw_list_3d {
            match command {
                crate::drawable::DrawCommand3D::Model(model, opts, shader_id, shader_opts, skin_id_cmd) => {
                    let model_mat = crate::graphics::model_raw::create_translation(opts.position);
                    let rot_mat = crate::graphics::model_raw::create_rotation(opts.rotation);
                    let scale_mat = crate::graphics::model_raw::create_scale(opts.scale);
                    let model_mat_all = crate::graphics::model_raw::multiply(model_mat, crate::graphics::model_raw::multiply(rot_mat, scale_mat));
                    
                    let mvp = if is_shadow_pass {
                        crate::graphics::model_raw::multiply(lvp, model_mat_all)
                    } else {
                        crate::graphics::model_raw::multiply(proj, crate::graphics::model_raw::multiply(view_mat, model_mat_all))
                    };

                    let base_globals = crate::graphics::model_raw::ModelGlobals {
                        mvp,
                        model: model_mat_all,
                        extra: [opts.opacity, 0.0, 0.0, 0.0],
                        ..Default::default()
                    };

                    for part in &model.parts {
                        if let Some(Some(mesh)) = models.get(part.id as usize) {
                            let mut globals = base_globals;
                            if !is_shadow_pass {
                                let get_tex_info = |img_id: Option<u32>, fallback_id: u32| -> [f32; 4] {
                                    let id = img_id.filter(|&id| images.get(id as usize).map(|v: &Option<ImageEntry>| v.is_some()).unwrap_or(false)).unwrap_or(fallback_id);
                                    let entry = images[id as usize].as_ref().unwrap();
                                    entry.uv_rect.unwrap_or([0.0, 0.0, 1.0, 1.0])
                                };
                                globals.albedo_uv = get_tex_info(part.material.albedo, white_image_id);
                                globals.pbr_uv = get_tex_info(part.material.pbr, black_image_id);
                                globals.normal_uv = get_tex_info(part.material.normal, normal_image_id);
                                globals.ao_uv = get_tex_info(part.material.occlusion, white_image_id);
                                globals.emissive_uv = get_tex_info(part.material.emissive, black_image_id);
                            }

                            if let Ok(offset) = model_renderer.upload_globals(queue, &globals) {
                                let pipeline = if is_shadow_pass {
                                    shadow_pipeline
                                } else if *shader_id == 0 {
                                    model_pipeline
                                } else {
                                    model_pipelines.get(shader_id).unwrap_or(model_pipeline)
                                };

                                rpass.set_pipeline(pipeline);
                                rpass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                                rpass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                                let mut bone_offset = 0;
                                if let Some(skin_id) = skin_id_cmd {
                                    if let Some(Some(skin)) = skins.get(*skin_id as usize) {
                                        if let Ok(off) = model_renderer.upload_bone_matrices(queue, &skin.bone_matrices) {
                                            bone_offset = off;
                                        }
                                    }
                                }

                                if is_shadow_pass {
                                    rpass.set_bind_group(0, &model_renderer.globals_bind_group, &[offset, 0]);
                                    rpass.set_bind_group(1, &model_renderer.bone_matrices_bind_group, &[bone_offset]);
                                } else {
                                    if let Ok(opts_offset) = model_renderer.upload_shader_opts_bytes(queue, shader_opts.as_bytes()) {
                                        rpass.set_bind_group(0, &model_renderer.globals_bind_group, &[offset, opts_offset]);
                                    }

                                    let get_view = |img_id: Option<u32>, fallback_id: u32| -> &wgpu::TextureView {
                                        let id = img_id.filter(|&id| images.get(id as usize).map(|v: &Option<ImageEntry>| v.is_some()).unwrap_or(false)).unwrap_or(fallback_id);
                                        let entry = images[id as usize].as_ref().unwrap();
                                        let ai = entry.atlas_index.unwrap_or(0);
                                        &atlases[ai as usize].texture.0.view
                                    };

                                    let tex_bg = model_renderer.create_texture_bind_group(
                                        device, 
                                        get_view(part.material.albedo, white_image_id),
                                        get_view(part.material.pbr, black_image_id),
                                        get_view(part.material.normal, normal_image_id),
                                        get_view(part.material.occlusion, white_image_id),
                                        get_view(part.material.emissive, black_image_id)
                                    );
                                    rpass.set_bind_group(1, &tex_bg, &[]);

                                    rpass.set_bind_group(2, &model_renderer.bone_matrices_bind_group, &[bone_offset]);

                                    let env_bg = model_renderer.create_environment_bind_group(
                                        device,
                                        shadow_view,
                                        irradiance_view,
                                        prefiltered_view,
                                        brdf_lut_view,
                                    );
                                    rpass.set_bind_group(3, &env_bg, &[]);
                                }

                                rpass.draw_indexed(0..mesh.index_count, 0, 0..1);
                            }
                        }
                    }
                }
                DrawCommand3D::ModelInstanced(model, opts, shader_id, shader_opts, skin_id_cmd, transforms) => {
                    let model_mat = crate::graphics::model_raw::create_translation(opts.position);
                    let rot_mat = crate::graphics::model_raw::create_rotation(opts.rotation);
                    let scale_mat = crate::graphics::model_raw::create_scale(opts.scale);
                    let model_mat_all = crate::graphics::model_raw::multiply(model_mat, crate::graphics::model_raw::multiply(rot_mat, scale_mat));
                    
                    let mvp = if is_shadow_pass {
                        crate::graphics::model_raw::multiply(lvp, model_mat_all)
                    } else {
                        crate::graphics::model_raw::multiply(proj, crate::graphics::model_raw::multiply(view_mat, model_mat_all))
                    };

                    let base_globals = crate::graphics::model_raw::ModelGlobals {
                        mvp,
                        model: model_mat_all,
                        extra: [opts.opacity, 0.0, 0.0, 0.0],
                        ..Default::default()
                    };

                    // Upload instance data
                    if let Err(e) = model_renderer.upload_instances(queue, transforms) {
                        eprintln!("[spot][render] Failed to upload instances: {}", e);
                        continue;
                    }

                    for part in &model.parts {
                        if let Some(Some(mesh)) = models.get(part.id as usize) {
                            let mut globals = base_globals;
                            if !is_shadow_pass {
                                let get_tex_info = |img_id: Option<u32>, fallback_id: u32| -> [f32; 4] {
                                    let id = img_id.filter(|&id| images.get(id as usize).map(|v: &Option<ImageEntry>| v.is_some()).unwrap_or(false)).unwrap_or(fallback_id);
                                    let entry = images[id as usize].as_ref().unwrap();
                                    entry.uv_rect.unwrap_or([0.0, 0.0, 1.0, 1.0])
                                };
                                globals.albedo_uv = get_tex_info(part.material.albedo, white_image_id);
                                globals.pbr_uv = get_tex_info(part.material.pbr, black_image_id);
                                globals.normal_uv = get_tex_info(part.material.normal, normal_image_id);
                                globals.ao_uv = get_tex_info(part.material.occlusion, white_image_id);
                                globals.emissive_uv = get_tex_info(part.material.emissive, black_image_id);
                            }

                            if let Ok(offset) = model_renderer.upload_globals(queue, &globals) {
                                let pipeline = if is_shadow_pass {
                                    instanced_shadow_pipeline
                                } else if *shader_id == 0 {
                                    instanced_model_pipeline
                                } else {
                                    // Custom shaders currently don't support instancing in this logic 
                                    // unless we also register an instanced variant of them.
                                    // For now, fallback to default instanced pipeline.
                                    instanced_model_pipeline
                                };

                                rpass.set_pipeline(pipeline);
                                rpass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                                rpass.set_vertex_buffer(1, model_renderer.instance_buffer.slice(..));
                                rpass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                                let mut bone_offset = 0;
                                if let Some(skin_id) = skin_id_cmd {
                                    if let Some(Some(skin)) = skins.get(*skin_id as usize) {
                                        if let Ok(off) = model_renderer.upload_bone_matrices(queue, &skin.bone_matrices) {
                                            bone_offset = off;
                                        }
                                    }
                                }

                                if is_shadow_pass {
                                    rpass.set_bind_group(0, &model_renderer.globals_bind_group, &[offset, 0]);
                                    rpass.set_bind_group(1, &model_renderer.bone_matrices_bind_group, &[bone_offset]);
                                } else {
                                    if let Ok(opts_offset) = model_renderer.upload_shader_opts_bytes(queue, shader_opts.as_bytes()) {
                                        rpass.set_bind_group(0, &model_renderer.globals_bind_group, &[offset, opts_offset]);
                                    }

                                    let get_view = |img_id: Option<u32>, fallback_id: u32| -> &wgpu::TextureView {
                                        let id = img_id.filter(|&id| images.get(id as usize).map(|v: &Option<ImageEntry>| v.is_some()).unwrap_or(false)).unwrap_or(fallback_id);
                                        let entry = images[id as usize].as_ref().unwrap();
                                        let ai = entry.atlas_index.unwrap_or(0);
                                        &atlases[ai as usize].texture.0.view
                                    };

                                    let tex_bg = model_renderer.create_texture_bind_group(
                                        device, 
                                        get_view(part.material.albedo, white_image_id),
                                        get_view(part.material.pbr, black_image_id),
                                        get_view(part.material.normal, normal_image_id),
                                        get_view(part.material.occlusion, white_image_id),
                                        get_view(part.material.emissive, black_image_id)
                                    );
                                    rpass.set_bind_group(1, &tex_bg, &[]);

                                    rpass.set_bind_group(2, &model_renderer.bone_matrices_bind_group, &[bone_offset]);

                                    let env_bg = model_renderer.create_environment_bind_group(
                                        device,
                                        shadow_view,
                                        irradiance_view,
                                        prefiltered_view,
                                        brdf_lut_view,
                                    );
                                    rpass.set_bind_group(3, &env_bg, &[]);
                                }

                                rpass.draw_indexed(0..mesh.index_count, 0, 0..transforms.len() as u32);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn draw_context(
        &mut self,
        surface: &wgpu::Surface<'_>,
        context: &Context,
    ) -> Result<(), wgpu::SurfaceError> {
        let _ = self.process_registrations();
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
        let (_lw, _lh) = context.window_logical_size();
        let sf = if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor
        } else {
            1.0
        };
        // No need to resize here anymore, we'll do it in draw_drawables_internal after getting the texture
        self.draw_drawables_internal(surface, drawables, sf, Some(context))
    }

    fn draw_drawables_internal(
        &mut self,
        surface: &wgpu::Surface<'_>,
        drawables: &[DrawCommand],
        scale_factor: f64,
        context: Option<&Context>,
    ) -> Result<(), wgpu::SurfaceError> {
        let frame = match surface.get_current_texture() {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[spot][graphics] get_current_texture failed: {:?}", e);
                return Err(e);
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("command_encoder"),
        });

        self.model_renderer.begin_frame();
        self.image_renderer.begin_frame();


        let width = self.config.width;
        let height = self.config.height;

        // 1. Shadow Pass (3D)
        if let Some(ctx) = context {
            if !ctx.draw_list_3d.is_empty() {
                let mut shadow_encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("shadow_encoder"),
                });
                {
                    let mut rpass = shadow_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("shadow_pass"),
                        color_attachments: &[],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &self.shadow_view,
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
                    
                    Self::render_3d_internal(
                        &mut self.model_renderer,
                        &self.queue,
                        &self.device,
                        &mut self.scene_globals,
                        &self.models,
                        &self.skins,
                        &self.images,
                        &self.atlases,
                        &self.model_pipeline,
                        &self.instanced_model_pipeline,
                        &self.shadow_pipeline,
                        &self.instanced_shadow_pipeline,
                        &self.model_pipelines,
                        self.white_image_id,
                        self.black_image_id,
                        self.normal_image_id,
                        &self.shadow_view,
                        &self.irradiance_view,
                        &self.prefiltered_view,
                        &self.brdf_lut_view,
                        &mut rpass,
                        ctx,
                        true,
                        width,
                        height,
                    );
                }
                self.queue.submit(std::iter::once(shadow_encoder.finish()));
            }
        }

        let width = self.config.width;
        let height = self.config.height;

        // 2. Main Color Pass
        {
            self.resolve_drawables(drawables, width, height);
            
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_render_pass"),
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
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

            // Draw 3D
            if let Some(ctx) = context {
                Self::render_3d_internal(
                    &mut self.model_renderer,
                    &self.queue,
                    &self.device,
                    &mut self.scene_globals,
                    &self.models,
                    &self.skins,
                    &self.images,
                    &self.atlases,
                    &self.model_pipeline,
                    &self.instanced_model_pipeline,
                    &self.shadow_pipeline,
                    &self.instanced_shadow_pipeline,
                    &self.model_pipelines,
                    self.white_image_id,
                    self.black_image_id,
                    self.normal_image_id,
                    &self.shadow_view,
                    &self.irradiance_view,
                    &self.prefiltered_view,
                    &self.brdf_lut_view,
                    &mut rpass,
                    ctx,
                    false,
                    width,
                    height,
                );
            }

            // Draw 2D
            let lw = width as f32 / scale_factor as f32;
            let lh = height as f32 / scale_factor as f32;
            let screen_size_data = [
                2.0 / lw,
                2.0 / lh,
                1.0 / lw,
                1.0 / lh,
            ];

            Self::render_batches_internal(
                &mut self.image_renderer,
                &self.queue,
                &self.atlases,
                &self.image_pipelines,
                &self.default_pipeline,
                &mut self.batch,
                &mut self.resolved_draws,
                &mut rpass,
                screen_size_data,
                width,
                height,
                scale_factor,
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }
}
