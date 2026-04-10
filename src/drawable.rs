use crate::Pt;
use crate::ShaderOpts;
use crate::Text;
#[cfg(feature = "model-3d")]
pub(crate) use crate::drawable_3d::DrawCommand3D;

/// Trait for objects that can be drawn into an [`Image`][crate::Image].
pub trait Drawable {
    /// Associated options for configuring the draw call (e.g., [`DrawOption`][crate::DrawOption]).
    type Options;

    /// Renders the object into the specified target image.
    ///
    /// This is the low-level drawing interface. Users should typically call
    /// `target.draw(ctx, drawable, options)` instead.
    fn draw_to(self, ctx: &mut crate::Context, target: crate::Image, options: Self::Options);
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ImageCommand {
    pub id: u32,
    pub target_texture_id: u32,
    pub opts: DrawOption,
    pub shader_id: u32,
    pub shader_opts: ShaderOpts,
    pub size: [Pt; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextCommand {
    pub target_texture_id: u32,
    pub text: Box<Text>,
    pub opts: DrawOption,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DrawCommand {
    Image(Box<ImageCommand>),
    Text(Box<TextCommand>),
}

/// Unified options for drawing images and text.
///
/// Controls the position, rotation, and scale of drawn items.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawOption {
    /// Position in logical units relative to the target's top-left corner.
    position: [Pt; 2],
    /// Rotation in radians.
    rotation: f32,
    /// Scale factors (x, y). Applied after size.
    scale: [f32; 2],
    opacity: f32,
}

impl Default for DrawOption {
    fn default() -> Self {
        Self {
            position: [Pt(0.0), Pt(0.0)],
            scale: [1.0, 1.0],
            rotation: 0.0,
            opacity: 1.0,
        }
    }
}

impl DrawOption {
    /// Creates a new DrawOption with position, rotation, and scale.
    pub fn new(position: [Pt; 2], rotation: f32, scale: [f32; 2]) -> Self {
        Self {
            position,
            rotation,
            scale,
            opacity: 1.0,
        }
    }

    pub fn position(&self) -> [Pt; 2] {
        self.position
    }

    /// Sets the drawing position relative to the current target's top-left corner.
    pub fn with_position(mut self, position: [Pt; 2]) -> Self {
        self.position = position;
        self
    }

    pub fn set_position(&mut self, x: Pt, y: Pt) {
        self.position = [x, y];
    }

    pub fn rotation(&self) -> f32 {
        self.rotation
    }

    /// Sets the rotation in radians.
    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn scale(&self) -> [f32; 2] {
        self.scale
    }

    /// Sets the scale multiplier (e.g., [2.0, 2.0] for double size).
    pub fn with_scale(mut self, scale: [f32; 2]) -> Self {
        self.scale = scale;
        self
    }

    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    /// Sets the opacity (alpha multiplier), from 0.0 to 1.0.
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }
}
