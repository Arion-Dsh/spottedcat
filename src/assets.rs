use crate::Context;

pub fn register_image_shader(ctx: &mut Context, wgsl_source: &str) -> u32 {
    ctx.register_image_shader(wgsl_source)
}

pub fn register_model_shader(ctx: &mut Context, wgsl_source: &str) -> u32 {
    ctx.register_model_shader(wgsl_source)
}

pub fn register_font(ctx: &mut Context, font_data: Vec<u8>) -> u32 {
    ctx.register_font(font_data)
}

pub fn get_registered_font(ctx: &Context, font_id: u32) -> Option<Vec<u8>> {
    ctx.registry.fonts.get(&font_id).cloned()
}

pub fn unregister_font(ctx: &mut Context, font_id: u32) {
    ctx.registry.fonts.remove(&font_id);
    if let Some(g) = ctx.runtime.graphics.as_mut() {
        g.font_cache.remove(&(font_id as u64));
        g.dirty_assets = true;
    }
}

pub fn compress_assets(ctx: &mut Context) {
    if let Some(mut g) = ctx.runtime.graphics.take() {
        let _ = g.compress_assets(ctx);
        ctx.runtime.graphics = Some(g);
    }
}

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

pub fn set_background_transparent(ctx: &mut Context, transparent: bool) {
    if let Some(g) = ctx.runtime.graphics.as_mut() {
        g.set_transparent(transparent);
    }
}

pub fn is_background_transparent(ctx: &Context) -> bool {
    ctx.runtime
        .graphics
        .as_ref()
        .map(|g| g.transparent())
        .unwrap_or(false)
}
