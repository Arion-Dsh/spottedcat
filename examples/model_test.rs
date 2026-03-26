use spottedcat::{Context, Spot, Model, Image, DrawOption3D, WindowConfig};
use std::time::Duration;

struct ModelTest {
    cube: Model,
    rotation: f32,
}

impl Spot for ModelTest {
    fn initialize(_context: &mut Context) -> Self {
        // Create a simple 2x2 texture
        let rgba = vec![
            255, 0, 0, 255,   // Red
            0, 255, 0, 255,   // Green
            0, 0, 255, 255,   // Blue
            255, 255, 0, 255, // Yellow
        ];
        let texture = Image::new_from_rgba8(2.into(), 2.into(), &rgba).unwrap();

        // Create a 3D cube model and apply the texture
        let cube = Model::cube(1.0).unwrap()
            .with_material(texture);

        Self {
            cube,
            rotation: 0.0,
        }
    }

    fn update(&mut self, _context: &mut Context, dt: Duration) {
        // Update rotation over time
        self.rotation += dt.as_secs_f32();
    }

    fn draw(&mut self, context: &mut Context) {
        // Draw the cube in the center with some rotation
        let opts = DrawOption3D::default()
            .with_position([0.0, 0.0, 0.0])
            .with_rotation([self.rotation, self.rotation * 0.5, 0.0]);
        
        self.cube.draw(context, opts);
    }

    fn remove(&self) {}
}

fn main() {
    spottedcat::run::<ModelTest>(WindowConfig {
        title: "3D Model Test".to_string(),
        ..Default::default()
    });
}
