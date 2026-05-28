use crate::graphics::texture::{GpuTexture, TextureUploadRegion};
use crate::platform;

use super::core::Graphics;

impl Graphics {
    pub(crate) fn process_registrations(&mut self, ctx: &mut crate::Context) -> anyhow::Result<()> {
        let has_pending = ctx.registry.textures.iter().any(|opt| {
            opt.as_ref()
                .map(|e| !e.is_ready(self.gpu_generation) || !e.pending_uploads.is_empty())
                .unwrap_or(false)
        });
        if !self.dirty_assets && !has_pending {
            return Ok(());
        }

        if ctx.registry.textures.iter().any(|opt| {
            opt.as_ref()
                .map(|e| e.dynamic_atlas && !e.is_ready(self.gpu_generation))
                .unwrap_or(false)
        }) {
            self.sync_dynamic_atlas_raw_data(ctx);
        }

        for i in 0..ctx.registry.textures.len() {
            let Some(entry) = ctx.registry.textures[i].as_mut() else {
                continue;
            };
            let needs_full_upload = !entry.is_ready(self.gpu_generation);
            if !needs_full_upload && entry.pending_uploads.is_empty() {
                continue;
            }

            if needs_full_upload {
                let usage = wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::RENDER_ATTACHMENT;
                let format = entry.gpu_format(self.config.format);

                let texture = if entry.is_render_target() || entry.dynamic_atlas {
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
                    upload_rgba_texture_region(
                        &self.queue,
                        &texture,
                        0,
                        0,
                        entry.pixel_width,
                        entry.pixel_height,
                        raw_data,
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
                entry.pending_uploads.clear();
            } else if let Some(texture) = entry.runtime.gpu_texture.as_ref() {
                let pending_uploads = std::mem::take(&mut entry.pending_uploads);
                for upload in pending_uploads {
                    upload_texture_region(&self.queue, texture, upload);
                }
            }
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

impl Graphics {
    fn sync_dynamic_atlas_raw_data(&self, ctx: &mut crate::Context) {
        if let Some(atlas) = self.font_atlas.as_ref() {
            atlas.sync_raw_data(&mut ctx.registry);
        }
        if let Some(atlas) = self.shared_atlas.as_ref() {
            atlas.sync_raw_data(&mut ctx.registry);
        }
    }
}

fn upload_texture_region(queue: &wgpu::Queue, texture: &GpuTexture, upload: TextureUploadRegion) {
    upload_rgba_texture_region(
        queue,
        texture,
        upload.x,
        upload.y,
        upload.width,
        upload.height,
        &upload.rgba,
    );
}

fn upload_rgba_texture_region(
    queue: &wgpu::Queue,
    texture: &GpuTexture,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    rgba: &[u8],
) {
    if width == 0 || height == 0 {
        return;
    }

    let mut upload_data = rgba.to_vec();
    if matches!(
        texture.0.format,
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    ) {
        for p in upload_data.chunks_exact_mut(4) {
            p.swap(0, 2);
        }
    }

    let bytes_per_row = 4 * width;
    let (data, bytes_per_row) =
        platform::align_write_texture_bytes(bytes_per_row, height, upload_data);

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture.0.texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x, y, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        &data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(bytes_per_row),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
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
