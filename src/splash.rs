use crate::{Context, DrawOption, Image, Key, MouseButton, Pt, Spot, TouchPhase, switch_scene};
use std::marker::PhantomData;
use std::rc::Rc;
use std::time::Duration;

const AUTO_ADVANCE_AFTER: f32 = 2.8;
const INPUT_SKIP_AFTER: f32 = 0.75;
const PANEL_WIDTH: usize = 96;
const PANEL_HEIGHT: usize = 72;
const LOGO_SIZE: usize = 32;
const SAFE_MARGIN_MIN: f32 = 18.0;
const LOGO_TOP_INSET: usize = 3;
const LOGO_BOTTOM_INSET: usize = 10;

/// A reusable startup splash scene with the built-in Spottedcat branding.
///
/// The visual language is intentionally pixel-forward: a Rusty-spotted cat
/// inside a compact frame, tying the intro back to the world's smallest wild
/// cat and the engine's small, agile, practical direction.
///
/// ```rust,no_run
/// use spottedcat::{OneShotSplash, Spot, WindowConfig, run};
///
/// struct Game;
///
/// impl Spot for Game {
///     fn initialize(_ctx: &mut spottedcat::Context) -> Self {
///         Self
///     }
///
///     fn update(&mut self, _ctx: &mut spottedcat::Context, _dt: std::time::Duration) {}
///
///     fn draw(&mut self, _ctx: &mut spottedcat::Context, _screen: spottedcat::Image) {}
/// }
///
/// fn main() {
///     run::<OneShotSplash<Game>>(WindowConfig::default());
/// }
/// ```
pub struct OneShotSplash<TNext: Spot + 'static> {
    inner: OneShotSplashInner<TNext>,
}

enum OneShotSplashInner<TNext: Spot + 'static> {
    Splash(BrandedSplash<TNext>),
    Next(TNext),
}

struct OneShotSplashSeen<TNext: Spot + 'static>(PhantomData<TNext>);

struct BrandedSplash<TNext: Spot + 'static> {
    elapsed: f32,
    panel: Option<Image>,
    logo: Option<Image>,
    wordmark: Option<Image>,
    panel_scale_px: usize,
    logo_scale_px: usize,
    wordmark_scale_px: usize,
    switched: bool,
    _next: PhantomData<TNext>,
}

impl<TNext: Spot + 'static> Spot for OneShotSplash<TNext> {
    fn initialize(ctx: &mut Context) -> Self {
        let already_shown = crate::get_resource::<OneShotSplashSeen<TNext>>(ctx).is_some();
        let inner = if already_shown {
            OneShotSplashInner::Next(TNext::initialize(ctx))
        } else {
            crate::insert_resource(ctx, Rc::new(OneShotSplashSeen::<TNext>(PhantomData)));
            OneShotSplashInner::Splash(BrandedSplash::new())
        };

        Self { inner }
    }

    fn update(&mut self, ctx: &mut Context, dt: Duration) {
        match &mut self.inner {
            OneShotSplashInner::Splash(splash) => splash.update(ctx, dt),
            OneShotSplashInner::Next(next) => next.update(ctx, dt),
        }
    }

    fn draw(&mut self, ctx: &mut Context, screen: Image) {
        match &mut self.inner {
            OneShotSplashInner::Splash(splash) => splash.draw(ctx, screen),
            OneShotSplashInner::Next(next) => next.draw(ctx, screen),
        }
    }

    fn resumed(&mut self, ctx: &mut Context) {
        match &mut self.inner {
            OneShotSplashInner::Splash(splash) => splash.resumed(ctx),
            OneShotSplashInner::Next(next) => next.resumed(ctx),
        }
    }

    fn suspended(&mut self, ctx: &mut Context) {
        match &mut self.inner {
            OneShotSplashInner::Splash(splash) => splash.suspended(ctx),
            OneShotSplashInner::Next(next) => next.suspended(ctx),
        }
    }

    fn remove(&mut self, ctx: &mut Context) {
        match &mut self.inner {
            OneShotSplashInner::Splash(splash) => splash.remove(ctx),
            OneShotSplashInner::Next(next) => next.remove(ctx),
        }
    }
}

