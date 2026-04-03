use std::collections::HashMap;

use crate::drawable::DrawCommand3D;
use crate::graphics::model_raw::{MeshData, ModelRenderer};
use crate::image::ImageEntry;
use crate::model::Vertex;

use super::core::{AtlasSlot, Graphics};

#[derive(Debug, Clone)]
pub struct SkinData {
    pub bones: Vec<Bone>,
    pub bone_matrices: Vec<[[f32; 4]; 4]>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bone {
    pub parent_index: Option<usize>,
    pub inverse_bind_matrix: [[f32; 4]; 4],
}

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            eye: [0.0, 0.0, 5.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            aspect: 1.0,
            fovy: std::f32::consts::PI / 4.0,
            znear: 0.1,
            zfar: 1000.0,
        }
    }
}

impl Camera {
    pub fn view_matrix(&self) -> [[f32; 4]; 4] {
        crate::math::mat4::look_at(self.eye, self.target, self.up)
    }

    pub fn projection_matrix(&self) -> [[f32; 4]; 4] {
        crate::math::projection::perspective(self.aspect, self.fovy, self.znear, self.zfar)
    }
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
) -> Option<MaterialTextureBinding<'a>> {
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
            "[spot][graphics] default {} texture {} is unavailable during prewarm",
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

pub(crate) struct Graphics3D {
    pub(crate) model_pipelines: HashMap<u32, wgpu::RenderPipeline>,
    pub(crate) instanced_model_pipelines: HashMap<u32, wgpu::RenderPipeline>,
    pub(crate) opaque_draw_indices_3d: Vec<usize>,
    pub(crate) transparent_draw_indices_3d: Vec<usize>,
    pub(crate) model_renderer: ModelRenderer,
    pub(crate) model_pipeline: wgpu::RenderPipeline,
    pub(crate) instanced_model_pipeline: wgpu::RenderPipeline,
    #[cfg(feature = "effects")]
    pub(crate) fog_background_bind_group_layout: wgpu::BindGroupLayout,
    #[cfg(feature = "effects")]
    pub(crate) fog_background_bind_group: wgpu::BindGroup,
    #[cfg(feature = "effects")]
    pub(crate) fog_background_pipeline: wgpu::RenderPipeline,
    pub(crate) gpu_models: Vec<Option<MeshData>>,
    pub(crate) gpu_skins: Vec<Option<SkinData>>,
    pub(crate) depth_texture: wgpu::Texture,
    pub(crate) depth_view: wgpu::TextureView,
    pub(crate) white_image_id: u32,
    pub(crate) black_image_id: u32,
    pub(crate) normal_image_id: u32,
    #[allow(dead_code)]
    pub(crate) shadow_texture: wgpu::Texture,
    pub(crate) shadow_view: wgpu::TextureView,
    pub(crate) shadow_pipeline: wgpu::RenderPipeline,
    pub(crate) instanced_shadow_pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    pub(crate) irradiance_texture: wgpu::Texture,
    #[allow(dead_code)]
    pub(crate) prefiltered_texture: wgpu::Texture,
    #[allow(dead_code)]
    pub(crate) brdf_lut_texture: wgpu::Texture,
    pub(crate) environment_bind_group: wgpu::BindGroup,
    pub(crate) scene_globals: crate::graphics::model_raw::SceneGlobals,
}

impl Graphics {
    pub(crate) fn model_3d(&self) -> Option<&Graphics3D> {
        self.model_3d.as_ref()
    }

    pub(crate) fn model_3d_mut(&mut self) -> Option<&mut Graphics3D> {
        self.model_3d.as_mut()
    }

    pub(crate) fn ensure_model_3d(&mut self) -> &mut Graphics3D {
        if self.model_3d.is_none() {
            let width = self.config.width.max(1);
            let height = self.config.height.max(1);
            let backend = self.adapter.get_info().backend;
            self.model_3d = Some(Self::build_model_3d(
                &self.device,
                &self.config,
                width,
                height,
                backend,
            ));
        }
        self.model_3d
            .as_mut()
            .expect("Graphics3D must exist after ensure_model_3d")
    }

