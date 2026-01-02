use crate::graphics::Graphics;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use std::cell::RefCell;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;
use std::sync::OnceLock;

#[cfg(all(not(target_arch = "wasm32"), target_os = "android"))]
pub(crate) const PREFERRED_WGPU_BACKENDS: &[wgpu::Backends] =
    &[wgpu::Backends::VULKAN, wgpu::Backends::GL];

#[cfg(not(all(not(target_arch = "wasm32"), target_os = "android")))]
#[allow(dead_code)]
pub(crate) const PREFERRED_WGPU_BACKENDS: &[wgpu::Backends] = &[wgpu::Backends::PRIMARY];

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use wasm_bindgen_futures;

#[cfg(not(target_arch = "wasm32"))]
use std::future::Future;
#[cfg(not(target_arch = "wasm32"))]
use std::pin::Pin;
#[cfg(not(target_arch = "wasm32"))]
use std::task::{Context as TaskContext, Poll, RawWaker, RawWakerVTable, Waker};

pub(crate) enum GraphicsInitState {
    NotStarted,
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    Pending,
    Ready(Option<Graphics>),
    Failed,
}

pub(crate) fn finalize_graphics(init_state: &mut GraphicsInitState) -> bool {
    let GraphicsInitState::Ready(slot) = init_state else {
        return false;
    };
    let Some(graphics) = slot.take() else {
        return false;
    };
    if set_global_graphics(graphics).is_err() {
        panic!("global Graphics already initialized");
    }
    true
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn begin_graphics_init(
    init_state: &mut GraphicsInitState,
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'static>,
    width: u32,
    height: u32,
) {
    match init_state {
        GraphicsInitState::NotStarted => {}
        GraphicsInitState::Ready(_) | GraphicsInitState::Failed => return,
    }

    let graphics_r = block_on(Graphics::new(instance, surface, width, height));
    match graphics_r {
        Ok(graphics) => *init_state = GraphicsInitState::Ready(Some(graphics)),
        Err(e) => {
            eprintln!("[spot][init] Graphics::new failed: {:?}", e);
            *init_state = GraphicsInitState::Failed;
        }
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub(crate) fn begin_graphics_init(
    init_state: &mut GraphicsInitState,
    instance: wgpu::Instance,
    surface_ptr: *const wgpu::Surface<'static>,
    width: u32,
    height: u32,
    callback: Box<dyn FnOnce(anyhow::Result<Graphics>)>,
) {
    match init_state {
        GraphicsInitState::NotStarted => {}
        GraphicsInitState::Pending | GraphicsInitState::Ready(_) | GraphicsInitState::Failed => return,
    }

    *init_state = GraphicsInitState::Pending;
    spawn_graphics_init(instance, surface_ptr, width, height, callback);
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn block_on<F: Future>(mut future: F) -> F::Output {
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

pub(crate) fn create_wgpu_instance() -> wgpu::Instance {
    #[cfg(all(not(target_arch = "wasm32"), target_os = "android"))]
    {
        return wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });
    }
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    {
        return wgpu::Instance::default();
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        return wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub(crate) fn spawn_graphics_init(
    instance: wgpu::Instance,
    surface_ptr: *const wgpu::Surface<'static>,
    width: u32,
    height: u32,
    callback: Box<dyn FnOnce(anyhow::Result<Graphics>)>,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let surface = unsafe { &*surface_ptr };
        let r = Graphics::new(&instance, surface, width, height).await;
        callback(r);
    });
}

#[cfg(not(target_arch = "wasm32"))]
static GLOBAL_GRAPHICS: OnceLock<Mutex<Graphics>> = OnceLock::new();

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
thread_local! {
    static GLOBAL_GRAPHICS: RefCell<Option<Graphics>> = RefCell::new(None);
}

pub(crate) fn set_global_graphics(graphics: Graphics) -> Result<(), Graphics> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        return GLOBAL_GRAPHICS
            .set(Mutex::new(graphics))
            .map_err(|m| m.into_inner().unwrap_or_else(|e| e.into_inner()));
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        return GLOBAL_GRAPHICS.with(|cell| {
            let mut slot = cell.borrow_mut();
            if slot.is_some() {
                Err(graphics)
            } else {
                *slot = Some(graphics);
                Ok(())
            }
        });
    }
}

pub(crate) fn with_graphics<R>(f: impl FnOnce(&mut Graphics) -> R) -> R {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mutex = GLOBAL_GRAPHICS
            .get()
            .expect("global Graphics not initialized");
        let mut g = mutex.lock().expect("Graphics mutex poisoned");
        return f(&mut g);
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        return GLOBAL_GRAPHICS.with(|cell| {
            let mut slot = cell.borrow_mut();
            let g = slot.as_mut().expect("global Graphics not initialized");
            f(g)
        });
    }
}

pub(crate) fn align_write_texture_bytes(
    bytes_per_row: u32,
    height: u32,
    data: Vec<u8>,
) -> (Vec<u8>, u32) {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        let align = 256u32;
        let padded = ((bytes_per_row + align - 1) / align) * align;
        if padded == bytes_per_row {
            return (data, bytes_per_row);
        }

        let mut out = vec![0u8; (padded * height) as usize];
        for row in 0..height {
            let src_off = (row * bytes_per_row) as usize;
            let dst_off = (row * padded) as usize;
            out[dst_off..dst_off + bytes_per_row as usize]
                .copy_from_slice(&data[src_off..src_off + bytes_per_row as usize]);
        }
        return (out, padded);
    }

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        let _ = height;
        return (data, bytes_per_row);
    }
}

pub(crate) fn surface_usage(surface_caps: &wgpu::SurfaceCapabilities) -> wgpu::TextureUsages {
    #[cfg(target_os = "android")]
    {
        let _ = surface_caps;
        return wgpu::TextureUsages::RENDER_ATTACHMENT;
    }

    #[cfg(not(target_os = "android"))]
    {
        let desired_usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST;
        if surface_caps.usages.contains(desired_usage) {
            desired_usage
        } else {
            wgpu::TextureUsages::RENDER_ATTACHMENT
        }
    }
}
