//! Image creation, manipulation, and atlas management.

use crate::image::{Bounds, Image, ImageEntry};
use crate::packer::AtlasPacker;
use crate::platform;
use crate::pt::Pt;
use crate::texture::Texture;
use std::sync::Arc;

use super::Graphics;
use super::core::AtlasSlot;

impl Graphics {
    pub(super) fn ensure_atlas_for_image(
        &mut self,
        w: u32,
        h: u32,
    ) -> anyhow::Result<(u32, crate::packer::PackerRect)> {
        if let Some(res) = self.try_ensure_atlas_for_image(w, h) {
            return Ok(res);
        }

        // If allocation fails and we have dirty assets, rebuild and try again
        if self.dirty_assets {
            self.rebuild_atlases()?;
            if let Some(res) = self.try_ensure_atlas_for_image(w, h) {
                return Ok(res);
            }
        }

        self.create_new_atlas(w, h)
    }

    fn try_ensure_atlas_for_image(
        &mut self,
        w: u32,
        h: u32,
    ) -> Option<(u32, crate::packer::PackerRect)> {
        let last_idx = self.atlases.len().checked_sub(1)?;
        let last = &mut self.atlases[last_idx];

        if let Some(rect) = last.packer.insert_raw(w, h) {
            return Some((last_idx as u32, rect));
        }
        None
    }

    fn create_new_atlas(
        &mut self,
        w: u32,
        h: u32,
    ) -> anyhow::Result<(u32, crate::packer::PackerRect)> {
        let atlas_size = 4096;
        let packer = AtlasPacker::new(atlas_size, atlas_size, 2);
        let texture =
            Texture::create_empty(&self.device, atlas_size, atlas_size, self.config.format);
        let bind_group = self
            .image_renderer
            .create_texture_bind_group(&self.device, &texture.0.view);
        self.atlases.push(AtlasSlot {
            packer,
            texture,
            bind_group,
        });

        let atlas_index = (self.atlases.len() - 1) as u32;
        let rect = self
            .atlases
            .last_mut()
            .expect("atlas")
            .packer
            .insert_raw(w, h)
            .ok_or_else(|| anyhow::anyhow!("image too large for atlas"))?;
        Ok((atlas_index, rect))
    }

    pub(crate) fn create_image(
        &mut self,
        width: Pt,
        height: Pt,
        rgba: &[u8],
    ) -> anyhow::Result<Image> {
        let bounds = Bounds::new(Pt(0.0), Pt(0.0), width, height);
        let entry = ImageEntry::new(None, bounds, None, Some(Arc::from(rgba)), None);
        let image = self.insert_image_entry(entry);
        self.dirty_assets = true; // Mark as dirty to ensure compress_assets picks it up
        Ok(image)
    }

    pub(crate) fn create_sub_image(
        &mut self,
        image: Image,
        bounds: Bounds,
    ) -> anyhow::Result<Image> {
        let parent_entry = self
            .images
            .get(image.index())
            .and_then(|v| v.as_ref())
            .ok_or_else(|| anyhow::anyhow!("Invalid parent image"))?;

        let (atlas_index, uv_rect) =
            if let (Some(p_atlas), Some(p_uv)) = (parent_entry.atlas_index, parent_entry.uv_rect) {
                let p_u0 = p_uv[0];
                let p_v0 = p_uv[1];
                let p_w = p_uv[2];
                let p_h = p_uv[3];

                let parent_w = parent_entry.bounds.width.as_f32();
                let parent_h = parent_entry.bounds.height.as_f32();

                let nx = bounds.x.as_f32() / parent_w;
                let ny = bounds.y.as_f32() / parent_h;
                let nw = bounds.width.as_f32() / parent_w;
                let nh = bounds.height.as_f32() / parent_h;

                let g_u0 = p_u0 + nx * p_w;
                let g_v0 = p_v0 + ny * p_h;
                let g_w = nw * p_w;
                let g_h = nh * p_h;

                (Some(p_atlas), Some([g_u0, g_v0, g_w, g_h]))
            } else {
                (None, None)
            };

        let entry = ImageEntry::new(atlas_index, bounds, uv_rect, None, Some(image.id()));
        Ok(self.insert_image_entry(entry))
    }

