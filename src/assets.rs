use crate::Context;

/// Registers an image shader and returns its shader id.
pub fn register_image_shader(ctx: &mut Context, wgsl_source: &str) -> u32 {
    ctx.register_image_shader(wgsl_source)
}

#[cfg(feature = "model-3d")]
/// Registers a 3D model shader and returns its shader id.
pub fn register_model_shader(ctx: &mut Context, wgsl_source: &str) -> u32 {
    ctx.register_model_shader(wgsl_source)
}

/// Returns a copy of the raw bytes for a registered font.
pub fn get_registered_font(ctx: &Context, font_id: u32) -> Option<Vec<u8>> {
    ctx.registry.fonts.get(&font_id).cloned()
}

/// Unregisters a font and clears any cached GPU state for it.
pub fn unregister_font(ctx: &mut Context, font_id: u32) {
    ctx.registry.fonts.remove(&font_id);
    if let Some(g) = ctx.runtime.graphics.as_mut() {
        g.font_cache.remove(&(font_id as u64));
        g.dirty_assets = true;
    }
}

/// Forces pending asset rebuild/re-upload to GPU to run immediately.
pub fn rebuild_assets(ctx: &mut Context) {
    if let Some(mut g) = ctx.runtime.graphics.take() {
        let _ = g.rebuild_textures(ctx);
        ctx.runtime.graphics = Some(g);
    }
}

/// Loads an asset from disk or from the platform-specific bundle.
pub fn load_asset(path: &str) -> anyhow::Result<Vec<u8>> {
    #[cfg(target_os = "android")]
    {
        use std::ffi::CString;
        if let Some(app) = crate::android::get_app() {
            let mut normalized_path = path;
            if normalized_path.starts_with("./") {
                normalized_path = &normalized_path[2..];
            }
            if normalized_path.starts_with("assets/") {
                normalized_path = &normalized_path[7..];
            }
            let asset_path = CString::new(normalized_path)?;
            let mut asset = app
                .asset_manager()
                .open(&asset_path)
                .ok_or_else(|| anyhow::anyhow!("Failed to open asset: {}", normalized_path))?;
            return Ok(asset.buffer()?.to_vec());
        }
    }

    Ok(std::fs::read(path)?)
}

/// Sets whether the current window background should be transparent.
pub fn set_background_transparent(ctx: &mut Context, transparent: bool) {
    if let Some(g) = ctx.runtime.graphics.as_mut() {
        g.set_transparent(transparent);
    }
}

/// Returns whether the current window background is transparent.
pub fn is_background_transparent(ctx: &Context) -> bool {
    ctx.runtime
        .graphics
        .as_ref()
        .map(|g| g.transparent())
        .unwrap_or(false)
}
