use spottedcat::{Context, DrawOption, Image, Pt, Spot, WindowConfig};
use std::time::Duration;

use bytemuck;

struct CustomShaderScene {
    tree: Image,
    tree2: Image,
    tree3: Image,
    tree4: Image,
    negative_shader_id: u32,
    grayscale_shader_id: u32,
    ripple_shader_id: u32,
    circle_shader_id: u32,
    t: f32,
}

impl CustomShaderScene {
    fn _dummy(&mut self, _context: &Context) {}
}

impl Spot for CustomShaderScene {
    fn initialize(_context: &mut Context) -> Self {
        const TREE_PNG: &[u8] = include_bytes!("../assets/happy-tree.png");
        let decoded = image::load_from_memory(TREE_PNG).expect("failed to decode happy-tree.png");
        let rgba = decoded.to_rgba8();
        let (w, h) = (rgba.width(), rgba.height());
        let tree = Image::new_from_rgba8(Pt::from(w), Pt::from(h), rgba.as_raw())
            .expect("failed to create happy-tree image");
        let tree2 = Image::new_from_image(tree).expect("failed to create happy-tree copy");
        let tree3 = Image::new_from_image(tree).expect("failed to create happy-tree copy");
        let tree4 = Image::new_from_image(tree).expect("failed to create happy-tree copy");

        // IMPORTANT: custom image shaders must match the engine's instance vertex layout.
        // That means vs_main takes the same VsIn (locations 0..3) and outputs uv.

        // Custom shader 1: Negative effect
        let negative_shader_src = r#"
@group(0) @binding(0)
var tex: texture_2d<f32>;

@group(0) @binding(1)
var samp: sampler;

struct ImageGlobals {
    data: array<vec4<f32>, 16>,
};

@group(1) @binding(0)
var<uniform> g: ImageGlobals;

struct VsIn {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) mvp_col0: vec2<f32>,
    @location(1) mvp_col1: vec2<f32>,
    @location(2) mvp_col3: vec2<f32>,
    @location(3) uv_rect: vec4<f32>,
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) uv_rect: vec4<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;

    // Triangle Strip Quad: BL, BR, TL, TR
    var pos_arr = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0,  1.0)
    );
    // UVs follow pos: (0,1), (1,1), (0,0), (1,0)
    var uv_arr = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0)
    );

    let pos = pos_arr[in.vertex_index];
    let uv = uv_arr[in.vertex_index];

    // Reconstruct position from compressed MVP columns
    let x = pos.x * in.mvp_col0.x + pos.y * in.mvp_col1.x + in.mvp_col3.x;
    let y = pos.x * in.mvp_col0.y + pos.y * in.mvp_col1.y + in.mvp_col3.y;
    out.clip_pos = vec4<f32>(x, y, 0.0, 1.0);

    // UVs: u = u0 + uv.x * w, v = v0 + uv.y * h
    out.uv = vec2<f32>(
        in.uv_rect.x + uv.x * in.uv_rect.z,
        in.uv_rect.y + uv.y * in.uv_rect.w
    );
    out.uv_rect = in.uv_rect;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let color = textureSample(tex, samp, in.uv);
    let k = g.data[0].y;
    let kk = clamp(k, 0.0, 1.0);
    let rgb = mix(color.rgb, vec3<f32>(1.0) - color.rgb, kk);
    return vec4<f32>(rgb, color.a);
}
"#;

        // Custom shader 4: Circle mask/outline
        let circle_shader_src = r#"
@group(0) @binding(0)
var tex: texture_2d<f32>;

@group(0) @binding(1)
var samp: sampler;

struct ImageGlobals {
    data: array<vec4<f32>, 16>,
};

@group(1) @binding(0)
var<uniform> g: ImageGlobals;

