use spottedcat::{
    Context, DrawOption, DrawOption3D, Image, Model, Pt, Spot, Text, WindowConfig, window_size,
};
use wasm_bindgen::prelude::*;

struct WasmDemo {
    happy_tree: Image,
    model: Model,
    rotation: f32,
    title: Text,
    subtitle: Text,
}

impl Spot for WasmDemo {
    fn initialize(ctx: &mut Context) -> Self {
        const HAPPY_TREE_BYTES: &[u8] = include_bytes!("../../../../assets/happy-tree.png");
        let img = image::load_from_memory(HAPPY_TREE_BYTES).unwrap();
        let happy_tree =
            Image::new(ctx, Pt::from(img.width() as f32), Pt::from(img.height() as f32), &img.to_rgba8())
                .expect("failed to create happy tree image");

        // Include font for WASM demo
        const FONT: &[u8] = include_bytes!("../../../../assets/DejaVuSans.ttf");
        let font_id = spottedcat::register_font(ctx, FONT.to_vec());

        let title = Text::new("SpottedCat WASM Demo", font_id)
            .with_font_size(Pt::from(24.0))
            .with_color([1.0, 1.0, 1.0, 1.0]);
        let subtitle = Text::new("3D Cube + 2D overlay", font_id)
            .with_font_size(Pt::from(14.0))
            .with_color([0.7, 0.9, 1.0, 1.0]);

        // Setup 3D scene
        spottedcat::set_ambient_light(ctx, [0.2, 0.2, 0.2, 1.0]);
        spottedcat::set_light(ctx, 0, [10.0, 10.0, 10.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
        spottedcat::set_camera(ctx, [3.6, 2.8, 6.2], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        spottedcat::set_camera_fovy(ctx, 32.7);

        let model = spottedcat::model::create_cube(ctx, 1.0).unwrap();

        Self {
            happy_tree,
            model,
            rotation: 0.0,
            title,
            subtitle,
        }
    }

    fn draw(&mut self, ctx: &mut Context, screen: spottedcat::Image) {
        let (_w, _h) = window_size(ctx);

        // 1. Draw 3D model
        let opts_3d = DrawOption3D::default()
            .with_position([0.0, 0.0, 0.0])
            .with_rotation([0.55, 0.75 + self.rotation * 0.45, 0.0]);
        screen.draw(ctx, &self.model, opts_3d);

        // 2. Draw UI directly to screen
        let title_opts = DrawOption::default().with_position([Pt::from(20.0), Pt::from(20.0)]);
        screen.draw(ctx, &self.title, title_opts);

        let tree_target_width = spottedcat::vw(ctx, 50.0).as_f32();
        let tree_scale = tree_target_width / self.happy_tree.width().as_f32();
        let tree_opts = DrawOption::default()
            .with_position([Pt::from(20.0), Pt::from(70.0)])
            .with_scale([tree_scale, tree_scale]);
        screen.draw(ctx, &self.happy_tree, tree_opts);

        let subtitle_y = 70.0 + tree_target_width + 20.0;
        let sub_text_opts =
            DrawOption::default().with_position([Pt::from(20.0), Pt::from(subtitle_y)]);
        screen.draw(ctx, &self.subtitle, sub_text_opts);
    }

    fn update(&mut self, ctx: &mut Context, dt: std::time::Duration) {
        self.rotation += dt.as_secs_f32() * 1.5;

        if spottedcat::key_pressed(ctx, spottedcat::Key::Space) {
            spottedcat::play_sine(ctx, 440.0, 0.3);
        }
    }

    fn remove(&mut self, _ctx: &mut Context) {}
}

#[wasm_bindgen]
pub fn run_demo() {
    console_error_panic_hook::set_once();

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    let config = {
        let mut config = WindowConfig::default();
        config.canvas_id = Some("spot-canvas".to_string());
        config
    };

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    let config = WindowConfig::default();

    spottedcat::run::<WasmDemo>(config);
}
