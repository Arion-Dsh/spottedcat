//! Spot - A simple 2D graphics library for drawing images.
//!
//! # Example
//! ```no_run
//! use spottedcat::{Context, Spot, Image, DrawOption, switch_scene};
//!
//! struct MyApp {
//!     image: Image,
//! }
//!
//! impl Spot for MyApp {
//!     fn initialize(_context: &mut Context) -> Self {
//!         let rgba = vec![255u8; 256 * 256 * 4];
//!         let image = Image::new_from_rgba8(256u32.into(), 256u32.into(), &rgba).unwrap();
//!         Self { image }
//!     }
//!
//!     fn draw(&mut self, context: &mut Context) {
//!         let opts = DrawOption::default()
//!             .with_position([spottedcat::Pt::from(100.0), spottedcat::Pt::from(100.0)])
//!             .with_scale([0.78125, 0.78125]);
//!         self.image.draw(context, opts);
//!     }
//!
//!     fn update(&mut self, _context: &mut Context, _dt: std::time::Duration) {}
//!     fn remove(&self) {}
//! }
//!
//! fn main() {
//!     spottedcat::run::<MyApp>(spottedcat::WindowConfig::default());
//! }
//!
//! // Scene switching example:
//! // switch_scene::<NewScene>();  // Switches to NewScene
//! ```

mod audio;
mod drawable;
mod glyph_cache;
mod graphics;
mod image;
mod image_raw;
mod input;
mod key;
mod mouse;
mod packer;
mod platform;
mod pt;
mod shader_opts;
mod text;
mod texture;
mod touch;
mod window;

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;
#[cfg(not(target_os = "android"))]
use winit::event_loop::EventLoop;
#[cfg(target_os = "android")]
use winit::event_loop::EventLoop;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use console_error_panic_hook;

use drawable::DrawCommand;
pub use drawable::DrawOption;
pub use image::{Bounds, Image};
pub use input::InputManager;
pub use key::Key;
pub use mouse::MouseButton;
pub use pt::Pt;
pub use shader_opts::ShaderOpts;
pub use text::Text;
pub use touch::{TouchInfo, TouchPhase};

#[derive(Debug, Clone)]
pub struct SoundOptions {
    pub volume: f32,
    pub fade_in: Duration,
    pub fade_out: Option<Duration>,
    pub start_paused: bool,
}

impl Default for SoundOptions {
    fn default() -> Self {
        Self {
            volume: 1.0,
            fade_in: Duration::ZERO,
            fade_out: None,
            start_paused: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: Pt,
    pub height: Pt,
    pub resizable: bool,
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    pub canvas_id: Option<String>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "spot".to_string(),
            width: Pt(800.0),
            height: Pt(600.0),
            resizable: true,
            #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
            canvas_id: None,
        }
    }
}

use crate::graphics::Graphics;

#[derive(Debug, Clone, Copy)]
pub(crate) struct DrawState {
    pub position: [Pt; 2],
    pub clip: Option<[Pt; 4]>,
    pub shader_id: Option<u32>,
    pub shader_opts: Option<ShaderOpts>,
}

impl Default for DrawState {
    fn default() -> Self {
        Self {
            position: [Pt(0.0), Pt(0.0)],
            clip: None,
            shader_id: None,
            shader_opts: None,
        }
    }
}

/// Drawing context for managing render commands.
///
/// The context accumulates drawing commands during a frame and is used by the
/// graphics system to render the scene.
#[derive(Debug)]
pub struct Context {
    draw_list: Vec<DrawCommand>,
    input: InputManager,
    scale_factor: f64,
    window_logical_size: (Pt, Pt),
    resources: ResourceMap,
    audio: audio::AudioSystem,
    state_stack: Vec<DrawState>,
    current_state: DrawState,
    last_image_opts: HashMap<u32, LastImageDrawInfo>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LastImageDrawInfo {
    pub(crate) opts: DrawOption,
}

#[derive(Default)]
struct ResourceMap {
    inner: HashMap<TypeId, Rc<dyn Any>>,
}

impl std::fmt::Debug for ResourceMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceMap")
            .field("len", &self.inner.len())
            .finish()
    }
}

impl Context {
    /// Creates a new drawing context.
    ///
    /// This is typically done automatically by the `run` function, but can be
    /// used to create a new context manually if needed.
    pub fn new() -> Self {
        Self {
            draw_list: Vec::new(),
            input: InputManager::new(),
            scale_factor: 1.0,
            window_logical_size: (Pt(0.0), Pt(0.0)),
            resources: ResourceMap::default(),
            audio: platform::with_audio(|a| a.clone()),
            state_stack: Vec::new(),
            current_state: DrawState::default(),
            last_image_opts: HashMap::new(),
        }
    }

