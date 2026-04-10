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

        // Create a larger outer image with a 400x150 logical display size
        let outer_width = 400u32;
        let outer_height = 150u32;
        let mut outer_rgba = vec![240u8; (outer_width as usize) * (outer_height as usize) * 4]; // Light blue background

        for i in 0..(outer_width as usize) * (outer_height as usize) {
            outer_rgba[i * 4] = 240; // R
            outer_rgba[i * 4 + 1] = 248; // G
            outer_rgba[i * 4 + 2] = 255; // B
            outer_rgba[i * 4 + 3] = 255; // A
        }

        let outer_image = Image::new(
            ctx,
            Pt::from(outer_width),
            Pt::from(outer_height),
            &outer_rgba,
        )
        .unwrap();

        // Create inner image with a 300x50 logical display size
        let width = 300u32;
        let height = 50u32;
        let mut rgba = vec![200u8; (width as usize) * (height as usize) * 4]; // Light gray background

        for i in 0..(width as usize) * (height as usize) {
            rgba[i * 4] = 200; // R
            rgba[i * 4 + 1] = 200; // G
            rgba[i * 4 + 2] = 200; // B
            rgba[i * 4 + 3] = 255; // A
        }

        let inner_image = Image::new(ctx, Pt::from(width), Pt::from(height), &rgba).unwrap();

        Self {
            font_id,
            outer_image,
            inner_image,
        }
    }

    fn draw(&mut self, ctx: &mut Context, screen: spottedcat::Image) {
        let width = 300.0;
        let height = 50.0;

        // Draw outer image at position (50, 50)
        let outer_pos = [50.0, 50.0];
        screen.draw(
            ctx,
            &self.outer_image,
            DrawOption::default().with_position([Pt::from(outer_pos[0]), Pt::from(outer_pos[1])]),
        );

        // Draw inner image at (10, 10) position within outer image
        // Absolute position: (50+10, 50+10) = (60, 60)
        let inner_pos = [outer_pos[0] + 10.0, outer_pos[1] + 10.0];
        screen.draw(
            ctx,
            &self.inner_image,
            DrawOption::default().with_position([Pt::from(inner_pos[0]), Pt::from(inner_pos[1])]),
        );

        // Calculate centered position for text within the inner image
        let text_content = "Centered Text";
        let font_size = Pt::from(20.0);

        // Create text to measure its dimensions
        let text = spottedcat::Text::new(text_content, self.font_id)
            .with_font_size(font_size)
            .with_color([0.0, 0.0, 0.0, 1.0]);

        // Get text dimensions
        let (text_width, text_height, _) = spottedcat::text::measure_with_y_offset(ctx, &text);

        // Calculate centered position relative to inner image
        let centered_x = (width - text_width) / 2.0;
        let centered_y = (height - text_height) / 2.0;

        // Absolute text position: inner_pos + centered
        let text_abs_pos = [inner_pos[0] + centered_x, inner_pos[1] + centered_y];

        screen.draw(
            ctx,
            &text,
            DrawOption::default()
                .with_position([Pt::from(text_abs_pos[0]), Pt::from(text_abs_pos[1])]),
        );
    }

    fn update(&mut self, _ctx: &mut Context, _dt: std::time::Duration) {}

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    run::<CenteredTextTestSpot>(WindowConfig::default());
}
