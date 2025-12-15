use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowId};

use crate::{Context, Spot, set_global_graphics, with_graphics, take_scene_switch_request};
use crate::graphics::Graphics;

use std::future::Future;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, RawWaker, RawWakerVTable, Waker};
use std::time::{Duration, Instant};

pub(crate) struct App {
    window: Option<Window>,
    window_id: Option<WindowId>,
    instance: wgpu::Instance,
    surface: Option<wgpu::Surface<'static>>,
    context: Context,
    spot: Option<Box<dyn Spot>>,
    scene_factory: Box<dyn Fn() -> Box<dyn Spot> + Send>,
    previous: Option<Instant>,
    lag: Duration,
    fixed_dt: Duration,
}

fn block_on<F: Future>(mut future: F) -> F::Output {
    fn noop_raw_waker() -> RawWaker {
        fn clone(_: *const ()) -> RawWaker {
            noop_raw_waker()
        }
        fn wake(_: *const ()) {}
        fn wake_by_ref(_: *const ()) {}
        fn drop(_: *const ()) {}

        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
        RawWaker::new(std::ptr::null(), &VTABLE)
    }

    let waker = unsafe { Waker::from_raw(noop_raw_waker()) };
    let mut cx = TaskContext::from_waker(&waker);
    let mut future = unsafe { Pin::new_unchecked(&mut future) };
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

impl App {
    pub(crate) fn new<T: Spot + 'static>() -> Self {
        Self {
            window: None,
            window_id: None,
            instance: wgpu::Instance::default(),
            surface: None,
            context: Context::new(),
            spot: None,
            scene_factory: Box::new(|| Box::new(T::initialize(Context::new()))),
            previous: None,
            lag: Duration::ZERO,
            fixed_dt: Duration::from_millis(16),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);
        self.previous = Some(Instant::now());
        self.lag = Duration::ZERO;

        let window = event_loop
            .create_window(Window::default_attributes().with_title("spot"))
            .expect("failed to create window");
        let size = window.inner_size();

        // SAFETY: We store the Window inside self, and leak a reference by transmuting the
        // surface lifetime to 'static. This is a common pattern for wgpu+winit; the surface
        // must not outlive the window (we drop surface before window in `exiting`).
        let surface = unsafe {
            let s = self.instance.create_surface(&window).expect("failed to create surface");
            std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(s)
        };

        let graphics = block_on(Graphics::new(&self.instance, &surface, size.width, size.height))
            .expect("failed to initialize Graphics");

        if set_global_graphics(graphics).is_err() {
            panic!("global Graphics already initialized");
        }

        let spot = Some((self.scene_factory)());

        self.window_id = Some(window.id());
        self.window = Some(window);
        self.surface = Some(surface);
        self.spot = spot;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) => {
                if let Some(surface) = self.surface.as_ref() {
                    with_graphics(|g| g.resize(surface, new_size.width, new_size.height));
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(surface) = self.surface.as_ref() {
                    self.context.begin_frame();
                    if let Some(spot) = self.spot.as_mut() {
                        spot.draw(&mut self.context);
                    }
                    
                    // Check if scene switch was requested
                    if let Some(factory) = take_scene_switch_request() {
                        if let Some(old_spot) = self.spot.take() {
                            old_spot.remove();
                        }
                        self.spot = Some(factory());
                    }
                    
                    let _ = with_graphics(|g| g.draw_context(surface, &self.context));
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if let Some(previous) = self.previous.replace(now) {
            let elapsed = now.duration_since(previous);
            self.lag = self.lag.saturating_add(elapsed);

            while self.lag >= self.fixed_dt {
                if let Some(spot) = self.spot.as_mut() {
                    spot.update(self.fixed_dt);
                }
                self.lag = self.lag.saturating_sub(self.fixed_dt);
            }
        }

        event_loop.set_control_flow(ControlFlow::Poll);

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
