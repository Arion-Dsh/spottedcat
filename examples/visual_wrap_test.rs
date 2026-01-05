fn main() {
    use spottedcat::{Context, DrawOption, Pt, Spot, WindowConfig, run, Text, load_font_from_bytes, Image};

    struct VisualWrapTest {
        wrapped_text: Text,
        background: Image,
    }

    impl Spot for VisualWrapTest {
        fn initialize(_context: &mut Context) -> Self {
            const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
            let font_data = load_font_from_bytes(FONT);
            
            // Create text with wrapping
            let wrapped_text = Text::new(
                "This is a very long sentence that should definitely wrap into multiple lines when displayed with a narrow width constraint. You should see this text break into several lines within the red box.",
                font_data
            )
                .with_font_size(Pt::from(20.0))
                .with_color([1.0, 1.0, 1.0, 1.0]) // White text
                .with_max_width(Pt::from(250.0)); // Force wrapping

            // Create a red background box to show the wrapping boundary
            let mut bg_data = vec![0u8; 300 * 200 * 4]; // 300x200 red background
            for i in (0..bg_data.len()).step_by(4) {
                bg_data[i] = 128;     // R (darker red)
                bg_data[i + 1] = 0;   // G  
                bg_data[i + 2] = 0;   // B
                bg_data[i + 3] = 100; // A (semi-transparent)
            }
            let background = Image::new_from_rgba8(Pt::from(300.0), Pt::from(200.0), &bg_data).unwrap();

            Self { 
                wrapped_text,
                background,
            }
        }

        fn draw(&mut self, context: &mut Context) {
            // Draw background box first
            let mut bg_opts = DrawOption::default();
            bg_opts.set_position([Pt::from(50.0), Pt::from(50.0)]);
            self.background.draw(context, bg_opts);

            // Draw wrapped text on top of background
            let mut text_opts = DrawOption::default();
            text_opts.set_position([Pt::from(75.0), Pt::from(75.0)]);
            self.wrapped_text.draw(context, text_opts);
        }

        fn update(&mut self, _context: &mut Context, _dt: std::time::Duration) {}
        fn remove(&self) {}
    }

    run::<VisualWrapTest>(WindowConfig::default());
}
