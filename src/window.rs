use std::{sync::Arc};
use wgpu;

use futures::executor::block_on;
use winit::{application::ApplicationHandler, window::Window};

use crate::{Image, SpottedCat};
use crate::image::{DRAW_QUEUE, LOAD_QUEUE};
use crate::events::{EventManager, EVENT_MANAGER};


impl ApplicationHandler for SpottedCat
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

        self.context = Some(crate::Context {
            window: window,
            surface,
            device,
            queue,
            config,
            graphic,
        });
        unsafe {
            EVENT_MANAGER = Some(EventManager::new());
        }

        let screen = Image::new(size.width, size.height);
        self.screen = Some(screen);
        self.spot.preload();

        #[allow(static_mut_refs)]
        let imgs = unsafe { std::mem::take(&mut LOAD_QUEUE) };
        for mut img in imgs {
            let _ = img.load(&self.context.as_ref().unwrap());
        }

    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        unsafe {
            #[allow(static_mut_refs)]
            EVENT_MANAGER.as_mut().unwrap().process_window_event(event.clone());
        }

        match event {
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            winit::event::WindowEvent::RedrawRequested => {
                #[allow(static_mut_refs)]
                let ctx = self.context.as_ref().unwrap() ;
                #[allow(static_mut_refs)]
                let images = unsafe { std::mem::take(&mut DRAW_QUEUE) };
                
                ctx.graphic.draw(&ctx.surface, images);
              
            }
            winit::event::WindowEvent::Resized(size) => {
                #[allow(static_mut_refs)]
                let ctx = self.context.as_mut().unwrap();
                ctx.config.width = size.width;
                ctx.config.height = size.height;
                ctx.surface.configure(&ctx.device, &ctx.config);
                ctx.graphic.resize(&ctx.config, ctx.window.scale_factor() as f32);
            }
            _ => {
                let _ = self.spot.update(0.0);
                let _ = self.spot.draw(&mut self.screen.as_mut().unwrap()); 
                self.context.as_ref().unwrap().window.request_redraw();
            }
        }
    }
}
