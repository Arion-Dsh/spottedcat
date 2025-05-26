mod image;
pub use image::Image;
mod window;
mod drawable;
use winit::{event_loop::EventLoop, window::Window};
use std::sync::Mutex;
use std::sync::Arc;
pub use graphics::DrawOptions as DrawOpt;

static mut RUNTIME:  Option<Context> = None;
static MUTEX: Mutex<()> = Mutex::new(());

type WindowID = winit::window::WindowId;

mod graphics;
use crate::graphics::Graphics;  

struct Context<'a> {
    window: Arc<Window>,
    surface: wgpu::Surface<'a>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    config: wgpu::SurfaceConfiguration,
    graphic: Graphics,
}

struct SpottedCat<T>
where
    T: Spot,
{
    spot: T,
    screen: Option<Image>,
}

#[warn(unreachable_code)]
pub fn run<T>(spot: T)
where
    T: Spot + 'static,
{
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut sp = SpottedCat { spot, screen: None };
    let _ = event_loop.run_app(&mut sp);
}

pub trait Spot {
    fn preload(&mut self);
    fn update(&mut self, dt: f32);
    fn draw(&mut self, screen: &mut Image);
    fn release(&mut self);
}
