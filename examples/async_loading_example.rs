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

        // Note: Asset registration must now happen on the thread owning the Context.
        // Image 1: Normal initialization (starts as Pending)
        let rgba1 = vec![255u8; 100 * 100 * 4];
        example.image1 =
            Some(spottedcat::image::create(ctx, Pt::from(100.0), Pt::from(100.0), &rgba1).unwrap());

        example
    }

    fn update(&mut self, ctx: &mut Context, dt: Duration) {
        self.loading_timer += dt.as_secs_f32();

        // Simulate a second image being registered late (e.g. triggered by user)
        if self.loading_timer > 3.0 && self.image2.is_none() {
            println!("Context: Registering second image late...");
            let rgba2 = vec![100u8; 100 * 100 * 4];
            self.image2 = Some(
                spottedcat::image::create(ctx, Pt::from(100.0), Pt::from(100.0), &rgba2).unwrap(),
            );
        }
    }

    fn draw(&mut self, ctx: &mut Context) {
        // Draw Image 1
        if let Some(img) = self.image1 {
            if spottedcat::image::is_ready(ctx, img) {
                spottedcat::image::draw(
                    ctx,
                    img,
                    DrawOption::default().with_position([Pt::from(50.0), Pt::from(50.0)]),
                );
            } else {
                // Use is_ready() to show a placeholder or loading state
                println!("Image 1 is still Pending GPU upload...");
            }
        }

        // Draw Image 2
        if let Some(img) = self.image2 {
            if spottedcat::image::is_ready(ctx, img) {
                spottedcat::image::draw(
                    ctx,
                    img,
                    DrawOption::default().with_position([Pt::from(200.0), Pt::from(50.0)]),
                );
            } else {
                println!("Image 2 is still Pending GPU upload...");
            }
        }
    }

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    run::<AsyncLoadingExample>(WindowConfig {
        title: "Async Asset Loading Example".to_string(),
        ..Default::default()
    });
}
