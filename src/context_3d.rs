#[cfg(feature = "model-3d")]
use std::collections::HashMap;

#[cfg(all(feature = "model-3d", feature = "effects"))]
use crate::FogSettings;
#[cfg(feature = "model-3d")]
use crate::context::Context;
#[cfg(feature = "model-3d")]
use crate::drawable::DrawCommand3D;

#[cfg(feature = "model-3d")]
#[derive(Debug)]
pub(crate) struct Model3dRuntime {
    pub(crate) draw_list: Vec<DrawCommand3D>,
    pub(crate) camera: crate::graphics::Camera,
}

#[cfg(feature = "model-3d")]
impl Default for Model3dRuntime {
    fn default() -> Self {
        Self {
            draw_list: Vec::new(),
            camera: crate::graphics::Camera::default(),
        }
    }
}

#[cfg(not(feature = "model-3d"))]
#[derive(Debug, Default)]
pub(crate) struct Model3dRuntime;

impl Model3dRuntime {
    pub(crate) fn begin_frame(&mut self) {
        #[cfg(feature = "model-3d")]
        self.draw_list.clear();
    }
}

#[cfg(feature = "model-3d")]
#[derive(Debug)]
pub(crate) struct Model3dRegistry {
    pub(crate) models: Vec<Option<crate::model::MeshDataPersistent>>,
    pub(crate) skins: Vec<Option<crate::graphics::SkinData>>,
    pub(crate) model_shaders: HashMap<u32, String>,
    pub(crate) next_mesh_id: u32,
    pub(crate) next_skin_id: u32,
    pub(crate) next_model_shader_id: u32,
}

#[cfg(feature = "model-3d")]
impl Default for Model3dRegistry {
    fn default() -> Self {
        Self {
            models: Vec::new(),
            skins: Vec::new(),
            model_shaders: HashMap::new(),
            next_mesh_id: 1,
            next_skin_id: 1,
            next_model_shader_id: 1,
        }
    }
}

#[cfg(not(feature = "model-3d"))]
#[derive(Debug, Default)]
pub(crate) struct Model3dRegistry;

#[cfg(feature = "model-3d")]
impl Context {
    /// Sets the 3D camera eye, target, and up vectors in one call.
    pub fn set_camera(&mut self, eye: [f32; 3], target: [f32; 3], up: [f32; 3]) {
        self.runtime.model_3d.camera.eye = eye;
        self.runtime.model_3d.camera.target = target;
        self.runtime.model_3d.camera.up = up;
    }

    /// Sets the camera eye position.
    pub fn set_camera_pos(&mut self, pos: [f32; 3]) {
        self.runtime.model_3d.camera.eye = pos;
    }

    /// Returns the current camera eye position.
    pub fn camera_position(&self) -> [f32; 3] {
        self.runtime.model_3d.camera.eye
    }

    /// Sets the camera target vector.
    pub fn set_camera_target(&mut self, x: f32, y: f32, z: f32) {
        self.runtime.model_3d.camera.target = [x, y, z];
    }

    /// Sets the camera up vector.
    pub fn set_camera_up(&mut self, x: f32, y: f32, z: f32) {
        self.runtime.model_3d.camera.up = [x, y, z];
    }

    /// Sets the camera field of view in degrees.
    pub fn set_camera_fov(&mut self, fov: f32) {
        self.runtime.model_3d.camera.fovy = fov;
    }

    /// Sets the camera aspect ratio.
    pub fn set_camera_aspect(&mut self, aspect: f32) {
        self.runtime.model_3d.camera.aspect = aspect;
    }

    /// Registers a 3D mesh and returns its stable mesh id.
    pub fn register_mesh(&mut self, vertices: &[crate::model::Vertex], indices: &[u32]) -> u32 {
        let id = self.registry.model_3d.next_mesh_id;
        self.registry.model_3d.next_mesh_id += 1;
        let mesh_data = crate::model::MeshDataPersistent {
            vertices: vertices.to_vec(),
            indices: indices.to_vec(),
        };

        while self.registry.model_3d.models.len() <= id as usize {
            self.registry.model_3d.models.push(None);
        }
        self.registry.model_3d.models[id as usize] = Some(mesh_data);
        self.registry.dirty_assets = true;
        id
    }

