//! # spottedcat
//!
//! Spottedcat is a lightweight cross-platform 2D/3D game engine built with Rust and wgpu.
//! It provides a simple API for rendering, input, audio, text, and scene management across desktop, web, iOS, and Android.
//! Designed for fast prototyping and creative interactive projects, it aims to stay small, practical, and easy to use.
//!
//! ## Basic Example
//!
//! ```rust,no_run
//! use spottedcat::{Context, Spot, Image, DrawOption, Pt, WindowConfig};
//! use std::time::Duration;
//!
//! struct MyApp {
//!     image: Image,
//! }
//!
//! impl Spot for MyApp {
//!     fn initialize(ctx: &mut Context) -> Self {
//!         // Create an image from raw RGBA8 data
//!         let rgba = vec![255u8; 64 * 64 * 4]; // Red square
//!         let image = Image::new(ctx, Pt::from(64.0), Pt::from(64.0), &rgba)
//!             .expect("Failed to create image");
//!         Self { image }
//!     }
//!
//!     fn update(&mut self, _ctx: &mut Context, _dt: Duration) {
//!         // Handle logic here
//!     }
//!
//!     fn draw(&mut self, ctx: &mut Context, screen: Image) {
//!         let (w, h) = spottedcat::window_size(ctx);
//!         
//!         // Draw image at center
//!         let opts = DrawOption::default()
//!             .with_position([w / 2.0, h / 2.0])
//!             .with_scale([2.0, 2.0]);
//!             
//!         screen.draw(ctx, &self.image, opts);
//!     }
//!
//!     fn remove(&mut self, _ctx: &mut Context) {}
//! }
//!
//! fn main() {
//!     spottedcat::run::<MyApp>(WindowConfig {
//!         title: "SpottedCat Example".to_string(),
//!         ..Default::default()
//!     });
//! }
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
mod graphics;
pub mod image;
mod image_raw;
mod input;
mod key;
mod launch;
pub mod math;
#[cfg(feature = "model-3d")]
pub mod model;
mod mouse;
mod platform;
mod platform_events;
mod pt;
mod scenes;
mod shader_opts;
mod sound;
mod splash;
pub mod text;

mod touch;
#[cfg(any(feature = "utils", feature = "model-3d", feature = "gltf"))]
pub mod utils;
mod window;

#[cfg(target_os = "android")]
pub use android_activity::AndroidApp;
pub use assets::*;
pub use context::Context;
pub use controls::*;
pub use drawable::{DrawOption, Drawable};
#[cfg(feature = "model-3d")]
pub use drawable_3d::DrawOption3D;
#[cfg(feature = "effects")]
pub use fog::{FogBackgroundSettings, FogSamplingSettings, FogSettings};

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
pub use splash::OneShotSplash;
pub use text::Text;
pub use graphics::texture::Texture;
pub use touch::{TouchInfo, TouchPhase};

// --- Functional API ---

/// Registers a TTF/OTF font for text rendering and returns a unique font ID.
pub fn register_font(ctx: &mut Context, font_data: Vec<u8>) -> u32 {
    ctx.register_font(font_data)
}

/// Registers a custom WGSL fragment shader extension for image rendering.
///
/// The shader should define a `user_fs_hook()` function to modify the output color.
pub fn register_shader(ctx: &mut Context, user_functions: &str) -> u32 {
    ctx.register_image_shader(user_functions)
}

/// Creates a logical point value ([`Pt`][crate::Pt]) from a scalar.
pub fn pt(x: f32) -> Pt {
    Pt::from(x)
}

pub fn unregister_font(ctx: &mut Context, font_id: u32) {
    assets::unregister_font(ctx, font_id);
}

/// Registers a sound from raw bytes and returns a unique sound ID.
pub fn register_sound(ctx: &mut Context, bytes: Vec<u8>) -> Option<u32> {
    sound::register_sound(ctx, bytes)
}

