//! Font registration and glyph rendering.

use crate::glyph_cache::GlyphEntry;
use crate::pt::Pt;
use ab_glyph::FontArc;

use super::Graphics;

impl Graphics {
    pub(crate) fn register_font(&mut self, font_data: Vec<u8>) -> u32 {
        let font_id = self.next_font_id;
        self.next_font_id = self.next_font_id.saturating_add(1);
        self.font_registry.insert(font_id, font_data);
        font_id
    }

    pub(crate) fn get_font(&self, font_id: u32) -> Option<&Vec<u8>> {
        self.font_registry.get(&font_id)
    }

    pub(crate) fn unregister_font(&mut self, font_id: u32) {
        self.font_registry.remove(&font_id);
        self.font_cache.remove(&(font_id as u64));
        self.dirty_assets = true;
    }

    /// Render a single glyph to the atlas and cache it
    pub(super) fn render_single_glyph(
        &mut self,
        font_id: u32,
        font_size: f32,
        glyph_id: u32,
    ) -> anyhow::Result<GlyphEntry> {
        use ab_glyph::{Font as _, FontArc, Glyph, PxScale, ScaleFont as _};

        let font_data = self
            .get_font(font_id)
            .ok_or_else(|| anyhow::anyhow!("Font ID {} not found", font_id))?;

        let font = if let Some(cached_font) = self.get_cached_font(font_id as u64) {
            cached_font
        } else {
            let font = FontArc::try_from_vec(font_data.clone())
                .map_err(|e| anyhow::anyhow!("Failed to parse font: {:?}", e))?;
            self.cache_font(font_id as u64, font.clone());
            font
        };

        let px_size = font_size.max(1.0);
        let scale = PxScale::from(px_size);
        let scaled = font.as_scaled(scale);

        let glyph = Glyph {
            id: ab_glyph::GlyphId(glyph_id as u16),
            scale,
            position: ab_glyph::point(0.0, 0.0),
        };

        let h_advance = scaled.h_advance(glyph.id);

        let outlined = scaled
            .outline_glyph(glyph)
            .ok_or_else(|| anyhow::anyhow!("Cannot outline glyph"))?;

        let bounds = outlined.px_bounds();
        let glyph_width = (bounds.max.x - bounds.min.x).ceil().max(1.0) as u32;
        let glyph_height = (bounds.max.y - bounds.min.y).ceil().max(1.0) as u32;

        let mut rgba_data = vec![0u8; (glyph_width * glyph_height * 4) as usize];

        outlined.draw(|x, y, v| {
            if x < glyph_width && y < glyph_height {
                let idx = ((y * glyph_width + x) * 4) as usize;
                let alpha = (v * 255.0).round().clamp(0.0, 255.0) as u8;
                rgba_data[idx] = 255;
                rgba_data[idx + 1] = 255;
                rgba_data[idx + 2] = 255;
                rgba_data[idx + 3] = alpha;
            }
        });

        let image = self.create_image(
            Pt::from(glyph_width as f32),
            Pt::from(glyph_height as f32),
            &rgba_data,
        )?;

        let image_entry = self
            .images
            .get(image.index())
            .and_then(|e| e.as_ref())
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Failed to get created glyph image"))?;

        Ok(GlyphEntry {
            image: image_entry,
            offset: [bounds.min.x, bounds.min.y],
            advance: h_advance,
        })
    }

    pub(super) fn get_cached_font(&self, font_hash: u64) -> Option<FontArc> {
        self.font_cache.get(&font_hash).cloned()
    }

    pub(super) fn cache_font(&mut self, font_hash: u64, font: FontArc) {
        self.font_cache.insert(font_hash, font);
    }
}
