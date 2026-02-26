//! Text layout and queuing for rendering.

use crate::DrawOption;
use crate::ShaderOpts;
use crate::Text;
use crate::glyph_cache::GlyphCacheKey;
use crate::pt::Pt;

use super::Graphics;
use super::core::ResolvedDraw;

impl Graphics {
    pub(crate) fn layout_and_queue_text(
        &mut self,
        text: &Text,
        opts: &DrawOption,
        viewport_rect: [f32; 4],
    ) -> anyhow::Result<()> {
        use ab_glyph::{Font as _, FontArc, PxScale, ScaleFont as _};

        let font_id = text.font_id;
        let font_data = self
            .get_font(font_id)
            .ok_or_else(|| anyhow::anyhow!("Font ID {} not found", font_id))?;

        let font = if let Some(cached_font) = self.get_cached_font(font_id as u64) {
            cached_font
        } else {
            let font = FontArc::try_from_vec(font_data.clone())
                .map_err(|e| anyhow::anyhow!("Failed to parse font: {}", e))?;
            self.cache_font(font_id as u64, font.clone());
            font
        };

        let px_size = text.font_size.as_f32().max(1.0);
        let scale = PxScale::from(px_size);
        let scaled = font.as_scaled(scale);

        let lines = if text.max_width.is_some() {
            text.get_wrapped_lines(&scaled)
                .into_iter()
                .map(|s| std::borrow::Cow::Owned(s))
                .collect()
        } else {
            vec![std::borrow::Cow::Borrowed(text.content.as_str())]
        };

        let start_pos = opts.position();
        let mut caret_pos = start_pos;

        // Calculate y_offset without calling text.measure_with_y_offset() to avoid re-entrant lock.
        // We use the same ink-bounds logic as the measure system.
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

        // Adjust caret_pos[1] so that the first line's baseline aligns with the ink bounds top
        // baseline_y = caret_pos[1] + ascent
        // We want baseline_y = start_pos[1] + y_offset
        // So caret_pos[1] = start_pos[1] + Pt::from(y_offset - ascent)
        caret_pos[1] += Pt::from(y_offset - ascent);

        let image_scale = opts.scale();
        let sx = image_scale[0];
        let sy = image_scale[1];

        for line in lines {
            let mut prev: Option<ab_glyph::GlyphId> = None;
            let baseline_y = caret_pos[1] + Pt::from(ascent);

            for ch in line.chars() {
                let glyph_id = scaled.glyph_id(ch);
                // ... (rest same)

                if let Some(p) = prev {
                    caret_pos[0] += Pt::from(scaled.kern(p, glyph_id));
                }
                prev = Some(glyph_id);

                let cache_key = GlyphCacheKey {
                    font_id,
                    font_size_bits: px_size.to_bits(),
                    glyph_id: glyph_id.0 as u32,
                };

                let entry = if let Some(e) = self.glyph_cache.get(&cache_key) {
                    e.clone()
                } else {
                    if let Ok(e) = self.render_single_glyph(font_id, px_size, glyph_id.0 as u32) {
                        self.glyph_cache.insert(cache_key, e.clone());
                        e
                    } else {
                        caret_pos[0] += Pt::from(scaled.h_advance(glyph_id));
                        continue;
                    }
                };

                let img_id = entry.image.id() as usize;
                let img_entry = if let Some(Some(e)) = self.images.get(img_id) {
                    e
                } else {
                    caret_pos[0] += Pt::from(entry.advance);
                    continue;
                };

                let draw_x = caret_pos[0] + Pt::from(entry.offset[0]);
                let draw_y = baseline_y + Pt::from(entry.offset[1]);

                let rel_x = (draw_x - start_pos[0]).as_f32() * sx;
                let rel_y = (draw_y - start_pos[1]).as_f32() * sy;

                let final_x = start_pos[0].as_f32() + rel_x;
                let final_y = start_pos[1].as_f32() + rel_y;

                let w = img_entry.bounds.width.as_f32() * sx;
                let h = img_entry.bounds.height.as_f32() * sy;

                if final_x + w >= viewport_rect[0]
                    && final_x <= viewport_rect[2]
                    && final_y + h >= viewport_rect[1]
                    && final_y <= viewport_rect[3]
                {
                    let mut glyph_opts = *opts;
                    glyph_opts.set_position(Pt::from(final_x), Pt::from(final_y));

                    let mut shader_opts = ShaderOpts::default();
                    shader_opts.set_vec4(0, text.color);

                    self.resolved_draws.push(ResolvedDraw {
                        img_entry: img_entry.clone(),
                        opts: glyph_opts,
                        shader_id: self.text_shader_id,
                        shader_opts,
                    });
                }

                caret_pos[0] += Pt::from(entry.advance);
            }
            caret_pos[0] = start_pos[0];
            caret_pos[1] += Pt::from(line_height);
        }
        Ok(())
    }
}
