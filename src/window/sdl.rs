use super::App;
use crate::input::InputState;
use crate::platform;
use crate::scenes::take_quit_request;
use crate::touch::TouchPhase;
use crate::{Key, MouseButton, Pt, Spot, WindowConfig};
use sdl3_sys::error::SDL_GetError;
use sdl3_sys::events::*;
use sdl3_sys::init::{SDL_INIT_EVENTS, SDL_INIT_VIDEO, SDL_Init, SDL_Quit};
use sdl3_sys::metal::{SDL_Metal_CreateView, SDL_Metal_DestroyView, SDL_Metal_GetLayer};
use sdl3_sys::timer::SDL_Delay;
use sdl3_sys::video::*;
use std::ffi::{CStr, CString};
use std::sync::atomic::{AtomicU8, Ordering};

const LIFECYCLE_NONE: u8 = 0;
const LIFECYCLE_ENTER_BACKGROUND: u8 = 1;
const LIFECYCLE_ENTER_FOREGROUND: u8 = 2;
const LIFECYCLE_TERMINATING: u8 = 4;

static SDL_LIFECYCLE_PENDING: AtomicU8 = AtomicU8::new(LIFECYCLE_NONE);

pub(crate) struct PlatformData {
    pub(crate) window: *mut SDL_Window,
    pub(crate) metal_view: *mut std::ffi::c_void,
    pub(crate) last_physical_size: Option<(u32, u32)>,
    pub(crate) backgrounded: bool,
    pub(crate) scene_suspended_for_background: bool,
    #[cfg(all(target_os = "ios", feature = "sensors"))]
    pub(crate) sensor_state: Option<super::ios::IosSensorState>,
}

impl PlatformData {
    pub(crate) fn new() -> Self {
        Self {
            window: std::ptr::null_mut(),
            metal_view: std::ptr::null_mut(),
            last_physical_size: None,
            backgrounded: false,
            scene_suspended_for_background: false,
            #[cfg(all(target_os = "ios", feature = "sensors"))]
            sensor_state: None,
        }
    }
}

struct SdlGuard;

impl SdlGuard {
    fn init() -> Self {
        SDL_LIFECYCLE_PENDING.store(LIFECYCLE_NONE, Ordering::SeqCst);
        let ok = unsafe { SDL_Init(SDL_INIT_VIDEO | SDL_INIT_EVENTS) };
        if !ok {
            panic!("SDL_Init failed: {}", sdl_error());
        }

        let watch_ok =
            unsafe { SDL_AddEventWatch(Some(lifecycle_event_watch), std::ptr::null_mut()) };
        if !watch_ok {
            eprintln!("[spot][sdl] SDL_AddEventWatch failed: {}", sdl_error());
        }
        Self
    }
}

impl Drop for SdlGuard {
    fn drop(&mut self) {
        unsafe {
            SDL_RemoveEventWatch(Some(lifecycle_event_watch), std::ptr::null_mut());
            SDL_Quit();
        }
    }
}

pub(crate) fn run_sdl<T: Spot + 'static>(window_config: WindowConfig) {
    let _sdl = SdlGuard::init();
    let mut app = App::new::<T>(window_config);
    app.run_loop();
}

impl App {
    fn run_loop(&mut self) {
        self.create_window();
        self.ensure_audio_initialized();
        self.ensure_surface();
        self.begin_graphics_init_if_needed();
        self.ensure_scene_ready();
        self.timing.reset();

        #[cfg(all(target_os = "ios", feature = "sensors"))]
        {
            if self.platform.sensor_state.is_none() {
                self.platform.sensor_state = Some(super::ios::IosSensorState::new());
            }
            if let Some(state) = self.platform.sensor_state.as_ref() {
                state.enable();
            }
        }

        if let Some(spot) = self.scene.spot_mut() {
            spot.resumed(&mut self.ctx);
        }

        let mut running = true;
        while running {
            running = self.poll_events();
            self.apply_pending_window_requests();
            if take_quit_request() {
                break;
            }

            if !self.apply_lifecycle_pending() {
                break;
            }
            if !self.platform.backgrounded {
                self.restore_foreground_resources();
                if !self.platform.scene_suspended_for_background {
                    self.run_updates();
                    self.draw_frame();
                }
            }
            unsafe { SDL_Delay(1) };
        }

        if !self.platform.scene_suspended_for_background
            && let Some(spot) = self.scene.spot_mut()
        {
            spot.suspended(&mut self.ctx);
        }
        #[cfg(all(target_os = "ios", feature = "sensors"))]
        if let Some(state) = self.platform.sensor_state.as_ref() {
            state.disable();
        }
        self.ctx.clear_transient_input();
        self.ctx.clear_transient_state();
        self.surface.take();
        self.destroy_window();
    }

