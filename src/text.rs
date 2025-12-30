use crate::{Context, DrawOption};
use std::fmt;

/// Text handle for drawing text to the screen.
///
/// Text can be created and drawn with custom fonts, sizes, colors, and positions.
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub(crate) content: String,
    pub(crate) font_size: crate::Pt,
    pub(crate) color: [f32; 4],
    pub(crate) font_data: Vec<u8>,
    pub(crate) stroke_width: crate::Pt,
    pub(crate) stroke_color: [f32; 4],
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
        Self {
            content: content.into(),
            font_size: crate::Pt(24.0),
            color: [1.0, 1.0, 1.0, 1.0],
            font_data,
            stroke_width: crate::Pt(0.0),
            stroke_color: [0.0, 0.0, 0.0, 1.0],
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
    /// let mut opts = DrawOption::new();
    /// opts.position = [spottedcat::Pt::from(100.0), spottedcat::Pt::from(100.0)];
    /// Text::new("Hello, World!", font_data)
    ///     .with_font_size(spottedcat::Pt::from(32.0))
    ///     .draw(&mut context, opts);
    /// ```
    pub fn draw(self, context: &mut Context, options: DrawOption) {
        context.push(crate::drawable::DrawCommand::Text(Box::new(self), options));
    }
}
