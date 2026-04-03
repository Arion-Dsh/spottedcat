//! Text layout and queuing for rendering.

use crate::DrawOption;
use crate::ShaderOpts;
use crate::pt::Pt;

use super::Graphics;
use super::core::ResolvedDraw;

impl Graphics {
    pub(crate) fn ensure_text_layout(
        &mut self,
        ctx: &mut crate::Context,
        text: &crate::Text,
        image_scale: [f32; 2],
    ) -> anyhow::Result<()> {
        use crate::text::{CachedGlyph, TextLayout};
        use ab_glyph::{Font as _, FontArc, PxScale, ScaleFont as _};
        use std::sync::atomic::Ordering;

        {
            let cache_lock = text.layout_cache.as_ref().lock().unwrap();
            if !text.dirty.load(Ordering::SeqCst)
                && let Some(layout) = cache_lock.as_ref()
                && layout.scale == image_scale
            {
                return Ok(());
            }
        }

        let font_id = text.font_id;
        let font_data = ctx
            .registry
            .fonts
            .get(&font_id)
            .ok_or_else(|| anyhow::anyhow!("Font ID {} not found", font_id))?;

        let font = if let Some(cached_font) = self.get_cached_font(font_id as u64) {
            cached_font
        } else {
            let font = FontArc::try_from_vec(font_data.clone())
                .unwrap_or_else(|e| panic!("[spot][graphics] Failed to parse font with ID {}: {}", font_id, e));
            self.cache_font(font_id as u64, font.clone());
            font
        };

        let px_size = text.font_size.as_f32().max(1.0);
        let scale = PxScale::from(px_size);
        let scaled = font.as_scaled(scale);

        let lines = if text.max_width.is_some() {
            text.get_wrapped_lines(&scaled)
                .into_iter()
                .map(std::borrow::Cow::Owned)
                .collect()
        } else {
            vec![std::borrow::Cow::Borrowed(text.content.as_str())]
        };

        let mut caret_pos = [Pt(0.0), Pt(0.0)];

        let mut global_min_y = scaled.ascent();
        for line in &lines {
            for ch in line.chars() {
                let id = scaled.glyph_id(ch);
                if let Some(glyph) = scaled.outline_glyph(ab_glyph::Glyph {
                    id,
                    scale,
                    position: ab_glyph::point(0.0, 0.0),
                }) {
                    global_min_y = global_min_y.min(glyph.px_bounds().min.y);
                }
            }
        }
        let y_offset = -global_min_y;

        let ascent = scaled.ascent();
        let descent = scaled.descent();
        let line_height = ascent - descent + scaled.line_gap();

        caret_pos[1] += Pt::from(y_offset - ascent);

        let sx = image_scale[0];
        let sy = image_scale[1];

        let mut cached_glyphs = Vec::new();

        for line in lines {
            let mut prev: Option<ab_glyph::GlyphId> = None;
            let baseline_y = caret_pos[1] + Pt::from(ascent);

            for ch in line.chars() {
                let glyph_id = scaled.glyph_id(ch);

                if let Some(p) = prev {
                    caret_pos[0] += Pt::from(scaled.kern(p, glyph_id));
                }
                prev = Some(glyph_id);

                let cache_key = crate::glyph_cache::GlyphCacheKey {
                    font_id,
                    font_size_bits: px_size.to_bits(),
                    glyph_id: glyph_id.0.into(),
                };

                let entry = if let Some(e) = self.glyph_cache.get(&cache_key) {
                    e.clone()
                } else if let Ok(e) =
                    self.render_single_glyph(ctx, font_id, px_size, glyph_id.0.into())
                {
                    self.glyph_cache.insert(cache_key, e.clone());
                    e
                } else {
                    caret_pos[0] += Pt::from(scaled.h_advance(glyph_id));
                    continue;
                };

                let img_id = entry.image.id();
                let img_entry = if let Some(Some(e)) = ctx.registry.images.get(img_id as usize) {
                    e
                } else {
                    caret_pos[0] += Pt::from(entry.advance);
                    continue;
                };

                let draw_x = caret_pos[0] + Pt::from(entry.offset[0]);
                let draw_y = baseline_y + Pt::from(entry.offset[1]);

                let rel_x = draw_x.as_f32() * sx;
                let rel_y = draw_y.as_f32() * sy;

                let w = img_entry.bounds.width.as_f32() * sx;
                let h = img_entry.bounds.height.as_f32() * sy;

                cached_glyphs.push(CachedGlyph {
                    instance: crate::image_raw::InstanceData {
                        pos: [rel_x, rel_y],
                        rotation: 0.0,
                        size: [w, h],
                        uv_rect: img_entry.uv_rect.unwrap_or([0.0, 0.0, 1.0, 1.0]),
                        clip_rect: [-1.0, -1.0, -1.0, -1.0],
                    },
                    image_id: img_id,
                });

                caret_pos[0] += Pt::from(entry.advance);
            }
            caret_pos[0] = Pt(0.0);
            caret_pos[1] += Pt::from(line_height);
        }

        let new_layout = TextLayout {
            glyphs: cached_glyphs,
            bounds: (0.0, 0.0, y_offset),
            scale: image_scale,
        };

        let mut cache_lock = text.layout_cache.as_ref().lock().unwrap();
        *cache_lock = Some(new_layout);
        text.dirty.store(false, Ordering::SeqCst);

        Ok(())
    }

    pub(crate) fn layout_and_queue_text(
        &mut self,
        ctx: &mut crate::Context,
        text: &crate::Text,
        opts: &DrawOption,
        viewport_rect: [f32; 4],
    ) -> anyhow::Result<()> {
        let start_pos = opts.position();
        let mut shader_opts = ShaderOpts::default();
        shader_opts.set_vec4(0, text.color);
        self.ensure_text_layout(ctx, text, opts.scale())?;
        if ctx.registry.dirty_assets {
            self.process_registrations(ctx)?;
        }

        let cache_lock = text.layout_cache.as_ref().lock().unwrap();
        let Some(layout) = cache_lock.as_ref() else {
            return Ok(());
        };

        for glyph in &layout.glyphs {
            let final_x = start_pos[0].as_f32() + glyph.instance.pos[0];
            let final_y = start_pos[1].as_f32() + glyph.instance.pos[1];

            if final_x + glyph.instance.size[0] >= viewport_rect[0]
                && final_x <= viewport_rect[2]
                && final_y + glyph.instance.size[1] >= viewport_rect[1]
                && final_y <= viewport_rect[3]
                && let Some(Some(img_entry)) = ctx.registry.images.get(glyph.image_id as usize)
                && let Some(atlas_index) = img_entry.atlas_index
                && let Some(uv_rect) = img_entry.uv_rect
            {
                let mut glyph_opts = *opts;
                glyph_opts.set_position(Pt::from(final_x), Pt::from(final_y));

                let draw_index = self.draw_index_counter;
                self.draw_index_counter += 1;

                self.resolved_draws.push(ResolvedDraw {
                    atlas_index,
                    bounds: img_entry.bounds,
                    uv_rect,
                    opts: glyph_opts,
                    shader_id: self.text_shader_id,
                    shader_opts,
                    layer: opts.layer(),
                    draw_index,
                });
            }
        }

        Ok(())
    }
}
