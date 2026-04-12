use std::sync::OnceLock;
use std::time::Duration;

use spottedcat::utils::gltf::AnimatedModel;
use spottedcat::{Context, DrawOption3D, Key, Spot, WindowConfig};

static MODEL_PATH: OnceLock<String> = OnceLock::new();

struct AnimatedGltfApp {
    actor: AnimatedModel,
    yaw: f32,
}

impl AnimatedGltfApp {
    fn select_clip_by_offset(&mut self, ctx: &mut Context, offset: isize) {
        let clip_count = self.actor.clip_count();
        if clip_count == 0 {
            return;
        }

        let current = self.actor.current_clip_index() as isize;
        let next = (current + offset).rem_euclid(clip_count as isize) as usize;
        let _ = self.actor.play_clip(ctx, next);
        eprintln!(
            "[animated_gltf] clip {}: {:?}",
            next,
            self.actor.current_clip_name()
        );
    }

    fn try_select_numbered_clip(&mut self, ctx: &mut Context) {
        let number_keys = [
            Key::Num1,
            Key::Num2,
            Key::Num3,
            Key::Num4,
            Key::Num5,
            Key::Num6,
            Key::Num7,
            Key::Num8,
            Key::Num9,
        ];

        for (idx, key) in number_keys.iter().enumerate() {
            if spottedcat::key_pressed(ctx, *key) && self.actor.play_clip(ctx, idx) {
                eprintln!(
                    "[animated_gltf] clip {}: {:?}",
                    idx,
                    self.actor.current_clip_name()
                );
            }
        }
    }
}

impl Spot for AnimatedGltfApp {
    fn initialize(ctx: &mut Context) -> Self {
        spottedcat::set_ambient_light(ctx, [0.25, 0.25, 0.28, 1.0]);
        spottedcat::set_light(ctx, 0, [8.0, 10.0, 8.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        spottedcat::set_camera(ctx, [0.0, 1.6, 5.5], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]);

        let path = MODEL_PATH
            .get()
            .expect("MODEL_PATH should be set before run()");
        let bytes = std::fs::read(path).expect("failed to read animated glTF");
        let mut actor = spottedcat::utils::gltf::load_animated_gltf_from_bytes(ctx, &bytes)
            .expect("failed to load animated glTF");

        let _ = actor.play_first_matching_clip(ctx, &["idle", "walk", "run"]);
        actor.apply_current_pose(ctx);

        eprintln!("[animated_gltf] loaded: {path}");
        for idx in 0..actor.clip_count() {
            eprintln!("[animated_gltf] clip {idx}: {:?}", actor.clip_name(idx));
        }
        eprintln!(
            "[animated_gltf] controls: [ and ] switch clips, 1-9 jump clips, Space pause/play, Left/Right rotate"
        );

        Self { actor, yaw: 0.0 }
    }

    fn update(&mut self, ctx: &mut Context, dt: Duration) {
        let dt_sec = dt.as_secs_f32();

        if spottedcat::key_down(ctx, Key::Left) {
            self.yaw += dt_sec * 1.25;
        }
        if spottedcat::key_down(ctx, Key::Right) {
            self.yaw -= dt_sec * 1.25;
        }

        if spottedcat::key_pressed(ctx, Key::Space) {
            if self.actor.is_playing() {
                self.actor.pause();
            } else {
                self.actor.play();
            }
        }
        if spottedcat::key_pressed(ctx, Key::BracketLeft) {
            self.select_clip_by_offset(ctx, -1);
        }
        if spottedcat::key_pressed(ctx, Key::BracketRight) {
            self.select_clip_by_offset(ctx, 1);
        }
        self.try_select_numbered_clip(ctx);

        self.actor.update(ctx, dt_sec);
    }

    fn draw(&mut self, ctx: &mut Context, screen: spottedcat::Image) {
        screen.draw(
            ctx,
            &self.actor,
            DrawOption3D::default()
                .with_position([0.0, 0.0, 0.0])
                .with_rotation([0.0, self.yaw, 0.0])
                .with_scale([1.0, 1.0, 1.0]),
        );
    }

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!(
            "usage: cargo run --example animated_gltf --features gltf -- <path-to-model.glb>"
        );
        std::process::exit(2);
    });
    let _ = MODEL_PATH.set(path);

    spottedcat::run::<AnimatedGltfApp>(WindowConfig {
        title: "SpottedCat Animated glTF".to_string(),
        ..Default::default()
    });
}
