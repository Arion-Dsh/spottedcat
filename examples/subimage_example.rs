fn main() {
    use spottedcat::{Bounds, Context, Image, ImageDrawOptions, Pt, Spot, WindowConfig, run};

    struct SubImageExample {
        tree: Image,
        tree_sub: Image,
    }

    impl Spot for SubImageExample {
        fn initialize(_context: &mut Context) -> Self {
            const TREE_PNG: &[u8] = include_bytes!("../assets/happy-tree.png");
            let decoded = image::load_from_memory(TREE_PNG).expect("failed to decode happy-tree.png");
            let rgba = decoded.to_rgba8();
            let (w, h) = (rgba.width(), rgba.height());
            let tree = Image::new_from_rgba8(w, h, rgba.as_raw())
                .expect("failed to create happy-tree image");

            let crop_w = (w / 2).max(1);
            let crop_h = (h / 2).max(1);
            let tree_sub = Image::sub_image(tree, Bounds::new(0, 0, crop_w, crop_h))
                .expect("failed to create sub image");

            Self { tree, tree_sub }
        }

        fn draw(&mut self, context: &mut Context) {
            let mut opts = ImageDrawOptions::default();
            opts.position = [Pt(20), Pt(20)];
            opts.scale = [1.0, 1.0];
            self.tree.draw(context, opts);

            let mut opts = ImageDrawOptions::default();
            opts.position = [Pt(420), Pt(20)];
            opts.scale = [1.0, 1.0];
            self.tree_sub.draw(context, opts);
        }

        fn update(&mut self, _context: &mut Context, _dt: std::time::Duration) {}

        fn remove(&self) {}
    }

    let mut cfg = WindowConfig::default();
    cfg.title = "subimage example".to_string();
    run::<SubImageExample>(cfg);
}
