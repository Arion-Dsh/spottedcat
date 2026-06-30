use spottedcat::{
    Context, DrawOption, Image, ImageShaderBindings, ImageShaderBlendMode, ImageShaderTemplate,
    Key, Pt, ShaderOpts, Spot, Text, WindowConfig, register_image_shader_template,
};
use wasm_bindgen::prelude::*;

const WIDTH: f32 = 640.0;
const HEIGHT: f32 = 480.0;

fn web_config(title: &str) -> WindowConfig {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        let mut config = WindowConfig::default();
        config.canvas_id = Some("spot-canvas".to_string());
        config.width = Pt::from(WIDTH);
        config.height = Pt::from(HEIGHT);
        config.title = title.to_string();
        config
    }

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        WindowConfig {
            title: title.to_string(),
            width: Pt::from(WIDTH),
            height: Pt::from(HEIGHT),
            ..Default::default()
        }
    }
}

fn register_demo_font(ctx: &mut Context) -> u32 {
    const FONT: &[u8] = include_bytes!("../../../../assets/DejaVuSans.ttf");
    spottedcat::register_font(ctx, FONT.to_vec())
}

fn draw_text(
    ctx: &mut Context,
    screen: Image,
    font_id: u32,
    text: impl Into<String>,
    size: f32,
    color: [f32; 4],
    x: f32,
    y: f32,
) {
    let text = Text::new(text.into(), font_id)
        .with_font_size(Pt::from(size))
        .with_color(color);
    screen.draw(
        ctx,
        &text,
        DrawOption::default().with_position([Pt::from(x), Pt::from(y)]),
    );
}

fn image_from_rgba(ctx: &mut Context, width: usize, height: usize, rgba: &[u8]) -> Image {
    Image::new(ctx, Pt::from(width as f32), Pt::from(height as f32), rgba)
        .expect("create demo image")
}

fn solid_image(ctx: &mut Context, width: usize, height: usize, color: [u8; 4]) -> Image {
    let mut rgba = vec![0; width * height * 4];
    for px in rgba.chunks_exact_mut(4) {
        px.copy_from_slice(&color);
    }
    image_from_rgba(ctx, width, height, &rgba)
}

struct InputDemo {
    marker: Image,
    font_id: u32,
    x: f32,
    y: f32,
    speed: f32,
}

impl Spot for InputDemo {
    fn initialize(ctx: &mut Context) -> Self {
        Self {
            marker: solid_image(ctx, 36, 36, [241, 126, 72, 255]),
            font_id: register_demo_font(ctx),
            x: WIDTH * 0.5,
            y: HEIGHT * 0.5,
            speed: 260.0,
        }
    }

    fn update(&mut self, ctx: &mut Context, dt: std::time::Duration) {
        let dt = dt.as_secs_f32().min(1.0 / 20.0);
        if spottedcat::key_down(ctx, Key::W) || spottedcat::key_down(ctx, Key::Up) {
            self.y -= self.speed * dt;
        }
        if spottedcat::key_down(ctx, Key::S) || spottedcat::key_down(ctx, Key::Down) {
            self.y += self.speed * dt;
        }
        if spottedcat::key_down(ctx, Key::A) || spottedcat::key_down(ctx, Key::Left) {
            self.x -= self.speed * dt;
        }
        if spottedcat::key_down(ctx, Key::D) || spottedcat::key_down(ctx, Key::Right) {
            self.x += self.speed * dt;
        }
        if spottedcat::key_pressed(ctx, Key::Space) {
            self.x = WIDTH * 0.5;
            self.y = HEIGHT * 0.5;
        }
        self.x = self.x.clamp(24.0, WIDTH - 60.0);
        self.y = self.y.clamp(86.0, HEIGHT - 72.0);
    }

    fn draw(&mut self, ctx: &mut Context, screen: Image) {
        draw_text(
            ctx,
            screen,
            self.font_id,
            "Input Demo",
            28.0,
            [0.95, 0.93, 0.88, 1.0],
            24.0,
            24.0,
        );
        draw_text(
            ctx,
            screen,
            self.font_id,
            "Move with WASD / arrow keys. Space resets.",
            18.0,
            [0.72, 0.95, 1.0, 1.0],
            24.0,
            62.0,
        );
        screen.draw(
            ctx,
            &self.marker,
            DrawOption::default().with_position([Pt::from(self.x), Pt::from(self.y)]),
        );
        draw_text(
            ctx,
            screen,
            self.font_id,
            format!("x {:.0}, y {:.0}", self.x, self.y),
            18.0,
            [0.9, 0.9, 0.9, 1.0],
            24.0,
            HEIGHT - 34.0,
        );
    }

    fn remove(&mut self, ctx: &mut Context) {
        spottedcat::unregister_font(ctx, self.font_id);
    }
}

struct ImageDemo {
    image: Image,
    font_id: u32,
    time: f32,
}

impl Spot for ImageDemo {
    fn initialize(ctx: &mut Context) -> Self {
        const W: usize = 300;
        const H: usize = 160;
        let mut rgba = vec![0u8; W * H * 4];
        for y in 0..H {
            for x in 0..W {
                let i = (y * W + x) * 4;
                let color = if x < W / 3 {
                    [255, 64, 64, 255]
                } else if x < W * 2 / 3 {
                    [64, 255, 128, 255]
                } else {
                    [80, 150, 255, 255]
                };
                rgba[i..i + 4].copy_from_slice(&color);
            }
        }
        Self {
            image: image_from_rgba(ctx, W, H, &rgba),
            font_id: register_demo_font(ctx),
            time: 0.0,
        }
    }

    fn update(&mut self, _ctx: &mut Context, dt: std::time::Duration) {
        self.time += dt.as_secs_f32();
    }