impl<TNext: Spot + 'static> BrandedSplash<TNext> {
    fn new() -> Self {
        Self {
            elapsed: 0.0,
            panel: None,
            logo: None,
            wordmark: None,
            panel_scale_px: 0,
            logo_scale_px: 0,
            wordmark_scale_px: 0,
            switched: false,
            _next: PhantomData,
        }
    }

    fn update(&mut self, ctx: &mut Context, dt: Duration) {
        self.elapsed += dt.as_secs_f32();

        let skip_requested = self.elapsed >= INPUT_SKIP_AFTER
            && (crate::key_pressed(ctx, Key::Enter)
                || crate::key_pressed(ctx, Key::NumpadEnter)
                || crate::key_pressed(ctx, Key::Space)
                || crate::mouse_button_pressed(ctx, MouseButton::Left)
                || crate::touches(ctx)
                    .iter()
                    .any(|touch| touch.phase == TouchPhase::Started));

        if !self.switched && (skip_requested || self.elapsed >= AUTO_ADVANCE_AFTER) {
            self.switched = true;
            switch_scene::<TNext>();
        }
    }

    fn draw(&mut self, ctx: &mut Context, screen: Image) {
        let (window_w, window_h) = crate::window_size(ctx);
        let window_w = window_w.as_f32();
        let window_h = window_h.as_f32();
        let is_portrait = window_h >= window_w;
        let safe_margin = (window_w.min(window_h) * 0.05).max(SAFE_MARGIN_MIN);

        let intro = (self.elapsed / 0.65).clamp(0.0, 1.0);
        let intro_eased = ease_out_cubic(intro);

        let panel_scale_px = ((window_w * if is_portrait { 0.68 } else { 0.50 })
            / PANEL_WIDTH as f32)
            .min((window_h * if is_portrait { 0.38 } else { 0.46 }) / PANEL_HEIGHT as f32)
            .floor()
            .clamp(2.0, 4.0) as usize;
        let logo_scale_px = panel_scale_px + 1;
        let wordmark_scale_px = panel_scale_px.saturating_sub(1).max(2);
        self.ensure_scaled_assets(ctx, panel_scale_px, logo_scale_px, wordmark_scale_px);

        let panel_w = (PANEL_WIDTH * panel_scale_px) as f32;
        let panel_h = (PANEL_HEIGHT * panel_scale_px) as f32;
        let panel_x = ((window_w - panel_w) * 0.5).round();
        let panel_y = ((window_h - panel_h) * 0.5)
            .clamp(safe_margin, window_h - panel_h - safe_margin)
            .round();

        if let Some(panel) = self.panel {
            screen.draw(
                ctx,
                &panel,
                DrawOption::default()
                    .with_position([Pt::from(panel_x), Pt::from(panel_y)])
                    .with_opacity(0.35 + 0.65 * intro_eased),
            );
        }

        let panel_px = panel_scale_px as f32;
        let inner_pad = 9.0 * panel_px;
        let content_top = panel_y + inner_pad;
        let content_bottom = panel_y + panel_h - inner_pad;
        let gap = 1.25 * panel_px;

        let (wordmark_w, wordmark_h) = self
            .wordmark
            .map(|img| (img.width().as_f32(), img.height().as_f32()))
            .unwrap_or((0.0, 0.0));
        let wordmark_x = ((window_w - wordmark_w) * 0.5).round();

        let (logo_w, logo_h) = self
            .logo
            .map(|img| (img.width().as_f32(), img.height().as_f32()))
            .unwrap_or((0.0, 0.0));
        let logo_x = ((window_w - logo_w) * 0.5).round();
        let logo_top_trim = LOGO_TOP_INSET as f32 * logo_scale_px as f32;
        let logo_bottom_trim = LOGO_BOTTOM_INSET as f32 * logo_scale_px as f32;
        let logo_visual_height = (logo_h - logo_top_trim - logo_bottom_trim).max(0.0);
        let lockup_height = logo_visual_height + gap + wordmark_h;
        let lockup_y = (((content_top + content_bottom - lockup_height) * 0.5) - 3.0 * panel_px)
            .clamp(content_top, content_bottom - lockup_height)
            .round();
        let logo_visual_top = (lockup_y + 0.5 * panel_px).round();
        let logo_y = (logo_visual_top - logo_top_trim).round();
        let logo_visual_bottom = logo_y + logo_h - logo_bottom_trim;
        let wordmark_y = (logo_visual_bottom + gap).round();

        if let Some(logo) = self.logo {
            screen.draw(
                ctx,
                &logo,
                DrawOption::default()
                    .with_position([Pt::from(logo_x), Pt::from(logo_y)])
                    .with_opacity(intro_eased),
            );
        }

        if let Some(wordmark) = self.wordmark {
            screen.draw(
                ctx,
                &wordmark,
                DrawOption::default()
                    .with_position([Pt::from(wordmark_x), Pt::from(wordmark_y)])
                    .with_opacity(intro_eased),
            );
        }
    }

    fn resumed(&mut self, _ctx: &mut Context) {}

    fn suspended(&mut self, _ctx: &mut Context) {}

    fn remove(&mut self, _ctx: &mut Context) {}
}

