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
//!         let image = Image::new_from_rgba8(_ctx, 256u32.into(), 256u32.into(), &rgba).unwrap();
//!         Self { image }
//!     }
//!
//!     fn draw(&mut self, ctx: &mut Context) {
//!         let opts = DrawOption::default()
//!             .with_position([spottedcat::Pt::from(100.0), spottedcat::Pt::from(100.0)])
//!             .with_scale([0.78125, 0.78125]);
//!         self.image.draw(ctx, opts);
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
mod fog;
mod glyph_cache;
pub mod graphics;
mod image;
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
mod text;
mod texture;
mod touch;
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
pub use text::Text;
pub use touch::{TouchInfo, TouchPhase};

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
