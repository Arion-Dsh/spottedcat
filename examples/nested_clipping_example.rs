use spottedcat::{Spot, Context, DrawOption, Image, Pt, WindowConfig, Text, load_font_from_file};
use std::time::Duration;

struct NestedClippingScene {
    grandpa: Image,
    father: Image,
    son: Image,
    text: Text,
}

impl Spot for NestedClippingScene {
    fn initialize(_context: &mut Context) -> Self {
        let grandpa_rgba = vec![100, 100, 100, 255].repeat(300 * 300);
        let grandpa = Image::new_from_rgba8(Pt::from(300.0), Pt::from(300.0), &grandpa_rgba).unwrap();
        
        let father_rgba = vec![0, 0, 255, 255].repeat(200 * 200);
        let father = Image::new_from_rgba8(Pt::from(200.0), Pt::from(200.0), &father_rgba).unwrap();
        
        let son_rgba = vec![255, 0, 0, 255].repeat(100 * 100);
        let son = Image::new_from_rgba8(Pt::from(100.0), Pt::from(100.0), &son_rgba).unwrap();

        let font = load_font_from_file("assets/DejaVuSans.ttf").unwrap();
        let text = Text::new("Clipped Text", font)
            .with_font_size(Pt::from(24.0))
            .with_color([1.0, 1.0, 1.0, 1.0]);

        Self { grandpa, father, son, text }
    }

    fn draw(&mut self, context: &mut Context) {
        // 1. Grandpa at (50, 50)
        let mut grandpa_opts = DrawOption::default();
        grandpa_opts.set_position([Pt::from(50.0), Pt::from(50.0)]);
        self.grandpa.draw(context, grandpa_opts);

        // 2. Father relative to Grandpa at (150, 150)
        let mut father_opts = DrawOption::default();
        father_opts.set_position([Pt::from(150.0), Pt::from(150.0)]);
        
        // draw_image returns the final DrawOption used for Father,
        // which includes absolute screen position and clipping.
        let father_screen_opts = self.grandpa.draw_image(
            context,
            grandpa_opts,
            self.father,
            father_opts,
        );

        // 3. Son relative to Father at (100, 100)
        let mut son_opts = DrawOption::default();
        son_opts.set_position([Pt::from(100.0), Pt::from(100.0)]);
        
        // Use father_screen_opts as parent_options to draw Son
        self.father.draw_image(
            context,
            father_screen_opts,
            self.son,
            son_opts,
        );

        // 4. Text relative to Father at (50, 50)
        // This is inside Father and Grandpa.
        let mut text_opts = DrawOption::default();
        text_opts.set_position([Pt::from(50.0), Pt::from(50.0)]);
        self.father.draw_text(context, father_screen_opts, self.text.clone(), text_opts);
    }

    fn update(&mut self, _context: &mut Context, _dt: Duration) {}
    fn remove(&self) {}
}

fn main() {
    let mut config = WindowConfig::default();
    config.title = "Nested Clipping Example".to_string();
    spottedcat::run::<NestedClippingScene>(config);
}
