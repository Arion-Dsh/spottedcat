use spottedcat::{Context, DrawOption, Image, Pt, Spot, WindowConfig, run};
use std::time::Duration;

struct AsyncLoadingExample {
    image1: Option<Image>,
    image2: Option<Image>,
    loading_timer: f32,
}

impl Spot for AsyncLoadingExample {
    fn initialize(ctx: &mut Context) -> Self {
        let mut example = Self {
            image1: None,
            image2: None,
            loading_timer: 0.0,
        };

        // Asset registration happens on the thread that owns the context.
        let rgba1 = vec![255u8; 100 * 100 * 4];
        example.image1 = Some(Image::new(ctx, Pt::from(100.0), Pt::from(100.0), &rgba1).unwrap());

        example
    }

    fn update(&mut self, ctx: &mut Context, dt: Duration) {
        self.loading_timer += dt.as_secs_f32();

        // Register a second image after startup.
        if self.loading_timer > 3.0 && self.image2.is_none() {
            println!("Context: Registering second image late...");
            let rgba2 = vec![100u8; 100 * 100 * 4];
            self.image2 = Some(Image::new(ctx, Pt::from(100.0), Pt::from(100.0), &rgba2).unwrap());
        }
    }

    fn draw(&mut self, ctx: &mut Context, screen: spottedcat::Image) {
        if let Some(img) = self.image1 {
            if img.is_ready(ctx) {
                screen.draw(
                    ctx,
                    &img,
                    DrawOption::default().with_position([Pt::from(50.0), Pt::from(50.0)]),
                );
            } else {
                println!("Image 1 is still Pending GPU upload...");
            }
        }

        if let Some(img) = self.image2 {
            if img.is_ready(ctx) {
                screen.draw(
                    ctx,
                    &img,
                    DrawOption::default().with_position([Pt::from(200.0), Pt::from(50.0)]),
                );
            } else {
                println!("Image 2 is still Pending GPU upload...");
            }
        }
    }
}

fn main() {
    run::<AsyncLoadingExample>(WindowConfig {
        title: "Async Asset Loading Example".to_string(),
        ..Default::default()
    });
}
