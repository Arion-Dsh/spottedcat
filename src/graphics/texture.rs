use crate::Pt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_GPU_TEXTURE_ID: AtomicU64 = AtomicU64::new(1);

/// Handle to a texture resource.
///
/// A texture owns the underlying pixel data or render target. To draw into it or sample from it,
/// create an [`Image`][crate::Image] via [`Texture::view`].
///
/// Use `Image::new(...)` for standard sampled images, and `Texture::new_render_target(...)`
/// to create a texture that can be used as a drawing target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Texture {
    pub(crate) id: u32,
    pub(crate) default_view_id: u32,
    pub(crate) width: Pt,
    pub(crate) height: Pt,
    pub(crate) pixel_width: u32,
    pub(crate) pixel_height: u32,
}

impl Texture {
    /// Creates a new texture from RGBA8 pixels.
    pub fn new(
        ctx: &mut crate::Context,
        width: Pt,
        height: Pt,
        rgba: &[u8],
    ) -> anyhow::Result<Self> {
        Self::new_from_rgba8(ctx, width, height, rgba)
    }

    /// Creates a render-target texture that can be drawn into and sampled from.
    pub fn new_render_target(ctx: &mut crate::Context, width: Pt, height: Pt) -> Self {
        ctx.register_render_target_texture(width, height)
    }

    /// Returns the default full-image view for this texture.
    pub fn view(self) -> crate::Image {
        crate::Image {
            id: self.default_view_id,
            texture_id: self.id,
            x: Pt(0.0),
            y: Pt(0.0),
            width: self.width,
            height: self.height,
            pixel_bounds: crate::image::PixelBounds {
                x: 0,
                y: 0,
                width: self.pixel_width,
                height: self.pixel_height,
            },
        }
    }

    /// Draws a drawable into this texture's full target view.
    ///
    /// The texture must be a render target created via [`Texture::new_render_target`].
    pub fn draw<D: crate::Drawable>(
        self,
        ctx: &mut crate::Context,
        drawable: D,
        options: D::Options,
    ) {
        self.view().draw(ctx, drawable, options);
    }

    /// Draws a source image into this texture's full target view using a custom image shader.
    ///
    /// The texture must be a render target created via [`Texture::new_render_target`].
    pub fn draw_with_shader<S: Into<crate::Image>>(
        self,
        ctx: &mut crate::Context,
        source: S,
        shader_id: u32,
        options: crate::DrawOption,
        shader_opts: crate::ShaderOpts,
    ) {
        self.view()
            .draw_with_shader(ctx, source, shader_id, options, shader_opts);
    }

    pub fn width(self) -> Pt {
        self.width
    }

    pub fn height(self) -> Pt {
        self.height
    }

    pub fn id(self) -> u32 {
        self.id
    }

    pub fn is_render_target(self, ctx: &crate::Context) -> bool {
        ctx.registry
            .textures
            .get(self.id as usize)
            .and_then(|v| v.as_ref())
            .map(TextureEntry::is_render_target)
            .unwrap_or(false)
    }

    pub fn is_ready(self, ctx: &crate::Context) -> bool {
        ctx.registry
            .textures
            .get(self.id as usize)
            .and_then(|v| v.as_ref())
            .map(|entry| entry.is_ready(ctx.registry.gpu_generation))
            .unwrap_or(false)
    }
    pub(crate) fn new_from_rgba8(
        ctx: &mut crate::Context,
        width: Pt,
        height: Pt,
        rgba: &[u8],
    ) -> anyhow::Result<Self> {
        let pixel_width = width.to_u32_clamped().max(1);
        let pixel_height = height.to_u32_clamped().max(1);
        let expected_len = (pixel_width as usize) * (pixel_height as usize) * 4;
        if rgba.len() != expected_len {
            anyhow::bail!(
                "RGBA data length mismatch: expected {} ({}x{}x4), got {}",
                expected_len,
                pixel_width,
                pixel_height,
                rgba.len()
            );
        }
        Ok(ctx.register_texture(pixel_width, pixel_height, width, height, rgba))
    }

    #[cfg(feature = "utils")]
    pub(crate) fn new_from_rgba8_with_pixels(
        ctx: &mut crate::Context,
        pixel_width: u32,
        pixel_height: u32,
        width: Pt,
        height: Pt,
        rgba: &[u8],
    ) -> anyhow::Result<Self> {
        let expected_len = (pixel_width as usize) * (pixel_height as usize) * 4;
        if rgba.len() != expected_len {
            anyhow::bail!(
                "RGBA data length mismatch: expected {} ({}x{}x4), got {}",
                expected_len,
                pixel_width,
                pixel_height,
                rgba.len()
            );
        }
        Ok(ctx.register_texture(pixel_width, pixel_height, width, height, rgba))
    }
}

impl From<Texture> for crate::Image {
    fn from(value: Texture) -> Self {
        value.view()
    }
}

impl From<&Texture> for crate::Image {
    fn from(value: &Texture) -> Self {
        value.view()
    }
}

impl From<&crate::Image> for crate::Image {
    fn from(value: &crate::Image) -> Self {
        *value
    }
}

impl crate::Drawable for &Texture {
    type Options = crate::DrawOption;

