 
use crate::DrawAble;
use crate::DrawOption;
use crate::ImageDrawOptions;
use crate::drawable::DrawCommand;
use crate::texture::Texture;
use crate::with_graphics;
use crate::Pt;

/// Rectangle bounds for defining sub-regions of images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bounds {
    /// X coordinate of the top-left corner.
    pub x: Pt,
    /// Y coordinate of the top-left corner.
    pub y: Pt,
    /// Width of the bounds.
    pub width: Pt,
    /// Height of the bounds.
    pub height: Pt,
}

impl Bounds {
    /// Creates new bounds with the specified dimensions.
    pub fn new(x: Pt, y: Pt, width: Pt, height: Pt) -> Self {
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
#[derive(Debug, Clone, Copy)]
pub struct Image {
    pub id: u32,
    pub x: Pt,
    pub y: Pt,
    pub width: Pt,
    pub height: Pt,
}

impl Image {
    pub(crate) fn index(self) -> usize {
        self.id as usize
    }
}

impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Image {}

impl std::hash::Hash for Image {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

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
    pub fn new_from_rgba8(width: Pt, height: Pt, rgba: &[u8]) -> anyhow::Result<Self> {
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
            let bg = g.create_texture_bind_group_from_texture(&texture);
            Ok(g.insert_image_entry(ImageEntry::new(texture, bg)))
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
    /// # use spottedcat::{Context, Image, ImageDrawOptions};
    /// # let mut context = Context::new();
    /// # let rgba = vec![255u8; 2 * 2 * 4];
    /// # let image = Image::new_from_rgba8(2, 2, &rgba).unwrap();
    /// let mut opts = ImageDrawOptions::default();
    /// opts.position = [spottedcat::Pt(100.0), spottedcat::Pt(100.0)];
    /// opts.scale = [2.0, 2.0];
    /// image.draw(&mut context, opts);
    /// ```
    pub fn draw(self, context: &mut crate::Context, options: crate::ImageDrawOptions) {
        context.push(crate::drawable::DrawCommand::Image(self, options));
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
    /// use spottedcat::{Context, Image, DrawAble, DrawOption, ImageDrawOptions};
    /// # let mut context = Context::new();
    ///
    /// // Load two images
    /// let rgba = vec![255u8; 2 * 2 * 4];
    /// let canvas = Image::new_from_rgba8(2, 2, &rgba).unwrap();
    /// let sprite = Image::new_from_rgba8(2, 2, &rgba).unwrap();
    ///
    /// // Create draw options for positioning sprite on canvas
    /// let option = DrawOption {
    ///     options: ImageDrawOptions {
    ///         position: [spottedcat::Pt(50.0), spottedcat::Pt(50.0)],  // Position on canvas
    ///         rotation: 0.0,
    ///         scale: [1.0, 1.0],
    ///     },
    /// };
    ///
    /// // Draw sprite onto canvas at specified position
    /// let sprite_drawable = DrawAble::Image(sprite);
    /// canvas.draw_sub(&mut context, sprite_drawable, option, None).unwrap();
    /// ```
    pub fn draw_sub(
        self,
        context: &mut crate::Context,
        drawable: DrawAble,
        option: DrawOption,
    ) -> anyhow::Result<()> {

        let drawable_with_options = match (drawable, option.clone()) {
                // 情况 1: 都是 Image
                (DrawAble::Image(img), DrawOption::Image(image_draw_options)) => {
                    if img == self {    
                        return Err(anyhow::anyhow!(
                            "cannot draw an image into itself; use a separate target image"
                        ));
                    }
                    DrawCommand::Image(img, image_draw_options) // 注意：这里没有分号，表示返回该值
                }

                // 情况 2: 都是 Text
                (DrawAble::Text(text), DrawOption::Text(text_option)) => {
                    DrawCommand::Text(text, text_option)    
                }

                // 情况 3: 类型不匹配 (例如 DrawAble 是 Image，但 Option 是 Text)
                (d, _) => {
                    return Err(anyhow::anyhow!(
                        "DrawOption 不匹配: 绘图对象是 {:?}, 但提供了错误的配置选项", 
                        d // 假设你的 Enum 实现了 Debug
                    ));
                }
            };

    

        context.push_offscreen(crate::OffscreenCommand {
            target: self,
            drawables: vec![drawable_with_options],
            option,
        });
        Ok(())
    }

    pub fn draw_to(
        self,
        context: &mut crate::Context,
        drawable: DrawAble,
        option: DrawOption,
    ) -> anyhow::Result<()> {
        self.draw_sub(context, drawable, option)
    }

    pub fn clear(self, color: [f32; 4]) -> anyhow::Result<()> {
        with_graphics(|g| g.clear_image(self, color))
    }

    pub fn copy_from(self, src: Image) -> anyhow::Result<()> {
        with_graphics(|g| g.copy_image(self, src))
    }

    pub fn bounds(self) -> anyhow::Result<Bounds> {
        Ok(Bounds {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        })
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
    pub(crate) bounds: Bounds,
    pub(crate) uvp: [[f32; 4]; 4],
    pub(crate) texture_bind_group: wgpu::BindGroup,
    pub(crate) visible: bool,
}

impl ImageEntry {
    pub(crate) fn new(texture: Texture, texture_bind_group: wgpu::BindGroup) -> Self {
        let bounds = Bounds {
            x: Pt(0.0),
            y: Pt(0.0),
            width: Pt::from(texture.0.width),
            height: Pt::from(texture.0.height),
        };
        let uvp = Self::uvp_from(texture.0.width, texture.0.height, bounds);
        Self {
            texture,
            bounds,
            uvp,
            texture_bind_group,
            visible: true,
        }
    }

    pub(crate) fn new_with_bounds(
        texture: Texture,
        texture_bind_group: wgpu::BindGroup,
        bounds: Bounds,
    ) -> Self {
        let uvp = Self::uvp_from(texture.0.width, texture.0.height, bounds);
        Self {
            texture,
            bounds,
            uvp,
            texture_bind_group,
            visible: true,
        }
    }

    fn uvp_from(tex_w: u32, tex_h: u32, bounds: Bounds) -> [[f32; 4]; 4] {
        let tw = tex_w as f32;
        let th = tex_h as f32;

        let u0 = bounds.x.as_f32() / tw;
        let v0 = bounds.y.as_f32() / th;
        let u1 = (bounds.x + bounds.width).as_f32() / tw;
        let v1 = (bounds.y + bounds.height).as_f32() / th;

        let sx = u1 - u0;
        let sy = v1 - v0;

        [
            [sx, 0.0, 0.0, 0.0],
            [0.0, sy, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [u0, v0, 0.0, 1.0],
        ]
    }
}
