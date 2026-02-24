use spottedcat::{Context, DrawOption, Image, Pt, Spot, WindowConfig, run};

struct SevenLevelNestTestSpot {
    images: Vec<Image>,
    font_id: u32,
}

impl Spot for SevenLevelNestTestSpot {
    fn initialize(_context: &mut Context) -> Self {
        let mut images = Vec::new();

        // Level 1 - Red parent (200x200)
        let mut rgba = vec![255u8; 200 * 200 * 4];
        for i in 0..200 * 200 {
            rgba[i * 4 + 1] = 0; // G
            rgba[i * 4 + 2] = 0; // B
            rgba[i * 4 + 3] = 255; // A
        }
        images.push(Image::new_from_rgba8(Pt::from(200.0), Pt::from(200.0), &rgba).unwrap());

        // Level 2 - Blue
        let mut rgba = vec![0u8; 200 * 200 * 4];
        for i in 0..200 * 200 {
            rgba[i * 4 + 2] = 255; // B
            rgba[i * 4 + 3] = 255; // A
        }
        images.push(Image::new_from_rgba8(Pt::from(200.0), Pt::from(200.0), &rgba).unwrap());

        // Level 3 - Green
        let mut rgba = vec![0u8; 200 * 200 * 4];
        for i in 0..200 * 200 {
            rgba[i * 4 + 1] = 255; // G
            rgba[i * 4 + 3] = 255; // A
        }
        images.push(Image::new_from_rgba8(Pt::from(200.0), Pt::from(200.0), &rgba).unwrap());

        // Level 4 - Yellow
        let mut rgba = vec![255u8; 200 * 200 * 4];
        for i in 0..200 * 200 {
            rgba[i * 4 + 2] = 0; // B
            rgba[i * 4 + 3] = 255; // A
        }
        images.push(Image::new_from_rgba8(Pt::from(200.0), Pt::from(200.0), &rgba).unwrap());

        // Level 5 - Cyan
        let mut rgba = vec![0u8; 200 * 200 * 4];
        for i in 0..200 * 200 {
            rgba[i * 4 + 1] = 255; // G
            rgba[i * 4 + 2] = 255; // B
            rgba[i * 4 + 3] = 255; // A
        }
        images.push(Image::new_from_rgba8(Pt::from(200.0), Pt::from(200.0), &rgba).unwrap());

        // Level 6 - Magenta
        let mut rgba = vec![255u8; 200 * 200 * 4];
        for i in 0..200 * 200 {
            rgba[i * 4 + 1] = 0; // G
            rgba[i * 4 + 3] = 255; // A
        }
        images.push(Image::new_from_rgba8(Pt::from(200.0), Pt::from(200.0), &rgba).unwrap());

        // Level 7 - White
        let mut rgba = vec![255u8; 100 * 100 * 4];
        for i in 0..100 * 100 {
            rgba[i * 4 + 3] = 255; // A
        }
        images.push(Image::new_from_rgba8(Pt::from(100.0), Pt::from(100.0), &rgba).unwrap());

        // Load default font
        const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
        let font_id = spottedcat::register_font(FONT.to_vec());

        Self { images, font_id }
    }

    fn draw(&mut self, context: &mut Context) {
        // Level 1 - Red parent
        let level1_opts = DrawOption::default().with_position([Pt::from(-10.0), Pt::from(50.0)]);

        self.images[0].with_clip_scope(context, level1_opts, |ctx| {
            // Level 2 - Blue
            let level2_opts = DrawOption::default().with_position([Pt::from(10.0), Pt::from(10.0)]);

            self.images[1].with_clip_scope(ctx, level2_opts, |ctx2| {
                // Level 3 - Green
                let level3_opts =
                    DrawOption::default().with_position([Pt::from(10.0), Pt::from(10.0)]);

                self.images[2].with_clip_scope(ctx2, level3_opts, |ctx3| {
                    // Level 4 - Yellow
                    let level4_opts =
                        DrawOption::default().with_position([Pt::from(10.0), Pt::from(10.0)]);

                    self.images[3].with_clip_scope(ctx3, level4_opts, |ctx4| {
                        // Level 5 - Cyan
                        let level5_opts =
                            DrawOption::default().with_position([Pt::from(10.0), Pt::from(10.0)]);

                        self.images[4].with_clip_scope(ctx4, level5_opts, |ctx5| {
                            // Level 6 - Magenta
                            let level6_opts = DrawOption::default()
                                .with_position([Pt::from(10.0), Pt::from(10.0)]);

                            self.images[5].with_clip_scope(ctx5, level6_opts, |ctx6| {
                                // Level 7 - White (half size with text)
                                let level7_opts = DrawOption::default()
                                    .with_position([Pt::from(10.0), Pt::from(10.0)]);

                                self.images[6].with_clip_scope(ctx6, level7_opts, |ctx7| {
                                    // Add text in the center of level7
                                    let text_opts = DrawOption::default()
                                        .with_position([Pt::from(25.0), Pt::from(40.0)]);

                                    let text = spottedcat::Text::new(
                                        "7级嵌套测试文字宽度测试",
                                        self.font_id,
                                    )
                                    .with_font_size(Pt::from(16.0))
                                    .with_color([0.0, 0.0, 0.0, 1.0]); // Black text

                                    text.draw(ctx7, text_opts);
                                });
                            });
                        });
                    });
                });
            });
        });
    }

    fn update(&mut self, _context: &mut Context, _dt: std::time::Duration) {}

    fn remove(&self) {}
}

fn main() {
    run::<SevenLevelNestTestSpot>(WindowConfig::default());
}
