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

/// Rectangle bounds in pure physical GPU pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PixelBounds {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Bounds {
    /// Returns the width of the bounds.
    pub fn width(&self) -> Pt {
        self.width
    }

    /// Returns the height of the bounds.
    pub fn height(&self) -> Pt {
        self.height
    }

    /// Returns the X coordinate of the top-left corner.
    pub fn x(&self) -> Pt {
        self.x
    }

    /// Returns the Y coordinate of the top-left corner.
    pub fn y(&self) -> Pt {
        self.y
    }

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
/// An image references a sub-rectangle of a [`Texture`][crate::Texture].
/// It can be used as a source for drawing into other images, or as a render target itself.
///
/// Use `target.draw(ctx, &source, options)` to draw a source image into a target image.
/// The `screen` image provided in [`Spot::draw`][crate::Spot::draw] is the default window target.
#[derive(Debug, Clone, Copy)]
pub struct Image {
    pub(crate) id: u32,
    pub(crate) texture_id: u32,
    pub(crate) x: Pt,
    pub(crate) y: Pt,
    pub(crate) width: Pt,
    pub(crate) height: Pt,
    pub(crate) pixel_bounds: PixelBounds,
}

impl Image {
    /// Creates a new texture-backed full image from RGBA8 data.
    pub fn new(
        ctx: &mut crate::Context,
        width: Pt,
        height: Pt,
        rgba: &[u8],
    ) -> anyhow::Result<Self> {
        let pixel_width = width.0.round() as u32;
        let pixel_height = height.0.round() as u32;
        Ok(ctx.register_image(pixel_width, pixel_height, width, height, rgba))
    }

    /// Returns the logical width of the image.
    pub fn width(self) -> Pt {
        self.width
    }

    /// Returns the logical height of the image.
    pub fn height(self) -> Pt {
        self.height
    }

    /// Returns the internal unique identifier for this image.
    pub fn id(self) -> u32 {
        self.id
    }

    /// Returns the owning texture identifier.
    pub fn texture_id(self) -> u32 {
        self.texture_id
    }

    /// Returns whether the backing texture has been uploaded and is ready for drawing.
    pub fn is_ready(self, ctx: &crate::Context) -> bool {
        ctx.registry
            .textures
            .get(self.texture_id as usize)
            .and_then(|v| v.as_ref())
            .map(|entry| entry.is_ready(ctx.registry.gpu_generation))
            .unwrap_or(false)
    }

    /// Returns the physical pixel bounds of the image in the texture or atlas.
    pub fn pixel_bounds(self) -> PixelBounds {
        self.pixel_bounds
    }

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
    #[cfg(feature = "utils")]
    pub(crate) fn new_from_rgba8_with_pixels(
        ctx: &mut crate::Context,
        pixel_width: u32,
        pixel_height: u32,
        width: Pt,
        height: Pt,
        rgba: &[u8],
    ) -> anyhow::Result<Self> {
        Ok(crate::Texture::new_from_rgba8_with_pixels(
            ctx,
            pixel_width,
            pixel_height,
            width,
            height,
            rgba,
        )?
        .view())
    }

    /// Creates a full-view copy of an existing image.
    pub fn from_image(ctx: &mut crate::Context, image: Image) -> anyhow::Result<Self> {
        Self::sub_image(
            ctx,
            image,
            Bounds {
                x: image.x,
                y: image.y,
                width: image.width,
                height: image.height,
            },
        )
    }

    /// Creates a sub-image from a region of an existing image.
    pub fn sub_image(
        ctx: &mut crate::Context,
        image: Image,
        bounds: Bounds,
    ) -> anyhow::Result<Self> {
        if bounds.x.0 < 0.0
            || bounds.y.0 < 0.0
            || bounds.x.0 + bounds.width.0 > image.width.0 + 0.001
            || bounds.y.0 + bounds.height.0 > image.height.0 + 0.001
        {
            anyhow::bail!(
                "Sub-image bounds [x:{}, y:{}, w:{}, h:{}] are out of range for parent image [w:{}, h:{}]",
                bounds.x.0,
                bounds.y.0,
                bounds.width.0,
                bounds.height.0,
                image.width.0,
                image.height.0
            );
        }

        let id = ctx.register_sub_image(image, bounds)?;
        Ok(Self {
            id,
            texture_id: image.texture_id,
            x: image.x + bounds.x,
            y: image.y + bounds.y,
            width: bounds.width,
            height: bounds.height,
            pixel_bounds: ctx.registry.images[id as usize]
                .as_ref()
                .unwrap()
                .pixel_bounds,
        })
    }

    /// Draws a drawable into this image (as a target) with the specified options.
    ///
    /// `options.position()` is interpreted relative to this target image's top-left corner.
    /// To draw to the screen, use the `screen` image provided in `Spot::draw`.
    pub fn draw<D: crate::Drawable>(
        self,
        ctx: &mut crate::Context,
        drawable: D,
        options: D::Options,
    ) {
        drawable.draw_to(ctx, self, options);
    }

    /// Draws a source image into this target with a custom image shader.
    ///
    /// `options.position()` is interpreted relative to this target's top-left corner.
    pub fn draw_with_shader<S: Into<crate::Image>>(
        self,
        ctx: &mut crate::Context,
        source: S,
        shader_id: u32,
        options: crate::DrawOption,
        shader_opts: crate::ShaderOpts,
    ) {
        let source = source.into();
        let target_texture_id = ctx.resolve_target_texture_id(self);
        ctx.push(crate::drawable::DrawCommand::Image(Box::new(
            crate::drawable::ImageCommand {
                id: source.id,
                target_texture_id,
                opts: options,
                shader_id,
                shader_opts,
                size: [source.width, source.height],
            },
        )));
    }

    /// Returns the source-texture bounds of this image.
    pub fn bounds(self) -> Bounds {
        Bounds {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        }
    }

    /// Destroys the image.
    pub fn destroy(self, ctx: &mut crate::Context) -> bool {
        ctx.registry
            .images
            .get_mut(self.index())
            .and_then(|v| v.take())
            .is_some()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ImageEntry {
    pub(crate) texture_id: u32,
    pub(crate) bounds: Bounds,
    pub(crate) pixel_bounds: PixelBounds,
    pub(crate) visible: bool,
}

impl ImageEntry {
    pub(crate) fn new(texture_id: u32, bounds: Bounds, pixel_bounds: PixelBounds) -> Self {
        Self {
            texture_id,
            bounds,
            pixel_bounds,
            visible: true,
        }
    }
}

impl crate::Drawable for &Image {
    type Options = crate::DrawOption;

    fn draw_to(self, ctx: &mut crate::Context, target: crate::Image, options: Self::Options) {
        let target_texture_id = ctx.resolve_target_texture_id(target);
        ctx.push(crate::drawable::DrawCommand::Image(Box::new(
            crate::drawable::ImageCommand {
                id: self.id,
                target_texture_id,
                opts: options,
                shader_id: 0,
                shader_opts: crate::ShaderOpts::default(),
                size: [self.width, self.height],
            },
        )));
    }
}