    #[cfg(feature = "effects")]
    pub(crate) fn create_fog_background_bind_group_layout(
        device: &wgpu::Device,
    ) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fog_background_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<
                            crate::graphics::model_raw::SceneGlobals,
                        >()
                            as u64),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Depth,
                    },
                    count: None,
                },
            ],
        })
    }

    #[cfg(feature = "effects")]
    pub(crate) fn create_fog_background_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        scene_globals_buffer: &wgpu::Buffer,
        depth_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fog_background_bg"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: scene_globals_buffer,
                        offset: 0,
                        size: std::num::NonZeroU64::new(std::mem::size_of::<
                            crate::graphics::model_raw::SceneGlobals,
                        >() as u64),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
            ],
        })
    }

    #[cfg(feature = "effects")]
    pub(crate) fn create_fog_background_pipeline(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        bind_group_layout: &wgpu::BindGroupLayout,
        backend: wgpu::Backend,
    ) -> wgpu::RenderPipeline {
        let shader_source = if backend == wgpu::Backend::Gl {
            eprintln!(
                "[spot][3d] Using fog background fallback shader on GL backend because depth textureLoad is unsupported."
            );
            include_str!("../shaders/fog_background_fallback.wgsl")
        } else {
            include_str!("../shaders/fog_background.wgsl")
        };
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fog_background_shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fog_background_pipeline_layout"),
            bind_group_layouts: &[bind_group_layout],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("fog_background_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
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
        })
    }

    fn build_model_3d(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        width: u32,
        height: u32,
        _backend: wgpu::Backend,
    ) -> Graphics3D {
        let model_renderer = ModelRenderer::new(device);

        let shadow_size = 1024;
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow_texture"),
            size: wgpu::Extent3d {
                width: shadow_size,
                height: shadow_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_texture.create_view(&Default::default());

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&Default::default());

        let irradiance_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("irr_stub"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let irradiance_view = irradiance_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });

        let prefiltered_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pref_stub"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let prefiltered_view = prefiltered_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });

        let brdf_lut_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("brdf_stub"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let brdf_lut_view = brdf_lut_texture.create_view(&Default::default());
        let environment_bind_group = model_renderer.create_environment_bind_group(
            device,
            &shadow_view,
            &irradiance_view,
            &prefiltered_view,
            &brdf_lut_view,
        );

        let model_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("model_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/model.wgsl").into()),
        });

        let model_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("model_pipeline_layout"),
                bind_group_layouts: &[
                    &model_renderer.globals_bind_group_layout,
                    &model_renderer.texture_bind_group_layout,
                    &model_renderer.bone_matrices_bind_group_layout,
                    &model_renderer.environment_bind_group_layout,
                ],
                immediate_size: 0,
            });

        let model_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let instanced_model_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("model_instanced_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/model_instanced.wgsl").into(),
            ),
        });

        let instanced_model_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                        format: config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });

        #[cfg(feature = "effects")]
        let fog_background_bind_group_layout =
            Self::create_fog_background_bind_group_layout(device);
        #[cfg(feature = "effects")]
        let fog_background_bind_group = Self::create_fog_background_bind_group(
            device,
            &fog_background_bind_group_layout,
            &model_renderer.scene_globals_buffer,
            &depth_view,
        );
        #[cfg(feature = "effects")]
        let fog_background_pipeline = Self::create_fog_background_pipeline(
            device,
            config.format,
            &fog_background_bind_group_layout,
            _backend,
        );

        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("shadow_pipeline_layout"),
                bind_group_layouts: &[
                    &model_renderer.globals_bind_group_layout,
                    &model_renderer.bone_matrices_bind_group_layout,
                ],
                immediate_size: 0,
            });

        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/shadow.wgsl").into()),
        });

        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow_pipeline"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shadow_shader,
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
            fragment: None,
            multiview_mask: None,
            cache: None,
        });

        let instanced_shadow_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("instanced_shadow_pipeline"),
                layout: Some(&shadow_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shadow_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[
                        Vertex::layout(),
                        wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<[[f32; 4]; 4]>()
                                as wgpu::BufferAddress,
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
                fragment: None,
                multiview_mask: None,
                cache: None,
            });

        Graphics3D {
            model_pipelines: HashMap::new(),
            instanced_model_pipelines: HashMap::new(),
            opaque_draw_indices_3d: Vec::new(),
            transparent_draw_indices_3d: Vec::new(),
            model_renderer,
            model_pipeline,
            instanced_model_pipeline,
            #[cfg(feature = "effects")]
            fog_background_bind_group_layout,
            #[cfg(feature = "effects")]
            fog_background_bind_group,
            #[cfg(feature = "effects")]
            fog_background_pipeline,
            gpu_models: Vec::new(),
            gpu_skins: Vec::new(),
            depth_texture,
            depth_view,
            shadow_texture,
            shadow_view,
            shadow_pipeline,
            instanced_shadow_pipeline,
            irradiance_texture,
            prefiltered_texture,
            brdf_lut_texture,
            environment_bind_group,
            white_image_id: 1,
            black_image_id: 2,
            normal_image_id: 3,
            scene_globals: crate::graphics::model_raw::SceneGlobals {
                camera_pos: [0.0, 0.0, 0.0, 0.0],
                camera_right: [1.0, 0.0, 0.0, 0.0],
                camera_up: [0.0, 1.0, 0.0, 0.0],
                camera_forward: [0.0, 0.0, -1.0, 0.0],
                projection_params: [1.0, 1.0, 0.1, 1000.0],
                ambient_color: [0.1, 0.1, 0.1, 1.0],
                fog_color: [0.0, 0.0, 0.0, 0.0],
                fog_distance: [0.0, 1.0, 1.0, 0.0],
                fog_height: [0.0, 1.0, 1.0, 0.0],
                fog_params: [0.0, 0.0, 0.0, 0.0],
                fog_background_zenith: [0.27, 0.38, 0.52, 0.38],
                fog_background_horizon: [0.75, 0.79, 0.80, 0.32],
                fog_background_nadir: [0.52, 0.56, 0.55, 0.18],
                fog_background_params: [0.05, 0.72, 0.55, 0.0],
                fog_sampling: [4.0, 10.0, 0.6, 0.0],
                lights: [crate::graphics::model_raw::Light {
                    position: [1.0, 1.0, 1.0, 0.0],
                    color: [1.0, 1.0, 1.0, 1.0],
                }; 4],
                light_view_proj: crate::math::mat4::identity(),
            },
        }
    }

    pub(crate) fn sync_new_runtime_3d_assets(
        &mut self,
        ctx: &mut crate::Context,
    ) -> anyhow::Result<()> {
        let missing_model_shader_ids = if let Some(model_3d) = self.model_3d() {
            ctx.registry
                .model_3d
                .model_shaders
                .keys()
                .copied()
                .filter(|&id| id != 0 && !model_3d.model_pipelines.contains_key(&id))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        for id in missing_model_shader_ids {
            if let Some(source) = ctx.registry.model_3d.model_shaders.get(&id) {
                self.restore_model_shader(id, source);
            }
        }

        if self.model_3d.is_some() {
            if self.model_3d().expect("checked Some").gpu_models.len()
                < ctx.registry.model_3d.models.len()
            {
                self.model_3d_mut()
                    .expect("checked Some")
                    .gpu_models
                    .resize_with(ctx.registry.model_3d.models.len(), || None);
            }
            for (idx, model_opt) in ctx.registry.model_3d.models.iter().enumerate() {
                if self.model_3d().expect("checked Some").gpu_models[idx].is_some()
                    || model_opt.is_none()
                {
                    continue;
                }
                let mesh_data = model_opt
                    .as_ref()
                    .expect("mesh present after is_none check");
                let gpu_mesh = MeshData::new(&self.device, &mesh_data.vertices, &mesh_data.indices);
                gpu_mesh.upload(&self.queue, &mesh_data.vertices, &mesh_data.indices);
                self.model_3d_mut().expect("checked Some").gpu_models[idx] = Some(gpu_mesh);
            }

            if self.model_3d().expect("checked Some").gpu_skins.len()
                < ctx.registry.model_3d.skins.len()
            {
                self.model_3d_mut()
                    .expect("checked Some")
                    .gpu_skins
                    .resize_with(ctx.registry.model_3d.skins.len(), || None);
            }
            for (idx, skin_opt) in ctx.registry.model_3d.skins.iter().enumerate() {
                if self.model_3d().expect("checked Some").gpu_skins[idx].is_some()
                    || skin_opt.is_none()
                {
                    continue;
                }
                self.model_3d_mut().expect("checked Some").gpu_skins[idx] = skin_opt.clone();
            }
        }

        Ok(())
    }

    pub(crate) fn prewarm_3d_materials(&mut self, ctx: &mut crate::Context) -> anyhow::Result<()> {
        if !ctx.runtime.model_3d.draw_list.is_empty() {
            self.ensure_model_3d();
            for command in &ctx.runtime.model_3d.draw_list {
                let model = match command {
                    DrawCommand3D::Model(model, ..) | DrawCommand3D::ModelInstanced(model, ..) => {
                        model
                    }
                };

                for part in model.parts.iter() {
                    let device = &self.device;
                    let atlases = &self.atlases;
                    let model_3d = self.model_3d.as_mut().expect("ensured above");
                    let (albedo, pbr, normal, ao, emissive) = resolve_material_textures(
                        &ctx.registry.images,
                        atlases,
                        part,
                        model_3d.white_image_id,
                        model_3d.black_image_id,
                        model_3d.normal_image_id,
                    );
                    let material_key = crate::graphics::model_raw::MaterialBindGroupKey {
                        atlas_indices: [albedo.0, pbr.0, normal.0, ao.0, emissive.0],
                    };
                    let _ = model_3d.model_renderer.texture_bind_group_for_atlases(
                        device,
                        material_key,
                        [albedo.2, pbr.2, normal.2, ao.2, emissive.2],
                    );
                }
            }
        }

        Ok(())
    }

    pub(crate) fn restore_3d_assets(&mut self, ctx: &mut crate::Context) {
        if self.model_3d.is_some() {
            self.model_3d_mut()
                .expect("checked Some")
                .model_pipelines
                .clear();
            self.model_3d_mut()
                .expect("checked Some")
                .instanced_model_pipelines
                .clear();

            for (&id, source) in &ctx.registry.model_3d.model_shaders {
                self.restore_model_shader(id, source);
            }

            self.model_3d_mut()
                .expect("checked Some")
                .gpu_models
                .clear();
            for model_opt in &ctx.registry.model_3d.models {
                if let Some(mesh_data) = model_opt {
                    let gpu_mesh =
                        MeshData::new(&self.device, &mesh_data.vertices, &mesh_data.indices);
                    gpu_mesh.upload(&self.queue, &mesh_data.vertices, &mesh_data.indices);
                    self.model_3d_mut()
                        .expect("checked Some")
                        .gpu_models
                        .push(Some(gpu_mesh));
                } else {
                    self.model_3d_mut()
                        .expect("checked Some")
                        .gpu_models
                        .push(None);
                }
            }

            self.model_3d_mut().expect("checked Some").gpu_skins.clear();
            for skin_opt in &ctx.registry.model_3d.skins {
                self.model_3d_mut()
                    .expect("checked Some")
                    .gpu_skins
                    .push(skin_opt.clone());
            }

            self.model_3d_mut()
                .expect("checked Some")
                .model_renderer
                .clear_texture_bind_group_cache();

            self.model_3d_mut().expect("checked Some").white_image_id = 1;
            self.model_3d_mut().expect("checked Some").black_image_id = 2;
            self.model_3d_mut().expect("checked Some").normal_image_id = 3;
        }
    }

    pub(crate) fn resize_3d_surface_resources(
        &mut self,
        width: u32,
        height: u32,
        old_width: u32,
        old_height: u32,
    ) {
        if self.model_3d.is_some()
            && (width != old_width || height != old_height || cfg!(target_os = "android"))
        {
            self.model_3d_mut().expect("checked Some").depth_texture =
                self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("depth_texture"),
                    size: wgpu::Extent3d {
                        width: self.config.width,
                        height: self.config.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Depth24Plus,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
            self.model_3d_mut().expect("checked Some").depth_view = self
                .model_3d()
                .expect("checked Some")
                .depth_texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            #[cfg(feature = "effects")]
            {
                self.model_3d_mut()
                    .expect("checked Some")
                    .fog_background_bind_group = Self::create_fog_background_bind_group(
                    &self.device,
                    &self
                        .model_3d()
                        .expect("checked Some")
                        .fog_background_bind_group_layout,
                    &self
                        .model_3d()
                        .expect("checked Some")
                        .model_renderer
                        .scene_globals_buffer,
                    &self.model_3d().expect("checked Some").depth_view,
                );
            }
        }
    }

    pub fn create_mesh(
        &mut self,
        ctx: &mut crate::Context,
        vertices: &[Vertex],
        indices: &[u32],
    ) -> anyhow::Result<u32> {
        let id = ctx.registry.model_3d.next_mesh_id;
        ctx.registry.model_3d.next_mesh_id += 1;

        while ctx.registry.model_3d.models.len() <= id as usize {
            ctx.registry.model_3d.models.push(None);
        }
        ctx.registry.model_3d.models[id as usize] = Some(crate::model::MeshDataPersistent {
            vertices: vertices.to_vec(),
            indices: indices.to_vec(),
        });

        let mesh = MeshData::new(&self.device, vertices, indices);
        mesh.upload(&self.queue, vertices, indices);
        self.ensure_model_3d()
            .model_renderer
            .meshes
            .insert(id, mesh);

        Ok(id)
    }

    pub fn create_skin(
        &mut self,
        ctx: &mut crate::Context,
        bones: Vec<Bone>,
        bone_matrices: Vec<[[f32; 4]; 4]>,
    ) -> u32 {
        let id = ctx.registry.model_3d.next_skin_id;
        ctx.registry.model_3d.next_skin_id += 1;

        while ctx.registry.model_3d.skins.len() <= id as usize {
            ctx.registry.model_3d.skins.push(None);
        }
        while self.ensure_model_3d().gpu_skins.len() <= id as usize {
            self.ensure_model_3d().gpu_skins.push(None);
        }
        let skin = SkinData {
            bones,
            bone_matrices,
        };
        ctx.registry.model_3d.skins[id as usize] = Some(skin.clone());
        self.ensure_model_3d().gpu_skins[id as usize] = Some(skin.clone());
        self.ensure_model_3d().model_renderer.skins.insert(id, skin);
        id
    }

    pub fn update_bone_matrices(
        &mut self,
        ctx: &mut crate::Context,
        skin_id: u32,
        matrices: &[[[f32; 4]; 4]],
    ) {
        if let Some(Some(skin)) = ctx.registry.model_3d.skins.get_mut(skin_id as usize) {
            for (i, matrix) in matrices.iter().enumerate() {
                if i < skin.bone_matrices.len() {
                    skin.bone_matrices[i] = *matrix;
                }
            }
        }
        if let Some(Some(skin)) = self.ensure_model_3d().gpu_skins.get_mut(skin_id as usize) {
            for (i, matrix) in matrices.iter().enumerate() {
                if i < skin.bone_matrices.len() {
                    skin.bone_matrices[i] = *matrix;
                }
            }
        }
        if let Some(skin) = self
            .ensure_model_3d()
            .model_renderer
            .skins
            .get_mut(&skin_id)
        {
            for (i, matrix) in matrices.iter().enumerate() {
                if i < skin.bone_matrices.len() {
                    skin.bone_matrices[i] = *matrix;
                }
            }
        }
    }
}
