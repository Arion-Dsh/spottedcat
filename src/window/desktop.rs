use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowId};
use crate::{
    Pt, take_quit_request, with_graphics,
};
use std::rc::Rc;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web_time::Instant;

use super::App;
use crate::platform;

pub(crate) struct PlatformData {
    pub(crate) window: Option<Window>,
    pub(crate) window_id: Option<WindowId>,
    #[cfg(all(target_os = "ios", feature = "sensors"))]
    pub(crate) sensor_state: Option<super::ios::IosSensorState>,
}

impl PlatformData {
    pub(crate) fn new() -> Self {
        Self {
            window: None,
            window_id: None,
            #[cfg(all(target_os = "ios", feature = "sensors"))]
            sensor_state: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        web_sys::console::log_1(&"[spot][wasm] resumed() called".into());

        event_loop.set_control_flow(ControlFlow::Poll);
        self.previous = Some(Instant::now());
        self.lag = Duration::ZERO;

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            web_sys::console::log_1(&"[spot][wasm] resumed".into());
        }

        // 1. Create window if it doesn't exist
        if self.platform.window.is_none() {
            let _w = self.window_config.width.0.max(1.0) as f64;
            let _h = self.window_config.height.0.max(1.0) as f64;

            let attributes = Window::default_attributes()
                .with_title(self.window_config.title.clone())
                .with_resizable(self.window_config.resizable);

            #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
            let attributes = {
                use wasm_bindgen::JsCast;
                use winit::platform::web::WindowAttributesExtWebSys;
                let canvas = self.platform.canvas_id.as_deref().and_then(|id| {
                    web_sys::window()
                        .and_then(|w| w.document())
                        .and_then(|d| d.get_element_by_id(id))
                        .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
                });
                attributes.with_inner_size(winit::dpi::LogicalSize::new(_w, _h)).with_canvas(canvas)
            };

            #[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
            let attributes = attributes.with_inner_size(winit::dpi::LogicalSize::new(_w, _h));

            let window = event_loop.create_window(attributes).expect("failed to create window");
            window.set_ime_allowed(true);
            
            self.scale_factor = window.scale_factor();
            self.context.set_scale_factor(self.scale_factor);
            let size = window.inner_size();
            self.context.set_window_logical_size(
                Pt::from_physical_px(size.width as f64, self.scale_factor),
                Pt::from_physical_px(size.height as f64, self.scale_factor),
            );
            eprintln!("[spot][init] Window created: {}x{} (dpr: {})", size.width, size.height, self.scale_factor);

            self.platform.window_id = Some(window.id());
            self.platform.window = Some(window);
        }

        // 2. (Re)create surface if needed
        if let Some(window) = self.platform.window.as_ref() {
            if self.surface.is_none() {
                let size = window.inner_size();
                match self.instance.create_surface(window) {
                    Ok(s) => {
                        let surface = unsafe {
                            std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(s)
                        };
                        self.surface = Some(surface);
                        
                        // If graphics is already initialized, resize the surface
                        if let platform::GraphicsInitState::Ready(_) = self.init_state {
                            if let Some(surface) = self.surface.as_ref() {
                                with_graphics(|g| g.resize(surface, size.width, size.height));
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[spot][surface] create failed: {:?}", e);
                    }
                }
            }
        }

        // 3. Initialize graphics if not already started
        if let platform::GraphicsInitState::NotStarted = self.init_state {
            if let Some(surface) = self.surface.as_ref() {
                if let Some(window) = self.platform.window.as_ref() {
                    let size = window.inner_size();
                    
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        platform::begin_graphics_init(
                            &mut self.init_state,
                            &self.instance,
                            surface,
                            size.width,
                            size.height,
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
                            Box::new(move |graphics_r| unsafe {
                                super::wasm::handle_wasm_graphics_init_result(app_ptr, graphics_r)
                            }),
                        );
                    }
                }
            }
        }

        // 4. Handle app-level resume
        if let Some(spot) = self.spot.as_mut() {
            spot.resumed(&mut self.context);
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

        if let Some(window) = self.platform.window.as_ref() {
            window.request_redraw();
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(spot) = self.spot.as_mut() {
            spot.suspended(&mut self.context);
        }
        #[cfg(all(target_os = "ios", feature = "sensors"))]
        if let Some(state) = self.platform.sensor_state.as_ref() {
            state.disable();
        }
        self.surface.take();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Focused(focused) => {
                self.context.input_mut().handle_focus(focused);
            }
            WindowEvent::Ime(ime) => {
                self.context.input_mut().handle_ime(ime);
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = scale_factor;
                self.context.set_scale_factor(self.scale_factor);

                if let Some(window) = self.platform.window.as_ref() {
                    let size = window.inner_size();
                    self.context.set_window_logical_size(
                        Pt::from_physical_px(size.width as f64, self.scale_factor),
                        Pt::from_physical_px(size.height as f64, self.scale_factor),
                    );
                }
            }
            WindowEvent::Resized(new_size) => {
                if let Some(surface) = self.surface.as_ref() {
                    with_graphics(|g| g.resize(surface, new_size.width, new_size.height));
                }

                #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
                {
                    self.platform.last_physical_size = Some((new_size.width.max(1), new_size.height.max(1)));
                }

                self.context.set_window_logical_size(
                    Pt::from_physical_px(new_size.width as f64, self.scale_factor),
                    Pt::from_physical_px(new_size.height as f64, self.scale_factor),
                );
            }
            WindowEvent::CursorMoved { position, .. } => {
                let x = Pt::from_physical_px(position.x, self.scale_factor);
                let y = Pt::from_physical_px(position.y, self.scale_factor);
                self.context.input_mut().handle_cursor_moved(x, y);
            }
            WindowEvent::Touch(touch) => {
                self.context
                    .input_mut()
                    .handle_touch(touch, self.scale_factor);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
                {
                    self.init_audio_on_gesture();
                    platform::try_resume_audio();
                }
                self.context.input_mut().handle_mouse_input(state, button);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.context.input_mut().handle_mouse_wheel(delta);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
                {
                    self.init_audio_on_gesture();
                    platform::try_resume_audio();
                }
                self.context
                    .input_mut()
                    .handle_keyboard_input(event.state, event.physical_key);

                if matches!(event.state, winit::event::ElementState::Pressed) {
                    if let Some(text) = event.text.as_deref() {
                        for ch in text.chars() {
                            self.context.input_mut().handle_received_character(ch);
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if self.spot.is_none() {
                    if platform::finalize_graphics(&mut self.init_state) {
                        let spot = (self.scene_factory)(&mut self.context);
                        self.spot = Some(spot);
                    }
                }

                #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
                {
                    if self.spot.is_some() {
                        self.sync_canvas_resize();
                    }
                }

                if let Some(surface) = self.surface.as_ref() {
                    self.context.begin_frame();

                    if let Some(spot) = self.spot.as_mut() {
                        spot.draw(&mut self.context);
                    }

                    if let Some(request) = crate::take_scene_switch_request() {
                        if let Some(payload) = request.payload {
                            self.context
                                .insert_resource_dyn(payload.type_id, payload.value);
                            self.context
                                .insert_resource(Rc::new(crate::ScenePayloadTypeId(payload.type_id)));
                        } else if let Some(last) =
                            self.context.remove_resource::<crate::ScenePayloadTypeId>()
                        {
                            if let Ok(last) = std::rc::Rc::try_unwrap(last) {
                                self.context.remove_resource_dyn(last.0);
                            }
                        }
                        if let Some(old_spot) = self.spot.take() {
                            old_spot.remove();
                        }
                        self.spot = Some((request.factory)(&mut self.context));
                    }

                    if self.spot.is_some() {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let r = with_graphics(|g| g.draw_context(surface, &self.context));
                            if let Some(Err(e)) = r {
                                match e {
                                    wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                                        if let Some(window) = self.platform.window.as_ref() {
                                            let size = window.inner_size();
                                            let surface_r = self.instance.create_surface(window);
                                            match surface_r {
                                                Ok(s) => {
                                                    let surface = unsafe {
                                                        std::mem::transmute::<
                                                            wgpu::Surface<'_>,
                                                            wgpu::Surface<'static>,
                                                        >(
                                                            s
                                                        )
                                                    };
                                                    self.surface = Some(surface);

                                                    if let Some(surface) = self.surface.as_ref() {
                                                        with_graphics(|g| {
                                                            g.resize(
                                                                surface,
                                                                size.width,
                                                                size.height,
                                                            )
                                                        });
                                                    }
                                                }
                                                Err(e) => {
                                                    eprintln!(
                                                        "[spot][surface] recreate after error failed: {:?}",
                                                        e
                                                    );
                                                    self.surface.take();
                                                }
                                            }
                                            window.request_redraw();
                                        }
                                    }
                                    wgpu::SurfaceError::OutOfMemory => {
                                        event_loop.exit();
                                    }
                                    wgpu::SurfaceError::Timeout => {
                                        if let Some(window) = self.platform.window.as_ref() {
                                            window.request_redraw();
                                        }
                                    }
                                    wgpu::SurfaceError::Other => {
                                        eprintln!("[spot][surface] other error: {:?}", e);
                                        if let Some(window) = self.platform.window.as_ref() {
                                            window.request_redraw();
                                        }
                                    }
                                }
                            }
                        }

                        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
                        {
                            let r = with_graphics(|g| g.draw_context(surface, &self.context));
                            if let Some(Err(e)) = r {
                                web_sys::console::error_1(
                                    &format!("[spot][wasm][surface] {:?}", e).into(),
                                );
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if take_quit_request() {
            event_loop.exit();
            return;
        }

        #[cfg(all(target_os = "ios", feature = "sensors"))]
        if let Some(state) = self.platform.sensor_state.as_ref() {
            state.poll(&mut self.context.input_mut());
        }

        let now = Instant::now();
        if let Some(previous) = self.previous.replace(now) {
            let elapsed = now.duration_since(previous);
            self.lag = self.lag.saturating_add(elapsed);

            while self.lag >= self.fixed_dt {
                if let Some(spot) = self.spot.as_mut() {
                    spot.update(&mut self.context, self.fixed_dt);
                }
                self.context.input_mut().end_frame();
                self.lag = self.lag.saturating_sub(self.fixed_dt);
            }
        }

        if let Some(window) = self.platform.window.as_ref() {
            window.request_redraw();
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Ensure surface is dropped before window.
        self.surface.take();
        self.platform.window.take();
    }
}
