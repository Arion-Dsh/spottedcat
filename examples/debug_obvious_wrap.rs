fn main() {
    use spottedcat::{Text, load_font_from_bytes, Pt};
    
    const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
    let font_data = load_font_from_bytes(FONT);
    
    // Test the wrapping logic first
    use ab_glyph::{Font as _, FontArc, PxScale};
    
    let font = FontArc::try_from_vec(font_data.clone()).unwrap();
    let px_size = 24.0f32.max(1.0);
    let scale = PxScale::from(px_size);
    let scaled = font.as_scaled(scale);
    
    // Create text with wrapping
    let text = Text::new(
        "This is a very long sentence that should definitely wrap into multiple lines when displayed with a narrow width constraint.",
        font_data
    )
        .with_font_size(Pt::from(24.0))
        .with_max_width(Pt::from(200.0));
    
    let lines = text.get_wrapped_lines(&scaled);
    println!("Number of lines: {}", lines.len());
    
    for (i, line) in lines.iter().enumerate() {
        let line_width = text.measure_line_width(line, &scaled);
        println!("Line {}: '{}' (width: {:.1}px)", i + 1, line, line_width);
    }
    
    let (width, height) = text.measure();
    println!("Total measured size: {:.1} x {:.1}", width, height);
}