/// Unregisters a sound and frees its resources.
pub fn unregister_sound(ctx: &mut Context, sound_id: u32) {
    sound::unregister_sound(ctx, sound_id)
}

/// Forces pending asset compression work to run immediately.


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
/// Sets camera eye position.
pub fn set_camera_pos(ctx: &mut Context, pos: [f32; 3]) {
    ctx.set_camera_pos(pos);
}

#[cfg(feature = "model-3d")]
/// Sets camera target vector.
pub fn set_camera_target(ctx: &mut Context, x: f32, y: f32, z: f32) {
    ctx.set_camera_target(x, y, z);
}

#[cfg(feature = "model-3d")]
/// Sets camera up vector.
pub fn set_camera_up(ctx: &mut Context, x: f32, y: f32, z: f32) {
    ctx.set_camera_up(x, y, z);
}

#[cfg(feature = "model-3d")]
/// Sets camera vertical field of view in degrees.
pub fn set_camera_fovy(ctx: &mut Context, fovy_degrees: f32) {
    ctx.set_camera_fovy(fovy_degrees);
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

#[cfg(all(feature = "model-3d", feature = "effects"))]
/// Resets global fog to the default disabled state.
pub fn clear_fog(ctx: &mut Context) {
    ctx.clear_fog();
}

#[cfg(feature = "model-3d")]
/// Sets the ambient light color for the active 3D scene.
pub fn set_ambient_light(ctx: &mut Context, color: [f32; 4]) {
    ctx.set_ambient_light(color);
}

/// Sets the window's logical size.
pub fn set_window_size(ctx: &mut Context, width: Pt, height: Pt) {
    ctx.set_window_logical_size(width, height);
}

/// Returns the window's logical size as a tuple of `(width, height)`.
pub fn window_size(ctx: &Context) -> (Pt, Pt) {
    ctx.window_logical_size()
}

/// Inserts or replaces a resource of type T in the context.
pub fn insert_resource<T: std::any::Any>(ctx: &mut Context, value: std::rc::Rc<T>) {
    ctx.insert_resource(value)
}

/// Returns a resource of type T from the context, if it exists.
pub fn get_resource<T: std::any::Any>(ctx: &Context) -> Option<std::rc::Rc<T>> {
    ctx.get_resource::<T>()
}

/// Removes and returns a resource of type T from the context, if it exists.
pub fn take_resource<T: std::any::Any>(ctx: &mut Context) -> Option<std::rc::Rc<T>> {
    ctx.take_resource::<T>()
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

/// Alias for [`mouse_pos`][crate::mouse_pos].
pub fn cursor_position(ctx: &Context) -> Option<(Pt, Pt)> {
    mouse_pos(ctx)
}

/// Returns a slice of active touch points.
pub fn touches(ctx: &Context) -> &[TouchInfo] {
    ctx.input().touches()
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
///
/// This is a convenience for `delta_time(ctx).as_secs_f32()`.
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
            target_texture_id: 0,
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
            target_texture_id: 0,
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
            target_texture_id: 0,
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
            target_texture_id: 0,
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
            target_texture_id: 0,
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

    #[test]
    fn test_render_target_registration() {
        let mut ctx = Context::new();
        let texture = Texture::new_render_target(&mut ctx, Pt::from(100.0), Pt::from(200.0));
        let image = texture.view();

        assert_eq!(texture.width(), Pt::from(100.0));
        assert_eq!(texture.height(), Pt::from(200.0));
        assert_eq!(image.width(), Pt::from(100.0));
        assert_eq!(image.height(), Pt::from(200.0));

        let texture_entry = ctx
            .registry
            .textures
            .get(texture.id() as usize)
            .unwrap()
            .as_ref()
            .unwrap();
        assert!(texture_entry.is_render_target());
        assert_eq!(texture_entry.pixel_width, 100);
        assert_eq!(texture_entry.pixel_height, 200);
    }
}
