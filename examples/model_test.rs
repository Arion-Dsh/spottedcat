use spottedcat::{Context, DrawOption3D, Image, Model, Pt, Spot, WindowConfig};

struct ModelTest {
    cube: Model,
    axis_x: Model,
    axis_y: Model,
    axis_z: Model,
    rotation: f32,
}

impl Spot for ModelTest {
    fn initialize(ctx: &mut Context) -> Self {
        ctx.set_camera_pos([2.6, 2.2, 4.6]);
        ctx.set_camera_target(0.0, 0.0, 0.0);
        ctx.set_camera_up(0.0, 1.0, 0.0);

        // Create a simple 2x2 texture
        let rgba = vec![
            255, 0, 0, 255, // Red
            0, 255, 0, 255, // Green
            0, 0, 255, 255, // Blue
            255, 255, 0, 255, // Yellow
        ];
        let texture = spottedcat::create_image(ctx, Pt::from(2.0), Pt::from(2.0), &rgba).unwrap();
        let axis_x_tex =
            spottedcat::create_image(ctx, Pt::from(1.0), Pt::from(1.0), &[255, 64, 64, 255]).unwrap();
        let axis_y_tex =
            spottedcat::create_image(ctx, Pt::from(1.0), Pt::from(1.0), &[64, 255, 64, 255]).unwrap();
        let axis_z_tex =
            spottedcat::create_image(ctx, Pt::from(1.0), Pt::from(1.0), &[64, 128, 255, 255]).unwrap();

        // Create a 3D cube model and apply the texture
        let cube = spottedcat::model::create_cube(ctx, 1.0).unwrap().with_material(texture);
        let axis_x = spottedcat::model::create_cube(ctx, 1.0)
            .unwrap()
            .with_material(axis_x_tex);
        let axis_y = spottedcat::model::create_cube(ctx, 1.0)
            .unwrap()
            .with_material(axis_y_tex);
        let axis_z = spottedcat::model::create_cube(ctx, 1.0)
            .unwrap()
            .with_material(axis_z_tex);

        Self {
            cube,
            axis_x,
            axis_y,
            axis_z,
            rotation: 0.0,
        }
    }

    fn update(&mut self, _ctx: &mut Context, dt: std::time::Duration) {
        self.rotation += dt.as_secs_f32();
    }

    fn draw(&mut self, ctx: &mut Context) {
        // Draw the main cube at the origin.
        let cube_opts = DrawOption3D::default()
            .with_position([0.0, 0.0, 0.0])
            .with_rotation([self.rotation, self.rotation * 0.5, 0.0]);
        spottedcat::model::draw(ctx, &self.cube, cube_opts);

        // Draw colored axes so front-face culling and orientation are easy to inspect.
        spottedcat::model::draw(
            ctx,
            &self.axis_x,
            DrawOption3D::default()
                .with_position([1.3, 0.0, 0.0])
                .with_scale([2.2, 0.06, 0.06]),
        );
        spottedcat::model::draw(
            ctx,
            &self.axis_y,
            DrawOption3D::default()
                .with_position([0.0, 1.3, 0.0])
                .with_scale([0.06, 2.2, 0.06]),
        );
        spottedcat::model::draw(
            ctx,
            &self.axis_z,
            DrawOption3D::default()
                .with_position([0.0, 0.0, 1.3])
                .with_scale([0.06, 0.06, 2.2]),
        );
    }

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    spottedcat::run::<ModelTest>(WindowConfig {
        title: "3D Model Test".to_string(),
        ..Default::default()
    });
}
