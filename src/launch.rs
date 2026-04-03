use crate::window;
use crate::{Pt, Spot};
#[cfg(target_os = "android")]
use android_activity::AndroidApp;
#[cfg(not(target_os = "android"))]
use winit::event_loop::EventLoop;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use console_error_panic_hook;

#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: Pt,
    pub height: Pt,
    pub resizable: bool,
    pub fullscreen: bool,
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    pub canvas_id: Option<String>,
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
    event_loop.run_app(&mut app).expect("event loop error");
}

#[cfg(target_os = "android")]
pub fn run<T: Spot + 'static>(window: WindowConfig, app: AndroidApp) {
    let mut app_impl = window::App::new::<T>(window);
    app_impl.run(app);
}
