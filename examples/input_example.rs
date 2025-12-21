use spottedcat::{Context, Spot, Text, TextOptions, Key};

struct InputExample {
    x: f32,
    y: f32,
    speed: f32,
}

impl Spot for InputExample {
    fn initialize(_: Context) -> Self {
        Self {
            x: 200.0,
            y: 200.0,
            speed: 240.0,
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
        const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");

        let mut opts = TextOptions::new(spottedcat::load_font_from_bytes(FONT));
        opts.position = [spottedcat::Pt(20.0), spottedcat::Pt(40.0)];
        opts.font_size = spottedcat::Pt(24.0);
        opts.color = [1.0, 1.0, 1.0, 1.0];

        Text::new("Input Example (WASD / Arrow Keys to move, ESC to reset)").draw(context, opts);

        let mut opts = TextOptions::new(spottedcat::load_font_from_bytes(FONT));
        opts.position = [spottedcat::Pt(20.0), spottedcat::Pt(80.0)];
        opts.font_size = spottedcat::Pt(20.0);
        opts.color = [0.7, 0.9, 1.0, 1.0];

        Text::new(format!("Position: ({:.1}, {:.1})", self.x, self.y)).draw(context, opts);

        let mut opts = TextOptions::new(spottedcat::load_font_from_bytes(FONT));
        opts.position = [spottedcat::Pt(20.0), spottedcat::Pt(120.0)];
        opts.font_size = spottedcat::Pt(18.0);
        opts.color = [0.9, 0.9, 0.9, 1.0];

        Text::new("Tip: hold keys for continuous movement; press ESC to reset.").draw(context, opts);
    }

    fn remove(&self) {}
}

fn main() {
    spottedcat::run::<InputExample>(spottedcat::WindowConfig::default());
}
