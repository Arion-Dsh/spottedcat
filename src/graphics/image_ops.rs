//! Image creation, manipulation, and atlas management.

use crate::packer::AtlasPacker;
use crate::platform;
use crate::texture::Texture;

use super::Graphics;
use super::core::AtlasSlot;

impl Graphics {
    pub(super) fn ensure_atlas_for_image(
        &mut self,
        ctx: &mut crate::Context,
        w: u32,
        h: u32,
    ) -> anyhow::Result<(u32, crate::packer::PackerRect)> {
        if let Some(res) = self.try_ensure_atlas_for_image(w, h) {
            return Ok(res);
        }

        // If allocation fails and we have dirty assets, rebuild and try again
        if self.dirty_assets {
            self.rebuild_atlases(ctx)?;
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
        let atlas = self
            .atlases
            .last_mut()
            .unwrap_or_else(|| panic!("[spot][atlas] atlas storage unexpectedly empty"));
        let rect = atlas
            .packer
            .insert_raw(w, h)
            .ok_or_else(|| anyhow::anyhow!("image too large for atlas"))?;
        Ok((atlas_index, rect))
    }

    pub(crate) fn process_registrations(&mut self, ctx: &mut crate::Context) -> anyhow::Result<()> {
        let has_pending = ctx
            .registry
            .images
            .iter()
            .any(|opt| opt.as_ref().map(|e| !e.is_ready()).unwrap_or(false));
        if !self.dirty_assets && !has_pending {
            return Ok(());
        }

        // Phase 1: Pack and upload all Pending root images incrementally
        for i in 0..ctx.registry.images.len() {
            if let Some(entry) = ctx.registry.images[i].as_ref()
                && !entry.is_ready()
                && entry.raw_data.is_some()
            {
                let raw_data = entry.raw_data.clone().unwrap_or_else(|| {
                    panic!("[spot][atlas] image {} lost raw data before upload", i)
                });
                let w = entry.bounds.width.to_u32_clamped();
                let h = entry.bounds.height.to_u32_clamped();

                let (atlas_index, rect) = match self.ensure_atlas_for_image(ctx, w, h) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("[spot][atlas] Failed to pack image {}: {:?}", i, e);
                        continue;
                    }
                };
                let atlas = self
                    .atlases
                    .get_mut(atlas_index as usize)
                    .unwrap_or_else(|| {
                        panic!(
                            "[spot][atlas] atlas {} missing while uploading image {}",
                            atlas_index, i
                        )
                    });

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

                if let Some(entry) = ctx.registry.images[i].as_mut() {
                    entry.atlas_index = Some(atlas_index);
                    entry.uv_rect = Some(uv_rect);
                }
            }
        }

        // Phase 2: Resolve sub-images that depend on readied roots
        let mut changed = true;
        while changed {
            changed = false;
            for i in 0..ctx.registry.images.len() {
                let (atlas_index, uv_rect) = if let Some(entry) = ctx.registry.images[i].as_ref()
                    && !entry.is_ready()
                    && let Some(parent_id) = entry.parent_id
                {
                    if let Some(Some(parent)) = ctx.registry.images.get(parent_id as usize)
                        && parent.is_ready()
                    {
                        let parent_uv = parent.uv_rect.unwrap_or_else(|| {
                            panic!(
                                "[spot][atlas] ready parent image {} is missing uv_rect",
                                parent_id
                            )
                        });
                        let p_atlas_index = parent.atlas_index.unwrap_or_else(|| {
                            panic!(
                                "[spot][atlas] ready parent image {} is missing atlas index",
                                parent_id
                            )
                        });

                        let pw = parent.bounds.width.as_f32();
                        let ph = parent.bounds.height.as_f32();
                        let sw = entry.bounds.width.as_f32();
                        let sh = entry.bounds.height.as_f32();
                        let sx = (entry.bounds.x.as_f32() - parent.bounds.x.as_f32()).max(0.0);
                        let sy = (entry.bounds.y.as_f32() - parent.bounds.y.as_f32()).max(0.0);

                        let uv_x = parent_uv[0] + (sx / pw) * parent_uv[2];
                        let uv_y = parent_uv[1] + (sy / ph) * parent_uv[3];
                        let uv_w = (sw / pw) * parent_uv[2];
                        let uv_h = (sh / ph) * parent_uv[3];

                        (Some(p_atlas_index), Some([uv_x, uv_y, uv_w, uv_h]))
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };

                if let Some(entry) = ctx.registry.images[i].as_mut() {
                    entry.atlas_index = atlas_index;
                    entry.uv_rect = uv_rect;
                    changed = true;
                }
            }
        }

        self.dirty_assets = false;
        ctx.registry.dirty_assets = false;
        Ok(())
    }

