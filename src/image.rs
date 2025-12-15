
use crate::image_raw::{ImageRaw, ImageRenderer, ImageTransform};
use crate::texture::Texture;
use crate::with_graphics;

/// Rectangle bounds for defining sub-regions of images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bounds {
    /// X coordinate of the top-left corner.
    pub x: u32,
    /// Y coordinate of the top-left corner.
    pub y: u32,
    /// Width of the bounds.
    pub width: u32,
    /// Height of the bounds.
    pub height: u32,
}

impl Bounds {
    /// Creates new bounds with the specified dimensions.
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Handle to an image resource.
///
/// Images are GPU textures that can be drawn to the screen. They are reference-counted
/// and can be cloned cheaply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Image(pub usize);

impl Image {
    /// Creates a new image from raw RGBA8 pixel data.
    ///
    /// # Arguments
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `rgba` - Raw pixel data in RGBA8 format (4 bytes per pixel)
    ///
    /// # Errors
    /// Returns an error if the data length doesn't match width * height * 4.
    pub fn new_from_rgba8(width: u32, height: u32, rgba: &[u8]) -> anyhow::Result<Self> {
        with_graphics(|g| {
            let (device, queue) = g.device_queue();
            let texture = Texture::from_rgba8_with_format(
                device,
                queue,
                width,
                height,
                rgba,
                g.surface_format(),
            )?;
            let raw = g.create_raw_from_texture(&texture)?;
            Ok(g.insert_image_entry(ImageEntry::new(texture, raw)))
        })
    }

    /// Creates a copy of an existing image.
    ///
    /// # Arguments
    /// * `image` - The source image to copy
    pub fn new_from_image(image: Image) -> anyhow::Result<Self> {
        with_graphics(|g| g.insert_sub_image(image, None))
    }

    /// Creates a sub-image from a region of an existing image.
    ///
    /// The sub-image shares the same GPU texture as the source image but renders
    /// only the specified region.
    ///
    /// # Arguments
    /// * `image` - The source image
    /// * `bounds` - The region to extract
    ///
    /// # Errors
    /// Returns an error if the bounds are out of range.
    pub fn sub_image(image: Image, bounds: Bounds) -> anyhow::Result<Self> {
        with_graphics(|g| g.insert_sub_image(image, Some(bounds)))
    }

    /// Draws this image to the context with the specified options.
    ///
    /// # Arguments
    /// * `context` - The drawing context to add this image to
    /// * `options` - Drawing options (position, rotation, scale)
    ///
    /// # Example
    /// ```no_run
    /// # use spot::{Context, Image, ImageDrawOptions};
    /// # let mut context = Context::new();
    /// # let rgba = vec![255u8; 2 * 2 * 4];
    /// # let image = Image::new_from_rgba8(2, 2, &rgba).unwrap();
    /// let mut opts = ImageDrawOptions::default();
    /// opts.position = [spot::Pt(100.0), spot::Pt(100.0)];
    /// opts.scale = [2.0, 2.0];
    /// image.draw(&mut context, opts);
    /// ```
    pub fn draw(self, context: &mut crate::Context, options: crate::ImageDrawOptions) {
        context.push(crate::drawable::DrawAble::Image(self, options));
    }