impl<TNext: Spot + 'static> BrandedSplash<TNext> {
    fn ensure_scaled_assets(
        &mut self,
        ctx: &mut Context,
        panel_scale_px: usize,
        logo_scale_px: usize,
        wordmark_scale_px: usize,
    ) {
        if self.panel_scale_px != panel_scale_px || self.panel.is_none() {
            self.panel = Some(ctx.register_image(
                (PANEL_WIDTH * panel_scale_px) as u32,
                (PANEL_HEIGHT * panel_scale_px) as u32,
                Pt::from(PANEL_WIDTH * panel_scale_px),
                Pt::from(PANEL_HEIGHT * panel_scale_px),
                &build_panel_rgba(panel_scale_px),
            ));
            self.panel_scale_px = panel_scale_px;
        }

        if self.logo_scale_px != logo_scale_px || self.logo.is_none() {
            self.logo = Some(ctx.register_image(
                (LOGO_SIZE * logo_scale_px) as u32,
                (LOGO_SIZE * logo_scale_px) as u32,
                Pt::from(LOGO_SIZE * logo_scale_px),
                Pt::from(LOGO_SIZE * logo_scale_px),
                &build_logo_rgba(logo_scale_px),
            ));
            self.logo_scale_px = logo_scale_px;
        }

        if self.wordmark_scale_px != wordmark_scale_px || self.wordmark.is_none() {
            let (width, height, rgba) = build_wordmark_rgba(wordmark_scale_px);
            self.wordmark = Some(ctx.register_image(
                width as u32,
                height as u32,
                Pt::from(width),
                Pt::from(height),
                &rgba,
            ));
            self.wordmark_scale_px = wordmark_scale_px;
        }
    }
}

fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

