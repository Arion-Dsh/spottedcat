use spottedcat::{Context, DrawOption3D, Model, Spot, WindowConfig};
use std::time::Duration;

struct InstancingTest {
    cube: Model,
    transforms: Vec<[[f32; 4]; 4]>,
    time: f32,
}

impl Spot for InstancingTest {
    fn initialize(ctx: &mut Context) -> Self {
        let cube = Model::cube(ctx, 0.5).unwrap();

        let mut transforms = Vec::with_capacity(10000);
        for x in -50..50 {
            for y in -50..50 {
                // Initial baseline transforms
                let px = x as f32 * 1.5;
                let py = y as f32 * 1.5;
                let pz = (x as f32 * 0.1).sin() * 2.0 + (y as f32 * 0.1).cos() * 2.0;

                transforms.push([
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [px, py, pz, 1.0],
                ]);
            }
        }

        Self {
            cube,
            transforms,
            time: 0.0,
        }
    }

    fn update(&mut self, _ctx: &mut Context, dt: Duration) {
        let dt_secs = dt.as_secs_f32();
        self.time += dt_secs;

        // Dynamically update 10,000 matrices on CPU as an extreme test
        let mut idx = 0;
        for x in -50..50 {
            for y in -50..50 {
                let px = x as f32 * 1.5;
                let py = y as f32 * 1.5;
                let dist = (px * px + py * py).sqrt();
                let pz = (dist - self.time * 5.0).sin() * 2.0;

                // Rotation
                let rot = self.time + (x as f32 * y as f32 * 0.01);
                let cx = rot.cos();
                let sx = rot.sin();

                self.transforms[idx] = [
                    [cx, 0.0, -sx, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [sx, 0.0, cx, 0.0],
                    [px, py, pz, 1.0],
                ];
                idx += 1;
            }
        }
    }

    fn draw(&mut self, ctx: &mut Context) {
        // Draw 10000 cubes in 1 call!
        self.cube.draw_instanced(
            ctx,
            DrawOption3D::default()
                .with_position([0.0, -10.0, -80.0])
                .with_rotation([1.0, self.time * 0.1, 0.0]),
            &self.transforms,
        );
    }

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    unsafe {
        std::env::set_var("SPOT_PROFILE_RENDER", "1");
    } // Print render stats
    spottedcat::run::<InstancingTest>(WindowConfig {
        title: "Instancing Test (10000 Cubes)".to_string(),
        width: spottedcat::Pt::from(1280.0),
        height: spottedcat::Pt::from(720.0),
        ..Default::default()
    });
}
