use std::collections::HashMap;

use crate::model::{Bone, Material, Model, ModelPart, Vertex};
use gltf::animation::Interpolation;
use gltf::animation::util::ReadOutputs;

#[derive(Debug, Clone)]
struct Vec3Track {
    keyframes: Vec<f32>,
    values: Vec<[f32; 3]>,
    interpolation: Interpolation,
}

#[derive(Debug, Clone)]
struct QuatTrack {
    keyframes: Vec<f32>,
    values: Vec<[f32; 4]>,
    interpolation: Interpolation,
}

#[derive(Debug, Clone, Copy)]
struct NodeTransform {
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
}

impl Default for NodeTransform {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

#[derive(Debug, Clone)]
struct AnimationChannelSet {
    translation_tracks: HashMap<usize, Vec3Track>,
    rotation_tracks: HashMap<usize, QuatTrack>,
    scale_tracks: HashMap<usize, Vec3Track>,
}

#[derive(Debug, Clone)]
pub struct AnimationClip {
    name: Option<String>,
    duration_sec: f32,
    channels: AnimationChannelSet,
}

impl AnimationClip {
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn duration_sec(&self) -> f32 {
        self.duration_sec
    }
}

#[derive(Debug, Clone)]
struct SkinBinding {
    skin_id: u32,
    node_parents: Vec<Option<usize>>,
    default_transforms: Vec<NodeTransform>,
    joint_nodes: Vec<usize>,
    inverse_bind_matrices: Vec<[[f32; 4]; 4]>,
}

#[derive(Debug, Clone)]
pub struct AnimatedModel {
    model: Model,
    skin: Option<SkinBinding>,
    clips: Vec<AnimationClip>,
    current_clip: usize,
    current_time_sec: f32,
    playback_speed: f32,
    looping: bool,
    playing: bool,
}

impl AnimatedModel {
    pub fn from_model(model: Model) -> Self {
        Self {
            model,
            skin: None,
            clips: Vec::new(),
            current_clip: 0,
            current_time_sec: 0.0,
            playback_speed: 1.0,
            looping: true,
            playing: true,
        }
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    pub fn clip_count(&self) -> usize {
        self.clips.len()
    }

    pub fn clips(&self) -> &[AnimationClip] {
        &self.clips
    }

    pub fn clip_name(&self, index: usize) -> Option<&str> {
        self.clips.get(index).and_then(AnimationClip::name)
    }

    pub fn current_clip_name(&self) -> Option<&str> {
        self.clip_name(self.current_clip)
    }

    pub fn find_clip_named(&self, name: &str) -> Option<usize> {
        let needle = name.trim().to_ascii_lowercase();
        self.clips.iter().position(|clip| {
            clip.name
                .as_deref()
                .map(|clip_name| {
                    let hay = clip_name.to_ascii_lowercase();
                    hay == needle || hay.contains(&needle)
                })
                .unwrap_or(false)
        })
    }

    pub fn current_clip_index(&self) -> usize {
        self.current_clip
    }

    pub fn current_time_sec(&self) -> f32 {
        self.current_time_sec
    }

    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }

    pub fn looping(&self) -> bool {
        self.looping
    }

    pub fn set_playback_speed(&mut self, speed: f32) {
        self.playback_speed = speed;
    }

