use spottedcat::{
    Context, DrawOption3D, Model, ModelShaderTemplate, ShaderOpts, Spot, WindowConfig,
    register_model_shader_template,
};
use std::time::Duration;

struct MetalSphere {
    sphere: Model,
    rotation: f32,
    shader_id: u32,
}

impl Spot for MetalSphere {
    fn initialize(ctx: &mut Context) -> Self {
        // 1. Create a smooth sphere
        let sphere = spottedcat::model::create_sphere(ctx, 1.0).unwrap();

        // 2. Register a metallic shader from the model template API.
        let shader_id = register_model_shader_template(
            ctx,
            ModelShaderTemplate::new()
                .with_shared(
                    r#"
fn tint(c: vec3<f32>) -> vec3<f32> {
    return c * vec3<f32>(0.92, 0.96, 1.0);
}
"#,
                )
                .with_fragment_body(
                    r#"
let N = normalize(in.normal);
let V = normalize(scene.camera_pos.xyz - in.world_pos);
let light_dir = normalize(scene.lights[0].position.xyz);
let half_dir = normalize(V + light_dir);
let diff = max(dot(N, light_dir), 0.0);
let spec = pow(max(dot(N, half_dir), 0.0), 64.0);
let fresnel = pow(1.0 - max(dot(N, V), 0.0), 5.0);
let final_rgb = tint(src.rgb) * (0.18 + diff * 0.72) + vec3<f32>(spec * (1.2 + fresnel));
return vec4<f32>(final_rgb, src.a * model_globals.extra.x);
"#,
                ),
        );

        Self {
            sphere,
            rotation: 0.0,
            shader_id,
        }
    }

    fn update(&mut self, _ctx: &mut Context, dt: Duration) {
        self.rotation += dt.as_secs_f32() * 0.5;
    }

    fn draw(&mut self, ctx: &mut Context, screen: spottedcat::Image) {
        let opts = DrawOption3D::default()
            .with_position([0.0, 0.0, 0.0])
            .with_rotation([0.0, self.rotation, 0.0]);

        // Draw with our metallic shader
        spottedcat::model::draw_with_shader(
            ctx,
            screen,
            &self.sphere,
            self.shader_id,
            opts,
            ShaderOpts::default(),
            None,
        );
    }

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    spottedcat::run::<MetalSphere>(WindowConfig {
        title: "Metal Sphere Example".to_string(),
        ..Default::default()
    });
}
