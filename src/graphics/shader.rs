//! Shader registration for custom image effects.

use crate::graphics::image_pipeline::ImagePipeline;
use crate::image_raw::InstanceData;
use crate::image_shader::{ImageShaderBlendMode, ImageShaderDesc};

use super::core::Graphics;

impl Graphics {
    fn create_image_pipeline_layout(&self, uses_extra_textures: bool) -> wgpu::PipelineLayout {
        let mut bind_group_layouts = vec![&self.image_renderer.texture_bind_group_layout];
        if uses_extra_textures {
            bind_group_layouts.push(&self.image_renderer.extra_texture_bind_group_layout);
        }
        bind_group_layouts.push(&self.image_renderer.user_globals_bind_group_layout);
        bind_group_layouts.push(&self.image_renderer.engine_globals_bind_group_layout);

        self.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("image_pipeline_layout"),
                bind_group_layouts: &bind_group_layouts,
                immediate_size: 0,
            })
    }

    fn image_blend_state(blend_mode: ImageShaderBlendMode) -> Option<wgpu::BlendState> {
        match blend_mode {
            ImageShaderBlendMode::Alpha => Some(wgpu::BlendState::ALPHA_BLENDING),
            ImageShaderBlendMode::Add => Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            }),
            ImageShaderBlendMode::Replace => None,
        }
    }

    fn create_image_pipeline_from_desc(
        &self,
        label: &'static str,
        desc: &ImageShaderDesc,
    ) -> ImagePipeline {
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(desc.source.clone().into()),
            });

        let uses_extra_textures = desc.uses_extra_textures();
        let pipeline_layout = self.create_image_pipeline_layout(uses_extra_textures);

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
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
                        blend: Self::image_blend_state(desc.blend_mode),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });

        ImagePipeline {
            pipeline,
            uses_extra_textures,
        }
    }

    fn create_default_image_pipeline(&self) -> wgpu::RenderPipeline {
        self.create_image_pipeline_from_desc(
            "image_shader",
            &ImageShaderDesc::from_wgsl(include_str!("../shaders/image.wgsl")),
        )
        .pipeline
    }

    pub(crate) fn rebuild_surface_format_dependent_pipelines(&mut self, ctx: &crate::Context) {
        self.default_pipeline = self.create_default_image_pipeline();
        #[cfg(feature = "model-3d")]
        if self.model_3d.is_some() {
            let (model_pipeline, instanced_model_pipeline) = self.create_default_model_pipelines();
            self.model_3d_mut().expect("checked Some").model_pipeline = model_pipeline;
            self.model_3d_mut()
                .expect("checked Some")
                .instanced_model_pipeline = instanced_model_pipeline;
            #[cfg(feature = "effects")]
            {
                self.model_3d_mut()
                    .expect("checked Some")
                    .fog_background_pipeline = Self::create_fog_background_pipeline(
                    &self.device,
                    self.config.format,
                    &self
                        .model_3d()
                        .expect("checked Some")
                        .fog_background_bind_group_layout,
                    self.adapter.get_info().backend,
                );
            }
        }

        self.image_pipelines.clear();
        #[cfg(feature = "model-3d")]
        if self.model_3d.is_some() {
            self.model_3d_mut()
                .expect("checked Some")
                .model_pipelines
                .clear();
            self.model_3d_mut()
                .expect("checked Some")
                .instanced_model_pipelines
                .clear();
        }
        for (&id, desc) in &ctx.registry.image_shaders {
            if id != 0 {
                self.restore_image_shader(id, desc);
            }
        }
        #[cfg(feature = "model-3d")]
        if self.model_3d.is_some() {
            for (&id, source) in &ctx.registry.model_3d.model_shaders {
                if id != 0 {
                    self.restore_model_shader(id, source);
                }
            }
        }

        self.pipelines_dirty = false;
    }

    pub(crate) fn restore_image_shader(&mut self, shader_id: u32, desc: &ImageShaderDesc) {
        let pipeline = self.create_image_pipeline_from_desc("custom_image_shader", desc);
        self.image_pipelines.insert(shader_id, pipeline);
    }
}
