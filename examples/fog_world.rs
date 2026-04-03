use spottedcat::{
    Context, DrawOption, DrawOption3D, FogBackgroundSettings, FogSamplingSettings, FogSettings,
    Image, Model, Pt, Spot, Text, WindowConfig,
};
use std::time::Duration;

struct FogWorld {
    cube: Model,
    sphere: Model,
    floor: Model,
    giant_bg: Model,
    time: f32,

    fps: f32,
    frame_count: u32,
    accumulated_time: f32,
    fps_text: Text,
}

impl Spot for FogWorld {
    fn initialize(ctx: &mut Context) -> Self {
        ctx.set_camera_pos([0.0, 3.6, 14.0]);
        ctx.set_camera_target(0.0, 1.2, -12.0);
        ctx.set_camera_up(0.0, 1.0, 0.0);
        ctx.set_ambient_light([0.16, 0.18, 0.17, 1.0]);

        let floor_tex = spottedcat::create_image(
            ctx,
            Pt::from(2.0),
            Pt::from(2.0),
            &[
                58, 69, 66, 255, 48, 58, 55, 255, 48, 58, 55, 255, 58, 69, 66, 255,
            ],
        )
        .unwrap();
        let cube_tex =
            spottedcat::create_image(ctx, Pt::from(1.0), Pt::from(1.0), &[152, 162, 156, 255])
                .unwrap();
        let sphere_tex =
            spottedcat::create_image(ctx, Pt::from(1.0), Pt::from(1.0), &[225, 232, 228, 255])
                .unwrap();
        let bg_tex =
            spottedcat::create_image(ctx, Pt::from(1.0), Pt::from(1.0), &[104, 113, 116, 255])
                .unwrap();

        let floor = spottedcat::model::create_plane(ctx, 1.0, 1.0)
            .unwrap()
            .with_material(floor_tex);
        let cube = spottedcat::model::create_cube(ctx, 1.0).unwrap().with_material(cube_tex);
        let sphere = spottedcat::model::create_sphere(ctx, 1.0).unwrap().with_material(sphere_tex);
        let giant_bg = spottedcat::model::create_sphere(ctx, 1.0).unwrap().with_material(bg_tex);

        const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
        let font_id = spottedcat::register_font(ctx, FONT.to_vec());
        let fps_text = Text::new("FPS: 0", font_id)
            .with_font_size(Pt::from(24.0))
            .with_color([0.0, 1.0, 0.0, 1.0]);

        Self {
            cube,
            sphere,
            floor,
            giant_bg,
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
    }

    fn draw(&mut self, ctx: &mut Context) {
        let fog = FogSettings::default()
            .with_color([0.72, 0.79, 0.77, 1.0])
            .with_strength(0.26)
            .with_background(
                FogBackgroundSettings::morning_mist()
                    .with_horizon_glow(0.035)
                    .with_blend(0.68, 0.52),
            )
            .with_sampling(FogSamplingSettings::default().with_height_samples(4, 8))
            .with_distance(14.0, 48.0, 0.22, 1.0)
            .with_height(-1.0, 9.0, 0.16, 1.0);
        ctx.set_fog(fog);

        spottedcat::model::draw(
            ctx,
            &self.floor,
            DrawOption3D::default()
                .with_position([0.0, -1.0, -8.0])
                .with_rotation([-std::f32::consts::FRAC_PI_2, 0.0, 0.0])
                .with_scale([50.0, 50.0, 1.0]),
        );

        spottedcat::model::draw(
            ctx,
            &self.giant_bg,
            DrawOption3D::default()
                .with_position([0.0, 6.5 + (self.time * 0.18).sin() * 0.4, -70.0])
                .with_scale([70.0, 70.0, 70.0])
                .with_opacity(0.16),
        );

        for row in 0..6 {
            let z = -4.0 - row as f32 * 4.4;
            let sway = (self.time * 0.8 + row as f32 * 0.7).sin() * 0.35;
            let lift = (self.time * 1.1 + row as f32 * 0.5).sin() * 0.12;

            spottedcat::model::draw(
                ctx,
                &self.cube,
                DrawOption3D::default()
                    .with_position([-3.0 + sway, -0.1 + lift, z])
                    .with_scale([1.1, 1.7, 1.1])
                    .with_rotation([0.0, self.time * 0.3 + row as f32 * 0.2, 0.0]),
            );

            spottedcat::model::draw(
                ctx,
                &self.cube,
                DrawOption3D::default()
                    .with_position([3.0 - sway, 0.2 - lift * 0.5, z - 1.0])
                    .with_scale([1.4, 2.2, 1.4])
                    .with_rotation([0.0, -self.time * 0.25 - row as f32 * 0.18, 0.0]),
            );

            spottedcat::model::draw(
                ctx,
                &self.sphere,
                DrawOption3D::default()
                    .with_position([0.0, 1.0 + row as f32 * 0.18, z - 2.0])
                    .with_scale([
                        1.2 + row as f32 * 0.18,
                        1.2 + row as f32 * 0.18,
                        1.2 + row as f32 * 0.18,
                    ]),
            );
        }

        ctx.clear_fog();

        // Draw FPS
        spottedcat::text::draw(
            ctx,
            &self.fps_text,
            DrawOption::default().with_position([Pt::from(10.0), Pt::from(10.0)]),
        );
    }

    fn remove(&mut self, ctx: &mut Context) {
        ctx.clear_fog();
    }
}

fn main() {
    spottedcat::run::<FogWorld>(WindowConfig {
        title: "Fog World Example".to_string(),
        ..Default::default()
    });
}