    pub fn set_window_logical_size(&mut self, width: Pt, height: Pt) {
        let w = Pt(width.0.max(0.0));
        let h = Pt(height.0.max(0.0));
        self.window_logical_size = (w, h);
    }

    pub fn window_logical_size(&self) -> (Pt, Pt) {
        self.window_logical_size
    }

    pub fn vw(&self, percent: f32) -> Pt {
        let (w, _) = self.window_logical_size;
        let p = if percent.is_finite() { percent } else { 0.0 };
        Pt::from((w.as_f32() * (p / 100.0)) as f32)
    }

    pub fn vh(&self, percent: f32) -> Pt {
        let (_, h) = self.window_logical_size;
        let p = if percent.is_finite() { percent } else { 0.0 };
        Pt::from((h.as_f32() * (p / 100.0)) as f32)
    }

    pub fn insert_resource<T: Any>(&mut self, value: Rc<T>) {
        self.resources
            .inner
            .insert(TypeId::of::<T>(), value as Rc<dyn Any>);
    }

    pub fn get_resource<T: Any>(&self) -> Option<Rc<T>> {
        self.resources
            .inner
            .get(&TypeId::of::<T>())
            .cloned()
            .and_then(|v| Rc::downcast::<T>(v).ok())
    }

    pub fn remove_resource<T: Any>(&mut self) -> Option<Rc<T>> {
        self.resources
            .inner
            .remove(&TypeId::of::<T>())
            .and_then(|v| Rc::downcast::<T>(v).ok())
    }

    pub(crate) fn insert_resource_dyn(&mut self, type_id: TypeId, value: Rc<dyn Any>) {
        self.resources.inner.insert(type_id, value);
    }

    pub(crate) fn remove_resource_dyn(&mut self, type_id: TypeId) {
        self.resources.inner.remove(&type_id);
    }

    /// Clears all drawing commands from the previous frame.
    ///
    /// This is called automatically at the start of each frame, but can be used
    /// manually if needed.
    pub(crate) fn begin_frame(&mut self) {
        self.draw_list.clear();
        self.state_stack.clear();
        self.current_state = DrawState::default();
        self.last_image_opts.clear();
    }

    pub(crate) fn input(&self) -> &InputManager {
        &self.input
    }

    pub(crate) fn input_mut(&mut self) -> &mut InputManager {
        &mut self.input
    }

    pub(crate) fn set_scale_factor(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
    }

    pub(crate) fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// Adds a drawable item to the draw list.
    ///
    /// This is used internally by Image::draw() and other drawing methods.
    pub(crate) fn push(&mut self, mut drawable: DrawCommand) {
        // Apply current state to the drawable
        match &mut drawable {
            DrawCommand::Image(_id, opts, shader_id, shader_opts, _) => {
                *opts = opts.apply_state(&self.current_state);
                // Inherit shader if not explicitly set
                if *shader_id == 0 {
                    if let Some(parent_shader_id) = self.current_state.shader_id {
                        // eprintln!("Inheriting shader {} from state", parent_shader_id);
                        *shader_id = parent_shader_id;
                        if let Some(parent_shader_opts) = self.current_state.shader_opts {
                            *shader_opts = parent_shader_opts;
                        }
                    }
                }
            }
            DrawCommand::Text(_, opts) => {
                *opts = opts.apply_state(&self.current_state);
            }
        }
        if let DrawCommand::Image(id, opts, _, _, size) = &drawable {
            // Culling check
            let pos = opts.position();
            let scale = opts.scale();
            let rot = opts.rotation();
            let w = size[0].as_f32() * scale[0];
            let h = size[1].as_f32() * scale[1];

            let (vw, vh) = self.window_logical_size;
            let screen_w = vw.as_f32();
            let screen_h = vh.as_f32();

            let is_visible = if rot == 0.0 {
                !(pos[0].as_f32() + w < 0.0
                    || pos[0].as_f32() > screen_w
                    || pos[1].as_f32() + h < 0.0
                    || pos[1].as_f32() > screen_h)
            } else {
                let c = rot.cos();
                let s = rot.sin();
                let x2 = w * c;
                let y2 = w * s;
                let x3 = -h * s;
                let y3 = h * c;
                let x4 = x2 + x3;
                let y4 = y2 + y3;

                let min_x = 0.0f32.min(x2).min(x3).min(x4);
                let max_x = 0.0f32.max(x2).max(x3).max(x4);
                let min_y = 0.0f32.min(y2).min(y3).min(y4);
                let max_y = 0.0f32.max(y2).max(y3).max(y4);

                !(pos[0].as_f32() + max_x < 0.0
                    || pos[0].as_f32() + min_x > screen_w
                    || pos[1].as_f32() + max_y < 0.0
                    || pos[1].as_f32() + min_y > screen_h)
            };

            if !is_visible {
                return;
            }

            self.last_image_opts
                .insert(*id, LastImageDrawInfo { opts: *opts });
        }
        if std::env::var("SPOT_DEBUG_DRAW").is_ok() {
            match &drawable {
                DrawCommand::Image(id, opts, shader_id, _shader_opts, _) => {
                    eprintln!(
                        "[spot][debug] draw image id={} shader_id={} pos={:?} clip={:?}",
                        id,
                        shader_id,
                        opts.position(),
                        opts.get_clip()
                    );
                }
                DrawCommand::Text(_text, opts) => {
                    eprintln!(
                        "[spot][debug] draw text pos={:?} clip={:?}",
                        opts.position(),
                        opts.get_clip()
                    );
                }
            }
        }
        self.draw_list.push(drawable);
    }