    pub fn playback_speed(&self) -> f32 {
        self.playback_speed
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn play(&mut self) {
        self.playing = true;
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn play_clip(&mut self, ctx: &mut crate::Context, index: usize) -> bool {
        if index >= self.clips.len() {
            return false;
        }

        self.current_clip = index;
        self.current_time_sec = 0.0;
        self.playing = true;
        self.apply_current_pose(ctx);
        true
    }

    pub fn play_clip_named(&mut self, ctx: &mut crate::Context, name: &str) -> bool {
        if let Some(index) = self.find_clip_named(name) {
            self.play_clip(ctx, index)
        } else {
            false
        }
    }

    pub fn play_first_matching_clip(
        &mut self,
        ctx: &mut crate::Context,
        preferred_names: &[&str],
    ) -> Option<usize> {
        for name in preferred_names {
            if let Some(index) = self.find_clip_named(name)
                && self.play_clip(ctx, index)
            {
                return Some(index);
            }
        }
        None
    }

    pub fn seek(&mut self, ctx: &mut crate::Context, time_sec: f32) {
        self.current_time_sec = time_sec.max(0.0);
        self.apply_current_pose(ctx);
    }

    pub fn update(&mut self, ctx: &mut crate::Context, dt_sec: f32) {
        if self.playing && !self.clips.is_empty() {
            self.current_time_sec += dt_sec.max(0.0) * self.playback_speed;
        }
        self.apply_current_pose(ctx);
    }

    pub fn apply_current_pose(&mut self, ctx: &mut crate::Context) {
        let Some(skin) = &self.skin else {
            return;
        };
        let Some(clip) = self.clips.get(self.current_clip) else {
            return;
        };

        let sample_time =
            sample_time_for_clip(self.current_time_sec, clip.duration_sec, self.looping);
        let mut local_transforms = skin.default_transforms.clone();

        for (node_idx, track) in &clip.channels.translation_tracks {
            local_transforms[*node_idx].translation = sample_vec3_track(track, sample_time);
        }
        for (node_idx, track) in &clip.channels.rotation_tracks {
            local_transforms[*node_idx].rotation = sample_quat_track(track, sample_time);
        }
        for (node_idx, track) in &clip.channels.scale_tracks {
            local_transforms[*node_idx].scale = sample_vec3_track(track, sample_time);
        }

        let local_matrices: Vec<_> = local_transforms
            .iter()
            .map(|transform| compose_transform(*transform))
            .collect();
        let mut world_cache = vec![None; local_matrices.len()];

        let bone_matrices: Vec<_> = skin
            .joint_nodes
            .iter()
            .zip(skin.inverse_bind_matrices.iter())
            .map(|(node_idx, inverse_bind)| {
                let world = compute_world_matrix(
                    *node_idx,
                    &skin.node_parents,
                    &local_matrices,
                    &mut world_cache,
                );
                crate::math::mat4::multiply(world, *inverse_bind)
            })
            .collect();

        ctx.update_bone_matrices(skin.skin_id, &bone_matrices);
    }
}

impl crate::Drawable for &AnimatedModel {
    type Options = crate::DrawOption3D;

    fn draw_to(self, ctx: &mut crate::Context, target: crate::Image, options: Self::Options) {
        if let Some(skin) = &self.skin {
            self.model.draw_skinned(ctx, target, options, skin.skin_id);
        } else {
            target.draw(ctx, &self.model, options);
        }
    }
}

/// Loads a GLB or glTF file from a byte slice into a Model.
pub fn load_gltf_from_bytes(ctx: &mut crate::Context, data: &[u8]) -> anyhow::Result<Model> {
    let imported = import_gltf(ctx, data)?;
    Ok(imported.model)
}

/// Loads a GLB or glTF file into an animated model wrapper.
///
/// The returned value works for both static and animated assets. If the asset contains
/// a skin and one or more animation clips, call [`AnimatedModel::update`] each frame
/// and render it with `target.draw(ctx, &animated_model, opts)`.
pub fn load_animated_gltf_from_bytes(
    ctx: &mut crate::Context,
    data: &[u8],
) -> anyhow::Result<AnimatedModel> {
    let imported = import_gltf(ctx, data)?;
    let mut animated = AnimatedModel {
        model: imported.model,
        skin: imported.skin,
        clips: imported.clips,
        current_clip: 0,
        current_time_sec: 0.0,
        playback_speed: 1.0,
        looping: true,
        playing: true,
    };
    animated.apply_current_pose(ctx);
    Ok(animated)
}

struct ImportedGltf {
    model: Model,
    skin: Option<SkinBinding>,
    clips: Vec<AnimationClip>,
}

fn import_gltf(ctx: &mut crate::Context, data: &[u8]) -> anyhow::Result<ImportedGltf> {
    let (document, buffers, images) = gltf::import_slice(data)?;

    // 1. Process images (textures)
    let mut spot_images = Vec::new();
    for img in images {
        let width = img.width;
        let height = img.height;

        // Ensure RGBA8 format
        let rgba = match img.format {
            gltf::image::Format::R8G8B8A8 => img.pixels,
            gltf::image::Format::R8G8B8 => {
                let mut rgba = Vec::with_capacity(img.pixels.len() / 3 * 4);
                for rgb in img.pixels.chunks_exact(3) {
                    rgba.extend_from_slice(rgb);
                    rgba.push(255);
                }
                rgba
            }
            _ => {
                // Fallback attempt or error
                img.pixels
            }
        };

        let spot_img = crate::Image::new(ctx, width.into(), height.into(), &rgba)?;
        spot_images.push(spot_img);
    }

    let mut model_parts = Vec::new();
    let node_count = document
        .nodes()
        .map(|node| node.index())
        .max()
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let mut node_parents = vec![None; node_count];
    let mut default_transforms = vec![NodeTransform::default(); node_count];
    for node in document.nodes() {
        let idx = node.index();
        let (translation, rotation, scale) = node.transform().decomposed();
        default_transforms[idx] = NodeTransform {
            translation,
            rotation,
            scale,
        };
        for child in node.children() {
            node_parents[child.index()] = Some(idx);
        }
    }

    // 2. Process meshes
    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            let pos_iter = reader
                .read_positions()
                .ok_or_else(|| anyhow::anyhow!("No positions in primitive"))?;
            let mut uv_iter = reader.read_tex_coords(0).map(|v| v.into_f32());
            let mut norm_iter = reader.read_normals();
            let mut joint_iter = reader.read_joints(0).map(|v| v.into_u16());
            let mut weight_iter = reader.read_weights(0).map(|v| v.into_f32());

            let mut primitive_vertices = Vec::new();
            for pos in pos_iter {
                let uv = uv_iter
                    .as_mut()
                    .and_then(|i| i.next())
                    .unwrap_or([0.0, 0.0]);
                let norm = norm_iter
                    .as_mut()
                    .and_then(|i| i.next())
                    .unwrap_or([0.0, 1.0, 0.0]);
                let joints = joint_iter
                    .as_mut()
                    .and_then(|i| i.next())
                    .unwrap_or([0, 0, 0, 0]);
                let weights = weight_iter
                    .as_mut()
                    .and_then(|i| i.next())
                    .unwrap_or([0.0, 0.0, 0.0, 0.0]);

                primitive_vertices.push(Vertex {
                    pos,
                    uv,
                    normal: norm,
                    tangent: [1.0, 0.0, 0.0], // Default tangent
                    joint_indices: [
                        joints[0] as u32,
                        joints[1] as u32,
                        joints[2] as u32,
                        joints[3] as u32,
                    ],
                    joint_weights: weights,
                });
            }

            let mut primitive_indices = Vec::new();
            if let Some(indices_reader) = reader.read_indices() {
                for idx in indices_reader.into_u32() {
                    primitive_indices.push(idx);
                }
            } else {
                // Generate sequential indices if missing
                for i in 0..primitive_vertices.len() {
                    primitive_indices.push(i as u32);
                }
            }

            // Check Material
            let mut material = Material::default();
            let g_mat = primitive.material();
            if let Some(tex) = g_mat.pbr_metallic_roughness().base_color_texture() {
                let img_idx = tex.texture().source().index();
                if let Some(spot_img) = spot_images.get(img_idx) {
                    material.albedo = Some(spot_img.id());
                }
            }

            // Create Mesh in Context for persistence
            let mesh_id = ctx.register_mesh(&primitive_vertices, &primitive_indices);

            model_parts.push(ModelPart {
                id: mesh_id,
                material,
            });
        }
    }

