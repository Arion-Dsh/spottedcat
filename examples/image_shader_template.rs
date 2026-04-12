extern crate spottedcat as sc;

use sc::{
    Context, DrawOption, Image, ImageShaderBindings, ImageShaderBlendMode, ImageShaderTemplate, Pt,
    ShaderOpts, Spot, WindowConfig, register_image_shader_template,
};

struct ImageShaderTemplateExample {
    sprite: Image,
    noise: Image,
    shader_id: u32,
    time: f32,
}

impl Spot for ImageShaderTemplateExample {
    fn initialize(ctx: &mut Context) -> Self {
        let sprite = Image::new(ctx, Pt::from(96.0), Pt::from(96.0), &build_sprite_rgba()).unwrap();
        let noise = Image::new(ctx, Pt::from(64.0), Pt::from(64.0), &build_noise_rgba()).unwrap();
        let shader_id = register_image_shader_template(
            ctx,
            ImageShaderTemplate::new()
                .with_extra_textures(true)
                .with_blend_mode(ImageShaderBlendMode::Add)
                .with_shared(
                    r#"
fn shimmer_tint(color: vec3<f32>, tint: vec3<f32>, noise: vec3<f32>, pulse: f32) -> vec3<f32> {
    return color * tint * (0.65 + noise * 0.8) * pulse;
}
"#,
                )
                .with_vertex_body("out.local_uv = out.local_uv * 0.96 + vec2<f32>(0.02, 0.02);")
                .with_fragment_body(
                    r#"
let history = textureSample(t0, extra_samp, in.uv);
let noise = textureSample(t1, extra_samp, fract(in.local_uv * 3.0 + vec2<f32>(user_globals[0].x * 0.13, user_globals[0].x * 0.09))).rgb;
let screen = textureSample(t2, extra_samp, in.uv);
let tint = user_globals[1].rgb;
let pulse = 0.55 + 0.45 * sin(user_globals[0].x * 2.2 + in.local_uv.x * 6.2831);
let trail = history.rgb * 0.94;
let composed = max(trail * 0.98 + screen.rgb * 0.08, shimmer_tint(src.rgb, tint, noise, pulse));
let alpha = max(src.a, history.a * 0.96) * opacity;
return vec4<f32>(composed, alpha);
"#,
                ),
        );

        Self {
            sprite,
            noise,
            shader_id,
            time: 0.0,
        }
    }

    fn update(&mut self, _ctx: &mut Context, dt: std::time::Duration) {
        self.time += dt.as_secs_f32();
    }

    fn draw(&mut self, ctx: &mut Context, screen: Image) {
        let (w, h) = spottedcat::window_size(ctx);
        let x = w.as_f32() * 0.5 + self.time.cos() * 180.0;
        let y = h.as_f32() * 0.5 + (self.time * 1.7).sin() * 120.0;

        let mut shader_opts = ShaderOpts::default();
        shader_opts.set_vec4(0, [self.time, 0.0, 0.0, 0.0]);
        shader_opts.set_vec4(1, [1.0, 0.45, 0.9, 1.0]);

        let bindings = ImageShaderBindings::new()
            .with_history(0)
            .with_extra_image(1, self.noise)
            .with_screen(2);

        screen.draw_with_shader_bindings(
            ctx,
            self.sprite,
            self.shader_id,
            DrawOption::default()
                .with_position([Pt::from(x), Pt::from(y)])
                .with_scale([2.0, 2.0]),
            shader_opts,
            bindings,
        );
    }

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn build_sprite_rgba() -> Vec<u8> {
    let size = 96u32;
    let mut out = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - size as f32 * 0.5;
            let dy = y as f32 - size as f32 * 0.5;
            let dist = (dx * dx + dy * dy).sqrt() / (size as f32 * 0.5);
            let ring = (1.0 - dist).clamp(0.0, 1.0);
            let alpha = if dist < 1.0 {
                (ring * ring * 255.0) as u8
            } else {
                0
            };
            let offset = ((y * size + x) * 4) as usize;
            out[offset] = (255.0 * ring) as u8;
            out[offset + 1] = (180.0 * ring) as u8;
            out[offset + 2] = 255;
            out[offset + 3] = alpha;
        }
    }
    out
}

fn build_noise_rgba() -> Vec<u8> {
    let size = 64u32;
    let mut out = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let v = (((x * 37 + y * 57 + x * y * 13) % 255) as u8).max(32);
            let offset = ((y * size + x) * 4) as usize;
            out[offset] = v;
            out[offset + 1] = v.saturating_sub(20);
            out[offset + 2] = 255u8.saturating_sub(v / 3);
            out[offset + 3] = 255;
        }
    }
    out
}

fn main() {
    spottedcat::run::<ImageShaderTemplateExample>(WindowConfig {
        title: "Image Shader Template".to_string(),
        width: Pt::from(960.0),
        height: Pt::from(640.0),
        ..Default::default()
    });
}