    fn current_draw_state(&self) -> DrawState {
        self.current_state
    }

    pub(crate) fn last_image_draw_info(&self, image_id: u32) -> Option<LastImageDrawInfo> {
        self.last_image_opts.get(&image_id).copied()
    }

    fn push_state(&mut self, state: DrawState) {
        self.state_stack.push(self.current_state);

        // Accumulate position correctly:
        // state.position passed from draw_image is the LOCAL relative position of the parent.
        // We add it to the current absolute position to get the new origin for children.
        self.current_state.position[0] += state.position[0];
        self.current_state.position[1] += state.position[1];

        // Merge clip: clip in state is already absolute screen-space bounds
        if let Some(new_clip_abs) = state.clip {
            let merged_clip = if let Some(old_clip_abs) = self.current_state.clip {
                // Intersect absolute clips
                let x = old_clip_abs[0].as_f32().max(new_clip_abs[0].as_f32());
                let y = old_clip_abs[1].as_f32().max(new_clip_abs[1].as_f32());
                let right = (old_clip_abs[0].as_f32() + old_clip_abs[2].as_f32())
                    .min(new_clip_abs[0].as_f32() + new_clip_abs[2].as_f32());
                let bottom = (old_clip_abs[1].as_f32() + old_clip_abs[3].as_f32())
                    .min(new_clip_abs[1].as_f32() + new_clip_abs[3].as_f32());

                let w = (right - x).max(0.0);
                let h = (bottom - y).max(0.0);
                Some([Pt::from(x), Pt::from(y), Pt::from(w), Pt::from(h)])
            } else {
                Some(new_clip_abs)
            };
            self.current_state.clip = merged_clip;
        }

        // Apply shader state if provided in the new state
        if let Some(sid) = state.shader_id {
            self.current_state.shader_id = Some(sid);
        }
        if let Some(sopts) = state.shader_opts {
            self.current_state.shader_opts = Some(sopts);
        }
    }

    fn pop_state(&mut self) {
        if let Some(prev_state) = self.state_stack.pop() {
            self.current_state = prev_state;
        }
    }

    /// Returns the list of drawing commands accumulated so far.
    ///
    /// This is used internally by the graphics system to render the scene.
    pub(crate) fn draw_list(&self) -> &[DrawCommand] {
        &self.draw_list
    }

    pub(crate) fn audio(&self) -> &audio::AudioSystem {
        &self.audio
    }
}

pub fn key_down(context: &Context, key: Key) -> bool {
    context.input().key_down(key)
}

pub fn key_pressed(context: &Context, key: Key) -> bool {
    context.input().key_pressed(key)
}

pub fn key_released(context: &Context, key: Key) -> bool {
    context.input().key_released(key)
}

pub fn mouse_button_down(context: &Context, button: MouseButton) -> bool {
    context.input().mouse_down(button)
}

pub fn mouse_button_pressed(context: &Context, button: MouseButton) -> bool {
    context.input().mouse_pressed(button)
}

pub fn mouse_button_released(context: &Context, button: MouseButton) -> bool {
    context.input().mouse_released(button)
}

pub fn mouse_button_pressed_position(context: &Context, button: MouseButton) -> Option<(Pt, Pt)> {
    if mouse_button_pressed(context, button) {
        cursor_position(context)
    } else {
        None
    }
}

pub fn window_size(context: &Context) -> (Pt, Pt) {
    context.window_logical_size()
}