    let model = Model {
        parts: model_parts.into(),
    };

    let skin = if let Some(gltf_skin) = document.skins().next() {
        let joint_nodes: Vec<_> = gltf_skin.joints().map(|node| node.index()).collect();
        let reader = gltf_skin.reader(|buffer| Some(&buffers[buffer.index()]));
        let inverse_bind_matrices: Vec<[[f32; 4]; 4]> = reader
            .read_inverse_bind_matrices()
            .map(|matrices| matrices.collect())
            .unwrap_or_else(|| vec![crate::math::mat4::identity(); joint_nodes.len()]);

        let bones: Vec<_> = joint_nodes
            .iter()
            .enumerate()
            .map(|(joint_idx, joint_node)| Bone {
                parent_index: node_parents[*joint_node].and_then(|parent_idx| {
                    joint_nodes
                        .iter()
                        .position(|node_idx| *node_idx == parent_idx)
                }),
                inverse_bind_matrix: inverse_bind_matrices
                    .get(joint_idx)
                    .copied()
                    .unwrap_or_else(crate::math::mat4::identity),
            })
            .collect();
        let initial_matrices = vec![crate::math::mat4::identity(); bones.len()];
        let skin_id = ctx.create_skin(bones, initial_matrices);

        Some(SkinBinding {
            skin_id,
            node_parents,
            default_transforms,
            joint_nodes,
            inverse_bind_matrices,
        })
    } else {
        None
    };

