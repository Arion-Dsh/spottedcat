//! Core Graphics structure and initialization.

use crate::DrawOption;
use crate::ShaderOpts;
use crate::drawable::DrawCommand;
#[cfg(feature = "model-3d")]
use crate::drawable::DrawCommand3D;
use crate::glyph_cache::GlyphCache;
#[cfg(feature = "model-3d")]
use crate::graphics::model_raw::{MeshData, ModelRenderer};
#[cfg(feature = "model-3d")]
use crate::image::ImageEntry;
use crate::image_raw::{ImageRenderer, InstanceData};
#[cfg(feature = "model-3d")]
use crate::model::Vertex;
use crate::packer::AtlasPacker;
use crate::texture::Texture;
use ab_glyph::FontArc;
use std::collections::HashMap;

pub(crate) struct AtlasSlot {
    pub packer: AtlasPacker,
    pub texture: Texture,
    pub bind_group: wgpu::BindGroup,
}

#[cfg(feature = "model-3d")]
#[derive(Debug, Clone)]
pub struct SkinData {
    pub bones: Vec<Bone>,
    pub bone_matrices: Vec<[[f32; 4]; 4]>,
}

#[cfg(feature = "model-3d")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bone {
    pub parent_index: Option<usize>, // Index into 'bones' Vec
    pub inverse_bind_matrix: [[f32; 4]; 4],
}

#[cfg(feature = "model-3d")]
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

#[cfg(feature = "model-3d")]
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

#[cfg(feature = "model-3d")]
impl Camera {
    pub fn view_matrix(&self) -> [[f32; 4]; 4] {
        let f = {
            let mut f = [
                self.target[0] - self.eye[0],
                self.target[1] - self.eye[1],
                self.target[2] - self.eye[2],
            ];
            let len = (f[0] * f[0] + f[1] * f[1] + f[2] * f[2]).sqrt();
            f[0] /= len;
            f[1] /= len;
            f[2] /= len;
            f
        };

        let s = {
            let mut s = [
                self.up[1] * f[2] - self.up[2] * f[1],
                self.up[2] * f[0] - self.up[0] * f[2],
                self.up[0] * f[1] - self.up[1] * f[0],
            ];
            let len = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
            s[0] /= len;
            s[1] /= len;
            s[2] /= len;
            s
        };

        let u = [
            f[1] * s[2] - f[2] * s[1],
            f[2] * s[0] - f[0] * s[2],
            f[0] * s[1] - f[1] * s[0],
        ];

        [
            [s[0], u[0], -f[0], 0.0],
            [s[1], u[1], -f[1], 0.0],
            [s[2], u[2], -f[2], 0.0],
            [
                -(s[0] * self.eye[0] + s[1] * self.eye[1] + s[2] * self.eye[2]),
                -(u[0] * self.eye[0] + u[1] * self.eye[1] + u[2] * self.eye[2]),
                f[0] * self.eye[0] + f[1] * self.eye[1] + f[2] * self.eye[2],
                1.0,
            ],
        ]
    }

    pub fn projection_matrix(&self) -> [[f32; 4]; 4] {
        crate::graphics::model_raw::create_perspective(
            self.aspect,
            self.fovy,
            self.znear,
            self.zfar,
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ResolvedDraw {
    pub atlas_index: u32,
    pub bounds: crate::image::Bounds,
    pub uv_rect: [f32; 4],
    pub opts: DrawOption,
    pub shader_id: u32,
    pub shader_opts: ShaderOpts,
    pub layer: i32,
}

#[cfg(feature = "model-3d")]
type MaterialTextureBinding<'a> = (u32, [f32; 4], &'a wgpu::TextureView);
#[cfg(feature = "model-3d")]
type MaterialTextureSet<'a> = (
    MaterialTextureBinding<'a>,
    MaterialTextureBinding<'a>,
    MaterialTextureBinding<'a>,
    MaterialTextureBinding<'a>,
    MaterialTextureBinding<'a>,
);

#[cfg(feature = "model-3d")]
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

#[cfg(feature = "model-3d")]
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

#[cfg(feature = "model-3d")]
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

#[cfg(feature = "model-3d")]
pub(crate) struct Graphics3D {
    pub(crate) model_pipelines: HashMap<u32, wgpu::RenderPipeline>,
    pub(crate) instanced_model_pipelines: HashMap<u32, wgpu::RenderPipeline>,
    pub(crate) opaque_draw_indices_3d: Vec<usize>,
    pub(crate) transparent_draw_indices_3d: Vec<usize>,
    pub(crate) model_renderer: ModelRenderer,
    pub(crate) model_pipeline: wgpu::RenderPipeline,
    pub(crate) instanced_model_pipeline: wgpu::RenderPipeline,
    pub(crate) fog_background_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) fog_background_bind_group: wgpu::BindGroup,
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

pub struct Graphics {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) adapter: wgpu::Adapter,
    pub(crate) config: wgpu::SurfaceConfiguration,
    pub(crate) image_renderer: ImageRenderer,
    pub(crate) default_pipeline: wgpu::RenderPipeline,
    pub(crate) image_pipelines: HashMap<u32, wgpu::RenderPipeline>,
    pub(crate) atlases: Vec<AtlasSlot>,
    pub(crate) batch: Vec<InstanceData>,
    pub(crate) font_cache: HashMap<u64, FontArc>,
    pub(crate) glyph_cache: GlyphCache,
    pub(crate) resolved_draws: Vec<ResolvedDraw>,
    pub(crate) text_shader_id: u32,
    pub(crate) dirty_assets: bool,
    pub(crate) pipelines_dirty: bool,
    pub(crate) gpu_generation: u32,
    #[cfg(feature = "model-3d")]
    pub(crate) model_3d: Option<Graphics3D>,
    pub(crate) transparent: bool,
}

