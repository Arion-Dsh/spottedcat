use spottedcat::{Context, DrawOption, Image, Pt, Spot, WindowConfig};
use wasm_bindgen::prelude::*;

struct WasmDemo {
    image: Image,
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

        Self { image }
    }

    fn draw(&mut self, context: &mut Context) {
        let mut opts = DrawOption::default();
        opts.set_position([Pt::from(40.0), Pt::from(40.0)]);
        opts.set_scale([5.0, 5.0]);
        self.image.draw(context, opts);
    }

    fn update(&mut self, _context: &mut Context, _dt: std::time::Duration) {}

    fn remove(&self) {}
}

#[wasm_bindgen]
pub fn run_demo() {
    console_error_panic_hook::set_once();

    let mut config = WindowConfig::default();
    config.canvas_id = Some("spot-canvas".to_string());

    spottedcat::run::<WasmDemo>(config);
}
