use crate::graphics::texture::GpuTexture;
use crate::platform;

use super::core::Graphics;

impl Graphics {
    pub(crate) fn process_registrations(&mut self, ctx: &mut crate::Context) -> anyhow::Result<()> {
        let has_pending = ctx.registry.textures.iter().any(|opt| {
            opt.as_ref()
                .map(|e| !e.is_ready(self.gpu_generation))
                .unwrap_or(false)
        });
        if !self.dirty_assets && !has_pending {
            return Ok(());
        }

        for i in 0..ctx.registry.textures.len() {
            let Some(entry) = ctx.registry.textures[i].as_mut() else {
                continue;
            };
            if entry.is_ready(self.gpu_generation) {
                continue;
            }

            let usage = if entry.is_render_target() {
                wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::RENDER_ATTACHMENT
            } else {
                wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::RENDER_ATTACHMENT
            };
            let format = entry.gpu_format(self.config.format);

            let texture = if entry.is_render_target() {
                GpuTexture::create_empty_with_usage_and_mips(
                    &self.device,
                    entry.pixel_width,
                    entry.pixel_height,
                    format,
                    usage,
                    1,
                )
            } else if entry.dynamic_atlas {
                GpuTexture::create_empty_with_usage_and_mips(
                    &self.device,
                    entry.pixel_width,
                    entry.pixel_height,
                    format,
                    usage,
                    1,
                )
            } else {
                GpuTexture::create_empty_with_usage(
                    &self.device,
                    entry.pixel_width,
                    entry.pixel_height,
                    format,
                    usage,
                )
            };

            if let Some(raw_data) = entry.raw_data.as_ref() {
                let mut upload_data = raw_data.to_vec();
                if matches!(
                    texture.0.format,
                    wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
                ) {
                    for p in upload_data.chunks_exact_mut(4) {
                        p.swap(0, 2);
                    }
                }

                let bytes_per_row = 4 * entry.pixel_width;
                let (data, bytes_per_row) = platform::align_write_texture_bytes(
                    bytes_per_row,
                    entry.pixel_height,
                    upload_data,
                );

                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture.0.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &data,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row),
                        rows_per_image: Some(entry.pixel_height),
                    },
                    wgpu::Extent3d {
                        width: entry.pixel_width,
                        height: entry.pixel_height,
                        depth_or_array_layers: 1,
                    },
                );

                if !entry.dynamic_atlas {
                    texture.generate_mipmaps(&self.device, &self.queue);
                }
            }

            let bind_group = self
                .image_renderer
                .create_texture_bind_group(&self.device, &texture.0.view);
            entry.runtime.gpu_texture = Some(texture);
            entry.runtime.bind_group = Some(bind_group);
            entry.runtime.generation = self.gpu_generation;
        }

        self.dirty_assets = false;
        ctx.registry.dirty_assets = false;
        Ok(())
    }

    pub(crate) fn rebuild_textures(&mut self, ctx: &mut crate::Context) -> anyhow::Result<()> {
        self.dirty_assets = false;
        self.image_renderer.clear_extra_texture_bind_group_cache();
        #[cfg(feature = "model-3d")]
        if let Some(model_3d) = self.model_3d_mut() {
            model_3d.model_renderer.clear_texture_bind_group_cache();
        }

        for entry in ctx.registry.textures.iter_mut().flatten() {
            entry.runtime.generation = 0;
            entry.runtime.gpu_texture = None;
            entry.runtime.bind_group = None;
        }

        self.process_registrations(ctx)
    }
}

pub(crate) fn resolve_image_uv(
    image_entry: &crate::image::ImageEntry,
    texture_entry: &crate::graphics::texture::TextureEntry,
) -> [f32; 4] {
    let full_w = texture_entry.pixel_width as f32;
    let full_h = texture_entry.pixel_height as f32;

    [
        image_entry.pixel_bounds.x as f32 / full_w,
        image_entry.pixel_bounds.y as f32 / full_h,
        image_entry.pixel_bounds.width as f32 / full_w,
        image_entry.pixel_bounds.height as f32 / full_h,
    ]
}
