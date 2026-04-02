use super::App;
use crate::Pt;
use crate::platform;
use crate::scenes::take_quit_request;
use android_activity::{AndroidApp, MainEvent, PollEvent};
use std::time::{Duration, Instant};

#[cfg(feature = "sensors")]
pub(crate) struct AndroidSensorState {
    pub(crate) _manager: *mut ndk_sys::ASensorManager,
    pub(crate) queue: *mut ndk_sys::ASensorEventQueue,
    pub(crate) gyro: *const ndk_sys::ASensor,
    pub(crate) accel: *const ndk_sys::ASensor,
    pub(crate) mag: *const ndk_sys::ASensor,
    pub(crate) rot: *const ndk_sys::ASensor,
    pub(crate) step_counter: *const ndk_sys::ASensor,
    pub(crate) step_detector: *const ndk_sys::ASensor,
    pub(crate) event_buffer: std::sync::Arc<std::sync::Mutex<Vec<ndk_sys::ASensorEvent>>>,
    pub(crate) last_local_day: u64,
    pub(crate) today_step_total: u64,
    pub(crate) yesterday_step_total: u64,
    pub(crate) last_hardware_count: Option<u64>,
    pub(crate) internal_data_path: std::path::PathBuf,
}

#[cfg(feature = "sensors")]
unsafe extern "C" fn sensor_callback(_fd: i32, _events: i32, data: *mut std::ffi::c_void) -> i32 {
    let state = unsafe { &*(data as *const AndroidSensorState) };
    let mut buffer = state.event_buffer.lock().unwrap();
    unsafe {
        let mut event = std::mem::zeroed::<ndk_sys::ASensorEvent>();
        while ndk_sys::ASensorEventQueue_getEvents(state.queue, &mut event, 1) > 0 {
            buffer.push(event);
        }
    }
    1 // Continue receiving callbacks
}

pub(crate) struct PlatformData {
    pub(crate) native_window: Option<ndk::native_window::NativeWindow>,
    pub(crate) floating_surface: Option<wgpu::Surface<'static>>,
    pub(crate) redraw_requested: bool,
    #[cfg(feature = "sensors")]
    pub(crate) sensor_state: Option<AndroidSensorState>,
    pub(crate) internal_data_path: Option<std::path::PathBuf>,
}

impl PlatformData {
    pub(crate) fn new() -> Self {
        Self {
            native_window: None,
            floating_surface: None,
            redraw_requested: false,
            #[cfg(feature = "sensors")]
            sensor_state: None,
            internal_data_path: None,
        }
    }
}

impl App {
    fn request_redraw(&mut self) {
        self.platform.redraw_requested = true;
    }

