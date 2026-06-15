use crate::window;
use crate::{Pt, Spot};
#[cfg(target_os = "android")]
use android_activity::AndroidApp;

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
    #[cfg(target_family = "wasm")]
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
            #[cfg(target_family = "wasm")]
            canvas_id: None,
            transparent: false,
        }
    }
}

/// Starts the application with the specified scene type `T` and configuration.
///
/// This function is the main entry point for SDL-backed platforms.
#[cfg(not(target_os = "android"))]
pub fn run<T: Spot + 'static>(window: WindowConfig) {
    window::run_sdl::<T>(window);
}

/// Starts the application on Android with the specified scene type `T`.
#[cfg(target_os = "android")]
pub fn run<T: Spot + 'static>(window: WindowConfig, _app: AndroidApp) {
    window::run_sdl::<T>(window);
}
