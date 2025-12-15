fn main() {
    struct DrawSubDemo {
        canvas: spot::Image,
        sprite: spot::Image,
    }

    impl spot::Spot for DrawSubDemo {
        fn initialize(_context: spot::Context) -> Self {
            // Create a 400x400 canvas (blue background)
            let mut canvas_rgba = vec![0u8; 400 * 400 * 4];
            for i in 0..(400 * 400) {
                canvas_rgba[i * 4] = 50;      // R
                canvas_rgba[i * 4 + 1] = 50;  // G
                canvas_rgba[i * 4 + 2] = 200; // B
                canvas_rgba[i * 4 + 3] = 255; // A
            }
            let canvas = spot::Image::new_from_rgba8(400, 400, &canvas_rgba)
                .expect("failed to create canvas");

            // Create a 20x20 sprite (red square)
            let mut sprite_rgba = vec![0u8; 20 * 20 * 4];
            for i in 0..(20 * 20) {
                sprite_rgba[i * 4] = 255;     // R
                sprite_rgba[i * 4 + 1] = 80;  // G
                sprite_rgba[i * 4 + 2] = 80;  // B
                sprite_rgba[i * 4 + 3] = 255; // A
            }
            let sprite = spot::Image::new_from_rgba8(20, 20, &sprite_rgba)
                .expect("failed to create sprite");

            Self { canvas, sprite }
        }

        fn draw(&mut self, context: &mut spot::Context) {
            // Compose B onto A.
            // In sub-canvas coordinates, (0,0) is A's top-left corner.
            let mut sub_opts = spot::ImageDrawOptions::default();
            sub_opts.position = [380.0, 380.0];
            let sub_opt = spot::DrawOption { options: sub_opts };
            let sprite_drawable = spot::DrawAble::Image(self.sprite, spot::ImageDrawOptions::default());
            self.canvas
                .draw_sub(sprite_drawable, sub_opt)
                .expect("failed to draw sprite onto canvas");

            let font_data = spot::load_font_from_file("assets/DejaVuSans.ttf")
                .expect("failed to load font");
            let mut text_opts = spot::TextOptions::new(font_data);
            text_opts.font_size = 32.0;
            text_opts.color = [1.0, 1.0, 1.0, 1.0];
            let text_drawable = spot::DrawAble::Text("Hello".to_string(), text_opts);
            let mut text_draw_opts = spot::ImageDrawOptions::default();
            text_draw_opts.position = [20.0, 50.0];
            let text_draw_opt = spot::DrawOption {
                options: text_draw_opts,
            };
            self.canvas
                .draw_sub(text_drawable, text_draw_opt)
                .expect("failed to draw text onto canvas");

            // Draw the composited canvas to screen
            let canvas_screen_pos = [10.0, 10.0];
            let mut opts = spot::ImageDrawOptions::default();
            opts.position = canvas_screen_pos;
            self.canvas.draw(context, opts);

            // Draw the sprite in screen space relative to the canvas top-left.
            // This keeps the sprite offset/size fixed in screen pixels and does not scale
            // with the canvas draw size.
            let mut opts = spot::ImageDrawOptions::default();
            opts.position = [canvas_screen_pos[0] + 10.0, canvas_screen_pos[1] + 10.0];
            self.sprite.draw(context, opts);

            // Also draw the original sprite separately for comparison
            let mut opts = spot::ImageDrawOptions::default();
            opts.position = [550.0, 10.0];
            opts.scale = [5.0, 5.0];
            self.sprite.draw(context, opts);
        }

        fn update(&self, _event: spot::Event) {}

        fn remove(&self) {}
    }

    spot::run::<DrawSubDemo>();
}