    /// Draws a drawable onto this image as a render target.
    ///
    /// # Arguments
    /// * `drawable` - The drawable to render onto this image
    /// * `option` - Draw options controlling position, rotation, scale
    ///
    /// # Note
    /// For `DrawAble::Image`, the `option.options` will override the drawable's original options.
    /// For `DrawAble::Text`, the text's `TextOptions` are used, but `position` and `scale` are applied from `option.options`.
    ///
    /// # Example
    /// ```no_run
    /// use spot::{Image, DrawAble, DrawOption, ImageDrawOptions};
    ///
    /// // Load two images
    /// let rgba = vec![255u8; 2 * 2 * 4];
    /// let canvas = Image::new_from_rgba8(2, 2, &rgba).unwrap();
    /// let sprite = Image::new_from_rgba8(2, 2, &rgba).unwrap();
    ///
    /// // Create draw options for positioning sprite on canvas
    /// let option = DrawOption {
    ///     options: ImageDrawOptions {
    ///         position: [spot::Pt(50.0), spot::Pt(50.0)],  // Position on canvas
    ///         rotation: 0.0,
    ///         scale: [1.0, 1.0],
    ///     },
    /// };
    ///
    /// // Draw sprite onto canvas at specified position
    /// let sprite_drawable = DrawAble::Image(sprite, ImageDrawOptions::default());
    /// canvas.draw_sub(sprite_drawable, option).unwrap();
    /// ```
    pub fn draw_sub(
        self,
        drawable: crate::drawable::DrawAble,
        option: crate::drawable::DrawOption,
    ) -> anyhow::Result<()> {
        with_graphics(|g| {
            let drawable_with_options = match drawable {
                crate::drawable::DrawAble::Image(img, _) => {
                    // Apply DrawOption to the image
                    crate::drawable::DrawAble::Image(img, option.options)
                }
                crate::drawable::DrawAble::Text(text, mut text_opts) => {
                    // Apply position from DrawOption to text
                    text_opts.position = option.options.position;
                    text_opts.scale = option.options.scale;
                    crate::drawable::DrawAble::Text(text, text_opts)
                }
            };
            g.draw_drawables_to_image(self, &[drawable_with_options], option)
        })
    }

    pub fn draw_to(
        self,
        drawable: crate::drawable::DrawAble,
        option: crate::drawable::DrawOption,
    ) -> anyhow::Result<()> {
        self.draw_sub(drawable, option)
    }

    pub fn clear(self, color: [f32; 4]) -> anyhow::Result<()> {
        with_graphics(|g| g.clear_image(self, color))
    }

    pub fn copy_from(self, src: Image) -> anyhow::Result<()> {
        with_graphics(|g| g.copy_image(self, src))
    }

    /// Destroys the image and frees its GPU resources.
    ///
    /// Returns true if the image was successfully destroyed.
    pub fn destroy(self) -> bool {
        with_graphics(|g| g.take_image_entry(self).is_some())
    }
}

pub(crate) struct ImageEntry {
    pub(crate) texture: Texture,
    pub(crate) raw: ImageRaw,
    pub(crate) bounds: Bounds,
    visible: bool,
}

impl ImageEntry {
    pub(crate) fn new(texture: Texture, raw: ImageRaw) -> Self {
        let bounds = Bounds {
            x: 0,
            y: 0,
            width: texture.0.width,
            height: texture.0.height,
        };
        Self {
            texture,
            raw,
            bounds,
            visible: true,
        }
    }

    pub(crate) fn new_with_bounds(texture: Texture, raw: ImageRaw, bounds: Bounds) -> Self {
        Self {
            texture,
            raw,
            bounds,
            visible: true,
        }
    }

    pub(crate) fn uvp_from_bounds(&self) -> [[f32; 4]; 4] {
        let tw = self.texture.0.width as f32;
        let th = self.texture.0.height as f32;

        let u0 = (self.bounds.x as f32) / tw;
        let v0 = (self.bounds.y as f32) / th;
        let u1 = ((self.bounds.x + self.bounds.width) as f32) / tw;
        let v1 = ((self.bounds.y + self.bounds.height) as f32) / th;

        let sx = u1 - u0;
        let sy = v1 - v0;

        [
            [sx, 0.0, 0.0, 0.0],
            [0.0, sy, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [u0, v0, 0.0, 1.0],
        ]
    }

    pub(crate) fn set_transform(&mut self, t: ImageTransform) {
        self.raw.set_transform(t);
    }

    pub(crate) fn flush(&mut self, renderer: &ImageRenderer, queue: &wgpu::Queue) {
        if !self.visible {
            return;
        }
        renderer.flush_image(queue, &mut self.raw);
    }

    pub(crate) fn draw<'a>(&self, renderer: &'a ImageRenderer, pass: &mut wgpu::RenderPass<'a>) {
        if !self.visible {
            return;
        }
        renderer.draw(pass, &self.raw);
    }
}
