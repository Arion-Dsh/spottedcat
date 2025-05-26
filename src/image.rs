
use crate::{
    graphics::{DrawItem, ImageState, Texture, TextureUniformState},
    DrawOpt, RUNTIME,
};
use std::{
    cell::RefCell,
    result::Result,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

/// Global counter for generating unique image IDs
static GLOBAL_THREAD_COUNT: AtomicUsize = AtomicUsize::new(0);

pub(crate) static mut DRAW_QUEUE: Vec<DrawItem> = Vec::new();

#[derive(Clone)]
pub struct Image {
    #[allow(dead_code)]
    pub(crate) id: usize,
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// Texture size in floating point
    pub(crate) t_size: [f32; 2],
    /// Bounds of the image within its parent (for sub-images)
    pub(crate) bounds: (u32, u32, u32, u32),
    /// The actual image data
    pub(crate) img: Option<image::DynamicImage>,
    /// GPU texture representation
    pub(crate) texture: Arc<RefCell<Option<Texture>>>,
    /// Graphics state for rendering
    pub(crate) image_state: Arc<RefCell<Option<ImageState>>>,
    /// Texture uniform state for shader parameters
    pub(crate) texture_uniform_state: Arc<RefCell<Option<TextureUniformState>>>,
    /// Reference to the original image if this is a sub-image
    pub(crate) original_image: Option<Box<Image>>,
}

impl Image {
    /// Create a new empty image with specified dimensions
    ///
    /// # Arguments
    /// * `w` - Width of the image
    /// * `h` - Height of the image
    ///
    /// # Returns
    /// A new empty image with the specified dimensions
    pub fn new(w: u32, h: u32) -> Image {
        let id = GLOBAL_THREAD_COUNT.fetch_add(1, Ordering::Relaxed);
        let img = Some(image::DynamicImage::new_rgba8(w, h));
        let texture = Arc::new(RefCell::new(None));
        Image {
            id,
            width: w,
            height: h,
            t_size: [w as f32, h as f32],
            bounds: (0, 0, w, h),
            img,
            texture: texture.clone(),
            image_state: Arc::new(RefCell::new(None)),
            texture_uniform_state: Arc::new(RefCell::new(None)),
            original_image: None,
        }
    }

    /// Create a new image from a file path
    ///
    /// # Arguments
    /// * `path` - Path to the image file
    ///
    /// # Returns
    /// A Result containing the new Image or an error string
    pub fn new_from_path(path: &str) -> Result<Image, String> {
        match image::open(path) {
            Ok(img) => {
                let texture = Arc::new(RefCell::new(None));
                let id = GLOBAL_THREAD_COUNT.fetch_add(1, Ordering::Relaxed);
                Ok(Image {
                    id,
                    width: img.width(),
                    height: img.height(),
                    t_size: [img.width() as f32, img.height() as f32],
                    bounds: (0, 0, img.width(), img.height()),
                    img: Some(img),
                    texture: texture.clone(),
                    image_state: Arc::new(RefCell::new(None)),
                    texture_uniform_state: Arc::new(RefCell::new(None)),
                    original_image: None,
                })
            }
            Err(e) => Err(format!("Failed to load image from {}: {}", path, e)),
        }
    }

    /// Load the image into GPU memory
    ///
    /// This method creates GPU resources for the image if they haven't been created yet.
    /// It also handles sub-images by loading their parent image first.
    ///
    /// # Returns
    /// A Result indicating success or an error message
    pub(crate) fn load(&mut self) -> Result<(), String> {
        // 如果纹理和所有相关的 Uniform 状态都已加载，则直接返回
        if self.texture.borrow().is_some()
            && self.image_state.borrow().is_some()
            && self.texture_uniform_state.borrow().is_some()
        {
            return Ok(());
        }

        // 如果是子图像，先加载父图像
        if let Some(original) = self.original_image.as_mut() {
            original.load()?; 
        }

        #[allow(static_mut_refs)]
        let runtime = unsafe { RUNTIME.as_ref().ok_or("Runtime not initialized")? };
        let device = &runtime.device;
        let queue = &runtime.queue;
        let config = &runtime.config;

        if self.texture.borrow().is_none() {
            if let Some(img_data) = self.img.take() {
                let texture = Texture::from_image(device, queue, &img_data, Some("Image Texture"))
                    .map_err(|e| format!("Failed to create texture: {}", e))?;
                *self.texture.borrow_mut() = Some(texture);
            } else if self.original_image.is_none() {
                return Err("Image data or original_image missing for texture creation".to_string());
            }
        }
        if self.texture_uniform_state.borrow().is_none() {
            let mut state = TextureUniformState::new(device);
            let t_size = [self.t_size[0], self.t_size[1]];
            let uv_offset = [self.bounds.0 as f32, self.bounds.1 as f32];
            let uv_size = [self.bounds.2 as f32, self.bounds.3 as f32];
            state.write_texture_uniform(queue, t_size, uv_offset, uv_size);
            *self.texture_uniform_state.borrow_mut() = Some(state);
        }
        if self.image_state.borrow().is_none() {
            let texture = self.texture.borrow().clone().unwrap();
            let ustate = self.texture_uniform_state.borrow().clone().unwrap();
            let state = ImageState::new(device, config, texture.into(), ustate.into());
            *self.image_state.borrow_mut() = Some(state);
        }

        Ok(())
    }

    /// Create a sub-image from this image
    ///
    /// # Arguments
    /// * `x` - X coordinate of the sub-image
    /// * `y` - Y coordinate of the sub-image
    /// * `w` - Width of the sub-image
    /// * `h` - Height of the sub-image
    ///
    /// # Returns
    /// A new Image representing the sub-region
    pub fn sub_image(&mut self, x: u32, y: u32, w: u32, h: u32) -> Image {
        let id = GLOBAL_THREAD_COUNT.fetch_add(1, Ordering::Relaxed);
        Image {
            id,
            width: w,
            height: h,
            t_size: [self.t_size[0], self.t_size[1]],
            bounds: (self.bounds.0 + x, self.bounds.1 + y, w, h),
            img: None,
            texture: self.texture.clone(),
            image_state: self.image_state.clone(),
            texture_uniform_state: Arc::new(RefCell::new(None)),
            original_image: Some(Box::new(self.clone())),
        }
    }

    /// Draw this image onto another image
    ///
    /// # Arguments
    /// * `img` - The target image to draw onto
    ///
    /// # Returns
    /// A Result indicating success or an error message
    pub fn draw(&mut self, mut img: Image, options: DrawOpt) {
        if let Err(e) = img.load() {
            eprintln!("Error loading image for drawing: {}", e);
            return; 
        }

        let texture = img.texture.borrow().clone().unwrap();
        let state = img.image_state.borrow().clone().unwrap();
        let ustate = img.texture_uniform_state.borrow().clone().unwrap();
        #[allow(static_mut_refs)]
        unsafe {
            DRAW_QUEUE.push(DrawItem {
                state: state.clone().into(),
                texture_uniform_state: ustate.clone().into(),
                size: [img.width as f32, img.height as f32, 0.0],
                texture: texture.clone().into(),
                options: options.into(),
            });
        }
    }

    /// Get the width of the image
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get the height of the image
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Get the dimensions of the image as a tuple (width, height)
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
