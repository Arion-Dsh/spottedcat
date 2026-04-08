use crate::Image;
use image::GenericImageView;

/// Loads an image from a byte slice (PNG, JPEG, etc.).
pub fn load_image_from_bytes(ctx: &mut crate::Context, data: &[u8]) -> anyhow::Result<Image> {
    let img = image::load_from_memory(data)?;
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8();
    crate::image::create(ctx, w.into(), h.into(), &rgba)
}
