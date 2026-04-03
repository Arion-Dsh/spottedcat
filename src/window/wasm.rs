use super::App;
use crate::{Pt, platform};
use wasm_bindgen::JsCast;

pub(crate) struct PlatformData {
    pub(crate) window: Option<winit::window::Window>,
    pub(crate) window_id: Option<winit::window::WindowId>,
    pub(crate) canvas_id: Option<String>,
    pub(crate) last_physical_size: Option<(u32, u32)>,
}

impl PlatformData {
    pub(crate) fn new() -> Self {
        Self {
            window: None,
            window_id: None,
            canvas_id: None,
            last_physical_size: None,
            audio_initialized: false,
        }
    }

    pub(crate) fn new_wasm(canvas_id: Option<String>) -> Self {
        Self {
            window: None,
            window_id: None,
            canvas_id,
            last_physical_size: None,
        }
    }
}

impl App {
    /// Lazily initialise the audio system on the first user gesture so that
    /// the browser's autoplay policy is satisfied.
    pub(crate) fn init_audio_on_gesture(&mut self) {
        if self.ctx.runtime.audio.is_some() {
            return;
        }
        match crate::audio::AudioSystem::new() {
            Ok(audio) => {
                self.ctx.runtime.audio = Some(audio);
            }
            Err(e) => {
                web_sys::console::warn_1(&format!("[spot][wasm] Audio unavailable: {e:?}").into());
            }
        }
    }

    pub(crate) fn sync_canvas_resize(&mut self) {
        let Some(window) = self.platform.window.as_ref() else {
            return;
        };
        let Some(surface) = self.surface.as_ref() else {
            return;
        };

        let canvas = self.platform.canvas_id.as_deref().and_then(|id| {
            web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id(id))
                .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        });

        let Some(canvas) = canvas else {
            return;
        };

        let rect = canvas.get_bounding_client_rect();
        let css_w = rect.width();
        let css_h = rect.height();
        if !(css_w.is_finite() && css_h.is_finite()) {
            return;
        }

        let dpr = self.scale_factor;
        let w = ((css_w * dpr).round() as i64).max(1) as u32;
        let h = ((css_h * dpr).round() as i64).max(1) as u32;

        if self.platform.last_physical_size == Some((w, h)) {
            return;
        }
        self.platform.last_physical_size = Some((w, h));

        canvas.set_width(w);
        canvas.set_height(h);

        if let Some(g) = self.ctx.runtime.graphics.as_mut() {
            g.resize(surface, w, h);
        }

        self.ctx.set_window_logical_size(
            Pt::from_physical_px(w as f64, self.scale_factor),
            Pt::from_physical_px(h as f64, self.scale_factor),
        );

        // Ensure winit is aware of the effective surface size too.
        window.request_redraw();
    }
}

pub(crate) unsafe fn handle_wasm_graphics_init_result(
    app_ptr: *mut App,
    graphics_r: anyhow::Result<crate::Graphics>,
) {
    match graphics_r {
        Ok(graphics) => {
            web_sys::console::log_1(&"[spot][wasm] Graphics initialized successfully".into());
            (*app_ptr).init_state = platform::GraphicsInitState::Ready(Some(graphics));
        }
        Err(e) => {
            web_sys::console::error_1(
                &format!("[spot][wasm][init] Graphics::new failed: {:?}", e).into(),
            );
            (*app_ptr).init_state = platform::GraphicsInitState::Failed;
        }
    }

    if let Some(window) = (*app_ptr).platform.window.as_ref() {
        window.request_redraw();
    }
}
