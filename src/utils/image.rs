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
