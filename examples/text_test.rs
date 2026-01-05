fn main() {
    use spottedcat::{Context, DrawOption, Pt, Spot, WindowConfig, run, Text, load_font_from_bytes};

    struct TextSpot {
        text: Text,
    }

    impl Spot for TextSpot {
        fn initialize(_context: &mut Context) -> Self {
            const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
            let font_data = load_font_from_bytes(FONT);
            
            let text = Text::new("Hello, World!", font_data)
                .with_font_size(Pt::from(32.0))
                .with_color([1.0, 1.0, 1.0, 1.0]);

            Self { text }
        }

        fn draw(&mut self, context: &mut Context) {
            let mut opts = DrawOption::default();
            opts.set_position([Pt::from(100.0), Pt::from(100.0)]);
            self.text.draw(context, opts);
        }

        fn update(&mut self, _context: &mut Context, _dt: std::time::Duration) {
        }

        fn remove(&self) {}
    }

    run::<TextSpot>(WindowConfig::default());
}
