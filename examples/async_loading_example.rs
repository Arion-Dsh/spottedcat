use spottedcat::{Context, DrawOption, Image, Pt, Spot, WindowConfig, run};
use std::thread;
use std::time::Duration;

struct AsyncLoadingExample {
    image1: Option<Image>,
    image2: Option<Image>,
    loading_timer: f32,
}

impl Spot for AsyncLoadingExample {
    fn initialize(_context: &mut Context) -> Self {
        let mut example = Self {
            image1: None,
            image2: None,
            loading_timer: 0.0,
        };

        // Simulate background loading for image 1
        thread::spawn(|| {
            // Simulate IO/Decode time
            thread::sleep(Duration::from_secs(2));

            let width = 300;
            let height = 300;
            let mut rgba = vec![255u8; width * height * 4];
            for i in 0..width * height {
                rgba[i * 4] = 255; // R
                rgba[i * 4 + 1] = 100; // G
                rgba[i * 4 + 3] = 255; // A
            }

            // Image::new_from_rgba8 is now thread-safe and fast!
            // It only allocates an ID and queues the data.
            let img = Image::new_from_rgba8(Pt::from(300.0), Pt::from(300.0), &rgba).unwrap();

            // We can even create sub-images immediately, even if parent is not yet on GPU
            let _sub = Image::sub_image(
                img,
                spottedcat::Bounds::new(
                    Pt::from(0.0),
                    Pt::from(0.0),
                    Pt::from(100.0),
                    Pt::from(100.0),
                ),
            )
            .unwrap();

            // Note: In a real app, you'd send this ID back to the main thread via a channel
            // or store it in an Arc/Mutex. For this example, we'll just cheat a bit
            // and assume it's assigned to a global or shared state (not shown here for brevity of the example).
        });

        // For this example's structure, let's just create them immediately in initialize
        // but they will stay "Pending" for a few ms until the first frame's compress_assets.

        // Image 1: Normal initialization (starts as Pending)
        let rgba1 = vec![255u8; 100 * 100 * 4];
        example.image1 =
            Some(Image::new_from_rgba8(Pt::from(100.0), Pt::from(100.0), &rgba1).unwrap());

        example
    }

    fn update(&mut self, _context: &mut Context, dt: Duration) {
        self.loading_timer += dt.as_secs_f32();

        // Simulate a second image being registered late (e.g. triggered by user)
        if self.loading_timer > 3.0 && self.image2.is_none() {
            println!("Context: Registering second image late...");
            let rgba2 = vec![100u8; 100 * 100 * 4];
            self.image2 =
                Some(Image::new_from_rgba8(Pt::from(100.0), Pt::from(100.0), &rgba2).unwrap());
        }
    }

    fn draw(&mut self, context: &mut Context) {
        // Draw Image 1
        if let Some(img) = self.image1 {
            if img.is_ready() {
                img.draw(
                    context,
                    DrawOption::default().with_position([Pt::from(50.0), Pt::from(50.0)]),
                );
            } else {
                // Use is_ready() to show a placeholder or loading state
                println!("Image 1 is still Pending GPU upload...");
            }
        }

        // Draw Image 2
        if let Some(img) = self.image2 {
            if img.is_ready() {
                img.draw(
                    context,
                    DrawOption::default().with_position([Pt::from(200.0), Pt::from(50.0)]),
                );
            } else {
                println!("Image 2 is still Pending GPU upload...");
            }
        }
    }

    fn remove(&self) {}
}

fn main() {
    run::<AsyncLoadingExample>(WindowConfig {
        title: "Async Asset Loading Example".to_string(),
        ..Default::default()
    });
}
