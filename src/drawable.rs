use crate::{Image, Pt};
use crate::Text;

#[derive(Debug, Clone, PartialEq)]
pub enum DrawAble {
    Image(Image),
    Text(Text),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DrawCommand {
    Image(Image, ImageDrawOptions),
    Text(Text, TextOptions),
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawOption {
    pub options: ImageDrawOptions,
}


/// Options for drawing images.
///
/// Controls the position, size, rotation, and scale of drawn images.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageDrawOptions {
    /// Position in screen pixels (top-left corner). Origin is at top-left of window.
    pub position: [Pt; 2],
    /// Rotation in radians.
    pub rotation: f32,
    /// Scale factors (x, y). Applied after size.
    pub scale: [f32; 2],
}

impl Default for ImageDrawOptions {
    fn default() -> Self {
        Self {
            position: [Pt(10.0), Pt(10.0)],
            scale: [1.0, 1.0],
            rotation: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextOptions {
    pub position: [Pt; 2],
    pub font_size: Pt,
    pub color: [f32; 4],
    pub scale: [f32; 2],
    pub font_data: Vec<u8>,
    pub stroke_width: Pt,
    pub stroke_color: [f32; 4],
}

impl TextOptions {
    pub fn new(font_data: Vec<u8>) -> Self {
        Self {
            position: [Pt(10.0), Pt(10.0)],
            font_size: Pt(24.0),
            color: [1.0, 1.0, 1.0, 1.0],
            scale: [1.0, 1.0],
            font_data,
            stroke_width: Pt(0.0),
            stroke_color: [0.0, 0.0, 0.0, 1.0],
        }
    }

    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let font_data = std::fs::read(path)?;
        Ok(Self::new(font_data))
    }

    pub fn with_font_from_bytes(mut self, font_data: Vec<u8>) -> Self {
        self.font_data = font_data;
        self
    }

    pub fn with_font_from_file(mut self, path: &str) -> anyhow::Result<Self> {
        let font_data = std::fs::read(path)?;
        self.font_data = font_data;
        Ok(self)
    }
}