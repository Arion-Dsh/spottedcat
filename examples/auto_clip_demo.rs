use spottedcat::{Spot, Context, DrawOption, Image, Pt, WindowConfig, Text, load_font_from_file};
use std::time::Duration;

struct AutoClipDemo {
    container: Image,
    box_red: Image,
    box_blue: Image,
    font_text: Text,
    timer: f32,
}

impl Spot for AutoClipDemo {
    fn initialize(_context: &mut Context) -> Self {
        // Create a large dark container
        let container_rgba = vec![40, 44, 52, 255].repeat(400 * 400);
        let container = Image::new_from_rgba8(Pt::from(400.0), Pt::from(400.0), &container_rgba).unwrap();
        
        // Create child boxes
        let red_rgba = vec![255, 100, 100, 255].repeat(150 * 150);
        let box_red = Image::new_from_rgba8(Pt::from(150.0), Pt::from(150.0), &red_rgba).unwrap();
        
        let blue_rgba = vec![100, 100, 255, 255].repeat(100 * 100);
        let box_blue = Image::new_from_rgba8(Pt::from(100.0), Pt::from(100.0), &blue_rgba).unwrap();

        let font = load_font_from_file("assets/DejaVuSans.ttf").unwrap();
        let font_text = Text::new("Auto Clipped Text", font)
            .with_font_size(Pt::from(20.0))
            .with_color([1.0, 1.0, 1.0, 1.0]);

        Self { 
            container, 
            box_red, 
            box_blue, 
            font_text,
            timer: 0.0,
        }
    }

    fn draw(&mut self, context: &mut Context) {
        // Center the container
        let win_size = context.window_logical_size();
        let container_pos = [
            (win_size.0 - Pt::from(400.0)) / 2.0,
            (win_size.1 - Pt::from(400.0)) / 2.0,
        ];
        
        let container_opts = DrawOption::default().with_position(container_pos);
        self.container.draw(context, container_opts);

        // Calculate moving positions for children
        let offset_x = self.timer.cos() * 150.0;
        let offset_y = self.timer.sin() * 150.0;

        // Draw children relative to container, with an explicit clip scope
        let red_opts = DrawOption::default()
            .with_position([Pt::from(125.0 + offset_x), Pt::from(125.0 + offset_y)]);

        self.container.with_clip_scope(context, container_opts, |context| {
            self.box_red.draw(context, red_opts);

            // Nested clipping: Draw blue box relative to red box
            let blue_opts = DrawOption::default()
                .with_position([Pt::from(25.0), Pt::from(25.0)]);

            self.box_red.with_clip_scope(context, red_opts, |context| {
                self.box_blue.draw(context, blue_opts);

                // Draw text relative to blue box
                let text_opts = DrawOption::default()
                    .with_position([Pt::from(10.0), Pt::from(40.0)]);

                self.box_blue.with_clip_scope(context, blue_opts, |context| {
                    self.font_text.clone().draw(context, text_opts);
                });
            });
        });
    }

    fn update(&mut self, _context: &mut Context, dt: Duration) {
        self.timer += dt.as_secs_f32();
    }
    
    fn remove(&self) {}
}

fn main() {
    let mut config = WindowConfig::default();
    config.title = "Auto Clipping & Relative Positioning Demo".to_string();
    spottedcat::run::<AutoClipDemo>(config);
}
