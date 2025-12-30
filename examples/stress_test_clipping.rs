use spottedcat::{Spot, Context, DrawOption, Image, Pt, WindowConfig};
use std::time::Duration;

struct StressTestScene {
    container: Image,
    child: Image,
}

impl Spot for StressTestScene {
    fn initialize(_context: &mut Context) -> Self {
        // 100x100 container
        let container_rgba = vec![50, 50, 50, 255].repeat(100 * 100);
        let container = Image::new_from_rgba8(Pt::from(100.0), Pt::from(100.0), &container_rgba).unwrap();
        
        // 5x5 child
        let child_rgba = vec![200, 100, 0, 255].repeat(5 * 5);
        let child = Image::new_from_rgba8(Pt::from(5.0), Pt::from(5.0), &child_rgba).unwrap();

        Self { container, child }
    }

    fn draw(&mut self, context: &mut Context) {
        let mut parent_opts = DrawOption::default();
        parent_opts.set_position([Pt::from(50.0), Pt::from(50.0)]);
        parent_opts.set_scale([7.0, 5.0]); // Make a large 700x500 container
        
        // Draw the parent container
        self.container.draw(context, parent_opts);

        // Perform 100,000 draw_child calls
        // They will all be clipped to the container's bounds
        let total = 100_000;
        let cols = 400;
        let spacing = 2.0;

        for i in 0..total {
            let x = (i % cols) as f32 * spacing;
            let y = (i / cols) as f32 * spacing;
            
            let mut child_opts = DrawOption::default();
            child_opts.set_position([Pt::from(x), Pt::from(y)]);
            
            // All children share the same parent and same clip area
            // This tests both logic overhead and batching efficiency
            self.container.draw_image(
                context,
                parent_opts,
                self.child,
                child_opts,
            );
        }
    }

    fn update(&mut self, _context: &mut Context, _dt: Duration) {}
    fn remove(&self) {}
}

fn main() {
    let mut config = WindowConfig::default();
    config.title = "100K Nested Clipping Stress Test".to_string();
    config.width = Pt::from(1000.0);
    config.height = Pt::from(800.0);
    spottedcat::run::<StressTestScene>(config);
}
