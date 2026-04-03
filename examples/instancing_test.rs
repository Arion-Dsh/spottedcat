use spottedcat::{Context, DrawOption, DrawOption3D, Model, Pt, Spot, Text, WindowConfig};
use std::time::Duration;

struct InstancingTest {
    cube: Model,
    transforms: Vec<[[f32; 4]; 4]>,
    time: f32,

    fps: f32,
    frame_count: u32,
    accumulated_time: f32,
    fps_text: Text,
}

impl Spot for InstancingTest {
    fn initialize(ctx: &mut Context) -> Self {
        let cube = spottedcat::model::create_cube(ctx, 0.5).unwrap();

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

        const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
        let font_id = spottedcat::register_font(ctx, FONT.to_vec());
        let fps_text = Text::new("FPS: 0", font_id)
            .with_font_size(Pt::from(24.0))
            .with_color([0.0, 1.0, 0.0, 1.0]);

        Self {
            cube,
            transforms,
            time: 0.0,
            fps: 0.0,
            frame_count: 0,
            accumulated_time: 0.0,
            fps_text,
        }
    }

    fn update(&mut self, _ctx: &mut Context, dt: Duration) {
        let dt_secs = dt.as_secs_f32();
        self.time += dt_secs;

        self.accumulated_time += dt_secs;
        self.frame_count += 1;

        if self.accumulated_time >= 1.0 {
            self.fps = self.frame_count as f32 / self.accumulated_time;
            self.fps_text.set_content(format!("FPS: {:.1}", self.fps));
            self.accumulated_time = 0.0;
            self.frame_count = 0;
        }

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
        spottedcat::model::draw_instanced(
            ctx,
            &self.cube,
            DrawOption3D::default()
                .with_position([0.0, -10.0, -80.0])
                .with_rotation([1.0, self.time * 0.1, 0.0]),
            &self.transforms,
        );

        // Draw FPS
        spottedcat::text::draw(
            ctx,
            &self.fps_text,
            DrawOption::default().with_position([Pt::from(10.0), Pt::from(10.0)]),
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
