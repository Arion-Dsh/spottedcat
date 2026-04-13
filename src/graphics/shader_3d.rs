use crate::model::RawVertex;

use super::core::Graphics;

impl Graphics {
    pub(crate) fn create_default_model_pipelines(
        &self,
    ) -> (
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
    ) {
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
                    buffers: &[RawVertex::layout()],
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

        let transparent_model_pipeline =
            self.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("transparent_model_pipeline"),
                    layout: Some(&model_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &model_shader,
                        entry_point: Some("vs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[RawVertex::layout()],
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth24Plus,
                        depth_write_enabled: false,
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
                            RawVertex::layout(),
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

        let transparent_instanced_model_pipeline =
            self.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("transparent_instanced_model_pipeline"),
                    layout: Some(&model_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &instanced_model_shader,
                        entry_point: Some("vs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[
                            RawVertex::layout(),
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
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth24Plus,
                        depth_write_enabled: false,
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

        (
            model_pipeline,
            transparent_model_pipeline,
            instanced_model_pipeline,
            transparent_instanced_model_pipeline,
        )
    }

    pub(crate) fn restore_model_shader(&mut self, shader_id: u32, wgsl_source: &str) {
        self.create_custom_model_pipelines(shader_id, wgsl_source);
    }

    fn create_custom_model_pipelines(&mut self, shader_id: u32, wgsl_source: &str) {
        self.ensure_model_3d();
        let device = &self.device;
        let format = self.config.format;
        let model_3d = self.model_3d.as_mut().expect("ensured");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("custom_model_shader"),
            source: wgpu::ShaderSource::Wgsl(wgsl_source.into()),
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
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[RawVertex::layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
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
                module: &shader,
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

        let transparent_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("custom_transparent_model_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[RawVertex::layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
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
                module: &shader,
                entry_point: Some("vs_main_instanced"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    RawVertex::layout(),
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
                front_face: wgpu::FrontFace::Ccw,
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
                module: &shader,
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

        let transparent_instanced_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("custom_transparent_instanced_model_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main_instanced"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[
                        RawVertex::layout(),
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
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
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
            .transparent_model_pipelines
            .insert(shader_id, transparent_pipeline);
        self.ensure_model_3d()
            .instanced_model_pipelines
            .insert(shader_id, instanced_pipeline);
        self.ensure_model_3d()
            .transparent_instanced_model_pipelines
            .insert(shader_id, transparent_instanced_pipeline);
    }
}
