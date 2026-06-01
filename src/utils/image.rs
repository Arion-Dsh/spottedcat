//! Helpers for turning decoded `image` crate buffers into [`crate::Image`].
//!
//! These helpers keep the source pixel dimensions while deriving a default
//! logical size from the current [`crate::scale_factor`].

use crate::{Context, Image, Pt};

/// Creates a [`crate::Image`] from an [`image::DynamicImage`].
///
/// The resulting image keeps the decoded pixel width and height and derives
/// its logical [`Pt`][crate::Pt] size from the current scale factor.
pub fn from_image(ctx: &mut Context, image: &image::DynamicImage) -> anyhow::Result<Image> {
    let rgba = image.to_rgba8();
    from_rgba_image(ctx, &rgba)
}

/// Creates a [`crate::Image`] from an [`image::RgbaImage`].
///
/// The resulting image keeps the source pixel width and height and derives
/// its logical [`Pt`][crate::Pt] size from the current scale factor.
pub fn from_rgba_image(ctx: &mut Context, image: &image::RgbaImage) -> anyhow::Result<Image> {
    let width_px = image.width();
    let height_px = image.height();
    let scale_factor = ctx.scale_factor().max(1.0);
    let width = Pt::from_physical_px(width_px as f64, scale_factor);
    let height = Pt::from_physical_px(height_px as f64, scale_factor);
    Image::new_from_rgba8_with_pixels(ctx, width_px, height_px, width, height, image.as_raw())
}

use std::sync::mpsc::{self, Receiver};

/// A handle to an image that is loading asynchronously.
#[derive(Debug)]
pub struct LoadingImage {
    path: String,
    rx: Receiver<Result<(u32, u32, Vec<u8>), String>>,
    image: Option<Image>,
    error: Option<String>,
}

impl LoadingImage {
    /// Returns the path of the image being loaded.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Checks if there was an error during loading or decoding.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Checks if the image has finished loading and is ready.
    ///
    /// If loading succeeded, this returns `Some(image)`.
    /// If still loading, returns `None`.
    /// If failed, returns `None` and sets the error (queryable via `error()`).
    pub fn poll(&mut self, ctx: &mut Context) -> Option<Image> {
        if let Some(img) = self.image {
            return Some(img);
        }

        if self.error.is_some() {
            return None;
        }

        match self.rx.try_recv() {
            Ok(Ok((width_px, height_px, rgba))) => {
                let scale_factor = ctx.scale_factor().max(1.0);
                let width = Pt::from_physical_px(width_px as f64, scale_factor);
                let height = Pt::from_physical_px(height_px as f64, scale_factor);
                match Image::new_from_rgba8_with_pixels(ctx, width_px, height_px, width, height, &rgba) {
                    Ok(img) => {
                        self.image = Some(img);
                        Some(img)
                    }
                    Err(e) => {
                        self.error = Some(format!("Failed to register image on GPU: {:?}", e));
                        None
                    }
                }
            }
            Ok(Err(e)) => {
                self.error = Some(e);
                None
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                if self.image.is_none() {
                    self.error = Some("Loading thread disconnected unexpectedly".to_string());
                }
                None
            }
        }
    }

    /// Returns the loaded Image if it is fully registered, otherwise returns None.
    /// Internally polls the loading status automatically.
    pub fn get(&mut self, ctx: &mut Context) -> Option<Image> {
        self.poll(ctx)
    }


    /// Returns the loaded Image if it is ready, otherwise returns the fallback placeholder image.
    /// Internally polls the loading status automatically.
    pub fn get_or(&mut self, ctx: &mut Context, fallback: Image) -> Image {
        self.get(ctx).unwrap_or(fallback)
    }
}


