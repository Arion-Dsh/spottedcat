use spottedcat::{
    Context, DrawOption, Image, Key, MouseButton, OneShotSplash, Pt, Spot, Text, TouchPhase,
    WindowConfig,
};
use wasm_bindgen::prelude::*;

const BIRD_X: f32 = 118.0;
const BIRD_SIZE: f32 = 28.0;
const GRAVITY: f32 = 920.0;
const JUMP_VELOCITY: f32 = -330.0;
const PIPE_WIDTH: f32 = 58.0;
const PIPE_GAP: f32 = 138.0;
const PIPE_SPEED: f32 = 175.0;
const PIPE_COUNT: usize = 3;
const GAME_WIDTH: f32 = 640.0;
const GAME_HEIGHT: f32 = 480.0;

#[derive(Clone, Copy)]
struct Pipe {
    x: f32,
    gap_y: f32,
    scored: bool,
}

struct FlappyCat {
    bird: Image,
    pipe: Image,
    ground: Image,
    font_id: u32,
    bird_y: f32,
    bird_vy: f32,
    pipes: [Pipe; PIPE_COUNT],
    score: u32,
    game_over: bool,
    time: f32,
}

impl Spot for FlappyCat {
    fn initialize(ctx: &mut Context) -> Self {
        const FONT: &[u8] = include_bytes!("../../../../assets/DejaVuSans.ttf");
        let font_id = spottedcat::register_font(ctx, FONT.to_vec());

        let bird = solid_image(ctx, 28, 28, [241, 126, 72, 255], |pixels, w| {
            rect(pixels, w, 5, 4, 18, 18, [231, 152, 96, 255]);
            rect(pixels, w, 8, 8, 4, 4, [79, 241, 217, 255]);
            rect(pixels, w, 17, 8, 4, 4, [79, 241, 217, 255]);
            rect(pixels, w, 13, 14, 4, 3, [74, 40, 34, 255]);
            rect(pixels, w, 9, 20, 10, 3, [243, 219, 194, 255]);
            rect(pixels, w, 4, 12, 3, 8, [166, 92, 46, 255]);
            rect(pixels, w, 22, 12, 3, 8, [166, 92, 46, 255]);
        });

        let pipe = solid_image(ctx, PIPE_WIDTH as usize, 1, [79, 241, 217, 255], |_, _| {});
        let ground = solid_image(ctx, 1, 24, [41, 28, 24, 255], |pixels, w| {
            rect(pixels, w, 0, 0, 1, 5, [241, 126, 72, 255]);
        });

        let mut game = Self {
            bird,
            pipe,
            ground,
            font_id,
            bird_y: 180.0,
            bird_vy: 0.0,
            pipes: [
                Pipe {
                    x: 360.0,
                    gap_y: 180.0,
                    scored: false,
                },
                Pipe {
                    x: 560.0,
                    gap_y: 245.0,
                    scored: false,
                },
                Pipe {
                    x: 760.0,
                    gap_y: 150.0,
                    scored: false,
                },
            ],
            score: 0,
            game_over: false,
            time: 0.0,
        };
        game.reset(ctx);
        game
    }

