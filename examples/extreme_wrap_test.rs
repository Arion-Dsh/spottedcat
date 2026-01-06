fn main() {
    use spottedcat::{Context, DrawOption, Pt, Spot, WindowConfig, run, Text, load_font_from_bytes};

    struct ExtremeWrapTest {
        wrapped_text: Text,
    }

    impl Spot for ExtremeWrapTest {
        fn initialize(_context: &mut Context) -> Self {
            const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
            let font_data = load_font_from_bytes(FONT);
            
            // Create text with VERY narrow width to force obvious wrapping
            let wrapped_text = Text::new(
                "Each word should be on its own line now with this extremely narrow width constraint that forces wrapping after almost every single word",
                font_data
            )
                .with_font_size(Pt::from(28.0))
                .with_color([1.0, 1.0, 0.0, 1.0]) // Yellow text
                .with_max_width(Pt::from(100.0)); // Extremely narrow - should force wrapping

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

    run::<ExtremeWrapTest>(WindowConfig::default());
}