    fn create_window(&mut self) {
        if !self.platform.window.is_null() {
            return;
        }

        let title = CString::new(self.window_config.title.clone())
            .unwrap_or_else(|_| CString::new("spot").unwrap());
        let width = self.window_config.width.0.max(1.0).round() as i32;
        let height = self.window_config.height.0.max(1.0).round() as i32;
        let mut flags = SDL_WINDOW_HIGH_PIXEL_DENSITY;
        if self.window_config.resizable {
            flags |= SDL_WINDOW_RESIZABLE;
        }
        if self.window_config.fullscreen {
            flags |= SDL_WINDOW_FULLSCREEN;
        }
        if self.window_config.transparent {
            flags |= SDL_WINDOW_TRANSPARENT;
        }
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            flags |= SDL_WINDOW_METAL;
        }

        let window = unsafe { SDL_CreateWindow(title.as_ptr(), width, height, flags) };
        if window.is_null() {
            panic!("SDL_CreateWindow failed: {}", sdl_error());
        }

        self.platform.window = window;
        self.refresh_window_metrics();
        let (w, h) = self.window_physical_size();
        eprintln!("[spot][init] SDL window created: {}x{}", w, h);
    }

    fn destroy_window(&mut self) {
        self.destroy_metal_view();
        if !self.platform.window.is_null() {
            unsafe { SDL_DestroyWindow(self.platform.window) };
            self.platform.window = std::ptr::null_mut();
        }
    }

    fn destroy_metal_view(&mut self) {
        if !self.platform.metal_view.is_null() {
            unsafe { SDL_Metal_DestroyView(self.platform.metal_view) };
            self.platform.metal_view = std::ptr::null_mut();
        }
    }

    fn ensure_audio_initialized(&mut self) {
        if self.ctx.runtime.audio.is_none() {
            match crate::audio::AudioSystem::new() {
                Ok(audio) => self.ctx.runtime.audio = Some(audio),
                Err(e) => eprintln!("[spot][audio] initialization failed: {:?}", e),
            }
        }
    }

    fn ensure_surface(&mut self) {
        if self.platform.backgrounded || self.surface.is_some() || self.platform.window.is_null() {
            return;
        }

        let Some(target) = self.surface_target() else {
            return;
        };
        let surface = unsafe { self.instance.create_surface_unsafe(target) };
        match surface {
            Ok(surface) => {
                let surface = unsafe {
                    std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(surface)
                };
                self.surface = Some(surface);
            }
            Err(e) => eprintln!("[spot][surface] create failed: {:?}", e),
        }
    }

    fn recreate_surface(&mut self) {
        self.surface.take();
        self.ensure_surface();
        let (w, h) = self.window_physical_size();
        if let Some(surface) = self.surface.as_ref()
            && let Some(g) = self.ctx.graphics_mut()
        {
            g.resize(surface, w, h);
        }
    }

    fn begin_graphics_init_if_needed(&mut self) {
        if !matches!(self.init_state, platform::GraphicsInitState::NotStarted) {
            return;
        }

        let Some(surface) = self.surface.as_ref() else {
            return;
        };
        let (width, height) = self.window_physical_size();
        platform::begin_graphics_init(
            &mut self.init_state,
            &self.instance,
            surface,
            width,
            height,
            self.window_config.transparent,
        );
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

    fn poll_events(&mut self) -> bool {
        let mut running = true;
        loop {
            let mut event = unsafe { std::mem::zeroed::<SDL_Event>() };
            if !unsafe { SDL_PollEvent(&mut event) } {
                break;
            }

            let ty = event.event_type();
            if ty == SDL_EVENT_QUIT {
                running = false;
            } else if ty == SDL_EVENT_TERMINATING {
                self.enter_background();
                running = false;
            } else if ty == SDL_EVENT_WILL_ENTER_BACKGROUND || ty == SDL_EVENT_DID_ENTER_BACKGROUND
            {
                self.enter_background();
            } else if ty == SDL_EVENT_WILL_ENTER_FOREGROUND || ty == SDL_EVENT_DID_ENTER_FOREGROUND
            {
                self.enter_foreground();
            } else if ty == SDL_EVENT_WINDOW_CLOSE_REQUESTED {
                running = false;
            } else if ty == SDL_EVENT_WINDOW_FOCUS_GAINED {
                self.ctx.input_mut().handle_focus(true);
            } else if ty == SDL_EVENT_WINDOW_FOCUS_LOST {
                self.ctx.input_mut().handle_focus(false);
            } else if ty == SDL_EVENT_WINDOW_PIXEL_SIZE_CHANGED
                || ty == SDL_EVENT_WINDOW_RESIZED
                || ty == SDL_EVENT_WINDOW_DISPLAY_SCALE_CHANGED
            {
                self.handle_resize_event();
            } else if ty == SDL_EVENT_MOUSE_MOTION {
                let motion = unsafe { event.motion };
                self.ctx
                    .input_mut()
                    .handle_cursor_moved(Pt(motion.x as f32), Pt(motion.y as f32));
            } else if ty == SDL_EVENT_MOUSE_BUTTON_DOWN || ty == SDL_EVENT_MOUSE_BUTTON_UP {
                let button = unsafe { event.button };
                let state = if button.down {
                    InputState::Pressed
                } else {
                    InputState::Released
                };
                self.ctx
                    .input_mut()
                    .handle_mouse_input(state, MouseButton::from_sdl_button(button.button));
            } else if ty == SDL_EVENT_MOUSE_WHEEL {
                let wheel = unsafe { event.wheel };
                self.ctx.input_mut().handle_mouse_wheel(wheel.x, wheel.y);
            } else if ty == SDL_EVENT_KEY_DOWN || ty == SDL_EVENT_KEY_UP {
                let key = unsafe { event.key };
                if let Some(key_code) = Key::from_sdl_scancode(key.scancode) {
                    let state = if key.down {
                        InputState::Pressed
                    } else {
                        InputState::Released
                    };
                    self.ctx.input_mut().handle_keyboard_input(state, key_code);
                }
            } else if ty == SDL_EVENT_TEXT_INPUT {
                let text = unsafe { event.text };
                if !text.text.is_null()
                    && let Ok(value) = unsafe { CStr::from_ptr(text.text) }.to_str()
                {
                    self.ctx.input_mut().handle_ime_commit(value);
                }
            } else if ty == SDL_EVENT_TEXT_EDITING {
                let edit = unsafe { event.edit };
                if !edit.text.is_null()
                    && let Ok(value) = unsafe { CStr::from_ptr(edit.text) }.to_str()
                {
                    self.ctx.input_mut().handle_ime_preedit(value.to_string());
                }
            } else if ty == SDL_EVENT_FINGER_DOWN
                || ty == SDL_EVENT_FINGER_MOTION
                || ty == SDL_EVENT_FINGER_UP
            {
                self.handle_touch_event(ty, unsafe { event.tfinger });
            }
        }
        running
    }

    fn apply_lifecycle_pending(&mut self) -> bool {
        let pending = SDL_LIFECYCLE_PENDING.swap(LIFECYCLE_NONE, Ordering::SeqCst);
        if pending & LIFECYCLE_TERMINATING != 0 {
            self.enter_background();
            return false;
        }
        if pending & LIFECYCLE_ENTER_BACKGROUND != 0 {
            self.enter_background();
        }
        if pending & LIFECYCLE_ENTER_FOREGROUND != 0 {
            self.enter_foreground();
        }
        true
    }

    fn enter_background(&mut self) {
        if self.platform.backgrounded && self.surface.is_none() {
            return;
        }

        self.platform.backgrounded = true;
        self.ctx.input_mut().handle_focus(false);
        self.ctx.clear_transient_input();
        self.ctx.clear_transient_state();
        self.release_surface();

        if !self.platform.scene_suspended_for_background {
            if let Some(spot) = self.scene.spot_mut() {
                spot.suspended(&mut self.ctx);
            }
            self.platform.scene_suspended_for_background = true;
        }

        #[cfg(all(target_os = "ios", feature = "sensors"))]
        if let Some(state) = self.platform.sensor_state.as_ref() {
            state.disable();
        }
    }

    fn enter_foreground(&mut self) {
        self.platform.backgrounded = false;
        self.restore_foreground_resources();
    }

    fn restore_foreground_resources(&mut self) {
        if self.platform.backgrounded || self.platform.window.is_null() {
            return;
        }
        if self.surface.is_some() && !self.platform.scene_suspended_for_background {
            return;
        }

        self.ensure_surface();
        self.refresh_window_metrics();
        let (w, h) = self.window_physical_size();
        if let Some(surface) = self.surface.as_ref()
            && let Some(g) = self.ctx.graphics_mut()
        {
            g.resize(surface, w, h);
        }

        self.begin_graphics_init_if_needed();
        self.ensure_scene_ready();

        if self.surface.is_some() && self.platform.scene_suspended_for_background {
            #[cfg(all(target_os = "ios", feature = "sensors"))]
            if let Some(state) = self.platform.sensor_state.as_ref() {
                state.enable();
            }

            if let Some(spot) = self.scene.spot_mut() {
                spot.resumed(&mut self.ctx);
            }
            self.platform.scene_suspended_for_background = false;
            self.timing.reset();
        }
    }

    fn release_surface(&mut self) {
        self.surface.take();
        self.destroy_metal_view();
    }

    fn run_updates(&mut self) {
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
    }

    fn draw_frame(&mut self) {
        if !self.apply_lifecycle_pending() {
            return;
        }
        if self.platform.backgrounded {
            return;
        }

        self.ensure_scene_ready();

        let Some(surface) = self.surface.as_ref() else {
            return;
        };

        let alpha = self.timing.alpha();
        self.ctx.set_draw_alpha(alpha);

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
            self.handle_surface_error(error);
        }
    }

    fn handle_surface_error(&mut self, error: wgpu::SurfaceError) {
        match error {
            wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                eprintln!(
                    "[spot][surface] Surface lost or outdated: {:?}. Recreating...",
                    error
                );
                self.recreate_surface();
            }
            wgpu::SurfaceError::OutOfMemory => {
                eprintln!("[spot][surface] out of memory");
            }
            wgpu::SurfaceError::Timeout | wgpu::SurfaceError::Other => {
                eprintln!("[spot][surface] draw error: {:?}", error);
            }
        }
    }

    fn handle_resize_event(&mut self) {
        self.refresh_window_metrics();
        let (w, h) = self.window_physical_size();
        if let Some(surface) = self.surface.as_ref()
            && let Some(g) = self.ctx.graphics_mut()
        {
            g.resize(surface, w, h);
        }
    }

    fn handle_touch_event(&mut self, ty: SDL_EventType, event: SDL_TouchFingerEvent) {
        // SDL finger coordinates are normalized to [0, 1] on mobile.
        let (w, h) = self.window_logical_size_raw();
        let x = Pt(event.x * w as f32);
        let y = Pt(event.y * h as f32);
        let phase = if ty == SDL_EVENT_FINGER_DOWN {
            TouchPhase::Started
        } else if ty == SDL_EVENT_FINGER_MOTION {
            TouchPhase::Moved
        } else {
            TouchPhase::Ended
        };

        self.ctx
            .input_mut()
            .handle_touch_raw(event.fingerID.value(), (x, y), phase);
    }

    fn apply_pending_window_requests(&mut self) {
        if self.platform.window.is_null() {
            let _ = self.ctx.take_window_title_request();
            let _ = self.ctx.take_cursor_visible_request();
            let _ = self.ctx.take_fullscreen_request();
            return;
        }

        if let Some(title) = self.ctx.take_window_title_request()
            && let Ok(title) = CString::new(title)
        {
            unsafe { SDL_SetWindowTitle(self.platform.window, title.as_ptr()) };
        }
        if let Some(visible) = self.ctx.take_cursor_visible_request() {
            unsafe {
                if visible {
                    let _ = sdl3_sys::mouse::SDL_ShowCursor();
                } else {
                    let _ = sdl3_sys::mouse::SDL_HideCursor();
                }
            }
        }
        if let Some(enabled) = self.ctx.take_fullscreen_request() {
            unsafe {
                let _ = SDL_SetWindowFullscreen(self.platform.window, enabled);
            }
        }
    }

    fn refresh_window_metrics(&mut self) {
        let scale = self.window_scale_factor();
        self.scale_factor = scale;
        let (w, h) = self.window_physical_size();
        self.platform.last_physical_size = Some((w, h));
        self.ctx.update_window_metrics_physical(w, h, scale);
    }

    fn window_physical_size(&self) -> (u32, u32) {
        let mut w = 1;
        let mut h = 1;
        unsafe {
            let _ = SDL_GetWindowSizeInPixels(self.platform.window, &mut w, &mut h);
        }
        (w.max(1) as u32, h.max(1) as u32)
    }

    fn window_logical_size_raw(&self) -> (i32, i32) {
        let mut w = 1;
        let mut h = 1;
        unsafe {
            let _ = SDL_GetWindowSize(self.platform.window, &mut w, &mut h);
        }
        (w.max(1), h.max(1))
    }

    fn window_scale_factor(&self) -> f64 {
        let scale = unsafe { SDL_GetWindowDisplayScale(self.platform.window) };
        if scale.is_finite() && scale > 0.0 {
            scale as f64
        } else {
            1.0
        }
    }

    fn surface_target(&mut self) -> Option<wgpu::SurfaceTargetUnsafe> {
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            if self.platform.metal_view.is_null() {
                self.platform.metal_view = unsafe { SDL_Metal_CreateView(self.platform.window) };
                if self.platform.metal_view.is_null() {
                    eprintln!(
                        "[spot][surface] SDL_Metal_CreateView failed: {}",
                        sdl_error()
                    );
                    return None;
                }
            }
            let layer = unsafe { SDL_Metal_GetLayer(self.platform.metal_view) };
            if layer.is_null() {
                eprintln!("[spot][surface] SDL_Metal_GetLayer failed: {}", sdl_error());
                return None;
            }
            return Some(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer));
        }

        #[cfg(target_os = "windows")]
        {
            use raw_window_handle::{
                RawDisplayHandle, RawWindowHandle, Win32WindowHandle, WindowsDisplayHandle,
            };
            use std::num::NonZeroIsize;

            let props = unsafe { SDL_GetWindowProperties(self.platform.window) };
            let hwnd = get_prop_ptr(props, SDL_PROP_WINDOW_WIN32_HWND_POINTER)
                .and_then(|p| NonZeroIsize::new(p.as_ptr() as isize))
                .expect("SDL window is missing a Win32 HWND property");
            let mut window = Win32WindowHandle::new(hwnd);
            window.hinstance = get_prop_ptr(props, SDL_PROP_WINDOW_WIN32_INSTANCE_POINTER)
                .and_then(|p| NonZeroIsize::new(p.as_ptr() as isize));
            return Some(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: RawDisplayHandle::Windows(WindowsDisplayHandle::new()),
                raw_window_handle: RawWindowHandle::Win32(window),
            });
        }

        #[cfg(target_os = "android")]
        {
            use raw_window_handle::{AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle};

            let props = unsafe { SDL_GetWindowProperties(self.platform.window) };
            let Some(native_window) = get_prop_ptr(props, SDL_PROP_WINDOW_ANDROID_WINDOW_POINTER)
            else {
                eprintln!("[spot][surface] SDL Android native window is not available yet");
                return None;
            };
            return Some(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: RawDisplayHandle::Android(
                    raw_window_handle::AndroidDisplayHandle::new(),
                ),
                raw_window_handle: RawWindowHandle::AndroidNdk(AndroidNdkWindowHandle::new(
                    native_window,
                )),
            });
        }

        #[cfg(all(
            unix,
            not(target_os = "macos"),
            not(target_os = "ios"),
            not(target_os = "android"),
            not(target_family = "wasm")
        ))]
        {
            use raw_window_handle::{
                RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
                XlibDisplayHandle, XlibWindowHandle,
            };
            use sdl3_sys::properties::SDL_GetNumberProperty;

            let props = unsafe { SDL_GetWindowProperties(self.platform.window) };
            if let (Some(display), Some(surface)) = (
                get_prop_ptr(props, SDL_PROP_WINDOW_WAYLAND_DISPLAY_POINTER),
                get_prop_ptr(props, SDL_PROP_WINDOW_WAYLAND_SURFACE_POINTER),
            ) {
                return Some(wgpu::SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
                        display,
                    )),
                    raw_window_handle: RawWindowHandle::Wayland(WaylandWindowHandle::new(surface)),
                });
            }

            if let Some(display) = get_prop_ptr(props, SDL_PROP_WINDOW_X11_DISPLAY_POINTER) {
                let window =
                    unsafe { SDL_GetNumberProperty(props, SDL_PROP_WINDOW_X11_WINDOW_NUMBER, 0) };
                if window != 0 {
                    return Some(wgpu::SurfaceTargetUnsafe::RawHandle {
                        raw_display_handle: RawDisplayHandle::Xlib(XlibDisplayHandle::new(
                            Some(display),
                            unsafe {
                                SDL_GetNumberProperty(props, SDL_PROP_WINDOW_X11_SCREEN_NUMBER, 0)
                            } as i32,
                        )),
                        raw_window_handle: RawWindowHandle::Xlib(XlibWindowHandle::new(
                            window as _,
                        )),
                    });
                }
            }
        }

        #[cfg(target_family = "wasm")]
        {
            use raw_window_handle::{RawDisplayHandle, RawWindowHandle, WebWindowHandle};
            use sdl3_sys::properties::SDL_GetStringProperty;

            const WEB_HANDLE_ID: u32 = 1;
            let props = unsafe { SDL_GetWindowProperties(self.platform.window) };
            let canvas_id = unsafe {
                SDL_GetStringProperty(
                    props,
                    SDL_PROP_WINDOW_EMSCRIPTEN_CANVAS_ID_STRING,
                    std::ptr::null(),
                )
            };
            if !canvas_id.is_null() {
                if let Ok(selector) = unsafe { CStr::from_ptr(canvas_id) }.to_str() {
                    set_emscripten_canvas_raw_handle(selector, WEB_HANDLE_ID);
                }
            }

            return Some(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: RawDisplayHandle::Web(
                    raw_window_handle::WebDisplayHandle::new(),
                ),
                raw_window_handle: RawWindowHandle::Web(WebWindowHandle::new(WEB_HANDLE_ID)),
            });
        }

        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        panic!("SDL window backend is not supported by spottedcat's wgpu surface bridge");
    }
}

