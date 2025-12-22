use spottedcat::{Context, Key, Spot, Text, TextOptions};

struct TextInputExample {
    committed: String,
    preedit: String,
    font_data: Vec<u8>,
    capture_enabled: bool,
}

impl Spot for TextInputExample {
    fn initialize(_: &mut Context) -> Self {
        // Prefer a system font that contains CJK glyphs.
        // Fallback to the bundled DejaVuSans.ttf.
        const FALLBACK_FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");

        let font_candidates = [
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/Supplemental/Songti.ttc",
            "/System/Library/Fonts/Supplemental/Heiti TC.ttc",
            "/System/Library/Fonts/Supplemental/STHeiti Medium.ttc",
        ];

        let mut font_data = None;
        for path in font_candidates {
            if let Ok(data) = spottedcat::load_font_from_file(path) {
                font_data = Some(data);
                break;
            }
        }

        let font_data = font_data.unwrap_or_else(|| spottedcat::load_font_from_bytes(FALLBACK_FONT));
        Self {
            committed: String::new(),
            preedit: String::new(),
            font_data,
            capture_enabled: false,
        }
    }

    fn update(&mut self, ctx: &mut Context, _dt: std::time::Duration) {
        if spottedcat::key_pressed(ctx, Key::F1) {
            self.capture_enabled = !self.capture_enabled;
            spottedcat::set_text_input_enabled(ctx, self.capture_enabled);
        }

        // Append characters entered this frame.
        self.committed.push_str(spottedcat::text_input(ctx));

        // Cache IME preedit so draw() doesn't need to query input state.
        self.preedit = spottedcat::ime_preedit(ctx).unwrap_or("").to_string();

        // Simple editing: Backspace deletes one Unicode scalar value.
        if spottedcat::key_pressed(ctx, Key::Backspace) {
            self.committed.pop();
        }

        // Clear input.
        if spottedcat::key_pressed(ctx, Key::Escape) {
            self.committed.clear();
            self.preedit.clear();
        }
    }

    fn draw(&mut self, ctx: &mut Context) {
        let mut title_opts = TextOptions::new(self.font_data.clone());
        title_opts.position = [spottedcat::Pt(20), spottedcat::Pt(40)];
        title_opts.font_size = spottedcat::Pt(22);
        title_opts.color = [1.0, 1.0, 1.0, 1.0];
        let status = if self.capture_enabled { "ON" } else { "OFF" };
        Text::new(format!(
            "Text Input Example (capture: {}, F1 toggle, Backspace delete, Esc clear)",
            status
        ))
        .draw(ctx, title_opts);

        let mut input_opts = TextOptions::new(self.font_data.clone());
        input_opts.position = [spottedcat::Pt(20), spottedcat::Pt(90)];
        input_opts.font_size = spottedcat::Pt(28);
        input_opts.color = [0.9, 0.9, 0.9, 1.0];

        let mut composed = self.committed.clone();
        if !self.preedit.is_empty() {
            composed.push_str(&self.preedit);
        }
        Text::new(composed).draw(ctx, input_opts);

        if !self.preedit.is_empty() {
            let mut ime_opts = TextOptions::new(self.font_data.clone());
            ime_opts.position = [spottedcat::Pt(20), spottedcat::Pt(130)];
            ime_opts.font_size = spottedcat::Pt(16);
            ime_opts.color = [0.6, 0.8, 1.0, 1.0];
            Text::new(format!("IME preedit: {}", self.preedit)).draw(ctx, ime_opts);
        }
    }

    fn remove(&self) {}
}

fn main() {
    spottedcat::run::<TextInputExample>(spottedcat::WindowConfig::default());
}
