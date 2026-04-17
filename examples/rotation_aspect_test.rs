use spottedcat::{Context, DrawOption, Image, Pt, Spot, Text, WindowConfig};

const DISC_SIZE: usize = 160;

struct RotationAspectTest {
    disc: Image,
    font_id: u32,
    angle: f32,
}

fn make_disc_rgba() -> Vec<u8> {
    let mut rgba = vec![0u8; DISC_SIZE * DISC_SIZE * 4];
    let center = (DISC_SIZE as f32 - 1.0) * 0.5;
    let radius = DISC_SIZE as f32 * 0.43;
    let ring_inner = radius - 4.0;

    for y in 0..DISC_SIZE {
        for x in 0..DISC_SIZE {
            let fx = x as f32 - center;
            let fy = y as f32 - center;
            let d = (fx * fx + fy * fy).sqrt();
            let idx = (y * DISC_SIZE + x) * 4;

            if d > radius {
                continue;
            }

            let on_ring = d >= ring_inner;
            let on_cross = fx.abs() < 3.0 || fy.abs() < 3.0;
            let on_diagonal = (fx - fy).abs() < 3.0;
            let color = if on_ring {
                [255, 255, 255, 255]
            } else if on_cross {
                [255, 78, 78, 255]
            } else if on_diagonal {
                [70, 220, 255, 255]
            } else {
                [40, 185, 120, 255]
            };

            rgba[idx..idx + 4].copy_from_slice(&color);
        }
    }

    rgba
}

fn top_left_for_center_rotation(center_x: f32, center_y: f32, size: f32, angle: f32) -> [Pt; 2] {
    let half = size * 0.5;
    let c = angle.cos();
    let s = angle.sin();
    let center_offset_x = c * half + s * half;
    let center_offset_y = c * half - s * half;

    [
        Pt::from(center_x - center_offset_x),
        Pt::from(center_y - center_offset_y),
    ]
}

impl Spot for RotationAspectTest {
    fn initialize(ctx: &mut Context) -> Self {
        let disc = Image::new(
            ctx,
            Pt::from(DISC_SIZE as f32),
            Pt::from(DISC_SIZE as f32),
            &make_disc_rgba(),
        )
        .expect("disc image should load");
        let font_id = spottedcat::register_font(ctx, include_bytes!("../assets/DejaVuSans.ttf").to_vec());

        Self {
            disc,
            font_id,
            angle: 0.0,
        }
    }

    fn update(&mut self, _ctx: &mut Context, dt: std::time::Duration) {
        self.angle += dt.as_secs_f32();
    }

    fn draw(&mut self, ctx: &mut Context, screen: Image) {
        let (w, h) = spottedcat::window_size(ctx);
        let sw = w.as_f32();
        let sh = h.as_f32();
        let disc = DISC_SIZE as f32;

        let title = Text::new("DrawOption::with_rotation aspect test", self.font_id)
            .with_font_size(Pt::from(20.0))
            .with_color([0.96, 0.98, 1.0, 1.0]);
        screen.draw(
            ctx,
            &title,
            DrawOption::default().with_position([Pt::from(18.0), Pt::from(22.0)]),
        );

        let hint = Text::new("Both discs should stay circular in this portrait window.", self.font_id)
            .with_font_size(Pt::from(14.0))
            .with_color([0.78, 0.84, 0.88, 1.0]);
        screen.draw(
            ctx,
            &hint,
            DrawOption::default().with_position([Pt::from(18.0), Pt::from(52.0)]),
        );

        let center_y = (sh * 0.42).max(92.0 + disc * 0.5);
        let left_center_x = sw * 0.5 - disc * 0.5 - 22.0;
        let right_center_x = sw * 0.5 + disc * 0.5 + 22.0;
        let left_pos = top_left_for_center_rotation(left_center_x, center_y, disc, 0.0);
        let right_pos = top_left_for_center_rotation(right_center_x, center_y, disc, self.angle);

        screen.draw(
            ctx,
            &self.disc,
            DrawOption::default().with_position(left_pos),
        );

        screen.draw(
            ctx,
            &self.disc,
            DrawOption::default()
                .with_position(right_pos)
                .with_rotation(self.angle),
        );

        let y = center_y - disc * 0.5;
        let left_x = left_center_x - disc * 0.5;
        let right_x = right_center_x - disc * 0.5;
        let labels_y = y + disc + 18.0;
        let reference = Text::new("0 rad", self.font_id)
            .with_font_size(Pt::from(15.0))
            .with_color([0.92, 0.94, 0.96, 1.0]);
        screen.draw(
            ctx,
            &reference,
            DrawOption::default().with_position([Pt::from(left_x + 48.0), Pt::from(labels_y)]),
        );

        let rotating = Text::new("rotating", self.font_id)
            .with_font_size(Pt::from(15.0))
            .with_color([0.92, 0.94, 0.96, 1.0]);
        screen.draw(
            ctx,
            &rotating,
            DrawOption::default().with_position([Pt::from(right_x + 36.0), Pt::from(labels_y)]),
        );
    }

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    spottedcat::run::<RotationAspectTest>(WindowConfig {
        title: "Rotation Aspect Test".to_string(),
        width: Pt::from(390.0),
        height: Pt::from(844.0),
        ..Default::default()
    });
}
