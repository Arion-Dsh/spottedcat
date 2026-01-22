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
mod text_layout;

pub use core::Graphics;
