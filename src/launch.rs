use crate::window;
use crate::{Pt, Spot};
#[cfg(target_os = "android")]
use android_activity::AndroidApp;
#[cfg(not(target_os = "android"))]
use winit::event_loop::EventLoop;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use console_error_panic_hook;

/// Configuration for the application window.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// The window title.
    pub title: String,
    /// Logical width of the window.
    pub width: Pt,
    /// Logical height of the window.
    pub height: Pt,
    /// Whether the window is resizable by the user.
    pub resizable: bool,
    /// Whether to start in fullscreen mode.
    pub fullscreen: bool,
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    /// Optional canvas element ID for WebAssembly.
    pub canvas_id: Option<String>,
    /// Whether the window should have a transparent background.
    pub transparent: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "spot".to_string(),
            width: Pt(800.0),
            height: Pt(600.0),
            resizable: true,
            fullscreen: false,
            #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
            canvas_id: None,
            transparent: false,
        }
    }
}

/// Starts the application with the specified scene type `T` and configuration.
///
/// This function is the main entry point for most platforms. On desktop and web,
/// it initializes the event loop and starts the renderer.
#[cfg(not(target_os = "android"))]
pub fn run<T: Spot + 'static>(window: WindowConfig) {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        console_error_panic_hook::set_once();
    }

    let event_loop = EventLoop::new().expect("failed to create winit EventLoop");
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    let mut app = window::App::new_wasm::<T>(window.clone(), window.canvas_id.clone());
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    let mut app = window::App::new::<T>(window);

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        let app = Box::new(app);
        let app = Box::leak(app);
        event_loop.run_app(app).expect("event loop error");
    }
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        event_loop.run_app(&mut app).expect("event loop error");
    }
}

/// Starts the application on Android with the specified scene type `T`.
#[cfg(target_os = "android")]
pub fn run<T: Spot + 'static>(window: WindowConfig, app: AndroidApp) {
    let mut app_impl = window::App::new::<T>(window);
    app_impl.run(app);
}
