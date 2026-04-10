//! Graphics module - split into focused submodules for readability.
//!
//! This module provides the core graphics functionality for rendering
//! images, text, and custom shaders.

pub(crate) mod core;
#[cfg(feature = "model-3d")]
pub(crate) mod core_3d;
pub(crate) mod atlas;
pub(crate) mod font;
pub(crate) mod image_ops;
#[cfg(feature = "model-3d")]
pub(crate) mod model_raw;
pub(crate) mod profile;
pub(crate) mod render;
#[cfg(feature = "model-3d")]
pub(crate) mod render_3d;
pub(crate) mod shader;
#[cfg(feature = "model-3d")]
pub(crate) mod shader_3d;
pub(crate) mod text_layout;
pub(crate) mod texture;