    pub(crate) fn insert_image_entry(&mut self, entry: ImageEntry) -> Image {
        let id = self.images.len() as u32;
        let bounds = entry.bounds;
        self.images.push(Some(entry));
        Image {
            id,
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: bounds.height,
        }
    }

    pub(crate) fn take_image_entry(&mut self, image: Image) -> Option<ImageEntry> {
        let entry = self.images.get_mut(image.index())?.take();
        if entry.is_some() {
            self.dirty_assets = true;
        }
        entry
    }

    pub(crate) fn process_registrations(&mut self) -> anyhow::Result<()> {
        if !self.dirty_assets {
            return Ok(());
        }

        // Phase 1: Pack and upload all Pending root images incrementally
        for i in 0..self.images.len() {
            if let Some(entry) = self.images[i].as_ref() {
                if !entry.is_ready() && entry.raw_data.is_some() {
                    // This is a pending root image. Try to fit it into existing atlases.
                    let entry = self.images[i].take().unwrap();
                    let raw_data = entry.raw_data.clone().unwrap();
                    let w = entry.bounds.width.to_u32_clamped();
                    let h = entry.bounds.height.to_u32_clamped();

                    // Try to find space in existing atlases or create a new one.
                    // This is still fairly cheap if atlas has space.
                    let (atlas_index, rect) = self.ensure_atlas_for_image(w, h)?;
                    let atlas = self.atlases.get_mut(atlas_index as usize).expect("atlas");

                    let mut extruded_data = atlas.packer.extrude_rgba8(&raw_data, w, h);
                    match atlas.texture.0.format {
                        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
                            for p in extruded_data.chunks_exact_mut(4) {
                                p.swap(0, 2);
                            }
                        }
                        _ => {}
                    }

                    let (tx, ty, tw, th) = atlas.packer.get_write_info(&rect);
                    let bytes_per_row = 4 * tw;
                    let (data, bytes_per_row) =
                        platform::align_write_texture_bytes(bytes_per_row, th, extruded_data);

                    self.queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &atlas.texture.0.texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d { x: tx, y: ty, z: 0 },
                            aspect: wgpu::TextureAspect::All,
                        },
                        &data,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(bytes_per_row),
                            rows_per_image: Some(th),
                        },
                        wgpu::Extent3d {
                            width: tw,
                            height: th,
                            depth_or_array_layers: 1,
                        },
                    );

                    atlas.texture.generate_mipmaps(&self.device, &self.queue);

