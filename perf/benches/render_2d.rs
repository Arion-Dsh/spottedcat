#[path = "../../examples/example_font.rs"]
mod example_font;

use spottedcat::{Context, DrawOption, Image, Pt, Spot, Text, Texture, WindowConfig, run};
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    SpriteBatch,
    SpriteStateChanges,
    TextCached,
    TextDynamic,
    Offscreen,
}

impl Scenario {
    fn from_env() -> Self {
        match std::env::var("SPOT_PERF_SCENARIO").as_deref() {
            Ok("sprite_batch") => Self::SpriteBatch,
            Ok("sprite_state_changes") => Self::SpriteStateChanges,
            Ok("text_cached") => Self::TextCached,
            Ok("text_dynamic") => Self::TextDynamic,
            Ok("offscreen") => Self::Offscreen,
            Ok(other) => panic!(
                "unknown SPOT_PERF_SCENARIO={other}; expected sprite_batch, sprite_state_changes, text_cached, text_dynamic, or offscreen"
            ),
            Err(_) => Self::SpriteBatch,
        }
    }

    fn default_objects(self) -> usize {
        match self {
            Self::SpriteBatch | Self::SpriteStateChanges => 20_000,
            Self::TextCached | Self::TextDynamic => 800,
            Self::Offscreen => 8_000,
        }
    }
}

struct Performance2D {
    scenario: Scenario,
    object_count: usize,
    images: Vec<Image>,
    texts: Vec<Text>,
    targets: Vec<Image>,
    tick: u64,
}

fn object_count(scenario: Scenario) -> usize {
    std::env::var("SPOT_PERF_OBJECTS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| scenario.default_objects())
        .max(1)
}

fn make_images(ctx: &mut Context, count: usize) -> Vec<Image> {
    (0..count)
        .map(|index| {
            let r = ((index * 73 + 41) % 255) as u8;
            let g = ((index * 151 + 83) % 255) as u8;
            let b = ((index * 199 + 127) % 255) as u8;
            let rgba = [r, g, b, 255, b, r, g, 255, g, b, r, 255, r, b, g, 255];
            Image::new(ctx, Pt::from(2.0), Pt::from(2.0), &rgba)
                .expect("performance texture should be created")
        })
        .collect()
}

impl Spot for Performance2D {
    fn initialize(ctx: &mut Context) -> Self {
        let scenario = Scenario::from_env();
        let object_count = object_count(scenario);
        let image_count = if matches!(scenario, Scenario::SpriteStateChanges) {
            64
        } else {
            1
        };
        let images = make_images(ctx, image_count);

        let font_id = example_font::register(ctx);
        let texts = if matches!(scenario, Scenario::TextCached | Scenario::TextDynamic) {
            (0..object_count)
                .map(|index| {
                    Text::new(format!("性能 Performance {index:04}"), font_id)
                        .with_font_size(Pt::from(14.0 + (index % 4) as f32))
                })
                .collect()
        } else {
            Vec::new()
        };

        let targets = if matches!(scenario, Scenario::Offscreen) {
            (0..8)
                .map(|_| Texture::new_render_target(ctx, Pt::from(512.0), Pt::from(512.0)).view())
                .collect()
        } else {
            Vec::new()
        };

        eprintln!(
            "[spot][perf-scene] scenario={scenario:?} objects={object_count} textures={} targets={}",
            images.len(),
            targets.len()
        );
        Self {
            scenario,
            object_count,
            images,
            texts,
            targets,
            tick: 0,
        }
    }

    fn update(&mut self, _ctx: &mut Context, _dt: Duration) {
        self.tick = self.tick.wrapping_add(1);
        if matches!(self.scenario, Scenario::TextDynamic) {
            for (index, text) in self.texts.iter_mut().enumerate() {
                text.set_content(format!(
                    "动态 Dynamic {index:04} {:04}",
                    self.tick.wrapping_add(index as u64) % 10_000
                ));
            }
        }
    }

    fn draw(&mut self, ctx: &mut Context, screen: Image) {
        match self.scenario {
            Scenario::SpriteBatch | Scenario::SpriteStateChanges => {
                for index in 0..self.object_count {
                    let x = (index % 200) as f32 * 6.4;
                    let y = ((index / 200) % 120) as f32 * 6.0;
                    let image = &self.images[index % self.images.len()];
                    screen.draw(
                        ctx,
                        image,
                        DrawOption::default()
                            .with_position([Pt::from(x), Pt::from(y)])
                            .with_scale([3.0, 3.0]),
                    );
                }
            }
            Scenario::TextCached | Scenario::TextDynamic => {
                for (index, text) in self.texts.iter().enumerate() {
                    let x = (index % 16) as f32 * 80.0;
                    let y = ((index / 16) % 45) as f32 * 16.0;
                    screen.draw(
                        ctx,
                        text,
                        DrawOption::default().with_position([Pt::from(x), Pt::from(y)]),
                    );
                }
            }
            Scenario::Offscreen => {
                let per_target = self.object_count.div_ceil(self.targets.len());
                for (target_index, target) in self.targets.iter().copied().enumerate() {
                    for local_index in 0..per_target {
                        let x = (local_index % 64) as f32 * 8.0;
                        let y = ((local_index / 64) % 64) as f32 * 8.0;
                        target.draw(
                            ctx,
                            &self.images[0],
                            DrawOption::default()
                                .with_position([Pt::from(x), Pt::from(y)])
                                .with_scale([4.0, 4.0])
                                .with_rotation(
                                    self.tick as f32 * 0.001 + target_index as f32 * 0.1,
                                ),
                        );
                    }
                    screen.draw(
                        ctx,
                        &target,
                        DrawOption::default()
                            .with_position([
                                Pt::from((target_index % 4) as f32 * 320.0),
                                Pt::from((target_index / 4) as f32 * 360.0),
                            ])
                            .with_scale([0.625, 0.625]),
                    );
                }
            }
        }
    }
}

fn main() {
    // The environment can override this before OnceLock reads it.
    unsafe {
        std::env::set_var("SPOT_PROFILE_RENDER", "1");
    }
    let update_hz = std::env::var("SPOT_PERF_UPDATE_HZ")
        .map(|value| {
            value
                .parse::<u32>()
                .expect("SPOT_PERF_UPDATE_HZ must be an integer")
        })
        .unwrap_or(60);
    run::<Performance2D>(WindowConfig {
        title: "Spottedcat 2D Performance Suite".to_string(),
        width: Pt::from(1280.0),
        height: Pt::from(720.0),
        update_hz,
        ..Default::default()
    });
}