    #[cfg(feature = "sensors")]
    fn current_local_epoch_day() -> u64 {
        crate::android::current_local_epoch_day().unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() / 86400)
                .unwrap_or(0)
        })
    }

    #[cfg(feature = "sensors")]
    fn persist_step_state(state: &AndroidSensorState) {
        let steps_file = state.internal_data_path.join("steps.txt");
        let content = format!(
            "v2 {} {} {} {}",
            state.last_local_day,
            state.today_step_total,
            state.yesterday_step_total,
            state.last_hardware_count.unwrap_or(0)
        );
        let _ = std::fs::write(steps_file, content);
    }

    #[cfg(feature = "sensors")]
    fn roll_step_day_if_needed(state: &mut AndroidSensorState, current_day: u64) -> bool {
        if state.last_local_day == 0 {
            state.last_local_day = current_day;
            return true;
        }

        if current_day <= state.last_local_day {
            return false;
        }

        let day_gap = current_day - state.last_local_day;
        if day_gap == 1 {
            state.yesterday_step_total = state.today_step_total;
        } else {
            state.yesterday_step_total = 0;
        }
        state.today_step_total = 0;
        state.last_local_day = current_day;
        true
    }

    fn setup_native_window_surface(&mut self, window: &ndk::native_window::NativeWindow) {
        let size = (window.width() as u32, window.height() as u32);
        if size.0 == 0 || size.1 == 0 {
            return;
        }

        eprintln!(
            "[spot][android] Setting up surface for window: {}x{}",
            size.0, size.1
        );

        // Force RGBA_8888 for better transparency support
        unsafe {
            ndk_sys::ANativeWindow_setBuffersGeometry(window.ptr().as_ptr() as *mut _, 0, 0, 1);
        }

        let surface = unsafe {
            self.instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: rwh_06::RawDisplayHandle::Android(
                        rwh_06::AndroidDisplayHandle::new(),
                    ),
                    raw_window_handle: rwh_06::RawWindowHandle::AndroidNdk({
                        let handle = rwh_06::AndroidNdkWindowHandle::new(
                            std::ptr::NonNull::new(window.ptr().as_mut() as *mut _ as *mut _)
                                .unwrap(),
                        );
                        handle
                    }),
                })
                .expect("failed to create surface")
        };

        let surface =
            unsafe { std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(surface) };
        self.surface = Some(surface);

        if let Some(surface) = self.surface.as_ref() {
            // Check if we can reuse an existing graphics device
            if let Some(g) = self.ctx.runtime.graphics.as_mut() {
                // Increment GPU generation to force asset sync after re-init/resume
                self.ctx.registry.gpu_generation += 1;

                g.resize(surface, size.0, size.1);
                eprintln!("[spot][android] Reusing existing graphics device for new surface.");
                // Graphics already initialized and reconfigured, no need for heavy init
                self.init_state = platform::GraphicsInitState::Ready(Box::new(None));

                self.ctx.set_window_logical_size(
                    Pt::from_physical_px(size.0 as f64, self.scale_factor),
                    Pt::from_physical_px(size.1 as f64, self.scale_factor),
                );
                return;
            }
        }

        // Increment GPU generation to force asset sync after re-init
        self.ctx.registry.gpu_generation += 1;

        // If we get here, we don't have a global device yet, so start fresh init
        self.init_state = platform::GraphicsInitState::NotStarted;
        platform::begin_graphics_init(
            &mut self.init_state,
            &self.instance,
            self.surface.as_ref().unwrap(),
            size.0,
            size.1,
            self.window_config.transparent,
        );

        self.ctx.set_window_logical_size(
            Pt::from_physical_px(size.0 as f64, self.scale_factor),
            Pt::from_physical_px(size.1 as f64, self.scale_factor),
        );

        eprintln!("[spot][android] Graphics initialization started for new surface.");
        self.request_redraw();
    }

    pub(crate) fn run(&mut self, app: AndroidApp) {
        // Initialize Android-specific features (JVM, Activity, floating window service registration)
        crate::android::init(app.clone());

        // Initialize scale factor based on screen density (160 dpi is baseline 1.0)
        self.scale_factor = app.config().density().unwrap_or(160) as f64 / 160.0;
        self.ctx.set_scale_factor(self.scale_factor);
        self.platform.internal_data_path = app.internal_data_path();

        eprintln!(
            "[spot][android] entering run loop. scale_factor: {}",
            self.scale_factor
        );

        self.timing.reset();
        self.request_redraw();
        let mut frame_count = 0u64;

        loop {
            let has_drawable_scene = self.scene.has_active_scene()
                && self.ctx.runtime.graphics.is_some()
                && (self.platform.floating_surface.is_some() || self.surface.is_some());
            let poll_timeout = if has_drawable_scene && self.platform.redraw_requested {
                Duration::ZERO
            } else {
                self.timing
                    .next_deadline()
                    .saturating_duration_since(Instant::now())
            };

            // Check for new floating surface from JNI
            if let Some(surface_obj) = crate::android::take_floating_surface() {
                let jvm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr() as *mut _) }.unwrap();
                let env = jvm.attach_current_thread().unwrap();
                let surface_ptr = unsafe {
                    ndk_sys::ANativeWindow_fromSurface(
                        env.get_native_interface(),
                        surface_obj.as_obj().as_raw(),
                    )
                };
                if !surface_ptr.is_null() {
                    let native_window = unsafe {
                        ndk::native_window::NativeWindow::from_ptr(
                            std::ptr::NonNull::new(surface_ptr).unwrap(),
                        )
                    };
                    let size = (native_window.width() as u32, native_window.height() as u32);

                    // Force RGBA_8888 for floating window transparency
                    unsafe {
                        ndk_sys::ANativeWindow_setBuffersGeometry(surface_ptr, 0, 0, 1);
                    }

                    match unsafe {
                        self.instance
                            .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                                raw_display_handle: rwh_06::RawDisplayHandle::Android(
                                    rwh_06::AndroidDisplayHandle::new(),
                                ),
                                raw_window_handle: rwh_06::RawWindowHandle::AndroidNdk({
                                    rwh_06::AndroidNdkWindowHandle::new(
                                        std::ptr::NonNull::new(surface_ptr as *mut _).unwrap(),
                                    )
                                }),
                            })
                    } {
                        Ok(s) => {
                            eprintln!("[spot][android][floating] surface created successfully");
                            let surface = unsafe {
                                std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(s)
                            };
                            self.platform.floating_surface = Some(surface);
                            if let Some(surface) = self.platform.floating_surface.as_ref() {
                                if let Some(g) = self.ctx.runtime.graphics.as_mut() {
                                    g.resize(surface, size.0, size.1);
                                }
                            }
                            // Also update context size to match floating window for now
                            self.ctx.set_window_logical_size(
                                Pt::from_physical_px(size.0 as f64, self.scale_factor),
                                Pt::from_physical_px(size.1 as f64, self.scale_factor),
                            );
                            self.request_redraw();
                        }
                        Err(e) => {
                            eprintln!("[spot][android][floating] surface creation failed: {:?}", e)
                        }
                    }
                }
            }

            if self.surface.is_none() {
                if let Some(window) = app.native_window() {
                    let size = (window.width() as u32, window.height() as u32);
                    if size.0 > 0 && size.1 > 0 {
                        eprintln!(
                            "[spot][android] Recovering missing surface from current native window: {}x{}",
                            size.0, size.1
                        );
                        self.platform.native_window = Some(window.clone());
                        self.setup_native_window_surface(&window);
                    }
                }
            }

            app.poll_events(Some(poll_timeout), |poll_event| {
                match poll_event {
                    PollEvent::Main(MainEvent::InitWindow { .. }) => {
                        self.platform.native_window = app.native_window();
                        if let Some(window) = self.platform.native_window.clone() {
                            let size = (window.width(), window.height());
                            eprintln!("[spot][android] InitWindow: {}x{}", size.0, size.1);
                            self.setup_native_window_surface(&window);
                        }
                    }
                    PollEvent::Main(MainEvent::TerminateWindow { .. }) => {
                        eprintln!("[spot][android] TerminateWindow");
                        self.surface.take();
                        self.platform.native_window.take();
                    }
                    PollEvent::Main(MainEvent::WindowResized { .. }) => {
                        if let (Some(surface), Some(window)) = (self.surface.as_ref(), self.platform.native_window.as_ref()) {
                            let size = (window.width() as u32, window.height() as u32);
                            eprintln!("[spot][android] WindowResized: {}x{}", size.0, size.1);
                            if size.0 > 0 && size.1 > 0 {
                                if let Some(g) = self.ctx.runtime.graphics.as_mut() {
                                    g.resize(surface, size.0, size.1);
                                }
                                self.ctx.set_window_logical_size(
                                    Pt::from_physical_px(size.0 as f64, self.scale_factor),
                                    Pt::from_physical_px(size.1 as f64, self.scale_factor),
                                );
                                self.request_redraw();
                            }
                        }
                    }
                    PollEvent::Main(MainEvent::Resume { .. }) => {
                        eprintln!("[spot][android] Resume");
                        self.platform.floating_surface = None;

                        // IMPORTANT: On Android, the native window might be same pointer but its 
                        // buffers could be reset or it might need format re-setting after wake up.
                        if let Some(window) = self.platform.native_window.clone() {
                            unsafe {
                                ndk_sys::ANativeWindow_setBuffersGeometry(window.ptr().as_ptr() as *mut _, 0, 0, 1);
                            }

                            // Recreate the surface to ensure it's fresh and matched to the resumed window state.
                            // This addresses the "occasional blank screen" issue after a few frames.
                            eprintln!("[spot][android] Re-creating surface on resume to ensure stability");
                            self.setup_native_window_surface(&window);

                            // Reset previous time to avoid huge dt jump after sleep
                            self.timing.reset();
                        } else {
                            eprintln!("[spot][android] Resume: No native window available. Waiting for InitWindow.");
                        }

                        if let Some(service_class) = crate::android::floating_window_service_class() {
                            crate::android::stop_service(service_class);
                        }

                        if self.ctx.runtime.audio.is_none() {
                            match crate::audio::AudioSystem::new() {
                                Ok(audio) => self.ctx.runtime.audio = Some(audio),
                                Err(e) => eprintln!("[spot][android][audio] initialization failed: {:?}", e),
                            }
                        }

                        if let Some(spot) = self.scene.spot_mut() {
                            spot.resumed(&mut self.ctx);
                        }
                        self.request_redraw();
                        #[cfg(feature = "sensors")]
                        self.init_sensors();
                    }
                    PollEvent::Main(MainEvent::Pause) => {
                        eprintln!("[spot][android] Pause");

                        // Switch to floating scene if registered and graphics are ready
                        if self.ctx.runtime.graphics.is_some() {
                            if let Some(factory) = crate::android::get_floating_scene_factory() {
                                self.scene.remove_current(&mut self.ctx);
                                self.scene.set_active_scene(factory(&mut self.ctx));
                                self.scene.mark_floating();
                            }
                        }

                        if let Some(service_class) = crate::android::floating_window_service_class() {
                            crate::android::start_service(service_class);
                        }
                        if let Some(spot) = self.scene.spot_mut() {
                            spot.suspended(&mut self.ctx);
                        }
                        #[cfg(feature = "sensors")]
                        self.disable_high_frequency_sensors();
                    }
                    PollEvent::Main(MainEvent::ConfigChanged { .. }) => {
                        self.scale_factor = app.config().density().unwrap_or(160) as f64 / 160.0;
                        self.ctx.set_scale_factor(self.scale_factor);
                        eprintln!("[spot][android] ConfigChanged scale_factor: {}", self.scale_factor);
                        self.request_redraw();
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

                                            self.ctx.input_mut().handle_touch_raw(id, (x, y), phase);
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

            // Smart Scene Restoration
            // We recreate the spot IF:
            // a) New graphics device was just created (must reload assets).
            // b) We were using a floating scene (must restore main scene).
            // c) The spot is missing.
            if self.surface.is_some() {
                let graphics_opt = platform::finalize_graphics(&mut self.init_state);
                let graphics_finalized = graphics_opt.is_some();
                let was_floating_scene = self.scene.is_floating_scene();
                if let Some(g) = graphics_opt {
                    self.ctx.runtime.graphics = Some(g);
                }
                if graphics_finalized || was_floating_scene || self.scene.needs_initial_scene() {
                    if self.ctx.runtime.graphics.is_some() {
                        self.scene.restore_root_scene(&mut self.ctx);

                        if graphics_finalized {
                            eprintln!("[spot][android] Scene recreated for new graphics device.");
                        } else if was_floating_scene {
                            eprintln!("[spot][android] Main scene restored from floating state.");
                        } else {
                            eprintln!("[spot][android] Scene initialized.");
                        }

                        self.scene.clear_floating();
                        self.timing.reset();
                        self.request_redraw();
                    }
                }
            }

            // Sensor polling if enabled
            #[cfg(feature = "sensors")]
            if self.platform.sensor_state.is_some() {
                let current_day = Self::current_local_epoch_day();
                if let Some(state) = self.platform.sensor_state.as_mut() {
                    let rolled_day = Self::roll_step_day_if_needed(state, current_day);
                    if rolled_day {
                        Self::persist_step_state(state);
                    }
                    self.ctx
                        .input_mut()
                        .handle_step_counter(state.today_step_total as f32);
                    self.ctx
                        .input_mut()
                        .handle_yesterday_step_counter(state.yesterday_step_total as f32);
                }

                let events = {
                    let state = self.platform.sensor_state.as_ref().unwrap();
                    let mut buffer = state.event_buffer.lock().unwrap();
                    std::mem::take(&mut *buffer)
                };

                unsafe {
                    for event in events {
                        match event.type_ {
                            1 => {
                                // ASENSOR_TYPE_ACCELEROMETER
                                let x = event
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .vector
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .x;
                                let y = event
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .vector
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .y;
                                let z = event
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .vector
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .z;
                                self.ctx.input_mut().handle_accelerometer(x, y, z);
                            }
                            2 => {
                                // ASENSOR_TYPE_MAGNETIC_FIELD
                                let x = event
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .vector
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .x;
                                let y = event
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .vector
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .y;
                                let z = event
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .vector
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .z;
                                self.ctx.input_mut().handle_magnetometer(x, y, z);
                            }
                            4 => {
                                // ASENSOR_TYPE_GYROSCOPE
                                let x = event
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .vector
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .x;
                                let y = event
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .vector
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .y;
                                let z = event
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .vector
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .z;
                                self.ctx.input_mut().handle_gyroscope(x, y, z);
                            }
                            11 => {
                                // ASENSOR_TYPE_ROTATION_VECTOR
                                let x = event
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .vector
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .x;
                                let y = event
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .vector
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .y;
                                let z = event
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .vector
                                    .__bindgen_anon_1
                                    .__bindgen_anon_1
                                    .z;
                                let w = event.__bindgen_anon_1.__bindgen_anon_1.data[3];
                                self.ctx.input_mut().handle_rotation(x, y, z, w);
                            }
                            x if x == ndk_sys::ASENSOR_TYPE_STEP_DETECTOR as i32 => {
                                self.ctx.input_mut().handle_step_detector();
                            }
                            x if x == ndk_sys::ASENSOR_TYPE_STEP_COUNTER as i32 => {
                                let count = event.__bindgen_anon_1.u64_.step_counter;
                                eprintln!("[spot][android] Step counter event: count={}", count);

                                let state = self.platform.sensor_state.as_mut().unwrap();
                                let current_day = Self::current_local_epoch_day();
                                let rolled_day = Self::roll_step_day_if_needed(state, current_day);

                                let delta = match state.last_hardware_count {
                                    Some(previous) if count >= previous => count - previous,
                                    Some(previous) => {
                                        eprintln!(
                                            "[spot][android] Step counter reset detected: previous={} new={}",
                                            previous,
                                            count
                                        );
                                        0
                                    }
                                    None => {
                                        eprintln!(
                                            "[spot][android] Establishing initial step counter at {}",
                                            count
                                        );
                                        0
                                    }
                                };

                                if delta > 0 {
                                    state.today_step_total =
                                        state.today_step_total.saturating_add(delta);
                                }
                                state.last_hardware_count = Some(count);

                                if delta > 0 || rolled_day {
                                    eprintln!(
                                        "[spot][android] Updated daily steps: current_day={} today={} yesterday={} delta={} last_count={}",
                                        current_day,
                                        state.today_step_total,
                                        state.yesterday_step_total,
                                        delta,
                                        count
                                    );
                                }
                                Self::persist_step_state(state);
                                eprintln!(
                                    "[spot][android] Computed steps_today={} yesterday_steps={} using count={}",
                                    state.today_step_total,
                                    state.yesterday_step_total,
                                    count,
                                );
                                self.ctx
                                    .input_mut()
                                    .handle_step_counter(state.today_step_total as f32);
                                self.ctx
                                    .input_mut()
                                    .handle_yesterday_step_counter(state.yesterday_step_total as f32);
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Fixed update loop
            let updates = self.timing.run_updates(4, |dt| {
                if let Some(spot) = self.scene.spot_mut() {
                    spot.update(&mut self.ctx, dt);
                }
                self.ctx.input_mut().end_frame();
            });
            if updates > 0 {
                self.request_redraw();
            }

            // Draw
            if self.scene.has_active_scene() && self.platform.redraw_requested {
                self.platform.redraw_requested = false;
                // Initialize frame context
                self.ctx.begin_frame();
                if let Some(spot) = self.scene.spot_mut() {
                    spot.draw(&mut self.ctx);
                }

                // Handle scene switch
                if self.scene.apply_pending_switch(&mut self.ctx) {
                    if let Some(spot) = self.scene.spot_mut() {
                        self.ctx.begin_frame();
                        spot.draw(&mut self.ctx);
                    }
                }

                // Render to ACTIVE surface
                // If floating surface exists, we are in floating mode, prioritize it.
                let mut graphics = self.ctx.runtime.graphics.take();
                let draw_result = if let Some(surface) = self.platform.floating_surface.as_ref() {
                    graphics
                        .as_mut()
                        .map(|g| g.draw_context(surface, &mut self.ctx))
                } else if let Some(surface) = self.surface.as_ref() {
                    graphics
                        .as_mut()
                        .map(|g| g.draw_context(surface, &mut self.ctx))
                } else {
                    None
                };
                self.ctx.runtime.graphics = graphics;

                if let Some(Err(e)) = draw_result {
                    match e {
                        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                            eprintln!(
                                "[spot][android] Surface error: {:?}. Attempting recovery by re-creating surface.",
                                e
                            );
                            if let Some(window) = self.platform.native_window.clone() {
                                self.setup_native_window_surface(&window);
                            }
                            self.request_redraw();
                        }
                        wgpu::SurfaceError::Timeout => {
                            eprintln!(
                                "[spot][android] Surface acquisition timeout. Frame skipped."
                            );
                            self.request_redraw();
                        }
                        wgpu::SurfaceError::OutOfMemory => {
                            eprintln!("[spot][android] Out of memory error. Surface dropped.");
                            self.surface.take();
                        }
                        _ => {
                            eprintln!("[spot][android] Surface draw error: {:?}", e);
                            self.request_redraw();
                        }
                    }
                } else {
                    self.request_redraw();
                }

                // Force Android driver to reclaim memory by polling with Wait.
                // This addresses the memory leak (~0.8MB/10s at 60FPS) observed on Android
                // even with minimal rendering.
                if let Some(g) = self.ctx.runtime.graphics.as_mut() {
                    g.poll_device(true);
                }

                // Periodic health check log every 300 frames (~5 seconds)
                frame_count += 1;
                if frame_count % 300 == 0 {
                    eprintln!("[spot][android] Loop alive. Frame: {}", frame_count);
                }
            }

            if take_quit_request() {
                break;
            }
        }
    }

    #[cfg(feature = "sensors")]
    pub(crate) fn init_sensors(&mut self) {
        unsafe {
            if self.platform.sensor_state.is_none() {
                let manager = ndk_sys::ASensorManager_getInstance();
                if manager.is_null() {
                    return;
                }

                let accel = ndk_sys::ASensorManager_getDefaultSensor(manager, 1);
                let mag = ndk_sys::ASensorManager_getDefaultSensor(manager, 2);
                let gyro = ndk_sys::ASensorManager_getDefaultSensor(manager, 4);
                let rot = ndk_sys::ASensorManager_getDefaultSensor(manager, 11);
                let step_detector = ndk_sys::ASensorManager_getDefaultSensor(
                    manager,
                    ndk_sys::ASENSOR_TYPE_STEP_DETECTOR as i32,
                );
                let step_counter = ndk_sys::ASensorManager_getDefaultSensor(
                    manager,
                    ndk_sys::ASENSOR_TYPE_STEP_COUNTER as i32,
                );
                if step_detector.is_null() {
                    eprintln!("[spot][android] Step detector sensor unavailable on this device.");
                }
                if step_counter.is_null() {
                    eprintln!("[spot][android] Step counter sensor unavailable on this device.");
                }

                let data_path = self
                    .platform
                    .internal_data_path
                    .clone()
                    .unwrap_or_else(|| std::path::PathBuf::from("/sdcard"));
                let steps_file = data_path.join("steps.txt");

                let mut last_local_day = Self::current_local_epoch_day();
                let mut today_step_total = 0u64;
                let mut yesterday_step_total = 0u64;
                let mut last_hardware_count = None;

                if let Ok(content) = std::fs::read_to_string(&steps_file) {
                    let parts: Vec<&str> = content.split_whitespace().collect();
                    if parts.first() == Some(&"v2") && parts.len() >= 5 {
                        last_local_day = parts[1].parse().unwrap_or(last_local_day);
                        today_step_total = parts[2].parse().unwrap_or(0);
                        yesterday_step_total = parts[3].parse().unwrap_or(0);
                        last_hardware_count = parts[4].parse().ok();
                    } else if parts.len() >= 3 {
                        last_local_day = parts[1].parse().unwrap_or(last_local_day);
                        last_hardware_count = None;
                    }
                }
                eprintln!(
                    "[spot][android] Restored step state: today={} yesterday={} last_day={} last_count={} file={}",
                    today_step_total,
                    yesterday_step_total,
                    last_local_day,
                    last_hardware_count
                        .map(|v: u64| v.to_string())
                        .unwrap_or_else(|| "none".to_string()),
                    steps_file.display()
                );

                let sensor_state = AndroidSensorState {
                    _manager: manager,
                    queue: std::ptr::null_mut(),
                    gyro,
                    accel,
                    mag,
                    rot,
                    step_counter,
                    step_detector,
                    event_buffer: std::sync::Arc::new(std::sync::Mutex::new(Vec::with_capacity(
                        32,
                    ))),
                    last_local_day,
                    today_step_total,
                    yesterday_step_total,
                    last_hardware_count,
                    internal_data_path: data_path,
                };

                let looper = ndk_sys::ALooper_forThread();
                if looper.is_null() {
                    return;
                }

                // We need a stable pointer. Since sensor_state is in an Option in self.platform,
                // and App (self) is pinned in the run loop's stack, it's mostly safe,
                // but let's be careful. Actually, ASensorManager_createEventQueue
                // requires a callback and data.
                // We'll store the state first.
                self.platform.sensor_state = Some(sensor_state);
                let state_ref = self.platform.sensor_state.as_mut().unwrap();

                let queue = ndk_sys::ASensorManager_createEventQueue(
                    manager,
                    looper,
                    ndk_sys::ALOOPER_POLL_CALLBACK as i32,
                    Some(sensor_callback),
                    state_ref as *mut _ as *mut std::ffi::c_void,
                );

                if queue.is_null() {
                    eprintln!("[spot][android] Failed to create sensor event queue");
                    self.platform.sensor_state = None;
                    return;
                }
                state_ref.queue = queue;
                eprintln!("[spot][android] Sensor event queue created successfully");
            }

            if let Some(state) = self.platform.sensor_state.as_ref() {
                if !state.accel.is_null() {
                    let rc = ndk_sys::ASensorEventQueue_enableSensor(state.queue, state.accel);
                    eprintln!("[spot][android] Enable accelerometer rc={}", rc);
                    let rc = ndk_sys::ASensorEventQueue_setEventRate(state.queue, state.accel, 20_000);
                    eprintln!("[spot][android] Accelerometer event rate rc={}", rc);
                }
                if !state.mag.is_null() {
                    let rc = ndk_sys::ASensorEventQueue_enableSensor(state.queue, state.mag);
                    eprintln!("[spot][android] Enable magnetometer rc={}", rc);
                    let rc = ndk_sys::ASensorEventQueue_setEventRate(state.queue, state.mag, 20_000);
                    eprintln!("[spot][android] Magnetometer event rate rc={}", rc);
                }
                if !state.gyro.is_null() {
                    let rc = ndk_sys::ASensorEventQueue_enableSensor(state.queue, state.gyro);
                    eprintln!("[spot][android] Enable gyroscope rc={}", rc);
                    let rc = ndk_sys::ASensorEventQueue_setEventRate(state.queue, state.gyro, 20_000);
                    eprintln!("[spot][android] Gyroscope event rate rc={}", rc);
                }
                if !state.rot.is_null() {
                    let rc = ndk_sys::ASensorEventQueue_enableSensor(state.queue, state.rot);
                    eprintln!("[spot][android] Enable rotation vector rc={}", rc);
                    let rc = ndk_sys::ASensorEventQueue_setEventRate(state.queue, state.rot, 20_000);
                    eprintln!("[spot][android] Rotation vector event rate rc={}", rc);
                }
                if !state.step_counter.is_null() {
                    let rc =
                        ndk_sys::ASensorEventQueue_enableSensor(state.queue, state.step_counter);
                    eprintln!("[spot][android] Enable step counter rc={}", rc);
                }
                if !state.step_detector.is_null() {
                    let rc =
                        ndk_sys::ASensorEventQueue_enableSensor(state.queue, state.step_detector);
                    eprintln!("[spot][android] Enable step detector rc={}", rc);
                }
            }
        }
    }

    #[cfg(feature = "sensors")]
    pub(crate) fn disable_high_frequency_sensors(&mut self) {
        unsafe {
            if let Some(state) = self.platform.sensor_state.as_ref() {
                if !state.accel.is_null() {
                    ndk_sys::ASensorEventQueue_disableSensor(state.queue, state.accel);
                }
                if !state.mag.is_null() {
                    ndk_sys::ASensorEventQueue_disableSensor(state.queue, state.mag);
                }
                if !state.gyro.is_null() {
                    ndk_sys::ASensorEventQueue_disableSensor(state.queue, state.gyro);
                }
                if !state.rot.is_null() {
                    ndk_sys::ASensorEventQueue_disableSensor(state.queue, state.rot);
                }
                Self::persist_step_state(state);
            }
        }
    }
}
