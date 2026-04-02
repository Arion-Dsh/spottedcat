//! Batch rendering and draw operations.

use crate::Context;
use crate::ShaderOpts;
use crate::drawable::{DrawCommand, DrawCommand3D};
use crate::graphics::model_raw::MaterialBindGroupKey;
use crate::image_raw::InstanceData;
use crate::pt::Pt;
use std::time::Instant;
use std::collections::HashMap;

use super::core::{AtlasSlot, Graphics, ResolvedDraw, SkinData};
use crate::graphics::model_raw::{MeshData, ModelRenderer};
use crate::image::ImageEntry;
use crate::image_raw::ImageRenderer;

pub(crate) struct RenderConfig<'a> {
    pub screen_size_data: [f32; 4],
    pub width: u32,
    pub height: u32,
    pub sf: f64,
    pub atlases: &'a [AtlasSlot],
    pub image_pipelines: &'a HashMap<u32, wgpu::RenderPipeline>,
    pub default_pipeline: &'a wgpu::RenderPipeline,
}

pub(crate) struct Render3DConfig<'a> {
    pub model_pipeline: &'a wgpu::RenderPipeline,
    pub instanced_model_pipeline: &'a wgpu::RenderPipeline,
    pub shadow_pipeline: &'a wgpu::RenderPipeline,
    pub instanced_shadow_pipeline: &'a wgpu::RenderPipeline,
    pub model_pipelines: &'a HashMap<u32, wgpu::RenderPipeline>,
    pub instanced_model_pipelines: &'a HashMap<u32, wgpu::RenderPipeline>,
    pub white_image_id: u32,
    pub black_image_id: u32,
    pub normal_image_id: u32,
    pub environment_bind_group: &'a wgpu::BindGroup,
    pub width: u32,
    pub height: u32,
}

type MaterialTextureBinding<'a> = (u32, [f32; 4], &'a wgpu::TextureView);
type MaterialTextureSet<'a> = (
    MaterialTextureBinding<'a>,
    MaterialTextureBinding<'a>,
    MaterialTextureBinding<'a>,
    MaterialTextureBinding<'a>,
    MaterialTextureBinding<'a>,
);

fn resolve_material_texture<'a>(
    images: &[Option<ImageEntry>],
    atlases: &'a [AtlasSlot],
    img_id: Option<u32>,
    fallback_id: u32,
) -> Option<(u32, [f32; 4], &'a wgpu::TextureView)> {
    let id = img_id
        .filter(|&id| images.get(id as usize).and_then(|v| v.as_ref()).is_some())
        .unwrap_or(fallback_id);
    let entry = images.get(id as usize).and_then(|v| v.as_ref())?;
    let atlas_index = entry.atlas_index?;
    let view = &atlases.get(atlas_index as usize)?.texture.0.view;
    let uv_rect = entry.uv_rect.unwrap_or([0.0, 0.0, 1.0, 1.0]);
    Some((atlas_index, uv_rect, view))
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

fn expect_default_material_texture<'a>(
    images: &[Option<ImageEntry>],
    atlases: &'a [AtlasSlot],
    image_id: u32,
    label: &str,
) -> MaterialTextureBinding<'a> {
    resolve_material_texture(images, atlases, Some(image_id), image_id).unwrap_or_else(|| {
        panic!(
            "[spot][render] default {} texture {} is unavailable",
            label, image_id
        )
    })
}

fn resolve_material_textures<'a>(
    images: &[Option<ImageEntry>],
    atlases: &'a [AtlasSlot],
    part: &crate::model::ModelPart,
    white_image_id: u32,
    black_image_id: u32,
    normal_image_id: u32,
) -> MaterialTextureSet<'a> {
    let white = expect_default_material_texture(images, atlases, white_image_id, "white");
    let black = expect_default_material_texture(images, atlases, black_image_id, "black");
    let normal_default =
        expect_default_material_texture(images, atlases, normal_image_id, "normal");

    let albedo = resolve_material_texture(images, atlases, part.material.albedo, white_image_id)
        .unwrap_or(white);
    let pbr = resolve_material_texture(images, atlases, part.material.pbr, black_image_id)
        .unwrap_or(black);
    let normal = resolve_material_texture(images, atlases, part.material.normal, normal_image_id)
        .unwrap_or(normal_default);
    let ao = resolve_material_texture(images, atlases, part.material.occlusion, white_image_id)
        .unwrap_or(white);
    let emissive =
        resolve_material_texture(images, atlases, part.material.emissive, black_image_id)
            .unwrap_or(black);

    (albedo, pbr, normal, ao, emissive)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct DrawCommand3DSortKey {
    instanced: bool,
    shader_id: u32,
    skin_id: u32,
    mesh_id: u32,
    albedo_id: u32,
    pbr_id: u32,
    normal_id: u32,
    ao_id: u32,
    emissive_id: u32,
}

