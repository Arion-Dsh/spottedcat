
use std::{cell::RefCell, path, sync::{atomic::{AtomicUsize, Ordering}, Arc}, result::Result};
use crate::{graphics::{DrawItem, ImageState, Texture}, DrawOptions, RUNTIME};


static GLOBAL_THREAD_COUNT: AtomicUsize = AtomicUsize::new(0);

pub(crate) static mut DRAW_QUEUE: Vec<DrawItem> = Vec::new();

#[derive(Clone)]
pub struct Image {
    pub(crate)id :usize,
    pub(crate) bounds: (u32, u32, u32, u32),
    pub(crate) img:    Option<image::DynamicImage>,
    pub(crate) texture: Arc<RefCell<Option<Texture>>>,
    pub(crate) image_state: Arc<RefCell<Option<ImageState>>>,
    pub(crate) original_image: Option<Box<Image>>,
} 

impl Image {
    pub fn new(w:u32, h:u32) -> Image {
        let id = GLOBAL_THREAD_COUNT.fetch_add(1, Ordering::Relaxed);
        let img = Some(image::DynamicImage::new_rgba8(w, h));
        let texture = Arc::new(RefCell::new(None));
        Image {
            id,
            bounds: (0, 0, w, h),
            img ,
            texture: texture.clone(),
            image_state: Arc::new(RefCell::new(None)),
            original_image: None,
        }
    }
    pub fn new_from_path(p: &str) -> Image {
        let id = GLOBAL_THREAD_COUNT.fetch_add(1, Ordering::Relaxed);
        let img = image::open(path::Path::new(p)).unwrap();
        let texture = Arc::new(RefCell::new(None));
        Image {
            id,
            bounds: (0, 0, img.width(), img.height()),
            img: Some(img),
            texture: texture.clone(),
            image_state: Arc::new(RefCell::new(None)),
            original_image: None,
        }
    }

    pub fn load(&mut self) -> Result<(), String> {
        // Get runtime references once
        let runtime = unsafe { RUNTIME.as_ref().ok_or("Runtime not initialized")? };
        let device = &runtime.device;
        let queue = &runtime.queue;
        let config = &runtime.config;

        // Create texture if we have an image
        if let Some(img) = self.img.take() {
            let texture = Texture::from_image(device, queue, &img, Some("Image Texture"))
                .map_err(|e| format!("Failed to create texture: {}", e))?;
            *self.texture.borrow_mut() = Some(texture);
        }

        // Initialize image state if not already initialized
        let mut image_state = self.image_state.borrow_mut();
        if image_state.is_none() {
            let state = ImageState::new(device, config);
            // Get texture reference
            let texture = self.texture.borrow();
            let texture = texture.as_ref().ok_or("Texture not initialized")?;
            
            // Write texture uniform
            state.write_texture_uniform(
                queue,
                [texture.width as f32, texture.height as f32],
                [self.bounds.0 as f32, self.bounds.1 as f32],
                [self.bounds.2 as f32, self.bounds.3 as f32]
            );
            
            *image_state = Some(state);
        }

        Ok(())
    }

    pub fn sub_image(&mut self, x:u32, y:u32, w:u32, h:u32) -> Image {
        let id = GLOBAL_THREAD_COUNT.fetch_add(1, Ordering::Relaxed);
        let image = Image {
            id,
            bounds: (self.bounds.0 + x, self.bounds.1 + y, w, h),
            img: None,
            texture: self.texture.clone(),
            image_state: self.image_state.clone(),
            original_image: Some(Box::new(self.clone())),
        };
        
        image
    }


    pub fn draw(&mut self, img: Image) {
        if let Some(state) = img.image_state.borrow().as_ref() {
            let texture = img.texture.borrow().as_ref().unwrap().clone();
            unsafe {
                DRAW_QUEUE.push(DrawItem {
                    state: state.clone().into(),
                    texture: texture.clone().into(),
                    options: DrawOptions::default(),
                });
            }
        } else {
            eprintln!("Warning: Attempted to draw image without initialized image_state");
        }
    }
}
