fn main() {
    struct DemoSpot {
        image: spot::Image,
        image_sub: spot::Image,
        image_clone: spot::Image,
    }

    impl spot::Spot for DemoSpot {
        fn initialize(_context: spot::Context) -> Self {
            let mut rgba = vec![0u8; 20 * 20 * 4];
            for y in 0..20u32 {
                for x in 0..20u32 {
                    let i = ((y * 20 + x) * 4) as usize;
                    let on = ((x / 5 + y / 5) % 2) == 0;
                    rgba[i] = if on { 255 } else { 30 };
                    rgba[i + 1] = if on { 80 } else { 200 };
                    rgba[i + 2] = if on { 80 } else { 255 };
                    rgba[i + 3] = 255;
                }
            }
            let image = spot::Image::new_from_rgba8(20, 20, &rgba).expect("failed to create test image");
            let image_sub = spot::Image::sub_image(image, spot::Bounds::new(5, 5, 10, 10))
                .expect("failed to create sub image");
            let image_clone = spot::Image::new_from_image(image)
                .expect("failed to create image from image");

            Self {
                image,
                image_sub,
                image_clone,
            }
        }

        fn draw(&mut self, context: &mut spot::Context) {
            let mut opts = spot::DrawOptions::default();
            opts.position = [50.0, 50.0];
            opts.size = [200.0, 200.0];
            self.image.draw(context, opts);

            let mut opts = spot::DrawOptions::default();
            opts.position = [300.0, 50.0];
            opts.size = [200.0, 200.0];
            self.image_sub.draw(context, opts);

            let mut opts = spot::DrawOptions::default();
            opts.position = [550.0, 50.0];
            opts.size = [200.0, 200.0];
            self.image_clone.draw(context, opts);
        }

        fn update(&self, _event: spot::Event) {}

        fn remove(&self) {}
    }

    spot::run::<DemoSpot>();
}
