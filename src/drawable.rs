use crate::Pt;
use crate::ShaderOpts;
use crate::Text;
#[cfg(feature = "model-3d")]
pub(crate) use crate::drawable_3d::DrawCommand3D;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ImageCommand {
    pub id: u32,
    pub opts: DrawOption,
    pub shader_id: u32,
    pub shader_opts: ShaderOpts,
    pub size: [Pt; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DrawCommand {
    Image(Box<ImageCommand>),
    Text(Box<Text>, DrawOption),
    #[allow(dead_code)]
    ClearImage(u32, [f32; 4]),
    #[allow(dead_code)]
    CopyImage(u32, u32),
}

/// Unified options for drawing images and text.
///
/// Controls the position, rotation, and scale of drawn items.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawOption {
    /// Position in screen pixels (top-left corner). Origin is at top-left of window.
    position: [Pt; 2],
    /// Rotation in radians.
    rotation: f32,
    /// Scale factors (x, y). Applied after size.
    scale: [f32; 2],
    opacity: f32,
    /// Layer for sorting (z-index). Higher values are drawn later.
    layer: i32,
    /// Optional clipping rectangle [x, y, width, height] in screen pixels.
    clip: Option<[Pt; 4]>,
}

impl Default for DrawOption {
    fn default() -> Self {
        Self {
            position: [Pt(0.0), Pt(0.0)],
            scale: [1.0, 1.0],
            rotation: 0.0,
            opacity: 1.0,
            layer: 0,
            clip: None,
        }
    }
}

impl DrawOption {
    /// Creates a new DrawOption with position, rotation, and scale.
    pub fn new(position: [Pt; 2], rotation: f32, scale: [f32; 2], layer: i32) -> Self {
        Self {
            position,
            rotation,
            scale,
            opacity: 1.0,
            layer,
            clip: None,
        }
    }

    pub fn position(&self) -> [Pt; 2] {
        self.position
    }

    /// Sets the drawing position. Coordinates are logical Pt relative to parent or window.
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

    pub fn layer(&self) -> i32 {
        self.layer
    }

    /// Sets the rendering layer (sorting index). Higher values are drawn later.
    pub fn with_layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }

    /// Sets an optional clipping rectangle [x, y, width, height] in logical coordinates.
    pub fn with_clip(mut self, clip: Option<[Pt; 4]>) -> Self {
        self.clip = clip;
        self
    }

    pub fn get_clip(&self) -> Option<[Pt; 4]> {
        self.clip
    }

    pub(crate) fn apply_state(&self, state: &crate::DrawState) -> Self {
        let mut new_opts = *self;

        // Add current state's position to our relative position to get absolute screen position
        new_opts.position[0] += state.position[0];
        new_opts.position[1] += state.position[1];

        // Layer is usually absolute or additive? Let's make it additive for nested offsets.
        // Actually, let's just use the DrawOption layer as the base.
        // If we want nested layers, we need state.layer too.

        // Merge clip
        if let Some(state_clip_abs) = state.clip {
            new_opts.clip = if let Some(own_clip_rel) = self.clip {
                // own_clip is relative to own relative position
                // Calculate absolute coordinates for our own clip
                let own_x_abs = new_opts.position[0].as_f32() + own_clip_rel[0].as_f32();
                let own_y_abs = new_opts.position[1].as_f32() + own_clip_rel[1].as_f32();

                let x = own_x_abs.max(state_clip_abs[0].as_f32());
                let y = own_y_abs.max(state_clip_abs[1].as_f32());
                let right = (own_x_abs + own_clip_rel[2].as_f32())
                    .min(state_clip_abs[0].as_f32() + state_clip_abs[2].as_f32());
                let bottom = (own_y_abs + own_clip_rel[3].as_f32())
                    .min(state_clip_abs[1].as_f32() + state_clip_abs[3].as_f32());

                let w = (right - x).max(0.0);
                let h = (bottom - y).max(0.0);
                Some([Pt::from(x), Pt::from(y), Pt::from(w), Pt::from(h)])
            } else {
                // If we don't have our own clip, we inherit the state clip
                Some(state_clip_abs)
            };
        }

        new_opts
    }
}
