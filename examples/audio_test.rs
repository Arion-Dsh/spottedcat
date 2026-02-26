use spottedcat::{Context, Spot, WindowConfig};
use std::time::Duration;

struct AudioTest {
    timer: Duration,
}

impl Spot for AudioTest {
    fn initialize(_context: &mut Context) -> Self {
        println!("Audio Test Initialized. Press any key to play a sine wave.");
        Self {
            timer: Duration::ZERO,
        }
    }

    fn update(&mut self, context: &mut Context, dt: Duration) {
        self.timer += dt;

        // Play a sine wave every 2 seconds automatically to verify it's working
        if self.timer.as_secs_f32() > 2.0 {
            println!("Auto-playing sine wave...");
            spottedcat::play_sine(440.0, 0.2);
            self.timer = Duration::ZERO;
        }

        // Manual trigger via input (if input manager is working)
        if spottedcat::key_pressed(context, spottedcat::Key::Space) {
            println!("Space pressed! Playing higher sine wave...");
            spottedcat::play_sine(880.0, 0.2);
        }
    }

    fn draw(&mut self, _context: &mut Context) {
        // Just keep the window alive
    }

    fn remove(&self) {
        println!("Audio Test Finished.");
    }
}

fn main() {
    spottedcat::run::<AudioTest>(WindowConfig {
        title: "Audio Test".to_string(),
        ..Default::default()
    });
}
