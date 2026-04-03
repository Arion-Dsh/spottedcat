use crate::DrawOption;
use crate::audio::AudioSystem;
use crate::context_3d::{Model3dRegistry, Model3dRuntime};
use crate::drawable::DrawCommand;
use crate::graphics::Graphics;
use crate::input::InputManager;
use crate::pt::Pt;
use crate::shader_opts::ShaderOpts;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone, Copy)]
pub(crate) struct DrawState {
    pub position: [Pt; 2],
    pub clip: Option<[Pt; 4]>,
    pub shader_id: Option<u32>,
    pub shader_opts: Option<ShaderOpts>,
    pub layer: i32,
}

impl Default for DrawState {
    fn default() -> Self {
        Self {
            position: [Pt(0.0), Pt(0.0)],
            clip: None,
            shader_id: None,
            shader_opts: None,
            layer: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LastImageDrawInfo {
    pub(crate) opts: DrawOption,
}

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
    pub(crate) state_stack: Vec<DrawState>,
    pub(crate) current_state: DrawState,
    pub(crate) last_image_opts: HashMap<u32, LastImageDrawInfo>,
    pub(crate) graphics: Option<Graphics>,
    pub(crate) audio: Option<AudioSystem>,
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
            state_stack: Vec::new(),
            current_state: DrawState::default(),
            last_image_opts: HashMap::new(),
            graphics: None,
            audio: None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ResourceRegistry {
    resources: ResourceMap,
    pub(crate) images: Vec<Option<crate::image::ImageEntry>>,
    #[cfg(feature = "model-3d")]
    pub(crate) model_3d: Model3dRegistry,
    pub(crate) fonts: HashMap<u32, Vec<u8>>,
    pub(crate) image_shaders: HashMap<u32, String>,
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
            images: Vec::new(),
            #[cfg(feature = "model-3d")]
            model_3d: Model3dRegistry::default(),
            fonts: HashMap::new(),
            image_shaders: HashMap::new(),
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

    pub fn with_audio<R>(&mut self, f: impl FnOnce(&mut AudioSystem) -> R) -> Option<R> {
        self.runtime.audio.as_mut().map(f)
    }

    fn register_defaults(&mut self) {
        self.register_image(Pt::from(1.0), Pt::from(1.0), &[255, 255, 255, 255]); // ID 1
        self.register_image(Pt::from(1.0), Pt::from(1.0), &[0, 0, 0, 255]);       // ID 2
        #[cfg(feature = "model-3d")]
        self.register_image(Pt::from(1.0), Pt::from(1.0), &[128, 128, 255, 255]); // ID 3 (Normal)

        let text_shader_src = r#"
            fn user_fs_hook() {
                let tint = user_globals[0];
                color = vec4<f32>(color.rgb * tint.rgb, color.a * tint.a);
            }
        "#;
        self.register_image_shader(text_shader_src);
    }

    pub fn set_window_logical_size(&mut self, width: Pt, height: Pt) {
        let w = Pt(width.0.max(0.0));
        let h = Pt(height.0.max(0.0));
        self.runtime.window_logical_size = (w, h);
    }

    pub fn window_logical_size(&self) -> (Pt, Pt) {
        self.runtime.window_logical_size
    }

    pub fn vw(&self, percent: f32) -> Pt {
        let (w, _) = self.runtime.window_logical_size;
        let p = if percent.is_finite() { percent } else { 0.0 };
        Pt::from(w.as_f32() * (p / 100.0))
    }

    pub fn vh(&self, percent: f32) -> Pt {
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

    pub fn get_resource<T: Any>(&self) -> Option<Rc<T>> {
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

    pub fn register_image(&mut self, width: Pt, height: Pt, rgba: &[u8]) -> crate::Image {
        let id = self.registry.next_image_id;
        self.registry.next_image_id += 1;
        let bounds = crate::image::Bounds::new(Pt(0.0), Pt(0.0), width, height);
        let entry = crate::image::ImageEntry::new(
            None,
            bounds,
            None,
            Some(std::sync::Arc::from(rgba)),
            None,
        );

        while self.registry.images.len() <= id as usize {
            self.registry.images.push(None);
        }
        self.registry.images[id as usize] = Some(entry);
        self.registry.dirty_assets = true;

        crate::Image {
            id,
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: bounds.height,
        }
    }

    pub fn register_sub_image(
        &mut self,
        image: crate::image::Image,
        bounds: crate::image::Bounds,
    ) -> anyhow::Result<u32> {
        let parent_id = image.id();
        let id = self.registry.next_image_id;
        self.registry.next_image_id += 1;

        let entry = crate::image::ImageEntry::new(None, bounds, None, None, Some(parent_id));

        while self.registry.images.len() <= id as usize {
            self.registry.images.push(None);
        }
        self.registry.images[id as usize] = Some(entry);
        self.registry.dirty_assets = true;
        Ok(id)
    }

    pub fn register_font(&mut self, font_data: Vec<u8>) -> u32 {
        let id = self.registry.next_font_id;
        self.registry.next_font_id += 1;
        self.registry.fonts.insert(id, font_data);
        self.registry.dirty_assets = true;
        id
    }

    pub fn register_image_shader(&mut self, user_functions: &str) -> u32 {
        let id = self.registry.next_image_shader_id;
        self.registry.next_image_shader_id += 1;
        self.registry
            .image_shaders
            .insert(id, user_functions.to_string());
        self.registry.dirty_assets = true;
        id
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
        self.runtime.state_stack.clear();
        self.runtime.current_state = DrawState::default();
        self.runtime.last_image_opts.clear();
    }

    pub fn input(&self) -> &InputManager {
        &self.runtime.input
    }

    pub fn input_mut(&mut self) -> &mut InputManager {
        &mut self.runtime.input
    }

    pub(crate) fn set_scale_factor(&mut self, scale_factor: f64) {
        self.runtime.scale_factor = scale_factor;
    }

    pub fn scale_factor(&self) -> f64 {
        self.runtime.scale_factor
    }

    pub(crate) fn push(&mut self, mut drawable: DrawCommand) {
        match &mut drawable {
            DrawCommand::Image(cmd) => {
                let opts = &mut cmd.opts;
                let shader_id = &mut cmd.shader_id;
                let shader_opts = &mut cmd.shader_opts;
                *opts = opts.apply_state(&self.runtime.current_state);
                *opts = opts.with_layer(opts.layer() + self.runtime.current_state.layer);
                if *shader_id == 0
                    && let Some(parent_shader_id) = self.runtime.current_state.shader_id
                {
                    *shader_id = parent_shader_id;
                    if let Some(parent_shader_opts) = self.runtime.current_state.shader_opts {
                        *shader_opts = parent_shader_opts;
                    }
                }
            }
            DrawCommand::Text(_, opts) => {
                *opts = opts.apply_state(&self.runtime.current_state);
                *opts = opts.with_layer(opts.layer() + self.runtime.current_state.layer);
            }
            DrawCommand::ClearImage(_, _) | DrawCommand::CopyImage(_, _) => {}
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

            let (vw, vh) = self.runtime.window_logical_size;
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

            self.runtime
                .last_image_opts
                .insert(id, LastImageDrawInfo { opts: *opts });
        }

        if std::env::var("SPOT_DEBUG_DRAW").is_ok() {
            match &drawable {
                DrawCommand::Image(cmd) => {
                    eprintln!(
                        "[spot][debug] draw image id={} shader_id={} pos={:?} clip={:?}",
                        cmd.id,
                        cmd.shader_id,
                        cmd.opts.position(),
                        cmd.opts.get_clip()
                    );
                }
                DrawCommand::Text(_, opts) => {
                    eprintln!(
                        "[spot][debug] draw text pos={:?} clip={:?}",
                        opts.position(),
                        opts.get_clip()
                    );
                }
                DrawCommand::ClearImage(_, _) | DrawCommand::CopyImage(_, _) => {}
            }
        }

        self.runtime.draw_list.push(drawable);
    }

    pub(crate) fn current_draw_state(&self) -> DrawState {
        self.runtime.current_state
    }

    pub(crate) fn last_image_draw_info(&self, image_id: u32) -> Option<LastImageDrawInfo> {
        self.runtime.last_image_opts.get(&image_id).copied()
    }

    pub(crate) fn push_state(&mut self, state: DrawState) {
        self.runtime.state_stack.push(self.runtime.current_state);
        self.runtime.current_state.position[0] += state.position[0];
        self.runtime.current_state.position[1] += state.position[1];
        self.runtime.current_state.layer += state.layer;

        if let Some(new_clip_abs) = state.clip {
            let merged_clip = if let Some(old_clip_abs) = self.runtime.current_state.clip {
                let x = old_clip_abs[0].as_f32().max(new_clip_abs[0].as_f32());
                let y = old_clip_abs[1].as_f32().max(new_clip_abs[1].as_f32());
                let right = (old_clip_abs[0].as_f32() + old_clip_abs[2].as_f32())
                    .min(new_clip_abs[0].as_f32() + new_clip_abs[2].as_f32());
                let bottom = (old_clip_abs[1].as_f32() + old_clip_abs[3].as_f32())
                    .min(new_clip_abs[1].as_f32() + new_clip_abs[3].as_f32());

                let w = (right - x).max(0.0);
                let h = (bottom - y).max(0.0);
                Some([Pt::from(x), Pt::from(y), Pt::from(w), Pt::from(h)])
            } else {
                Some(new_clip_abs)
            };
            self.runtime.current_state.clip = merged_clip;
        }

        if let Some(sid) = state.shader_id {
            self.runtime.current_state.shader_id = Some(sid);
        }
        if let Some(sopts) = state.shader_opts {
            self.runtime.current_state.shader_opts = Some(sopts);
        }
    }

    pub(crate) fn pop_state(&mut self) {
        if let Some(prev_state) = self.runtime.state_stack.pop() {
            self.runtime.current_state = prev_state;
        }
    }
}
