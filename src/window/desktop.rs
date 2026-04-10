use super::App;
use crate::Pt;
use crate::platform;
use crate::scenes::take_quit_request;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use wasm_bindgen::JsCast;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Fullscreen, Window, WindowId};

pub(crate) struct PlatformData {
    pub(crate) window: Option<Window>,
    pub(crate) window_id: Option<WindowId>,
    #[cfg(all(target_os = "ios", feature = "sensors"))]
    pub(crate) sensor_state: Option<super::ios::IosSensorState>,
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    pub(crate) canvas_id: Option<String>,
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    pub(crate) last_physical_size: Option<(u32, u32)>,
}

impl PlatformData {
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub(crate) fn new() -> Self {
        Self {
            window: None,
            window_id: None,
            #[cfg(all(target_os = "ios", feature = "sensors"))]
            sensor_state: None,
            #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
            canvas_id: None,
            #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
            last_physical_size: None,
        }
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    pub(crate) fn new_wasm(canvas_id: Option<String>) -> Self {
        Self {
            window: None,
            window_id: None,
            #[cfg(all(target_os = "ios", feature = "sensors"))]
            sensor_state: None,
            canvas_id,
            last_physical_size: None,
        }
    }
}

impl App {
    fn sync_window_metrics(&mut self, width: u32, height: u32) {
        self.ctx
            .update_window_metrics_physical(width, height, self.scale_factor);
    }

    fn request_redraw(&self) {
        if let Some(window) = self.platform.window.as_ref() {
            window.request_redraw();
        }
    }

    fn apply_pending_window_requests(&mut self) {
        let Some(window) = self.platform.window.as_ref() else {
            let _ = self.ctx.take_window_title_request();
            let _ = self.ctx.take_cursor_visible_request();
            let _ = self.ctx.take_fullscreen_request();
            return;
        };

        if let Some(title) = self.ctx.take_window_title_request() {
            window.set_title(&title);
        }
        if let Some(visible) = self.ctx.take_cursor_visible_request() {
            window.set_cursor_visible(visible);
        }
        if let Some(enabled) = self.ctx.take_fullscreen_request() {
            if enabled {
                window.set_fullscreen(Some(Fullscreen::Borderless(None)));
            } else {
                window.set_fullscreen(None);
            }
        }
    }

    fn create_window_if_needed(&mut self, event_loop: &ActiveEventLoop) {
        if self.platform.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title(self.window_config.title.clone())
            .with_resizable(self.window_config.resizable)
            .with_transparent(true);

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        let attributes = {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            let width = self.window_config.width.0.max(1.0) as f64;
            let height = self.window_config.height.0.max(1.0) as f64;

            let canvas = self.platform.canvas_id.as_deref().and_then(|id| {
                web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get_element_by_id(id))
                    .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            });

            attributes
                .with_inner_size(winit::dpi::LogicalSize::new(width, height))
                .with_canvas(canvas)
        };

        #[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
        let attributes = {
            let width = self.window_config.width.0.max(1.0) as f64;
            let height = self.window_config.height.0.max(1.0) as f64;
            attributes.with_inner_size(winit::dpi::LogicalSize::new(width, height))
        };

        let window = event_loop
            .create_window(attributes)
            .expect("failed to create window");
        window.set_ime_allowed(true);
        if self.window_config.fullscreen {
            window.set_fullscreen(Some(Fullscreen::Borderless(None)));
        }

        self.scale_factor = window.scale_factor();
        let size = window.inner_size();
        self.sync_window_metrics(size.width, size.height);
        eprintln!(
            "[spot][init] Window created: {}x{} (dpr: {})",
            size.width, size.height, self.scale_factor
        );

        self.platform.window_id = Some(window.id());
        self.platform.window = Some(window);
    }

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    fn ensure_audio_initialized(&mut self) {
        if self.ctx.runtime.audio.is_none() {
            match crate::audio::AudioSystem::new() {
                Ok(audio) => self.ctx.runtime.audio = Some(audio),
                Err(e) => eprintln!("[spot][audio] initialization failed: {:?}", e),
            }
        }
    }