fn build_panel_rgba(scale: usize) -> Vec<u8> {
    let mut rgba = solid_canvas(PANEL_WIDTH, PANEL_HEIGHT, [22, 28, 41, 220]);

    fill_rect(
        &mut rgba,
        PANEL_WIDTH,
        2,
        2,
        PANEL_WIDTH - 4,
        PANEL_HEIGHT - 4,
        [29, 37, 53, 235],
    );

    for y in 6..(PANEL_HEIGHT - 6) {
        for x in 6..(PANEL_WIDTH - 6) {
            if (x / 3 + y / 3) % 2 == 0 {
                set_px(&mut rgba, PANEL_WIDTH, x, y, [32, 41, 58, 235]);
            }
        }
    }

    outline_rect(
        &mut rgba,
        PANEL_WIDTH,
        1,
        1,
        PANEL_WIDTH - 2,
        PANEL_HEIGHT - 2,
        [79, 241, 217, 255],
    );
    outline_rect(
        &mut rgba,
        PANEL_WIDTH,
        3,
        3,
        PANEL_WIDTH - 6,
        PANEL_HEIGHT - 6,
        [241, 126, 72, 255],
    );

    fill_rect(&mut rgba, PANEL_WIDTH, 8, 8, 10, 4, [79, 241, 217, 255]);
    fill_rect(
        &mut rgba,
        PANEL_WIDTH,
        PANEL_WIDTH - 18,
        8,
        10,
        4,
        [241, 126, 72, 255],
    );
    fill_rect(
        &mut rgba,
        PANEL_WIDTH,
        8,
        PANEL_HEIGHT - 12,
        14,
        4,
        [241, 126, 72, 255],
    );
    fill_rect(
        &mut rgba,
        PANEL_WIDTH,
        PANEL_WIDTH - 22,
        PANEL_HEIGHT - 12,
        14,
        4,
        [79, 241, 217, 255],
    );

    upscale_rgba(&rgba, PANEL_WIDTH, PANEL_HEIGHT, scale)
}

fn build_logo_rgba(scale: usize) -> Vec<u8> {
    let mut rgba = solid_canvas(LOGO_SIZE, LOGO_SIZE, [0, 0, 0, 0]);

    let outline = [41, 28, 24, 255];
    let fur_dark = [166, 92, 46, 255];
    let fur_mid = [205, 118, 61, 255];
    let fur_light = [231, 152, 96, 255];
    let muzzle = [243, 219, 194, 255];
    let spot = [107, 61, 39, 255];
    let eye = [79, 241, 217, 255];
    let nose = [74, 40, 34, 255];

    fill_rect(&mut rgba, LOGO_SIZE, 10, 5, 4, 5, outline);
    fill_rect(&mut rgba, LOGO_SIZE, 18, 5, 4, 5, outline);
    fill_rect(&mut rgba, LOGO_SIZE, 11, 6, 2, 2, fur_dark);
    fill_rect(&mut rgba, LOGO_SIZE, 19, 6, 2, 2, fur_dark);
    set_px(&mut rgba, LOGO_SIZE, 11, 4, outline);
    set_px(&mut rgba, LOGO_SIZE, 20, 4, outline);
    set_px(&mut rgba, LOGO_SIZE, 10, 3, outline);
    set_px(&mut rgba, LOGO_SIZE, 21, 3, outline);

    fill_rect(&mut rgba, LOGO_SIZE, 7, 10, 18, 11, outline);
    fill_rect(&mut rgba, LOGO_SIZE, 6, 12, 2, 7, outline);
    fill_rect(&mut rgba, LOGO_SIZE, 24, 12, 2, 7, outline);
    fill_rect(&mut rgba, LOGO_SIZE, 8, 11, 16, 11, fur_mid);
    fill_rect(&mut rgba, LOGO_SIZE, 8, 20, 3, 2, fur_mid);
    fill_rect(&mut rgba, LOGO_SIZE, 21, 20, 3, 2, fur_mid);
    fill_rect(&mut rgba, LOGO_SIZE, 9, 12, 14, 9, fur_light);
    fill_rect(&mut rgba, LOGO_SIZE, 10, 20, 12, 1, fur_light);

    fill_rect(&mut rgba, LOGO_SIZE, 11, 17, 10, 5, muzzle);
    fill_rect(&mut rgba, LOGO_SIZE, 12, 16, 8, 2, muzzle);
    fill_rect(&mut rgba, LOGO_SIZE, 13, 15, 2, 1, muzzle);
    fill_rect(&mut rgba, LOGO_SIZE, 17, 15, 2, 1, muzzle);

    fill_rect(&mut rgba, LOGO_SIZE, 10, 13, 3, 2, outline);
    fill_rect(&mut rgba, LOGO_SIZE, 19, 13, 3, 2, outline);
    fill_rect(&mut rgba, LOGO_SIZE, 11, 13, 2, 2, eye);
    fill_rect(&mut rgba, LOGO_SIZE, 19, 13, 2, 2, eye);
    set_px(&mut rgba, LOGO_SIZE, 12, 13, [224, 255, 246, 255]);
    set_px(&mut rgba, LOGO_SIZE, 20, 13, [224, 255, 246, 255]);

    fill_rect(&mut rgba, LOGO_SIZE, 15, 17, 2, 2, nose);
    set_px(&mut rgba, LOGO_SIZE, 14, 19, nose);
    set_px(&mut rgba, LOGO_SIZE, 17, 19, nose);
    set_px(&mut rgba, LOGO_SIZE, 13, 18, nose);
    set_px(&mut rgba, LOGO_SIZE, 18, 18, nose);

    fill_rect(&mut rgba, LOGO_SIZE, 12, 10, 2, 2, spot);
    fill_rect(&mut rgba, LOGO_SIZE, 18, 10, 2, 2, spot);
    fill_rect(&mut rgba, LOGO_SIZE, 15, 11, 2, 2, spot);
    fill_rect(&mut rgba, LOGO_SIZE, 9, 16, 2, 2, spot);
    fill_rect(&mut rgba, LOGO_SIZE, 21, 16, 2, 2, spot);
    set_px(&mut rgba, LOGO_SIZE, 11, 12, spot);
    set_px(&mut rgba, LOGO_SIZE, 20, 12, spot);
    set_px(&mut rgba, LOGO_SIZE, 14, 10, spot);
    set_px(&mut rgba, LOGO_SIZE, 17, 10, spot);

    fill_rect(&mut rgba, LOGO_SIZE, 9, 8, 1, 4, outline);
    fill_rect(&mut rgba, LOGO_SIZE, 22, 8, 1, 4, outline);
    set_px(&mut rgba, LOGO_SIZE, 8, 10, outline);
    set_px(&mut rgba, LOGO_SIZE, 23, 10, outline);

    upscale_rgba(&rgba, LOGO_SIZE, LOGO_SIZE, scale)
}