unsafe extern "C" fn lifecycle_event_watch(
    _userdata: *mut std::ffi::c_void,
    event: *mut SDL_Event,
) -> bool {
    if event.is_null() {
        return true;
    }

    let ty = unsafe { (*event).event_type() };
    if ty == SDL_EVENT_TERMINATING {
        SDL_LIFECYCLE_PENDING.fetch_or(LIFECYCLE_TERMINATING, Ordering::SeqCst);
    } else if ty == SDL_EVENT_WILL_ENTER_BACKGROUND || ty == SDL_EVENT_DID_ENTER_BACKGROUND {
        SDL_LIFECYCLE_PENDING.fetch_or(LIFECYCLE_ENTER_BACKGROUND, Ordering::SeqCst);
    } else if ty == SDL_EVENT_WILL_ENTER_FOREGROUND || ty == SDL_EVENT_DID_ENTER_FOREGROUND {
        SDL_LIFECYCLE_PENDING.fetch_or(LIFECYCLE_ENTER_FOREGROUND, Ordering::SeqCst);
    }
    true
}

#[cfg(target_family = "wasm")]
fn set_emscripten_canvas_raw_handle(selector: &str, id: u32) {
    unsafe extern "C" {
        fn emscripten_run_script(script: *const std::ffi::c_char);
    }

    let script = format!(
        "var c=document.querySelector({});if(c){{c.dataset.rawHandle='{}';}}",
        js_string(selector),
        id
    );
    if let Ok(script) = CString::new(script) {
        unsafe { emscripten_run_script(script.as_ptr()) };
    }
}

#[cfg(target_family = "wasm")]
fn js_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

#[cfg(any(
    target_os = "windows",
    target_os = "android",
    all(
        unix,
        not(target_os = "macos"),
        not(target_os = "ios"),
        not(target_os = "android"),
        not(target_family = "wasm")
    )
))]
fn get_prop_ptr(
    props: sdl3_sys::properties::SDL_PropertiesID,
    name: *const std::ffi::c_char,
) -> Option<std::ptr::NonNull<std::ffi::c_void>> {
    std::ptr::NonNull::new(unsafe {
        sdl3_sys::properties::SDL_GetPointerProperty(props, name, std::ptr::null_mut())
    })
}

fn sdl_error() -> String {
    let ptr = SDL_GetError();
    if ptr.is_null() {
        "unknown SDL error".to_string()
    } else {
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }
}
