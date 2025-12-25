use crate::{Image, Pt};
use crate::Text;

#[derive(Debug, Clone, PartialEq)]
pub enum DrawAble {
    Image(Image),
    Text(Text),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DrawCommand {
    Image(Image, DrawOption),
    Text(Text, DrawOption),
}


/// Unified options for drawing images and text.
///
/// Controls the position, rotation, and scale of drawn items.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawOption {
    /// Position in screen pixels (top-left corner). Origin is at top-left of window.
    pub position: [Pt; 2],
    /// Rotation in radians.
    pub rotation: f32,
    /// Scale factors (x, y). Applied after size.
    pub scale: [f32; 2],
}

impl Default for DrawOption {
    fn default() -> Self {
        Self {
            position: [Pt(10.0), Pt(10.0)],
            scale: [1.0, 1.0],
            rotation: 0.0,
        }
    }
}

impl DrawOption {
    pub fn new() -> Self {
        Self::default()
    }
}