fn build_wordmark_rgba(scale: usize) -> (usize, usize, Vec<u8>) {
    let scale = scale.max(1);
    let top_text = "RUSTY-SPOTTED";
    let bottom_text = "CAT";
    let top_width = measure_pixel_text_small(top_text);
    let bottom_width = measure_pixel_text(bottom_text);
    let rail_width = 5usize;
    let rail_gap = 2usize;
    let bottom_lockup_width = rail_width + rail_gap + bottom_width + rail_gap + rail_width;
    let base_width = top_width.max(bottom_lockup_width);
    let base_height = 13usize;
    let mut rgba = solid_canvas(base_width, base_height, [0, 0, 0, 0]);
    let main = [238, 226, 202, 255];
    let accent = [241, 126, 72, 255];
    let shadow = [79, 241, 217, 255];
    let top_x = (base_width - top_width) / 2;
    let bottom_x = (base_width - bottom_width) / 2;
    let top_y = 0usize;
    let bottom_y = 6usize;

    draw_pixel_text_small(&mut rgba, base_width, top_x, top_y, top_text, shadow);
    draw_pixel_text_small(&mut rgba, base_width, top_x, top_y + 1, top_text, accent);
    draw_pixel_text_small(&mut rgba, base_width, top_x, top_y, top_text, main);

    let rail_y = bottom_y + 2;
    let left_rail_x = bottom_x.saturating_sub(rail_gap + rail_width);
    let right_rail_x = bottom_x + bottom_width + rail_gap;
    fill_rect(
        &mut rgba,
        base_width,
        left_rail_x,
        rail_y,
        rail_width,
        1,
        shadow,
    );
    fill_rect(
        &mut rgba,
        base_width,
        left_rail_x,
        rail_y + 1,
        rail_width,
        1,
        accent,
    );
    fill_rect(
        &mut rgba,
        base_width,
        left_rail_x,
        rail_y,
        rail_width,
        1,
        main,
    );
    fill_rect(
        &mut rgba,
        base_width,
        right_rail_x,
        rail_y,
        rail_width,
        1,
        shadow,
    );
    fill_rect(
        &mut rgba,
        base_width,
        right_rail_x,
        rail_y + 1,
        rail_width,
        1,
        accent,
    );
    fill_rect(
        &mut rgba,
        base_width,
        right_rail_x,
        rail_y,
        rail_width,
        1,
        main,
    );

    draw_pixel_text(
        &mut rgba,
        base_width,
        bottom_x,
        bottom_y,
        bottom_text,
        shadow,
    );
    draw_pixel_text(
        &mut rgba,
        base_width,
        bottom_x,
        bottom_y + 1,
        bottom_text,
        accent,
    );
    draw_pixel_text(&mut rgba, base_width, bottom_x, bottom_y, bottom_text, main);

    (
        base_width * scale,
        base_height * scale,
        upscale_rgba(&rgba, base_width, base_height, scale),
    )
}