struct VsIn {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) mvp_col0: vec2<f32>,
    @location(1) mvp_col1: vec2<f32>,
    @location(2) mvp_col3: vec2<f32>,
    @location(3) uv_rect: vec4<f32>,
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) uv_rect: vec4<f32>,
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

    let pos = pos_arr[in.vertex_index];
    let uv = uv_arr[in.vertex_index];

    let x = pos.x * in.mvp_col0.x + pos.y * in.mvp_col1.x + in.mvp_col3.x;
    let y = pos.x * in.mvp_col0.y + pos.y * in.mvp_col1.y + in.mvp_col3.y;
    out.clip_pos = vec4<f32>(x, y, 0.0, 1.0);

    out.uv = vec2<f32>(
        in.uv_rect.x + uv.x * in.uv_rect.z,
        in.uv_rect.y + uv.y * in.uv_rect.w
    );
    out.uv_rect = in.uv_rect;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // g.data[0] = [radius, thickness, softness, mix]
    // g.data[1] = [r, g, b, a]
    let radius = g.data[0].x;
    let thickness = g.data[0].y;
    let softness = max(g.data[0].z, 0.0001);
    let mixv = clamp(g.data[0].w, 0.0, 1.0);
    let ring_color = g.data[1];

    let base = textureSample(tex, samp, in.uv);
    let local_uv = (in.uv - in.uv_rect.xy) / in.uv_rect.zw;
    let d = distance(local_uv, vec2<f32>(0.5, 0.5));
    let half_t = 0.5 * thickness;
    let dist_to_ring = abs(d - radius);
    let ring = 1.0 - smoothstep(half_t, half_t + softness, dist_to_ring);
    let overlay = vec4<f32>(ring_color.rgb, ring_color.a) * ring;
    return mix(base, overlay, mixv);
}
"#;

        // Custom shader 2: Grayscale
        let grayscale_shader_src = r#"
@group(0) @binding(0)
var tex: texture_2d<f32>;

@group(0) @binding(1)
var samp: sampler;

struct ImageGlobals {
    data: array<vec4<f32>, 16>,
};

@group(1) @binding(0)
var<uniform> g: ImageGlobals;

struct VsIn {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) mvp_col0: vec2<f32>,
    @location(1) mvp_col1: vec2<f32>,
    @location(2) mvp_col3: vec2<f32>,
    @location(3) uv_rect: vec4<f32>,
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
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

    let pos = pos_arr[in.vertex_index];
    let uv = uv_arr[in.vertex_index];

    let x = pos.x * in.mvp_col0.x + pos.y * in.mvp_col1.x + in.mvp_col3.x;
    let y = pos.x * in.mvp_col0.y + pos.y * in.mvp_col1.y + in.mvp_col3.y;
    out.clip_pos = vec4<f32>(x, y, 0.0, 1.0);

    out.uv = vec2<f32>(
        in.uv_rect.x + uv.x * in.uv_rect.z,
        in.uv_rect.y + uv.y * in.uv_rect.w
    );
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let color = textureSample(tex, samp, in.uv);
    let l = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let k = clamp(g.data[0].x, 0.0, 1.0);
    return vec4<f32>(mix(color.rgb, vec3<f32>(l), k), color.a);
}
"#;

        // Custom shader 3: Ripple (water wave) distortion
        let ripple_shader_src = r#"
@group(0) @binding(0)
var tex: texture_2d<f32>;

@group(0) @binding(1)
var samp: sampler;

struct ImageGlobals {
    data: array<vec4<f32>, 16>,
};

@group(1) @binding(0)
var<uniform> g: ImageGlobals;

