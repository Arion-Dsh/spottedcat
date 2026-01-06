fn main() {
    use spottedcat::{Context, DrawOption, Pt, Spot, WindowConfig, run, Text, load_font_from_bytes};

    struct SimpleWrapTest {
        wrapped_text: Text,
    }

    impl Spot for SimpleWrapTest {
        fn initialize(_context: &mut Context) -> Self {
            const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
            let font_data = load_font_from_bytes(FONT);
            
            // Create text with very narrow width to force wrapping
            let wrapped_text = Text::new(
                "Hello world this should wrap",
                font_data
            )
                .with_font_size(Pt::from(32.0))
                .with_color([1.0, 1.0, 1.0, 1.0])
                .with_max_width(Pt::from(150.0)); // Very narrow

            Self { 
                wrapped_text,
            }
        }

        fn draw(&mut self, context: &mut Context) {
            let opts = DrawOption::default()
                .with_position([Pt::from(50.0), Pt::from(50.0)]);
            self.wrapped_text.draw(context, opts);
        }

        fn update(&mut self, _context: &mut Context, _dt: std::time::Duration) {}
        fn remove(&self) {}
    }

    run::<SimpleWrapTest>(WindowConfig::default());
}
