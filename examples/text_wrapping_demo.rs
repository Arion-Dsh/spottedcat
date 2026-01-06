fn main() {
    use spottedcat::{Context, DrawOption, Pt, Spot, WindowConfig, run, Text, load_font_from_bytes};

    struct WrappedTextSpot {
        wrapped_text: Text,
        normal_text: Text,
    }

    impl Spot for WrappedTextSpot {
        fn initialize(_context: &mut Context) -> Self {
            const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
            let font_data = load_font_from_bytes(FONT);
            
            // Create wrapped text with max width constraint
            let wrapped_text = Text::new(
                "This is a long text that should wrap when it exceeds the maximum width constraint. The text wrapping feature allows you to create multi-line text displays.",
                font_data.clone()
            )
                .with_font_size(Pt::from(24.0))
                .with_color([0.0, 1.0, 0.0, 1.0]) // Green color
                .with_max_width(Pt::from(300.0)); // Wrap at 300 pixels

            // Create normal text without wrapping for comparison
            let normal_text = Text::new(
                "This is a long text that should wrap when it exceeds the maximum width constraint.",
                font_data
            )
                .with_font_size(Pt::from(24.0))
                .with_color([1.0, 1.0, 0.0, 1.0]); // Yellow color

            Self { 
                wrapped_text,
                normal_text,
            }
        }

        fn draw(&mut self, context: &mut Context) {
            // Draw wrapped text
            let wrapped_opts = DrawOption::default()
                .with_position([Pt::from(50.0), Pt::from(50.0)]);
            self.wrapped_text.draw(context, wrapped_opts);

            // Draw normal text for comparison
            let normal_opts = DrawOption::default()
                .with_position([Pt::from(50.0), Pt::from(200.0)]);
            self.normal_text.draw(context, normal_opts);
        }

        fn update(&mut self, _context: &mut Context, _dt: std::time::Duration) {}

        fn remove(&self) {}
    }

    run::<WrappedTextSpot>(WindowConfig::default());
}