fn measure_pixel_text_small(text: &str) -> usize {
    let mut width = 0usize;
    let mut first = true;
    for ch in text.chars() {
        if !first {
            width += 1;
        }
        width += pixel_char_width_small(ch);
        first = false;
    }
    width
}

fn measure_pixel_text(text: &str) -> usize {
    let mut width = 0usize;
    let mut first = true;
    for ch in text.chars() {
        if !first {
            width += 1;
        }
        width += pixel_char_width(ch);
        first = false;
    }
    width
}

fn draw_pixel_text(rgba: &mut [u8], width: usize, x: usize, y: usize, text: &str, color: [u8; 4]) {
    let mut cursor_x = x;
    for ch in text.chars() {
        if ch == ' ' {
            cursor_x += 4;
            continue;
        }
        let glyph = pixel_glyph(ch);
        for (row_idx, row) in glyph.iter().enumerate() {
            for (col_idx, bit) in row.chars().enumerate() {
                if bit == '1' {
                    set_px(rgba, width, cursor_x + col_idx, y + row_idx, color);
                }
            }
        }
        cursor_x += pixel_char_width(ch) + 1;
    }
}

fn draw_pixel_text_small(
    rgba: &mut [u8],
    width: usize,
    x: usize,
    y: usize,
    text: &str,
    color: [u8; 4],
) {
    let mut cursor_x = x;
    for ch in text.chars() {
        if ch == ' ' {
            cursor_x += 3;
            continue;
        }
        let glyph = pixel_glyph_small(ch);
        for (row_idx, row) in glyph.iter().enumerate() {
            for (col_idx, bit) in row.chars().enumerate() {
                if bit == '1' {
                    set_px(rgba, width, cursor_x + col_idx, y + row_idx, color);
                }
            }
        }
        cursor_x += pixel_char_width_small(ch) + 1;
    }
}

fn pixel_glyph(ch: char) -> [&'static str; 7] {
    match ch {
        'A' => [
            "01110", "10001", "10001", "11111", "10001", "10001", "10001",
        ],
        'C' => [
            "01111", "10000", "10000", "10000", "10000", "10000", "01111",
        ],
        'D' => [
            "11110", "10001", "10001", "10001", "10001", "10001", "11110",
        ],
        'E' => [
            "11111", "10000", "10000", "11110", "10000", "10000", "11111",
        ],
        'O' => [
            "01110", "10001", "10001", "10001", "10001", "10001", "01110",
        ],
        'P' => [
            "11110", "10001", "10001", "11110", "10000", "10000", "10000",
        ],
        'R' => [
            "11110", "10001", "10001", "11110", "10100", "10010", "10001",
        ],
        'S' => [
            "01111", "10000", "10000", "01110", "00001", "00001", "11110",
        ],
        'T' => [
            "11111", "00100", "00100", "00100", "00100", "00100", "00100",
        ],
        'U' => [
            "10001", "10001", "10001", "10001", "10001", "10001", "01110",
        ],
        'Y' => [
            "10001", "10001", "01010", "00100", "00100", "00100", "00100",
        ],
        '-' => [
            "00000", "00000", "00000", "11111", "00000", "00000", "00000",
        ],
        _ => [
            "00000", "00000", "00000", "00000", "00000", "00000", "00000",
        ],
    }
}

