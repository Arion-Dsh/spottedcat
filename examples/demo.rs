fn main() {
    use spottedcat::{Bounds, Context, DrawOption, Image, Pt, Spot, WindowConfig, run};

    struct DemoSpot {
        tree: Image,
        image: Image,
        image_sub: Image,
        image_clone: Image,
    }

    impl Spot for DemoSpot {
        fn initialize(_context: &mut Context) -> Self {
            const TREE_PNG: &[u8] = include_bytes!("../assets/happy-tree.png");
            let decoded = image::load_from_memory(TREE_PNG).expect("failed to decode happy-tree.png");
            let rgba = decoded.to_rgba8();
            let (w, h) = (rgba.width(), rgba.height());
            let tree = Image::new_from_rgba8(Pt::from(w), Pt::from(h), rgba.as_raw())
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
                Image::new_from_rgba8(Pt::from(20.0), Pt::from(20.0), &rgba).expect("failed to create test image");
            let image_sub = Image::sub_image(image, Bounds::new(Pt::from(5.0), Pt::from(5.0), Pt::from(10.0), Pt::from(10.0)))
                .expect("failed to create sub image");
            let image_clone = Image::new_from_image(image).expect("failed to create image from image");

            Self {
                tree,
                image,
                image_sub,
                image_clone,
            }
        }

        fn draw(&mut self, context: &mut Context) {
            let mut opts = DrawOption::default();
            opts.position = [Pt::from(20.0), Pt::from(300.0)];
            self.tree.draw(context, opts);

            let mut opts = DrawOption::default();
            opts.position = [Pt::from(50.0), Pt::from(50.0)];
            opts.scale = [10.0, 10.0];
            self.image.draw(context, opts);

            let mut opts = DrawOption::default();
            opts.position = [Pt::from(300.0), Pt::from(50.0)];
            opts.scale = [20.0, 20.0];
            self.image_sub.draw(context, opts);

            let mut opts = DrawOption::default();
            opts.position = [Pt::from(550.0), Pt::from(50.0)];
            opts.scale = [10.0, 10.0];
            self.image_clone.draw(context, opts);
        }

        fn update(&mut self, context: &mut Context, _dt: std::time::Duration) {
            let (w, h) = spottedcat::window_size(context);
            println!("window size: {}x{}", w, h);
        }

        fn remove(&self) {}
    }

    run::<DemoSpot>(WindowConfig::default());
}
