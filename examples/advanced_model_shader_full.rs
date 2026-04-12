use spottedcat::{Context, DrawOption3D, Model, ShaderOpts, Spot, WindowConfig};
use std::time::Duration;

const METAL_SHADER_SRC: &str = include_str!("shaders/metal_sphere_model.wgsl");

struct AdvancedModelShaderFull {
    sphere: Model,
    rotation: f32,
    shader_id: u32,
}

impl Spot for AdvancedModelShaderFull {
    fn initialize(ctx: &mut Context) -> Self {
        let sphere = spottedcat::model::create_sphere(ctx, 1.0).unwrap();
        let shader_id = spottedcat::register_model_shader(ctx, METAL_SHADER_SRC);

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
    spottedcat::run::<AdvancedModelShaderFull>(WindowConfig {
        title: "Advanced Model Shader Full".to_string(),
        ..Default::default()
    });
}
