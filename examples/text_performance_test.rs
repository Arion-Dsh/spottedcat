mod example_font;

fn main() {
    use spottedcat::{Context, DrawOption, Pt, Spot, Text, WindowConfig, run};

    struct TextPerformanceSpot {
        texts: Vec<Text>,
    }

    impl Spot for TextPerformanceSpot {
        fn initialize(ctx: &mut Context) -> Self {
            let font_id = example_font::register(ctx);

            let mut texts = Vec::new();

            // Create multiple texts with different sizes and colors
            for i in 0..10 {
                let text = Text::new(format!("Text Line {}", i), font_id)
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

        fn draw(&mut self, ctx: &mut Context, screen: spottedcat::Image) {
            for (i, text) in self.texts.iter().enumerate() {
                let opts = DrawOption::default()
                    .with_position([Pt::from(50.0), Pt::from(50.0 + i as f32 * 40.0)]);
                screen.draw(ctx, text, opts);
            }
        }
    }

    run::<TextPerformanceSpot>(WindowConfig::default());
}
