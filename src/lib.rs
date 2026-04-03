//! Spot - A simple 2D graphics library for drawing images.
//!
//! # Example
//! ```no_run
//! use spottedcat::{Context, DrawOption, Image, Spot, switch_scene};
//!
//! struct MyApp {
//!     image: Image,
//! }
//!
//! impl Spot for MyApp {
//!     fn initialize(_ctx: &mut Context) -> Self {
//!         let rgba = vec![255u8; 256 * 256 * 4];
//!         let image = spottedcat::image::create(_ctx, 256u32.into(), 256u32.into(), &rgba).unwrap();
//!         Self { image }
//!     }
//!
//!     fn draw(&mut self, ctx: &mut Context) {
//!         let opts = DrawOption::default()
//!             .with_position([spottedcat::Pt::from(100.0), spottedcat::Pt::from(100.0)])
//!             .with_scale([0.78125, 0.78125]);
//!         spottedcat::image::draw(ctx, self.image, opts);
//!     }
//!
//!     fn update(&mut self, _ctx: &mut Context, _dt: std::time::Duration) {}
//!     fn remove(&mut self, _ctx: &mut Context) {}
//! }
//!
//! fn main() {
//!     spottedcat::run::<MyApp>(spottedcat::WindowConfig::default());
//! }
//!
//! // Scene switching example:
//! // switch_scene::<NewScene>();  // Switches to NewScene
//! ```

#[cfg(target_os = "android")]
pub mod android;
mod assets;
mod audio;
mod context;
mod context_3d;
mod controls;
mod drawable;
mod drawable_3d;
#[cfg(feature = "effects")]
mod fog;
mod glyph_cache;
pub mod graphics;
pub mod image;
mod image_raw;
mod input;
mod key;
mod launch;
#[cfg(feature = "model-3d")]
pub mod model;
mod mouse;
mod packer;
mod platform;
mod platform_events;
mod pt;
mod scenes;
mod shader_opts;
mod sound;
pub mod text;
mod texture;
mod touch;
#[cfg(any(feature = "utils", feature = "model-3d", feature = "gltf"))]
pub mod utils;
mod window;

#[cfg(target_os = "android")]
pub use android_activity::AndroidApp;
pub use assets::*;
pub use context::Context;
pub(crate) use context::DrawState;
pub use controls::*;
pub use drawable::DrawOption;
#[cfg(feature = "model-3d")]
pub use drawable_3d::DrawOption3D;
#[cfg(feature = "effects")]
pub use fog::{FogBackgroundSettings, FogSamplingSettings, FogSettings};
pub use graphics::core::Graphics;
pub use image::{Bounds, Image};
pub use input::InputManager;
pub use key::Key;
pub use launch::{WindowConfig, run};
#[cfg(feature = "model-3d")]
pub use model::Model;
pub use mouse::MouseButton;
pub use platform_events::PlatformEvent;
pub use pt::Pt;
pub use scenes::{Spot, quit, switch_scene, switch_scene_with};
pub use shader_opts::ShaderOpts;
pub use sound::*;
pub use text::Text;
pub use touch::{TouchInfo, TouchPhase};

// --- Functional API ---

/// Registers a TTF/OTF font for text rendering.
pub fn register_font(ctx: &mut Context, font_data: Vec<u8>) -> u32 {
    ctx.register_font(font_data)
}

/// Registers a custom WGSL shader for image rendering.
pub fn register_shader(ctx: &mut Context, user_functions: &str) -> u32 {
    ctx.register_image_shader(user_functions)
}

/// Creates a point value from a scalar.
pub fn pt(x: f32) -> Pt {
    Pt::from(x)
}

#[cfg(feature = "model-3d")]
/// Sets camera eye, target and up vectors in one call.
pub fn set_camera(ctx: &mut Context, eye: [f32; 3], target: [f32; 3], up: [f32; 3]) {
    ctx.set_camera(eye, target, up);
}

#[cfg(feature = "model-3d")]
/// Returns current camera eye position.
pub fn camera_position(ctx: &Context) -> [f32; 3] {
    ctx.camera_position()
}

#[cfg(feature = "model-3d")]
/// Sets ambient light color.
pub fn set_ambient(ctx: &mut Context, color: [f32; 4]) {
    ctx.set_ambient(color);
}

#[cfg(feature = "model-3d")]
/// Sets a PBR light (up to 4 lights).
pub fn set_light(ctx: &mut Context, index: usize, position: [f32; 4], color: [f32; 4]) {
    ctx.set_light(index, position, color);
}

#[cfg(all(feature = "model-3d", feature = "effects"))]
/// Sets global fog settings.
pub fn set_fog(ctx: &mut Context, settings: FogSettings) {
    ctx.set_fog(settings);
}

/// Sets the window's logical size.
pub fn set_window_size(ctx: &mut Context, width: Pt, height: Pt) {
    ctx.set_window_logical_size(width, height);
}

/// Returns the window's logical size.
pub fn window_size(ctx: &Context) -> (Pt, Pt) {
    ctx.window_logical_size()
}

/// Returns the window's scale factor (DPI).
pub fn scale_factor(ctx: &Context) -> f64 {
    ctx.scale_factor()
}

/// Returns a percentage of the window width as Pt.
pub fn vw(ctx: &Context, percent: f32) -> Pt {
    ctx.vw(percent)
}

/// Returns a percentage of the window height as Pt.
pub fn vh(ctx: &Context, percent: f32) -> Pt {
    ctx.vh(percent)
}