pub fn cursor_position(context: &Context) -> Option<(Pt, Pt)> {
    context.input().cursor_position()
}

pub fn text_input_enabled(context: &Context) -> bool {
    context.input().text_input_enabled()
}

pub fn set_text_input_enabled(context: &mut Context, enabled: bool) {
    context.input_mut().set_text_input_enabled(enabled);
}

pub fn text_input(context: &Context) -> &str {
    context.input().text_input()
}

pub fn get_input(context: &Context) -> &str {
    context.input().text_input()
}

pub fn touches(context: &Context) -> &[TouchInfo] {
    context.input().touches()
}

pub fn touch_down(context: &Context) -> bool {
    !context.input().touches().is_empty()
}

pub fn ime_preedit(context: &Context) -> Option<&str> {
    context.input().ime_preedit()
}

pub fn register_image_shader(wgsl_source: &str) -> u32 {
    with_graphics(|g| g.register_image_shader(wgsl_source))
}

pub fn register_font(font_data: Vec<u8>) -> u32 {
    with_graphics(|g| g.register_font(font_data))
}

pub fn get_registered_font(font_id: u32) -> Option<Vec<u8>> {
    with_graphics(|g| g.get_font(font_id).cloned())
}

pub fn register_sound(bytes: Vec<u8>) -> anyhow::Result<u32> {
    let sound_data = audio::decode_sound_from_bytes(bytes)?;
    Ok(platform::with_audio(|a| a.register_sound(sound_data)))
}

pub fn play_sound(context: &Context, sound_id: u32, options: SoundOptions) -> Option<u64> {
    let opts = audio::PlayOptions {
        volume: options.volume,
        fade_in: options.fade_in,
        fade_out: options.fade_out,
        start_paused: options.start_paused,
    };
    context
        .audio()
        .play_registered_sound_with_options(sound_id, opts)
}

pub fn play_sound_simple(context: &Context, sound_id: u32) -> Option<u64> {
    context
        .audio()
        .play_registered_sound_with_options(sound_id, audio::PlayOptions::default())
}

pub fn pause_sound(context: &Context, play_id: u64) {
    context.audio().pause_play_id(play_id);
}

pub fn resume_sound(context: &Context, play_id: u64) {
    context.audio().resume_play_id(play_id);
}

pub fn stop_sound(context: &Context, play_id: u64) {
    context.audio().stop_play_id(play_id);
}

pub fn fade_in_sound(context: &Context, play_id: u64, duration: Duration) {
    context.audio().fade_in_play_id(play_id, duration);
}

pub fn fade_out_sound(context: &Context, play_id: u64, duration: Duration) {
    context.audio().fade_out_play_id(play_id, duration);
}

pub fn set_sound_volume(context: &Context, play_id: u64, volume: f32) {
    context.audio().set_volume_play_id(play_id, volume);
}

pub fn is_sound_playing(context: &Context, play_id: u64) -> bool {
    context.audio().is_playing_play_id(play_id)
}

pub fn play_sine(context: &Context, freq: f32, volume: f32) -> Option<u64> {
    context.audio().play_sine(freq, volume)
}

type SceneFactory = Box<dyn FnOnce(&mut Context) -> Box<dyn Spot>>;

pub(crate) struct ScenePayload {
    pub(crate) type_id: TypeId,
    pub(crate) value: Rc<dyn Any>,
}

pub(crate) struct ScenePayloadTypeId(pub(crate) TypeId);

pub(crate) struct SceneSwitchRequest {
    pub(crate) factory: SceneFactory,
    pub(crate) payload: Option<ScenePayload>,
}

use std::cell::RefCell;

thread_local! {
    static SCENE_SWITCH_REQUEST: RefCell<Option<SceneSwitchRequest>> = const { RefCell::new(None) };
}

fn with_graphics<R>(f: impl FnOnce(&mut Graphics) -> R) -> R {
    platform::with_graphics(f)
}

fn init_scene_switch() {
    // thread_local is lazily initialized, no explicit init needed here
}

fn request_scene_switch<F>(factory: F)
where
    F: FnOnce(&mut Context) -> Box<dyn Spot> + 'static,
{
    SCENE_SWITCH_REQUEST.with(|request| {
        *request.borrow_mut() = Some(SceneSwitchRequest {
            factory: Box::new(factory),
            payload: None,
        });
    });
}

fn request_scene_switch_with<F>(factory: F, payload: ScenePayload)
where
    F: FnOnce(&mut Context) -> Box<dyn Spot> + 'static,
{
    SCENE_SWITCH_REQUEST.with(|request| {
        *request.borrow_mut() = Some(SceneSwitchRequest {
            factory: Box::new(factory),
            payload: Some(payload),
        });
    });
}

