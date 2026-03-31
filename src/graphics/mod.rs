//! Graphics module - split into focused submodules for readability.
//!
//! This module provides the core graphics functionality for rendering
//! images, text, and custom shaders.

mod core;
mod font;
mod image_ops;
mod model_raw;
mod profile;
mod render;
mod shader;
mod text_layout;

pub use core::Graphics;
pub use core::{Bone, Camera, SkinData};
pub use core::{create_rotation_from_quat, create_scale, identity};
pub use model_raw::{
    Light, SceneGlobals, create_perspective, create_rotation, create_translation, multiply,
};
