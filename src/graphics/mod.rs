//! Graphics module - split into focused submodules for readability.
//!
//! This module provides the core graphics functionality for rendering
//! images, text, and custom shaders.

mod core;
mod font;
mod image_ops;
mod profile;
mod render;
mod shader;
mod model_raw;
mod text_layout;

pub use core::Graphics;
pub use core::{Bone, SkinData};
pub use core::{identity, create_scale, create_rotation_from_quat};
pub use model_raw::{multiply, create_translation, create_rotation, create_perspective, Light, SceneGlobals};
