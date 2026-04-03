use spottedcat::{Context, DrawOption, Image, Pt, ShaderOpts, Spot, Text, WindowConfig};

struct FlipTest {
    image: Image,
    font_id: u32,
    text_obj: Text,
    time: f32,
    yellow_shader_id: u32,
}

impl Spot for FlipTest {
    fn initialize(ctx: &mut Context) -> Self {
        let img_raw = vec![255u8; 64 * 64 * 4];
        let image =
            spottedcat::image::create(ctx, Pt::from(64.0), Pt::from(64.0), &img_raw).unwrap();

        let font_data = include_bytes!("../assets/DejaVuSans.ttf");
        let font_id = spottedcat::register_font(ctx, font_data.to_vec());
        let text_obj = Text::new("Flipped!", font_id)
            .with_font_size(Pt::from(16.0))
            .with_color([1.0, 1.0, 1.0, 1.0]);

        let fill_shader_src = r#"
            fn user_fs_hook() {
                let fill_color = user_globals[0];
                color = vec4<f32>(fill_color.rgb, color.a * fill_color.a);
            }
        "#;
        let yellow_shader_id = spottedcat::register_image_shader(ctx, fill_shader_src);

        Self {
            image,
            font_id,
            text_obj,
            time: 0.0,
            yellow_shader_id,
        }
    }

    fn update(&mut self, _ctx: &mut Context, dt: std::time::Duration) {
        self.time += dt.as_secs_f32();
    }

    fn draw(&mut self, ctx: &mut Context) {
        let (sw, sh) = spottedcat::window_size(ctx);
        let fsw = sw.as_f32();
        let fsh = sh.as_f32();

        // 1. Label
        let mut t_instr = self.text_obj.clone().with_font_size(Pt::from(20.0));
        t_instr.set_content("Check: 4 Trees (Red 100%, Green 80%, Blue 60%, Yellow 40%)");
        spottedcat::text::draw(
            ctx,
            &t_instr,
            DrawOption::default().with_position([Pt::from(10.0), Pt::from(20.0)]),
        );

        // Use a small scale
        let s = 0.3;

        let draw_item = |ctx: &mut Context,
                         x: f32,
                         y: f32,
                         sx: f32,
                         sy: f32,
                         color: [f32; 4],
                         alpha: f32,
                         shader_alpha: f32,
                         label: &str| {
            let opts = DrawOption::default()
                .with_position([Pt::from(x), Pt::from(y)])
                .with_scale([sx * s, sy * s])
                .with_opacity(alpha);

            let mut shader_opts = ShaderOpts::default().with_opacity(shader_alpha);
            shader_opts.set_vec4(0, color);

            spottedcat::image::draw_with_shader(ctx, self.image, 1, opts, shader_opts);

            let mut t = self.text_obj.clone();
            t.set_content(label);
            spottedcat::text::draw(
                ctx,
                &t,
                DrawOption::default().with_position([Pt::from(x - 20.0), Pt::from(y + 10.0)]),
            );
        };

        // Extreme spacing: corners and center
        // 1. Red - Normal - Top Left
        draw_item(
            ctx,
            100.0,
            100.0,
            1.0,
            1.0,
            [1.0, 0.0, 0.0, 1.0],
            1.0,
            1.0,
            "1.Red (Opaque)",
        );

        // 2. Green - Flip H - Top Right
        // 0.8 * 0.5 = 0.4
        draw_item(
            ctx,
            fsw - 100.0,
            100.0,
            -1.0,
            1.0,
            [0.0, 1.0, 0.0, 1.0],
            0.8,
            0.5,
            "2.Green (0.8*0.5=0.4)",
        );

        // 3. Blue - Flip V - Bottom Left
        // 0.5 * 1.0 = 0.5
        draw_item(
            ctx,
            100.0,
            fsh - 100.0,
            1.0,
            -1.0,
            [0.0, 0.0, 1.0, 1.0],
            0.5,
            1.0,
            "3.Blue (0.5*1.0=0.5)",
        );

        // 4. Yellow - Both - Bottom Right
        // Solid Yellow Fill via Custom Hook, Opacity 1.0
        let mut yellow_opts = ShaderOpts::default().with_opacity(1.0);
        yellow_opts.set_vec4(0, [1.0, 1.0, 0.0, 1.0]); // Yellow in Slot 0

        let yellow_draw = |ctx: &mut Context, x: f32, y: f32| {
            let opts = DrawOption::default()
                .with_position([Pt::from(x), Pt::from(y)])
                .with_scale([-s, -s]);

            spottedcat::image::draw_with_shader(
                ctx,
                self.image,
                self.yellow_shader_id,
                opts,
                yellow_opts,
            );
        };
        yellow_draw(ctx, fsw - 100.0, fsh - 100.0);

        let mut t_y = self.text_obj.clone();
        t_y.set_content("4.Solid Yellow (User Hook)");
        spottedcat::text::draw(
            ctx,
            &t_y,
            DrawOption::default().with_position([Pt::from(fsw - 140.0), Pt::from(fsh - 90.0)]),
        );

        // 5. Center - Solid FILLED (via Custom Hook)
        let move_y = (self.time.sin() * 100.0) + (fsh * 0.5);
        let mut fill_opts = ShaderOpts::default();
        fill_opts.set_vec4(0, [1.0, 0.5, 0.0, 1.0]); // Orange Fill

        spottedcat::image::draw_with_shader(
            ctx,
            self.image,
            self.yellow_shader_id, // User-registered Fill Shader
            DrawOption::default()
                .with_position([Pt::from(fsw * 0.5), Pt::from(move_y)])
                .with_scale([0.5, 0.5]),
            fill_opts,
        );

        let mut t_c = self.text_obj.clone();
        t_c.set_content("5.Solid Fill (Orange)");
        spottedcat::text::draw(
            ctx,
            &t_c,
            DrawOption::default()
                .with_position([Pt::from(fsw * 0.5 - 20.0), Pt::from(move_y + 10.0)]),
        );
    }

    fn remove(&mut self, ctx: &mut Context) {
        spottedcat::unregister_font(ctx, self.font_id);
    }
}

fn main() {
    let config = WindowConfig {
        title: "Flip Test (Final Diagnose)".to_string(),
        width: Pt::from(1000.0),
        height: Pt::from(800.0),
        ..Default::default()
    };
    spottedcat::run::<FlipTest>(config);
}
