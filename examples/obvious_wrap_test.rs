fn main() {
    use spottedcat::{Context, DrawOption, Pt, Spot, WindowConfig, run, Text, load_font_from_bytes};

    struct ObviousWrapTest {
        wrapped_text: Text,
        no_wrap_text: Text,
    }

    impl Spot for ObviousWrapTest {
        fn initialize(_context: &mut Context) -> Self {
            const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
            let font_data = load_font_from_bytes(FONT);
            
            // Create text with wrapping - very narrow width
            let wrapped_text = Text::new(
                "This is a very long sentence that should definitely wrap into multiple lines when displayed with a narrow width constraint.",
                font_data.clone()
            )
                .with_font_size(Pt::from(24.0))
                .with_color([1.0, 0.0, 1.0, 1.0]) // Magenta
                .with_max_width(Pt::from(200.0)); // Force wrapping

            // Create same text without wrapping for comparison
            let no_wrap_text = Text::new(
                "This is a very long sentence that should definitely wrap into multiple lines when displayed with a narrow width constraint.",
                font_data
            )
                .with_font_size(Pt::from(24.0))
                .with_color([0.0, 1.0, 1.0, 1.0]); // Cyan

            Self { 
                wrapped_text,
                no_wrap_text,
            }
        }

        fn draw(&mut self, context: &mut Context) {
            // Draw wrapped text at top
            let wrap_opts = DrawOption::default()
                .with_position([Pt::from(50.0), Pt::from(50.0)]);
            self.wrapped_text.draw(context, wrap_opts);

            // Draw non-wrapped text below for comparison
            let no_wrap_opts = DrawOption::default()
                .with_position([Pt::from(50.0), Pt::from(200.0)]);
            self.no_wrap_text.draw(context, no_wrap_opts);
        }

        fn update(&mut self, _context: &mut Context, _dt: std::time::Duration) {}
        fn remove(&self) {}
    }

    run::<ObviousWrapTest>(WindowConfig::default());
}
