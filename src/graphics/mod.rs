//! Graphics module - split into focused submodules for readability.
//!
//! This module provides the core graphics functionality for rendering
//! images, text, and custom shaders.

mod core;
#[cfg(feature = "model-3d")]
mod core_3d;
mod font;
mod image_ops;
#[cfg(feature = "model-3d")]
mod model_raw;
mod profile;
mod render;
#[cfg(feature = "model-3d")]
mod render_3d;
mod shader;
#[cfg(feature = "model-3d")]
mod shader_3d;
mod text_layout;
