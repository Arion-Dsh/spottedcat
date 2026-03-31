use spottedcat::{
    AndroidApp, Context, DrawOption, DrawOption3D, Image, Model, PlatformEvent, Pt, Spot, Text,
    WindowConfig,
};
#[cfg(target_os = "android")]
use jni::{
    objects::{JClass, JString},
    JNIEnv,
};

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_com_example_gameactivityexample_MainActivity_sendNativeEvent(
    mut env: JNIEnv,
    _class: JClass,
    event_type: JString,
    data: JString,
) {
    let t: String = env.get_string(&event_type).unwrap().into();
    let d: String = env.get_string(&data).unwrap().into();
    spottedcat::push_platform_event(PlatformEvent::Event(t, d));
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: AndroidApp) {
    struct AndroidFfiSpot {
        happy_tree: Image,
        text: Text,
        fps_text: Text,
        gyro_text: Text,
        accel_text: Text,
        mag_text: Text,
        rot_text: Text,
        rot_data: [f32; 4],
        step_text: Text,
        bridge_text: Text,
        step_count: f32,
        step_detected_timer: f32,
        touch_pos: Option<(Pt, Pt)>,
        last_fps_time: std::time::Instant,
        frame_count: u32,
        current_fps: f32,
        model: Model,
        rotation_anim: f32,
        gyro_data: [f32; 3],
        accel_data: [f32; 3],
        mag_data: [f32; 3],
    }

    impl Spot for AndroidFfiSpot {
        fn initialize(ctx: &mut Context) -> Self {
            spottedcat::set_background_transparent(ctx, false);
            eprintln!("[spot][android] initialize called");
            // Load an image from assets
            const HAPPY_TREE_BYTES: &[u8] = include_bytes!("../../../../assets/happy-tree.png");
            let img = image::load_from_memory(HAPPY_TREE_BYTES)
                .unwrap()
                .to_rgba8();
            let happy_tree =
                Image::new_from_rgba8(ctx, Pt::from(img.width()), Pt::from(img.height()), &img).unwrap();

            // Register a font and create text
            const FALLBACK_FONT: &[u8] = include_bytes!("../../../../assets/DejaVuSans.ttf");
            let font_id = spottedcat::register_font(ctx, FALLBACK_FONT.to_vec());

            let text = Text::new("3D Model & Gyro Test!", font_id)
                .with_font_size(Pt::from(32.0))
                .with_color([1.0, 1.0, 1.0, 1.0]);

            let fps_text = Text::new("FPS: 0.0", font_id).with_font_size(Pt::from(24.0));
            let gyro_text = Text::new("Gyro: 0.0, 0.0, 0.0", font_id).with_font_size(Pt::from(20.0));
            let accel_text = Text::new("Accel: 0.0, 0.0, 0.0", font_id).with_font_size(Pt::from(20.0));
            let mag_text = Text::new("Mag: 0.0, 0.0, 0.0", font_id).with_font_size(Pt::from(20.0));
            let rot_text = Text::new("Rot: 0.0, 0.0, 0.0, 0.0", font_id).with_font_size(Pt::from(20.0));
            let step_text = Text::new("Steps: 0", font_id).with_font_size(Pt::from(20.0));

            // Setup 3D scene
            ctx.set_ambient_light([0.2, 0.2, 0.2, 1.0]);
            ctx.set_light(0, [10.0, 10.0, 10.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
            ctx.set_camera_pos([0.0, 0.0, 5.0]);

            let model = Model::cube(ctx, 1.5).unwrap();

            Self {
                happy_tree,
                text,
                fps_text,
                gyro_text,
                accel_text,
                mag_text,
                rot_text,
                rot_data: [0.0; 4],
                step_text,
                bridge_text: Text::new("Bridge: No Event", font_id)
                    .with_font_size(Pt::from(20.0))
                    .with_color([0.5, 1.0, 0.5, 1.0]),
                step_count: 0.0,
                step_detected_timer: 0.0,
                touch_pos: None,
                last_fps_time: std::time::Instant::now(),
                frame_count: 0,
                current_fps: 0.0,
                model,
                rotation_anim: 0.0,
                gyro_data: [0.0; 3],
                accel_data: [0.0; 3],
                mag_data: [0.0; 3],
            }
        }

        fn update(&mut self, ctx: &mut Context, dt: std::time::Duration) {
            // Log that update is running (at low frequency to avoid spam)
            if self.frame_count % 60 == 0 {
                eprintln!("[spot][android] update loop running");
            }

            self.rotation_anim += dt.as_secs_f32() * 1.5;

            // Update sensor data
            self.gyro_data = spottedcat::gyroscope(ctx).unwrap_or([0.0; 3]);
            self.accel_data = spottedcat::accelerometer(ctx).unwrap_or([0.0; 3]);
            self.mag_data = spottedcat::magnetometer(ctx).unwrap_or([0.0; 3]);
            self.rot_data = spottedcat::rotation(ctx).unwrap_or([0.0; 4]);

            self.gyro_text.set_content(format!(
                "Gyro: {:.2}, {:.2}, {:.2}",
                self.gyro_data[0], self.gyro_data[1], self.gyro_data[2]
            ));
            self.accel_text.set_content(format!(
                "Accel: {:.2}, {:.2}, {:.2}",
                self.accel_data[0], self.accel_data[1], self.accel_data[2]
            ));
            self.mag_text.set_content(format!(
                "Mag: {:.2}, {:.2}, {:.2}",
                self.mag_data[0], self.mag_data[1], self.mag_data[2]
            ));
            self.rot_text.set_content(format!(
                "Rot: {:.2}, {:.2}, {:.2}, {:.2}",
                self.rot_data[0], self.rot_data[1], self.rot_data[2], self.rot_data[3]
            ));
            
            self.step_count = spottedcat::step_count(ctx).unwrap_or(0.0);
            if spottedcat::step_detected(ctx) {
                self.step_detected_timer = 0.5;
            }
            self.step_detected_timer = (self.step_detected_timer - dt.as_secs_f32()).max(0.0);
            self.step_text.set_content(format!("Steps: {:.0}", self.step_count));
            if self.step_detected_timer > 0.0 {
                self.step_text.set_color([1.0, 1.0, 0.0, 1.0]); // Yellow on detection
            } else {
                self.step_text.set_color([1.0, 1.0, 1.0, 1.0]);
            }

            // --- Platform Bridge Example ---
            for event in spottedcat::poll_platform_events(ctx) {
                let PlatformEvent::Event(t, d) = event;
                self.bridge_text.set_content(format!("{}: {}", t, d));
            }

            // Touch to trigger Kotlin method
            if spottedcat::touch_down(ctx) {
                #[cfg(target_os = "android")]
                if let (Some(jvm), Some(activity)) =
                    (spottedcat::android::get_jvm(), spottedcat::android::get_activity())
                {
                    let mut env = jvm.attach_current_thread().unwrap();
                    let msg = env.new_string("Hello from Rust!").unwrap();
                    let _ = env.call_method(
                        activity.as_obj(),
                        "triggerTestEvent",
                        "(Ljava/lang/String;)V",
                        &[(&msg).into()],
                    );
                }
            }

            // 1. Check direct touch events
            let mut active_touch = false;
            let current_touches = spottedcat::touches(ctx);
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

            // 2. Fallback to mouse/cursor
            if !active_touch {
                if let Some(cursor) = spottedcat::cursor_position(ctx) {
                    self.touch_pos = Some(cursor);
                }
            }
        }

        fn draw(&mut self, ctx: &mut Context) {
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

            // Draw 3D model with gyroscope tilt
            // We use gyro X and Y to nudge the rotation
            let opts_3d = DrawOption3D::default()
                .with_position([0.0, 0.0, 0.0])
                .with_rotation([
                    self.gyro_data[0] * 0.5, 
                    self.rotation_anim + self.gyro_data[1] * 0.5, 
                    self.gyro_data[2] * 0.5
                ]);
            self.model.draw(ctx, opts_3d);

            // Draw UI
            let text_opts = DrawOption::default().with_position([Pt::from(50.0), Pt::from(100.0)]);
            self.text.draw(ctx, text_opts);

            self.fps_text.draw(
                ctx,
                DrawOption::default().with_position([Pt::from(50.0), Pt::from(150.0)]),
            );
            
            self.gyro_text.draw(
                ctx,
                DrawOption::default().with_position([Pt::from(50.0), Pt::from(190.0)]),
            );

            self.accel_text.draw(
                ctx,
                DrawOption::default().with_position([Pt::from(50.0), Pt::from(220.0)]),
            );

            self.mag_text.draw(
                ctx,
                DrawOption::default().with_position([Pt::from(50.0), Pt::from(250.0)]),
            );

            self.rot_text.draw(
                ctx,
                DrawOption::default().with_position([Pt::from(50.0), Pt::from(280.0)]),
            );
            
            self.step_text.draw(
                ctx,
                DrawOption::default().with_position([Pt::from(50.0), Pt::from(310.0)]),
            );

            self.bridge_text.draw(
                ctx,
                DrawOption::default().with_position([Pt::from(50.0), Pt::from(340.0)]),
            );

            // Draw Bridge text
            self.bridge_text.draw(
                ctx,
                DrawOption::default().with_position([Pt::from(50.0), Pt::from(300.0)]),
            );

            // Draw image at touch position or center
            let pos = self.touch_pos.unwrap_or_else(|| {
                let (w, h) = spottedcat::window_size(ctx);
                (w / 2.0, h / 2.0)
            });

            let img_opts = DrawOption::default().with_position([
                pos.0 - self.happy_tree.width() / 2.0,
                pos.1 - self.happy_tree.height() / 2.0,
            ]);
            self.happy_tree.draw(ctx, img_opts);
        }

        fn resumed(&mut self, _ctx: &mut Context) {
            eprintln!("[spot][android] resumed called");
        }

        fn suspended(&mut self, _ctx: &mut Context) {
            eprintln!("[spot][android] suspended called");
        }

        fn remove(&mut self, _ctx: &mut Context) {
            eprintln!("[spot][android] remove called");
        }
    }

    struct OverlaySpot {
        text: Text,
    }
    
    impl Spot for OverlaySpot {
        fn initialize(ctx: &mut Context) -> Self {
            spottedcat::set_background_transparent(ctx, true);
            eprintln!("[spot][android] OverlaySpot initialize called");
            
            const FALLBACK_FONT: &[u8] = include_bytes!("../../../../assets/DejaVuSans.ttf");
            let font_id = spottedcat::register_font(ctx, FALLBACK_FONT.to_vec());
            let text = Text::new("Floating Cat!", font_id)
                .with_font_size(Pt::from(24.0))
                .with_color([1.0, 1.0, 0.0, 1.0]);
            Self { text }
        }
        fn draw(&mut self, ctx: &mut Context) {
            self.text.draw(
                ctx,
                DrawOption::default().with_position([Pt::from(10.0), Pt::from(30.0)]),
            );
        }
        fn update(&mut self, _ctx: &mut Context, _dt: std::time::Duration) {}
        fn remove(&mut self, _ctx: &mut Context) {}
    }

    #[cfg(target_os = "android")]
    {
        spottedcat::android::set_floating_window_scene::<OverlaySpot>();
        spottedcat::android::set_floating_window_service("com/example/gameactivityexample/FloatingWindowService");
    }

    spottedcat::run::<AndroidFfiSpot>(WindowConfig::default(), app);
}
