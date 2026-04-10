use spottedcat::{Context, DrawOption, Image, Pt, Spot, Text, WindowConfig};

struct HappyTreeDesktop {
    image: Image,
    font_id: u32,
}

impl Spot for HappyTreeDesktop {
    fn initialize(ctx: &mut Context) -> Self {
        const HAPPY_TREE_BYTES: &[u8] = include_bytes!("../assets/happy-tree.png");
        const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");

        let img = image::load_from_memory(HAPPY_TREE_BYTES).expect("happy tree should decode");
        let image =
            spottedcat::utils::image::from_image(ctx, &img).expect("happy tree should load");
        let font_id = spottedcat::register_font(ctx, FONT.to_vec());

        Self { image, font_id }
    }

    fn update(&mut self, _ctx: &mut Context, _dt: std::time::Duration) {}

    fn draw(&mut self, ctx: &mut Context, screen: spottedcat::Image) {
        let (w, h) = spottedcat::window_size(ctx);
        let image_x = (w - self.image.width()) / 2.0;
        let image_y = (h - self.image.height()) / 2.0;

        screen.draw(
            ctx,
            &self.image,
            DrawOption::default().with_position([image_x, image_y]),
        );

        let overlay = Text::new(
            format!(
                "happy-tree\nscale_factor: {:.2}\ndefault size: {} x {} pt",
                spottedcat::scale_factor(ctx),
                self.image.width(),
                self.image.height()
            ),
            self.font_id,
        )
        .with_font_size(Pt::from(20.0))
        .with_color([0.95, 0.97, 1.0, 1.0]);

        screen.draw(
            ctx,
            &overlay,
            DrawOption::default().with_position([Pt::from(24.0), Pt::from(24.0)]),
        );
    }

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    spottedcat::run::<HappyTreeDesktop>(WindowConfig {
        title: "Happy Tree Desktop".to_string(),
        width: Pt::from(960.0),
        height: Pt::from(720.0),
        ..Default::default()
    });
}
