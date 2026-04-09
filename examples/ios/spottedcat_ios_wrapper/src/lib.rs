#[cfg(target_os = "ios")]
use spottedcat::{Context, DrawOption, DrawOption3D, Image, Model, Pt, Spot, Text, WindowConfig};

#[cfg(target_os = "ios")]
use block2::RcBlock;
#[cfg(target_os = "ios")]
use objc2::{msg_send, rc::Retained, runtime::AnyObject};
#[cfg(target_os = "ios")]
use objc2_core_motion::{CMPedometer, CMPedometerData};
#[cfg(target_os = "ios")]
use objc2_foundation::{MainThreadMarker, NSCalendar, NSDate};
#[cfg(target_os = "ios")]
use std::sync::{Arc, Mutex};

#[cfg(target_os = "ios")]
fn set_shared_text(target: &Arc<Mutex<String>>, value: impl Into<String>) {
    if let Ok(mut text) = target.lock() {
        *text = value.into();
    }
}

#[cfg(target_os = "ios")]
fn format_history(labels: &[String], counts: &[Option<i32>]) -> String {
    let mut lines = vec!["Pedometer (last 7 days)".to_string()];
    let mut total = 0i32;

    for (label, count) in labels.iter().zip(counts.iter()) {
        match count {
            Some(value) => {
                total += *value;
                lines.push(format!("{label}: {value}"));
            }
            None => lines.push(format!("{label}: ...")),
        }
    }

    if counts.iter().all(Option::is_some) {
        lines.push(format!("7d total: {total}"));
    } else {
        lines.push("7d total: loading...".to_string());
    }

    lines.join("\n")
}

#[cfg(target_os = "ios")]
struct IosPedometerHistory {
    pedometer: Retained<CMPedometer>,
    history_text: Arc<Mutex<String>>,
    status_text: Arc<Mutex<String>>,
}

#[cfg(target_os = "ios")]
impl IosPedometerHistory {
    fn new() -> Self {
        let _mtm = MainThreadMarker::new().expect("must be on the main thread for CMPedometer");

        Self {
            pedometer: unsafe { CMPedometer::new() },
            history_text: Arc::new(Mutex::new("Pedometer history idle".to_string())),
            status_text: Arc::new(Mutex::new("History: idle".to_string())),
        }
    }

    fn history_text(&self) -> Arc<Mutex<String>> {
        self.history_text.clone()
    }

    fn status_text(&self) -> Arc<Mutex<String>> {
        self.status_text.clone()
    }

    fn query_last_7_days(&self) {
        unsafe {
            let pedometer_available: bool =
                msg_send![objc2::class!(CMPedometer), isStepCountingAvailable];
            if !pedometer_available {
                set_shared_text(
                    &self.status_text,
                    "History: unavailable on this device or simulator",
                );
                set_shared_text(
                    &self.history_text,
                    "Pedometer (last 7 days)\nRequires a real iPhone with motion data.",
                );
                return;
            }

            set_shared_text(&self.status_text, "History: requesting last 7 days...");

            let now: Retained<NSDate> = msg_send![objc2::class!(NSDate), date];
            let calendar: Retained<NSCalendar> =
                msg_send![objc2::class!(NSCalendar), currentCalendar];
            let start_of_today: Retained<NSDate> =
                msg_send![&calendar, startOfDayForDate: &*now];
            let now_ts: f64 = msg_send![&*now, timeIntervalSince1970];
            let start_of_today_ts: f64 = msg_send![&*start_of_today, timeIntervalSince1970];

            let labels = Arc::new(vec![
                "6d ago".to_string(),
                "5d ago".to_string(),
                "4d ago".to_string(),
                "3d ago".to_string(),
                "2d ago".to_string(),
                "Yesterday".to_string(),
                "Today".to_string(),
            ]);
            let counts = Arc::new(Mutex::new(vec![None; 7]));
            set_shared_text(&self.history_text, format_history(&labels, &vec![None; 7]));

            for idx in 0..7 {
                let day_start_ts = start_of_today_ts - ((6 - idx) as f64 * 86_400.0);
                let day_end_ts = if idx == 6 {
                    now_ts
                } else {
                    day_start_ts + 86_400.0
                };

                let start_date: Retained<NSDate> =
                    msg_send![objc2::class!(NSDate), dateWithTimeIntervalSince1970: day_start_ts];
                let end_date: Retained<NSDate> =
                    msg_send![objc2::class!(NSDate), dateWithTimeIntervalSince1970: day_end_ts];

                let history_text = self.history_text.clone();
                let status_text = self.status_text.clone();
                let labels_ref = labels.clone();
                let counts_ref = counts.clone();

                let handler = RcBlock::new(
                    move |data: *mut CMPedometerData, error: *mut AnyObject| {
                        if !error.is_null() {
                            set_shared_text(
                                &status_text,
                                "History: query failed or motion permission denied",
                            );
                            return;
                        }

                        let step_count = if data.is_null() {
                            0
                        } else {
                            let steps_obj: *mut AnyObject = msg_send![&*data, numberOfSteps];
                            if steps_obj.is_null() {
                                0
                            } else {
                                let value: i32 = msg_send![steps_obj, intValue];
                                value
                            }
                        };

                        if let Ok(mut counts_locked) = counts_ref.lock() {
                            counts_locked[idx] = Some(step_count);
                            let formatted = format_history(&labels_ref, &counts_locked);
                            set_shared_text(&history_text, formatted);
                            if counts_locked.iter().all(Option::is_some) {
                                set_shared_text(&status_text, "History: updated from CMPedometer");
                            }
                        }
                    },
                );

                let _: () = msg_send![
                    &self.pedometer,
                    queryPedometerDataFromDate: &*start_date,
                    toDate: &*end_date,
                    withHandler: &*handler
                ];
            }
        }
    }
}

