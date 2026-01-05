use crate::{Context, DrawOption};
use std::fmt;
use ab_glyph::{Font as _, ScaleFont as _};

/// Text handle for drawing text to the screen.
///
/// Text can be created and drawn with custom fonts, sizes, colors, and positions.
/// Supports text wrapping with maximum width constraints.
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub(crate) content: String,
    pub(crate) font_size: crate::Pt,
    pub(crate) color: [f32; 4],
    pub(crate) font_data: Vec<u8>,
    pub(crate) font_hash: u64,
    pub(crate) stroke_width: crate::Pt,
    pub(crate) stroke_color: [f32; 4],
    pub(crate) max_width: Option<crate::Pt>,
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
    /// # use spottedcat::{Text, load_font_from_bytes};
    /// const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
    /// let font_data = load_font_from_bytes(FONT);
    /// let text = Text::new("Hello, World!", font_data);
    /// ```
    pub fn new(content: impl Into<String>, font_data: Vec<u8>) -> Self {
        fn hash_bytes(bytes: &[u8]) -> u64 {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            bytes.hash(&mut h);
            h.finish()
        }

        let font_hash = hash_bytes(&font_data);
        Self {
            content: content.into(),
            font_size: crate::Pt(24.0),
            color: [1.0, 1.0, 1.0, 1.0],
            font_data,
            font_hash,
            stroke_width: crate::Pt(0.0),
            stroke_color: [0.0, 0.0, 0.0, 1.0],
            max_width: None,
        }
    }

    pub fn with_font_size(mut self, font_size: crate::Pt) -> Self {
        self.font_size = font_size;
        self
    }

    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    pub fn with_stroke_width(mut self, stroke_width: crate::Pt) -> Self {
        self.stroke_width = stroke_width;
        self
    }

    pub fn with_stroke_color(mut self, stroke_color: [f32; 4]) -> Self {
        self.stroke_color = stroke_color;
        self
    }

    pub fn with_max_width(mut self, max_width: crate::Pt) -> Self {
        self.max_width = Some(max_width);
        self
    }

    /// Returns the font size of this text.
    ///
    /// # Example
    /// ```no_run
    /// # use spottedcat::{Text, load_font_from_bytes};
    /// const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
    /// let font_data = load_font_from_bytes(FONT);
    /// let text = Text::new("Hello, World!", font_data)
    ///     .with_font_size(spottedcat::Pt::from(32.0));
    /// let font_size = text.font_size();
    /// ```
    pub fn font_size(&self) -> crate::Pt {
        self.font_size
    }

    /// Draws this text to the context with the specified options.
    ///
    /// # Arguments
    /// * `context` - The drawing context to add this text to
    /// * `options` - Text drawing options (position, font size, color, scale, font)
    ///
    /// # Example
    /// ```no_run
    /// # use spottedcat::{Context, Text, DrawOption, load_font_from_bytes};
    /// # let mut context = Context::new();
    /// const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
    /// let font_data = load_font_from_bytes(FONT);
    /// let mut opts = DrawOption::default();
    /// opts.set_position([spottedcat::Pt::from(100.0), spottedcat::Pt::from(100.0)]);
    /// Text::new("Hello, World!", font_data)
    ///     .with_font_size(spottedcat::Pt::from(32.0))
    ///     .draw(&mut context, opts);
    /// ```
    /// Returns the logical size of the text in pixels.
    pub fn measure(&self) -> (f32, f32) {
        let (w, h, _) = self.measure_with_y_offset();
        (w, h)
    }

    /// Returns (width, height, y_offset) in pixels.
    ///
    /// `y_offset` can be added to a top-left draw position so that the rendered glyphs' ink bounds
    /// align with the measured box. This helps UI vertical centering look correct.
    /// 
    /// If max_width is set, text will be wrapped and height will account for multiple lines.
    pub fn measure_with_y_offset(&self) -> (f32, f32, f32) {
        use ab_glyph::{Font as _, FontArc, PxScale, ScaleFont as _};

        let font = match FontArc::try_from_vec(self.font_data.clone()) {
            Ok(f) => f,
            Err(_) => return (0.0, 0.0, 0.0),
        };

        let px_size = self.font_size.as_f32().max(1.0);
        let scale = PxScale::from(px_size);
        let scaled = font.as_scaled(scale);

        // Handle text wrapping
        let lines = self.get_wrapped_lines(&scaled);
        
        let mut max_width = 0.0f32;
        let mut total_height = 0.0f32;
        let line_height = scaled.ascent() - scaled.descent();
        
        for line in &lines {
            let line_width = self.measure_line_width(line, &scaled);
            max_width = max_width.max(line_width);
            total_height += line_height;
        }
        
        let y_offset = scaled.ascent();
        (max_width, total_height, y_offset)
    }
    
    /// Get wrapped lines based on max_width constraint
    pub fn get_wrapped_lines(&self, scaled: &ab_glyph::PxScaleFont<&ab_glyph::FontArc>) -> Vec<String> {
        if let Some(max_width) = self.max_width {
            let max_w = max_width.as_f32();
            if max_w <= 0.0 {
                return vec![self.content.clone()];
            }
            
            let mut lines = Vec::new();
            let mut current_line = String::new();
            let mut current_width = 0.0f32;
            let mut prev: Option<ab_glyph::GlyphId> = None;
            
            for word in self.content.split_whitespace() {
                let word_width = self.measure_word_width(word, scaled);
                let space_width = scaled.h_advance(scaled.glyph_id(' '));
                
                if current_line.is_empty() {
                    // First word in line
                    if word_width <= max_w {
                        current_line.push_str(word);
                        current_width = word_width;
                        // Set prev for kerning with next word
                        for ch in word.chars().rev().take(1) {
                            prev = Some(scaled.glyph_id(ch));
                        }
                    } else {
                        // Word is longer than max_width, break it character by character
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
                } else {
                    // Check if word fits on current line
                    let space_and_word_width = if let Some(p) = prev {
                        scaled.kern(p, scaled.glyph_id(' ')) + space_width + word_width
                    } else {
                        space_width + word_width
                    };
                    
                    if current_width + space_and_word_width <= max_w {
                        // Word fits on current line
                        current_line.push(' ');
                        current_line.push_str(word);
                        current_width += space_and_word_width;
                        // Update prev for kerning
                        for ch in word.chars().rev().take(1) {
                            prev = Some(scaled.glyph_id(ch));
                        }
                    } else {
                        // Word doesn't fit, start new line
                        lines.push(current_line.clone());
                        current_line = word.to_string();
                        current_width = word_width;
                        // Update prev for kerning
                        for ch in word.chars().rev().take(1) {
                            prev = Some(scaled.glyph_id(ch));
                        }
                    }
                }
            }
            
            if !current_line.is_empty() {
                lines.push(current_line);
            }
            
            lines
        } else {
            // No wrapping, split by explicit newlines only
            self.content.split('\n').map(|s| s.to_string()).collect()
        }
    }
    
    /// Measure width of a single line
    pub fn measure_line_width(&self, line: &str, scaled: &ab_glyph::PxScaleFont<&ab_glyph::FontArc>) -> f32 {
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
    pub fn measure_word_width(&self, word: &str, scaled: &ab_glyph::PxScaleFont<&ab_glyph::FontArc>) -> f32 {
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

    pub fn draw(&self, context: &mut Context, options: DrawOption) {
        context.push(crate::drawable::DrawCommand::Text(Box::new(self.clone()), options));
    }
}