    let clips = document
        .animations()
        .map(|animation| {
            let mut translation_tracks = HashMap::new();
            let mut rotation_tracks = HashMap::new();
            let mut scale_tracks = HashMap::new();
            let mut duration_sec = 0.0_f32;

            for channel in animation.channels() {
                let node_idx = channel.target().node().index();
                let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));
                let keyframes: Vec<f32> = reader
                    .read_inputs()
                    .map(|inputs| inputs.collect())
                    .unwrap_or_default();
                if let Some(last) = keyframes.last().copied() {
                    duration_sec = duration_sec.max(last);
                }
                let interpolation = channel.sampler().interpolation();

                let Some(outputs) = reader.read_outputs() else {
                    continue;
                };

                match outputs {
                    ReadOutputs::Translations(values) => {
                        translation_tracks.insert(
                            node_idx,
                            Vec3Track {
                                keyframes,
                                values: normalize_vec3_outputs(values.collect(), interpolation),
                                interpolation,
                            },
                        );
                    }
                    ReadOutputs::Rotations(values) => {
                        rotation_tracks.insert(
                            node_idx,
                            QuatTrack {
                                keyframes,
                                values: normalize_quat_outputs(
                                    values.into_f32().collect(),
                                    interpolation,
                                ),
                                interpolation,
                            },
                        );
                    }
                    ReadOutputs::Scales(values) => {
                        scale_tracks.insert(
                            node_idx,
                            Vec3Track {
                                keyframes,
                                values: normalize_vec3_outputs(values.collect(), interpolation),
                                interpolation,
                            },
                        );
                    }
                    _ => {}
                }
            }

            AnimationClip {
                name: animation.name().map(ToOwned::to_owned),
                duration_sec,
                channels: AnimationChannelSet {
                    translation_tracks,
                    rotation_tracks,
                    scale_tracks,
                },
            }
        })
        .collect();

    Ok(ImportedGltf { model, skin, clips })
}

fn normalize_vec3_outputs(values: Vec<[f32; 3]>, interpolation: Interpolation) -> Vec<[f32; 3]> {
    if interpolation == Interpolation::CubicSpline {
        values
            .chunks(3)
            .filter_map(|chunk| chunk.get(1).copied())
            .collect()
    } else {
        values
    }
}

fn normalize_quat_outputs(values: Vec<[f32; 4]>, interpolation: Interpolation) -> Vec<[f32; 4]> {
    if interpolation == Interpolation::CubicSpline {
        values
            .chunks(3)
            .filter_map(|chunk| chunk.get(1).copied())
            .collect()
    } else {
        values
    }
}

fn sample_time_for_clip(time_sec: f32, duration_sec: f32, looping: bool) -> f32 {
    if duration_sec <= 0.0 {
        0.0
    } else if looping {
        time_sec.rem_euclid(duration_sec)
    } else {
        time_sec.clamp(0.0, duration_sec)
    }
}