#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn spottedcat_ios_start() {
    struct IosFfiSpot {
        happy_tree: Image,
        text: Text,
        fps_text: Text,
        today_steps_text: Text,
        yesterday_steps_text: Text,
        history_status_text: Text,
        history_text: Text,
        touch_pos: Option<(Pt, Pt)>,
        last_fps_time: std::time::Instant,
        frame_count: u32,
        current_fps: f32,
        update_count: u32,
        current_ups: f32,
        model: Model,
        rotation: f32,
        history_state: IosPedometerHistory,
        requested_history: bool,
    }

    impl Spot for IosFfiSpot {
        fn initialize(ctx: &mut Context) -> Self {
            eprintln!("[spot][ios] initialize called");

            const HAPPY_TREE_BYTES: &[u8] = include_bytes!("../../../../assets/happy-tree.png");
            let img = image::load_from_memory(HAPPY_TREE_BYTES).unwrap();
            let happy_tree = spottedcat::utils::image::from_image(ctx, &img).unwrap();

            const FALLBACK_FONT: &[u8] = include_bytes!("../../../../assets/DejaVuSans.ttf");
            let font_id = spottedcat::register_font(ctx, FALLBACK_FONT.to_vec());

            let text = Text::new("3D Model & Pedometer Test!", font_id)
                .with_font_size(Pt::from(32.0))
                .with_color([1.0, 1.0, 1.0, 1.0]);
            let fps_text = Text::new("FPS: 0.0", font_id).with_font_size(Pt::from(24.0));
            let today_steps_text =
                Text::new("Today's Steps: 0", font_id).with_font_size(Pt::from(22.0));
            let yesterday_steps_text =
                Text::new("Yesterday's Steps: 0", font_id).with_font_size(Pt::from(22.0));
            let history_status_text = Text::new("History: idle", font_id)
                .with_font_size(Pt::from(18.0))
                .with_color([0.5, 1.0, 0.8, 1.0]);
            let history_text = Text::new("Pedometer history idle", font_id)
                .with_font_size(Pt::from(16.0))
                .with_color([0.8, 0.9, 1.0, 1.0]);

            spottedcat::set_ambient_light(ctx, [0.2, 0.2, 0.2, 1.0]);
            spottedcat::set_light(ctx, 0, [10.0, 10.0, 10.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
            spottedcat::set_camera_pos(ctx, [0.0, 0.0, 5.0]);

            let model = spottedcat::model::create_cube(ctx, 1.0).unwrap();
            let history_state = IosPedometerHistory::new();

            Self {
                happy_tree,
                text,
                fps_text,
                today_steps_text,
                yesterday_steps_text,
                history_status_text,
                history_text,
                touch_pos: None,
                last_fps_time: std::time::Instant::now(),
                frame_count: 0,
                current_fps: 0.0,
                update_count: 0,
                current_ups: 0.0,
                model,
                rotation: 0.0,
                history_state,
                requested_history: false,
            }
        }

        fn update(&mut self, ctx: &mut Context, dt: std::time::Duration) {
            self.update_count += 1;
            if self.update_count % 60 == 0 {
                eprintln!("[spot][ios] update loop running");
            }

            self.rotation += dt.as_secs_f32() * 1.5;

            let today_steps = spottedcat::today_step_count(ctx).unwrap_or(0.0);
            let yesterday_steps = spottedcat::yesterday_step_count(ctx).unwrap_or(0.0);
            self.today_steps_text
                .set_content(format!("Today's Steps: {:.0}", today_steps));
            self.yesterday_steps_text
                .set_content(format!("Yesterday's Steps: {:.0}", yesterday_steps));

            if !self.requested_history {
                self.history_state.query_last_7_days();
                self.requested_history = true;
            }

            if let Ok(status) = self.history_state.status_text().lock() {
                self.history_status_text.set_content(status.clone());
            }
            if let Ok(history) = self.history_state.history_text().lock() {
                self.history_text.set_content(history.clone());
            }

            let mut active_touch = false;
            let current_touches = spottedcat::touches(ctx);
            if !current_touches.is_empty() {
                eprintln!("[spot][ios] active touches count: {}", current_touches.len());
            }

            for touch in current_touches {
                if self.touch_pos.is_none()
                    || (touch.position.0 - self.touch_pos.unwrap().0)
                        .as_f32()
                        .abs()
                        > 1.0
                {
                    eprintln!("[spot][ios] touch detected at: {:?}", touch.position);
                }
                self.touch_pos = Some(touch.position);
                active_touch = true;
            }

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
                self.current_ups = self.update_count as f32 / elapsed.as_secs_f32();
                self.fps_text
                    .set_content(format!("FPS: {:.1} | UPS: {:.1}", self.current_fps, self.current_ups));
                self.last_fps_time = now;
                self.frame_count = 0;
                self.update_count = 0;
            }

            let opts_3d = DrawOption3D::default()
                .with_position([0.0, 0.0, 0.0])
                .with_rotation([0.0, self.rotation, 0.0]);
            spottedcat::model::draw(ctx, &self.model, opts_3d);

            spottedcat::text::draw(
                ctx,
                &self.text,
                DrawOption::default().with_position([Pt::from(50.0), Pt::from(100.0)]),
            );
            spottedcat::text::draw(
                ctx,
                &self.fps_text,
                DrawOption::default().with_position([Pt::from(50.0), Pt::from(150.0)]),
            );
            spottedcat::text::draw(
                ctx,
                &self.today_steps_text,
                DrawOption::default().with_position([Pt::from(50.0), Pt::from(190.0)]),
            );
            spottedcat::text::draw(
                ctx,
                &self.yesterday_steps_text,
                DrawOption::default().with_position([Pt::from(50.0), Pt::from(220.0)]),
            );
            spottedcat::text::draw(
                ctx,
                &self.history_status_text,
                DrawOption::default().with_position([Pt::from(50.0), Pt::from(250.0)]),
            );
            spottedcat::text::draw(
                ctx,
                &self.history_text,
                DrawOption::default().with_position([Pt::from(50.0), Pt::from(280.0)]),
            );

            let (w, h) = spottedcat::window_size(ctx);
            let pos = self.touch_pos.unwrap_or_else(|| (w / 2.0, h / 2.0));
            let img_opts = DrawOption::default().with_position([
                pos.0 - self.happy_tree.width() / 2.0,
                pos.1 - self.happy_tree.height() / 2.0,
            ]);
            self.happy_tree.draw(ctx, img_opts);
        }

        fn resumed(&mut self, _ctx: &mut Context) {
            eprintln!("[spot][ios] resumed called");
            self.requested_history = false;
        }

        fn suspended(&mut self, _ctx: &mut Context) {
            eprintln!("[spot][ios] suspended called");
        }

        fn remove(&mut self, _ctx: &mut Context) {
            eprintln!("[spot][ios] remove called");
        }
    }

    spottedcat::run::<IosFfiSpot>(WindowConfig::default());
}
