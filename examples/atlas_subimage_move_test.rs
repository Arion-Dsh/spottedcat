use spottedcat::{Context, DrawOption, Image, Pt, Spot, WindowConfig, run};

/// Tests three layers of sub-image + render-target nesting:
///   happy-tree (original)
///     └─ canvas_a (render target, 512x512)  ← c1 (sub of original) drawn here
///          └─ canvas_b (render target, 256x256)  ← c2 (sub of c1) drawn here
///               └─ canvas_c (render target, 128x128)  ← c3 (sub of c2) drawn here
///
/// Final layout on screen:
///   [original @ 0.5x] | [canvas_a (with nested results)]
struct SubImageNestTest {
    tree: Option<Image>,
    c1: Option<Image>,
    c2: Option<Image>,
    c3: Option<Image>,
    canvas_a: Option<Image>,
    canvas_b: Option<Image>,
    ready: bool,
}

impl Spot for SubImageNestTest {
    fn initialize(_ctx: &mut Context) -> Self {
        Self {
            tree: None,
            c1: None,
            c2: None,
            c3: None,
            canvas_a: None,
            canvas_b: None,
            ready: false,
        }
    }

    fn update(&mut self, ctx: &mut Context, _dt: std::time::Duration) {
        if self.ready {
            return;
        }

        // Load happy-tree
        const BYTES: &[u8] = include_bytes!("../assets/happy-tree.png");
        let img = image::load_from_memory(BYTES).unwrap();
        let rgba = img.to_rgba8();
        let tree = Image::new(
            ctx,
            Pt::from(img.width() as f32),
            Pt::from(img.height() as f32),
            &rgba,
        )
        .unwrap();

        // Sub-image layer 1: tree canopy (top 512x512, centered)
        let c1 = Image::sub_image(
            ctx,
            tree,
            spottedcat::image::Bounds::new(
                Pt::from(256.0),
                Pt::from(50.0),
                Pt::from(512.0),
                Pt::from(512.0),
            ),
        )
        .unwrap();

        // Sub-image layer 2: face area (center of c1)
        let c2 = Image::sub_image(
            ctx,
            c1,
            spottedcat::image::Bounds::new(
                Pt::from(128.0),
                Pt::from(128.0),
                Pt::from(256.0),
                Pt::from(256.0),
            ),
        )
        .unwrap();

        // Sub-image layer 3: smile (center of c2)
        let c3 = Image::sub_image(
            ctx,
            c2,
            spottedcat::image::Bounds::new(
                Pt::from(64.0),
                Pt::from(64.0),
                Pt::from(128.0),
                Pt::from(128.0),
            ),
        )
        .unwrap();

        // Render targets for each nesting level
        let canvas_a =
            spottedcat::Texture::new_render_target(ctx, Pt::from(512.0), Pt::from(512.0)).view();
        let canvas_b =
            spottedcat::Texture::new_render_target(ctx, Pt::from(256.0), Pt::from(256.0)).view();

        println!("--- Nested Sub-Image Render Target Test ---");
        println!("tree:     {:?}", tree.bounds());
        println!("sub 1:    {:?}", c1.bounds());
        println!("sub 2:    {:?}", c2.bounds());
        println!("sub 3:    {:?}", c3.bounds());

        self.tree = Some(tree);
        self.c1 = Some(c1);
        self.c2 = Some(c2);
        self.c3 = Some(c3);
        self.canvas_a = Some(canvas_a);
        self.canvas_b = Some(canvas_b);
        self.ready = true;
    }

    fn draw(&mut self, ctx: &mut Context, screen: Image) {
        let (Some(tree), Some(c1), Some(c2), Some(c3), Some(ca), Some(cb)) = (
            self.tree,
            self.c1,
            self.c2,
            self.c3,
            self.canvas_a,
            self.canvas_b,
        ) else {
            return;
        };

        // canvas_b (256x256):
        //   - c2 fills it at natural 1x scale (background)
        //   - c3 (128x128) drawn at 2x scale from position (64,64)
        //     → acts as a "zoom lens": same area but magnified, going past edge
        //     → bottom-right of canvas_b shows c3 content at 2x, clearly different
        cb.draw(ctx, &c2, DrawOption::default());
        cb.draw(
            ctx,
            &c3,
            DrawOption::default()
                .with_position([Pt::from(64.0), Pt::from(64.0)])
                .with_scale([2.0, 2.0]),
        );

        // canvas_a (512x512):
        //   - c1 fills it at natural 1x scale
        //   - canvas_b (with zoom lens baked in) overlaid at (128, 128) → nested inset
        ca.draw(ctx, &c1, DrawOption::default());
        ca.draw(
            ctx,
            &cb,
            DrawOption::default().with_position([Pt::from(128.0), Pt::from(128.0)]),
        );

        // --- Screen: 2 panels ---
        // Left: original tree (reference)
        screen.draw(
            ctx,
            &tree,
            DrawOption::default()
                .with_position([Pt::from(10.0), Pt::from(10.0)])
                .with_scale([0.35, 0.35]),
        );

        // Right: canvas_a — the full nested composite
        //   Shows: c1 → canvas_b (c2 + c3@2x) nested in bottom-right quarter
        screen.draw(
            ctx,
            &ca,
            DrawOption::default()
                .with_position([Pt::from(400.0), Pt::from(10.0)])
                .with_scale([0.75, 0.75]),
        );
    }

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    run::<SubImageNestTest>(WindowConfig {
        title: "SubImage Nested Render Target Test".to_string(),
        ..Default::default()
    });
}
