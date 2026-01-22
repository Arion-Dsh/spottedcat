fn main() {
    use spottedcat::{
        Context, DrawOption, Pt, Spot, Text, WindowConfig, load_font_from_bytes, run,
    };

    struct TextPerformanceSpot {
        texts: Vec<Text>,
    }

    impl Spot for TextPerformanceSpot {
        fn initialize(_context: &mut Context) -> Self {
            const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
            let font_data = load_font_from_bytes(FONT);
            let font_id = spottedcat::register_font(font_data);

            let mut texts = Vec::new();

            // Create multiple texts with different sizes and colors
            for i in 0..10 {
                let text = Text::new(&format!("Text Line {}", i), font_id)
                    .with_font_size(Pt::from(16.0 + i as f32 * 2.0))
                    .with_color([1.0, 1.0 - i as f32 * 0.1, 0.5, 1.0]);
                texts.push(text);
            }

            // Add some Chinese text for testing
            let chinese_text = Text::new("你好世界！测试中文渲染", font_id)
                .with_font_size(Pt::from(24.0))
                .with_color([0.0, 1.0, 1.0, 1.0]);
            texts.push(chinese_text);

            Self { texts }
        }

        fn draw(&mut self, context: &mut Context) {
            for (i, text) in self.texts.iter().enumerate() {
                let opts = DrawOption::default()
                    .with_position([Pt::from(50.0), Pt::from(50.0 + i as f32 * 40.0)]);
                text.draw(context, opts);
            }
        }

        fn update(&mut self, _context: &mut Context, _dt: std::time::Duration) {}

        fn remove(&self) {}
    }

    run::<TextPerformanceSpot>(WindowConfig::default());
}