fn pixel_char_width(ch: char) -> usize {
    match ch {
        ' ' => 3,
        '-' => 5,
        _ => 5,
    }
}

fn pixel_glyph_small(ch: char) -> [&'static str; 5] {
    match ch {
        'A' => ["010", "101", "111", "101", "101"],
        'C' => ["011", "100", "100", "100", "011"],
        'D' => ["110", "101", "101", "101", "110"],
        'E' => ["111", "110", "111", "100", "111"],
        'I' => ["111", "010", "010", "010", "111"],
        'O' => ["111", "101", "101", "101", "111"],
        'P' => ["110", "101", "110", "100", "100"],
        'R' => ["110", "101", "110", "101", "101"],
        'S' => ["011", "100", "010", "001", "110"],
        'T' => ["111", "010", "010", "010", "010"],
        'U' => ["101", "101", "101", "101", "111"],
        'Y' => ["101", "101", "010", "010", "010"],
        '-' => ["000", "000", "111", "000", "000"],
        _ => ["000", "000", "000", "000", "000"],
    }
}

fn pixel_char_width_small(ch: char) -> usize {
    match ch {
        ' ' => 2,
        '-' => 3,
        _ => 3,
    }
}

fn solid_canvas(width: usize, height: usize, color: [u8; 4]) -> Vec<u8> {
    let mut rgba = vec![0; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            set_px(&mut rgba, width, x, y, color);
        }
    }
    rgba
}

fn upscale_rgba(rgba: &[u8], width: usize, height: usize, scale: usize) -> Vec<u8> {
    let scale = scale.max(1);
    let up_width = width * scale;
    let up_height = height * scale;
    let mut out = vec![0; up_width * up_height * 4];

    for y in 0..height {
        for x in 0..width {
            let src = (y * width + x) * 4;
            let color = [rgba[src], rgba[src + 1], rgba[src + 2], rgba[src + 3]];
            for oy in 0..scale {
                for ox in 0..scale {
                    set_px(&mut out, up_width, x * scale + ox, y * scale + oy, color);
                }
            }
        }
    }

    out
}

fn outline_rect(
    rgba: &mut [u8],
    width: usize,
    x: usize,
    y: usize,
    rect_width: usize,
    rect_height: usize,
    color: [u8; 4],
) {
    fill_rect(rgba, width, x, y, rect_width, 1, color);
    fill_rect(
        rgba,
        width,
        x,
        y + rect_height.saturating_sub(1),
        rect_width,
        1,
        color,
    );
    fill_rect(rgba, width, x, y, 1, rect_height, color);
    fill_rect(
        rgba,
        width,
        x + rect_width.saturating_sub(1),
        y,
        1,
        rect_height,
        color,
    );
}

fn fill_rect(
    rgba: &mut [u8],
    width: usize,
    x: usize,
    y: usize,
    rect_width: usize,
    rect_height: usize,
    color: [u8; 4],
) {
    let total_height = rgba.len() / 4 / width;
    let max_x = (x + rect_width).min(width);
    let max_y = (y + rect_height).min(total_height);
    for py in y..max_y {
        for px in x..max_x {
            set_px(rgba, width, px, py, color);
        }
    }
}

fn set_px(rgba: &mut [u8], width: usize, x: usize, y: usize, color: [u8; 4]) {
    let index = (y * width + x) * 4;
    if index + 3 >= rgba.len() {
        return;
    }
    rgba[index] = color[0];
    rgba[index + 1] = color[1];
    rgba[index + 2] = color[2];
    rgba[index + 3] = color[3];
}
