use spottedcat::{Context, DrawOption3D, Image, Model, Pt, Spot, WindowConfig};
use std::sync::Arc;
use std::time::Duration;

struct RenderStateStress {
    models: Vec<Model>,
    transparent_plane: Model,
    instanced_cube: Model,
    instanced_transforms: Arc<[[[f32; 4]; 4]]>,
    time: f32,
}

fn checker_texture(ctx: &mut Context, a: [u8; 4], b: [u8; 4]) -> Image {
    let rgba = vec![
        a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3], b[0], b[1], b[2], b[3], a[0], a[1], a[2],
        a[3],
    ];
    spottedcat::image::create(ctx, Pt::from(2.0), Pt::from(2.0), &rgba)
        .expect("checker texture should be created")
}

impl Spot for RenderStateStress {
    fn initialize(ctx: &mut Context) -> Self {
        let red = checker_texture(ctx, [255, 96, 96, 255], [160, 32, 32, 255]);
        let green = checker_texture(ctx, [96, 255, 160, 255], [24, 120, 64, 255]);
        let blue = checker_texture(ctx, [96, 168, 255, 255], [24, 56, 144, 255]);
        let gold = checker_texture(ctx, [255, 220, 96, 255], [160, 120, 24, 255]);
        let glass = checker_texture(ctx, [180, 220, 255, 180], [80, 120, 180, 120]);

        let cube_red = spottedcat::model::create_cube(ctx, 0.9)
            .unwrap()
            .with_material(red);
        let cube_green = spottedcat::model::create_cube(ctx, 0.9)
            .unwrap()
            .with_material(green);
        let sphere_blue = spottedcat::model::create_sphere(ctx, 0.55)
            .unwrap()
            .with_material(blue);
        let sphere_gold = spottedcat::model::create_sphere(ctx, 0.55)
            .unwrap()
            .with_material(gold);
        let transparent_plane = spottedcat::model::create_plane(ctx, 1.8, 1.8)
            .unwrap()
            .with_material(glass);
        let instanced_cube = spottedcat::model::create_cube(ctx, 0.35)
            .unwrap()
            .with_material(gold);

        let mut instanced_transforms = Vec::with_capacity(80 * 40);
        for z in 0..40 {
            for x in 0..80 {
                let px = (x as f32 - 40.0) * 0.9;
                let py = ((x as f32 * 0.35).sin() + (z as f32 * 0.25).cos()) * 0.8;
                let pz = -20.0 - z as f32 * 1.4;
                instanced_transforms.push([
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [px, py, pz, 1.0],
                ]);
            }
        }

        Self {
            models: vec![cube_red, sphere_blue, cube_green, sphere_gold],
            transparent_plane,
            instanced_cube,
            instanced_transforms: Arc::from(instanced_transforms),
            time: 0.0,
        }
    }

    fn update(&mut self, ctx: &mut Context, dt: Duration) {
        self.time += dt.as_secs_f32();
        spottedcat::set_camera_pos(ctx, [0.0, 14.0, 26.0]);
        spottedcat::set_camera_target(ctx, 0.0, 0.0, -28.0);
    }

    fn draw(&mut self, ctx: &mut Context) {
        let columns = 32;
        let rows = 24;

        for row in 0..rows {
            for col in 0..columns {
                let draw_index = row * columns + col;
                let model_index = (draw_index * 7 + row * 3) % self.models.len();
                let x = (col as f32 - columns as f32 * 0.5) * 1.8;
                let y = (row as f32 - rows as f32 * 0.5) * 1.35;
                let z = -6.0 - ((row + col) % 5) as f32 * 1.6;
                let spin = self.time * 0.8 + draw_index as f32 * 0.03;

                let opts = DrawOption3D::default()
                    .with_position([x, y, z])
                    .with_rotation([spin * 0.4, spin, 0.0]);
                spottedcat::model::draw(ctx, &self.models[model_index], opts);
            }
        }

        for idx in 0..18 {
            let x = -14.0 + idx as f32 * 1.7;
            let z = -2.0 - idx as f32 * 0.4;
            let opts = DrawOption3D::default()
                .with_position([x, 0.0, z])
                .with_rotation([0.0, self.time * 0.25 + idx as f32 * 0.1, 0.0])
                .with_opacity(0.35);
            spottedcat::model::draw(ctx, &self.transparent_plane, opts);
        }

        spottedcat::model::draw_instanced_shared(
            ctx,
            &self.instanced_cube,
            DrawOption3D::default()
                .with_position([0.0, -10.0, 0.0])
                .with_rotation([0.15, self.time * 0.15, 0.0]),
            self.instanced_transforms.clone(),
        );
    }

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    unsafe {
        std::env::set_var("SPOT_PROFILE_RENDER", "1");
    }

    spottedcat::run::<RenderStateStress>(WindowConfig {
        title: "Render State Stress".to_string(),
        width: Pt::from(1440.0),
        height: Pt::from(900.0),
        ..Default::default()
    });
}
