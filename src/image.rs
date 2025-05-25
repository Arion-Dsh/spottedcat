
use std::{cell::RefCell, path, sync::{atomic::{AtomicUsize, Ordering}, Arc}};
use crate::{graphics::{ImageState, Texture}, RUNTIME};


static GLOBAL_THREAD_COUNT: AtomicUsize = AtomicUsize::new(0);

pub(crate) static mut DRAW_QUEUE: Vec<(Arc<ImageState>, Arc<Texture>, bool)> = Vec::new();

#[derive(Clone)]
pub struct Image {
    pub(crate)id :usize,
    pub(crate) bounds: (u32, u32, u32, u32),
    pub(crate) img:    Option<image::DynamicImage>,
    pub(crate) texture: Arc<RefCell<Option<Texture>>>,
    pub(crate) image_state: Arc<RefCell<Option<ImageState>>>,
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
            image_state: Arc::new(RefCell::new(None))
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
            image_state: Arc::new(RefCell::new(None))
        }
    }

    pub fn load(&mut self){
        if let Some(img) = self.img.take() {
            let texture = Texture::from_image(
                &unsafe { RUNTIME.as_ref().unwrap() }.device,
                &unsafe { RUNTIME.as_ref().unwrap() }.queue,
                &img,
                Some("Image Texture"),  
            );
            self.texture.borrow_mut().replace(texture.unwrap());
        }
        {
            let mut image_state = self.image_state.borrow_mut();
            if image_state.is_none() {
                let state = ImageState::new(&unsafe { RUNTIME.as_ref().unwrap() }.device, &unsafe { RUNTIME.as_ref().unwrap() }.config);
                *image_state = Some(state);
            }
        }
    }

    pub fn sub_image(&mut self, x:u32, y:u32, w:u32, h:u32) -> Image {
        let id = GLOBAL_THREAD_COUNT.fetch_add(1, Ordering::Relaxed);
        let image = Image {
            id,
            bounds: (self.bounds.0 + x, self.bounds.1 + y, w, h),
            img: None,
            texture: self.texture.clone(),
            image_state: self.image_state.clone()
        };
        
        image
    }

    /// Draws the given image onto the current image. The given image must have been previously loaded with the `load` method.
    /// 
    /// # Example
    /// 
    pub fn draw(&mut self, img: Image) {
        if let Some(state) = img.image_state.borrow().as_ref() {
            let texture = img.texture.borrow().as_ref().unwrap().clone();
            unsafe {
                DRAW_QUEUE.push((Arc::new(state.clone()), Arc::new(texture), false));
            }
        } else {
            eprintln!("Warning: Attempted to draw image without initialized image_state");
        }
    }
}
