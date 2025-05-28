mod drawable;
mod image;
mod window;
mod keycode;
mod events;
mod audio;
use std::sync::{Arc, Mutex};
use winit::{event_loop::EventLoop, window::Window};
pub use keycode::Keycode;
pub use graphics::DrawOptions as DrawOpt;
pub use image::Image;
pub use events::*;
pub use audio::*;

static mut CAT: Mutex<Option<SpottedCat>> = Mutex::new(None);

#[allow(dead_code)]
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

struct SpottedCat
{
    spot: Box<dyn Spot>,
    screen: Option<Image>,
    context: Option<Context<'static>>,
}

#[warn(unreachable_code)]
pub fn run<T>(spot: T)
where
    T: Spot + 'static,
{
    let event_loop = EventLoop::new().unwrap();
    let cat = SpottedCat { 
        spot: Box::new(spot), 
        screen: None, 
        context: None,
    };
    #[allow(static_mut_refs)]
        let mut cat_lock = unsafe { CAT.lock().unwrap() };
    *cat_lock = Some(cat);
    let _ = event_loop.run_app(cat_lock.as_mut().unwrap());
}

pub trait Spot {
    fn preload(&mut self);
    fn update(&mut self, dt: f32);
    fn draw(&mut self, screen: &mut Image);
    fn release(&mut self);
}
