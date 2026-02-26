use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowId};

use crate::platform;
use crate::{
    Context, Pt, ScenePayloadTypeId, Spot, WindowConfig, take_scene_switch_request, with_graphics,
};
use std::rc::Rc;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web_time::Instant;

type GraphicsInitState = platform::GraphicsInitState;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
unsafe fn handle_wasm_graphics_init_result(
    app_ptr: *mut App,
    graphics_r: anyhow::Result<crate::graphics::Graphics>,
) {
    match graphics_r {
        Ok(graphics) => {
            (*app_ptr).init_state = GraphicsInitState::Ready(Some(graphics));
        }
        Err(e) => {
            web_sys::console::error_1(
                &format!("[spot][wasm][init] Graphics::new failed: {:?}", e).into(),
            );
            (*app_ptr).init_state = GraphicsInitState::Failed;
        }
    }

    if let Some(window) = (*app_ptr).window.as_ref() {
        window.request_redraw();
    }
}

pub(crate) struct App {
    window: Option<Window>,
    window_id: Option<WindowId>,
    instance: wgpu::Instance,
    surface: Option<wgpu::Surface<'static>>,
    context: Context,
    spot: Option<Box<dyn Spot>>,
    scene_factory: Box<dyn Fn(&mut Context) -> Box<dyn Spot> + Send>,
    window_config: WindowConfig,
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    canvas_id: Option<String>,
    init_state: GraphicsInitState,
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    last_physical_size: Option<(u32, u32)>,
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    audio_initialized: bool,
    scale_factor: f64,
    previous: Option<Instant>,
    lag: Duration,
    fixed_dt: Duration,
}

impl App {
    pub(crate) fn new<T: Spot + 'static>(window_config: WindowConfig) -> Self {
        let instance = platform::create_wgpu_instance();
        let audio = crate::audio::AudioSystem::new().expect("failed to initialize audio system");
        let _ = platform::set_global_audio(audio);

