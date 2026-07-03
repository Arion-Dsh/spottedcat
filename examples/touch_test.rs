use spottedcat::{Context, Spot, TouchPhase};

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

            println!(
                "Touch ID: {}, Pos: {:?}, Phase: {:?}",
                touch.id, touch.position, touch.phase
            );
        }
    }
}

fn main() {
    spottedcat::run::<TouchTest>(spottedcat::WindowConfig::default());
}
