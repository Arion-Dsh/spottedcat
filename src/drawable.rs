use crate::Pt;
use crate::ShaderOpts;
use crate::Text;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DrawCommand {
    Image(u32, DrawOption, u32, ShaderOpts),
    Text(Box<Text>, DrawOption),
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
            clip: None,
        }
    }
}

impl DrawOption {
    pub fn new(position: [Pt; 2], rotation: f32, scale: [f32; 2]) -> Self {
        Self {
            position,
            rotation,
            scale,
            opacity: 1.0,
            clip: None,
        }
    }

    pub fn position(&self) -> [Pt; 2] {
        self.position
    }

    pub fn with_position(mut self, position: [Pt; 2]) -> Self {
        self.position = position;
        self
    }

    pub fn rotation(&self) -> f32 {
        self.rotation
    }

    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn scale(&self) -> [f32; 2] {
        self.scale
    }

    pub fn with_scale(mut self, scale: [f32; 2]) -> Self {
        self.scale = scale;
        self
    }

    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

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