    pub fn compress_assets(&mut self, ctx: &mut crate::Context) -> anyhow::Result<()> {
        self.rebuild_atlases(ctx)
    }

    pub(crate) fn rebuild_atlases(&mut self, ctx: &mut crate::Context) -> anyhow::Result<()> {
        self.dirty_assets = false;
        self.model_renderer.clear_texture_bind_group_cache();
        // Drop old atlases
        self.atlases.clear();

        // 1. Reset ALL image ready states since old atlases are gone
        for i in 0..ctx.registry.images.len() {
            if let Some(entry) = ctx.registry.images[i].as_mut() {
                entry.atlas_index = None;
                entry.uv_rect = None;
            }
        }

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

        // 2. Re-pack all root images
        for i in 0..ctx.registry.images.len() {
            if let Some(entry) = ctx.registry.images[i].as_ref()
                && entry.raw_data.is_some()
            {
                let width = entry.bounds.width;
                let height = entry.bounds.height;
                let raw_data = entry.raw_data.clone().unwrap_or_else(|| {
                    panic!("[spot][atlas] image {} lost raw data during rebuild", i)
                });

                let w = width.to_u32_clamped();
                let h = height.to_u32_clamped();

                let (atlas_index, rect) = match self.ensure_atlas_for_image(ctx, w, h) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("[spot][atlas] Failed to pack image {}: {:?}", i, e);
                        continue;
                    }
                };
                let atlas = self
                    .atlases
                    .get_mut(atlas_index as usize)
                    .unwrap_or_else(|| {
                        panic!(
                            "[spot][atlas] atlas {} missing while rebuilding image {}",
                            atlas_index, i
                        )
                    });

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

                if let Some(entry) = ctx.registry.images[i].as_mut() {
                    entry.atlas_index = Some(atlas_index);
                    entry.uv_rect = Some(uv_rect);
                }
            }
        }

        // Phase 2: Resolve sub-images that depend on readied roots
        let mut changed = true;
        while changed {
            changed = false;
            for i in 0..ctx.registry.images.len() {
                let (atlas_index, uv_rect) = if let Some(entry) = ctx.registry.images[i].as_ref()
                    && !entry.is_ready()
                    && let Some(parent_id) = entry.parent_id
                {
                    if let Some(Some(parent)) = ctx.registry.images.get(parent_id as usize)
                        && parent.is_ready()
                    {
                        let parent_uv = parent.uv_rect.unwrap_or_else(|| {
                            panic!(
                                "[spot][atlas] ready parent image {} is missing uv_rect",
                                parent_id
                            )
                        });
                        let p_atlas_index = parent.atlas_index.unwrap_or_else(|| {
                            panic!(
                                "[spot][atlas] ready parent image {} is missing atlas index",
                                parent_id
                            )
                        });

                        let pw = parent.bounds.width.as_f32();
                        let ph = parent.bounds.height.as_f32();
                        let sw = entry.bounds.width.as_f32();
                        let sh = entry.bounds.height.as_f32();
                        let sx = (entry.bounds.x.as_f32() - parent.bounds.x.as_f32()).max(0.0);
                        let sy = (entry.bounds.y.as_f32() - parent.bounds.y.as_f32()).max(0.0);

                        let uv_x = parent_uv[0] + (sx / pw) * parent_uv[2];
                        let uv_y = parent_uv[1] + (sy / ph) * parent_uv[3];
                        let uv_w = (sw / pw) * parent_uv[2];
                        let uv_h = (sh / ph) * parent_uv[3];

                        (Some(p_atlas_index), Some([uv_x, uv_y, uv_w, uv_h]))
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };

                if let Some(entry) = ctx.registry.images[i].as_mut() {
                    entry.atlas_index = atlas_index;
                    entry.uv_rect = uv_rect;
                    changed = true;
                }
            }
        }

        Ok(())
    }

    pub(crate) fn copy_image(
        &mut self,
        ctx: &mut crate::Context,
        dst_id: u32,
        src_id: u32,
    ) -> anyhow::Result<()> {
        let dst_entry = ctx
            .registry
            .images
            .get(dst_id as usize)
            .and_then(|v| v.as_ref())
            .ok_or_else(|| anyhow::anyhow!("invalid dst image"))?;
        let src_entry = ctx
            .registry
            .images
            .get(src_id as usize)
            .and_then(|v| v.as_ref())
            .ok_or_else(|| anyhow::anyhow!("invalid src image"))?;

        if !dst_entry.is_ready() || !src_entry.is_ready() {
            return Err(anyhow::anyhow!("image not ready for copy"));
        }

        let (dst_atlas_index, dst_uv_rect) = expect_ready_image_atlas_info(dst_entry);
        let (src_atlas_index, src_uv_rect) = expect_ready_image_atlas_info(src_entry);

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

        let atlas = self
            .atlases
            .get(dst_atlas_index as usize)
            .unwrap_or_else(|| {
                panic!(
                    "[spot][atlas] atlas {} missing for copy_image on ready image",
                    dst_atlas_index
                )
            });
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

    pub(crate) fn clear_image(
        &mut self,
        ctx: &mut crate::Context,
        target_id: u32,
        color: [f32; 4],
    ) -> anyhow::Result<()> {
        let entry = ctx
            .registry
            .images
            .get(target_id as usize)
            .and_then(|v| v.as_ref())
            .ok_or_else(|| anyhow::anyhow!("invalid target image"))?;
        let _bounds = entry.bounds;

        if !entry.is_ready() {
            return Err(anyhow::anyhow!("image not ready for clear"));
        }

        let (atlas_index, uv_rect) = expect_ready_image_atlas_info(entry);

        let atlas = self.atlases.get(atlas_index as usize).unwrap_or_else(|| {
            panic!(
                "[spot][atlas] atlas {} missing for clear_image on ready image",
                atlas_index
            )
        });
        let aw = atlas.packer.width() as f32;
        let ah = atlas.packer.height() as f32;
        let w = (uv_rect[2] * aw).round() as u32;
        let h = (uv_rect[3] * ah).round() as u32;
        let x = (uv_rect[0] * aw).round() as u32;
        let y = (uv_rect[1] * ah).round() as u32;

        let rgba = [
            (color[0] * 255.0) as u8,
            (color[1] * 255.0) as u8,
            (color[2] * 255.0) as u8,
            (color[3] * 255.0) as u8,
        ];
        let data = vec![rgba; (w * h) as usize]
            .into_iter()
            .flatten()
            .collect::<Vec<u8>>();
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

fn expect_ready_image_atlas_info(entry: &crate::image::ImageEntry) -> (u32, [f32; 4]) {
    let atlas_index = entry
        .atlas_index
        .unwrap_or_else(|| panic!("[spot][atlas] ready image is missing atlas index"));
    let uv_rect = entry
        .uv_rect
        .unwrap_or_else(|| panic!("[spot][atlas] ready image is missing uv rect"));
    (atlas_index, uv_rect)
}
