use android_activity::{AndroidApp, MainEvent, PollEvent};
use crate::{
    Pt, take_quit_request,
    take_scene_switch_request, with_graphics,
    ScenePayloadTypeId,
};
use std::rc::Rc;
use std::time::{Duration, Instant};
use super::App;
use crate::platform;

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
}

#[cfg(feature = "sensors")]
unsafe extern "C" fn sensor_callback(
    _fd: i32,
    _events: i32,
    data: *mut std::ffi::c_void,
) -> i32 {
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
    #[cfg(feature = "sensors")]
    pub(crate) sensor_state: Option<AndroidSensorState>,
}

impl PlatformData {
    pub(crate) fn new() -> Self {
        Self {
            native_window: None,
            floating_surface: None,
            #[cfg(feature = "sensors")]
            sensor_state: None,
        }
    }
}

impl App {
    pub(crate) fn run(&mut self, app: AndroidApp) {
        // Initialize Android-specific features (JVM, Activity, floating window service registration)
        crate::android::init(app.clone());
        
        // Initialize scale factor based on screen density (160 dpi is baseline 1.0)
        self.scale_factor = app.config().density().unwrap_or(160) as f64 / 160.0;
        self.context.set_scale_factor(self.scale_factor);
        
        eprintln!("[spot][android] entering run loop. scale_factor: {}", self.scale_factor);
        
        self.previous = Some(Instant::now());

        loop {
            // Check for new floating surface from JNI
            if let Some(surface_obj) = crate::android::take_floating_surface() {
                let jvm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr() as *mut _) }.unwrap();
                let env = jvm.attach_current_thread().unwrap();
                let surface_ptr = unsafe {
                    ndk_sys::ANativeWindow_fromSurface(env.get_native_interface(), surface_obj.as_obj().as_raw())
                };
                if !surface_ptr.is_null() {
                    let native_window = unsafe { ndk::native_window::NativeWindow::from_ptr(std::ptr::NonNull::new(surface_ptr).unwrap()) };
                    let size = (native_window.width() as u32, native_window.height() as u32);
                    
                    // Force RGBA_8888 for floating window transparency
                    unsafe {
                        ndk_sys::ANativeWindow_setBuffersGeometry(surface_ptr, 0, 0, 1);
                    }

                    match unsafe {
                        self.instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                            raw_display_handle: rwh_06::RawDisplayHandle::Android(rwh_06::AndroidDisplayHandle::new()),
                            raw_window_handle: rwh_06::RawWindowHandle::AndroidNdk({
                                rwh_06::AndroidNdkWindowHandle::new(std::ptr::NonNull::new(surface_ptr as *mut _).unwrap())
                            }),
                        })
                    } {
                        Ok(s) => {
                            eprintln!("[spot][android][floating] surface created successfully");
                            let surface = unsafe { std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(s) };
                            self.platform.floating_surface = Some(surface);
                            if let Some(surface) = self.platform.floating_surface.as_ref() {
                                with_graphics(|g| g.resize(surface, size.0, size.1));
                            }
                            // Also update context size to match floating window for now
                            self.context.set_window_logical_size(
                                Pt::from_physical_px(size.0 as f64, self.scale_factor),
                                Pt::from_physical_px(size.1 as f64, self.scale_factor),
                            );
                        }
                        Err(e) => eprintln!("[spot][android][floating] surface creation failed: {:?}", e),
                    }
                }
            }

            app.poll_events(Some(std::time::Duration::from_millis(0)), |poll_event| {
                match poll_event {
                    PollEvent::Main(MainEvent::InitWindow { .. }) => {
                        eprintln!("[spot][android] InitWindow");
                        self.platform.native_window = app.native_window();
                        if let Some(window) = self.platform.native_window.as_ref() {
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
                                    
                                    // Force RGBA_8888 for better transparency support
                                    unsafe {
                                        ndk_sys::ANativeWindow_setBuffersGeometry(window.ptr().as_ptr() as *mut _, 0, 0, 1);
                                    }

                                    if let Some(surface) = self.surface.as_ref() {
                                        with_graphics(|g| g.resize(surface, size.0, size.1));
                                    }
                                    
                                    self.context.set_window_logical_size(
                                        Pt::from_physical_px(size.0 as f64, self.scale_factor),
                                        Pt::from_physical_px(size.1 as f64, self.scale_factor),
                                    );
                                    
                                    // Initialize graphics if not started
                                    if let platform::GraphicsInitState::Ready(_) = self.init_state {
                                         if let Some(surface) = self.surface.as_ref() {
                                             with_graphics(|g| g.resize(surface, size.0, size.1));
                                         }
                                    } else if let platform::GraphicsInitState::NotStarted = self.init_state {
                                        platform::begin_graphics_init(
                                            &mut self.init_state,
                                            &self.instance,
                                            self.surface.as_ref().unwrap(),
                                            size.0,
                                            size.1,
                                            true, // Force transparency capability for Android
                                        );
                                    }
                                }
                                Err(e) => {
                                    eprintln!("[spot][android] Failed to create surface: {:?}", e);
                                }
                            }
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
                            with_graphics(|g| g.resize(surface, size.0, size.1));
                            self.context.set_window_logical_size(
                                Pt::from_physical_px(size.0 as f64, self.scale_factor),
                                Pt::from_physical_px(size.1 as f64, self.scale_factor),
                            );
                        }
                    }
                    PollEvent::Main(MainEvent::Resume { .. }) => {
                        eprintln!("[spot][android] Resume");
                        self.platform.floating_surface = None;
                        
                        // Switch back to original scene only if graphics are ready
                        if with_graphics(|_| ()).is_some() {
                            if let Some(spot) = self.spot.take() {
                                spot.remove();
                            }
                            self.spot = Some((self.scene_factory)(&mut self.context));
                        }

                        if let Some(service_class) = crate::android::floating_window_service_class() {
                            crate::android::stop_service(service_class);
                        }
                        if let Some(spot) = self.spot.as_mut() {
                            spot.resumed(&mut self.context);
                        }
                        #[cfg(feature = "sensors")]
                        self.init_sensors();
                    }
                    PollEvent::Main(MainEvent::Pause) => {
                        eprintln!("[spot][android] Pause");

                        // Switch to floating scene if registered and graphics are ready
                        if with_graphics(|_| ()).is_some() {
                            if let Some(factory) = crate::android::get_floating_scene_factory() {
                                if let Some(spot) = self.spot.take() {
                                    spot.remove();
                                }
                                self.spot = Some(factory(&mut self.context));
                            }
                        }

                        if let Some(service_class) = crate::android::floating_window_service_class() {
                            crate::android::start_service(service_class);
                        }
                        if let Some(spot) = self.spot.as_mut() {
                            spot.suspended(&mut self.context);
                        }
                        #[cfg(feature = "sensors")]
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

            // Sensor polling if enabled
            #[cfg(feature = "sensors")]
            if let Some(state) = self.platform.sensor_state.as_ref() {
                unsafe {
                    let events = {
                        let mut buffer = state.event_buffer.lock().unwrap();
                        std::mem::take(&mut *buffer)
                    };
                    for event in events {
                        match event.type_ {
                            1 => { // ASENSOR_TYPE_ACCELEROMETER
                                let x = event.__bindgen_anon_1.__bindgen_anon_1.vector.__bindgen_anon_1.__bindgen_anon_1.x;
                                let y = event.__bindgen_anon_1.__bindgen_anon_1.vector.__bindgen_anon_1.__bindgen_anon_1.y;
                                let z = event.__bindgen_anon_1.__bindgen_anon_1.vector.__bindgen_anon_1.__bindgen_anon_1.z;
                                self.context.input_mut().handle_accelerometer(x, y, z);
                            }
                            2 => { // ASENSOR_TYPE_MAGNETIC_FIELD
                                let x = event.__bindgen_anon_1.__bindgen_anon_1.vector.__bindgen_anon_1.__bindgen_anon_1.x;
                                let y = event.__bindgen_anon_1.__bindgen_anon_1.vector.__bindgen_anon_1.__bindgen_anon_1.y;
                                let z = event.__bindgen_anon_1.__bindgen_anon_1.vector.__bindgen_anon_1.__bindgen_anon_1.z;
                                self.context.input_mut().handle_magnetometer(x, y, z);
                            }
                            4 => { // ASENSOR_TYPE_GYROSCOPE
                                let x = event.__bindgen_anon_1.__bindgen_anon_1.vector.__bindgen_anon_1.__bindgen_anon_1.x;
                                let y = event.__bindgen_anon_1.__bindgen_anon_1.vector.__bindgen_anon_1.__bindgen_anon_1.y;
                                let z = event.__bindgen_anon_1.__bindgen_anon_1.vector.__bindgen_anon_1.__bindgen_anon_1.z;
                                self.context.input_mut().handle_gyroscope(x, y, z);
                            }
                            11 => { // ASENSOR_TYPE_ROTATION_VECTOR
                                let x = event.__bindgen_anon_1.__bindgen_anon_1.vector.__bindgen_anon_1.__bindgen_anon_1.x;
                                let y = event.__bindgen_anon_1.__bindgen_anon_1.vector.__bindgen_anon_1.__bindgen_anon_1.y;
                                let z = event.__bindgen_anon_1.__bindgen_anon_1.vector.__bindgen_anon_1.__bindgen_anon_1.z;
                                let w = event.__bindgen_anon_1.__bindgen_anon_1.data[3]; 
                                self.context.input_mut().handle_rotation(x, y, z, w);
                            }
                            17 => { // ASENSOR_TYPE_STEP_DETECTOR
                                self.context.input_mut().handle_step_detector();
                            }
                            18 => { // ASENSOR_TYPE_STEP_COUNTER
                                let count = event.__bindgen_anon_1.__bindgen_anon_1.data[0];
                                self.context.input_mut().handle_step_counter(count);
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Fixed update loop
            let now = Instant::now();
            if let Some(previous) = self.previous.replace(now) {
                let elapsed = now.duration_since(previous);
                self.lag = self.lag.saturating_add(elapsed);

                let mut updates = 0;
                while self.lag >= self.fixed_dt && updates < 4 {
                    if let Some(spot) = self.spot.as_mut() {
                        spot.update(&mut self.context, self.fixed_dt);
                    }
                    self.context.input_mut().end_frame();
                    self.lag = self.lag.saturating_sub(self.fixed_dt);
                    updates += 1;
                }
                if self.lag >= self.fixed_dt {
                    // Spiral of death prevention
                    self.lag = std::time::Duration::from_millis(0);
                }
            }

            // Draw
            if self.spot.is_some() {
                // Initialize frame context
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
                    
                    // Re-draw with the new spot immediately if possible
                    if let Some(spot) = self.spot.as_mut() {
                        self.context.begin_frame();
                        spot.draw(&mut self.context);
                    }
                }

                // Render to ACTIVE surface
                // If floating surface exists, we are in floating mode, prioritize it.
                if let Some(surface) = self.platform.floating_surface.as_ref() {
                    let _ = with_graphics(|g| g.draw_context(surface, &self.context));
                } else if let Some(surface) = self.surface.as_ref() {
                    let _ = with_graphics(|g| g.draw_context(surface, &self.context));
                }

                // Force Android driver to reclaim memory by polling with Wait.
                // This addresses the memory leak (~0.8MB/10s at 60FPS) observed on Android
                // even with minimal rendering.
                with_graphics(|g| g.poll_device(true));

                // Throttle to 60 FPS to prevent driver-level memory growth due to high-frequency acquire calls
                let frame_time = Duration::from_micros(16666);
                let elapsed = now.elapsed();
                if elapsed < frame_time {
                    std::thread::sleep(frame_time - elapsed);
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
                let step_detector = ndk_sys::ASensorManager_getDefaultSensor(manager, 17);
                let step_counter = ndk_sys::ASensorManager_getDefaultSensor(manager, 18);

                let sensor_state = AndroidSensorState {
                    _manager: manager,
                    queue: std::ptr::null_mut(),
                    gyro,
                    accel,
                    mag,
                    rot,
                    step_counter,
                    step_detector,
                    event_buffer: std::sync::Arc::new(std::sync::Mutex::new(Vec::with_capacity(32))),
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
                    self.platform.sensor_state = None;
                    return;
                }
                state_ref.queue = queue;
            }

            if let Some(state) = self.platform.sensor_state.as_ref() {
                if !state.accel.is_null() {
                    ndk_sys::ASensorEventQueue_enableSensor(state.queue, state.accel);
                    ndk_sys::ASensorEventQueue_setEventRate(state.queue, state.accel, 20_000);
                }
                if !state.mag.is_null() {
                    ndk_sys::ASensorEventQueue_enableSensor(state.queue, state.mag);
                    ndk_sys::ASensorEventQueue_setEventRate(state.queue, state.mag, 20_000);
                }
                if !state.gyro.is_null() {
                    ndk_sys::ASensorEventQueue_enableSensor(state.queue, state.gyro);
                    ndk_sys::ASensorEventQueue_setEventRate(state.queue, state.gyro, 20_000);
                }
                if !state.rot.is_null() {
                    ndk_sys::ASensorEventQueue_enableSensor(state.queue, state.rot);
                    ndk_sys::ASensorEventQueue_setEventRate(state.queue, state.rot, 20_000);
                }
                if !state.step_counter.is_null() {
                    ndk_sys::ASensorEventQueue_enableSensor(state.queue, state.step_counter);
                }
                if !state.step_detector.is_null() {
                    ndk_sys::ASensorEventQueue_enableSensor(state.queue, state.step_detector);
                }
            }
        }
    }

    #[cfg(feature = "sensors")]
    pub(crate) fn disable_sensors(&mut self) {
        unsafe {
            if let Some(state) = self.platform.sensor_state.as_ref() {
                if !state.accel.is_null() { ndk_sys::ASensorEventQueue_disableSensor(state.queue, state.accel); }
                if !state.mag.is_null() { ndk_sys::ASensorEventQueue_disableSensor(state.queue, state.mag); }
                if !state.gyro.is_null() { ndk_sys::ASensorEventQueue_disableSensor(state.queue, state.gyro); }
                if !state.rot.is_null() { ndk_sys::ASensorEventQueue_disableSensor(state.queue, state.rot); }
                if !state.step_counter.is_null() { ndk_sys::ASensorEventQueue_disableSensor(state.queue, state.step_counter); }
                if !state.step_detector.is_null() { ndk_sys::ASensorEventQueue_disableSensor(state.queue, state.step_detector); }
            }
        }
    }
}
