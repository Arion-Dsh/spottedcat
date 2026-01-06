fn main() {
    use spottedcat::{Context, DrawOption, Pt, Spot, WindowConfig, run, Text, load_font_from_bytes, Image, Key};

    struct InteractiveTextSpot {
        text: Text,
        red_square: Image,
        square_visible: bool,
    }

    impl Spot for InteractiveTextSpot {
        fn initialize(_context: &mut Context) -> Self {
            const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
            let font_data = load_font_from_bytes(FONT);
            
            // Create green text with wrapping
            let text = Text::new("GREEN TEXT WITH WRAPPING - This is a longer text that demonstrates the new text wrapping feature. Press W to toggle wrapping.", font_data)
                .with_font_size(Pt::from(32.0))
                .with_color([0.0, 1.0, 0.0, 1.0]) // Green color
                .with_max_width(Pt::from(250.0)); // Enable wrapping

            // Create red square background
            let mut red_data = vec![0u8; 200 * 200 * 4]; // 200x200 red square
            for i in (0..red_data.len()).step_by(4) {
                red_data[i] = 255;     // R
                red_data[i + 1] = 0;   // G  
                red_data[i + 2] = 0;   // B
                red_data[i + 3] = 255; // A
            }
            let red_square = Image::new_from_rgba8(Pt::from(200.0), Pt::from(200.0), &red_data).unwrap();

            Self { 
                text, 
                red_square,
                square_visible: true,
            }
        }

        fn draw(&mut self, context: &mut Context) {
            // Draw red square if visible
            if self.square_visible {
                let square_opts = DrawOption::default()
                    .with_position([Pt::from(300.0), Pt::from(200.0)]);
                
                // Draw the red square first
                self.red_square.draw(context, square_opts);
                
                // Then draw green text clipped to the red square bounds
                let text_opts = DrawOption::default()
                    .with_position([Pt::from(50.0), Pt::from(80.0)]); // Relative to square position

                self.red_square.with_clip_scope(context, square_opts, |context| {
                    self.text.clone().draw(context, text_opts);
                });
            }
        }

        fn update(&mut self, context: &mut Context, _dt: std::time::Duration) {
            // Handle keyboard input
            if spottedcat::key_pressed(context, Key::W) {
                self.square_visible = false;
                println!("Square hidden (W key pressed)");
            }
            if spottedcat::key_pressed(context, Key::Q) {
                self.square_visible = true;
                println!("Square shown (Q key pressed)");
            }
        }

        fn remove(&self) {}
    }

    run::<InteractiveTextSpot>(WindowConfig::default());
}
