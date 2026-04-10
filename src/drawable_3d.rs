#[cfg(feature = "model-3d")]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DrawCommand3D {
    Model(
        u32,
        crate::model::Model,
        DrawOption3D,
        u32,
        crate::ShaderOpts,
        Option<u32>,
    ),
    ModelInstanced(
        u32,
        crate::model::Model,
        DrawOption3D,
        u32,
        crate::ShaderOpts,
        Option<u32>,
        std::sync::Arc<[[[f32; 4]; 4]]>,
    ),
}

/// Unified options for drawing 3D models.
#[cfg(feature = "model-3d")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawOption3D {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub opacity: f32,
}

#[cfg(feature = "model-3d")]
impl Default for DrawOption3D {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            opacity: 1.0,
        }
    }
}

#[cfg(feature = "model-3d")]
impl DrawOption3D {
    pub fn new(position: [f32; 3], rotation: [f32; 3], scale: [f32; 3]) -> Self {
        Self {
            position,
            rotation,
            scale,
            opacity: 1.0,
        }
    }

    pub fn with_position(mut self, position: [f32; 3]) -> Self {
        self.position = position;
        self
    }

    pub fn with_rotation(mut self, rotation: [f32; 3]) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_scale(mut self, scale: [f32; 3]) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }
}