impl std::fmt::Debug for Graphics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Graphics")
            .field("device", &self.device)
            .field("transparent", &self.transparent)
            .finish_non_exhaustive()
    }
}

impl Graphics {
    #[cfg(feature = "model-3d")]
    pub(crate) fn model_3d(&self) -> Option<&Graphics3D> {
        self.model_3d.as_ref()
    }

    #[cfg(feature = "model-3d")]
    pub(crate) fn model_3d_mut(&mut self) -> Option<&mut Graphics3D> {
        self.model_3d.as_mut()
    }

    #[cfg(feature = "model-3d")]
    pub(crate) fn ensure_model_3d(&mut self) -> &mut Graphics3D {
        if self.model_3d.is_none() {
            let width = self.config.width.max(1);
            let height = self.config.height.max(1);
            self.model_3d = Some(Self::build_model_3d(
                &self.device,
                &self.config,
                width,
                height,
            ));
        }
        self.model_3d
            .as_mut()
            .expect("Graphics3D must exist after ensure_model_3d")
    }

    #[cfg(feature = "model-3d")]
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

    #[cfg(feature = "model-3d")]
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

    #[cfg(feature = "model-3d")]
    pub(crate) fn create_fog_background_pipeline(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fog_background_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/fog_background.wgsl").into()),
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

    #[cfg(feature = "model-3d")]
    fn build_model_3d(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        width: u32,
        height: u32,
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

        let fog_background_bind_group_layout =
            Self::create_fog_background_bind_group_layout(device);
        let fog_background_bind_group = Self::create_fog_background_bind_group(
            device,
            &fog_background_bind_group_layout,
            &model_renderer.scene_globals_buffer,
            &depth_view,
        );
        let fog_background_pipeline = Self::create_fog_background_pipeline(
            device,
            config.format,
            &fog_background_bind_group_layout,
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
            fog_background_bind_group_layout,
            fog_background_bind_group,
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
                light_view_proj: identity(),
            },
        }
    }