    /// Registers a custom WGSL shader block for 3D model rendering.
    pub fn register_model_shader(&mut self, user_functions: &str) -> u32 {
        let id = self.registry.model_3d.next_model_shader_id;
        self.registry.model_3d.next_model_shader_id += 1;
        self.registry
            .model_3d
            .model_shaders
            .insert(id, user_functions.to_string());
        self.registry.dirty_assets = true;
        id
    }

    /// Sets the ambient light color for the active 3D scene.
    pub fn set_ambient_light(&mut self, color: [f32; 4]) {
        if let Some(g) = self.runtime.graphics.as_mut() {
            g.ensure_model_3d().scene_globals.ambient_color = color;
        }
    }

    /// Alias for [`Context::set_ambient_light`].
    pub fn set_ambient(&mut self, color: [f32; 4]) {
        self.set_ambient_light(color);
    }

    /// Sets one of the scene lights. Indexes outside `0..4` are ignored.
    pub fn set_light(&mut self, index: usize, position: [f32; 4], color: [f32; 4]) {
        if let Some(g) = self.runtime.graphics.as_mut()
            && index < 4
        {
            g.ensure_model_3d().scene_globals.lights[index] =
                crate::graphics::Light { position, color };
        }
    }

    #[cfg(feature = "effects")]
    /// Applies global fog parameters to the active 3D scene.
    pub fn set_fog(&mut self, fog: FogSettings) {
        if let Some(g) = self.runtime.graphics.as_mut() {
            let scene_globals = &mut g.ensure_model_3d().scene_globals;
            scene_globals.fog_color = fog.color;
            scene_globals.fog_distance = [
                fog.distance_start,
                fog.distance_end,
                fog.distance_exponent,
                fog.distance_density,
            ];
            scene_globals.fog_height = [
                fog.height_base,
                fog.height_falloff,
                fog.height_exponent,
                fog.height_density,
            ];
            scene_globals.fog_params = [
                if fog.is_enabled() {
                    fog.effective_strength()
                } else {
                    0.0
                },
                0.0,
                0.0,
                0.0,
            ];
            scene_globals.fog_background_zenith = [
                fog.background.zenith_color[0],
                fog.background.zenith_color[1],
                fog.background.zenith_color[2],
                fog.background.zenith_mix,
            ];
            scene_globals.fog_background_horizon = [
                fog.background.horizon_color[0],
                fog.background.horizon_color[1],
                fog.background.horizon_color[2],
                fog.background.horizon_mix,
            ];
            scene_globals.fog_background_nadir = [
                fog.background.nadir_color[0],
                fog.background.nadir_color[1],
                fog.background.nadir_color[2],
                fog.background.nadir_mix,
            ];
            scene_globals.fog_background_params = [
                fog.background.horizon_glow_strength,
                fog.background.sky_fog_blend,
                fog.background.geometry_fog_blend,
                0.0,
            ];
            scene_globals.fog_sampling = [
                fog.sampling.min_height_samples.max(1) as f32,
                fog.sampling
                    .max_height_samples
                    .max(fog.sampling.min_height_samples.max(1)) as f32,
                fog.sampling.height_sample_scale.max(0.05),
                0.0,
            ];
        }
    }

    #[cfg(feature = "effects")]
    /// Resets global fog to the default disabled state.
    pub fn clear_fog(&mut self) {
        self.set_fog(FogSettings::default());
    }

    /// Creates a skin with the provided bones and bind-pose matrices.
    ///
    /// Returns `0` when 3D rendering is unavailable.
    pub fn create_skin(
        &mut self,
        bones: Vec<crate::graphics::Bone>,
        matrices: Vec<[[f32; 4]; 4]>,
    ) -> u32 {
        if let Some(mut g) = self.runtime.graphics.take() {
            let id = g.create_skin(self, bones, matrices);
            self.runtime.graphics = Some(g);
            id
        } else {
            0
        }
    }

    /// Updates the bone matrices for a previously created skin.
    pub fn update_bone_matrices(&mut self, skin_id: u32, matrices: &[[[f32; 4]; 4]]) {
        if let Some(mut g) = self.runtime.graphics.take() {
            g.update_bone_matrices(self, skin_id, matrices);
            self.runtime.graphics = Some(g);
        }
    }

    pub(crate) fn push_3d(&mut self, drawable: DrawCommand3D) {
        self.runtime.model_3d.draw_list.push(drawable);
    }
}
