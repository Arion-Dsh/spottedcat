 
use crate::with_graphics;
use crate::Pt;

/// Rectangle bounds for defining sub-regions of images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bounds {
    /// X coordinate of the top-left corner.
    pub(crate) x: Pt,
    /// Y coordinate of the top-left corner.
    pub(crate) y: Pt,
    /// Width of the bounds.
    pub(crate) width: Pt,
    /// Height of the bounds.
    pub(crate) height: Pt,
}
impl Bounds {
    pub fn width(&self) -> Pt {
        self.width
    }
    pub fn height(&self) -> Pt {
        self.height
    }
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
    pub(crate) id: u32,
    pub(crate) x: Pt,
    pub(crate) y: Pt,
    pub(crate) width: Pt,
    pub(crate) height: Pt,
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
        with_graphics(|g| g.create_image(width, height, rgba))
    }

    /// Creates a copy of an existing image.
    ///
    /// # Arguments
    /// * `image` - The source image to copy
    pub fn new_from_image(image: Image) -> anyhow::Result<Self> {
        with_graphics(|g| {
            let bounds = g.image_bounds(image)?;
            g.create_sub_image(image, bounds)
        })
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
        with_graphics(|g| g.create_sub_image(image, bounds))
    }

    /// Draws this image to the context with the specified options.
    ///
    /// # Arguments
    /// * `context` - The drawing context to add this image to
    /// * `options` - Drawing options (position, rotation, scale)
    ///
    /// # Example
    /// ```no_run
    /// # use spottedcat::{Context, Image, DrawOption};
    /// # let mut context = Context::new();
    /// # let rgba = vec![255u8; 2 * 2 * 4];
    /// # let image = Image::new_from_rgba8(2u32.into(), 2u32.into(), &rgba).unwrap();
    /// let mut opts = DrawOption::default();
    /// opts.position = [spottedcat::Pt::from(100.0), spottedcat::Pt::from(100.0)];
    /// opts.scale = [2.0, 2.0];
    /// image.draw(&mut context, opts);
    /// ```
    pub fn draw(self, context: &mut crate::Context, options: crate::DrawOption) {
        context.push(crate::drawable::DrawCommand::Image(
            self.id,
            options,
            0,
            crate::ShaderOpts::default(),
        ));
    }

    pub fn draw_with_shader(
        self,
        context: &mut crate::Context,
        shader_id: u32,
        options: crate::DrawOption,
        shader_opts: crate::ShaderOpts,
    ) {
        context.push(crate::drawable::DrawCommand::Image(
            self.id,
            options,
            shader_id,
            shader_opts,
        ));
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

    /// Returns the global screen-space bounds of this image when drawn.
    /// 
    /// # Arguments
    /// * `options` - The same DrawOption used when calling `draw()`
    pub(crate) fn screen_bounds(self, options: crate::DrawOption) -> [Pt; 4] {
        let pos = options.position();
        let scale = options.scale();
        let x = pos[0];
        let y = pos[1];
        let w = self.width * scale[0];
        let h = self.height * scale[1];
        [x, y, w, h]
    }

    /// Draws a child image clipped to this image's screen-space bounds.
    ///
    /// The `child_options.position` is interpreted as **relative to the parent's position**.
    ///
    /// If the parent already has a clip area defined, the child will be clipped
    /// to the intersection of the parent's clip and the parent's bounds,
    /// enabling nested clipping.
    /// 
    /// Returns the **absolute screen-space DrawOption** used to draw the child,
    /// which can be passed as `parent_options` to subsequent `draw_image` or `draw_text` calls
    /// for deeper nesting.
    ///
    /// # Arguments
    /// * `context` - The drawing context
    /// * `parent_options` - The DrawOption used to draw THIS image (the parent)
    /// * `child` - The child image to draw
    /// * `child_options` - The DrawOption for the child image (position is relative)
    pub fn draw_image(
        self,
        context: &mut crate::Context,
        parent_options: crate::DrawOption,
        child: Image,
        child_options: crate::DrawOption,
    ) -> crate::DrawOption {
        let final_options = self.compute_child_options(parent_options, child_options);
        child.draw(context, final_options);
        final_options
    }

    /// Draws a child text clipped to this image's screen-space bounds.
    ///
    /// Similar to `draw_image`, but for `Text`.
    pub fn draw_text(
        self,
        context: &mut crate::Context,
        parent_options: crate::DrawOption,
        text: crate::Text,
        child_options: crate::DrawOption,
    ) -> crate::DrawOption {
        let final_options = self.compute_child_options(parent_options, child_options);
        text.draw(context, final_options);
        final_options
    }

    fn compute_child_options(
        self,
        parent_options: crate::DrawOption,
        mut child_options: crate::DrawOption,
    ) -> crate::DrawOption {
        let parent_bounds = self.screen_bounds(parent_options);
        
        // Convert child's relative position to absolute screen position
        let mut child_pos = child_options.position();
        let parent_pos = parent_options.position();
        child_pos[0] += parent_pos[0];
        child_pos[1] += parent_pos[1];
        child_options.set_position(child_pos);

        let final_clip = if let Some(parent_clip) = parent_options.clip() {
            // Compute intersection of parent's clip and parent's own bounds
            let x = parent_bounds[0].as_f32().max(parent_clip[0].as_f32());
            let y = parent_bounds[1].as_f32().max(parent_clip[1].as_f32());
            
            let parent_right = parent_bounds[0].as_f32() + parent_bounds[2].as_f32();
            let parent_bottom = parent_bounds[1].as_f32() + parent_bounds[3].as_f32();
            let clip_right = parent_clip[0].as_f32() + parent_clip[2].as_f32();
            let clip_bottom = parent_clip[1].as_f32() + parent_clip[3].as_f32();
            
            let right = parent_right.min(clip_right);
            let bottom = parent_bottom.min(clip_bottom);
            
            let width = (right - x).max(0.0);
            let height = (bottom - y).max(0.0);
            
            [Pt::from(x), Pt::from(y), Pt::from(width), Pt::from(height)]
        } else {
            parent_bounds
        };

        child_options.set_clip(Some(final_clip));
        child_options
    }
}

pub(crate) struct ImageEntry {
    pub(crate) atlas_index: u32,
    pub(crate) bounds: Bounds,
    pub(crate) uv_rect: [f32; 4], // [u, v, w, h]
    pub(crate) visible: bool,
}

impl ImageEntry {
    pub(crate) fn new(atlas_index: u32, bounds: Bounds, uv_rect: [f32; 4]) -> Self {
        Self {
            atlas_index,
            bounds,
            uv_rect,
            visible: true,
        }
    }
}
