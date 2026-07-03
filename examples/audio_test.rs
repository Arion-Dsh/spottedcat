use spottedcat::{Context, Spot, WindowConfig};
use std::time::Duration;

struct AudioTest {
    timer: Duration,
}

impl Spot for AudioTest {
    fn initialize(_ctx: &mut Context) -> Self {
        println!("Audio Test Initialized. Press any key to play a sine wave.");
        Self {
            timer: Duration::ZERO,
        }
    }

    fn update(&mut self, ctx: &mut Context, dt: Duration) {
        self.timer += dt;

        if self.timer.as_secs_f32() > 2.0 {
            println!("Auto-playing sine wave...");
            spottedcat::play_sine(ctx, 440.0, 0.2);
            self.timer = Duration::ZERO;
        }

        if spottedcat::key_pressed(ctx, spottedcat::Key::Space) {
            println!("Space pressed! Playing higher sine wave...");
            spottedcat::play_sine(ctx, 880.0, 0.2);
        }
    }

    fn draw(&mut self, _ctx: &mut Context, _screen: spottedcat::Image) {}

    fn remove(&mut self, _ctx: &mut Context) {
        println!("Audio Test Finished.");
    }
}

fn main() {
    spottedcat::run::<AudioTest>(WindowConfig {
        title: "Audio Test".to_string(),
        ..Default::default()
    });
}
