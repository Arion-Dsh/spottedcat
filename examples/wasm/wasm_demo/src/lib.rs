use spottedcat::{Context, DrawOption, Image, Pt, Spot, Text, WindowConfig};
use wasm_bindgen::prelude::*;

struct WasmDemo {
    image: Image,
    font_id: u32,
}

impl Spot for WasmDemo {
    fn initialize(_context: &mut Context) -> Self {
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

        let image = Image::new_from_rgba8(Pt::from(64.0), Pt::from(64.0), &rgba)
            .expect("failed to create test image");

        // Include font for WASM demo
        const FONT: &[u8] = include_bytes!("../../../../assets/DejaVuSans.ttf");
        let font_id = spottedcat::register_font(FONT.to_vec());

        Self { image, font_id }
    }

    fn draw(&mut self, context: &mut Context) {
        let opts = DrawOption::default()
            .with_position([Pt::from(40.0), Pt::from(100.0)])
            .with_scale([5.0, 5.0]);
        self.image.draw(context, opts);

        let text_opts = DrawOption::default().with_position([Pt::from(40.0), Pt::from(40.0)]);
        Text::new("SpottedCat WASM Demo", self.font_id)
            .with_font_size(Pt::from(32.0))
            .with_color([1.0, 1.0, 1.0, 1.0])
            .draw(context, text_opts);

        let sub_text_opts = DrawOption::default().with_position([Pt::from(40.0), Pt::from(80.0)]);
        Text::new("Click to play a tone!", self.font_id)
            .with_font_size(Pt::from(16.0))
            .with_color([0.7, 0.9, 1.0, 1.0])
            .draw(context, sub_text_opts);
    }

    fn update(&mut self, context: &mut Context, _dt: std::time::Duration) {
        if spottedcat::key_pressed(context, spottedcat::Key::Space) {
            spottedcat::play_sine(440.0, 0.3);
        }
    }

    fn remove(&self) {}
}

#[wasm_bindgen]
pub fn run_demo() {
    console_error_panic_hook::set_once();

    let mut config = WindowConfig::default();
    config.canvas_id = Some("spot-canvas".to_string());

    spottedcat::run::<WasmDemo>(config);
}
