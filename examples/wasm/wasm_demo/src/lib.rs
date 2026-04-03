use spottedcat::{
    Context, DrawOption, DrawOption3D, Image, Model, Pt, Spot, Text, WindowConfig, window_size,
};
use wasm_bindgen::prelude::*;

struct WasmDemo {
    image: Image,
    model: Model,
    rotation: f32,
    title: Text,
    subtitle: Text,
}

impl Spot for WasmDemo {
    fn initialize(ctx: &mut Context) -> Self {
        let mut rgba = vec![0u8; 64 * 64 * 4];
        for y in 0..64u32 {
            for x in 0..64u32 {
                let i = ((y * 64 + x) * 4) as usize;
                let on = ((x / 8 + y / 8) % 2) == 0;
                rgba[i] = if on { 255 } else { 30 };
                rgba[i + 1] = if on { 80 } else { 200 };
                rgba[i + 2] = if on { 80 } else { 255 };
                rgba[i + 3] = 255;
            }
        }

        let image = spottedcat::image::create(ctx, Pt::from(64.0), Pt::from(64.0), &rgba)
            .expect("failed to create test image");

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
            image,
            model,
            rotation: 0.0,
            title,
            subtitle,
        }
    }

    fn draw(&mut self, ctx: &mut Context) {
        let (w, h) = window_size(ctx);
        let image_scale = if w.as_f32() < 480.0 { 2.0 } else { 3.0 };
        let image_size = 64.0 * image_scale;
        let center_x = w.as_f32() * 0.5;
        let center_y = h.as_f32() * 0.56;
        let image_x = center_x - image_size * 0.5;
        let image_y = center_y - image_size * 0.5;
        let text_x = center_x - 96.0;
        let title_y = center_y - image_size * 0.5 - 48.0;
        let subtitle_y = title_y + 28.0;

        // Draw 3D model
        let opts_3d = DrawOption3D::default()
            .with_position([0.0, 0.0, 0.0])
            .with_rotation([0.55, 0.75 + self.rotation * 0.45, 0.0]);
        spottedcat::model::draw(ctx, &self.model, opts_3d);

        let opts = DrawOption::default()
            .with_position([Pt::from(image_x), Pt::from(image_y)])
            .with_scale([image_scale, image_scale]);
        spottedcat::image::draw(ctx, self.image, opts);

        let text_opts =
            DrawOption::default().with_position([Pt::from(text_x), Pt::from(title_y)]);
        spottedcat::text::draw(ctx, &self.title, text_opts);

        let sub_text_opts =
            DrawOption::default().with_position([Pt::from(text_x), Pt::from(subtitle_y)]);
        spottedcat::text::draw(ctx, &self.subtitle, sub_text_opts);
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
