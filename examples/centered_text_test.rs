use spottedcat::{Context, DrawOption, Pt, Spot, WindowConfig, run, Image, load_font_from_bytes};

struct CenteredTextTestSpot {
    font_id: u32,
}

impl Spot for CenteredTextTestSpot {
    fn initialize(_context: &mut Context) -> Self {
        // Load default font
        const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
        let font_data = load_font_from_bytes(FONT);
        let font_id = spottedcat::register_font(font_data);

        Self { font_id }
    }

    fn draw(&mut self, context: &mut Context) {
        println!("=== Centered Text in 50px Height Image Test ===");
        
        // Create a larger outer image (400x150)
        let outer_width = 400.0;
        let outer_height = 150.0;
        let mut outer_rgba = vec![240u8; (outer_width as usize) * (outer_height as usize) * 4]; // Light blue background
        
        for i in 0..(outer_width as usize) * (outer_height as usize) {
            outer_rgba[i * 4] = 240;     // R
            outer_rgba[i * 4 + 1] = 248; // G  
            outer_rgba[i * 4 + 2] = 255; // B
            outer_rgba[i * 4 + 3] = 255; // A
        }
        
        let outer_image = Image::new_from_rgba8(Pt::from(outer_width), Pt::from(outer_height), &outer_rgba).unwrap();
        
        // Create inner image (300x50)
        let width = 300.0;
        let height = 50.0;
        let mut rgba = vec![200u8; (width as usize) * (height as usize) * 4]; // Light gray background
        
        for i in 0..(width as usize) * (height as usize) {
            rgba[i * 4] = 200;     // R
            rgba[i * 4 + 1] = 200; // G  
            rgba[i * 4 + 2] = 200; // B
            rgba[i * 4 + 3] = 255; // A
        }
        
        let inner_image = Image::new_from_rgba8(Pt::from(width), Pt::from(height), &rgba).unwrap();
        
        // Draw outer image at position (50, 50)
        let outer_opts = DrawOption::default()
            .with_position([Pt::from(50.0), Pt::from(50.0)]);
        
        outer_image.with_clip_scope(context, outer_opts, |ctx1| {
            println!("Outer Image: 400x150 at (50, 50) - auto drawn");
            
            // Draw inner image at (10, 10) position within outer image
            let inner_opts = DrawOption::default()
                .with_position([Pt::from(10.0), Pt::from(10.0)]);
            
            inner_image.with_clip_scope(ctx1, inner_opts, |ctx2| {
                println!("Inner Image: 300x50 at (10, 10) relative - auto drawn");
                
                // Calculate centered position for text
                let text_content = "Centered Text";
                let font_size = Pt::from(20.0);
                
                // Create text to measure its dimensions including baseline offset
                let text = spottedcat::Text::new(text_content, self.font_id)
                    .with_font_size(font_size)
                    .with_color([0.0, 0.0, 0.0, 1.0]); // Black text
                
                // Get text dimensions and baseline offset
                let (text_width, text_height, y_offset) = text.measure_with_y_offset();
                
                // Calculate centered position within the inner image
                let centered_x = (width - text_width) / 2.0;
                let centered_y = (height - text_height) / 2.0;
                
                println!("Text: '{}' size: {:.1}x{:.1}, y_offset: {:.1}, geometric centered at ({:.1}, {:.1})", 
                        text_content, text_width, text_height, y_offset, centered_x, centered_y);
                
                // Draw text at geometric centered position
                let text_opts = DrawOption::default()
                    .with_position([Pt::from(centered_x), Pt::from(centered_y)]);
                
                text.draw(ctx2, text_opts);
                println!("Text drawn centered in nested images");
            });
        });
    }

    fn update(&mut self, _context: &mut Context, _dt: std::time::Duration) {
    }

    fn remove(&self) {}
}

fn main() {
    run::<CenteredTextTestSpot>(WindowConfig::default());
}
