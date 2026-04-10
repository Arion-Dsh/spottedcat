use spottedcat::{Context, Spot, TouchPhase};
use std::time::Duration;

struct TouchTest {}

impl Spot for TouchTest {
    fn initialize(_ctx: &mut Context) -> Self {
        Self {}
    }

    fn draw(&mut self, ctx: &mut Context, _screen: spottedcat::Image) {
        let touches = spottedcat::touches(ctx);
        for touch in touches {
            let _color = match touch.phase {
                TouchPhase::Started => [1.0, 0.0, 0.0, 1.0],
                TouchPhase::Moved => [0.0, 1.0, 0.0, 1.0],
                TouchPhase::Ended => [0.0, 0.0, 1.0, 1.0],
                TouchPhase::Cancelled => [0.5, 0.5, 0.5, 1.0],
            };

            // Draw something at touch position
            // Since we don't have a primitive draw circle, we can just print for now or use a small image
            println!(
                "Touch ID: {}, Pos: {:?}, Phase: {:?}",
                touch.id, touch.position, touch.phase
            );
        }
    }

    fn update(&mut self, _ctx: &mut Context, _dt: Duration) {}
    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    spottedcat::run::<TouchTest>(spottedcat::WindowConfig::default());
}
