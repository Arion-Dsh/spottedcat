use spottedcat::{Context, DrawOption, Key, Spot, Text};

struct InputExample {
    x: f32,
    y: f32,
    speed: f32,
    font_id: u32,
}

impl Spot for InputExample {
    fn initialize(_: &mut Context) -> Self {
        const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
        let font_id = spottedcat::register_font(FONT.to_vec());

        Self {
            x: 200.0,
            y: 200.0,
            speed: 240.0,
            font_id,
        }
    }

    fn update(&mut self, ctx: &mut Context, dt: std::time::Duration) {
        let dt = dt.as_secs_f32();

        if spottedcat::key_down(ctx, Key::W) || spottedcat::key_down(ctx, Key::Up) {
            self.y -= self.speed * dt;
        }
        if spottedcat::key_down(ctx, Key::S) || spottedcat::key_down(ctx, Key::Down) {
            self.y += self.speed * dt;
        }
        if spottedcat::key_down(ctx, Key::A) || spottedcat::key_down(ctx, Key::Left) {
            self.x -= self.speed * dt;
        }
        if spottedcat::key_down(ctx, Key::D) || spottedcat::key_down(ctx, Key::Right) {
            self.x += self.speed * dt;
        }

        if spottedcat::key_pressed(ctx, Key::Escape) {
            self.x = 200.0;
            self.y = 200.0;
        }
    }

    fn draw(&mut self, context: &mut Context) {
        let title_opts = DrawOption::default()
            .with_position([spottedcat::Pt::from(20.0), spottedcat::Pt::from(40.0)]);
        Text::new("Input Example (Use WASD or Arrow keys)", self.font_id)
            .with_font_size(spottedcat::Pt::from(24.0))
            .with_color([1.0, 1.0, 1.0, 1.0])
            .draw(context, title_opts);

        let keys_opts = DrawOption::default()
            .with_position([spottedcat::Pt::from(20.0), spottedcat::Pt::from(90.0)]);
        Text::new(
            format!("Position: ({:.1}, {:.1})", self.x, self.y),
            self.font_id,
        )
        .with_font_size(spottedcat::Pt::from(20.0))
        .with_color([0.7, 0.9, 1.0, 1.0])
        .draw(context, keys_opts);

        let mouse_opts = DrawOption::default()
            .with_position([spottedcat::Pt::from(20.0), spottedcat::Pt::from(160.0)]);
        Text::new(
            "Tip: hold keys for continuous movement; press ESC to reset.",
            self.font_id,
        )
        .with_font_size(spottedcat::Pt::from(18.0))
        .with_color([0.9, 0.9, 0.9, 1.0])
        .draw(context, mouse_opts);
    }

    fn remove(&self) {}
}

fn main() {
    spottedcat::run::<InputExample>(spottedcat::WindowConfig::default());
}
