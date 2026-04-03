use spottedcat::{Context, DrawOption, Image, Pt, Spot, WindowConfig, run};

struct CenteredTextTestSpot {
    font_id: u32,
    outer_image: Image,
    inner_image: Image,
}

impl Spot for CenteredTextTestSpot {
    fn initialize(ctx: &mut Context) -> Self {
        // Load default font
        const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
        let font_id = spottedcat::register_font(ctx, FONT.to_vec());

        // Create a larger outer image (400x150)
        let outer_width = 400.0;
        let outer_height = 150.0;
        let mut outer_rgba = vec![240u8; (outer_width as usize) * (outer_height as usize) * 4]; // Light blue background

        for i in 0..(outer_width as usize) * (outer_height as usize) {
            outer_rgba[i * 4] = 240; // R
            outer_rgba[i * 4 + 1] = 248; // G
            outer_rgba[i * 4 + 2] = 255; // B
            outer_rgba[i * 4 + 3] = 255; // A
        }

        let outer_image = spottedcat::image::create(
            ctx,
            Pt::from(outer_width),
            Pt::from(outer_height),
            &outer_rgba,
        )
        .unwrap();

        // Create inner image (300x50)
        let width = 300.0;
        let height = 50.0;
        let mut rgba = vec![200u8; (width as usize) * (height as usize) * 4]; // Light gray background

        for i in 0..(width as usize) * (height as usize) {
            rgba[i * 4] = 200; // R
            rgba[i * 4 + 1] = 200; // G
            rgba[i * 4 + 2] = 200; // B
            rgba[i * 4 + 3] = 255; // A
        }

        let inner_image =
            spottedcat::image::create(ctx, Pt::from(width), Pt::from(height), &rgba).unwrap();

        Self {
            font_id,
            outer_image,
            inner_image,
        }
    }

    fn draw(&mut self, ctx: &mut Context) {
        let width = 300.0;
        let height = 50.0;

        // Draw outer image at position (50, 50)
        let outer_opts = DrawOption::default().with_position([Pt::from(50.0), Pt::from(50.0)]);

        spottedcat::image::with_clip_scope(ctx, self.outer_image, outer_opts, |ctx1| {
            // Draw inner image at (10, 10) position within outer image
            let inner_opts = DrawOption::default().with_position([Pt::from(10.0), Pt::from(10.0)]);

            spottedcat::image::with_clip_scope(ctx1, self.inner_image, inner_opts, |ctx2| {
                // Calculate centered position for text
                let text_content = "Centered Text";
                let font_size = Pt::from(20.0);

                // Create text to measure its dimensions including baseline offset
                let text = spottedcat::Text::new(text_content, self.font_id)
                    .with_font_size(font_size)
                    .with_color([0.0, 0.0, 0.0, 1.0]); // Black text

                // Get text dimensions and baseline offset
                let (text_width, text_height, _) =
                    spottedcat::text::measure_with_y_offset(ctx2, &text);

                // Calculate centered position within the inner image
                let centered_x = (width - text_width) / 2.0;
                let centered_y = (height - text_height) / 2.0;

                // Draw text at geometric centered position
                let text_opts = DrawOption::default()
                    .with_position([Pt::from(centered_x), Pt::from(centered_y)]);

                spottedcat::text::draw(ctx2, &text, text_opts);
            });
        });
    }

    fn update(&mut self, _ctx: &mut Context, _dt: std::time::Duration) {}

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    run::<CenteredTextTestSpot>(WindowConfig::default());
}
