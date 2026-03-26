use spottedcat::{Context, Spot, Model, DrawOption3D, ShaderOpts, WindowConfig};
use std::time::Duration;

struct MetalSphere {
    sphere: Model,
    rotation: f32,
    shader_id: u32,
}

impl Spot for MetalSphere {
    fn initialize(_context: &mut Context) -> Self {
        // 1. Create a smooth sphere
        let sphere = Model::sphere(1.0).unwrap();

        // 2. Register a custom "metallic" shader
        // It uses the normal to calculate specular reflection
        let shader_src = r#"
            fn user_fs_hook() {
                // N and V are available in the fs_main scope of model.wgsl
                let light_dir = normalize(scene.lights[0].position.xyz);
                let half_dir = normalize(V + light_dir);
                
                let diff_val = max(dot(N, light_dir), 0.0);
                let spec = pow(max(dot(N, half_dir), 0.0), 32.0);
                
                let base_color = vec3<f32>(0.8, 0.8, 0.9); // Silver-ish
                let final_rgb = base_color * (diff_val * 0.5 + 0.2) + vec3<f32>(spec);
                
                final_color = vec4<f32>(final_rgb, final_color.a);
            }
        "#;
        let shader_id = spottedcat::register_model_shader(shader_src);

        Self {
            sphere,
            rotation: 0.0,
            shader_id,
        }
    }

    fn update(&mut self, _context: &mut Context, dt: Duration) {
        self.rotation += dt.as_secs_f32() * 0.5;
    }

    fn draw(&mut self, context: &mut Context) {
        let opts = DrawOption3D::default()
            .with_position([0.0, 0.0, 0.0])
            .with_rotation([0.0, self.rotation, 0.0]);
        
        // Draw with our metallic shader
        self.sphere.draw_with_shader(context, self.shader_id, opts, ShaderOpts::default(), None);
    }

    fn remove(&self) {}
}

fn main() {
    spottedcat::run::<MetalSphere>(WindowConfig {
        title: "Metal Sphere Example".to_string(),
        ..Default::default()
    });
}