struct VsIn {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) mvp_col0: vec2<f32>,
    @location(1) mvp_col1: vec2<f32>,
    @location(2) mvp_col3: vec2<f32>,
    @location(3) uv_rect: vec4<f32>,
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
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

    let pos = pos_arr[in.vertex_index];
    let uv = uv_arr[in.vertex_index];

    let x = pos.x * in.mvp_col0.x + pos.y * in.mvp_col1.x + in.mvp_col3.x;
    let y = pos.x * in.mvp_col0.y + pos.y * in.mvp_col1.y + in.mvp_col3.y;
    out.clip_pos = vec4<f32>(x, y, 0.0, 1.0);

    out.uv = vec2<f32>(
        in.uv_rect.x + uv.x * in.uv_rect.z,
        in.uv_rect.y + uv.y * in.uv_rect.w
    );
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // g.data[0] = [t, amp, freq, speed]
    let t = g.data[0].x;
    let amp = g.data[0].y;
    let freq = g.data[0].z;
    let speed = g.data[0].w;

    let center = vec2<f32>(0.5, 0.5);
    let d = in.uv - center;
    let r = length(d);
    let dir = select(vec2<f32>(0.0, 0.0), d / r, r > 0.0001);

    let w = sin(r * freq - t * speed);
    let uv2 = in.uv + dir * (w * amp);
    let c = textureSample(tex, samp, clamp(uv2, vec2<f32>(0.0), vec2<f32>(1.0)));
    return c;
}
"#;

        let negative_shader_id = spottedcat::register_image_shader(negative_shader_src);
        let grayscale_shader_id = spottedcat::register_image_shader(grayscale_shader_src);
        let ripple_shader_id = spottedcat::register_image_shader(ripple_shader_src);
        let circle_shader_id = spottedcat::register_image_shader(circle_shader_src);

        Self {
            tree,
            tree2,
            tree3,
            tree4,
            negative_shader_id,
            grayscale_shader_id,
            ripple_shader_id,
            circle_shader_id,
            t: 0.0,
        }
    }

    fn draw(&mut self, context: &mut Context) {
        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        struct GrayscaleGlobals {
            k: f32,
            _pad: [f32; 3],
        }

        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        struct RippleGlobals {
            t: f32,
            amp: f32,
            freq: f32,
            speed: f32,
        }

        // Baseline (no shader) vs custom shader.

        let opts_a = DrawOption::default()
            .with_position([Pt::from(80.0), Pt::from(120.0)])
            .with_scale([0.8, 0.8])
            .with_rotation(self.t);
        self.tree.draw(context, opts_a);

        let opts_b = DrawOption::default()
            .with_position([Pt::from(420.0), Pt::from(120.0)])
            .with_scale([0.8, 0.8])
            .with_rotation(-self.t);
        self.tree2.draw_with_shader(
            context,
            self.negative_shader_id,
            opts_b,
            spottedcat::ShaderOpts::from_bytes(bytemuck::cast_slice(&[
                0.0f32,
                1.0f32,
                0.0f32,
                0.0f32,
            ])),
        );

        let opts_c = DrawOption::default()
            .with_position([Pt::from(240.0), Pt::from(380.0)])
            .with_scale([0.8, 0.8])
            .with_rotation(self.t * 0.5);

        let globals_c = GrayscaleGlobals { k: 1.0, _pad: [0.0; 3] };
        self.tree2.draw_with_shader(
            context,
            self.grayscale_shader_id,
            opts_c,
            spottedcat::ShaderOpts::from_pod(&globals_c),
        );

        let opts_d = DrawOption::default()
            .with_position([Pt::from(520.0), Pt::from(380.0)])
            .with_scale([0.8, 0.8])
            .with_rotation(0.0);
        let globals_d = RippleGlobals {
            t: self.t,
            amp: 0.02,
            freq: 28.0,
            speed: 6.0,
        };
        self.tree3.draw_with_shader(
            context,
            self.ripple_shader_id,
            opts_d,
            spottedcat::ShaderOpts::from_pod(&globals_d),
        );

        let opts_e = DrawOption::default()
            .with_position([Pt::from(80.0), Pt::from(420.0)])
            .with_scale([0.8, 0.8])
            .with_rotation(0.0);
        let circle_bytes: &[u8] = bytemuck::cast_slice(&[
            0.33f32, // radius
            0.01f32, // thickness
            0.02f32, // softness
            1.0f32,  // mix
            1.0f32,  // r
            0.2f32,  // g
            0.2f32,  // b
            1.0f32,  // a
        ]);
        self.tree4.draw_with_shader(
            context,
            self.circle_shader_id,
            opts_e,
            spottedcat::ShaderOpts::from_bytes(circle_bytes),
        );
    }

    fn update(&mut self, _context: &mut Context, dt: Duration) {
        self.t += dt.as_secs_f32();
    }

    fn remove(&self) {}
}

fn main() {
    let mut config = WindowConfig::default();
    config.title = "Custom Shader Example".to_string();
    spottedcat::run::<CustomShaderScene>(config);
}
