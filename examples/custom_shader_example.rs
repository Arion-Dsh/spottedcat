use spottedcat::{Context, DrawOption, Image, Pt, Spot, WindowConfig};
use std::time::Duration;

struct CustomShaderScene {
    tree: Image,
    tree2: Image,
    tree3: Image,
    tree4: Image,
    tree5: Image,
    negative_shader_id: u32,
    grayscale_shader_id: u32,
    ripple_shader_id: u32,
    circle_shader_id: u32,
    ripple_phase: f32,
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
        let tree5 = Image::new_from_image(tree).expect("failed to create happy-tree copy");

        // IMPORTANT: custom image shaders must match the engine's instance vertex layout.
        // That means vs_main takes the same VsIn (locations 0..3) and outputs uv.
        // Now users can override specific functions instead of rewriting the entire shader.

        let negative_shader_src = r#"
fn user_fs_hook(in: VsOut, color: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(vec3<f32>(1.0) - color.rgb, color.a);
}
"#;

        // Circle mask (outside)
        // user_globals[0] = [radius, _unused, softness, mix]
        // user_globals[1] = [r, g, b, a]
        let circle_shader_src = r#"
fn user_fs_hook(in: VsOut, color: vec4<f32>) -> vec4<f32> {
    let radius = user_globals[0].x;
    let softness = max(user_globals[0].z, 0.0001);
    let mixv = clamp(user_globals[0].w, 0.0, 1.0);
    let ring_color = user_globals[1];

    let p = (in.local_uv - vec2<f32>(0.5, 0.5)) * 2.0;
    let d = length(p);

    // Outside mask: 0.0 inside the circle, 1.0 outside (with smooth edge)
    let outside = smoothstep(radius, radius + softness, d);

    let base = color;
    let overlay = vec4<f32>(ring_color.rgb, ring_color.a);
    return mix(base, overlay, outside * mixv);
}
"#;

        let grayscale_shader_src = r#"
fn user_fs_hook(in: VsOut, color: vec4<f32>) -> vec4<f32> {
    let l = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
    return vec4<f32>(vec3<f32>(l), color.a);
}
"#;

        // user_globals[0] = [amp, freq, phase, _]
        let ripple_shader_src = r#"
fn user_fs_hook(in: VsOut, color: vec4<f32>) -> vec4<f32> {
    let amp = user_globals[0].x;
    let freq = user_globals[0].y;
    let phase = user_globals[0].z;

    let wave = sin(in.local_uv.y * freq + phase) * amp;
    // Offset in atlas UV space: scale by the sub-rect size to keep sampling within the image.
    let uv2 = in.uv + vec2<f32>(wave * in.uv_scale.x, 0.0);
    let c2 = textureSample(tex, samp, uv2);
    return vec4<f32>(c2.rgb, color.a);
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
            tree5,
            negative_shader_id,
            grayscale_shader_id,
            ripple_shader_id,
            circle_shader_id,
            ripple_phase: 0.0,
        }
    }

    fn draw(&mut self, context: &mut Context) {
        // Default rendering
        let opts_a = DrawOption::default()
            .with_position([Pt::from(80.0), Pt::from(80.0)])
            .with_scale([0.8, 0.8])
            .with_rotation(0.0);
        self.tree.draw(context, opts_a);

        // Negative
        let opts_b = DrawOption::default()
            .with_position([Pt::from(300.0), Pt::from(80.0)])
            .with_scale([0.8, 0.8])
            .with_rotation(0.0);
        let mut shader_opts_b = spottedcat::ShaderOpts::default();
        shader_opts_b.set_opacity(1.0);
        self.tree2.draw_with_shader(context, self.negative_shader_id, opts_b, shader_opts_b);

        // Grayscale
        let opts_c = DrawOption::default()
            .with_position([Pt::from(520.0), Pt::from(80.0)])
            .with_scale([0.8, 0.8])
            .with_rotation(0.0);
        let mut shader_opts_c = spottedcat::ShaderOpts::default();
        shader_opts_c.set_opacity(1.0);
        self.tree3.draw_with_shader(context, self.grayscale_shader_id, opts_c, shader_opts_c);

        // Ripple (static)
        let opts_d = DrawOption::default()
            .with_position([Pt::from(80.0), Pt::from(300.0)])
            .with_scale([0.8, 0.8])
            .with_rotation(0.0);
        let mut shader_opts_d = spottedcat::ShaderOpts::default();
        // user_globals[0] = [amp, freq, phase, _]
        shader_opts_d.set_vec4(0, [0.02, 40.0, self.ripple_phase, 0.0]);
        shader_opts_d.set_opacity(1.0);
        self.tree4.draw_with_shader(context, self.ripple_shader_id, opts_d, shader_opts_d);

        // Circle outside mask
        let opts_e = DrawOption::default()
            .with_position([Pt::from(300.0), Pt::from(300.0)])
            .with_scale([0.8, 0.8])
            .with_rotation(0.0);
        let mut shader_opts_e = spottedcat::ShaderOpts::default();
        shader_opts_e.set_vec4(0, [0.55, 0.0, 0.02, 1.0]);
        shader_opts_e.set_vec4(1, [1.0, 0.2, 0.2, 1.0]);
        shader_opts_e.set_opacity(1.0);
        self.tree5.draw_with_shader(context, self.circle_shader_id, opts_e, shader_opts_e);
    }

    fn update(&mut self, _context: &mut Context, dt: Duration) {
        // Advance phase to animate the ripple without affecting other shaders.
        self.ripple_phase += dt.as_secs_f32() * 4.0;
    }

    fn remove(&self) {}
}

fn main() {
    let mut config = WindowConfig::default();
    config.title = "Custom Shader Example".to_string();
    spottedcat::run::<CustomShaderScene>(config);
}
