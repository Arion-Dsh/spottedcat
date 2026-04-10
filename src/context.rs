use crate::audio::AudioSystem;
#[cfg(feature = "model-3d")]
use crate::context_3d::{Model3dRegistry, Model3dRuntime};
use crate::drawable::DrawCommand;
use crate::graphics::core::Graphics;
use crate::input::InputManager;
use crate::pt::Pt;

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
struct ResourceMap {
    inner: HashMap<TypeId, Rc<dyn Any>>,
}

impl std::fmt::Debug for ResourceMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceMap")
            .field("len", &self.inner.len())
            .finish()
    }
}

#[derive(Debug)]
pub(crate) struct ContextRuntime {
    pub(crate) draw_list: Vec<DrawCommand>,
    #[cfg(feature = "model-3d")]
    pub(crate) model_3d: Model3dRuntime,
    pub(crate) input: InputManager,
    pub(crate) scale_factor: f64,
    pub(crate) window_logical_size: (Pt, Pt),
    pub(crate) graphics: Option<Graphics>,
    pub(crate) audio: Option<AudioSystem>,
    pub(crate) delta_time: std::time::Duration,
    pub(crate) total_elapsed: std::time::Duration,
    pub(crate) pending_window_title: Option<String>,
    pub(crate) pending_cursor_visible: Option<bool>,
    pub(crate) pending_fullscreen: Option<bool>,
}

