#[cfg(not(target_os = "android"))]
use winit::application::ApplicationHandler;
#[cfg(not(target_os = "android"))]
use winit::event::WindowEvent;
#[cfg(not(target_os = "android"))]
use winit::event_loop::{ActiveEventLoop, ControlFlow};
#[cfg(not(target_os = "android"))]
use winit::window::{Window, WindowId};

#[cfg(target_os = "android")]
use android_activity::{AndroidApp, MainEvent, PollEvent};

use crate::platform;
use crate::{
    Context, Pt, ScenePayloadTypeId, Spot, WindowConfig, take_quit_request,
    take_scene_switch_request, with_graphics,
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
            web_sys::console::log_1(&"[spot][wasm] Graphics initialized successfully".into());
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
    #[cfg(not(target_os = "android"))]
    window: Option<Window>,
    #[cfg(not(target_os = "android"))]
    window_id: Option<WindowId>,
    #[cfg(target_os = "android")]
    native_window: Option<ndk::native_window::NativeWindow>,
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
    #[cfg(all(target_os = "android", feature = "gyroscope"))]
    sensor_state: Option<AndroidSensorState>,
}

#[cfg(all(target_os = "android", feature = "gyroscope"))]
struct AndroidSensorState {
    manager: *mut ndk_sys::ASensorManager,
    queue: *mut ndk_sys::ASensorEventQueue,
    gyro: *const ndk_sys::ASensor,
}

impl App {
    pub(crate) fn new<T: Spot + 'static>(window_config: WindowConfig) -> Self {
        let instance = platform::create_wgpu_instance();
        let audio = crate::audio::AudioSystem::new().expect("failed to initialize audio system");
        let _ = platform::set_global_audio(audio);

