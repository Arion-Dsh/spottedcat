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
///
/// Use [`ImageShaderDesc::from_wgsl`] for full manual control, or [`ImageShaderTemplate`]
/// (via [`register_image_shader_template`][crate::register_image_shader_template])
/// for a simplified, slot-based approach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageShaderDesc {
    pub source: String,
    pub uses_extra_textures: bool,
    pub blend_mode: ImageShaderBlendMode,
    /// If enabled, the engine automatically injects standard WGSL structs (VsIn, VsOut, EngineGlobals)
    /// and declarations (t_history, t_screen, etc.) based on the semantic slot configuration.
    pub internal_prelude: bool,
    pub extra_texture_names: [Option<String>; 4],
    /// If set, the engine maps the `with_history()` semantic binding to this slot.
    pub history_slot: Option<usize>,
    /// If set, the engine maps the `with_screen()` semantic binding to this slot.
    pub screen_slot: Option<usize>,
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
            internal_prelude: false,
            extra_texture_names: Default::default(),
            history_slot: None,
            screen_slot: None,
        }
    }

    /// Automatically prepends engine-standard WGSL definitions (EngineGlobals, VsIn, VsOut, etc.)
    /// before the user-provided source. 
    /// 
    /// This allows writing full shader code without copy-pasting standard boilerplate.
    /// When using the prelude, standard variables like `screen`, `opacity`, and `scale_factor`
    /// are automatically injected into the vertex and fragment bodies.
    pub fn with_internal_prelude(mut self, enabled: bool) -> Self {
        self.internal_prelude = enabled;
        self
    }

    /// Provides a custom name for an extra texture slot when using the internal prelude.
    pub fn with_texture_alias(mut self, slot: usize, name: impl Into<String>) -> Self {
        if slot < 4 {
            self.extra_texture_names[slot] = Some(name.into());
        }
        self
    }

    /// Explicitly sets which slot should be used for the history texture semantic.
    pub fn with_history_slot(mut self, slot: usize) -> Self {
        if slot < 4 {
            self.history_slot = Some(slot);
        }
        self
    }

    /// Explicitly sets which slot should be used for the screen snapshot semantic.
    pub fn with_screen_slot(mut self, slot: usize) -> Self {
        if slot < 4 {
            self.screen_slot = Some(slot);
        }
        self
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

/// Per-draw image shader bindings.
///
/// This struct supports both **Semantic Bindings** (recommended) and **Positional Bindings** (legacy).
///
/// ### Semantic Bindings
/// Semantic bindings associate a texture with a specific *role* (like `history` or `screen`)
/// defined during shader registration. The engine handles the underlying slot mapping.
///
/// ```rust
/// use spottedcat::{Image, ImageShaderBindings};
///
/// fn build_bindings(noise_img: Image) -> ImageShaderBindings {
///     ImageShaderBindings::new()
///         .with_history() // Automatically finds the history slot
///         .with_image("t_noise", noise_img) // Automatically finds the "t_noise" slot
/// }
/// ```
///
/// ### Positional Bindings
/// Positional bindings explicitly target a specific slot (0-3).
/// Note: Semantic bindings (like `.with_history()`) take precedence and will
/// overwrite positional bindings if they share the same slot.
///
/// ```rust
/// use spottedcat::{Image, ImageShaderBindings};
///
/// fn build_bindings(noise_img: Image, mask_img: Image) -> ImageShaderBindings {
///     ImageShaderBindings::new()
///         .with_extra_image(0, noise_img)
///         .with_extra_image(1, mask_img)
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageShaderBindings {
    /// Manual index-based overrides. Legacy/Internal.
    pub(crate) extra_inputs: [ImageShaderInput; MAX_IMAGE_SHADER_EXTRA_TEXTURES],
    /// Semantic history intent.
    pub(crate) history: bool,
    /// Semantic screen intent.
    pub(crate) screen: bool,
    /// Semantic named image intents.
    pub(crate) named_inputs: std::collections::HashMap<String, ImageShaderInput>,
}

impl Default for ImageShaderBindings {
    fn default() -> Self {
        Self {
            extra_inputs: Default::default(),
            history: false,
            screen: false,
            named_inputs: Default::default(),
        }
    }
}

impl ImageShaderBindings {
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds an extra image to the requested shader slot (Manual/Legacy).
    pub fn with_extra_image(mut self, slot: usize, image: Image) -> Self {
        if slot < MAX_IMAGE_SHADER_EXTRA_TEXTURES {
            self.extra_inputs[slot] = ImageShaderInput::Image(image);
        }
        self
    }

    /// Binds the current target snapshot to the requested shader slot (Manual/Legacy).
    pub fn with_screen_at_slot(mut self, slot: usize) -> Self {
        if slot < MAX_IMAGE_SHADER_EXTRA_TEXTURES {
            self.extra_inputs[slot] = ImageShaderInput::Screen;
        }
        self
    }

    /// Binds the previous-frame target snapshot to the requested shader slot (Manual/Legacy).
    pub fn with_history_at_slot(mut self, slot: usize) -> Self {
        if slot < MAX_IMAGE_SHADER_EXTRA_TEXTURES {
            self.extra_inputs[slot] = ImageShaderInput::History;
        }
        self
    }

    /// SEMANTIC: Binds the current target snapshot to whichever slot the shader expects.
    pub fn with_screen(mut self) -> Self {
        self.screen = true;
        self
    }

    /// SEMANTIC: Binds the previous-frame target snapshot to whichever slot the shader expects.
    pub fn with_history(mut self) -> Self {
        self.history = true;
        self
    }

    /// SEMANTIC: Binds an image to a named slot in the shader.
    pub fn with_image(mut self, name: impl Into<String>, image: Image) -> Self {
        self.named_inputs.insert(name.into(), ImageShaderInput::Image(image));
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
        assert!(!desc.internal_prelude);
        assert_eq!(desc.blend_mode, ImageShaderBlendMode::Alpha);
    }
}
