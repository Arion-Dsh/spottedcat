//! Spot - A simple 2D graphics library for drawing images.
//!
//! # Example
//! ```no_run
//! use spot::{Context, Spot, Image, DrawOptions, switch_scene};
//!
//! struct MyApp {
//!     image: Image,
//! }
//!
//! impl Spot for MyApp {
//!     fn initialize(_context: Context) -> Self {
//!         let rgba = vec![255u8; 256 * 256 * 4];
//!         let image = Image::new_from_rgba8(256, 256, &rgba).unwrap();
//!         Self { image }
//!     }
//!
//!     fn draw(&mut self, context: &mut Context) {
//!         let mut opts = DrawOptions::default();
//!         opts.position = [100.0, 100.0];
//!         opts.size = [200.0, 200.0];
//!         self.image.draw(context, opts);
//!     }
//!
//!     fn update(&self, _event: spot::Event) {}
//!     fn remove(&self) {}
//! }
//!
//! fn main() {
//!     spot::run::<MyApp>();
//! }
//!
//! // Scene switching example:
//! // switch_scene::<NewScene>();  // Switches to NewScene
//! ```

mod graphics;
mod window;
mod image_raw;
mod image;
mod texture;
mod drawable;
mod font;
mod text;
mod text_renderer;

use std::sync::{Mutex, OnceLock};
use winit::event_loop::EventLoop;

pub use image::{Bounds, Image};
pub use drawable::{DrawOptions, TextOptions};
pub use font::{load_font_from_file, load_font_from_bytes};
pub use text::Text;

use crate::drawable::DrawAble;
use crate::graphics::Graphics;


/// Drawing context for managing render commands.
///
/// The context accumulates drawing commands during a frame and is used by the
/// graphics system to render the scene.
#[derive(Debug, Clone)]
pub struct Context {
    draw_list: Vec<DrawAble>,
}

impl Context {
    /// Creates a new drawing context.
    ///
    /// This is typically done automatically by the `run` function, but can be
    /// used to create a new context manually if needed.
    pub fn new() -> Self {
        Self {
            draw_list: Vec::new(),
        }
    }

    /// Clears all drawing commands from the previous frame.
    ///
    /// This is called automatically at the start of each frame, but can be used
    /// manually if needed.
    pub fn begin_frame(&mut self) {
        self.draw_list.clear();
    }

    /// Adds a drawable item to the draw list.
    ///
    /// This is used internally by Image::draw() and other drawing methods.
    pub(crate) fn push(&mut self, drawable: DrawAble) {
        self.draw_list.push(drawable);
    }

    /// Returns the list of drawing commands accumulated so far.
    ///
    /// This is used internally by the graphics system to render the scene.
    pub(crate) fn draw_list(&self) -> &[DrawAble] {
        &self.draw_list
    }

    pub(crate) fn push_text(&mut self, text: String, options: TextOptions) {
        self.push(DrawAble::Text(text, options));
    }
}

type SceneFactory = Box<dyn FnOnce() -> Box<dyn Spot> + Send>;

static GLOBAL_GRAPHICS: OnceLock<Mutex<Graphics>> = OnceLock::new();
static SCENE_SWITCH_REQUEST: OnceLock<Mutex<Option<SceneFactory>>> = OnceLock::new();

fn set_global_graphics(graphics: Graphics) -> Result<(), Graphics> {
    GLOBAL_GRAPHICS
        .set(Mutex::new(graphics))
        .map_err(|m| m.into_inner().unwrap_or_else(|e| e.into_inner()))
}

fn global_graphics() -> &'static Mutex<Graphics> {
    GLOBAL_GRAPHICS.get().expect("global Graphics not initialized")
}

fn with_graphics<R>(f: impl FnOnce(&mut Graphics) -> R) -> R {
    let mut g = global_graphics().lock().expect("Graphics mutex poisoned");
    f(&mut g)
}

