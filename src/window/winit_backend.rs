use super::WindowBackend;
use crate::{Spot, WindowConfig};

pub(crate) struct WinitWgpuBackend;

impl WindowBackend for WinitWgpuBackend {
    #[cfg(not(target_os = "android"))]
    fn run<T: Spot + 'static>(window: WindowConfig) {
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            console_error_panic_hook::set_once();
        }

        let event_loop =
            winit::event_loop::EventLoop::new().expect("failed to create winit EventLoop");

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            let app = super::App::new_wasm::<T>(window.clone(), window.canvas_id.clone());
            let app = Box::new(app);
            let app = Box::leak(app);
            event_loop.run_app(app).expect("event loop error");
        }

        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        {
            let mut app = super::App::new::<T>(window);
            event_loop.run_app(&mut app).expect("event loop error");
        }
    }

    #[cfg(target_os = "android")]
    fn run<T: Spot + 'static>(window: WindowConfig, app: android_activity::AndroidApp) {
        let mut app_impl = super::App::new::<T>(window);
        app_impl.run(app);
    }
}