/// Returns true if the specified key is currently held down.
pub fn key_down(ctx: &Context, key: Key) -> bool {
    ctx.input().key_down(key)
}

/// Returns true if the specified key was just pressed this frame.
pub fn key_pressed(ctx: &Context, key: Key) -> bool {
    ctx.input().key_pressed(key)
}

/// Returns true if the specified mouse button is currently held down.
pub fn mouse_down(ctx: &Context, btn: MouseButton) -> bool {
    ctx.input().mouse_down(btn)
}

/// Returns true if the specified mouse button was just pressed this frame.
pub fn mouse_pressed(ctx: &Context, btn: MouseButton) -> bool {
    ctx.input().mouse_pressed(btn)
}

/// Returns the current mouse position in logical coordinates.
pub fn mouse_pos(ctx: &Context) -> Option<(Pt, Pt)> {
    ctx.input().cursor_position()
}

/// Requests a window title update.
pub fn set_window_title(ctx: &mut Context, title: impl Into<String>) {
    ctx.set_window_title(title);
}

/// Requests cursor visibility update.
pub fn set_cursor_visible(ctx: &mut Context, visible: bool) {
    ctx.set_cursor_visible(visible);
}

/// Requests fullscreen toggle.
pub fn set_fullscreen(ctx: &mut Context, enabled: bool) {
    ctx.set_fullscreen(enabled);
}

/// Scene switch helper that keeps the ctx-first API shape.
pub fn switch_scene_ctx<T: Spot + 'static>(_ctx: &mut Context) {
    switch_scene::<T>();
}

/// Scene switch with payload helper that keeps the ctx-first API shape.
pub fn switch_scene_with_ctx<T: Spot + 'static, P: std::any::Any>(_ctx: &mut Context, payload: P) {
    switch_scene_with::<T, P>(payload);
}

/// Quit helper that keeps the ctx-first API shape.
pub fn quit_ctx(_ctx: &mut Context) {
    quit();
}

// --- Utilities & Time ---

/// Returns the time elapsed since the last frame.
pub fn delta_time(ctx: &Context) -> std::time::Duration {
    ctx.delta_time()
}

/// Returns the time elapsed since the last frame in seconds.
pub fn dt(ctx: &Context) -> f32 {
    ctx.delta_time().as_secs_f32()
}

/// Returns total elapsed time since engine start.
pub fn total_elapsed(ctx: &Context) -> std::time::Duration {
    ctx.total_elapsed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawable::{DrawCommand, ImageCommand};

    #[test]
    fn test_image_culling_flip() {
        let mut ctx = Context::new();
        ctx.set_window_logical_size(Pt::from(800.0), Pt::from(600.0));

        let img_id = 1u32;
        let img_size = [Pt::from(100.0), Pt::from(100.0)];

        let opts = DrawOption::default().with_position([Pt::from(100.0), Pt::from(100.0)]);
        ctx.push(DrawCommand::Image(Box::new(ImageCommand {
            id: img_id,
            opts,
            shader_id: 0,
            shader_opts: ShaderOpts::default(),
            size: img_size,
        })));
        assert_eq!(
            ctx.runtime.draw_list.len(),
            1,
            "Normal image should be visible"
        );
        ctx.runtime.draw_list.clear();

        let opts = DrawOption::default()
            .with_position([Pt::from(100.0), Pt::from(100.0)])
            .with_scale([-1.0, 1.0]);
        ctx.push(DrawCommand::Image(Box::new(ImageCommand {
            id: img_id,
            opts,
            shader_id: 0,
            shader_opts: ShaderOpts::default(),
            size: img_size,
        })));
        assert_eq!(
            ctx.runtime.draw_list.len(),
            1,
            "Flipped H image at 100 should be visible (covers 0-100)"
        );
        ctx.runtime.draw_list.clear();

        let opts = DrawOption::default()
            .with_position([Pt::from(-0.1), Pt::from(100.0)])
            .with_scale([-1.0, 1.0]);
        ctx.push(DrawCommand::Image(Box::new(ImageCommand {
            id: img_id,
            opts,
            shader_id: 0,
            shader_opts: ShaderOpts::default(),
            size: img_size,
        })));
        assert_eq!(
            ctx.runtime.draw_list.len(),
            0,
            "Flipped H image at -0.1 should be culled (covers -100 to -0.1)"
        );
        ctx.runtime.draw_list.clear();

        let opts = DrawOption::default()
            .with_position([Pt::from(100.0), Pt::from(100.0)])
            .with_scale([1.0, -1.0]);
        ctx.push(DrawCommand::Image(Box::new(ImageCommand {
            id: img_id,
            opts,
            shader_id: 0,
            shader_opts: ShaderOpts::default(),
            size: img_size,
        })));
        assert_eq!(
            ctx.runtime.draw_list.len(),
            1,
            "Flipped V image at 100 should be visible (covers 0-100 in Y)"
        );
        ctx.runtime.draw_list.clear();

        let opts = DrawOption::default()
            .with_position([Pt::from(100.0), Pt::from(100.0)])
            .with_scale([-1.0, -1.0]);
        ctx.push(DrawCommand::Image(Box::new(ImageCommand {
            id: img_id,
            opts,
            shader_id: 0,
            shader_opts: ShaderOpts::default(),
            size: img_size,
        })));
        assert_eq!(
            ctx.runtime.draw_list.len(),
            1,
            "Both-flipped image at 100,100 should be visible"
        );
        ctx.runtime.draw_list.clear();
    }
}
