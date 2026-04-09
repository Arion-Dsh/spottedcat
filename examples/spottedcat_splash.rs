use spottedcat::{Context, DrawOption, Pt, Spot, SpottedcatSplash, Text, WindowConfig, run};

struct GameplayScene {
    font_id: u32,
}

impl Spot for GameplayScene {
    fn initialize(ctx: &mut Context) -> Self {
        const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
        let font_id = spottedcat::register_font(ctx, FONT.to_vec());
        Self { font_id }
    }

    fn update(&mut self, _ctx: &mut Context, _dt: std::time::Duration) {}

    fn draw(&mut self, ctx: &mut Context) {
        let (window_w, window_h) = spottedcat::window_size(ctx);

        let title = Text::new("Main Scene", self.font_id)
            .with_font_size(Pt::from(34.0))
            .with_color([0.95, 0.93, 0.88, 1.0]);
        let title_width = spottedcat::text::measure(ctx, &title).0;
        spottedcat::text::draw(
            ctx,
            &title,
            DrawOption::default().with_position([
                Pt::from((window_w.as_f32() - title_width) * 0.5),
                Pt::from(window_h.as_f32() * 0.36),
            ]),
        );

        let hint = Text::new(
            "Use `run::<SpottedcatSplash<YourScene>>()` to show the intro first.",
            self.font_id,
        )
        .with_font_size(Pt::from(18.0))
        .with_color([0.54, 0.9, 0.84, 1.0]);
        let hint_width = spottedcat::text::measure(ctx, &hint).0;
        spottedcat::text::draw(
            ctx,
            &hint,
            DrawOption::default().with_position([
                Pt::from((window_w.as_f32() - hint_width) * 0.5),
                Pt::from(window_h.as_f32() * 0.5),
            ]),
        );
    }

    fn remove(&mut self, ctx: &mut Context) {
        spottedcat::unregister_font(ctx, self.font_id);
    }
}

fn main() {
    run::<SpottedcatSplash<GameplayScene>>(WindowConfig {
        title: "Rusty-spotted cat".to_string(),
        width: Pt::from(540.0),
        height: Pt::from(960.0),
        ..Default::default()
    });
}
