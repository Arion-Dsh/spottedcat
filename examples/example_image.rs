use spottedcat::{Context, Image, Pt};

pub fn happy_tree(ctx: &mut Context) -> Image {
    let (width, height, rgba) = happy_tree_rgba();
    Image::new(ctx, Pt::from(width as f32), Pt::from(height as f32), &rgba)
        .expect("generated happy tree should load")
}

fn happy_tree_rgba() -> (u32, u32, Vec<u8>) {
    let width = 1024u32;
    let height = 768u32;
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    let cx = width as f32 * 0.5;
    let canopy_cy = height as f32 * 0.34;

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let sky_t = y as f32 / height as f32;
            let mut color = [
                (72.0 + 80.0 * (1.0 - sky_t)) as u8,
                (140.0 + 70.0 * (1.0 - sky_t)) as u8,
                (210.0 + 35.0 * (1.0 - sky_t)) as u8,
                255,
            ];

            if y > height * 2 / 3 {
                color = [72, 156, 82, 255];
            }

            let trunk_half = 44.0 + (y as f32 - canopy_cy).max(0.0) * 0.03;
            if y > height / 3 && y < height - 80 && (x as f32 - cx).abs() < trunk_half {
                color = [118, 78, 42, 255];
            }

            let dx = x as f32 - cx;
            let dy = y as f32 - canopy_cy;
            let canopy = (dx * dx) / (310.0 * 310.0) + (dy * dy) / (235.0 * 235.0) < 1.0;
            if canopy {
                let shade = ((x / 32 + y / 24) % 3) as u8 * 14;
                color = [38, 126 + shade, 68, 255];
            }

            let face_y = canopy_cy + 35.0;
            let left_eye =
                ((x as f32 - (cx - 82.0)).powi(2) + (y as f32 - face_y).powi(2)) < 18.0 * 18.0;
            let right_eye =
                ((x as f32 - (cx + 82.0)).powi(2) + (y as f32 - face_y).powi(2)) < 18.0 * 18.0;
            if left_eye || right_eye {
                color = [18, 36, 26, 255];
            }

            let smile_dx = (x as f32 - cx).abs();
            let smile_y = face_y + 60.0 + (smile_dx / 95.0).powi(2) * 28.0;
            if smile_dx < 105.0 && (y as f32 - smile_y).abs() < 5.0 {
                color = [18, 36, 26, 255];
            }

            rgba[idx..idx + 4].copy_from_slice(&color);
        }
    }

    (width, height, rgba)
}
