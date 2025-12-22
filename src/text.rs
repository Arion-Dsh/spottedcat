use crate::{Context, TextOptions};
use std::fmt;

/// Text handle for drawing text to the screen.
///
/// Text can be created and drawn with custom fonts, sizes, colors, and positions.
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    content: String,
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
    /// # use spottedcat::Text;
    /// let text = Text::new("Hello, World!");
    /// ```
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }

    /// Draws this text to the context with the specified options.
    ///
    /// # Arguments
    /// * `context` - The drawing context to add this text to
    /// * `options` - Text drawing options (position, font size, color, scale, font)
    ///
    /// # Example
    /// ```no_run
    /// # use spottedcat::{Context, Text, TextOptions, load_font_from_bytes};
    /// # let mut context = Context::new();
    /// const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
    /// let mut opts = TextOptions::new(load_font_from_bytes(FONT));
    /// opts.position = [spottedcat::Pt(100.0), spottedcat::Pt(100.0)];
    /// opts.font_size = spottedcat::Pt(32.0);
    /// Text::new("Hello, World!").draw(&mut context, opts);
    /// ```
    pub fn draw(self, context: &mut Context, options: TextOptions) {
        context.push_text(self.content, options);
    }
}
