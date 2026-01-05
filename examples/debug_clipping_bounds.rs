fn main() {
    use spottedcat::{Text, load_font_from_bytes, Pt};
    
    const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
    let font_data = load_font_from_bytes(FONT);
    
    // Create text like in nested_clipping_example
    let text = Text::new("Clipped Text", font_data)
        .with_font_size(Pt::from(24.0))
        .with_color([1.0, 1.0, 1.0, 1.0]);
    
    let (width, height) = text.measure();
    println!("Text size: {:.1} x {:.1}", width, height);
    
    // Calculate positions like in nested_clipping_example
    // Father: 200x200 at (200, 200)
    let father_x = 200.0;
    let father_y = 200.0;
    let father_w = 200.0;
    let father_h = 200.0;
    
    // Text relative to Father at (50, 50)
    let text_rel_x = 50.0;
    let text_rel_y = 50.0;
    
    // Text absolute screen position
    let text_abs_x = father_x + text_rel_x;
    let text_abs_y = father_y + text_rel_y;
    
    println!("Father bounds: ({:.1}, {:.1}) to ({:.1}, {:.1})", 
        father_x, father_y, father_x + father_w, father_y + father_h);
    println!("Text position: ({:.1}, {:.1})", text_abs_x, text_abs_y);
    println!("Text bounds: ({:.1}, {:.1}) to ({:.1}, {:.1})", 
        text_abs_x, text_abs_y, text_abs_x + width, text_abs_y + height);
    
    // Check if text is fully within Father
    let text_right = text_abs_x + width;
    let text_bottom = text_abs_y + height;
    let father_right = father_x + father_w;
    let father_bottom = father_y + father_h;
    
    println!("Text right: {:.1}, Father right: {:.1}", text_right, father_right);
    println!("Text bottom: {:.1}, Father bottom: {:.1}", text_bottom, father_bottom);
    
    if text_abs_x >= father_x && text_abs_y >= father_y && 
       text_right <= father_right && text_bottom <= father_bottom {
        println!("Text should be FULLY VISIBLE within Father");
    } else {
        println!("Text should be CLIPPED by Father");
        
        // Calculate visible area
        let visible_left = text_abs_x.max(father_x);
        let visible_top = text_abs_y.max(father_y);
        let visible_right = text_right.min(father_right);
        let visible_bottom = text_bottom.min(father_bottom);
        let visible_width = (visible_right - visible_left).max(0.0);
        let visible_height = (visible_bottom - visible_top).max(0.0);
        
        println!("Visible area: {:.1} x {:.1} at ({:.1}, {:.1})", 
            visible_width, visible_height, visible_left, visible_top);
        
        // Estimate how many characters should be visible
        let char_ratio = visible_width / width;
        let total_chars = "Clipped Text".len();
        let visible_chars = (total_chars as f32 * char_ratio).round() as usize;
        println!("Estimated visible characters: {} / {}", visible_chars, total_chars);
        if visible_chars < total_chars {
            let visible_text = "Clipped Text".chars().take(visible_chars).collect::<String>();
            println!("Visible text approximation: '{}'", visible_text);
        }
    }
}
