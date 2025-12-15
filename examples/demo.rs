fn main() {
    struct DemoSpot {
        tree: spot::Image,
        image: spot::Image,
        image_sub: spot::Image,
        image_clone: spot::Image,
    }

    impl spot::Spot for DemoSpot {
        fn initialize(_context: spot::Context) -> Self {
            const TREE_PNG: &[u8] = include_bytes!("../assets/happy-tree.png");
            let decoded = image::load_from_memory(TREE_PNG).expect("failed to decode happy-tree.png");
            let rgba = decoded.to_rgba8();
            let (w, h) = (rgba.width(), rgba.height());
            let tree = spot::Image::new_from_rgba8(w, h, rgba.as_raw())
                .expect("failed to create happy-tree image");

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
            let image =
                spot::Image::new_from_rgba8(20, 20, &rgba).expect("failed to create test image");
            let image_sub = spot::Image::sub_image(image, spot::Bounds::new(5, 5, 10, 10))
                .expect("failed to create sub image");
            let image_clone =
                spot::Image::new_from_image(image).expect("failed to create image from image");

            Self {
                tree,
                image,
                image_sub,
                image_clone,
            }
        }

        fn draw(&mut self, context: &mut spot::Context) {
            let mut opts = spot::ImageDrawOptions::default();
            opts.position = [spot::Pt(20.0), spot::Pt(300.0)];
            self.tree.draw(context, opts);

            let mut opts = spot::ImageDrawOptions::default();
            opts.position = [spot::Pt(50.0), spot::Pt(50.0)];
            opts.scale = [10.0, 10.0];
            self.image.draw(context, opts);

            let mut opts = spot::ImageDrawOptions::default();
            opts.position = [spot::Pt(300.0), spot::Pt(50.0)];
            opts.scale = [20.0, 20.0];
            self.image_sub.draw(context, opts);

            let mut opts = spot::ImageDrawOptions::default();
            opts.position = [spot::Pt(550.0), spot::Pt(50.0)];
            opts.scale = [10.0, 10.0];
            self.image_clone.draw(context, opts);
        }

        fn update(&mut self, _context: &mut spot::Context, _dt: std::time::Duration) {}

        fn remove(&self) {}
    }

    spot::run::<DemoSpot>(spot::WindowConfig::default());
}
