use spottedcat::{Context, DrawOption3D, Image, Model, Spot, WindowConfig};
use std::time::Duration;

struct BillboardExample {
    wall: Model,
    character: Model,
    billboard_plane: Model,
    time: f32,
}

impl Spot for BillboardExample {
    fn initialize(ctx: &mut Context) -> Self {
        // Create a large wall to demonstrate occlusion
        let wall_pixels = vec![
            180, 180, 180, 255, 200, 200, 200, 255, 200, 200, 200, 255, 180, 180, 180, 255,
        ];
        let wall_tex = spottedcat::create_image(ctx, 2.into(), 2.into(), &wall_pixels).unwrap();
        // A wall is just a stretched cube
        let wall = spottedcat::model::create_cube(ctx, 1.0)
            .unwrap()
            .with_material(wall_tex);

        // Character (a small cube)
        let char_pixels = vec![
            50, 50, 255, 255, 100, 100, 255, 255, 100, 100, 255, 255, 50, 50, 255, 255,
        ];
        let char_tex = spottedcat::create_image(ctx, 2.into(), 2.into(), &char_pixels).unwrap();
        let character = spottedcat::model::create_cube(ctx, 0.5)
            .unwrap()
            .with_material(char_tex);

        // Create a billboard plane for the name tag/health bar
        let mut bb_pixels = vec![0; 4 * 64 * 16];
        for y in 0..16 {
            for x in 0..64 {
                let idx = ((y * 64) + x) as usize * 4;
                // Green health bar, red background
                if x < 48 {
                    // 75% Health
                    bb_pixels[idx] = 0;
                    bb_pixels[idx + 1] = 255;
                    bb_pixels[idx + 2] = 0;
                    bb_pixels[idx + 3] = 255;
                } else {
                    // Lost Health
                    bb_pixels[idx] = 255;
                    bb_pixels[idx + 1] = 0;
                    bb_pixels[idx + 2] = 0;
                    bb_pixels[idx + 3] = 255;
                }
            }
        }
        let bb_tex = spottedcat::create_image(ctx, 64.into(), 16.into(), &bb_pixels).unwrap();

        // 1.0 wide, 0.25 tall
        let billboard_plane = spottedcat::model::create_plane(ctx, 1.0, 0.25)
            .unwrap()
            .with_material(bb_tex);

        Self {
            wall,
            character,
            billboard_plane,
            time: 0.0,
        }
    }

    fn update(&mut self, _ctx: &mut Context, dt: Duration) {
        self.time += dt.as_secs_f32();
    }

    fn draw(&mut self, ctx: &mut Context) {
        // Draw the static wall in the middle
        let wall_opts = DrawOption3D::default()
            .with_position([0.0, 0.0, 0.0])
            .with_scale([0.2, 2.0, 2.0]);
        spottedcat::model::draw(ctx, &self.wall, wall_opts);

        // Calculate a position orbiting around the wall
        let orb_x = (self.time).cos() * 2.0;
        let orb_z = (self.time).sin() * 2.0;
        let char_pos = [orb_x, -0.5, orb_z];

        let char_opts = DrawOption3D::default().with_position(char_pos);
        spottedcat::model::draw(ctx, &self.character, char_opts);

        // -- IMPLEMENTING OPTION 1: 3D BILLBOARD --
        // Now draw the Billboard Plane ABOVE the character!
        let bb_pos = [char_pos[0], char_pos[1] + 0.6, char_pos[2]];

        // In a real game with a moving camera, you would calculate a LookAt rotation matrix here
        // so the plane always faces the camera.
        // In `spot`, the default camera is fixed at [0,0,-5] looking at +Z.
        // So a default rotation of [0,0,0] for our +Z facing plane makes it perfectly parallel to the screen.
        let bb_opts = DrawOption3D::default()
            .with_position(bb_pos)
            .with_rotation([0.0, 0.0, 0.0]); // Always face camera

        // The name tag will naturally be occluded by the wall when the character walks behind it!
        spottedcat::model::draw(ctx, &self.billboard_plane, bb_opts);
    }

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    spottedcat::run::<BillboardExample>(WindowConfig {
        title: "Billboard (Option 1) Example".to_string(),
        ..Default::default()
    });
}
