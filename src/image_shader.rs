use crate::Image;

/// Maximum number of additional sampled textures supported by custom image shaders.
pub const MAX_IMAGE_SHADER_EXTRA_TEXTURES: usize = 4;

/// Supported high-level blend modes for image shaders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageShaderBlendMode {
    #[default]
    Alpha,
    Add,
    Replace,
}

/// High-level description for registering a custom image shader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageShaderDesc {
    pub source: String,
    pub uses_extra_textures: bool,
    pub blend_mode: ImageShaderBlendMode,
}

impl ImageShaderDesc {
    /// Uses caller-provided WGSL directly.
    ///
    /// Bind groups remain engine-defined:
    /// `@group(0)` source texture + sampler,
    /// optional `@group(1)` extra textures when enabled,
    /// then user globals and engine globals.
    ///
    /// See `docs/image-shader.md` for the full binding contract.
    pub fn from_wgsl(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            uses_extra_textures: false,
            blend_mode: ImageShaderBlendMode::Alpha,
        }
    }

    /// Enables or disables the optional extra texture bind group for this shader.
    pub fn with_extra_textures(mut self, enabled: bool) -> Self {
        self.uses_extra_textures = enabled;
        self
    }

    /// Selects a high-level blend mode without exposing backend details.
    pub fn with_blend_mode(mut self, blend_mode: ImageShaderBlendMode) -> Self {
        self.blend_mode = blend_mode;
        self
    }

    pub(crate) fn uses_extra_textures(&self) -> bool {
        self.uses_extra_textures
    }
}

/// Optional per-draw bindings for custom image shaders.
#[derive(Debug, Clone, Copy)]
pub enum ImageShaderInput {
    None,
    Image(Image),
    Screen,
    History,
}

impl Default for ImageShaderInput {
    fn default() -> Self {
        Self::None
    }
}

impl PartialEq for ImageShaderInput {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::None, Self::None)
            | (Self::Screen, Self::Screen)
            | (Self::History, Self::History) => true,
            (Self::Image(lhs), Self::Image(rhs)) => lhs.id() == rhs.id(),
            _ => false,
        }
    }
}

impl Eq for ImageShaderInput {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImageShaderBindings {
    pub extra_inputs: [ImageShaderInput; MAX_IMAGE_SHADER_EXTRA_TEXTURES],
}

impl ImageShaderBindings {
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds an extra image to the requested shader slot.
    pub fn with_extra_image(mut self, slot: usize, image: Image) -> Self {
        if slot < MAX_IMAGE_SHADER_EXTRA_TEXTURES {
            self.extra_inputs[slot] = ImageShaderInput::Image(image);
        }
        self
    }

    /// Binds the current target snapshot to the requested shader slot.
    pub fn with_screen(mut self, slot: usize) -> Self {
        if slot < MAX_IMAGE_SHADER_EXTRA_TEXTURES {
            self.extra_inputs[slot] = ImageShaderInput::Screen;
        }
        self
    }

    /// Binds the previous-frame target snapshot to the requested shader slot.
    pub fn with_history(mut self, slot: usize) -> Self {
        if slot < MAX_IMAGE_SHADER_EXTRA_TEXTURES {
            self.extra_inputs[slot] = ImageShaderInput::History;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{ImageShaderBlendMode, ImageShaderDesc};

    #[test]
    fn image_shader_desc_clamps_extra_texture_count() {
        let desc = ImageShaderDesc::from_wgsl("shader").with_extra_textures(true);
        assert!(desc.uses_extra_textures);
    }

    #[test]
    fn image_shader_desc_defaults_match_full_wgsl_registration() {
        let desc = ImageShaderDesc::from_wgsl("shader");
        assert!(!desc.uses_extra_textures);
        assert_eq!(desc.blend_mode, ImageShaderBlendMode::Alpha);
    }
}