    fn ensure_surface(&mut self) {
        if self.surface.is_some() {
            return;
        }

        let Some(window) = self.platform.window.as_ref() else {
            return;
        };

        let size = window.inner_size();
        match self.instance.create_surface(window) {
            Ok(surface) => {
                let surface = unsafe {
                    std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(surface)
                };
                self.surface = Some(surface);

                if let platform::GraphicsInitState::Ready(_) = self.init_state
                    && let Some(surface) = self.surface.as_ref()
                    && let Some(g) = self.ctx.graphics_mut()
                {
                    g.resize(surface, size.width, size.height);
                }
            }
            Err(e) => eprintln!("[spot][surface] create failed: {:?}", e),
        }
    }

    fn recreate_surface(&mut self) {
        let Some(window) = self.platform.window.as_ref() else {
            return;
        };

        let size = window.inner_size();
        match self.instance.create_surface(window) {
            Ok(surface) => {
                let surface = unsafe {
                    std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(surface)
                };
                self.surface = Some(surface);

                if let Some(surface) = self.surface.as_ref()
                    && let Some(g) = self.ctx.graphics_mut()
                {
                    g.resize(surface, size.width, size.height);
                }
                eprintln!("[spot][surface] Surface recreated successfully.");
            }
            Err(e) => {
                eprintln!("[spot][surface] recreate after error failed: {:?}", e);
                self.surface.take();
            }
        }
    }

