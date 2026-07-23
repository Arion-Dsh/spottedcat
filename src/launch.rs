use crate::window;
use crate::{Pt, Spot};
#[cfg(target_os = "android")]
use android_activity::AndroidApp;
use std::time::Duration;

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
    /// Fixed-frequency game logic updates per second.
    ///
    /// Rendering remains synchronized independently with the display. For example,
    /// `update_hz: 60` produces a fixed `Duration` of roughly 16.67 ms for `update`,
    /// while `update_hz: 120` produces roughly 8.33 ms.
    pub update_hz: u32,
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
            update_hz: 60,
        }
    }
}

impl WindowConfig {
    pub(crate) fn fixed_update_step(&self) -> Duration {
        assert!(
            self.update_hz > 0,
            "WindowConfig::update_hz must be greater than zero"
        );
        let step = Duration::from_secs_f64(1.0 / f64::from(self.update_hz));
        assert!(
            !step.is_zero(),
            "WindowConfig::update_hz is too high to represent as a Duration"
        );
        step
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_update_rate_is_sixty_hz() {
        let config = WindowConfig::default();
        assert_eq!(config.update_hz, 60);
        assert!((config.fixed_update_step().as_secs_f64() - 1.0 / 60.0).abs() < 1e-9);
    }

    #[test]
    fn update_rate_controls_fixed_step() {
        let config = WindowConfig {
            update_hz: 120,
            ..Default::default()
        };
        assert!((config.fixed_update_step().as_secs_f64() - 1.0 / 120.0).abs() < 1e-9);
    }

    #[test]
    #[should_panic(expected = "update_hz must be greater than zero")]
    fn zero_update_rate_is_rejected() {
        WindowConfig {
            update_hz: 0,
            ..Default::default()
        }
        .fixed_update_step();
    }
}

/// Starts the application with the specified scene type `T` and configuration.
///
/// This function is the main entry point for most platforms. On desktop and web,
/// it initializes the event loop and starts the renderer.
#[cfg(not(target_os = "android"))]
pub fn run<T: Spot + 'static>(window: WindowConfig) {
    <window::WinitWgpuBackend as window::WindowBackend>::run::<T>(window);
}

/// Starts the application on Android with the specified scene type `T`.
#[cfg(target_os = "android")]
pub fn run<T: Spot + 'static>(window: WindowConfig, app: AndroidApp) {
    <window::WinitWgpuBackend as window::WindowBackend>::run::<T>(window, app);
}
