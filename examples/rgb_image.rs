use spottedcat::{Context, DrawOption, Image, Pt, Spot, Text, WindowConfig};

const WIDTH: usize = 300;
const HEIGHT: usize = 120;

struct RgbImageExample {
    image: Image,
    font_id: u32,
}

fn make_rgb_bars() -> Vec<u8> {
    let mut rgba = vec![0u8; WIDTH * HEIGHT * 4];

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let offset = (y * WIDTH + x) * 4;
            let color = if x < WIDTH / 3 {
                [255, 0, 0, 255]
            } else if x < (WIDTH * 2) / 3 {
                [0, 255, 0, 255]
            } else {
                [0, 0, 255, 255]
            };

            rgba[offset..offset + 4].copy_from_slice(&color);
        }
    }

    rgba
}

impl Spot for RgbImageExample {
    fn initialize(ctx: &mut Context) -> Self {
        const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");

        let rgba = make_rgb_bars();
        let image = Image::new(ctx, Pt::from(WIDTH as f32), Pt::from(HEIGHT as f32), &rgba)
            .expect("rgb image should load");
        let font_id = spottedcat::register_font(ctx, FONT.to_vec());

        Self { image, font_id }
    }

    fn update(&mut self, _ctx: &mut Context, _dt: std::time::Duration) {}

    fn draw(&mut self, ctx: &mut Context, screen: Image) {
        let (w, h) = spottedcat::window_size(ctx);
        let image_x = (w - self.image.width()) / 2.0;
        let image_y = (h - self.image.height()) / 2.0;

        screen.draw(
            ctx,
            &self.image,
            DrawOption::default().with_position([image_x, image_y]),
        );

        let title = Text::new(
            "RGB image check: left=red, middle=green, right=blue",
            self.font_id,
        )
        .with_font_size(Pt::from(22.0))
        .with_color([0.95, 0.97, 1.0, 1.0]);

        screen.draw(
            ctx,
            &title,
            DrawOption::default().with_position([Pt::from(24.0), Pt::from(24.0)]),
        );
    }

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    spottedcat::run::<RgbImageExample>(WindowConfig {
        title: "RGB Image Example".to_string(),
        width: Pt::from(960.0),
        height: Pt::from(720.0),
        ..Default::default()
    });
}