    fn update(&mut self, ctx: &mut Context, dt: std::time::Duration) {
        let dt = dt.as_secs_f32().min(1.0 / 20.0);
        self.time += dt;

        let flap = spottedcat::key_pressed(ctx, Key::Space)
            || spottedcat::key_pressed(ctx, Key::Up)
            || spottedcat::mouse_button_pressed(ctx, MouseButton::Left)
            || spottedcat::touches(ctx)
                .iter()
                .any(|touch| touch.phase == TouchPhase::Started);

        if self.game_over {
            if flap {
                self.reset(ctx);
            }
            return;
        }

        if flap {
            self.bird_vy = JUMP_VELOCITY;
            spottedcat::play_sine(ctx, 620.0, 0.08);
        }

        self.bird_vy += GRAVITY * dt;
        self.bird_y += self.bird_vy * dt;

        let (w, h) = spottedcat::window_size(ctx);
        let width = w.as_f32();
        let height = h.as_f32();
        let ground_y = height - 36.0;

        for i in 0..self.pipes.len() {
            self.pipes[i].x -= PIPE_SPEED * dt;

            if !self.pipes[i].scored && self.pipes[i].x + PIPE_WIDTH < BIRD_X {
                self.pipes[i].scored = true;
                self.score += 1;
                spottedcat::play_sine(ctx, 880.0, 0.06);
            }

            if self.pipes[i].x + PIPE_WIDTH < -20.0 {
                let farthest = self
                    .pipes
                    .iter()
                    .map(|pipe| pipe.x)
                    .fold(0.0_f32, f32::max);
                self.pipes[i] = Pipe {
                    x: farthest + 210.0,
                    gap_y: 130.0 + ((self.time * 1.7 + i as f32 * 2.1).sin() + 1.0) * 75.0,
                    scored: false,
                };
            }
        }

        if self.bird_y < 0.0 || self.bird_y + BIRD_SIZE > ground_y {
            self.crash(ctx);
        }

        for pipe in self.pipes {
            if overlaps(
                BIRD_X,
                self.bird_y,
                BIRD_SIZE,
                BIRD_SIZE,
                pipe.x,
                0.0,
                PIPE_WIDTH,
                pipe.gap_y - PIPE_GAP * 0.5,
            ) || overlaps(
                BIRD_X,
                self.bird_y,
                BIRD_SIZE,
                BIRD_SIZE,
                pipe.x,
                pipe.gap_y + PIPE_GAP * 0.5,
                PIPE_WIDTH,
                ground_y - (pipe.gap_y + PIPE_GAP * 0.5),
            ) {
                self.crash(ctx);
                break;
            }
        }

        if width < 300.0 {
            self.crash(ctx);
        }
    }

    fn draw(&mut self, ctx: &mut Context, screen: Image) {
        let (w, h) = spottedcat::window_size(ctx);
        let width = w.as_f32();
        let height = h.as_f32();
        let ground_y = height - 36.0;

        draw_text(
            ctx,
            screen,
            self.font_id,
            "Flappy Cat",
            22.0,
            [0.95, 0.93, 0.88, 1.0],
            18.0,
            18.0,
        );

        for pipe in self.pipes {
            let top_h = (pipe.gap_y - PIPE_GAP * 0.5).max(0.0);
            let bottom_y = pipe.gap_y + PIPE_GAP * 0.5;
            let bottom_h = (ground_y - bottom_y).max(0.0);

            screen.draw(
                ctx,
                &self.pipe,
                DrawOption::default()
                    .with_position([Pt::from(pipe.x), Pt::from(0.0)])
                    .with_scale([1.0, top_h]),
            );
            screen.draw(
                ctx,
                &self.pipe,
                DrawOption::default()
                    .with_position([Pt::from(pipe.x), Pt::from(bottom_y)])
                    .with_scale([1.0, bottom_h]),
            );
        }

        screen.draw(
            ctx,
            &self.bird,
            DrawOption::default().with_position([Pt::from(BIRD_X), Pt::from(self.bird_y)]),
        );

        screen.draw(
            ctx,
            &self.ground,
            DrawOption::default()
                .with_position([Pt::from(0.0), Pt::from(ground_y)])
                .with_scale([width, 1.0]),
        );

        draw_text(
            ctx,
            screen,
            self.font_id,
            &format!("Score {}", self.score),
            20.0,
            [0.7, 0.95, 1.0, 1.0],
            18.0,
            ground_y + 9.0,
        );

        let help = if self.game_over {
            "Space / click / touch to restart"
        } else {
            "Space / click / touch to flap"
        };
        draw_text(
            ctx,
            screen,
            self.font_id,
            help,
            14.0,
            [0.9, 0.9, 0.9, 1.0],
            18.0,
            height - 22.0,
        );

        if self.game_over {
            draw_centered_text(
                ctx,
                screen,
                self.font_id,
                "GAME OVER",
                42.0,
                [1.0, 0.72, 0.48, 1.0],
                width * 0.5,
                height * 0.40,
            );
            draw_centered_text(
                ctx,
                screen,
                self.font_id,
                "Flap to try again",
                22.0,
                [0.9, 0.95, 1.0, 1.0],
                width * 0.5,
                height * 0.52,
            );
        }
    }