pub(crate) fn take_scene_switch_request() -> Option<SceneSwitchRequest> {
    SCENE_SWITCH_REQUEST.with(|request| request.borrow_mut().take())
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
/// # use spottedcat::{Context, Spot};
/// # struct MyApp;
/// # impl Spot for MyApp {
/// #     fn initialize(_: &mut Context) -> Self { MyApp }
/// #     fn draw(&mut self, _: &mut Context) {}
/// #     fn update(&mut self, _: &mut Context, _dt: std::time::Duration) {}
/// #     fn remove(&self) {}
/// # }
/// spottedcat::run::<MyApp>(spottedcat::WindowConfig::default());
/// ```
#[cfg(not(target_os = "android"))]
pub fn run<T: Spot + 'static>(window: WindowConfig) {
    init_scene_switch();

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        console_error_panic_hook::set_once();
    }

    let event_loop = EventLoop::new().expect("failed to create winit EventLoop");
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    let mut app = window::App::new_wasm::<T>(window.clone(), window.canvas_id.clone());
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    let mut app = window::App::new::<T>(window);
    event_loop.run_app(&mut app).expect("event loop error");
}

#[cfg(target_os = "android")]
pub fn run<T: Spot + 'static>(
    window: WindowConfig,
    app: winit::platform::android::activity::AndroidApp,
) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    init_scene_switch();

    let event_loop = EventLoop::builder()
        .with_android_app(app)
        .build()
        .expect("failed to create winit EventLoop for Android");

    let mut app = window::App::new::<T>(window);
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
/// # use spottedcat::{Context, Spot, switch_scene};
/// # struct MenuScene;
/// # struct GameScene;
/// # impl Spot for MenuScene {
/// #     fn initialize(_: &mut Context) -> Self { MenuScene }
/// #     fn draw(&mut self, _: &mut Context) {}
/// #     fn update(&mut self, _: &mut Context, _dt: std::time::Duration) {}
/// #     fn remove(&self) {}
/// # }
/// # impl Spot for GameScene {
/// #     fn initialize(_: &mut Context) -> Self { GameScene }
/// #     fn draw(&mut self, _: &mut Context) {}
/// #     fn update(&mut self, _: &mut Context, _dt: std::time::Duration) {}
/// #     fn remove(&self) {}
/// # }
/// // In your scene's draw or update method:
/// // if some_condition {
/// //     switch_scene::<GameScene>();
/// // }
/// ```
pub fn switch_scene<T: Spot + 'static>() {
    request_scene_switch(|ctx| Box::new(T::initialize(ctx)));
}

/// Switches to a new scene and provides a payload for the next scene to read.
///
/// The payload can be retrieved in the new scene via `Context::take_resource::<T>()`.
pub fn switch_scene_with<T: Spot + 'static, P: Any>(payload: P) {
    request_scene_switch_with(
        |ctx| Box::new(T::initialize(ctx)),
        ScenePayload {
            type_id: TypeId::of::<P>(),
            value: Rc::new(payload),
        },
    );
}

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
    fn initialize(context: &mut Context) -> Self
    where
        Self: Sized;

    /// Draws the current frame.
    ///
    /// Called every frame. Use the context to issue drawing commands.
    ///
    /// # Arguments
    /// * `context` - Drawing context to add render commands to
    fn draw(&mut self, context: &mut Context);

    fn update(&mut self, context: &mut Context, dt: Duration);

    fn resumed(&mut self, _context: &mut Context) {}

    fn suspended(&mut self, _context: &mut Context) {}

    /// Cleanup when the application is shutting down.
    fn remove(&self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_management() {
        let mut ctx = Context::new();
        let val = Rc::new(42i32);
        ctx.insert_resource(val.clone());

        assert_eq!(ctx.get_resource::<i32>(), Some(val.clone()));

        let removed = ctx.remove_resource::<i32>();
        assert_eq!(removed, Some(val));
        assert_eq!(ctx.get_resource::<i32>(), None);
    }

    #[test]
    fn test_resource_dyn_management() {
        let mut ctx = Context::new();
        let type_id = TypeId::of::<String>();
        let val = Rc::new("hello".to_string());

        ctx.insert_resource_dyn(type_id, val.clone() as Rc<dyn Any>);
        assert_eq!(ctx.get_resource::<String>(), Some(val.clone()));

        ctx.remove_resource_dyn(type_id);
        assert_eq!(ctx.get_resource::<String>(), None);
    }
}