fn init_scene_switch() {
    let _ = SCENE_SWITCH_REQUEST.set(Mutex::new(None));
}

fn request_scene_switch<F>(factory: F)
where
    F: FnOnce() -> Box<dyn Spot> + Send + 'static,
{
    if let Some(request) = SCENE_SWITCH_REQUEST.get() {
        let mut guard = request.lock().expect("Scene switch mutex poisoned");
        *guard = Some(Box::new(factory));
    }
}

pub(crate) fn take_scene_switch_request() -> Option<SceneFactory> {
    SCENE_SWITCH_REQUEST
        .get()
        .and_then(|request| request.lock().ok())
        .and_then(|mut guard| guard.take())
}

/// Runs the application with the specified Spot type.
///
/// This is the main entry point for Spot applications. It creates a window,
/// initializes the graphics system, runs the event loop, and initializes your application.
///
/// # Type Parameters
/// * `T` - Your application type that implements the `Spot` trait
///
/// # Example
/// ```no_run
/// # use spot::{Context, Spot};
/// # struct MyApp;
/// # impl Spot for MyApp {
/// #     fn initialize(_: Context) -> Self { MyApp }
/// #     fn draw(&mut self, _: &mut Context) {}
/// #     fn update(&self, _: spot::Event) {}
/// #     fn remove(&self) {}
/// # }
/// spot::run::<MyApp>();
/// ```
pub fn run<T: Spot + 'static>() {
    init_scene_switch();
    let event_loop = EventLoop::new().expect("failed to create winit EventLoop");
    let mut app = window::App::new::<T>();
    event_loop.run_app(&mut app).expect("event loop error");
}

/// Switches to a new scene of the specified type.
///
/// This function requests a scene change that will take effect at the end of the current frame.
/// The old scene's `remove()` method will be called automatically, and the new scene will be
/// initialized with a fresh context.
///
/// # Type Parameters
/// * `T` - The new scene type to switch to
///
/// # Example
/// ```no_run
/// # use spot::{Spot, switch_scene};
/// # struct MenuScene;
/// # struct GameScene;
/// # impl Spot for MenuScene {
/// #     fn initialize(_: spot::Context) -> Self { MenuScene }
/// #     fn draw(&mut self, _: &mut spot::Context) {}
/// #     fn update(&self, _: spot::Event) {}
/// #     fn remove(&self) {}
/// # }
/// # impl Spot for GameScene {
/// #     fn initialize(_: spot::Context) -> Self { GameScene }
/// #     fn draw(&mut self, _: &mut spot::Context) {}
/// #     fn update(&self, _: spot::Event) {}
/// #     fn remove(&self) {}
/// # }
/// // In your scene's draw or update method:
/// // if some_condition {
/// //     switch_scene::<GameScene>();
/// // }
/// ```
pub fn switch_scene<T: Spot + 'static>() {
    request_scene_switch(|| Box::new(T::initialize(Context::new())));
}

/// Events that can be received by the application.
///
/// Currently empty, but will be extended with input events in the future.
pub enum Event {}

/// Main application trait that must be implemented by your application.
///
/// This trait defines the lifecycle of a Spot application.
pub trait Spot {
    /// Initializes the application.
    ///
    /// Called once when the application starts. Use this to load resources
    /// and set up initial state.
    ///
    /// # Arguments
    /// * `context` - Initial drawing context
    fn initialize(context: Context) -> Self
    where
        Self: Sized;

    /// Draws the current frame.
    ///
    /// Called every frame. Use the context to issue drawing commands.
    ///
    /// # Arguments
    /// * `context` - Drawing context to add render commands to
    fn draw(&mut self, context: &mut Context);
    
    /// Handles events.
    ///
    /// Called when events occur (currently unused, reserved for future input events).
    ///
    /// # Arguments
    /// * `event` - The event to handle
    fn update(&self, event: Event);
    
    /// Cleanup when the application is shutting down.
    fn remove(&self);
}