        Self {
            window: None,
            window_id: None,
            instance,
            surface: None,
            context: Context::new(),
            spot: None,
            scene_factory: Box::new(|ctx| Box::new(T::initialize(ctx))),
            window_config,
            #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
            canvas_id: None,
            init_state: GraphicsInitState::NotStarted,
            #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
            last_physical_size: None,
            #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
            audio_initialized: false,
            scale_factor: 1.0,
            previous: None,
            lag: Duration::ZERO,
            fixed_dt: Duration::from_secs_f64(1.0 / 60.0),
        }
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    pub(crate) fn new_wasm<T: Spot + 'static>(
        window_config: WindowConfig,
        canvas_id: Option<String>,
    ) -> Self {
        let instance = platform::create_wgpu_instance();
        // Audio init is deferred to first user gesture to satisfy browser
        // autoplay policy.  See `init_audio_on_gesture` below.

        Self {
            window: None,
            window_id: None,
            instance,
            surface: None,
            context: Context::new(),
            spot: None,
            scene_factory: Box::new(|ctx| Box::new(T::initialize(ctx))),
            window_config,
            canvas_id,
            init_state: GraphicsInitState::NotStarted,
            last_physical_size: None,
            audio_initialized: false,
            scale_factor: 1.0,
            previous: None,
            lag: Duration::ZERO,
            fixed_dt: Duration::from_nanos(8_333_333),
        }
    }

    /// Lazily initialise the audio system on the first user gesture so that
    /// the browser's autoplay policy is satisfied.
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    fn init_audio_on_gesture(&mut self) {
        if self.audio_initialized {
            return;
        }
        self.audio_initialized = true;
        match crate::audio::AudioSystem::new() {
            Ok(audio) => {
                let _ = platform::set_global_audio(audio);
            }
            Err(e) => {
                web_sys::console::warn_1(&format!("[spot][wasm] Audio unavailable: {e:?}").into());
            }
        }
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    fn sync_canvas_resize(&mut self) {
        use wasm_bindgen::JsCast;

        let Some(window) = self.window.as_ref() else {
            return;
        };
        let Some(surface) = self.surface.as_ref() else {
            return;
        };

        let canvas = self.canvas_id.as_deref().and_then(|id| {
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

        if self.last_physical_size == Some((w, h)) {
            return;
        }
        self.last_physical_size = Some((w, h));

        canvas.set_width(w);
        canvas.set_height(h);

        with_graphics(|g| g.resize(surface, w, h));

        self.context.set_window_logical_size(
            Pt::from_physical_px(w as f64, self.scale_factor),
            Pt::from_physical_px(h as f64, self.scale_factor),
        );

        // Ensure winit is aware of the effective surface size too.
        window.request_redraw();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);
        self.previous = Some(Instant::now());
        self.lag = Duration::ZERO;

        #[cfg(all(not(target_arch = "wasm32"), target_os = "android"))]
        {
            // Android can keep the winit Window alive while the underlying native surface is
            // recreated multiple times. Even if we still have a Surface handle, it may be stale.
            // Recreate and reconfigure the surface on every resume.
            if let Some(window) = self.window.as_ref() {
                self.scale_factor = window.scale_factor();
                self.context.set_scale_factor(self.scale_factor);
                let size = window.inner_size();

                self.context.set_window_logical_size(
                    Pt::from_physical_px(size.width as f64, self.scale_factor),
                    Pt::from_physical_px(size.height as f64, self.scale_factor),
                );

                let surface_r = self.instance.create_surface(window);
                match surface_r {
                    Ok(s) => {
                        let surface = unsafe {
                            std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(s)
                        };
                        self.surface = Some(surface);

                        if let Some(surface) = self.surface.as_ref() {
                            with_graphics(|g| g.resize(surface, size.width, size.height));
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "[spot][android][surface] recreate on resume failed: {:?}",
                            e
                        );
                        self.surface.take();
                    }
                }

                if let Some(spot) = self.spot.as_mut() {
                    spot.resumed(&mut self.context);
                }
                window.request_redraw();
                return;
            }
        }

        // If we already have a window but the surface was dropped (common on Android backgrounding),
        // recreate just the surface and reconfigure it. Creating a second window here can lead to
        // invalid surface state and wgpu panics.
        if self.window.is_some() && self.surface.is_none() {
            if let Some(window) = self.window.as_ref() {
                let size = window.inner_size();
                let surface_r = self.instance.create_surface(window);
                match surface_r {
                    Ok(s) => {
                        let surface = unsafe {
                            std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(s)
                        };
                        self.surface = Some(surface);

                        if let Some(surface) = self.surface.as_ref() {
                            with_graphics(|g| g.resize(surface, size.width, size.height));
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "[spot][android][surface] recreate on resume failed: {:?}",
                            e
                        );
                        self.surface.take();
                    }
                }

                if let Some(spot) = self.spot.as_mut() {
                    spot.resumed(&mut self.context);
                }
                window.request_redraw();
            }
            return;
        }

        if self.window.is_some() && self.surface.is_some() {
            if let Some(window) = self.window.as_ref() {
                if let Some(spot) = self.spot.as_mut() {
                    spot.resumed(&mut self.context);
                }
                window.request_redraw();
            }
            return;
        }

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            web_sys::console::log_1(&"[spot][wasm] resumed".into());
        }

        let w = (self.window_config.width.0).max(1.0) as f64;
        let h = (self.window_config.height.0).max(1.0) as f64;

        let window = {
            #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
            {
                use wasm_bindgen::JsCast;
                use winit::platform::web::WindowAttributesExtWebSys;

                let canvas = self.canvas_id.as_deref().and_then(|id| {
                    web_sys::window()
                        .and_then(|w| w.document())
                        .and_then(|d| d.get_element_by_id(id))
                        .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
                });

                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title(self.window_config.title.clone())
                            .with_inner_size(winit::dpi::LogicalSize::new(w, h))
                            .with_resizable(self.window_config.resizable)
                            .with_canvas(canvas),
                    )
                    .expect("failed to create window")
            }

            #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
            {
                let mut attributes = Window::default_attributes()
                    .with_title(self.window_config.title.clone())
                    .with_resizable(self.window_config.resizable);

                #[cfg(not(any(target_os = "ios", target_os = "android")))]
                {
                    attributes = attributes.with_inner_size(winit::dpi::LogicalSize::new(w, h));
                }

                event_loop
                    .create_window(attributes)
                    .expect("failed to create window")
            }
        };

        window.set_ime_allowed(true);
        self.scale_factor = window.scale_factor();
        self.context.set_scale_factor(self.scale_factor);
        let size = window.inner_size();
        self.context.set_window_logical_size(
            Pt::from_physical_px(size.width as f64, self.scale_factor),
            Pt::from_physical_px(size.height as f64, self.scale_factor),
        );

        self.window_id = Some(window.id());
        self.window = Some(window);

        #[cfg(all(not(target_arch = "wasm32"), target_os = "android"))]
        {
            let size = size;

            for &backends in platform::PREFERRED_WGPU_BACKENDS {
                self.instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                    backends,
                    ..Default::default()
                });

                let window = self.window.as_ref().expect("window");
                let surface = unsafe {
                    let s = self
                        .instance
                        .create_surface(window)
                        .expect("failed to create surface");
                    std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(s)
                };
                self.surface = Some(surface);

                self.init_state = GraphicsInitState::NotStarted;
                let s = self.surface.as_ref().expect("surface");
                platform::begin_graphics_init(
                    &mut self.init_state,
                    &self.instance,
                    s,
                    size.width,
                    size.height,
                );

                if !matches!(self.init_state, GraphicsInitState::Failed) {
                    break;
                }
                self.surface.take();
            }
        }

        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            let window = self.window.as_ref().expect("window");
            let surface = unsafe {
                let s = self
                    .instance
                    .create_surface(window)
                    .expect("failed to create surface");
                std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(s)
            };
            self.surface = Some(surface);
        }

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            let window = self.window.as_ref().expect("window");
            let surface = unsafe {
                let s = self
                    .instance
                    .create_surface(window)
                    .expect("failed to create surface");
                std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(s)
            };
            self.surface = Some(surface);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let s = self.surface.as_ref().expect("surface");
            platform::begin_graphics_init(
                &mut self.init_state,
                &self.instance,
                s,
                size.width,
                size.height,
            );
        }

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            let s = self.surface.as_ref().expect("surface");
            let instance = self.instance.clone();
            let surface_ptr: *const wgpu::Surface<'static> = s;
            let app_ptr: *mut App = self;
            let w = size.width;
            let h = size.height;

            platform::begin_graphics_init(
                &mut self.init_state,
                instance,
                surface_ptr,
                w,
                h,
                Box::new(move |graphics_r| unsafe {
                    handle_wasm_graphics_init_result(app_ptr, graphics_r)
                }),
            );
        }

        if let Some(window) = self.window.as_ref() {
            if let Some(spot) = self.spot.as_mut() {
                spot.resumed(&mut self.context);
            }
            window.request_redraw();
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // On Android, the underlying native surface can be destroyed when the app is backgrounded.
        // Keep the window, but drop the surface so we recreate/configure it on resume/redraw.
        if let Some(spot) = self.spot.as_mut() {
            spot.suspended(&mut self.context);
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

                if let Some(window) = self.window.as_ref() {
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
                    self.last_physical_size = Some((new_size.width.max(1), new_size.height.max(1)));
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
                // On Android the surface may disappear without a clean suspended/resumed sequence.
                // If we don't have a surface, try to recreate it lazily and schedule another redraw.
                #[cfg(all(not(target_arch = "wasm32"), target_os = "android"))]
                if self.surface.is_none() {
                    if let Some(window) = self.window.as_ref() {
                        let size = window.inner_size();
                        let surface_r = self.instance.create_surface(window);
                        match surface_r {
                            Ok(s) => {
                                let surface = unsafe {
                                    std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(
                                        s,
                                    )
                                };
                                self.surface = Some(surface);

                                if let Some(surface) = self.surface.as_ref() {
                                    with_graphics(|g| g.resize(surface, size.width, size.height));
                                }
                                window.request_redraw();
                                return;
                            }
                            Err(e) => {
                                eprintln!(
                                    "[spot][android][surface] recreate on redraw failed: {:?}",
                                    e
                                );
                                // Surface handle may not be available yet; try again next frame.
                                self.surface.take();
                                window.request_redraw();
                                return;
                            }
                        }
                    }
                }

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

                    if let Some(request) = take_scene_switch_request() {
                        if let Some(payload) = request.payload {
                            self.context
                                .insert_resource_dyn(payload.type_id, payload.value);
                            self.context
                                .insert_resource(Rc::new(ScenePayloadTypeId(payload.type_id)));
                        } else if let Some(last) =
                            self.context.remove_resource::<ScenePayloadTypeId>()
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
                                        if let Some(window) = self.window.as_ref() {
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
                                                        "[spot][android][surface] recreate after error failed: {:?}",
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
                                        if let Some(window) = self.window.as_ref() {
                                            window.request_redraw();
                                        }
                                    }
                                    wgpu::SurfaceError::Other => {
                                        eprintln!("[spot][surface] other error: {:?}", e);
                                        if let Some(window) = self.window.as_ref() {
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

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
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

        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Ensure surface is dropped before window.
        self.surface.take();
        self.window.take();
    }
}