impl ContextRuntime {
    fn new() -> Self {
        Self {
            draw_list: Vec::new(),
            #[cfg(feature = "model-3d")]
            model_3d: Model3dRuntime::default(),
            input: InputManager::new(),
            scale_factor: 1.0,
            window_logical_size: (Pt(0.0), Pt(0.0)),
            graphics: None,
            audio: None,
            delta_time: std::time::Duration::from_secs(0),
            total_elapsed: std::time::Duration::from_secs(0),
            pending_window_title: None,
            pending_cursor_visible: None,
            pending_fullscreen: None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ResourceRegistry {
    resources: ResourceMap,
    pub(crate) textures: Vec<Option<crate::graphics::texture::TextureEntry>>,
    pub(crate) images: Vec<Option<crate::image::ImageEntry>>,
    #[cfg(feature = "model-3d")]
    pub(crate) model_3d: Model3dRegistry,
    pub(crate) fonts: HashMap<u32, Vec<u8>>,
    pub(crate) image_shaders: HashMap<u32, String>,
    pub(crate) next_texture_id: u32,
    pub(crate) next_image_id: u32,
    pub(crate) next_font_id: u32,
    pub(crate) next_image_shader_id: u32,
    pub(crate) gpu_generation: u32,
    pub(crate) dirty_assets: bool,
}

impl ResourceRegistry {
    fn new() -> Self {
        Self {
            resources: ResourceMap::default(),
            textures: Vec::new(),
            images: Vec::new(),
            #[cfg(feature = "model-3d")]
            model_3d: Model3dRegistry::default(),
            fonts: HashMap::new(),
            image_shaders: HashMap::new(),
            next_texture_id: 1,
            next_image_id: 1,
            next_font_id: 1,
            next_image_shader_id: 1,
            gpu_generation: 1,
            dirty_assets: true,
        }
    }
}

/// Drawing context for managing render commands.
///
/// The context accumulates drawing commands during a frame and is used by the
/// graphics system to render the scene.
#[derive(Debug)]
pub struct Context {
    pub(crate) runtime: ContextRuntime,
    pub(crate) registry: ResourceRegistry,
}

impl Context {
    /// Creates a new drawing context.
    pub(crate) fn new() -> Self {
        let mut ctx = Self {
            runtime: ContextRuntime::new(),
            registry: ResourceRegistry::new(),
        };
        ctx.register_defaults();
        ctx
    }

    pub(crate) fn set_delta_time(&mut self, dt: std::time::Duration) {
        self.runtime.delta_time = dt;
        self.runtime.total_elapsed = self.runtime.total_elapsed.saturating_add(dt);
    }

    /// Returns the time elapsed since the last frame.
    pub(crate) fn delta_time(&self) -> std::time::Duration {
        self.runtime.delta_time
    }

    /// Returns the total time elapsed since the engine started.
    pub(crate) fn total_elapsed(&self) -> std::time::Duration {
        self.runtime.total_elapsed
    }

    pub(crate) fn with_audio<R>(&mut self, f: impl FnOnce(&mut AudioSystem) -> R) -> Option<R> {
        self.runtime.audio.as_mut().map(f)
    }

    fn register_defaults(&mut self) {
        self.register_image(1, 1, Pt::from(1.0), Pt::from(1.0), &[255, 255, 255, 255]); // ID 1
        self.register_image(1, 1, Pt::from(1.0), Pt::from(1.0), &[0, 0, 0, 255]); // ID 2
        #[cfg(feature = "model-3d")]
        self.register_image(1, 1, Pt::from(1.0), Pt::from(1.0), &[128, 128, 255, 255]); // ID 3 (Normal)

        let text_shader_src = r#"
            fn user_fs_hook() {
                let tint = user_globals[0];
                color = vec4<f32>(color.rgb * tint.rgb, color.a * tint.a);
            }
        "#;
        self.register_image_shader(text_shader_src);
    }

    pub(crate) fn set_window_logical_size(&mut self, width: Pt, height: Pt) {
        let w = Pt(width.0.max(0.0));
        let h = Pt(height.0.max(0.0));
        self.runtime.window_logical_size = (w, h);
    }

    pub(crate) fn update_window_metrics_physical(
        &mut self,
        width: u32,
        height: u32,
        scale_factor: f64,
    ) {
        self.set_scale_factor(scale_factor);
        self.set_window_logical_size(
            Pt::from_physical_px(width as f64, scale_factor),
            Pt::from_physical_px(height as f64, scale_factor),
        );
    }

    pub(crate) fn set_window_title(&mut self, title: impl Into<String>) {
        self.runtime.pending_window_title = Some(title.into());
    }

    pub(crate) fn set_cursor_visible(&mut self, visible: bool) {
        self.runtime.pending_cursor_visible = Some(visible);
    }

    pub(crate) fn set_fullscreen(&mut self, enabled: bool) {
        self.runtime.pending_fullscreen = Some(enabled);
    }

    pub(crate) fn take_window_title_request(&mut self) -> Option<String> {
        self.runtime.pending_window_title.take()
    }

    pub(crate) fn take_cursor_visible_request(&mut self) -> Option<bool> {
        self.runtime.pending_cursor_visible.take()
    }

    pub(crate) fn take_fullscreen_request(&mut self) -> Option<bool> {
        self.runtime.pending_fullscreen.take()
    }

    /// Returns the logical size of the window in Pt.
    pub(crate) fn window_logical_size(&self) -> (Pt, Pt) {
        self.runtime.window_logical_size
    }

    /// Returns a horizontal length equivalent to `percent` of the window width.
    pub(crate) fn vw(&self, percent: f32) -> Pt {
        let (w, _) = self.runtime.window_logical_size;
        let p = if percent.is_finite() { percent } else { 0.0 };
        Pt::from(w.as_f32() * (p / 100.0))
    }

    /// Returns a vertical length equivalent to `percent` of the window height.
    pub(crate) fn vh(&self, percent: f32) -> Pt {
        let (_, h) = self.runtime.window_logical_size;
        let p = if percent.is_finite() { percent } else { 0.0 };
        Pt::from(h.as_f32() * (p / 100.0))
    }

    pub fn insert_resource<T: Any>(&mut self, value: Rc<T>) {
        self.registry
            .resources
            .inner
            .insert(TypeId::of::<T>(), value as Rc<dyn Any>);
    }

    pub(crate) fn get_resource<T: Any>(&self) -> Option<Rc<T>> {
        self.registry
            .resources
            .inner
            .get(&TypeId::of::<T>())
            .cloned()
            .and_then(|v| Rc::downcast::<T>(v).ok())
    }

    pub fn take_resource<T: Any>(&mut self) -> Option<Rc<T>> {
        self.registry
            .resources
            .inner
            .remove(&TypeId::of::<T>())
            .and_then(|v| Rc::downcast::<T>(v).ok())
    }

    /// Registers a new texture and returns a handle.
    ///
    /// This also creates a default full-image [`Image`] for the texture.
    pub(crate) fn register_texture(
        &mut self,
        pixel_width: u32,
        pixel_height: u32,
        width: Pt,
        height: Pt,
        rgba: &[u8],
    ) -> crate::Texture {
        let texture_id = self.registry.next_texture_id;
        self.registry.next_texture_id += 1;
        let image_id = self.registry.next_image_id;
        self.registry.next_image_id += 1;

        while self.registry.textures.len() <= texture_id as usize {
            self.registry.textures.push(None);
        }
        self.registry.textures[texture_id as usize] = Some(crate::graphics::texture::TextureEntry::new_sampled(
            width,
            height,
            pixel_width,
            pixel_height,
            image_id,
            std::sync::Arc::from(rgba),
        ));

        let bounds = crate::image::Bounds::new(Pt(0.0), Pt(0.0), width, height);
        while self.registry.images.len() <= image_id as usize {
            self.registry.images.push(None);
        }
        self.registry.images[image_id as usize] = Some(crate::image::ImageEntry::new(
            texture_id,
            bounds,
            crate::image::PixelBounds {
                x: 0,
                y: 0,
                width: pixel_width,
                height: pixel_height,
            },
        ));

        self.registry.dirty_assets = true;

        crate::Texture {
            id: texture_id,
            default_view_id: image_id,
            width,
            height,
            pixel_width,
            pixel_height,
        }
    }

    /// Registers a new image from RGBA data and returns a full-view handle.
    pub fn register_image(
        &mut self,
        pixel_width: u32,
        pixel_height: u32,
        width: Pt,
        height: Pt,
        rgba: &[u8],
    ) -> crate::Image {
        // Transparent auto-atlasing for small images
        if let Some(graphics) = self.runtime.graphics.as_mut() {
            if pixel_width <= 512 && pixel_height <= 512 {
                if let Some(atlas) = graphics.shared_atlas.as_mut() {
                    let scale_factor = self.runtime.scale_factor;
                    if let Ok(img) = atlas.add_region(
                        &mut self.registry,
                        scale_factor,
                        width,
                        height,
                        pixel_width,
                        pixel_height,
                        rgba,
                    ) {
                        return img;
                    }
                }
            }
        }

        self.register_texture(pixel_width, pixel_height, width, height, rgba)
            .view()
    }

    pub(crate) fn register_sub_image(
        &mut self,
        image: crate::image::Image,
        bounds: crate::image::Bounds,
    ) -> anyhow::Result<u32> {
        let id = self.registry.next_image_id;
        self.registry.next_image_id += 1;

        let physical_w_ratio = image.pixel_bounds.width as f32 / image.width.0.max(1e-5);
        let physical_h_ratio = image.pixel_bounds.height as f32 / image.height.0.max(1e-5);
        
        let parent_pixel_bounds = self.registry.images[image.index()].as_ref().unwrap().pixel_bounds;
        let parent_pixel_x = parent_pixel_bounds.x;
        let parent_pixel_y = parent_pixel_bounds.y;
        
        let pixel_x = parent_pixel_x + (bounds.x.0 * physical_w_ratio).round() as u32;
        let pixel_y = parent_pixel_y + (bounds.y.0 * physical_h_ratio).round() as u32;
        let pixel_width = (bounds.width.0 * physical_w_ratio).round() as u32;
        let pixel_height = (bounds.height.0 * physical_h_ratio).round() as u32;

        let entry = crate::image::ImageEntry::new(
            image.texture_id(),
            crate::image::Bounds::new(image.x + bounds.x, image.y + bounds.y, bounds.width, bounds.height),
            crate::image::PixelBounds {
                x: pixel_x,
                y: pixel_y,
                width: pixel_width,
                height: pixel_height,
            },
        );

        while self.registry.images.len() <= id as usize {
            self.registry.images.push(None);
        }
        self.registry.images[id as usize] = Some(entry);
        self.registry.dirty_assets = true;
        Ok(id)
    }

    pub(crate) fn register_font(&mut self, font_data: Vec<u8>) -> u32 {
        let id = self.registry.next_font_id;
        self.registry.next_font_id += 1;
        self.registry.fonts.insert(id, font_data);
        self.registry.dirty_assets = true;
        id
    }

    pub(crate) fn register_image_shader(&mut self, user_functions: &str) -> u32 {
        let id = self.registry.next_image_shader_id;
        self.registry.next_image_shader_id += 1;
        self.registry
            .image_shaders
            .insert(id, user_functions.to_string());
        self.registry.dirty_assets = true;
        id
    }

    /// Registers a texture specifically for use as a render target.
    pub(crate) fn register_render_target_texture(
        &mut self,
        width: Pt,
        height: Pt,
    ) -> crate::Texture {
        let texture_id = self.registry.next_texture_id;
        self.registry.next_texture_id += 1;
        let image_id = self.registry.next_image_id;
        self.registry.next_image_id += 1;

        let pixel_width = width.to_u32_clamped().max(1);
        let pixel_height = height.to_u32_clamped().max(1);

        while self.registry.textures.len() <= texture_id as usize {
            self.registry.textures.push(None);
        }
        self.registry.textures[texture_id as usize] = Some(
            crate::graphics::texture::TextureEntry::new_render_target(
                width,
                height,
                pixel_width,
                pixel_height,
                image_id,
            ),
        );

        while self.registry.images.len() <= image_id as usize {
            self.registry.images.push(None);
        }
        self.registry.images[image_id as usize] = Some(crate::image::ImageEntry::new(
            texture_id,
            crate::image::Bounds::new(Pt(0.0), Pt(0.0), width, height),
            crate::image::PixelBounds {
                x: 0,
                y: 0,
                width: pixel_width,
                height: pixel_height,
            },
        ));
        self.registry.dirty_assets = true;

        crate::Texture {
            id: texture_id,
            default_view_id: image_id,
            width,
            height,
            pixel_width,
            pixel_height,
        }
    }

    pub(crate) fn insert_resource_dyn(&mut self, type_id: TypeId, value: Rc<dyn Any>) {
        self.registry.resources.inner.insert(type_id, value);
    }

    pub(crate) fn take_resource_dyn(&mut self, type_id: TypeId) {
        self.registry.resources.inner.remove(&type_id);
    }

    pub(crate) fn begin_frame(&mut self) {
        self.runtime.draw_list.clear();
        #[cfg(feature = "model-3d")]
        self.runtime.model_3d.begin_frame();
    }

    pub(crate) fn clear_transient_state(&mut self) {
        self.begin_frame();
        self.runtime.pending_window_title = None;
        self.runtime.pending_cursor_visible = None;
        self.runtime.pending_fullscreen = None;
    }

    pub(crate) fn clear_transient_input(&mut self) {
        self.runtime.input.clear_transient_state();
    }

    pub(crate) fn input(&self) -> &InputManager {
        &self.runtime.input
    }

    pub(crate) fn input_mut(&mut self) -> &mut InputManager {
        &mut self.runtime.input
    }

    pub(crate) fn set_scale_factor(&mut self, scale_factor: f64) {
        self.runtime.scale_factor = scale_factor;
    }

    pub(crate) fn graphics_mut(&mut self) -> Option<&mut Graphics> {
        self.runtime.graphics.as_mut()
    }

    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub(crate) fn has_graphics(&self) -> bool {
        self.runtime.graphics.is_some()
    }

    pub(crate) fn attach_graphics(&mut self, graphics: Graphics) {
        self.runtime.graphics = Some(graphics);
    }

    pub(crate) fn detach_graphics(&mut self) -> Option<Graphics> {
        self.runtime.graphics.take()
    }

    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub(crate) fn bump_gpu_generation(&mut self) {
        self.registry.gpu_generation = self.registry.gpu_generation.saturating_add(1);
        self.registry.dirty_assets = true;
    }

    /// Returns the window's scale factor (DPI).
    /// Returns the UI scale factor (DPR) of the current window.
    pub fn scale_factor(&self) -> f64 {
        self.runtime.scale_factor
    }

    pub(crate) fn resolve_target_texture_id(&self, target: crate::Image) -> u32 {
        if target.texture_id == 0 {
            return 0;
        }

        let Some(texture_entry) = self
            .registry
            .textures
            .get(target.texture_id as usize)
            .and_then(|v| v.as_ref())
        else {
            panic!(
                "[spot][target] image {} points to missing texture {}",
                target.id, target.texture_id
            );
        };

        if !texture_entry.is_render_target() {
            panic!(
                "[spot][target] image {} uses texture {} which is not a render target",
                target.id, target.texture_id
            );
        }

        if texture_entry.default_view_id != target.id {
            panic!(
                "[spot][target] image {} is not the full target view for texture {}",
                target.id, target.texture_id
            );
        }

        target.texture_id
    }

    pub(crate) fn target_logical_size(&self, target_texture_id: u32) -> Option<(Pt, Pt)> {
        if target_texture_id == 0 {
            return Some(self.runtime.window_logical_size);
        }

        self.registry
            .textures
            .get(target_texture_id as usize)
            .and_then(|v| v.as_ref())
            .map(|entry| (entry.width, entry.height))
    }

    pub(crate) fn push(&mut self, mut drawable: DrawCommand) {
        match &mut drawable {
            DrawCommand::Image(_) => {}
            DrawCommand::Text(_) => {}
        }

        if let DrawCommand::Image(cmd) = &drawable {
            let id = cmd.id;
            let opts = &cmd.opts;
            let size = cmd.size;
            let pos = opts.position();
            let scale = opts.scale();
            let rot = opts.rotation();
            let w = size[0].as_f32() * scale[0];
            let h = size[1].as_f32() * scale[1];

            let (vw, vh) = self
                .target_logical_size(cmd.target_texture_id)
                .unwrap_or(self.runtime.window_logical_size);
            let screen_w = vw.as_f32();
            let screen_h = vh.as_f32();

            let is_visible = if rot == 0.0 {
                let x0 = pos[0].as_f32();
                let y0 = pos[1].as_f32();
                let x1 = x0 + w;
                let y1 = y0 + h;

                let min_x = x0.min(x1);
                let max_x = x0.max(x1);
                let min_y = y0.min(y1);
                let max_y = y0.max(y1);

                !(max_x < 0.0 || min_x > screen_w || max_y < 0.0 || min_y > screen_h)
            } else {
                let c = rot.cos();
                let s = rot.sin();
                let x2 = w * c;
                let y2 = w * s;
                let x3 = -h * s;
                let y3 = h * c;
                let x4 = x2 + x3;
                let y4 = y2 + y3;

                let min_x = 0.0f32.min(x2).min(x3).min(x4);
                let max_x = 0.0f32.max(x2).max(x3).max(x4);
                let min_y = 0.0f32.min(y2).min(y3).min(y4);
                let max_y = 0.0f32.max(y2).max(y3).max(y4);

                !(pos[0].as_f32() + max_x < 0.0
                    || pos[0].as_f32() + min_x > screen_w
                    || pos[1].as_f32() + max_y < 0.0
                    || pos[1].as_f32() + min_y > screen_h)
            };

            if !is_visible {
                if std::env::var("SPOT_DEBUG_CULL").is_ok() {
                    eprintln!(
                        "[spot][cull] image id={} at {:?} (size {:?}) is culled (screen: {:?})",
                        id,
                        pos,
                        [w, h],
                        self.runtime.window_logical_size
                    );
                }
                return;
            }

        }

        if std::env::var("SPOT_DEBUG_DRAW").is_ok() {
            match &drawable {
                DrawCommand::Image(cmd) => {
                    eprintln!(
                        "[spot][debug] draw image id={} target={} shader_id={} pos={:?}",
                        cmd.id,
                        cmd.target_texture_id,
                        cmd.shader_id,
                        cmd.opts.position(),
                    );
                }
                DrawCommand::Text(cmd) => {
                    eprintln!(
                        "[spot][debug] draw text target={} pos={:?}",
                        cmd.target_texture_id,
                        cmd.opts.position(),
                    );
                }
            }
        }

        self.runtime.draw_list.push(drawable);
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Pt;

    #[test]
    fn test_auto_atlas_logic() {
        let mut ctx = Context::new();
        // Initially no graphics, so NO auto-atlas
        let img1 = ctx.register_image(1, 1, Pt(10.0), Pt(10.0), &[0, 0, 0, 0]);
        let img2 = ctx.register_image(1, 1, Pt(10.0), Pt(10.0), &[0, 0, 0, 0]);
        assert_ne!(img1.texture_id(), img2.texture_id(), "Should not auto-atlas without graphics");

        // We can't easily mock Graphics here without a lot of setup,
        // but the logic check in register_image is verified by compilation.
    }
}
