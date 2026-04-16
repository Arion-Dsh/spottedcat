use spottedcat::{
    Context, DrawOption3D, Image, Model, ModelShaderTemplate, Pt, ShaderOpts, Spot, WindowConfig,
    register_model_shader_template,
};
use std::time::Duration;

struct MixedModelShaderStress {
    models: Vec<Model>,
    shader_id: u32,
    time: f32,
}

fn checker_texture(ctx: &mut Context, a: [u8; 4], b: [u8; 4]) -> Image {
    let rgba = vec![
        a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3], b[0], b[1], b[2], b[3], a[0], a[1], a[2],
        a[3],
    ];
    Image::new(ctx, Pt::from(2.0), Pt::from(2.0), &rgba).expect("checker texture should be created")
}

impl Spot for MixedModelShaderStress {
    fn initialize(ctx: &mut Context) -> Self {
        let red = checker_texture(ctx, [255, 96, 96, 255], [160, 32, 32, 255]);
        let green = checker_texture(ctx, [96, 255, 160, 255], [24, 120, 64, 255]);
        let blue = checker_texture(ctx, [96, 168, 255, 255], [24, 56, 144, 255]);
        let gold = checker_texture(ctx, [255, 220, 96, 255], [160, 120, 24, 255]);

        let shader_id = register_model_shader_template(
            ctx,
            ModelShaderTemplate::new().with_fragment_body(
                r#"
let pulse = 0.6 + user_globals[2].x * 0.4;
let rim = pow(1.0 - max(dot(normalize(in.normal), normalize(scene.camera_pos.xyz - in.world_pos)), 0.0), 3.0);
let lit = src.rgb * pulse + vec3<f32>(rim * 0.35, rim * 0.2, rim * 0.45);
return vec4<f32>(lit, src.a * model_globals.extra.x);
"#,
            ),
        );

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

        Self {
            models: vec![cube_red, sphere_blue, cube_green, sphere_gold],
            shader_id,
            time: 0.0,
        }
    }

    fn update(&mut self, ctx: &mut Context, dt: Duration) {
        self.time += dt.as_secs_f32();
        spottedcat::set_camera_pos(ctx, [0.0, 12.0, 28.0]);
        spottedcat::set_camera_target(ctx, 0.0, 0.0, -18.0);
    }

    fn draw(&mut self, ctx: &mut Context, screen: spottedcat::Image) {
        let columns = 36;
        let rows = 24;

        for row in 0..rows {
            for col in 0..columns {
                let draw_index = row * columns + col;
                let model = &self.models[(draw_index * 5 + row * 3) % self.models.len()];
                let x = (col as f32 - columns as f32 * 0.5) * 1.7;
                let y = (row as f32 - rows as f32 * 0.5) * 1.2;
                let z = -5.0 - ((row + col) % 6) as f32 * 1.35;
                let spin = self.time * 0.85 + draw_index as f32 * 0.025;
                let opts = DrawOption3D::default()
                    .with_position([x, y, z])
                    .with_rotation([spin * 0.3, spin, 0.0]);

                if (row + col) % 2 == 0 {
                    screen.draw(ctx, model, opts);
                } else {
                    let mut shader_opts = ShaderOpts::default();
                    shader_opts.set_vec4(2, [0.5 + row as f32 / rows as f32, 0.0, 0.0, 0.0]);
                    spottedcat::model::draw_with_shader(
                        ctx,
                        screen,
                        model,
                        self.shader_id,
                        opts,
                        shader_opts,
                        None,
                    );
                }
            }
        }
    }

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    unsafe {
        std::env::set_var("SPOT_PROFILE_RENDER", "1");
    }

    spottedcat::run::<MixedModelShaderStress>(WindowConfig {
        title: "Mixed Model Shader Stress".to_string(),
        width: Pt::from(1440.0),
        height: Pt::from(900.0),
        ..Default::default()
    });
}
