use crate::model::Vertex;

use super::Graphics;

fn inject_user_hooks(base_template: &str, user_functions: &str) -> String {
    let mut combined_shader = base_template.to_string();

    if let Some(vs_start) = user_functions.find("fn user_vs_hook") {
        let vs_body_start = user_functions[vs_start..]
            .find('{')
            .map(|i| vs_start + i + 1)
            .unwrap_or(vs_start);
        let vs_end = user_functions[vs_body_start..]
            .find("fn user_fs_hook")
            .map(|rel| vs_body_start + rel)
            .unwrap_or(user_functions.len());
        let vs_body_end = user_functions[..vs_end].rfind('}').unwrap_or(vs_end);
        let vs_src = user_functions[vs_body_start..vs_body_end].trim();
        if !vs_src.is_empty() {
            let marker = "// USER_VS_HOOK";
            if let Some(pos) = combined_shader.rfind(marker) {
                combined_shader.insert_str(pos + marker.len(), &format!("\n{{\n{}\n}}", vs_src));
            }
        }
    }

    if let Some(fs_start) = user_functions.find("fn user_fs_hook") {
        let fs_body_start = user_functions[fs_start..]
            .find('{')
            .map(|i| fs_start + i + 1)
            .unwrap_or(fs_start);
        let fs_end = user_functions.len();
        let fs_body_end = user_functions[..fs_end].rfind('}').unwrap_or(fs_end);
        let fs_src = user_functions[fs_body_start..fs_body_end].trim();
        if !fs_src.is_empty() {
            let marker = "// USER_FS_HOOK";
            if let Some(pos) = combined_shader.rfind(marker) {
                combined_shader.insert_str(pos + marker.len(), &format!("\n{{\n{}\n}}", fs_src));
            }
        }
    }

    combined_shader
}

impl Graphics {
    pub(crate) fn create_default_model_pipelines(
        &self,
    ) -> (wgpu::RenderPipeline, wgpu::RenderPipeline) {
        let model_3d = self
            .model_3d()
            .expect("model_3d must exist before creating 3D pipelines");
        let model_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("model_shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/model.wgsl").into()),
            });

        let model_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("model_pipeline_layout"),
                    bind_group_layouts: &[
                        &model_3d.model_renderer.globals_bind_group_layout,
                        &model_3d.model_renderer.texture_bind_group_layout,
                        &model_3d.model_renderer.bone_matrices_bind_group_layout,
                        &model_3d.model_renderer.environment_bind_group_layout,
                    ],
                    immediate_size: 0,
                });

        let model_pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("model_pipeline"),
                layout: Some(&model_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &model_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[Vertex::layout()],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &model_shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });

        let instanced_model_shader =
            self.device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("model_instanced_shader"),
                    source: wgpu::ShaderSource::Wgsl(
                        include_str!("../shaders/model_instanced.wgsl").into(),
                    ),
                });

        let instanced_model_pipeline =
            self.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("instanced_model_pipeline"),
                    layout: Some(&model_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &instanced_model_shader,
                        entry_point: Some("vs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[
                            Vertex::layout(),
                            wgpu::VertexBufferLayout {
                                array_stride: 64,
                                step_mode: wgpu::VertexStepMode::Instance,
                                attributes: &[
                                    wgpu::VertexAttribute {
                                        offset: 0,
                                        shader_location: 5,
                                        format: wgpu::VertexFormat::Float32x4,
                                    },
                                    wgpu::VertexAttribute {
                                        offset: 16,
                                        shader_location: 6,
                                        format: wgpu::VertexFormat::Float32x4,
                                    },
                                    wgpu::VertexAttribute {
                                        offset: 32,
                                        shader_location: 7,
                                        format: wgpu::VertexFormat::Float32x4,
                                    },
                                    wgpu::VertexAttribute {
                                        offset: 48,
                                        shader_location: 8,
                                        format: wgpu::VertexFormat::Float32x4,
                                    },
                                ],
                            },
                        ],
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        front_face: wgpu::FrontFace::Cw,
                        cull_mode: Some(wgpu::Face::Back),
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth24Plus,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Less,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &instanced_model_shader,
                        entry_point: Some("fs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: self.config.format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview_mask: None,
                    cache: None,
                });

        (model_pipeline, instanced_model_pipeline)
    }

    pub(crate) fn restore_model_shader(&mut self, shader_id: u32, user_functions: &str) {
        self.create_custom_model_pipelines(shader_id, user_functions);
    }

    fn create_custom_model_pipelines(&mut self, shader_id: u32, user_functions: &str) {
        self.ensure_model_3d();
        let device = &self.device;
        let format = self.config.format;
        let model_3d = self.model_3d.as_mut().expect("ensured");
        let standard_shader_src =
            inject_user_hooks(include_str!("../shaders/model.wgsl"), user_functions);
        let instanced_shader_src = inject_user_hooks(
            include_str!("../shaders/model_instanced.wgsl"),
            user_functions,
        );

        let standard_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("custom_model_shader"),
            source: wgpu::ShaderSource::Wgsl(standard_shader_src.into()),
        });
        let instanced_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("custom_model_instanced_shader"),
            source: wgpu::ShaderSource::Wgsl(instanced_shader_src.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("custom_model_pipeline_layout"),
            bind_group_layouts: &[
                &model_3d.model_renderer.globals_bind_group_layout,
                &model_3d.model_renderer.texture_bind_group_layout,
                &model_3d.model_renderer.bone_matrices_bind_group_layout,
                &model_3d.model_renderer.environment_bind_group_layout,
            ],
            immediate_size: 0,
        });

        let standard_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("custom_model_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &standard_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Vertex::layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &standard_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let instanced_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("custom_instanced_model_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &instanced_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    Vertex::layout(),
                    wgpu::VertexBufferLayout {
                        array_stride: 64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 5,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: 16,
                                shader_location: 6,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: 32,
                                shader_location: 7,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: 48,
                                shader_location: 8,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                        ],
                    },
                ],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &instanced_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        self.ensure_model_3d()
            .model_pipelines
            .insert(shader_id, standard_pipeline);
        self.ensure_model_3d()
            .instanced_model_pipelines
            .insert(shader_id, instanced_pipeline);
    }
}
