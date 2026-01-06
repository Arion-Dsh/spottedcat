fn main() {
    use spottedcat::{Context, DrawOption, Pt, Spot, WindowConfig, run, Text, load_font_from_bytes};

    struct SimpleTextTest {
        text: Text,
    }

    impl Spot for SimpleTextTest {
        fn initialize(_context: &mut Context) -> Self {
            const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
            let font_data = load_font_from_bytes(FONT);
            
            // Create simple text
            let text = Text::new("Clipped Text", font_data)
                .with_font_size(Pt::from(24.0))
                .with_color([1.0, 1.0, 1.0, 1.0]);

            Self { text }
        }

        fn draw(&mut self, context: &mut Context) {
            let opts = DrawOption::default()
                .with_position([Pt::from(50.0), Pt::from(50.0)]);
            self.text.draw(context, opts);
        }

        fn update(&mut self, _context: &mut Context, _dt: std::time::Duration) {}
        fn remove(&self) {}
    }

    run::<SimpleTextTest>(WindowConfig::default());
}
