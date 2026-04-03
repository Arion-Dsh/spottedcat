use crate::{Context, DrawOption};
use ab_glyph::ScaleFont as _;
use std::fmt;

/// Text handle for drawing text to the screen.
///
/// Text can be created and drawn with custom fonts, sizes, colors, and positions.
/// Supports text wrapping with maximum width constraints.
#[derive(Debug)]
pub struct Text {
    pub(crate) content: String,
    pub(crate) font_size: crate::Pt,
    pub(crate) color: [f32; 4],
    pub(crate) font_id: u32,
    pub(crate) stroke_width: crate::Pt,
    pub(crate) stroke_color: [f32; 4],
    pub(crate) max_width: Option<crate::Pt>,
    pub(crate) layout_cache: std::sync::Arc<std::sync::Mutex<Option<TextLayout>>>,
    pub(crate) dirty: std::sync::atomic::AtomicBool,
}

impl Clone for Text {
    fn clone(&self) -> Self {
        Self {
            content: self.content.clone(),
            font_size: self.font_size,
            color: self.color,
            font_id: self.font_id,
            stroke_width: self.stroke_width,
            stroke_color: self.stroke_color,
            max_width: self.max_width,
            layout_cache: std::sync::Arc::new(std::sync::Mutex::new(None)),
            dirty: std::sync::atomic::AtomicBool::new(true),
        }
    }
}

impl PartialEq for Text {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content
            && self.font_size == other.font_size
            && self.color == other.color
            && self.font_id == other.font_id
            && self.stroke_width == other.stroke_width
            && self.stroke_color == other.stroke_color
            && self.max_width == other.max_width
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextLayout {
    pub(crate) glyphs: Vec<CachedGlyph>,
    pub(crate) bounds: (f32, f32, f32), // width, height, y_offset
    pub(crate) scale: [f32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CachedGlyph {
    pub(crate) instance: crate::image_raw::InstanceData,
    pub(crate) image_id: u32,
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.content)
    }
}

impl Text {
    /// Creates a new text instance with the given content.
    ///
    /// # Arguments
    /// * `content` - The text string to display
    ///
    /// # Example
    /// ```no_run
    /// # use spottedcat::{Context, Text};
    /// # fn example(ctx: &mut Context) {
    /// const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
    /// let font_id = spottedcat::register_font(ctx, FONT.to_vec());
    /// let text = Text::new("Hello, World!", font_id);
    /// # }
    /// ```
    pub fn new(content: impl Into<String>, font_id: u32) -> Self {
        Self {
            content: content.into(),
            font_size: crate::Pt(24.0),
            color: [1.0, 1.0, 1.0, 1.0],
            font_id,
            stroke_width: crate::Pt(0.0),
            stroke_color: [0.0, 0.0, 0.0, 1.0],
            max_width: None,
            layout_cache: std::sync::Arc::new(std::sync::Mutex::new(None)),
            dirty: std::sync::atomic::AtomicBool::new(true),
        }
    }