fn draw_command_3d_is_transparent(command: &DrawCommand3D) -> bool {
    match command {
        DrawCommand3D::Model(_, opts, ..) | DrawCommand3D::ModelInstanced(_, opts, ..) => {
            opts.opacity < 1.0
        }
    }
}

fn draw_command_3d_sort_key(command: &DrawCommand3D) -> DrawCommand3DSortKey {
    match command {
        DrawCommand3D::Model(model, _, shader_id, _, skin_id) => DrawCommand3DSortKey {
            instanced: false,
            shader_id: *shader_id,
            skin_id: skin_id.unwrap_or(0),
            mesh_id: model.first_id(),
            albedo_id: model
                .parts
                .first()
                .and_then(|part| part.material.albedo)
                .unwrap_or(0),
            pbr_id: model
                .parts
                .first()
                .and_then(|part| part.material.pbr)
                .unwrap_or(0),
            normal_id: model
                .parts
                .first()
                .and_then(|part| part.material.normal)
                .unwrap_or(0),
            ao_id: model
                .parts
                .first()
                .and_then(|part| part.material.occlusion)
                .unwrap_or(0),
            emissive_id: model
                .parts
                .first()
                .and_then(|part| part.material.emissive)
                .unwrap_or(0),
        },
        DrawCommand3D::ModelInstanced(model, _, shader_id, _, skin_id, _) => DrawCommand3DSortKey {
            instanced: true,
            shader_id: *shader_id,
            skin_id: skin_id.unwrap_or(0),
            mesh_id: model.first_id(),
            albedo_id: model
                .parts
                .first()
                .and_then(|part| part.material.albedo)
                .unwrap_or(0),
            pbr_id: model
                .parts
                .first()
                .and_then(|part| part.material.pbr)
                .unwrap_or(0),
            normal_id: model
                .parts
                .first()
                .and_then(|part| part.material.normal)
                .unwrap_or(0),
            ao_id: model
                .parts
                .first()
                .and_then(|part| part.material.occlusion)
                .unwrap_or(0),
            emissive_id: model
                .parts
                .first()
                .and_then(|part| part.material.emissive)
                .unwrap_or(0),
        },
    }
}

impl Graphics {
    fn prepare_3d_command_order(&mut self, ctx: &Context) {
        self.opaque_draw_indices_3d.clear();
        self.transparent_draw_indices_3d.clear();

        self.opaque_draw_indices_3d
            .reserve(ctx.runtime.draw_list_3d.len());
        self.transparent_draw_indices_3d
            .reserve(ctx.runtime.draw_list_3d.len());

        for (index, command) in ctx.runtime.draw_list_3d.iter().enumerate() {
            if draw_command_3d_is_transparent(command) {
                self.transparent_draw_indices_3d.push(index);
            } else {
                self.opaque_draw_indices_3d.push(index);
            }
        }

        self.opaque_draw_indices_3d
            .sort_by_key(|&index| draw_command_3d_sort_key(&ctx.runtime.draw_list_3d[index]));
    }

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