    fn remove(&mut self, ctx: &mut Context) {
        spottedcat::unregister_font(ctx, self.font_id);
    }
}

impl FlappyCat {
    fn reset(&mut self, ctx: &mut Context) {
        let (w, h) = spottedcat::window_size(ctx);
        let width = w.as_f32().max(GAME_WIDTH);
        let height = h.as_f32().max(GAME_HEIGHT);
        self.bird_y = height * 0.42;
        self.bird_vy = 0.0;
        self.score = 0;
        self.game_over = false;
        self.time = 0.0;

        for i in 0..PIPE_COUNT {
            self.pipes[i] = Pipe {
                x: width + 120.0 + i as f32 * 210.0,
                gap_y: 135.0 + ((i * 47) % 120) as f32,
                scored: false,
            };
        }
    }

    fn crash(&mut self, ctx: &mut Context) {
        if !self.game_over {
            self.game_over = true;
            spottedcat::play_sine(ctx, 180.0, 0.16);
        }
    }
}

fn draw_text(
    ctx: &mut Context,
    screen: Image,
    font_id: u32,
    text: &str,
    size: f32,
    color: [f32; 4],
    x: f32,
    y: f32,
) {
    let text = Text::new(text, font_id)
        .with_font_size(Pt::from(size))
        .with_color(color);
    screen.draw(
        ctx,
        &text,
        DrawOption::default().with_position([Pt::from(x), Pt::from(y)]),
    );
}

fn draw_centered_text(
    ctx: &mut Context,
    screen: Image,
    font_id: u32,
    text: &str,
    size: f32,
    color: [f32; 4],
    center_x: f32,
    y: f32,
) {
    let text = Text::new(text, font_id)
        .with_font_size(Pt::from(size))
        .with_color(color);
    let (text_width, _text_height) = spottedcat::text::measure(ctx, &text);
    screen.draw(
        ctx,
        &text,
        DrawOption::default()
            .with_position([Pt::from(center_x - text_width * 0.5), Pt::from(y)]),
    );
}

fn solid_image<F>(ctx: &mut Context, width: usize, height: usize, color: [u8; 4], paint: F) -> Image
where
    F: FnOnce(&mut [u8], usize),
{
    let mut pixels = vec![0; width * height * 4];
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.copy_from_slice(&color);
    }
    paint(&mut pixels, width);
    Image::new(ctx, Pt::from(width as f32), Pt::from(height as f32), &pixels).unwrap()
}

fn rect(pixels: &mut [u8], width: usize, x: usize, y: usize, w: usize, h: usize, color: [u8; 4]) {
    for yy in y..(y + h) {
        for xx in x..(x + w) {
            let i = (yy * width + xx) * 4;
            pixels[i..i + 4].copy_from_slice(&color);
        }
    }
}

fn overlaps(ax: f32, ay: f32, aw: f32, ah: f32, bx: f32, by: f32, bw: f32, bh: f32) -> bool {
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

#[wasm_bindgen]
pub fn run_flappy_cat() {
    console_error_panic_hook::set_once();

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    let config = {
        let mut config = WindowConfig::default();
        config.canvas_id = Some("spot-canvas".to_string());
        config.width = Pt::from(GAME_WIDTH);
        config.height = Pt::from(GAME_HEIGHT);
        config.title = "Flappy Cat".to_string();
        config
    };

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    let config = WindowConfig::default();

    spottedcat::run::<OneShotSplash<FlappyCat>>(config);
}
