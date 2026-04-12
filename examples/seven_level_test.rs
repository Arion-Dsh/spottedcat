use spottedcat::{Context, DrawOption, Image, Pt, Spot, WindowConfig, run};

struct SevenLevelNestTestSpot {
    images: Vec<Image>,
    white_image: Image,
    font_id: u32,
}

impl Spot for SevenLevelNestTestSpot {
    fn initialize(ctx: &mut Context) -> Self {
        let mut images = Vec::new();

        // Level 1-7 - Render targets
        for _ in 0..6 {
            images.push(
                spottedcat::Texture::new_render_target(ctx, Pt::from(200.0), Pt::from(200.0))
                    .view(),
            );
        }
        // Final level at half size
        images.push(
            spottedcat::Texture::new_render_target(ctx, Pt::from(100.0), Pt::from(100.0)).view(),
        );

        // Load default font
        const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
        let font_id = spottedcat::register_font(ctx, FONT.to_vec());

        let white_image =
            spottedcat::Image::new(ctx, Pt::from(1.0), Pt::from(1.0), &[255, 255, 255, 255])
                .expect("Failed to create white image");

        Self {
            images,
            white_image,
            font_id,
        }
    }

    fn draw(&mut self, ctx: &mut Context, screen: spottedcat::Image) {
        // Define colors for each level
        let colors = [
            [1.0, 0.0, 0.0, 1.0], // Red
            [0.0, 0.0, 1.0, 1.0], // Blue
            [0.0, 1.0, 0.0, 1.0], // Green
            [1.0, 1.0, 0.0, 1.0], // Yellow
            [0.0, 1.0, 1.0, 1.0], // Cyan
            [1.0, 0.0, 1.0, 1.0], // Magenta
            [1.0, 1.0, 1.0, 1.0], // White (Level 7)
        ];

        // 1. Initialize render targets with their respective colors
        for i in 0..7 {
            let target = self.images[i];
            target.draw_with_shader(
                ctx,
                &self.white_image,
                1, // Built-in tint shader
                DrawOption::default()
                    .with_scale([target.width().as_f32(), target.height().as_f32()]),
                spottedcat::ShaderOpts::default().with_color(colors[i]),
            );
        }

        // 2. Build the recursive stack (from bottom to top)
        // Level 7 on Level 6
        self.images[5].draw(
            ctx,
            &self.images[6],
            DrawOption::default().with_position([Pt::from(10.0), Pt::from(10.0)]),
        );

        // Level 6 on Level 5
        self.images[4].draw(
            ctx,
            &self.images[5],
            DrawOption::default().with_position([Pt::from(10.0), Pt::from(10.0)]),
        );

        // Level 5 on Level 4
        self.images[3].draw(
            ctx,
            &self.images[4],
            DrawOption::default().with_position([Pt::from(10.0), Pt::from(10.0)]),
        );

        // Level 4 on Level 3
        self.images[2].draw(
            ctx,
            &self.images[3],
            DrawOption::default().with_position([Pt::from(10.0), Pt::from(10.0)]),
        );

        // Level 3 on Level 2
        self.images[1].draw(
            ctx,
            &self.images[2],
            DrawOption::default().with_position([Pt::from(10.0), Pt::from(10.0)]),
        );

        // Level 2 on Level 1
        self.images[0].draw(
            ctx,
            &self.images[1],
            DrawOption::default().with_position([Pt::from(10.0), Pt::from(10.0)]),
        );

        // 3. Draw the final root onto the screen
        screen.draw(
            ctx,
            &self.images[0],
            DrawOption::default().with_position([Pt::from(50.0), Pt::from(50.0)]),
        );

        // 4. Draw text on the top-most layer (Level 7)
        let text = spottedcat::Text::new("7级嵌套递归渲染测试", self.font_id)
            .with_font_size(Pt::from(16.0))
            .with_color([0.0, 0.0, 0.0, 1.0]);

        self.images[6].draw(
            ctx,
            &text,
            DrawOption::default().with_position([Pt::from(10.0), Pt::from(40.0)]),
        );
    }

    fn update(&mut self, _ctx: &mut Context, _dt: std::time::Duration) {}

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    run::<SevenLevelNestTestSpot>(WindowConfig::default());
}