                        self.resolved_draws.push(ResolvedDraw {
                            atlas_index,
                            bounds: entry.bounds,
                            uv_rect,
                            opts: cmd.opts,
                            shader_id: cmd.shader_id,
                            shader_opts: cmd.shader_opts,
                            layer: cmd.opts.layer(),
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
        // Sort by layer first, then atlas and shader to maximize batching
        resolved_draws.sort_by(|a, b| {
            a.layer
                .cmp(&b.layer)
                .then(a.atlas_index.cmp(&b.atlas_index))
                .then(a.shader_id.cmp(&b.shader_id))
        });

        let mut current_opacity = 1.0f32;

        // Upload initial engine globals
        let engine_globals = crate::image_raw::EngineGlobals {
            screen: config.screen_size_data,
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

        rpass.set_scissor_rect(0, 0, config.width.max(1), config.height.max(1));
        let mut last_set_scissor: Option<(u32, u32, u32, u32)> = None;

        for resolved in resolved_draws.iter() {
            let opts = resolved.opts;
            let shader_id = resolved.shader_id;
            let shader_opts = resolved.shader_opts;
            let draw_opacity = opts.opacity();

            let effective_user_globals = shader_opts;

            let state_changed = current_atlas_index != Some(resolved.atlas_index)
                || current_shader_id != shader_id
                || current_user_globals != effective_user_globals
                || current_clip != opts.get_clip()
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
                    let x0 = (clip[0].as_f32() * config.sf as f32).clamp(0.0, config.width as f32);
                    let y0 = (clip[1].as_f32() * config.sf as f32).clamp(0.0, config.height as f32);
                    let x1 = ((clip[0].as_f32() + clip[2].as_f32()) * config.sf as f32)
                        .clamp(0.0, config.width as f32);
                    let y1 = ((clip[1].as_f32() + clip[3].as_f32()) * config.sf as f32)
                        .clamp(0.0, config.height as f32);
                    let fw = (x1 - x0).max(0.0) as u32;
                    let fh = (y1 - y0).max(0.0) as u32;
                    if fw > 0 && fh > 0 {
                        (x0 as u32, y0 as u32, fw, fh)
                    } else {
                        (0, 0, 1, 1)
                    }
                } else {
                    (0, 0, config.width, config.height)
                };

                if last_set_scissor != Some((sx, sy, sw, sh)) {
                    rpass.set_scissor_rect(sx, sy, sw, sh);
                    last_set_scissor = Some((sx, sy, sw, sh));
                }
            }

            current_atlas_index = Some(resolved.atlas_index);
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
        config: Render3DConfig,
        rpass: &mut wgpu::RenderPass<'a>,
        ctx: &Context,
        opaque_draw_indices: &[usize],
        transparent_draw_indices: &[usize],
        is_shadow_pass: bool,
    ) {
        let mut camera = ctx.runtime.camera;
        camera.aspect = config.width as f32 / config.height as f32;
        let proj = camera.projection_matrix();
        let view_mat = camera.view_matrix();

        scene_globals.camera_pos = [camera.eye[0], camera.eye[1], camera.eye[2], 1.0];

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
        let mut current_pipeline: Option<*const wgpu::RenderPipeline> = None;
        let mut current_mesh_binding: Option<(u32, bool)> = None;
        let mut current_material_key: Option<MaterialBindGroupKey> = None;
        let mut current_shader_opts: Option<ShaderOpts> = None;
        let mut current_shader_opts_offset: u32 = 0;
        let mut current_bone_offset: Option<u32> = None;
        let mut environment_bound = false;

        for command in opaque_draw_indices
            .iter()
            .chain(transparent_draw_indices.iter())
            .map(|&index| &ctx.runtime.draw_list_3d[index])
        {
            match command {
                crate::drawable::DrawCommand3D::Model(
                    model,
                    opts,
                    shader_id,
                    shader_opts,
                    skin_id_cmd,
                ) => {
                    let model_mat = crate::graphics::model_raw::create_translation(opts.position);
                    let rot_mat = crate::graphics::model_raw::create_rotation(opts.rotation);
                    let scale_mat = crate::graphics::model_raw::create_scale(opts.scale);
                    let model_mat_all = crate::graphics::model_raw::multiply(
                        model_mat,
                        crate::graphics::model_raw::multiply(rot_mat, scale_mat),
                    );

                    let mvp = if is_shadow_pass {
                        crate::graphics::model_raw::multiply(lvp, model_mat_all)
                    } else {
                        crate::graphics::model_raw::multiply(
                            proj,
                            crate::graphics::model_raw::multiply(view_mat, model_mat_all),
                        )
                    };

                    let base_globals = crate::graphics::model_raw::ModelGlobals {
                        mvp,
                        model: model_mat_all,
                        extra: [opts.opacity, 0.0, 0.0, 0.0],
                        ..Default::default()
                    };
                    let pipeline = if is_shadow_pass {
                        config.shadow_pipeline
                    } else if *shader_id == 0 {
                        config.model_pipeline
                    } else {
                        config
                            .model_pipelines
                            .get(shader_id)
                            .unwrap_or(config.model_pipeline)
                    };
                    let mut bone_offset = 0;
                    if let Some(skin_id) = skin_id_cmd
                        && let Some(Some(skin)) = skins.get(*skin_id as usize)
                        && let Ok(off) =
                            model_renderer.bone_offset_for_skin(queue, *skin_id, &skin.bone_matrices)
                    {
                        bone_offset = off;
                    }
                    if !is_shadow_pass && current_shader_opts != Some(*shader_opts) {
                        if let Ok(offset) =
                            model_renderer.upload_shader_opts_bytes(queue, shader_opts.as_bytes())
                        {
                            current_shader_opts = Some(*shader_opts);
                            current_shader_opts_offset = offset;
                        }
                    }

                    for part in model.parts.iter() {
                        if let Some(Some(mesh)) = models.get(part.id as usize) {
                            let mut globals = base_globals;
                            let mut material_texture_data = None;
                            if !is_shadow_pass {
                                let (albedo, pbr, normal, ao, emissive) = resolve_material_textures(
                                    images,
                                    atlases,
                                    part,
                                    config.white_image_id,
                                    config.black_image_id,
                                    config.normal_image_id,
                                );

                                globals.albedo_uv = albedo.1;
                                globals.pbr_uv = pbr.1;
                                globals.normal_uv = normal.1;
                                globals.ao_uv = ao.1;
                                globals.emissive_uv = emissive.1;
                                material_texture_data = Some((albedo, pbr, normal, ao, emissive));
                            }

                            if let Ok(offset) = model_renderer.upload_globals(queue, &globals) {
                                let pipeline_ptr = pipeline as *const wgpu::RenderPipeline;
                                if current_pipeline != Some(pipeline_ptr) {
                                    rpass.set_pipeline(pipeline);
                                    current_pipeline = Some(pipeline_ptr);
                                }
                                if current_mesh_binding != Some((part.id, false)) {
                                    rpass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                                    rpass.set_index_buffer(
                                        mesh.index_buffer.slice(..),
                                        wgpu::IndexFormat::Uint32,
                                    );
                                    current_mesh_binding = Some((part.id, false));
                                }

                                if is_shadow_pass {
                                    rpass.set_bind_group(
                                        0,
                                        &model_renderer.globals_bind_group,
                                        &[offset, 0],
                                    );
                                    if current_bone_offset != Some(bone_offset) {
                                        rpass.set_bind_group(
                                            1,
                                            &model_renderer.bone_matrices_bind_group,
                                            &[bone_offset],
                                        );
                                        current_bone_offset = Some(bone_offset);
                                    }
                                } else {
                                    rpass.set_bind_group(
                                        0,
                                        &model_renderer.globals_bind_group,
                                        &[offset, current_shader_opts_offset],
                                    );

                                    let (albedo, pbr, normal, ao, emissive) = material_texture_data
                                        .unwrap_or_else(|| {
                                            panic!(
                                                "[spot][render] material textures missing after resolution for mesh {}",
                                                part.id
                                            )
                                        });
                                    let material_key = MaterialBindGroupKey {
                                        atlas_indices: [
                                            albedo.0, pbr.0, normal.0, ao.0, emissive.0,
                                        ],
                                    };
                                    let tex_bg = model_renderer.texture_bind_group_for_atlases(
                                        device,
                                        material_key,
                                        [albedo.2, pbr.2, normal.2, ao.2, emissive.2],
                                    );
                                    if current_material_key != Some(material_key) {
                                        rpass.set_bind_group(1, tex_bg, &[]);
                                        current_material_key = Some(material_key);
                                    }

                                    if current_bone_offset != Some(bone_offset) {
                                        rpass.set_bind_group(
                                            2,
                                            &model_renderer.bone_matrices_bind_group,
                                            &[bone_offset],
                                        );
                                        current_bone_offset = Some(bone_offset);
                                    }
                                    if !environment_bound {
                                        rpass.set_bind_group(3, config.environment_bind_group, &[]);
                                        environment_bound = true;
                                    }
                                }

                                rpass.draw_indexed(0..mesh.index_count, 0, 0..1);
                            }
                        }
                    }
                }
                DrawCommand3D::ModelInstanced(
                    model,
                    opts,
                    shader_id,
                    shader_opts,
                    skin_id_cmd,
                    transforms,
                ) => {
                    let model_mat = crate::graphics::model_raw::create_translation(opts.position);
                    let rot_mat = crate::graphics::model_raw::create_rotation(opts.rotation);
                    let scale_mat = crate::graphics::model_raw::create_scale(opts.scale);
                    let model_mat_all = crate::graphics::model_raw::multiply(
                        model_mat,
                        crate::graphics::model_raw::multiply(rot_mat, scale_mat),
                    );

                    let mvp = if is_shadow_pass {
                        crate::graphics::model_raw::multiply(lvp, model_mat_all)
                    } else {
                        crate::graphics::model_raw::multiply(
                            proj,
                            crate::graphics::model_raw::multiply(view_mat, model_mat_all),
                        )
                    };

                    let base_globals = crate::graphics::model_raw::ModelGlobals {
                        mvp,
                        model: model_mat_all,
                        extra: [opts.opacity, 0.0, 0.0, 0.0],
                        ..Default::default()
                    };
                    let pipeline = if is_shadow_pass {
                        config.instanced_shadow_pipeline
                    } else if *shader_id == 0 {
                        config.instanced_model_pipeline
                    } else {
                        config
                            .instanced_model_pipelines
                            .get(shader_id)
                            .unwrap_or(config.instanced_model_pipeline)
                    };
                    let mut bone_offset = 0;
                    if let Some(skin_id) = skin_id_cmd
                        && let Some(Some(skin)) = skins.get(*skin_id as usize)
                        && let Ok(off) =
                            model_renderer.bone_offset_for_skin(queue, *skin_id, &skin.bone_matrices)
                    {
                        bone_offset = off;
                    }
                    if !is_shadow_pass && current_shader_opts != Some(*shader_opts) {
                        if let Ok(offset) =
                            model_renderer.upload_shader_opts_bytes(queue, shader_opts.as_bytes())
                        {
                            current_shader_opts = Some(*shader_opts);
                            current_shader_opts_offset = offset;
                        }
                    }

                    // Upload instance data
                    if let Err(e) = model_renderer.upload_instances(queue, transforms.as_ref()) {
                        eprintln!("[spot][render] Failed to upload instances: {}", e);
                        continue;
                    }

                    for part in model.parts.iter() {
                        if let Some(Some(mesh)) = models.get(part.id as usize) {
                            let mut globals = base_globals;
                            let mut material_texture_data = None;
                            if !is_shadow_pass {
                                let (albedo, pbr, normal, ao, emissive) = resolve_material_textures(
                                    images,
                                    atlases,
                                    part,
                                    config.white_image_id,
                                    config.black_image_id,
                                    config.normal_image_id,
                                );

                                globals.albedo_uv = albedo.1;
                                globals.pbr_uv = pbr.1;
                                globals.normal_uv = normal.1;
                                globals.ao_uv = ao.1;
                                globals.emissive_uv = emissive.1;
                                material_texture_data = Some((albedo, pbr, normal, ao, emissive));
                            }

                            if let Ok(offset) = model_renderer.upload_globals(queue, &globals) {
                                let pipeline_ptr = pipeline as *const wgpu::RenderPipeline;
                                if current_pipeline != Some(pipeline_ptr) {
                                    rpass.set_pipeline(pipeline);
                                    current_pipeline = Some(pipeline_ptr);
                                }
                                if current_mesh_binding != Some((part.id, true)) {
                                    rpass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                                    rpass.set_vertex_buffer(
                                        1,
                                        model_renderer.instance_buffer.slice(..),
                                    );
                                    rpass.set_index_buffer(
                                        mesh.index_buffer.slice(..),
                                        wgpu::IndexFormat::Uint32,
                                    );
                                    current_mesh_binding = Some((part.id, true));
                                }

                                if is_shadow_pass {
                                    rpass.set_bind_group(
                                        0,
                                        &model_renderer.globals_bind_group,
                                        &[offset, 0],
                                    );
                                    if current_bone_offset != Some(bone_offset) {
                                        rpass.set_bind_group(
                                            1,
                                            &model_renderer.bone_matrices_bind_group,
                                            &[bone_offset],
                                        );
                                        current_bone_offset = Some(bone_offset);
                                    }
                                } else {
                                    rpass.set_bind_group(
                                        0,
                                        &model_renderer.globals_bind_group,
                                        &[offset, current_shader_opts_offset],
                                    );

                                    let (albedo, pbr, normal, ao, emissive) = material_texture_data
                                        .unwrap_or_else(|| {
                                            panic!(
                                                "[spot][render] material textures missing after resolution for instanced mesh {}",
                                                part.id
                                            )
                                        });
                                    let material_key = MaterialBindGroupKey {
                                        atlas_indices: [
                                            albedo.0, pbr.0, normal.0, ao.0, emissive.0,
                                        ],
                                    };
                                    let tex_bg = model_renderer.texture_bind_group_for_atlases(
                                        device,
                                        material_key,
                                        [albedo.2, pbr.2, normal.2, ao.2, emissive.2],
                                    );
                                    if current_material_key != Some(material_key) {
                                        rpass.set_bind_group(1, tex_bg, &[]);
                                        current_material_key = Some(material_key);
                                    }

                                    if current_bone_offset != Some(bone_offset) {
                                        rpass.set_bind_group(
                                            2,
                                            &model_renderer.bone_matrices_bind_group,
                                            &[bone_offset],
                                        );
                                        current_bone_offset = Some(bone_offset);
                                    }
                                    if !environment_bound {
                                        rpass.set_bind_group(3, config.environment_bind_group, &[]);
                                        environment_bound = true;
                                    }
                                }

                                rpass.draw_indexed(
                                    0..mesh.index_count,
                                    0,
                                    0..transforms.len() as u32,
                                );
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
        ctx: &mut Context,
    ) -> Result<(), wgpu::SurfaceError> {
        self.sync_assets(ctx).map_err(|e| {
            eprintln!("[spot][graphics] sync_assets failed: {:?}", e);
            wgpu::SurfaceError::Lost
        })?;
        let _ = self.process_registrations(ctx);
        let draws = std::mem::take(&mut ctx.runtime.draw_list);
        self.process_image_commands(ctx, &draws);
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

        self.model_renderer.begin_frame();
        self.image_renderer.begin_frame();

        let width = self.config.width;
        let height = self.config.height;
        if let Some(ctx_ref) = ctx.as_deref()
            && !ctx_ref.runtime.draw_list_3d.is_empty()
        {
            self.prepare_3d_command_order(ctx_ref);
        }

        // 1. Shadow Pass (3D)
        if let Some(ref mut ctx) = ctx
            && !ctx.runtime.draw_list_3d.is_empty()
        {
            let mut shadow_encoder =
                self.device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
                    &self.gpu_models,
                    &self.gpu_skins,
                    &ctx.registry.images,
                    &self.atlases,
                    Render3DConfig {
                        model_pipeline: &self.model_pipeline,
                        instanced_model_pipeline: &self.instanced_model_pipeline,
                        shadow_pipeline: &self.shadow_pipeline,
                        instanced_shadow_pipeline: &self.instanced_shadow_pipeline,
                        model_pipelines: &self.model_pipelines,
                        instanced_model_pipelines: &self.instanced_model_pipelines,
                        white_image_id: self.white_image_id,
                        black_image_id: self.black_image_id,
                        normal_image_id: self.normal_image_id,
                        environment_bind_group: &self.environment_bind_group,
                        width,
                        height,
                    },
                    &mut rpass,
                    ctx,
                    &self.opaque_draw_indices_3d,
                    &self.transparent_draw_indices_3d,
                    true,
                );
            }
            self.queue.submit(std::iter::once(shadow_encoder.finish()));
        }

        // 2. Main Color Pass
        {
            if let Some(ref mut ctx) = ctx {
                self.resolve_drawables(ctx, drawables, width, height);
            }

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
            if let Some(ref mut ctx) = ctx {
                Self::render_3d_internal(
                    &mut self.model_renderer,
                    &self.queue,
                    &self.device,
                    &mut self.scene_globals,
                    &self.gpu_models,
                    &self.gpu_skins,
                    &ctx.registry.images,
                    &self.atlases,
                    Render3DConfig {
                        model_pipeline: &self.model_pipeline,
                        instanced_model_pipeline: &self.instanced_model_pipeline,
                        shadow_pipeline: &self.shadow_pipeline,
                        instanced_shadow_pipeline: &self.instanced_shadow_pipeline,
                        model_pipelines: &self.model_pipelines,
                        instanced_model_pipelines: &self.instanced_model_pipelines,
                        white_image_id: self.white_image_id,
                        black_image_id: self.black_image_id,
                        normal_image_id: self.normal_image_id,
                        environment_bind_group: &self.environment_bind_group,
                        width,
                        height,
                    },
                    &mut rpass,
                    ctx,
                    &self.opaque_draw_indices_3d,
                    &self.transparent_draw_indices_3d,
                    false,
                );
            }

            // Draw 2D
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
                    width,
                    height,
                    sf: scale_factor,
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
