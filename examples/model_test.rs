use spottedcat::{Context, DrawOption3D, Model, Pt, Spot, WindowConfig};

struct ModelTest {
    cube: Model,
    axis_x: Model,
    axis_y: Model,
    axis_z: Model,
    rotation: f32,
}

impl Spot for ModelTest {
    fn initialize(ctx: &mut Context) -> Self {
        spottedcat::set_camera_pos(ctx, [0.0, 2.0, 8.0]);
        spottedcat::set_camera_target(ctx, 0.0, 0.0, 0.0);
        spottedcat::set_camera_up(ctx, 0.0, 1.0, 0.0);
        spottedcat::set_camera_fovy(ctx, 45.0);

        // Create a simple 2x2 texture
        let rgba = vec![
            255, 0, 0, 255, // Red
            0, 255, 0, 255, // Green
            0, 0, 255, 255, // Blue
            255, 255, 0, 255, // Yellow
        ];
        let texture = spottedcat::Image::new(ctx, Pt::from(2.0), Pt::from(2.0), &rgba).unwrap();
        let axis_x_tex =
            spottedcat::Image::new(ctx, Pt::from(1.0), Pt::from(1.0), &[255, 64, 64, 255]).unwrap();
        let axis_y_tex =
            spottedcat::Image::new(ctx, Pt::from(1.0), Pt::from(1.0), &[64, 255, 64, 255]).unwrap();
        let axis_z_tex =
            spottedcat::Image::new(ctx, Pt::from(1.0), Pt::from(1.0), &[64, 128, 255, 255])
                .unwrap();

        // Create a 3D cube model and apply the texture
        let cube = spottedcat::model::create_cube(ctx, 1.0)
            .unwrap()
            .with_material(texture);
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

    fn update(&mut self, ctx: &mut Context, dt: std::time::Duration) {
        let dt_secs = dt.as_secs_f32();
        self.rotation += dt_secs;

        // Camera control demo
        let mut pos = spottedcat::camera_position(ctx);
        if spottedcat::key_down(ctx, spottedcat::Key::W) {
            pos[2] -= 5.0 * dt_secs;
        }
        if spottedcat::key_down(ctx, spottedcat::Key::S) {
            pos[2] += 5.0 * dt_secs;
        }
        if spottedcat::key_down(ctx, spottedcat::Key::A) {
            pos[0] -= 5.0 * dt_secs;
        }
        if spottedcat::key_down(ctx, spottedcat::Key::D) {
            pos[0] += 5.0 * dt_secs;
        }
        spottedcat::set_camera_pos(ctx, pos);

        // FOV control demo (using Q/E)
        static mut CURRENT_FOV: f32 = 45.0;
        unsafe {
            if spottedcat::key_down(ctx, spottedcat::Key::Q) {
                CURRENT_FOV = (CURRENT_FOV - 30.0 * dt_secs).max(10.0);
                spottedcat::set_camera_fovy(ctx, CURRENT_FOV);
            }
            if spottedcat::key_down(ctx, spottedcat::Key::E) {
                CURRENT_FOV = (CURRENT_FOV + 30.0 * dt_secs).min(120.0);
                spottedcat::set_camera_fovy(ctx, CURRENT_FOV);
            }
        }
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
