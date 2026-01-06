fn main() {
    use spottedcat::{Context, DrawOption, Pt, Spot, WindowConfig, run, Text, load_font_from_bytes};

    struct DebugWrapTest {
        wrapped_text: Text,
        frame_count: u32,
    }

    impl Spot for DebugWrapTest {
        fn initialize(_context: &mut Context) -> Self {
            const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
            let font_data = load_font_from_bytes(FONT);
            
            // Create text with wrapping
            let wrapped_text = Text::new(
                "Hello world this should wrap into multiple lines",
                font_data
            )
                .with_font_size(Pt::from(32.0))
                .with_color([1.0, 1.0, 1.0, 1.0])
                .with_max_width(Pt::from(150.0));

            Self { 
                wrapped_text,
                frame_count: 0,
            }
        }

        fn draw(&mut self, context: &mut Context) {
            // Print debug info every 60 frames (about 1 second)
            if self.frame_count % 60 == 0 {
                println!("Frame {}: Drawing text", self.frame_count);
                
                // Test wrapping logic
                use ab_glyph::{Font as _, FontArc, PxScale};
                
                const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
                let font_data = load_font_from_bytes(FONT);
                let font = FontArc::try_from_vec(font_data).unwrap();
                let px_size = 32.0f32.max(1.0);
                let scale = PxScale::from(px_size);
                let scaled = font.as_scaled(scale);
                
                let lines = self.wrapped_text.get_wrapped_lines(&scaled);
                println!("  Number of lines: {}", lines.len());
                for (i, line) in lines.iter().enumerate() {
                    println!("  Line {}: '{}'", i + 1, line);
                }
            }
            
            let opts = DrawOption::default()
                .with_position([Pt::from(50.0), Pt::from(50.0)]);
            self.wrapped_text.draw(context, opts);
            
            self.frame_count += 1;
        }

        fn update(&mut self, _context: &mut Context, _dt: std::time::Duration) {}
        fn remove(&self) {}
    }

    run::<DebugWrapTest>(WindowConfig::default());
}