    /// Sets the text content safely without re-allocating the entire struct.
    pub fn set_content(&mut self, content: impl Into<String>) {
        let new_content = content.into();
        if self.content != new_content {
            self.content = new_content;
            self.dirty.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    pub fn set_color(&mut self, color: [f32; 4]) {
        if self.color != color {
            self.color = color;
            // self.dirty = true; // Color change does not need re-layout if using tinting
        }
    }

    pub fn set_font_size(&mut self, font_size: crate::Pt) {
        if self.font_size != font_size {
            self.font_size = font_size;
            self.dirty.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    pub fn set_max_width(&mut self, max_width: Option<crate::Pt>) {
        if self.max_width != max_width {
            self.max_width = max_width;
            self.dirty.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    pub fn with_font_size(mut self, font_size: crate::Pt) -> Self {
        if self.font_size != font_size {
            self.font_size = font_size;
            self.dirty.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        self
    }

    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        // self.dirty.store(true, Ordering::SeqCst);
        self
    }

    pub fn with_stroke_width(mut self, stroke_width: crate::Pt) -> Self {
        if self.stroke_width != stroke_width {
            self.stroke_width = stroke_width;
            self.dirty.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        self
    }

    pub fn with_stroke_color(mut self, stroke_color: [f32; 4]) -> Self {
        if self.stroke_color != stroke_color {
            self.stroke_color = stroke_color;
            self.dirty.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        self
    }

    pub fn with_max_width(mut self, max_width: crate::Pt) -> Self {
        if self.max_width != Some(max_width) {
            self.max_width = Some(max_width);
            self.dirty.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        self
    }

    /// Returns the font size of this text.
    ///
    /// # Example
    /// ```no_run
    /// # use spottedcat::{Context, Text};
    /// # fn example(ctx: &mut Context) {
    /// # const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
    /// # let font_id = spottedcat::register_font(ctx, FONT.to_vec());
    /// let text = Text::new("Hello, World!", font_id)
    ///     .with_font_size(spottedcat::Pt::from(32.0));
    /// let font_size = text.font_size();
    /// # }
    /// ```
    pub fn font_size(&self) -> crate::Pt {
        self.font_size
    }

    pub fn font_id(&self) -> u32 {
        self.font_id
    }

    pub fn max_width(&self) -> Option<crate::Pt> {
        self.max_width
    }

    /// Draws this text to the context with the specified options.
    ///
    /// # Arguments
    /// * `context` - The drawing context to add this text to
    /// * `options` - Text drawing options (position, font size, color, scale, font)
    ///
    /// # Example
    /// ```no_run
    /// # use spottedcat::{Context, Text, DrawOption};
    /// # fn example(ctx: &mut Context) {
    /// # const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
    /// # let font_id = spottedcat::register_font(ctx, FONT.to_vec());
    /// let opts = DrawOption::default()
    ///     .with_position([spottedcat::Pt::from(100.0), spottedcat::Pt::from(100.0)]);
    /// Text::new("Hello, World!", font_id)
    ///     .with_font_size(spottedcat::Pt::from(32.0))
    ///     .draw(ctx, opts);
    /// # }
    /// ```
    /// Returns the logical size of the text in pixels.
    pub(crate) fn measure(&self, ctx: &Context) -> (f32, f32) {
        let (w, h, _) = self.measure_with_y_offset(ctx);
        (w, h)
    }

    /// Returns (width, height, y_offset) in pixels.
    ///
    /// `y_offset` can be added to a top-left draw position so that the rendered glyphs' ink bounds
    /// align with the measured box. This helps UI vertical centering look correct.
    ///
    /// If max_width is set, text will be wrapped and height will account for multiple lines.
    pub(crate) fn measure_with_y_offset(&self, ctx: &Context) -> (f32, f32, f32) {
        use ab_glyph::{Font as _, FontArc, Glyph, PxScale, ScaleFont as _};

        let font_data = match ctx.registry.fonts.get(&self.font_id) {
            Some(data) => data,
            None => return (0.0, 0.0, 0.0),
        };

        let font = match FontArc::try_from_vec(font_data.clone()) {
            Ok(f) => f,
            Err(_) => return (0.0, 0.0, 0.0),
        };

        let px_size = self.font_size.as_f32().max(1.0);
        let scale = PxScale::from(px_size);
        let scaled = font.as_scaled(scale);

        // Handle text wrapping
        let lines = self.get_wrapped_lines(&scaled);
        if lines.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let mut max_width = 0.0f32;
        let mut global_min_y = scaled.ascent();
        let mut min_top = f32::INFINITY;
        let mut max_bottom = 0.0f32;
        let line_height = scaled.ascent() - scaled.descent() + scaled.line_gap();

        for line in &lines {
            let line_width = self.measure_line_width(line, &scaled);
            max_width = max_width.max(line_width);

            // Calculate actual glyph bounds for this line (same as render_text_to_image)
            let mut line_min_y = scaled.ascent();
            let mut line_max_y = scaled.descent();

            for ch in line.chars() {
                let id = scaled.glyph_id(ch);
                if let Some(glyph) = scaled.outline_glyph(Glyph {
                    id,
                    scale,
                    position: ab_glyph::point(0.0, 0.0),
                }) {
                    let bounds = glyph.px_bounds();
                    line_min_y = line_min_y.min(bounds.min.y);
                    line_max_y = line_max_y.max(bounds.max.y);
                }
            }

            // Track global bounds for y_offset calculation
            global_min_y = global_min_y.min(line_min_y);
        }

        // y_offset should align with the baseline used in rendering
        let y_offset = -global_min_y;

        for (index, line) in lines.iter().enumerate() {
            let mut line_min_y = scaled.ascent();
            let mut line_max_y = scaled.descent();

            for ch in line.chars() {
                let id = scaled.glyph_id(ch);
                if let Some(glyph) = scaled.outline_glyph(Glyph {
                    id,
                    scale,
                    position: ab_glyph::point(0.0, 0.0),
                }) {
                    let bounds = glyph.px_bounds();
                    line_min_y = line_min_y.min(bounds.min.y);
                    line_max_y = line_max_y.max(bounds.max.y);
                }
            }

            let baseline_y = y_offset + index as f32 * line_height;
            min_top = min_top.min(baseline_y + line_min_y);
            max_bottom = max_bottom.max(baseline_y + line_max_y);
        }

        let total_height = if min_top.is_finite() {
            (max_bottom - min_top).max(0.0)
        } else {
            0.0
        };

        (max_width, total_height, y_offset)
    }

    /// Get wrapped lines based on max_width constraint
    pub fn get_wrapped_lines(
        &self,
        scaled: &ab_glyph::PxScaleFont<&ab_glyph::FontArc>,
    ) -> Vec<String> {
        if let Some(max_width) = self.max_width {
            let max_w = max_width.as_f32();
            if max_w <= 0.0 {
                return self.content.split('\n').map(|s| s.to_string()).collect();
            }

            let mut lines = Vec::new();
            for paragraph in self.content.split('\n') {
                self.wrap_paragraph(paragraph, scaled, max_w, &mut lines);
            }
            lines
        } else {
            // No wrapping, split by explicit newlines only
            self.content.split('\n').map(|s| s.to_string()).collect()
        }
    }

    fn wrap_paragraph(
        &self,
        paragraph: &str,
        scaled: &ab_glyph::PxScaleFont<&ab_glyph::FontArc>,
        max_w: f32,
        lines: &mut Vec<String>,
    ) {
        if paragraph.is_empty() {
            lines.push(String::new());
            return;
        }

        let mut current_line = String::new();
        let mut current_width = 0.0f32;
        let mut prev: Option<ab_glyph::GlyphId> = None;
        let mut saw_word = false;

        for word in paragraph.split_whitespace() {
            saw_word = true;
            let word_width = self.measure_word_width(word, scaled);
            let space_width = scaled.h_advance(scaled.glyph_id(' '));

            if current_line.is_empty() {
                if word_width <= max_w {
                    current_line.push_str(word);
                    current_width = word_width;
                    prev = word.chars().next_back().map(|ch| scaled.glyph_id(ch));
                } else {
                    self.wrap_long_word(word, scaled, max_w, lines);
                }
            } else {
                let space_and_word_width = if let Some(p) = prev {
                    scaled.kern(p, scaled.glyph_id(' ')) + space_width + word_width
                } else {
                    space_width + word_width
                };

                if current_width + space_and_word_width <= max_w {
                    current_line.push(' ');
                    current_line.push_str(word);
                    current_width += space_and_word_width;
                    prev = word.chars().next_back().map(|ch| scaled.glyph_id(ch));
                } else {
                    lines.push(current_line.clone());
                    current_line.clear();
                    current_width = 0.0;
                    prev = None;

                    if word_width <= max_w {
                        current_line.push_str(word);
                        current_width = word_width;
                        prev = word.chars().next_back().map(|ch| scaled.glyph_id(ch));
                    } else {
                        self.wrap_long_word(word, scaled, max_w, lines);
                    }
                }
            }
        }

        if !saw_word {
            lines.push(String::new());
            return;
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }

    fn wrap_long_word(
        &self,
        word: &str,
        scaled: &ab_glyph::PxScaleFont<&ab_glyph::FontArc>,
        max_w: f32,
        lines: &mut Vec<String>,
    ) {
        let mut char_line = String::new();
        let mut char_width = 0.0f32;
        let mut char_prev: Option<ab_glyph::GlyphId> = None;

        for ch in word.chars() {
            let id = scaled.glyph_id(ch);
            let char_w = if let Some(p) = char_prev {
                scaled.kern(p, id) + scaled.h_advance(id)
            } else {
                scaled.h_advance(id)
            };

            if char_width + char_w <= max_w && !char_line.is_empty() {
                char_line.push(ch);
                char_width += char_w;
                char_prev = Some(id);
            } else if char_line.is_empty() {
                char_line.push(ch);
                char_width = char_w;
                char_prev = Some(id);
            } else {
                lines.push(char_line);
                char_line = ch.to_string();
                char_width = char_w;
                char_prev = Some(id);
            }
        }

        if !char_line.is_empty() {
            lines.push(char_line);
        }
    }

    /// Measure width of a single line
    pub fn measure_line_width(
        &self,
        line: &str,
        scaled: &ab_glyph::PxScaleFont<&ab_glyph::FontArc>,
    ) -> f32 {
        let mut width = 0.0f32;
        let mut prev: Option<ab_glyph::GlyphId> = None;

        for ch in line.chars() {
            let id = scaled.glyph_id(ch);
            if let Some(p) = prev {
                width += scaled.kern(p, id);
            }
            width += scaled.h_advance(id);
            prev = Some(id);
        }

        width
    }

    /// Measure width of a single word (for wrapping logic)
    pub fn measure_word_width(
        &self,
        word: &str,
        scaled: &ab_glyph::PxScaleFont<&ab_glyph::FontArc>,
    ) -> f32 {
        let mut width = 0.0f32;
        let mut prev: Option<ab_glyph::GlyphId> = None;

        for ch in word.chars() {
            let id = scaled.glyph_id(ch);
            if let Some(p) = prev {
                width += scaled.kern(p, id);
            }
            width += scaled.h_advance(id);
            prev = Some(id);
        }

        width
    }

    pub(crate) fn draw(&self, ctx: &mut Context, options: DrawOption) {
        // Draw text at the exact position provided by the caller
        // The caller is responsible for handling baseline offset if needed
        ctx.push(crate::drawable::DrawCommand::Text(
            Box::new(self.clone()),
            options,
        ));
    }
}

/// Draws text to the screen.
pub fn draw(ctx: &mut Context, text: &Text, options: DrawOption) {
    text.draw(ctx, options);
}

/// Returns the measured text size in logical pixels.
pub fn measure(ctx: &Context, text: &Text) -> (f32, f32) {
    text.measure(ctx)
}

/// Returns measured text size and y-offset in logical pixels.
pub fn measure_with_y_offset(ctx: &Context, text: &Text) -> (f32, f32, f32) {
    text.measure_with_y_offset(ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ab_glyph::{Font as _, FontArc, Glyph, PxScale};

    const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");

    #[test]
    fn measure_matches_rendered_multiline_height() {
        let mut ctx = Context::new();
        let font_id = crate::register_font(&mut ctx, FONT.to_vec());
        let text = Text::new("Ag\nAg", font_id).with_font_size(crate::Pt::from(24.0));

        let (width, height, y_offset) = text.measure_with_y_offset(&ctx);

        let font = FontArc::try_from_vec(FONT.to_vec()).expect("font");
        let scale = PxScale::from(24.0);
        let scaled = font.as_scaled(scale);
        let lines = text.get_wrapped_lines(&scaled);
        let line_height = scaled.ascent() - scaled.descent() + scaled.line_gap();

        let mut global_min_y = scaled.ascent();
        let mut line_bounds = Vec::new();
        for line in &lines {
            let line_width = text.measure_line_width(line, &scaled);
            let mut line_min_y = scaled.ascent();
            let mut line_max_y = scaled.descent();

            for ch in line.chars() {
                let id = scaled.glyph_id(ch);
                if let Some(glyph) = scaled.outline_glyph(Glyph {
                    id,
                    scale,
                    position: ab_glyph::point(0.0, 0.0),
                }) {
                    let bounds = glyph.px_bounds();
                    line_min_y = line_min_y.min(bounds.min.y);
                    line_max_y = line_max_y.max(bounds.max.y);
                }
            }

            global_min_y = global_min_y.min(line_min_y);
            line_bounds.push((line_width, line_min_y, line_max_y));
        }

        let expected_y_offset = -global_min_y;
        let mut min_top = f32::INFINITY;
        let mut max_bottom = 0.0f32;
        let mut expected_width = 0.0f32;

        for (index, (line_width, line_min_y, line_max_y)) in line_bounds.iter().enumerate() {
            let baseline_y = expected_y_offset + index as f32 * line_height;
            expected_width = expected_width.max(*line_width);
            min_top = min_top.min(baseline_y + line_min_y);
            max_bottom = max_bottom.max(baseline_y + line_max_y);
        }

        let expected_height = (max_bottom - min_top).max(0.0);

        assert!((width - expected_width).abs() < 0.01);
        assert!((height - expected_height).abs() < 0.01);
        assert!((y_offset - expected_y_offset).abs() < 0.01);
    }

    #[test]
    fn wrapping_preserves_explicit_newlines() {
        let font = FontArc::try_from_vec(FONT.to_vec()).expect("font");
        let scaled = font.as_scaled(PxScale::from(24.0));
        let text = Text::new("hello\nworld", 1).with_max_width(crate::Pt::from(1.0));

        let lines = text.get_wrapped_lines(&scaled);

        assert!(lines.len() >= 2);
        assert_eq!(lines.first().map(String::as_str), Some("h"));
        assert_eq!(lines.last().map(String::as_str), Some("d"));
    }
}
