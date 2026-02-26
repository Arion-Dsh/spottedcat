use spottedcat::{Context, DrawOption, Image, Pt, Spot, Text, WindowConfig};

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    struct AndroidFfiSpot {
        happy_tree: Image,
        text: Text,
        fps_text: Text,
        touch_pos: Option<(Pt, Pt)>,
        last_fps_time: std::time::Instant,
        frame_count: u32,
        current_fps: f32,
    }

    impl Spot for AndroidFfiSpot {
        fn initialize(_context: &mut Context) -> Self {
            eprintln!("[spot][android] initialize called");
            // Load an image from assets
            const HAPPY_TREE_BYTES: &[u8] = include_bytes!("../../../../assets/happy-tree.png");
            let img = image::load_from_memory(HAPPY_TREE_BYTES)
                .unwrap()
                .to_rgba8();
            let happy_tree =
                Image::new_from_rgba8(Pt::from(img.width()), Pt::from(img.height()), &img).unwrap();

            // Register a font and create text
            const FALLBACK_FONT: &[u8] = include_bytes!("../../../../assets/DejaVuSans.ttf");
            let font_id = spottedcat::register_font(FALLBACK_FONT.to_vec());

            let text = Text::new("Tap to move tree!", font_id)
                .with_font_size(Pt::from(32.0))
                .with_color([1.0, 1.0, 1.0, 1.0]);

            let fps_text = Text::new("FPS: 0.0", font_id).with_font_size(Pt::from(24.0));

            Self {
                happy_tree,
                text,
                fps_text,
                touch_pos: None,
                last_fps_time: std::time::Instant::now(),
                frame_count: 0,
                current_fps: 0.0,
            }
        }

        fn update(&mut self, context: &mut Context, _dt: std::time::Duration) {
            // Log that update is running (at low frequency to avoid spam)
            if self.frame_count % 60 == 0 {
                eprintln!("[spot][android] update loop running");
            }

            // 1. Check direct touch events
            let mut active_touch = false;
            let current_touches = spottedcat::touches(context);
            if !current_touches.is_empty() {
                eprintln!(
                    "[spot][android] active touches count: {}",
                    current_touches.len()
                );
            }

            for touch in current_touches {
                // Any active touch updates the position
                if self.touch_pos.is_none()
                    || (touch.position.0 - self.touch_pos.unwrap().0)
                        .as_f32()
                        .abs()
                        > 1.0
                {
                    eprintln!("[spot][android] touch detected at: {:?}", touch.position);
                }
                self.touch_pos = Some(touch.position);
                active_touch = true;
            }

            // 2. Fallback to mouse/cursor (synthesis from touch works on most backends)
            if !active_touch {
                if let Some(cursor) = spottedcat::cursor_position(context) {
                    self.touch_pos = Some(cursor);
                }
            }
        }

        fn draw(&mut self, context: &mut Context) {
            // Calculate Real FPS based on draw calls
            self.frame_count += 1;
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(self.last_fps_time);

            if elapsed >= std::time::Duration::from_secs(1) {
                self.current_fps = self.frame_count as f32 / elapsed.as_secs_f32();
                self.fps_text
                    .set_content(format!("FPS: {:.1}", self.current_fps));
                self.last_fps_time = now;
                self.frame_count = 0;
            }

            // Draw background text
            let text_opts = DrawOption::default().with_position([Pt::from(50.0), Pt::from(100.0)]);
            self.text.draw(context, text_opts);

            // Draw current FPS value
            self.fps_text.draw(
                context,
                DrawOption::default().with_position([Pt::from(50.0), Pt::from(150.0)]),
            );

            // Draw image at touch position or center
            let pos = self.touch_pos.unwrap_or_else(|| {
                let (w, h) = spottedcat::window_size(context);
                (w / 2.0, h / 2.0)
            });

            // Center the image on the touch/cursor position
            let img_opts = DrawOption::default().with_position([
                pos.0 - self.happy_tree.width() / 2.0,
                pos.1 - self.happy_tree.height() / 2.0,
            ]);
            self.happy_tree.draw(context, img_opts);
        }

        fn resumed(&mut self, _context: &mut Context) {
            eprintln!("[spot][android] resumed called");
        }

        fn suspended(&mut self, _context: &mut Context) {
            eprintln!("[spot][android] suspended called");
        }

        fn remove(&self) {
            eprintln!("[spot][android] remove called");
        }
    }

    spottedcat::run::<AndroidFfiSpot>(WindowConfig::default(), app);
}
