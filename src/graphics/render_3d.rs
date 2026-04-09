use crate::Context;
use crate::ShaderOpts;
use crate::drawable::DrawCommand3D;
use crate::graphics::model_raw::MaterialBindGroupKey;
use crate::graphics::model_raw::{MeshData, ModelRenderer};
use crate::image::ImageEntry;

use super::core::{AtlasSlot, Graphics};
use crate::model::SkinData;

pub(super) struct Render3DConfig<'a> {
    pub model_pipeline: &'a wgpu::RenderPipeline,
    pub instanced_model_pipeline: &'a wgpu::RenderPipeline,
    pub shadow_pipeline: &'a wgpu::RenderPipeline,
    pub instanced_shadow_pipeline: &'a wgpu::RenderPipeline,
    pub model_pipelines: &'a std::collections::HashMap<u32, wgpu::RenderPipeline>,
    pub instanced_model_pipelines: &'a std::collections::HashMap<u32, wgpu::RenderPipeline>,
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

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(0.0001);
    [v[0] / len, v[1] / len, v[2] / len]
}

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

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
    pub(super) fn prepare_3d_command_order(&mut self, ctx: &Context) {
        let model_3d = self.ensure_model_3d();
        model_3d.opaque_draw_indices_3d.clear();
        model_3d.transparent_draw_indices_3d.clear();

        model_3d
            .opaque_draw_indices_3d
            .reserve(ctx.runtime.model_3d.draw_list.len());
        model_3d
            .transparent_draw_indices_3d
            .reserve(ctx.runtime.model_3d.draw_list.len());

        for (index, command) in ctx.runtime.model_3d.draw_list.iter().enumerate() {
            if draw_command_3d_is_transparent(command) {
                model_3d.transparent_draw_indices_3d.push(index);
            } else {
                model_3d.opaque_draw_indices_3d.push(index);
            }
        }

        model_3d
            .opaque_draw_indices_3d
            .sort_by_key(|&index| draw_command_3d_sort_key(&ctx.runtime.model_3d.draw_list[index]));
    }

    pub(super) fn render_shadow_pass(&mut self, ctx: &mut Context, width: u32, height: u32) {
        self.ensure_model_3d();
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
                    view: &self.model_3d().expect("ensured").shadow_view,
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

            let queue = &self.queue;
            let device = &self.device;
            let atlases = &self.atlases;
            let model_3d = self.model_3d.as_mut().expect("ensured");
            Self::render_3d_internal(
                &mut model_3d.model_renderer,
                queue,
                device,
                &mut model_3d.scene_globals,
                &model_3d.gpu_models,
                &model_3d.gpu_skins,
                &ctx.registry.images,
                atlases,
                Render3DConfig {
                    model_pipeline: &model_3d.model_pipeline,
                    instanced_model_pipeline: &model_3d.instanced_model_pipeline,
                    shadow_pipeline: &model_3d.shadow_pipeline,
                    instanced_shadow_pipeline: &model_3d.instanced_shadow_pipeline,
                    model_pipelines: &model_3d.model_pipelines,
                    instanced_model_pipelines: &model_3d.instanced_model_pipelines,
                    white_image_id: model_3d.white_image_id,
                    black_image_id: model_3d.black_image_id,
                    normal_image_id: model_3d.normal_image_id,
                    environment_bind_group: &model_3d.environment_bind_group,
                    width,
                    height,
                },
                &mut rpass,
                ctx,
                &model_3d.opaque_draw_indices_3d,
                &model_3d.transparent_draw_indices_3d,
                true,
            );
        }
        self.queue.submit(std::iter::once(shadow_encoder.finish()));
    }

    pub(super) fn render_main_3d_pass<'a>(
        &mut self,
        ctx: &mut Context,
        width: u32,
        height: u32,
        rpass: &mut wgpu::RenderPass<'a>,
    ) {
        let queue = &self.queue;
        let device = &self.device;
        let atlases = &self.atlases;
        let model_3d = self.model_3d.as_mut().expect("ensured");
        Self::render_3d_internal(
            &mut model_3d.model_renderer,
            queue,
            device,
            &mut model_3d.scene_globals,
            &model_3d.gpu_models,
            &model_3d.gpu_skins,
            &ctx.registry.images,
            atlases,
            Render3DConfig {
                model_pipeline: &model_3d.model_pipeline,
                instanced_model_pipeline: &model_3d.instanced_model_pipeline,
                shadow_pipeline: &model_3d.shadow_pipeline,
                instanced_shadow_pipeline: &model_3d.instanced_shadow_pipeline,
                model_pipelines: &model_3d.model_pipelines,
                instanced_model_pipelines: &model_3d.instanced_model_pipelines,
                white_image_id: model_3d.white_image_id,
                black_image_id: model_3d.black_image_id,
                normal_image_id: model_3d.normal_image_id,
                environment_bind_group: &model_3d.environment_bind_group,
                width,
                height,
            },
            rpass,
            ctx,
            &model_3d.opaque_draw_indices_3d,
            &model_3d.transparent_draw_indices_3d,
            false,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_3d_internal<'a>(
        model_renderer: &mut ModelRenderer,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        scene_globals: &mut crate::model::SceneGlobals,
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
        let mut camera = ctx.runtime.model_3d.camera;
        camera.aspect = config.width as f32 / config.height as f32;
        let proj = camera.projection_matrix();
        let view_mat = camera.view_matrix();
        let forward = normalize3([
            camera.target[0] - camera.eye[0],
            camera.target[1] - camera.eye[1],
            camera.target[2] - camera.eye[2],
        ]);
        let right = normalize3(cross3(camera.up, forward));
        let up = cross3(forward, right);

        scene_globals.camera_pos = [camera.eye[0], camera.eye[1], camera.eye[2], 1.0];
        scene_globals.camera_right = [right[0], right[1], right[2], 0.0];
        scene_globals.camera_up = [up[0], up[1], up[2], 0.0];
        scene_globals.camera_forward = [forward[0], forward[1], forward[2], 0.0];
        scene_globals.projection_params = [proj[0][0], proj[1][1], camera.znear, camera.zfar];

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
            .map(|&index| &ctx.runtime.model_3d.draw_list[index])
        {
            match command {
                DrawCommand3D::Model(model, opts, shader_id, shader_opts, skin_id_cmd) => {
                    let model_mat = crate::math::mat4::from_translation(opts.position);
                    let rot_mat = crate::math::mat4::from_rotation(opts.rotation);
                    let scale_mat = crate::math::mat4::from_scale(opts.scale);
                    let model_mat_all = crate::math::mat4::multiply(
                        model_mat,
                        crate::math::mat4::multiply(rot_mat, scale_mat),
                    );

                    let mvp = if is_shadow_pass {
                        crate::math::mat4::multiply(lvp, model_mat_all)
                    } else {
                        crate::math::mat4::multiply(
                            proj,
                            crate::math::mat4::multiply(view_mat, model_mat_all),
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
                        && let Ok(off) = model_renderer.bone_offset_for_skin(
                            queue,
                            *skin_id,
                            &skin.bone_matrices,
                        )
                    {
                        bone_offset = off;
                    }
                    if !is_shadow_pass
                        && current_shader_opts != Some(*shader_opts)
                        && let Ok(offset) =
                            model_renderer.upload_shader_opts_bytes(queue, shader_opts.as_bytes())
                    {
                        current_shader_opts = Some(*shader_opts);
                        current_shader_opts_offset = offset;
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
                    let model_mat = crate::math::mat4::from_translation(opts.position);
                    let rot_mat = crate::math::mat4::from_rotation(opts.rotation);
                    let scale_mat = crate::math::mat4::from_scale(opts.scale);
                    let model_mat_all = crate::math::mat4::multiply(
                        model_mat,
                        crate::math::mat4::multiply(rot_mat, scale_mat),
                    );

                    let mvp = if is_shadow_pass {
                        crate::math::mat4::multiply(lvp, model_mat_all)
                    } else {
                        crate::math::mat4::multiply(
                            proj,
                            crate::math::mat4::multiply(view_mat, model_mat_all),
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
                        && let Ok(off) = model_renderer.bone_offset_for_skin(
                            queue,
                            *skin_id,
                            &skin.bone_matrices,
                        )
                    {
                        bone_offset = off;
                    }
                    if !is_shadow_pass
                        && current_shader_opts != Some(*shader_opts)
                        && let Ok(offset) =
                            model_renderer.upload_shader_opts_bytes(queue, shader_opts.as_bytes())
                    {
                        current_shader_opts = Some(*shader_opts);
                        current_shader_opts_offset = offset;
                    }

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
}
