use spottedcat::Context;

pub fn register(ctx: &mut Context) -> u32 {
    let Some(bytes) = default_font_bytes() else {
        panic!("no system font found for text example");
    };
    spottedcat::register_font(ctx, bytes)
}

fn default_font_bytes() -> Option<Vec<u8>> {
    default_font_paths()
        .iter()
        .find_map(|path| std::fs::read(path).ok())
}

#[cfg(target_os = "windows")]
fn default_font_paths() -> &'static [&'static str] {
    &[
        "C:\\Windows\\Fonts\\arial.ttf",
        "C:\\Windows\\Fonts\\segoeui.ttf",
    ]
}

#[cfg(target_os = "macos")]
fn default_font_paths() -> &'static [&'static str] {
    &[
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/Supplemental/Helvetica.ttf",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
    ]
}

#[cfg(target_os = "linux")]
fn default_font_paths() -> &'static [&'static str] {
    &[
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/freefont/FreeSans.ttf",
    ]
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn default_font_paths() -> &'static [&'static str] {
    &[]
}