/// Starts loading an image asynchronously from the specified path.
///
/// This reads and decodes the image on a background thread.
/// Call `poll(ctx)` on the returned `LoadingImage` during your `update` or `draw`
/// loop to obtain the registered `Image` once it is ready.
pub fn load_image_async(path: impl Into<String>) -> LoadingImage {
    let path = path.into();
    let (tx, rx) = mpsc::channel();
    let path_clone = path.clone();

    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::spawn(move || {
            let res = (|| -> Result<(u32, u32, Vec<u8>), anyhow::Error> {
                let bytes = crate::assets::load_asset(&path_clone)?;
                let img = image::load_from_memory(&bytes)?;
                let rgba = img.to_rgba8();
                Ok((rgba.width(), rgba.height(), rgba.into_raw()))
            })();
            let _ = tx.send(res.map_err(|e| e.to_string()));
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(async move {
            let res = (|| -> Result<(u32, u32, Vec<u8>), anyhow::Error> {
                let bytes = crate::assets::load_asset(&path_clone)?;
                let img = image::load_from_memory(&bytes)?;
                let rgba = img.to_rgba8();
                Ok((rgba.width(), rgba.height(), rgba.into_raw()))
            })();
            let _ = tx.send(res.map_err(|e| e.to_string()));
        });
    }

    LoadingImage {
        path,
        rx,
        image: None,
        error: None,
    }
}

use std::collections::HashMap;

/// A high-level manager that handles loading and caching multiple asynchronous images.
#[derive(Debug, Default)]
pub struct AsyncImageLoader {
    loading: HashMap<String, LoadingImage>,
    loaded: HashMap<String, Image>,
    errors: HashMap<String, String>,
}

impl AsyncImageLoader {
    /// Creates a new asynchronous image loader.
    pub fn new() -> Self {
        Self {
            loading: HashMap::new(),
            loaded: HashMap::new(),
            errors: HashMap::new(),
        }
    }

    /// Triggers the loading of an image from the specified path, if it is not already loaded or loading.
    pub fn load(&mut self, path: impl Into<String>) {
        let path = path.into();
        if !self.loaded.contains_key(&path) && !self.loading.contains_key(&path) {
            self.errors.remove(&path);
            let loader = load_image_async(&path);
            self.loading.insert(path, loader);
        }
    }

    /// Updates the loading state and returns `(loaded_count, total_count)`.
    ///
    /// Both successfully loaded and failed assets count as "done".
    pub fn progress(&mut self, ctx: &mut Context) -> (usize, usize) {
        let mut finished = Vec::new();
        for (path, loader) in self.loading.iter_mut() {
            if let Some(img) = loader.get(ctx) {
                finished.push((path.clone(), Ok(img)));
            } else if let Some(err) = loader.error() {
                finished.push((path.clone(), Err(err.to_string())));
            }
        }

        for (path, result) in finished {
            match result {
                Ok(img) => {
                    self.loaded.insert(path.clone(), img);
                }
                Err(err) => {
                    self.errors.insert(path.clone(), err);
                }
            }
            self.loading.remove(&path);
        }

        let done = self.loaded.len() + self.errors.len();
        let total = done + self.loading.len();
        (done, total)
    }

    /// Returns the loading progress ratio from `0.0` to `1.0`.
    pub fn progress_ratio(&mut self, ctx: &mut Context) -> f32 {
        let (done, total) = self.progress(ctx);
        if total == 0 {
            1.0
        } else {
            done as f32 / total as f32
        }
    }

    /// Checks if all queued images have finished loading (either successfully or with error).
    pub fn is_done(&mut self, ctx: &mut Context) -> bool {
        let (done, total) = self.progress(ctx);
        done == total && total > 0
    }

    /// Checks if a specific image has finished loading and is ready for rendering.
    pub fn is_ready(&mut self, ctx: &mut Context, path: &str) -> bool {
        if self.loaded.contains_key(path) {
            return true;
        }

        if let Some(mut loader) = self.loading.remove(path) {
            if let Some(img) = loader.get(ctx) {
                self.loaded.insert(path.to_string(), img);
                return true;
            }
            self.loading.insert(path.to_string(), loader);
        }

        false
    }

    /// Retrieves the loaded image handle if it is ready, otherwise returns `None`.
    pub fn get(&mut self, ctx: &mut Context, path: &str) -> Option<Image> {
        if let Some(&img) = self.loaded.get(path) {
            return Some(img);
        }

        if let Some(mut loader) = self.loading.remove(path) {
            if let Some(img) = loader.get(ctx) {
                self.loaded.insert(path.to_string(), img);
                return Some(img);
            }
            self.loading.insert(path.to_string(), loader);
        }

        None
    }

    /// Retrieves the loaded image if ready, otherwise returns the specified `fallback` image.
    pub fn get_or(&mut self, ctx: &mut Context, path: &str, fallback: Image) -> Image {
        self.get(ctx, path).unwrap_or(fallback)
    }

    /// Checks if loading failed for a specific path, returning the error message if any.
    pub fn error(&self, path: &str) -> Option<&str> {
        self.errors.get(path).map(|s| s.as_str()).or_else(|| {
            self.loading.get(path).and_then(|loader| loader.error())
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_loading() {
        let mut ctx = Context::new();
        // Start loading the happy-tree image
        let mut loading = load_image_async("assets/happy-tree.png");
        assert_eq!(loading.path(), "assets/happy-tree.png");
        assert!(loading.error().is_none());

        // Wait a bit for the background thread to finish loading and decoding
        let start = std::time::Instant::now();
        let mut image = None;
        while start.elapsed() < std::time::Duration::from_secs(5) {
            if let Some(img) = loading.poll(&mut ctx) {
                image = Some(img);
                break;
            }
            if let Some(err) = loading.error() {
                panic!("Failed to load asynchronously: {}", err);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let img = image.expect("Failed to load image in 5 seconds");
        // Verify dimensions (since happy-tree.png is decoded, it should have non-zero size)
        assert!(img.width().as_f32() > 0.0);
        assert!(img.height().as_f32() > 0.0);
    }

    #[test]
    fn test_async_image_loader() {
        let mut ctx = Context::new();
        let fallback = Image::new(&mut ctx, Pt(1.0), Pt(1.0), &[255, 255, 255, 255]).unwrap();
        let mut loader = AsyncImageLoader::new();

        // 1. Initial state check
        assert_eq!(loader.progress_ratio(&mut ctx), 1.0); // Empty is 100% loaded
        assert!(!loader.is_done(&mut ctx)); // But is_done requires total > 0

        // Trigger loading
        loader.load("assets/happy-tree.png");
        
        // 2. Loading state check (immediately after queuing)
        assert!(loader.progress_ratio(&mut ctx) < 1.0);
        assert!(!loader.is_done(&mut ctx));

        // Wait for it to become ready
        let start = std::time::Instant::now();
        let mut ready = false;
        while start.elapsed() < std::time::Duration::from_secs(5) {
            if loader.is_done(&mut ctx) {
                ready = true;
                break;
            }
            if let Some(err) = loader.error("assets/happy-tree.png") {
                panic!("Failed to load asynchronously via manager: {}", err);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // 3. Final state check
        assert!(ready);
        assert_eq!(loader.progress_ratio(&mut ctx), 1.0);
        assert!(loader.is_ready(&mut ctx, "assets/happy-tree.png"));
        
        let img = loader.get_or(&mut ctx, "assets/happy-tree.png", fallback);
        assert_ne!(img, fallback);
        assert!(img.width().as_f32() > 0.0);
    }
}