    fn draw_to(self, ctx: &mut crate::Context, target: crate::Image, options: Self::Options) {
        target.draw(ctx, &self.view(), options);
    }
}

#[derive(Clone)]
pub(crate) struct GpuTexture(pub(crate) Arc<AnyGpuTexture>);

pub(crate) struct AnyGpuTexture {
    pub format: wgpu::TextureFormat,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl Drop for AnyGpuTexture {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}

impl GpuTexture {
    pub fn create_empty_with_usage(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Self {
        let mip_level_count = Self::calculate_mip_levels(width, height);
        Self::create_empty_with_usage_and_mips(
            device,
            width,
            height,
            format,
            usage,
            mip_level_count,
        )
    }

    pub fn create_empty_with_usage_and_mips(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        mip_level_count: u32,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("spot_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("spot_texture_view"),
            format: Some(format),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(mip_level_count),
            base_array_layer: 0,
            array_layer_count: Some(1),
            usage: Some(
                usage
                    & (wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::STORAGE_BINDING),
            ),
        });
        let _gpu_texture_id = NEXT_GPU_TEXTURE_ID.fetch_add(1, Ordering::Relaxed);

        Self(Arc::new(AnyGpuTexture {
            format,
            texture,
            view,
        }))
    }

    fn calculate_mip_levels(width: u32, height: u32) -> u32 {
        let max_dim = width.max(height);
        (max_dim as f32).log2().floor() as u32 + 1
    }

    pub fn generate_mipmaps(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let texture = &self.0.texture;
        let mip_level_count = texture.mip_level_count();

        if mip_level_count <= 1 {
            return;
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("mipmap_generation"),
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mipmap_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/mipmap.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mipmap_pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.0.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mipmap_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        for mip_level in 1..mip_level_count {
            let src_mip = mip_level - 1;
            let dst_mip = mip_level;

            let src_view = texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some(&format!("mipmap_src_view_{}", src_mip)),
                format: Some(self.0.format),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: src_mip,
                mip_level_count: Some(1),
                base_array_layer: 0,
                array_layer_count: Some(1),
                usage: Some(wgpu::TextureUsages::TEXTURE_BINDING),
            });

            let dst_view = texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some(&format!("mipmap_dst_view_{}", dst_mip)),
                format: Some(self.0.format),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: dst_mip,
                mip_level_count: Some(1),
                base_array_layer: 0,
                array_layer_count: Some(1),
                usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("mipmap_bind_group_{}", mip_level)),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&src_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("mimap_pass_{}", mip_level)),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &dst_view,
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

            render_pass.set_pipeline(&pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        queue.submit(Some(encoder.finish()));
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TextureEntry {
    pub(crate) width: Pt,
    pub(crate) height: Pt,
    pub(crate) pixel_width: u32,
    pub(crate) pixel_height: u32,
    pub(crate) default_view_id: u32,
    pub(crate) render_target: bool,
    pub(crate) dynamic_atlas: bool,
    pub(crate) raw_data: Option<Arc<[u8]>>,
    pub(crate) runtime: TextureRuntimeData,
}

#[derive(Clone)]
pub(crate) struct TextureRuntimeData {
    pub(crate) gpu_texture: Option<GpuTexture>,
    pub(crate) bind_group: Option<wgpu::BindGroup>,
    pub(crate) generation: u32,
}

impl std::fmt::Debug for TextureRuntimeData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextureRuntimeData")
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl TextureEntry {
    pub(crate) fn new_sampled(
        width: Pt,
        height: Pt,
        pixel_width: u32,
        pixel_height: u32,
        default_view_id: u32,
        raw_data: Arc<[u8]>,
    ) -> Self {
        Self {
            width,
            height,
            pixel_width,
            pixel_height,
            default_view_id,
            render_target: false,
            dynamic_atlas: false,
            raw_data: Some(raw_data),
            runtime: TextureRuntimeData {
                gpu_texture: None,
                bind_group: None,
                generation: 0,
            },
        }
    }

    pub(crate) fn new_dynamic_atlas(
        width: Pt,
        height: Pt,
        pixel_width: u32,
        pixel_height: u32,
        default_view_id: u32,
        raw_data: Arc<[u8]>,
    ) -> Self {
        Self {
            width,
            height,
            pixel_width,
            pixel_height,
            default_view_id,
            render_target: false,
            dynamic_atlas: true,
            raw_data: Some(raw_data),
            runtime: TextureRuntimeData {
                gpu_texture: None,
                bind_group: None,
                generation: 0,
            },
        }
    }

    pub(crate) fn new_render_target(
        width: Pt,
        height: Pt,
        pixel_width: u32,
        pixel_height: u32,
        default_view_id: u32,
    ) -> Self {
        Self {
            width,
            height,
            pixel_width,
            pixel_height,
            default_view_id,
            render_target: true,
            dynamic_atlas: false,
            raw_data: None,
            runtime: TextureRuntimeData {
                gpu_texture: None,
                bind_group: None,
                generation: 0,
            },
        }
    }

    pub(crate) fn is_ready(&self, current_gen: u32) -> bool {
        self.runtime.gpu_texture.is_some() && self.runtime.generation == current_gen
    }

    pub(crate) fn is_render_target(&self) -> bool {
        self.render_target
    }
}
