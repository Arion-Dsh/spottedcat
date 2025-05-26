use std::{rc::Rc, sync::Arc};
use wgpu;

use futures::executor::block_on;
use winit::{application::ApplicationHandler, window::Window};

use crate::{Image, SpottedCat, RUNTIME};
use crate::image::DRAW_QUEUE;


impl<T> ApplicationHandler for SpottedCat<T>
    where T: crate::Spot + 'static
{
     fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

        window.request_redraw();

        let size = window.inner_size();


        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .unwrap();

        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, queue) = block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: Default::default(),
            },
                None,   
        ))
        .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

       

        surface.configure(&device, &config);

        let device = Arc::new(device);
        let queue = Arc::new(queue);
        let graphic = crate::graphics::Graphics::new(device.clone(), queue.clone(), &config, window.scale_factor() as f32);

        unsafe { RUNTIME = Some(crate::Context {
            window: window,
            surface,
            device,
            queue,
            config,
            graphic,
        }) };
        let mut screen = Image::new(size.width, size.height);
        let _ = screen.load();
        self.screen = Some(screen);
        self.spot.preload();

    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {

        match event {
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            winit::event::WindowEvent::RedrawRequested => {
                let ctx = unsafe { RUNTIME.as_ref().unwrap() };
                let images = unsafe { std::mem::take(&mut DRAW_QUEUE) };
                ctx.graphic.draw(&ctx.surface, images);
              
            }
            winit::event::WindowEvent::Resized(size) => {
                
                unsafe {
                    let ctx = RUNTIME.as_mut().unwrap();
                    ctx.config.width = size.width;
                    ctx.config.height = size.height;
                    ctx.surface.configure(&ctx.device, &ctx.config);
                    ctx.graphic.resize(&ctx.config, ctx.window.scale_factor() as f32);
                }
            }
            _ => {
                let _ = self.spot.update(0.0);
                let _ = self.spot.draw(&mut self.screen.as_mut().unwrap()); 
                unsafe {
                    RUNTIME.as_ref().unwrap().window.request_redraw();
                }
            }
        }
    }

    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        let _ = (event_loop, cause);
        
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: ()) {
        let _ = (event_loop, event);
    }

    fn device_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let _ = (event_loop, device_id, event);
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let _ = event_loop;
    }

    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let _ = event_loop;
    }

    fn exiting(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let _ = event_loop;
    }

    fn memory_warning(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let _ = event_loop;
    }
}