        Self {
            #[cfg(not(target_os = "android"))]
            window: None,
            #[cfg(not(target_os = "android"))]
            window_id: None,
            #[cfg(target_os = "android")]
            native_window: None,
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
            #[cfg(all(target_os = "android", feature = "gyroscope"))]
            sensor_state: None,
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
            #[cfg(not(target_os = "android"))]
            window: None,
            #[cfg(not(target_os = "android"))]
            window_id: None,
            #[cfg(target_os = "android")]
            native_window: None,
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

    #[cfg(all(target_os = "android", feature = "gyroscope"))]
    fn init_sensors(&mut self) {
        unsafe {
            if self.sensor_state.is_none() {
                let manager = ndk_sys::ASensorManager_getInstance();
                if manager.is_null() {
                    return;
                }

                // ASENSOR_TYPE_GYROSCOPE = 4
                let gyro = ndk_sys::ASensorManager_getDefaultSensor(manager, 4);
                if gyro.is_null() {
                    return;
                }

                // Create a looper-less event queue (null looper)
                // However, ASensorManager_createEventQueue requires a looper in some versions?
                // Actually, if we use a non-looper queue, we can poll it.
                // But the easiest way is to use the current thread's looper if it exists.
                let looper = ndk_sys::ALooper_forThread();
                if looper.is_null() {
                    return;
                }

                let queue = ndk_sys::ASensorManager_createEventQueue(
                    manager,
                    looper,
                    ndk_sys::ALOOPER_POLL_CALLBACK as i32,
                    None,
                    std::ptr::null_mut(),
                );

                if queue.is_null() {
                    return;
                }

                self.sensor_state = Some(AndroidSensorState {
                    manager,
                    queue,
                    gyro,
                });
            }

            if let Some(state) = self.sensor_state.as_ref() {
                ndk_sys::ASensorEventQueue_enableSensor(state.queue, state.gyro);
                ndk_sys::ASensorEventQueue_setEventRate(state.queue, state.gyro, 20_000); // 50Hz
            }
        }
    }

    #[cfg(all(target_os = "android", feature = "gyroscope"))]
    fn disable_sensors(&mut self) {
        unsafe {
            if let Some(state) = self.sensor_state.as_ref() {
                ndk_sys::ASensorEventQueue_disableSensor(state.queue, state.gyro);
            }
        }
    }

    #[cfg(target_os = "android")]
    pub(crate) fn run_android(&mut self, app: AndroidApp) {
        use std::time::Instant;
        
        // Initialize scale factor based on screen density (160 dpi is baseline 1.0)
        self.scale_factor = app.config().density().unwrap_or(160) as f64 / 160.0;
        self.context.set_scale_factor(self.scale_factor);
        
        eprintln!("[spot][android] entering run_android loop. scale_factor: {}", self.scale_factor);
        
        self.previous = Some(Instant::now());

        loop {
            app.poll_events(Some(std::time::Duration::from_millis(0)), |poll_event| {
                match poll_event {
                    PollEvent::Main(MainEvent::InitWindow { .. }) => {
                        eprintln!("[spot][android] InitWindow");
                        self.native_window = app.native_window();
                        if let Some(window) = self.native_window.as_ref() {
                            let size = (window.width() as u32, window.height() as u32);
                            
                            // Re-enable surface creation using unsafe so we don't need trait bounds for native window
                            match unsafe {
                                self.instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                                    raw_display_handle: rwh_06::RawDisplayHandle::Android(rwh_06::AndroidDisplayHandle::new()),
                                    raw_window_handle: rwh_06::RawWindowHandle::AndroidNdk({
                                        let handle = rwh_06::AndroidNdkWindowHandle::new(std::ptr::NonNull::new(window.ptr().as_mut() as *mut _ as *mut _).unwrap());
                                        handle
                                    }),
                                })
                            } {
                                Ok(s) => {
                                    let surface = unsafe {
                                        std::mem::transmute::<
                                            wgpu::Surface<'_>,
                                            wgpu::Surface<'static>,
                                        >(s)
                                    };
                                    self.surface = Some(surface);

                                    if let Some(surface) = self.surface.as_ref() {
                                        with_graphics(|g| g.resize(surface, size.0, size.1));
                                    }
                                    
                                    self.context.set_window_logical_size(
                                        Pt::from_physical_px(size.0 as f64, self.scale_factor),
                                        Pt::from_physical_px(size.1 as f64, self.scale_factor),
                                    );
                                    
                                    // Initialize graphics if not started
                                    if let GraphicsInitState::Ready(_) = self.init_state {
                                         if let Some(surface) = self.surface.as_ref() {
                                             with_graphics(|g| g.resize(surface, size.0, size.1));
                                         }
                                    } else if let GraphicsInitState::NotStarted = self.init_state {
                                        platform::begin_graphics_init(
                                            &mut self.init_state,
                                            &self.instance,
                                            self.surface.as_ref().unwrap(),
                                            size.0,
                                            size.1,
                                        );
                                    }
                                    self.context.set_window_logical_size(
                                        Pt::from_physical_px(size.0 as f64, self.scale_factor),
                                        Pt::from_physical_px(size.1 as f64, self.scale_factor),
                                    );
                                }
                                Err(e) => {
                                    eprintln!("[spot][android][surface] creation failed: {:?}", e);
                                }
                            }
                        }
                    }
                    PollEvent::Main(MainEvent::TerminateWindow { .. }) => {
                        eprintln!("[spot][android] TerminateWindow");
                        self.surface.take();
                        self.native_window.take();
                    }
                    PollEvent::Main(MainEvent::WindowResized { .. }) => {
                        if let (Some(surface), Some(window)) = (self.surface.as_ref(), self.native_window.as_ref()) {
                            let size = (window.width() as u32, window.height() as u32);
                            eprintln!("[spot][android] WindowResized: {}x{}", size.0, size.1);
                            with_graphics(|g| g.resize(surface, size.0, size.1));
                            self.context.set_window_logical_size(
                                Pt::from_physical_px(size.0 as f64, self.scale_factor),
                                Pt::from_physical_px(size.1 as f64, self.scale_factor),
                            );
                        }
                    }
                    PollEvent::Main(MainEvent::Resume { .. }) => {
                        eprintln!("[spot][android] Resume");
                        if let Some(spot) = self.spot.as_mut() {
                            spot.resumed(&mut self.context);
                        }
                        #[cfg(all(target_os = "android", feature = "gyroscope"))]
                        self.init_sensors();
                    }
                    PollEvent::Main(MainEvent::Pause) => {
                        eprintln!("[spot][android] Pause");
                        if let Some(spot) = self.spot.as_mut() {
                            spot.suspended(&mut self.context);
                        }
                        #[cfg(all(target_os = "android", feature = "gyroscope"))]
                        self.disable_sensors();
                    }
                    PollEvent::Main(MainEvent::ConfigChanged { .. }) => {
                        self.scale_factor = app.config().density().unwrap_or(160) as f64 / 160.0;
                        self.context.set_scale_factor(self.scale_factor);
                        eprintln!("[spot][android] ConfigChanged scale_factor: {}", self.scale_factor);
                    }
                    PollEvent::Main(MainEvent::Destroy) => {
                        eprintln!("[spot][android] Destroy");
                        return;
                    }
                    PollEvent::Main(MainEvent::InputAvailable) => {
                        if let Ok(mut iter) = app.input_events_iter() {
                            loop {
                                let read = iter.next(|event| {
                                    match event {
                                        android_activity::input::InputEvent::MotionEvent(motion_event) => {
                                            let action = motion_event.action();
                                            let (pointer_index, phase) = match action {
                                                android_activity::input::MotionAction::Down => (0, crate::TouchPhase::Started),
                                                android_activity::input::MotionAction::PointerDown => (motion_event.pointer_index(), crate::TouchPhase::Started),
                                                android_activity::input::MotionAction::Up => (0, crate::TouchPhase::Ended),
                                                android_activity::input::MotionAction::PointerUp => (motion_event.pointer_index(), crate::TouchPhase::Ended),
                                                android_activity::input::MotionAction::Move => (motion_event.pointer_index(), crate::TouchPhase::Moved),
                                                android_activity::input::MotionAction::Cancel => (0, crate::TouchPhase::Cancelled),
                                                _ => return android_activity::InputStatus::Unhandled,
                                            };

                                            let pointer = motion_event.pointer_at_index(pointer_index);
                                            let id = pointer.pointer_id() as u64;
                                            let x = Pt::from_physical_px(pointer.x() as f64, self.scale_factor);
                                            let y = Pt::from_physical_px(pointer.y() as f64, self.scale_factor);
                                            
                                            self.context.input_mut().handle_touch_raw(id, (x, y), phase);
                                            android_activity::InputStatus::Handled
                                        }
                                        _ => android_activity::InputStatus::Unhandled,
                                    }
                                });
                                if !read { break; }
                            }
                        }
                    }
                    _ => {}
                }
            });

            // If we have a surface and graphics is not initialized yet, try to finalize it
            if self.surface.is_some() && self.spot.is_none() {
                if platform::finalize_graphics(&mut self.init_state) {
                    let spot = (self.scene_factory)(&mut self.context);
                    self.spot = Some(spot);
                }
            }

            // Fixed update loop
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

            // Draw
            if let Some(surface) = self.surface.as_ref() {
                if self.spot.is_some() {
                    self.context.begin_frame();
                    if let Some(spot) = self.spot.as_mut() {
                        spot.draw(&mut self.context);
                    }
                    
                    // Handle scene switch
                    if let Some(request) = take_scene_switch_request() {
                        if let Some(payload) = request.payload {
                            self.context.insert_resource_dyn(payload.type_id, payload.value);
                            self.context.insert_resource(Rc::new(ScenePayloadTypeId(payload.type_id)));
                        } else if let Some(last) = self.context.remove_resource::<ScenePayloadTypeId>() {
                            if let Ok(last) = std::rc::Rc::try_unwrap(last) {
                                self.context.remove_resource_dyn(last.0);
                            }
                        }
                        if let Some(old_spot) = self.spot.take() {
                            old_spot.remove();
                        }
                        self.spot = Some((request.factory)(&mut self.context));
                    }

                    // Render
                    let _ = with_graphics(|g| g.draw_context(surface, &self.context));
                }
            }

            if take_quit_request() {
                break;
            }

            // Gyroscope polling if enabled
            #[cfg(all(target_os = "android", feature = "gyroscope"))]
            {
                if let Some(state) = self.sensor_state.as_ref() {
                    unsafe {
                        let mut event = std::mem::zeroed::<ndk_sys::ASensorEvent>();
                        while ndk_sys::ASensorEventQueue_getEvents(state.queue, &mut event, 1) > 0 {
                            if event.type_ == 4 { // ASENSOR_TYPE_GYROSCOPE
                                let x = event.__bindgen_anon_1.__bindgen_anon_1.vector.__bindgen_anon_1.__bindgen_anon_1.x;
                                let y = event.__bindgen_anon_1.__bindgen_anon_1.vector.__bindgen_anon_1.__bindgen_anon_1.y;
                                let z = event.__bindgen_anon_1.__bindgen_anon_1.vector.__bindgen_anon_1.__bindgen_anon_1.z;
                                self.context.input_mut().handle_gyroscope(x, y, z);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(not(target_os = "android"))]
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
        if self.window.is_none() {
            let _w = self.window_config.width.0.max(1.0) as f64;
            let _h = self.window_config.height.0.max(1.0) as f64;

            let attributes = Window::default_attributes()
                .with_title(self.window_config.title.clone())
                .with_resizable(self.window_config.resizable);

            #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
            let attributes = {
                use wasm_bindgen::JsCast;
                use winit::platform::web::WindowAttributesExtWebSys;
                let canvas = self.canvas_id.as_deref().and_then(|id| {
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

            self.window_id = Some(window.id());
            self.window = Some(window);
        }

        // 2. (Re)create surface if needed
        if let Some(window) = self.window.as_ref() {
            if self.surface.is_none() {
                let size = window.inner_size();
                match self.instance.create_surface(window) {
                    Ok(s) => {
                        let surface = unsafe {
                            std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(s)
                        };
                        self.surface = Some(surface);
                        
                        // If graphics is already initialized, resize the surface
                        if let GraphicsInitState::Ready(_) = self.init_state {
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
        if let GraphicsInitState::NotStarted = self.init_state {
            if let Some(surface) = self.surface.as_ref() {
                if let Some(window) = self.window.as_ref() {
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
                                handle_wasm_graphics_init_result(app_ptr, graphics_r)
                            }),
                        );
                    }
                }
            }
        }

        // 4. Handle app-level resume and sensors
        if let Some(spot) = self.spot.as_mut() {
            spot.resumed(&mut self.context);
        }

        #[cfg(all(target_os = "android", feature = "gyroscope"))]
        self.init_sensors();

        if let Some(window) = self.window.as_ref() {
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

        #[cfg(all(target_os = "android", feature = "gyroscope"))]
        self.disable_sensors();
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

                #[cfg(target_os = "android")]
                eprintln!("[spot][android] Window resized: {}x{}, scale_factor: {}", new_size.width, new_size.height, self.scale_factor);

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
                #[cfg(target_os = "android")]
                eprintln!("[spot][android] Touch event: id={:?}, phase={:?}, loc={:?}", touch.id, touch.phase, touch.location);

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

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if take_quit_request() {
            event_loop.exit();
            return;
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

        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }

        #[cfg(all(target_os = "android", feature = "gyroscope"))]
        {
            if let Some(state) = self.sensor_state.as_ref() {
                unsafe {
                    let mut event = std::mem::zeroed::<ndk_sys::ASensorEvent>();
                    while ndk_sys::ASensorEventQueue_getEvents(state.queue, &mut event, 1) > 0 {
                        // ASENSOR_TYPE_GYROSCOPE = 4
                        if event.type_ == 4 {
                            // event.unnamed_field.vector.v is [f32; 3]
                            // but ndk-sys has it as a union.
                            // In 0.6.0 it should be accessible.
                            let x = event.__bindgen_anon_1.__bindgen_anon_1.vector.__bindgen_anon_1.__bindgen_anon_1.x;
                            let y = event.__bindgen_anon_1.__bindgen_anon_1.vector.__bindgen_anon_1.__bindgen_anon_1.y;
                            let z = event.__bindgen_anon_1.__bindgen_anon_1.vector.__bindgen_anon_1.__bindgen_anon_1.z;
                            self.context.input_mut().handle_gyroscope(x, y, z);
                        }
                    }
                }
            }
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Ensure surface is dropped before window.
        self.surface.take();
        self.window.take();
    }
}
