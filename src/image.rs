use crate::Pt;
use crate::with_graphics;
use std::sync::Arc;

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
    pub fn x(&self) -> Pt {
        self.x
    }
    pub fn y(&self) -> Pt {
        self.y
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
    pub fn width(&self) -> Pt {
        self.width
    }
    pub fn height(&self) -> Pt {
        self.height
    }
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Returns true if the image is ready for rendering.
    pub fn is_ready(&self) -> bool {
        with_graphics(|g| {
            g.images
                .get(self.index())
                .and_then(|v| v.as_ref())
                .map(|e| e.is_ready())
                .unwrap_or(false)
        })
        .unwrap_or(false)
    }
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
            .unwrap_or_else(|| Err(anyhow::anyhow!("Graphics not initialized")))
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
        .unwrap_or_else(|| Err(anyhow::anyhow!("Graphics not initialized")))
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
            .unwrap_or_else(|| Err(anyhow::anyhow!("Graphics not initialized")))
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
    /// opts = opts.with_position([spottedcat::Pt::from(100.0), spottedcat::Pt::from(100.0)]);
    /// opts = opts.with_scale([2.0, 2.0]);
    /// image.draw(&mut context, opts);
    /// ```
    pub fn draw(self, context: &mut crate::Context, options: crate::DrawOption) {
        context.push(crate::drawable::DrawCommand::Image(
            self.id,
            options,
            0,
            crate::ShaderOpts::default(),
            [self.width, self.height],
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
            [self.width, self.height],
        ));
    }

    pub fn clear(self, color: [f32; 4]) -> anyhow::Result<()> {
        with_graphics(|g| g.clear_image(self, color))
            .unwrap_or_else(|| Err(anyhow::anyhow!("Graphics not initialized")))
    }

    pub fn copy_from(self, src: Image) -> anyhow::Result<()> {
        with_graphics(|g| g.copy_image(self, src))
            .unwrap_or_else(|| Err(anyhow::anyhow!("Graphics not initialized")))
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
        with_graphics(|g| g.take_image_entry(self).is_some()).unwrap_or(false)
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

    pub fn with_clip_scope_draw<F, D>(
        self,
        context: &mut crate::Context,
        options: crate::DrawOption,
        draw: D,
        f: F,
    ) where
        D: FnOnce(Self, &mut crate::Context, crate::DrawOption),
        F: FnOnce(&mut crate::Context),
    {
        // First, draw the parent image to establish the clip region
        draw(self, context, options);

        // Then set up the clipping state for child elements
        let parent_opts_abs = if let Some(info) = context.last_image_draw_info(self.id) {
            info.opts
        } else {
            let state = context.current_draw_state();
            options.apply_state(&state)
        };

        let parent_pos_abs = parent_opts_abs.position();
        let parent_bounds = self.screen_bounds(parent_opts_abs);

        // Get the current origin to calculate relative position
        let current_origin = context.current_draw_state().position;
        let parent_pos_relative = [
            parent_pos_abs[0] - current_origin[0],
            parent_pos_abs[1] - current_origin[1],
        ];

        let state = crate::DrawState {
            position: parent_pos_relative,
            clip: Some([
                parent_bounds[0],
                parent_bounds[1],
                parent_bounds[2],
                parent_bounds[3],
            ]),
            shader_id: context.current_draw_state().shader_id,
            shader_opts: context.current_draw_state().shader_opts,
        };

        context.push_state(state);
        f(context);
        context.pop_state();
    }

    pub fn with_clip_scope<F>(self, context: &mut crate::Context, options: crate::DrawOption, f: F)
    where
        F: FnOnce(&mut crate::Context),
    {
        self.with_clip_scope_draw(context, options, |img, ctx, opts| img.draw(ctx, opts), f);
    }

    pub fn with_shader_scope<F>(
        self,
        context: &mut crate::Context,
        shader_id: u32,
        shader_opts: crate::ShaderOpts,
        f: F,
    ) where
        F: FnOnce(&mut crate::Context),
    {
        let mut state = crate::DrawState::default();
        state.shader_id = Some(shader_id);
        state.shader_opts = Some(shader_opts);
        // We do NOT set position or clip here.
        // position will be (0,0) so it won't shift children.
        // clip is None, so push_state will retain the current clip.

        context.push_state(state);
        f(context);
        context.pop_state();
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ImageEntry {
    pub(crate) atlas_index: Option<u32>,
    pub(crate) bounds: Bounds,
    pub(crate) uv_rect: Option<[f32; 4]>, // [u, v, w, h]
    pub(crate) visible: bool,
    pub(crate) raw_data: Option<Arc<[u8]>>,
    pub(crate) parent_id: Option<u32>,
}

impl ImageEntry {
    pub(crate) fn new(
        atlas_index: Option<u32>,
        bounds: Bounds,
        uv_rect: Option<[f32; 4]>,
        raw_data: Option<Arc<[u8]>>,
        parent_id: Option<u32>,
    ) -> Self {
        Self {
            atlas_index,
            bounds,
            uv_rect,
            visible: true,
            raw_data,
            parent_id,
        }
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.atlas_index.is_some() && self.uv_rect.is_some()
    }
}
