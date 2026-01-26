use spottedcat::{Context, DrawOption, Pt, Spot, WindowConfig, run, Image, load_font_from_bytes};

struct SevenLevelNestTestSpot {
    red_image: Image,
    font_id: u32,
}

impl Spot for SevenLevelNestTestSpot {
    fn initialize(_context: &mut Context) -> Self {
        // Create red parent image (200x200)
        let mut red_rgba = vec![255u8; 200 * 200 * 4];
        for i in 0..200 * 200 {
            red_rgba[i * 4 + 1] = 0;     // G
            red_rgba[i * 4 + 2] = 0;     // B
            red_rgba[i * 4 + 3] = 255;   // A
        }
        let red_image = Image::new_from_rgba8(Pt::from(200.0), Pt::from(200.0), &red_rgba).unwrap();

        // Load default font
        const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
        let font_data = load_font_from_bytes(FONT);
        let font_id = spottedcat::register_font(font_data);

        Self { red_image, font_id }
    }

    fn draw(&mut self, context: &mut Context) {
        println!("=== 7 Level Deep Nesting Test (same size, 10,10 offset) ===");
        
        // Level 1 - Red parent
        let level1_opts = DrawOption::default()
            .with_position([Pt::from(-10.0), Pt::from(50.0)]);
        
        self.red_image.with_clip_scope(context, level1_opts, |ctx| {
            println!("RED level1: 200x200 at (50, 50) - auto drawn");
            
            // Level 2 - Blue
            let level2_opts = DrawOption::default()
                .with_position([Pt::from(10.0), Pt::from(10.0)]);
            
            let mut level2_rgba = vec![0u8; 200 * 200 * 4];
            for i in 0..200 * 200 {
                level2_rgba[i * 4 + 2] = 255; // B
                level2_rgba[i * 4 + 3] = 255; // A
            }
            let level2_image = Image::new_from_rgba8(Pt::from(200.0), Pt::from(200.0), &level2_rgba).unwrap();
            
            level2_image.with_clip_scope(ctx, level2_opts, |ctx2| {
                println!("BLUE level2: 200x200 at (10, 10) relative - 2nd level");
                
                // Level 3 - Green
                let level3_opts = DrawOption::default()
                    .with_position([Pt::from(10.0), Pt::from(10.0)]);
                
                let mut level3_rgba = vec![0u8; 200 * 200 * 4];
                for i in 0..200 * 200 {
                    level3_rgba[i * 4] = 0;       // R
                    level3_rgba[i * 4 + 1] = 255; // G
                    level3_rgba[i * 4 + 2] = 0;   // B
                    level3_rgba[i * 4 + 3] = 255; // A
                }
                let level3_image = Image::new_from_rgba8(Pt::from(200.0), Pt::from(200.0), &level3_rgba).unwrap();
                
                level3_image.with_clip_scope(ctx2, level3_opts, |ctx3| {
                    println!("GREEN level3: 200x200 at (10, 10) relative - 3rd level");
                    
                    // Level 4 - Yellow
                    let level4_opts = DrawOption::default()
                        .with_position([Pt::from(10.0), Pt::from(10.0)]);
                    
                    let mut level4_rgba = vec![255u8; 200 * 200 * 4];
                    for i in 0..200 * 200 {
                        level4_rgba[i * 4] = 255;     // R
                        level4_rgba[i * 4 + 1] = 255; // G
                        level4_rgba[i * 4 + 2] = 0;   // B
                        level4_rgba[i * 4 + 3] = 255; // A
                    }
                    let level4_image = Image::new_from_rgba8(Pt::from(200.0), Pt::from(200.0), &level4_rgba).unwrap();
                    
                    level4_image.with_clip_scope(ctx3, level4_opts, |ctx4| {
                        println!("YELLOW level4: 200x200 at (10, 10) relative - 4th level");
                        
                        // Level 5 - Cyan
                        let level5_opts = DrawOption::default()
                            .with_position([Pt::from(10.0), Pt::from(10.0)]);
                        
                        let mut level5_rgba = vec![0u8; 200 * 200 * 4];
                        for i in 0..200 * 200 {
                            level5_rgba[i * 4] = 0;       // R
                            level5_rgba[i * 4 + 1] = 255; // G
                            level5_rgba[i * 4 + 2] = 255; // B
                            level5_rgba[i * 4 + 3] = 255; // A
                        }
                        let level5_image = Image::new_from_rgba8(Pt::from(200.0), Pt::from(200.0), &level5_rgba).unwrap();
                        
                        level5_image.with_clip_scope(ctx4, level5_opts, |ctx5| {
                            println!("CYAN level5: 200x200 at (10, 10) relative - 5th level");
                            
                            // Level 6 - Magenta
                            let level6_opts = DrawOption::default()
                                .with_position([Pt::from(10.0), Pt::from(10.0)]);
                            
                            let mut level6_rgba = vec![255u8; 200 * 200 * 4];
                            for i in 0..200 * 200 {
                                level6_rgba[i * 4] = 255;     // R
                                level6_rgba[i * 4 + 1] = 0;   // G
                                level6_rgba[i * 4 + 2] = 255; // B
                                level6_rgba[i * 4 + 3] = 255; // A
                            }
                            let level6_image = Image::new_from_rgba8(Pt::from(200.0), Pt::from(200.0), &level6_rgba).unwrap();
                            
                            level6_image.with_clip_scope(ctx5, level6_opts, |ctx6| {
                                println!("MAGENTA level6: 200x200 at (10, 10) relative - 6th level");
                                
                                // Level 7 - White (half size with text)
                                let level7_opts = DrawOption::default()
                                    .with_position([Pt::from(10.0), Pt::from(10.0)]);
                                
                                let mut level7_rgba = vec![255u8; 100 * 100 * 4];
                                for i in 0..100 * 100 {
                                    level7_rgba[i * 4 + 3] = 255; // A
                                }
                                let level7_image = Image::new_from_rgba8(Pt::from(100.0), Pt::from(100.0), &level7_rgba).unwrap();
                                
                                level7_image.with_clip_scope(ctx6, level7_opts, |ctx7| {
                                    println!("WHITE level7: 100x100 at (10, 10) relative - 7th level (half size)");
                                    
                                    // Add text in the center of level7 (now properly clipped)
                                    let text_opts = DrawOption::default()
                                        .with_position([Pt::from(25.0), Pt::from(40.0)]);
                                    
                                    let text = spottedcat::Text::new("7级嵌套测试文字宽度测试", self.font_id)
                                        .with_font_size(Pt::from(16.0))
                                        .with_color([0.0, 0.0, 0.0, 1.0]); // Black text
                                    
                                    text.draw(ctx7, text_opts);
                                    println!("TEXT: '7级嵌套测试文字宽度测试' at (25, 40) relative - properly clipped by level7");
                                });
                            });
                        });
                    });
                });
            });
        });
    }

    fn update(&mut self, _context: &mut Context, _dt: std::time::Duration) {
    }

    fn remove(&self) {}
}

fn main() {
    run::<SevenLevelNestTestSpot>(WindowConfig::default());
}
