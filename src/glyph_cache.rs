use crate::image::ImageEntry;
use std::collections::HashMap;

/// Key for caching individual glyphs in the atlas
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct GlyphCacheKey {
    pub font_id: u32,
    pub font_size_bits: u32,
    pub glyph_id: u32,
}

/// Cached glyph data including atlas image and positioning offset
#[derive(Clone, Copy, Debug)]
pub(crate) struct GlyphEntry {
    pub image: ImageEntry,
    pub offset: [f32; 2],
    pub advance: f32, // Horizontal advance width
}

/// Manages glyph rendering and caching to texture atlas
pub(crate) struct GlyphCache {
    cache: HashMap<GlyphCacheKey, GlyphEntry>,
}

impl GlyphCache {
    pub(crate) fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub(crate) fn get(&self, key: &GlyphCacheKey) -> Option<&GlyphEntry> {
        self.cache.get(key)
    }

    pub(crate) fn insert(&mut self, key: GlyphCacheKey, entry: GlyphEntry) {
        self.cache.insert(key, entry);
    }

    pub(crate) fn contains(&self, key: &GlyphCacheKey) -> bool {
        self.cache.contains_key(key)
    }
}

impl Default for GlyphCache {
    fn default() -> Self {
        Self::new()
    }
}