    pub async fn new(
        instance: &wgpu::Instance,
        surface: &wgpu::Surface<'_>,
        width: u32,
        height: u32,
        transparent: bool,
    ) -> anyhow::Result<Self> {
        let width = width.max(1);
        let height = height.max(1);
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            })
            .await?;

        let info = adapter.get_info();
        eprintln!(
            "[spot][init] Selected adapter: {:?} ({:?})",
            info.name, info.backend
        );

        let adapter_limits = adapter.limits();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: adapter_limits,
                experimental_features: wgpu::ExperimentalFeatures::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let adapter_clone = adapter.clone();

        let caps = surface.get_capabilities(&adapter);
        let mut config = surface
            .get_default_config(&adapter, width, height)
            .unwrap_or_else(|| wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: pick_surface_format(&caps),
                width: width.max(1),
                height: height.max(1),
                present_mode: caps.present_modes[0],
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 1,
            });

        config.alpha_mode = pick_alpha_mode(&caps, transparent);

        config.present_mode = crate::graphics::profile::pick_present_mode(&caps);
        config.usage = crate::platform::surface_usage(&caps);

        surface.configure(&device, &config);

        let image_renderer = ImageRenderer::new(&device, config.format, 200000);

        let image_pipelines = HashMap::new();

        let atlas_size = 2048;
        let packer = AtlasPacker::new(atlas_size, atlas_size, 2);
        let atlas_texture = Texture::create_empty(&device, atlas_size, atlas_size, config.format);
        let atlas_bind_group =
            image_renderer.create_texture_bind_group(&device, &atlas_texture.0.view);
        let atlases = vec![AtlasSlot {
            packer,
            texture: atlas_texture,
            bind_group: atlas_bind_group,
        }];

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("image_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/image.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("image_pipeline_layout"),
            bind_group_layouts: &[
                &image_renderer.texture_bind_group_layout,
                &image_renderer.user_globals_bind_group_layout,
                &image_renderer.engine_globals_bind_group_layout,
            ],
            immediate_size: 0,
        });

        let default_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("image_pipeline"),
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
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        #[cfg(feature = "model-3d")]
        let model_3d = None;

        let graphics = Self {
            device,
            queue,
            adapter: adapter_clone,
            config,
            image_renderer,
            default_pipeline,
            image_pipelines,
            atlases,
            batch: Vec::with_capacity(10000),
            font_cache: HashMap::new(),
            glyph_cache: GlyphCache::new(),
            resolved_draws: Vec::with_capacity(10000),
            text_shader_id: 0,
            dirty_assets: true,
            pipelines_dirty: false,
            gpu_generation: 0, // This will be set by the platform/app
            #[cfg(feature = "model-3d")]
            model_3d,
            transparent,
        };

        // Default resources will be registered via the Context in App initialization
        Ok(graphics)
    }

    fn sync_new_runtime_assets(&mut self, ctx: &mut crate::Context) -> anyhow::Result<()> {
        for (&id, source) in &ctx.registry.image_shaders {
            if id != 0 && !self.image_pipelines.contains_key(&id) {
                self.restore_image_shader(id, source);
            }
        }
        #[cfg(feature = "model-3d")]
        {
            let missing_model_shader_ids = if let Some(model_3d) = self.model_3d() {
                ctx.registry
                    .model_shaders
                    .keys()
                    .copied()
                    .filter(|&id| id != 0 && !model_3d.model_pipelines.contains_key(&id))
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            for id in missing_model_shader_ids {
                if let Some(source) = ctx.registry.model_shaders.get(&id) {
                    self.restore_model_shader(id, source);
                }
            }
        }

        for (&id, data) in &ctx.registry.fonts {
            if let std::collections::hash_map::Entry::Vacant(entry) =
                self.font_cache.entry(id as u64)
            {
                if let Ok(font) = FontArc::try_from_vec(data.clone()) {
                    entry.insert(font);
                } else {
                    eprintln!(
                        "[spot][graphics] Warning: Failed to sync font with ID {}",
                        id
                    );
                }
            }
        }

        #[cfg(feature = "model-3d")]
        if self.model_3d.is_some() {
            if self.model_3d().expect("checked Some").gpu_models.len() < ctx.registry.models.len() {
                self.model_3d_mut()
                    .expect("checked Some")
                    .gpu_models
                    .resize_with(ctx.registry.models.len(), || None);
            }
            for (idx, model_opt) in ctx.registry.models.iter().enumerate() {
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

            if self.model_3d().expect("checked Some").gpu_skins.len() < ctx.registry.skins.len() {
                self.model_3d_mut()
                    .expect("checked Some")
                    .gpu_skins
                    .resize_with(ctx.registry.skins.len(), || None);
            }
            for (idx, skin_opt) in ctx.registry.skins.iter().enumerate() {
                if self.model_3d().expect("checked Some").gpu_skins[idx].is_some()
                    || skin_opt.is_none()
                {
                    continue;
                }
                self.model_3d_mut().expect("checked Some").gpu_skins[idx] = skin_opt.clone();
            }
        }

        self.process_registrations(ctx)?;
        ctx.registry.dirty_assets = false;
        Ok(())
    }

    pub(crate) fn prepare_frame_resources(
        &mut self,
        ctx: &mut crate::Context,
        drawables: &[DrawCommand],
    ) -> anyhow::Result<()> {
        for drawable in drawables {
            if let DrawCommand::Text(text, opts) = drawable {
                self.ensure_text_layout(ctx, text, opts.scale())?;
            }
        }

        if ctx.registry.dirty_assets {
            self.process_registrations(ctx)?;
        }

        #[cfg(feature = "model-3d")]
        if !ctx.runtime.draw_list_3d.is_empty() {
            self.ensure_model_3d();
            for command in &ctx.runtime.draw_list_3d {
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

    pub(crate) fn sync_assets(&mut self, ctx: &mut crate::Context) -> anyhow::Result<()> {
        if self.pipelines_dirty {
            self.rebuild_surface_format_dependent_pipelines(ctx);
        }

        if self.gpu_generation == ctx.registry.gpu_generation {
            if ctx.registry.dirty_assets {
                self.sync_new_runtime_assets(ctx)?;
            }
            return Ok(());
        }

        eprintln!(
            "[spot][graphics] GPU generation mismatch ({} vs {}). Restoring assets...",
            self.gpu_generation, ctx.registry.gpu_generation
        );

        // Reset transient caches
        self.font_cache.clear();
        self.glyph_cache.clear();

        // 1. Restore Shaders
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
        for (&id, source) in &ctx.registry.image_shaders {
            self.restore_image_shader(id, source);
        }
        #[cfg(feature = "model-3d")]
        if self.model_3d.is_some() {
            for (&id, source) in &ctx.registry.model_shaders {
                self.restore_model_shader(id, source);
            }
        }

        // 2. Restore Fonts
        for (&id, data) in &ctx.registry.fonts {
            if let Ok(font) = ab_glyph::FontArc::try_from_vec(data.clone()) {
                self.font_cache.insert(id as u64, font);
            } else {
                eprintln!(
                    "[spot][graphics] Warning: Failed to restore font with ID {}",
                    id
                );
            }
        }

        // 3. Restore Images (Atlases)
        self.dirty_assets = true; // Ensure rebuild does work
        self.rebuild_atlases(ctx)?;

        // 4. Restore Meshes
        #[cfg(feature = "model-3d")]
        if self.model_3d.is_some() {
            self.model_3d_mut()
                .expect("checked Some")
                .gpu_models
                .clear();
            for model_opt in &ctx.registry.models {
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
        }

        // 5. Restore Skins
        #[cfg(feature = "model-3d")]
        if self.model_3d.is_some() {
            self.model_3d_mut().expect("checked Some").gpu_skins.clear();
            for skin_opt in &ctx.registry.skins {
                self.model_3d_mut()
                    .expect("checked Some")
                    .gpu_skins
                    .push(skin_opt.clone());
            }
        }

        // 6. Clear 3D model bind group caches as they reference old atlas views
        #[cfg(feature = "model-3d")]
        if self.model_3d.is_some() {
            self.model_3d_mut()
                .expect("checked Some")
                .model_renderer
                .clear_texture_bind_group_cache();
        }

        self.gpu_generation = ctx.registry.gpu_generation;

        // Set default IDs (aligned with Context::register_defaults)
        // White: 1, Black: 2, Normal: 3
        #[cfg(feature = "model-3d")]
        if self.model_3d.is_some() {
            self.model_3d_mut().expect("checked Some").white_image_id = 1;
            self.model_3d_mut().expect("checked Some").black_image_id = 2;
            self.model_3d_mut().expect("checked Some").normal_image_id = 3;
        }
        self.text_shader_id = 1;

        eprintln!(
            "[spot][graphics] Asset restoration complete. generation={}",
            self.gpu_generation
        );

        Ok(())
    }

    pub fn resize(&mut self, surface: &wgpu::Surface<'_>, width: u32, height: u32) {
        if width == 0 || height == 0 {
            eprintln!(
                "[spot][graphics] Warning: Attempted resize with zero dimension: {}x{}",
                width, height
            );
            return;
        }

        let caps = surface.get_capabilities(&self.adapter);
        if caps.formats.is_empty() {
            eprintln!("[spot][graphics] Surface has no supported formats on resize!");
            return;
        }

        #[cfg(feature = "model-3d")]
        let old_width = self.config.width;
        #[cfg(feature = "model-3d")]
        let old_height = self.config.height;
        let old_format = self.config.format;

        self.config.width = width;
        self.config.height = height;

        // Try to keep the same format if possible to avoid pipeline incompatibility
        if !caps.formats.contains(&old_format) {
            let new_fmt = pick_surface_format(&caps);
            eprintln!(
                "[spot][graphics] Warning: Original surface format {:?} not supported by new surface. Switching to {:?}. Pipelines may become invalid!",
                old_format, new_fmt
            );
            self.config.format = new_fmt;
            self.pipelines_dirty = true;
        } else {
            self.config.format = old_format;
        }

        self.config.present_mode = crate::graphics::profile::pick_present_mode(&caps);
        self.config.usage = crate::platform::surface_usage(&caps);
        self.config.alpha_mode = pick_alpha_mode(&caps, self.transparent);
        surface.configure(&self.device, &self.config);

        // Only recreate depth texture if size changed.
        // IMPORTANT: On Android, we always recreate on resize to ensure stability with new surfaces.
        #[cfg(feature = "model-3d")]
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

    #[cfg(feature = "model-3d")]
    pub fn create_mesh(
        &mut self,
        ctx: &mut crate::Context,
        vertices: &[Vertex],
        indices: &[u32],
    ) -> anyhow::Result<u32> {
        let id = ctx.registry.next_mesh_id;
        ctx.registry.next_mesh_id += 1;

        while ctx.registry.models.len() <= id as usize {
            ctx.registry.models.push(None);
        }
        ctx.registry.models[id as usize] = Some(crate::model::MeshDataPersistent {
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

    #[cfg(feature = "model-3d")]
    pub fn create_skin(
        &mut self,
        ctx: &mut crate::Context,
        bones: Vec<Bone>,
        bone_matrices: Vec<[[f32; 4]; 4]>,
    ) -> u32 {
        let id = ctx.registry.next_skin_id;
        ctx.registry.next_skin_id += 1;

        while ctx.registry.skins.len() <= id as usize {
            ctx.registry.skins.push(None);
        }
        while self.ensure_model_3d().gpu_skins.len() <= id as usize {
            self.ensure_model_3d().gpu_skins.push(None);
        }
        let skin = SkinData {
            bones,
            bone_matrices,
        };
        ctx.registry.skins[id as usize] = Some(skin.clone());
        self.ensure_model_3d().gpu_skins[id as usize] = Some(skin.clone());
        self.ensure_model_3d().model_renderer.skins.insert(id, skin);
        id
    }

    #[cfg(feature = "model-3d")]
    pub fn update_bone_matrices(
        &mut self,
        ctx: &mut crate::Context,
        skin_id: u32,
        matrices: &[[[f32; 4]; 4]],
    ) {
        if let Some(Some(skin)) = ctx.registry.skins.get_mut(skin_id as usize) {
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

    pub fn set_transparent(&mut self, transparent: bool) {
        self.transparent = transparent;
    }

    pub fn poll_device(&self, force_wait: bool) {
        let _ = self.device.poll(if force_wait {
            wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_millis(1)),
            }
        } else {
            wgpu::PollType::Poll
        });
    }

    pub fn transparent(&self) -> bool {
        self.transparent
    }
}

// Basic math helpers
pub fn identity() -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

pub fn create_scale(s: [f32; 3]) -> [[f32; 4]; 4] {
    [
        [s[0], 0.0, 0.0, 0.0],
        [0.0, s[1], 0.0, 0.0],
        [0.0, 0.0, s[2], 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

pub fn create_rotation_from_quat(q: [f32; 4]) -> [[f32; 4]; 4] {
    let x = q[0];
    let y = q[1];
    let z = q[2];
    let w = q[3];
    let x2 = x + x;
    let y2 = y + y;
    let z2 = z + z;
    let xx = x * x2;
    let xy = x * y2;
    let xz = x * z2;
    let yy = y * y2;
    let yz = y * z2;
    let zz = z * z2;
    let wx = w * x2;
    let wy = w * y2;
    let wz = w * z2;

    [
        [1.0 - (yy + zz), xy + wz, xz - wy, 0.0],
        [xy - wz, 1.0 - (xx + zz), yz + wx, 0.0],
        [xz + wy, yz - wx, 1.0 - (xx + yy), 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn pick_surface_format(caps: &wgpu::SurfaceCapabilities) -> wgpu::TextureFormat {
    // Prefer Srgb formats with alpha
    let preferred = [
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8Unorm,
    ];

    for &fmt in &preferred {
        if caps.formats.contains(&fmt) {
            return fmt;
        }
    }
    caps.formats[0]
}

fn pick_alpha_mode(
    caps: &wgpu::SurfaceCapabilities,
    requested_transparent: bool,
) -> wgpu::CompositeAlphaMode {
    if !requested_transparent && caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
        return wgpu::CompositeAlphaMode::Opaque;
    }

    // If transparent is requested, try to find a transparent-capable mode.
    // Even if not requested, we might want to use a transparent-capable mode
    // to allow dynamic toggling later.
    let transparent_modes = [
        #[cfg(target_os = "android")]
        wgpu::CompositeAlphaMode::Inherit, // Android GLES usually needs Inherit for transparency
        wgpu::CompositeAlphaMode::PostMultiplied,
        wgpu::CompositeAlphaMode::PreMultiplied,
        #[cfg(not(target_os = "android"))]
        wgpu::CompositeAlphaMode::Inherit,
    ];

    for mode in transparent_modes {
        if caps.alpha_modes.contains(&mode) {
            return mode;
        }
    }

    caps.alpha_modes[0]
}
