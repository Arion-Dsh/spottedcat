use spottedcat::{DrawOption, Image};

fn main() {
    struct DrawSubDemo {
        canvas: spottedcat::Image,
        sprite: spottedcat::Image,
        font_data: Vec<u8>,
        rng_state: u32,
    }

    impl spottedcat::Spot for DrawSubDemo {
        fn initialize(_context: &mut spottedcat::Context) -> Self {
            // Create a 400x400 canvas (blue background)
            let mut canvas_rgba = vec![0u8; 400 * 400 * 4];
            for i in 0..(400 * 400) {
                canvas_rgba[i * 4] = 50;      // R
                canvas_rgba[i * 4 + 1] = 50;  // G
                canvas_rgba[i * 4 + 2] = 200; // B
                canvas_rgba[i * 4 + 3] = 255; // A
            }
            let canvas = spottedcat::Image::new_from_rgba8(
                spottedcat::Pt::from(400.0),
                spottedcat::Pt::from(400.0),
                &canvas_rgba,
            )
                .expect("failed to create canvas");

            // Create a 20x20 sprite (red square)
            let mut sprite_rgba = vec![0u8; 20 * 20 * 4];
            for i in 0..(20 * 20) {
                sprite_rgba[i * 4] = 255;     // R
                sprite_rgba[i * 4 + 1] = 80;  // G
                sprite_rgba[i * 4 + 2] = 80;  // B
                sprite_rgba[i * 4 + 3] = 255; // A
            }
            let sprite = spottedcat::Image::new_from_rgba8(
                spottedcat::Pt::from(20.0),
                spottedcat::Pt::from(20.0),
                &sprite_rgba,
            )
                .expect("failed to create sprite");

            let font_data = spottedcat::load_font_from_file("assets/DejaVuSans.ttf")
                .expect("failed to load font");

            Self {
                canvas,
                sprite,
                font_data,
                rng_state: 1,
            }
        }

        fn draw(&mut self, context: &mut spottedcat::Context) {
            self.canvas
                .clear([50.0 / 255.0, 50.0 / 255.0, 200.0 / 255.0, 1.0])
                .expect("failed to clear canvas");

            // Compose B onto A.
            // In sub-canvas coordinates, (0,0) is A's top-left corner.
            let mut sub_opts = spottedcat::ImageDrawOptions::default();
            sub_opts.position = [spottedcat::Pt::from(380.0), spottedcat::Pt::from(380.0)];
            let sub_opt = spottedcat::DrawOption::Image(sub_opts);
            let sprite_drawable = spottedcat::DrawAble::Image(self.sprite);
            self.canvas
                .draw_sub(context, sprite_drawable, sub_opt)
                .expect("failed to draw sprite onto canvas");

            self.rng_state = self
                .rng_state
                .wrapping_mul(1664525)
                .wrapping_add(1013904223);
            let r = (self.rng_state % 11) as u32;
            if r > 8 {
                let mut text_opts = spottedcat::TextOptions::new(self.font_data.clone());
                text_opts.font_size = spottedcat::Pt::from(32.0);
                text_opts.color = [1.0, 1.0, 1.0, 1.0];
                let mut text_draw_opts = spottedcat::ImageDrawOptions::default();
                text_draw_opts.position = [spottedcat::Pt::from(20.0), spottedcat::Pt::from(50.0)];

                self.canvas
                    .draw_sub(
                        context,
                        spottedcat::DrawAble::Text(spottedcat::Text::new(format!("Hello ({})", r))),
                        spottedcat::DrawOption::Text(text_opts),
                    )
                    .expect("failed to draw text onto canvas");
            }

            // Draw the composited canvas to screen
            let canvas_screen_pos = [spottedcat::Pt::from(10.0), spottedcat::Pt::from(10.0)];
            let mut opts = spottedcat::ImageDrawOptions::default();
            opts.position = canvas_screen_pos;
            self.canvas.draw(context, opts);

            // Draw the sprite in screen space relative to the canvas top-left.
            // This keeps the sprite offset/size fixed in screen pixels and does not scale
            // with the canvas draw size.
            let mut opts = spottedcat::ImageDrawOptions::default();
            opts.position = [
                spottedcat::Pt::from(canvas_screen_pos[0].as_f32() + 10.0),
                spottedcat::Pt::from(canvas_screen_pos[1].as_f32() + 10.0),
            ];
            self.sprite.draw(context, opts);


            // // Also draw the original sprite separately for comparison
            let opts = spottedcat::ImageDrawOptions::default();
            // opts.position = [spottedcat::Pt(550.0), spottedcat::Pt(10.0)];
            // opts.scale = [5.0, 5.0];
            self.sprite.draw(context, opts);

            self.canvas
                    .draw_sub(context, spottedcat::DrawAble::Image(self.sprite), spottedcat::DrawOption::Image(opts))
                    .expect("failed to draw sprite onto canvas");
        }

        fn update(&mut self, _context: &mut spottedcat::Context, _dt: std::time::Duration) {}

        fn remove(&self) {}
    }

    spottedcat::run::<DrawSubDemo>(spottedcat::WindowConfig::default());
}
