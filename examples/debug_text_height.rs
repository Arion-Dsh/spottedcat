fn main() {
    use spottedcat::{Text, load_font_from_bytes, Pt};
    
    const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
    let font_data = load_font_from_bytes(FONT);
    
    // Create text WITHOUT wrapping (like in nested_clipping_example)
    let text = Text::new("Clipped Text", font_data.clone())
        .with_font_size(Pt::from(24.0))
        .with_color([1.0, 1.0, 1.0, 1.0]);
    
    // Test the wrapping logic
    use ab_glyph::{Font as _, FontArc, PxScale, ScaleFont as _};
    
    let font = FontArc::try_from_vec(font_data).unwrap();
    let px_size = 24.0f32.max(1.0);
    let scale = PxScale::from(px_size);
    let scaled = font.as_scaled(scale);
    
    let lines = text.get_wrapped_lines(&scaled);
    println!("Number of lines: {}", lines.len());
    
    // Calculate dimensions like in render_text_to_image
    let line_height = scaled.ascent() - scaled.descent();
    let text_height = (lines.len() as f32 * line_height).ceil().max(1.0) as u32;
    println!("Line height: {:.1}px", line_height);
    println!("Render text height: {}px", text_height);
    
    // Compare with measure method
    let (width, height) = text.measure();
    println!("Measure height: {:.1}px", height);
    
    for (i, line) in lines.iter().enumerate() {
        let line_width = text.measure_line_width(line, &scaled);
        println!("Line {}: '{}' (width: {:.1}px)", i + 1, line, line_width);
    }
    
    println!("Total measured size: {:.1} x {:.1}", width, height);
}