                    let uv_param = atlas.packer.get_uv_param(&rect);
                    let uv_rect = [uv_param[0], uv_param[1], uv_param[2], uv_param[3]];
                    let new_entry = ImageEntry::new(
                        Some(atlas_index),
                        entry.bounds,
                        Some(uv_rect),
                        entry.raw_data,
                        None,
                    );
                    self.images[i] = Some(new_entry);
                }
            }
        }

        // Phase 2: Resolve sub-images that depend on either old or newly readied roots
        for i in 0..self.images.len() {
            if let Some(mut entry) = self.images[i].take() {
                if !entry.is_ready() && entry.parent_id.is_some() {
                    let parent_id = entry.parent_id.unwrap();
                    if let Some(parent_entry) =
                        self.images.get(parent_id as usize).and_then(|v| v.as_ref())
                    {
                        if parent_entry.is_ready() {
                            let p_atlas = parent_entry.atlas_index.unwrap();
                            let p_uv = parent_entry.uv_rect.unwrap();
                            let p_u0 = p_uv[0];
                            let p_v0 = p_uv[1];
                            let p_w = p_uv[2];
                            let p_h = p_uv[3];

                            let parent_w = parent_entry.bounds.width.as_f32();
                            let parent_h = parent_entry.bounds.height.as_f32();

                            let nx = entry.bounds.x.as_f32() / parent_w;
                            let ny = entry.bounds.y.as_f32() / parent_h;
                            let nw = entry.bounds.width.as_f32() / parent_w;
                            let nh = entry.bounds.height.as_f32() / parent_h;

                            let g_u0 = p_u0 + nx * p_w;
                            let g_v0 = p_v0 + ny * p_h;
                            let g_w = nw * p_w;
                            let g_h = nh * p_h;

                            entry.uv_rect = Some([g_u0, g_v0, g_w, g_h]);
                            entry.atlas_index = Some(p_atlas);
                        }
                    }
                }
                self.images[i] = Some(entry);
            }
        }

        self.dirty_assets = false;
        Ok(())
    }

    pub fn compress_assets(&mut self) -> anyhow::Result<()> {
        self.rebuild_atlases()
    }

    pub(crate) fn rebuild_atlases(&mut self) -> anyhow::Result<()> {
        self.dirty_assets = false;
        // Drop old atlases
        self.atlases.clear();
        self.glyph_cache.clear();

        // Create initial atlas
        let atlas_size = 4096;
        let packer = AtlasPacker::new(atlas_size, atlas_size, 2);
        let texture =
            Texture::create_empty(&self.device, atlas_size, atlas_size, self.config.format);
        let bind_group = self
            .image_renderer
            .create_texture_bind_group(&self.device, &texture.0.view);
        self.atlases.push(AtlasSlot {
            packer,
            texture,
            bind_group,
        });

        // Re-pack all root images
        for i in 0..self.images.len() {
            if let Some(entry) = self.images[i].as_ref() {
                if entry.raw_data.is_some() {
                    let entry = self.images[i].take().unwrap();
                    let raw_data = entry.raw_data.clone().unwrap();
                    let width = entry.bounds.width;
                    let height = entry.bounds.height;

                    let w = width.to_u32_clamped();
                    let h = height.to_u32_clamped();

                    let (atlas_index, rect) = self.ensure_atlas_for_image(w, h)?;
                    let atlas = self.atlases.get_mut(atlas_index as usize).expect("atlas");

                    let mut extruded_data = atlas.packer.extrude_rgba8(&raw_data, w, h);

                    match atlas.texture.0.format {
                        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
                            for p in extruded_data.chunks_exact_mut(4) {
                                p.swap(0, 2);
                            }
                        }
                        _ => {}
                    }

                    let (tx, ty, tw, th) = atlas.packer.get_write_info(&rect);
                    let bytes_per_row = 4 * tw;
                    let (data, bytes_per_row) =
                        platform::align_write_texture_bytes(bytes_per_row, th, extruded_data);

                    self.queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &atlas.texture.0.texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d { x: tx, y: ty, z: 0 },
                            aspect: wgpu::TextureAspect::All,
                        },
                        &data,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(bytes_per_row),
                            rows_per_image: Some(th),
                        },
                        wgpu::Extent3d {
                            width: tw,
                            height: th,
                            depth_or_array_layers: 1,
                        },
                    );

                    atlas.texture.generate_mipmaps(&self.device, &self.queue);

                    let uv_param = atlas.packer.get_uv_param(&rect);
                    let uv_rect = [uv_param[0], uv_param[1], uv_param[2], uv_param[3]];
                    let new_entry = ImageEntry::new(
                        Some(atlas_index),
                        entry.bounds,
                        Some(uv_rect),
                        entry.raw_data,
                        None,
                    );
                    self.images[i] = Some(new_entry);
                }
            }
        }

        // Re-calculate sub-images
        for i in 0..self.images.len() {
            if let Some(mut entry) = self.images[i].take() {
                if let Some(parent_id) = entry.parent_id {
                    let parent_found = if let Some(parent_entry) =
                        self.images.get(parent_id as usize).and_then(|v| v.as_ref())
                    {
                        if let (Some(p_atlas), Some(p_uv)) =
                            (parent_entry.atlas_index, parent_entry.uv_rect)
                        {
                            let p_u0 = p_uv[0];
                            let p_v0 = p_uv[1];
                            let p_w = p_uv[2];
                            let p_h = p_uv[3];

                            let parent_w = parent_entry.bounds.width.as_f32();
                            let parent_h = parent_entry.bounds.height.as_f32();

                            let nx = entry.bounds.x.as_f32() / parent_w;
                            let ny = entry.bounds.y.as_f32() / parent_h;
                            let nw = entry.bounds.width.as_f32() / parent_w;
                            let nh = entry.bounds.height.as_f32() / parent_h;

                            let g_u0 = p_u0 + nx * p_w;
                            let g_v0 = p_v0 + ny * p_h;
                            let g_w = nw * p_w;
                            let g_h = nh * p_h;

                            entry.uv_rect = Some([g_u0, g_v0, g_w, g_h]);
                            entry.atlas_index = Some(p_atlas);
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if !parent_found {
                        // Parent is gone or still pending
                        // If it's still pending, we keep this sub-image as pending too
                        if self
                            .images
                            .get(parent_id as usize)
                            .and_then(|v| v.as_ref())
                            .is_some()
                        {
                            entry.uv_rect = None;
                            entry.atlas_index = None;
                            self.images[i] = Some(entry);
                        } else {
                            // Parent is actually gone
                            continue;
                        }
                        continue;
                    }
                }
                self.images[i] = Some(entry);
            }
        }

        Ok(())
    }

    pub(crate) fn image_bounds(&self, image: Image) -> anyhow::Result<Bounds> {
        self.images
            .get(image.index())
            .and_then(|v| v.as_ref())
            .map(|e| e.bounds)
            .ok_or_else(|| anyhow::anyhow!("invalid image"))
    }

    pub(crate) fn copy_image(&mut self, dst: Image, src: Image) -> anyhow::Result<()> {
        let dst_entry = self
            .images
            .get(dst.index())
            .and_then(|v| v.as_ref())
            .ok_or_else(|| anyhow::anyhow!("invalid dst image"))?;
        let src_entry = self
            .images
            .get(src.index())
            .and_then(|v| v.as_ref())
            .ok_or_else(|| anyhow::anyhow!("invalid src image"))?;

        if !dst_entry.is_ready() || !src_entry.is_ready() {
            return Err(anyhow::anyhow!("image not ready for copy"));
        }

        let dst_atlas_index = dst_entry.atlas_index.unwrap();
        let src_atlas_index = src_entry.atlas_index.unwrap();
        let dst_uv_rect = dst_entry.uv_rect.unwrap();
        let src_uv_rect = src_entry.uv_rect.unwrap();

        if dst_atlas_index != src_atlas_index {
            return Err(anyhow::anyhow!(
                "copy_image across atlases is not supported"
            ));
        }
        if dst_entry.bounds.width != src_entry.bounds.width
            || dst_entry.bounds.height != src_entry.bounds.height
        {
            return Err(anyhow::anyhow!("size mismatch"));
        }

        let atlas = self.atlases.get(dst_atlas_index as usize).expect("atlas");
        let aw = atlas.packer.width() as f32;
        let ah = atlas.packer.height() as f32;

        let src_x = (src_uv_rect[0] * aw).round() as u32;
        let src_y = (src_uv_rect[1] * ah).round() as u32;
        let dst_x = (dst_uv_rect[0] * aw).round() as u32;
        let dst_y = (dst_uv_rect[1] * ah).round() as u32;
        let w = dst_entry.bounds.width.to_u32_clamped();
        let h = dst_entry.bounds.height.to_u32_clamped();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("graphics_copy_image_encoder"),
            });
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas.texture.0.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: src_x,
                    y: src_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &atlas.texture.0.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: dst_x,
                    y: dst_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }

    pub(crate) fn clear_image(&mut self, target: Image, color: [f32; 4]) -> anyhow::Result<()> {
        let bounds = self.image_bounds(target)?;
        let w = bounds.width.to_u32_clamped();
        let h = bounds.height.to_u32_clamped();
        let pixel = [
            (color[0] * 255.0) as u8,
            (color[1] * 255.0) as u8,
            (color[2] * 255.0) as u8,
            (color[3] * 255.0) as u8,
        ];
        let data: Vec<u8> = pixel.repeat((w * h) as usize);
        let entry = self
            .images
            .get(target.index())
            .unwrap()
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("invalid target image"))?;

        if !entry.is_ready() {
            return Err(anyhow::anyhow!("image not ready for clear"));
        }

        let atlas_index = entry.atlas_index.unwrap();
        let uv_rect = entry.uv_rect.unwrap();

        let atlas = self.atlases.get(atlas_index as usize).expect("atlas");
        let aw = atlas.packer.width() as f32;
        let ah = atlas.packer.height() as f32;
        let x = (uv_rect[0] * aw).round() as u32;
        let y = (uv_rect[1] * ah).round() as u32;
        let bytes_per_row = 4 * w;
        let (data, bytes_per_row) = platform::align_write_texture_bytes(bytes_per_row, h, data);

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas.texture.0.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        Ok(())
    }
}
