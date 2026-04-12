use spottedcat::{
    Context, DrawOption, Image, ImageShaderBindings, ImageShaderBlendMode, ImageShaderDesc, Pt,
    ShaderOpts, Spot, WindowConfig, register_image_shader_desc,
};

const FULL_SHADER: &str = r#"
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

@group(1) @binding(0) var t_history: texture_2d<f32>;
@group(1) @binding(1) var t_noise: texture_2d<f32>;
@group(1) @binding(2) var t_screen: texture_2d<f32>;
@group(1) @binding(3) var t_unused: texture_2d<f32>;
@group(1) @binding(4) var extra_samp: sampler;

@group(2) @binding(0) var<uniform> user_globals: array<vec4<f32>, 16>;

struct EngineGlobals {
    screen: vec4<f32>,
    opacity: f32,
    shader_opacity: f32,
    scale_factor: f32,
    _padding: f32,
};

@group(3) @binding(0) var<uniform> _sp_internal: EngineGlobals;

struct VsIn {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) pos: vec2<f32>,
    @location(1) rotation: f32,
    @location(2) size: vec2<f32>,
    @location(3) uv_rect: vec4<f32>,
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) local_uv: vec2<f32>,
    @location(2) uv_scale: vec2<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;

    var pos_arr = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0,  1.0)
    );
    var uv_arr = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0)
    );

    let local_pos = pos_arr[in.vertex_index];
    let uv = uv_arr[in.vertex_index];
    let sw_inv_2 = _sp_internal.screen.x;
    let sh_inv_2 = _sp_internal.screen.y;
    let sw_inv = _sp_internal.screen.z;
    let sh_inv = _sp_internal.screen.w;

    let tx = in.pos.x * sw_inv_2 - 1.0;
    let ty = 1.0 - in.pos.y * sh_inv_2;
    let sx = in.size.x * sw_inv;
    let sy = in.size.y * sh_inv;

    let c = cos(in.rotation);
    let s = sin(in.rotation);
    let dx = tx - (c * sx * -1.0 - s * sy * 1.0);
    let dy = ty - (s * sx * -1.0 + c * sy * 1.0);

    let x = local_pos.x * (c * sx) + local_pos.y * (-s * sy) + dx;
    let y = local_pos.x * (s * sx) + local_pos.y * (c * sy) + dy;

    out.clip_pos = vec4<f32>(x, y, 0.0, 1.0);
    out.local_uv = uv;
    out.uv = vec2<f32>(
        in.uv_rect.x + uv.x * in.uv_rect.z,
        in.uv_rect.y + uv.y * in.uv_rect.w
    );
    out.uv_scale = in.uv_rect.zw;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let src = textureSample(tex, samp, in.uv);
    let history = textureSample(t_history, extra_samp, in.uv);
    let screen = textureSample(t_screen, extra_samp, in.uv);
    let time = user_globals[0].x;
    let tint = user_globals[1].rgb;
    let noise_uv = fract(in.local_uv * 3.0 + vec2<f32>(time * 0.13, time * 0.09));
    let noise = textureSample(t_noise, extra_samp, noise_uv).rgb;

    let trail = history.rgb * 0.94;
    let shimmer = tint * (0.65 + noise * 0.8);
    let pulse = 0.55 + 0.45 * sin(time * 2.2 + in.local_uv.x * 6.2831);
    let composed = max(trail * 0.98 + screen.rgb * 0.08, src.rgb * shimmer * pulse);
    let alpha = max(src.a, history.a * 0.96) * _sp_internal.opacity * _sp_internal.shader_opacity;

    return vec4<f32>(composed, alpha);
}
"#;

struct FullShaderExample {
    sprite: Image,
    noise: Image,
    shader_id: u32,
    time: f32,
}

impl Spot for FullShaderExample {
    fn initialize(ctx: &mut Context) -> Self {
        let sprite = Image::new(ctx, Pt::from(96.0), Pt::from(96.0), &build_sprite_rgba()).unwrap();
        let noise = Image::new(ctx, Pt::from(64.0), Pt::from(64.0), &build_noise_rgba()).unwrap();
        let shader_id = register_image_shader_desc(
            ctx,
            ImageShaderDesc::from_wgsl(FULL_SHADER)
                .with_extra_textures(true)
                .with_blend_mode(ImageShaderBlendMode::Add),
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
    spottedcat::run::<FullShaderExample>(WindowConfig {
        title: "Full Image Shader".to_string(),
        width: Pt::from(960.0),
        height: Pt::from(640.0),
        ..Default::default()
    });
}
