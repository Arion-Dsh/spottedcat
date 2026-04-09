use spottedcat::{Context, DrawOption, Image, ShaderOpts, Spot, WindowConfig};

struct ShaderScopeApp {
    image: Image,
    child_image: Image,
    shader_id: u32,
    time: f32,
}

impl Spot for ShaderScopeApp {
    fn initialize(ctx: &mut Context) -> Self {
        // Create a 64x64 logical blue image
        let rgba = vec![0, 0, 255, 255]
            .into_iter()
            .cycle()
            .take(64 * 64 * 4)
            .collect::<Vec<u8>>();
        let image = Image::new(
            ctx,
            spottedcat::Pt::from(64.0),
            spottedcat::Pt::from(64.0),
            &rgba,
        )
        .unwrap();

        // Create a 32x32 logical red image
        let child_rgba = vec![255, 0, 0, 255]
            .into_iter()
            .cycle()
            .take(32 * 32 * 4)
            .collect::<Vec<u8>>();
        let child_image = Image::new(
            ctx,
            spottedcat::Pt::from(32.0),
            spottedcat::Pt::from(32.0),
            &child_rgba,
        )
        .unwrap();

        // A shader that uses screen coordinates to make a visible wave
        // Note: engine uses hooks. global 'user_globals' is available. available vars: in, color.
        let shader_src = r#"
            fn user_fs_hook() {
                let time = user_globals[0].x;
                
                // Screen space wave effect
                // 'in' is the VsOut struct from image.wgsl
                let scan_line = sin(in.clip_pos.y * 0.1 - time * 5.0);
                let wave_x = sin(in.clip_pos.x * 0.05 + time * 2.0);
                
                // Mix in some green based on the wave
                let g_effect = (scan_line + wave_x) * 0.5 + 0.5; // 0.0 to 1.0
                
                // 'color' is the mutable output variable initialized with texture color * opacity
                color = vec4<f32>(color.r, g_effect, color.b, color.a);
            }
        "#;

        let shader_id = spottedcat::register_image_shader(ctx, shader_src);

        Self {
            image,
            child_image,
            shader_id,
            time: 0.0,
        }
    }

    fn draw(&mut self, ctx: &mut Context) {
        let opts = DrawOption::default()
            .with_position([spottedcat::Pt::from(100.0), spottedcat::Pt::from(200.0)])
            .with_scale([2.0, 2.0]);

        let mut shader_opts = ShaderOpts::default();
        shader_opts.set_vec4(0, [self.time, 0.0, 0.0, 0.0]);

        // Draw parent with shader scope
        self.image
            .with_shader_scope(ctx, self.shader_id, shader_opts, |ctx| {
                // Draw the parent itself (it needs to be drawn explicitly if we want it visible)
                self.image.draw(ctx, opts);

                // Draw child relative to parent
                let child_opts = DrawOption::default().with_position([
                    spottedcat::Pt::from(100.0),
                    spottedcat::Pt::from(200.0) + spottedcat::Pt::from(20.0),
                ]); // Slightly offset

                // This child should INHERIT the shader and the wave should be continuous
                self.child_image.draw(ctx, child_opts);
            });

        // Draw another instance WITHOUT scope to compare
        let ref_opts =
            opts.with_position([spottedcat::Pt::from(400.0), spottedcat::Pt::from(200.0)]);
        self.image.draw(ctx, ref_opts);
    }

    fn update(&mut self, _ctx: &mut Context, dt: std::time::Duration) {
        self.time += dt.as_secs_f32();
    }

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    spottedcat::run::<ShaderScopeApp>(WindowConfig::default());
}