fn compose_transform(transform: NodeTransform) -> [[f32; 4]; 4] {
    let translation = crate::math::mat4::from_translation(transform.translation);
    let rotation = crate::math::mat4::from_quat(transform.rotation);
    let scale = crate::math::mat4::from_scale(transform.scale);
    crate::math::mat4::multiply(translation, crate::math::mat4::multiply(rotation, scale))
}

fn compute_world_matrix(
    node_idx: usize,
    node_parents: &[Option<usize>],
    local_matrices: &[[[f32; 4]; 4]],
    world_cache: &mut [Option<[[f32; 4]; 4]>],
) -> [[f32; 4]; 4] {
    if let Some(cached) = world_cache[node_idx] {
        return cached;
    }

    let world = if let Some(parent_idx) = node_parents[node_idx] {
        crate::math::mat4::multiply(
            compute_world_matrix(parent_idx, node_parents, local_matrices, world_cache),
            local_matrices[node_idx],
        )
    } else {
        local_matrices[node_idx]
    };

    world_cache[node_idx] = Some(world);
    world
}

fn sample_vec3_track(track: &Vec3Track, time_sec: f32) -> [f32; 3] {
    if track.keyframes.is_empty() || track.values.is_empty() {
        return [0.0, 0.0, 0.0];
    }

    let idx = track_segment_index(&track.keyframes, time_sec);
    if idx + 1 >= track.keyframes.len() || idx + 1 >= track.values.len() {
        return track.values[idx.min(track.values.len() - 1)];
    }
    if track.interpolation == Interpolation::Step {
        return track.values[idx];
    }

    let start = track.keyframes[idx];
    let end = track.keyframes[idx + 1];
    let alpha = normalize_alpha(start, end, time_sec);
    lerp_vec3(track.values[idx], track.values[idx + 1], alpha)
}

fn sample_quat_track(track: &QuatTrack, time_sec: f32) -> [f32; 4] {
    if track.keyframes.is_empty() || track.values.is_empty() {
        return [0.0, 0.0, 0.0, 1.0];
    }

    let idx = track_segment_index(&track.keyframes, time_sec);
    if idx + 1 >= track.keyframes.len() || idx + 1 >= track.values.len() {
        return normalize_quat(track.values[idx.min(track.values.len() - 1)]);
    }
    if track.interpolation == Interpolation::Step {
        return normalize_quat(track.values[idx]);
    }

    let start = track.keyframes[idx];
    let end = track.keyframes[idx + 1];
    let alpha = normalize_alpha(start, end, time_sec);
    nlerp_quat(track.values[idx], track.values[idx + 1], alpha)
}

fn track_segment_index(keyframes: &[f32], time_sec: f32) -> usize {
    if keyframes.len() <= 1 {
        return 0;
    }

    for idx in 0..(keyframes.len() - 1) {
        if time_sec < keyframes[idx + 1] {
            return idx;
        }
    }

    keyframes.len() - 1
}

fn normalize_alpha(start: f32, end: f32, time_sec: f32) -> f32 {
    let span = end - start;
    if span.abs() <= f32::EPSILON {
        0.0
    } else {
        ((time_sec - start) / span).clamp(0.0, 1.0)
    }
}

fn lerp_vec3(a: [f32; 3], b: [f32; 3], alpha: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * alpha,
        a[1] + (b[1] - a[1]) * alpha,
        a[2] + (b[2] - a[2]) * alpha,
    ]
}

fn nlerp_quat(a: [f32; 4], b: [f32; 4], alpha: f32) -> [f32; 4] {
    let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    let target = if dot < 0.0 {
        [-b[0], -b[1], -b[2], -b[3]]
    } else {
        b
    };

    normalize_quat([
        a[0] + (target[0] - a[0]) * alpha,
        a[1] + (target[1] - a[1]) * alpha,
        a[2] + (target[2] - a[2]) * alpha,
        a[3] + (target[3] - a[3]) * alpha,
    ])
}

fn normalize_quat(q: [f32; 4]) -> [f32; 4] {
    let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if len <= f32::EPSILON {
        [0.0, 0.0, 0.0, 1.0]
    } else {
        [q[0] / len, q[1] / len, q[2] / len, q[3] / len]
    }
}
