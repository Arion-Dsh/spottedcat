use spottedcat::{Context, MouseButton, Pt, Spot, Text, DrawOption};

struct MouseClickExample {
    last_click: Option<(Pt, Pt)>,
}

impl Spot for MouseClickExample {
    fn initialize(_: &mut Context) -> Self {
        Self { last_click: None }
    }

    fn update(&mut self, ctx: &mut Context, _dt: std::time::Duration) {
        if let Some((x, y)) = spottedcat::mouse_button_pressed_position(ctx, MouseButton::Left) {
            self.last_click = Some((x, y));
        }
    }

    fn draw(&mut self, context: &mut Context) {
        const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
        let font_data = spottedcat::load_font_from_bytes(FONT);

        let mut title_opts = DrawOption::new();
        title_opts.position = [spottedcat::Pt::from(20.0), spottedcat::Pt::from(40.0)];
        Text::new(
            "Mouse Click Example (Left click to record position)",
            font_data.clone(),
        )
        .with_font_size(spottedcat::Pt::from(24.0))
        .with_color([1.0, 1.0, 1.0, 1.0])
        .draw(context, title_opts);

        let mut pos_opts = DrawOption::new();
        pos_opts.position = [spottedcat::Pt::from(20.0), spottedcat::Pt::from(90.0)];

        let text = match self.last_click {
            Some((x, y)) => format!("Last left click: ({:.1}, {:.1})", x.as_f32(), y.as_f32()),
            None => "Last left click: (none)".to_string(),
        };

        Text::new(text, font_data)
            .with_font_size(spottedcat::Pt::from(20.0))
            .with_color([0.7, 0.9, 1.0, 1.0])
            .draw(context, pos_opts);
    }

    fn remove(&self) {}
}

fn main() {
    spottedcat::run::<MouseClickExample>(spottedcat::WindowConfig::default());
}
