use spottedcat::{Spot, Context, DrawOption, Image, Pt, WindowConfig};
use std::time::Duration;

struct ClippingScene {
    parent_image: Image,
    child_image: Image,
}

impl Spot for ClippingScene {
    fn initialize(_context: &mut Context) -> Self {
        // Create a parent image (blue)
        let parent_rgba = vec![0, 0, 255, 255].repeat(200 * 200);
        let parent_image = Image::new_from_rgba8(Pt::from(200.0), Pt::from(200.0), &parent_rgba).unwrap();
        
        // Create a child image (red)
        let child_rgba = vec![255, 0, 0, 255].repeat(100 * 100);
        let child_image = Image::new_from_rgba8(Pt::from(100.0), Pt::from(100.0), &child_rgba).unwrap();

        Self {
            parent_image,
            child_image,
        }
    }

    fn draw(&mut self, context: &mut Context) {
        // 1. Draw parent image at (100, 100)
        let parent_opts = DrawOption::default()
            .with_position([Pt::from(100.0), Pt::from(100.0)]);
        self.parent_image.draw(context, parent_opts);

        // 2. Draw child image partially outside parent, with clipping
        // Parent bounds in screen space: [100, 100, 200, 200]
        let child_opts = DrawOption::default()
            .with_position([Pt::from(150.0), Pt::from(150.0)]); // Relative to parent
        self.parent_image
            .draw_image(context, parent_opts, self.child_image, child_opts);
    }

    fn update(&mut self, _context: &mut Context, _dt: Duration) {}
    fn remove(&self) {}
}

fn main() {
    let mut config = WindowConfig::default();
    config.title = "Clipping Example".to_string();
    spottedcat::run::<ClippingScene>(config);
}