    fn begin_graphics_init_if_needed(&mut self) {
        if !matches!(self.init_state, platform::GraphicsInitState::NotStarted) {
            return;
        }

        let Some(surface) = self.surface.as_ref() else {
            return;
        };
        let Some(window) = self.platform.window.as_ref() else {
            return;
        };
        let size = window.inner_size();

        #[cfg(not(target_arch = "wasm32"))]
        {
            platform::begin_graphics_init(
                &mut self.init_state,
                &self.instance,
                surface,
                size.width,
                size.height,
                self.window_config.transparent,
            );
        }

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            let instance = self.instance.clone();
            let surface_ptr: *const wgpu::Surface<'static> = surface;
            let app_ptr: *mut App = self;
            platform::begin_graphics_init(
                &mut self.init_state,
                instance,
                surface_ptr,
                size.width,
                size.height,
                self.window_config.transparent,
                Box::new(move |graphics_r| unsafe {
                    super::wasm::handle_wasm_graphics_init_result(app_ptr, graphics_r)
                }),
            );
        }
    }

    fn ensure_scene_ready(&mut self) {
        if self.scene.has_active_scene() {
            return;
        }

        if let Some(graphics) = platform::finalize_graphics(&mut self.init_state) {
            self.ctx.attach_graphics(graphics);
            self.scene.initialize_if_missing(&mut self.ctx);
        }
    }

    fn handle_surface_error(&mut self, event_loop: &ActiveEventLoop, error: wgpu::SurfaceError) {
        match error {
            wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                eprintln!(
                    "[spot][surface] Surface lost or outdated: {:?}. Recreating...",
                    error
                );
                self.recreate_surface();
                self.request_redraw();
            }
            wgpu::SurfaceError::OutOfMemory => event_loop.exit(),
            wgpu::SurfaceError::Timeout | wgpu::SurfaceError::Other => {
                eprintln!("[spot][surface] draw error: {:?}", error);
                self.request_redraw();
            }
        }
    }

    fn draw_frame(&mut self, event_loop: &ActiveEventLoop) {
        self.ensure_scene_ready();

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        self.sync_canvas_resize();

        let Some(surface) = self.surface.as_ref() else {
            return;
        };

        self.ctx.begin_frame();
        let screen = super::make_screen_target(&self.ctx);
        if let Some(spot) = self.scene.spot_mut() {
            spot.draw(&mut self.ctx, screen);
        }

        self.scene.apply_pending_switch(&mut self.ctx);

        let mut graphics = self.ctx.detach_graphics();
        let draw_result = graphics
            .as_mut()
            .map(|g| g.draw_context(surface, &mut self.ctx));
        if let Some(graphics) = graphics {
            self.ctx.attach_graphics(graphics);
        }

        if let Some(Err(error)) = draw_result {
            self.handle_surface_error(event_loop, error);
        } else {
            self.request_redraw();
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.timing.reset();
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.timing.next_deadline()));

        self.create_window_if_needed(event_loop);
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        self.ensure_audio_initialized();
        self.ensure_surface();
        self.begin_graphics_init_if_needed();

        if let Some(spot) = self.scene.spot_mut() {
            spot.resumed(&mut self.ctx);
        }

        #[cfg(all(target_os = "ios", feature = "sensors"))]
        {
            if self.platform.sensor_state.is_none() {
                self.platform.sensor_state = Some(super::ios::IosSensorState::new());
            }
            if let Some(state) = self.platform.sensor_state.as_ref() {
                state.enable();
            }
        }

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            self.sync_canvas_resize();
            if let Some(window) = self.platform.window.as_ref() {
                let window_ptr = window as *const winit::window::Window;
                let closure = wasm_bindgen::prelude::Closure::once(move || unsafe {
                    (*window_ptr).request_redraw();
                });
                web_sys::window()
                    .and_then(|w| {
                        w.request_animation_frame(closure.as_ref().unchecked_ref())
                            .ok()
                    })
                    .expect("failed to request_animation_frame");
                closure.forget();
            }
        }
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        self.request_redraw();
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(spot) = self.scene.spot_mut() {
            spot.suspended(&mut self.ctx);
        }
        self.ctx.clear_transient_input();
        self.ctx.clear_transient_state();
        #[cfg(all(target_os = "ios", feature = "sensors"))]
        if let Some(state) = self.platform.sensor_state.as_ref() {
            state.disable();
        }
        self.surface.take();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Focused(focused) => self.ctx.input_mut().handle_focus(focused),
            WindowEvent::Ime(ime) => self.ctx.input_mut().handle_ime(ime),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = scale_factor;
                if let Some(window) = self.platform.window.as_ref() {
                    let size = window.inner_size();
                    self.sync_window_metrics(size.width, size.height);
                }
            }
            WindowEvent::Resized(new_size) => {
                if let Some(surface) = self.surface.as_ref()
                    && let Some(g) = self.ctx.graphics_mut()
                {
                    g.resize(surface, new_size.width, new_size.height);
                }

                #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
                {
                    self.platform.last_physical_size =
                        Some((new_size.width.max(1), new_size.height.max(1)));
                }

                self.sync_window_metrics(new_size.width, new_size.height);
            }
            WindowEvent::CursorMoved { position, .. } => {
                let x = Pt::from_physical_px(position.x, self.scale_factor);
                let y = Pt::from_physical_px(position.y, self.scale_factor);
                self.ctx.input_mut().handle_cursor_moved(x, y);
            }
            WindowEvent::Touch(touch) => {
                self.ctx.input_mut().handle_touch(touch, self.scale_factor);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
                {
                    self.init_audio_on_gesture();
                    platform::try_resume_audio(&mut self.ctx);
                }
                self.ctx.input_mut().handle_mouse_input(state, button);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.ctx.input_mut().handle_mouse_wheel(delta);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
                {
                    self.init_audio_on_gesture();
                    platform::try_resume_audio(&mut self.ctx);
                }

                self.ctx
                    .input_mut()
                    .handle_keyboard_input(event.state, event.physical_key);

                if matches!(event.state, winit::event::ElementState::Pressed)
                    && let Some(text) = event.text.as_deref()
                {
                    for ch in text.chars() {
                        self.ctx.input_mut().handle_received_character(ch);
                    }
                }
            }
            WindowEvent::RedrawRequested => self.draw_frame(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.apply_pending_window_requests();

        if take_quit_request() {
            event_loop.exit();
            return;
        }

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        self.timing.run_updates(8, |dt| {
            #[cfg(all(target_os = "ios", feature = "sensors"))]
            if let Some(state) = self.platform.sensor_state.as_ref() {
                state.poll(&mut self.ctx.input_mut());
            }

            self.ctx.set_delta_time(dt);
            if let Some(spot) = self.scene.spot_mut() {
                spot.update(&mut self.ctx, dt);
            }
            self.ctx.input_mut().end_frame();
        });

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        self.request_redraw();

        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        let updates = self.timing.run_updates(8, |dt| {
            #[cfg(all(target_os = "ios", feature = "sensors"))]
            if let Some(state) = self.platform.sensor_state.as_ref() {
                state.poll(&mut self.ctx.input_mut());
            }

            self.ctx.set_delta_time(dt);
            if let Some(spot) = self.scene.spot_mut() {
                spot.update(&mut self.ctx, dt);
            }
            self.ctx.input_mut().end_frame();
        });

        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        if updates > 0 {
            self.request_redraw();
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.timing.next_deadline()));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.surface.take();
        self.platform.window.take();
    }
}