    fn draw(&mut self, ctx: &mut Context, screen: Image) {
        let scale = 1.0 + self.time.sin() * 0.08;
        let x = WIDTH * 0.5 - self.image.width().as_f32() * scale * 0.5;
        let y = HEIGHT * 0.5 - self.image.height().as_f32() * scale * 0.5;
        draw_text(
            ctx,
            screen,
            self.font_id,
            "Image Demo",
            28.0,
            [0.95, 0.93, 0.88, 1.0],
            24.0,
            24.0,
        );
        draw_text(
            ctx,
            screen,
            self.font_id,
            "Raw RGBA pixels become an Image, then DrawOption scales it.",
            17.0,
            [0.72, 0.95, 1.0, 1.0],
            24.0,
            62.0,
        );
        screen.draw(
            ctx,
            &self.image,
            DrawOption::default()
                .with_position([Pt::from(x), Pt::from(y)])
                .with_scale([scale, scale]),
        );
    }

    fn remove(&mut self, ctx: &mut Context) {
        spottedcat::unregister_font(ctx, self.font_id);
    }
}

struct ShaderDemo {
    sprite: Image,
    noise: Image,
    shader_id: u32,
    font_id: u32,
    time: f32,
}

impl Spot for ShaderDemo {
    fn initialize(ctx: &mut Context) -> Self {
        let shader_id = register_image_shader_template(
            ctx,
            ImageShaderTemplate::new()
                .with_extra_textures(true)
                .with_blend_mode(ImageShaderBlendMode::Add)
                .with_history_at(0)
                .with_texture_alias(1, "t_noise")
                .with_fragment_body(
                    r#"
let history = textureSample(t_history, extra_samp, in.uv);
let noise = textureSample(t_noise, extra_samp, fract(in.local_uv * 3.0 + vec2<f32>(user_globals[0].x * 0.16, user_globals[0].x * 0.11))).rgb;
let pulse = 0.55 + 0.45 * sin(user_globals[0].x * 3.0 + in.local_uv.x * 6.2831);
let color = src.rgb * vec3<f32>(1.0, 0.45, 0.95) * (0.65 + noise * 0.8) * pulse;
let trail = history.rgb * 0.94;
let alpha = max(src.a, history.a * 0.96) * opacity;
return vec4<f32>(max(trail, color), alpha);
"#,
                ),
        );
        Self {
            sprite: image_from_rgba(ctx, 96, 96, &build_glow_rgba()),
            noise: image_from_rgba(ctx, 64, 64, &build_noise_rgba()),
            shader_id,
            font_id: register_demo_font(ctx),
            time: 0.0,
        }
    }

    fn update(&mut self, _ctx: &mut Context, dt: std::time::Duration) {
        self.time += dt.as_secs_f32();
    }

    fn draw(&mut self, ctx: &mut Context, screen: Image) {
        draw_text(
            ctx,
            screen,
            self.font_id,
            "Shader Demo",
            28.0,
            [0.95, 0.93, 0.88, 1.0],
            24.0,
            24.0,
        );
        draw_text(
            ctx,
            screen,
            self.font_id,
            "A template WGSL shader uses noise, time, and history trails.",
            17.0,
            [0.72, 0.95, 1.0, 1.0],
            24.0,
            62.0,
        );

        let x = WIDTH * 0.5 + self.time.cos() * 170.0 - 96.0;
        let y = HEIGHT * 0.56 + (self.time * 1.7).sin() * 96.0 - 96.0;
        let mut opts = ShaderOpts::default();
        opts.set_vec4(0, [self.time, 0.0, 0.0, 0.0]);
        let bindings = ImageShaderBindings::new()
            .with_history()
            .with_image("t_noise", self.noise);

        screen.draw_with_shader_bindings(
            ctx,
            self.sprite,
            self.shader_id,
            DrawOption::default()
                .with_position([Pt::from(x), Pt::from(y)])
                .with_scale([2.0, 2.0]),
            opts,
            bindings,
        );
    }

    fn remove(&mut self, ctx: &mut Context) {
        spottedcat::unregister_font(ctx, self.font_id);
    }
}

fn build_glow_rgba() -> Vec<u8> {
    let size = 96usize;
    let mut out = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - size as f32 * 0.5;
            let dy = y as f32 - size as f32 * 0.5;
            let dist = (dx * dx + dy * dy).sqrt() / (size as f32 * 0.5);
            let ring = (1.0 - dist).clamp(0.0, 1.0);
            let i = (y * size + x) * 4;
            out[i] = (255.0 * ring) as u8;
            out[i + 1] = (170.0 * ring) as u8;
            out[i + 2] = 255;
            out[i + 3] = if dist < 1.0 { (ring * ring * 255.0) as u8 } else { 0 };
        }
    }
    out
}

fn build_noise_rgba() -> Vec<u8> {
    let size = 64usize;
    let mut out = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let v = (((x * 37 + y * 57 + x * y * 13) % 255) as u8).max(32);
            let i = (y * size + x) * 4;
            out[i] = v;
            out[i + 1] = v.saturating_sub(20);
            out[i + 2] = 255u8.saturating_sub(v / 3);
            out[i + 3] = 255;
        }
    }
    out
}

#[wasm_bindgen]
pub fn run_input_demo() {
    console_error_panic_hook::set_once();
    spottedcat::run::<InputDemo>(web_config("Spottedcat Input Demo"));
}

#[wasm_bindgen]
pub fn run_image_demo() {
    console_error_panic_hook::set_once();
    spottedcat::run::<ImageDemo>(web_config("Spottedcat Image Demo"));
}

#[wasm_bindgen]
pub fn run_shader_demo() {
    console_error_panic_hook::set_once();
    spottedcat::run::<ShaderDemo>(web_config("Spottedcat Shader Demo"));
}

