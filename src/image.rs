use crate::Pt;

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
    pub fn is_ready(&self, ctx: &crate::Context) -> bool {
        ctx.registry
            .images
            .get(self.index())
            .and_then(|v| v.as_ref())
            .map(|e| e.is_ready())
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
    /// Errors
    /// Returns an error if the data length doesn't match width * height * 4.
    pub fn new_from_rgba8(
        ctx: &mut crate::Context,
        width: Pt,
        height: Pt,
        rgba: &[u8],
    ) -> anyhow::Result<Self> {
        let w = width.0.max(0.0);
        let h = height.0.max(0.0);
        let expected_len = (w * h * 4.0) as usize;
        if rgba.len() != expected_len {
            anyhow::bail!(
                "RGBA data length mismatch: expected {} ({}x{}x4), got {}",
                expected_len,
                w,
                h,
                rgba.len()
            );
        }
        let image = ctx.register_image(width, height, rgba);
        Ok(image)
    }

    /// Arguments
    /// * `image` - The source image to copy
    pub fn new_from_image(ctx: &mut crate::Context, image: Image) -> anyhow::Result<Self> {
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
    pub fn sub_image(
        ctx: &mut crate::Context,
        image: Image,
        bounds: Bounds,
    ) -> anyhow::Result<Self> {
        // Validation: Ensure sub-image bounds are within parent image bounds
        // Note: We use a small epsilon for float comparisons if needed, but here direct bounds check is usually fine
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
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: bounds.height,
        })
    }

    /// Draws this image to the context with the specified options.
    ///
    /// # Arguments
    /// * `context` - The drawing context to add this image to
    /// * `options` - Drawing options (position, rotation, scale)
    ///
    /// # Example
    /// # use spottedcat::{Context, Image, DrawOption, Pt};
    /// # let mut context = Context::new();
    /// # let rgba = vec![255u8; 2 * 2 * 4];
    /// # let image = Image::new_from_rgba8(&mut ctx, Pt::from(2.0), Pt::from(2.0), &rgba).unwrap();
    /// let mut opts = DrawOption::default();
    /// opts = opts.with_position([spottedcat::Pt::from(100.0), spottedcat::Pt::from(100.0)]);
    /// opts = opts.with_scale([2.0, 2.0]);
    /// image.draw(&mut ctx, opts);
    /// ```
    pub fn draw(self, ctx: &mut crate::Context, options: crate::DrawOption) {
        ctx.push(crate::drawable::DrawCommand::Image(Box::new(
            crate::drawable::ImageCommand {
                id: self.id,
                opts: options,
                shader_id: 0,
                shader_opts: crate::ShaderOpts::default(),
                size: [self.width, self.height],
            },
        )));
    }

    pub fn draw_with_shader(
        self,
        ctx: &mut crate::Context,
        shader_id: u32,
        options: crate::DrawOption,
        shader_opts: crate::ShaderOpts,
    ) {
        ctx.push(crate::drawable::DrawCommand::Image(Box::new(
            crate::drawable::ImageCommand {
                id: self.id,
                opts: options,
                shader_id,
                shader_opts,
                size: [self.width, self.height],
            },
        )));
    }

    pub fn clear(self, ctx: &mut crate::Context, color: [f32; 4]) -> anyhow::Result<()> {
        ctx.push(crate::drawable::DrawCommand::ClearImage(self.id, color));
        Ok(())
    }

    pub fn copy_from(self, ctx: &mut crate::Context, src: Image) -> anyhow::Result<()> {
        ctx.push(crate::drawable::DrawCommand::CopyImage(self.id, src.id));
        Ok(())
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
    pub fn destroy(self, ctx: &mut crate::Context) -> bool {
        ctx.registry
            .images
            .get_mut(self.index())
            .and_then(|v| v.take())
            .is_some()
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
        ctx: &mut crate::Context,
        options: crate::DrawOption,
        draw: D,
        f: F,
    ) where
        D: FnOnce(Self, &mut crate::Context, crate::DrawOption),
        F: FnOnce(&mut crate::Context),
    {
        // First, draw the parent image to establish the clip region
        draw(self, ctx, options);

        // Then set up the clipping state for child elements
        let parent_opts_abs = if let Some(info) = ctx.last_image_draw_info(self.id) {
            info.opts
        } else {
            let state = ctx.current_draw_state();
            options.apply_state(&state)
        };

        let parent_pos_abs = parent_opts_abs.position();
        let parent_bounds = self.screen_bounds(parent_opts_abs);

        // Get the current origin to calculate relative position
        let current_origin = ctx.current_draw_state().position;
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
            shader_id: ctx.current_draw_state().shader_id,
            shader_opts: ctx.current_draw_state().shader_opts,
            layer: 0,
        };

        ctx.push_state(state);
        f(ctx);
        ctx.pop_state();
    }

    pub fn with_clip_scope<F>(self, ctx: &mut crate::Context, options: crate::DrawOption, f: F)
    where
        F: FnOnce(&mut crate::Context),
    {
        self.with_clip_scope_draw(ctx, options, |img, ctx, opts| img.draw(ctx, opts), f);
    }

    pub fn with_shader_scope<F>(
        self,
        ctx: &mut crate::Context,
        shader_id: u32,
        shader_opts: crate::ShaderOpts,
        f: F,
    ) where
        F: FnOnce(&mut crate::Context),
    {
        let state = crate::DrawState {
            shader_id: Some(shader_id),
            shader_opts: Some(shader_opts),
            ..Default::default()
        };
        // We do NOT set position or clip here.
        // position will be (0,0) so it won't shift children.
        // clip is None, so push_state will retain the current clip.

        ctx.push_state(state);
        f(ctx);
        ctx.pop_state();
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
