fn main() {
    use spottedcat::{Context, DrawOption, Pt, Spot, WindowConfig, run, Text, load_font_from_bytes, Image};

    struct SimpleTextSpot {
        text: Text,
        background: Image,
    }

    impl Spot for SimpleTextSpot {
        fn initialize(_context: &mut Context) -> Self {
            const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
            let font_data = load_font_from_bytes(FONT);
            
            // Create white text
            let text = Text::new("TEST TEXT", font_data)
                .with_font_size(Pt::from(48.0))
                .with_color([1.0, 1.0, 1.0, 1.0]);

            // Create a simple colored background for reference
            let bg_data = vec![255u8; 100 * 100 * 4]; // White background
            let background = Image::new_from_rgba8(Pt::from(100.0), Pt::from(100.0), &bg_data).unwrap();

            Self { text, background }
        }

        fn draw(&mut self, context: &mut Context) {
            // Draw background first
            let opts = DrawOption::default()
                .with_position([Pt::from(50.0), Pt::from(50.0)]);
            self.background.draw(context, opts);

            // Draw text
            let opts = DrawOption::default()
                .with_position([Pt::from(100.0), Pt::from(100.0)]);
            self.text.draw(context, opts);
        }

        fn update(&mut self, _context: &mut Context, _dt: std::time::Duration) {
        }

        fn remove(&self) {}
    }

    run::<SimpleTextSpot>(WindowConfig::default());
}
