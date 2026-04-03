#[cfg(target_os = "android")]
use spottedcat::{
    Context, DrawOption, DrawOption3D, Image, Model, PlatformEvent, Pt, Spot, Text, TouchPhase,
    WindowConfig,
};
#[cfg(target_os = "android")]
use spottedcat::AndroidApp;
#[cfg(target_os = "android")]
use jni::{
    objects::{JClass, JObject, JString, JValue},
    JNIEnv,
};
#[cfg(target_os = "android")]
use std::collections::BTreeMap;

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
fn load_android_class<'a>(
    env: &mut JNIEnv<'a>,
    activity: &JObject<'a>,
    class_name: &str,
) -> Result<JClass<'a>, String> {
    if class_name.starts_with("android/") || class_name.starts_with("java/") {
        return env.find_class(class_name).map_err(|e| e.to_string());
    }

    let class_loader = env
        .call_method(activity, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])
        .and_then(|v| v.l())
        .map_err(|e| e.to_string())?;
    let class_name_java = env
        .new_string(class_name.replace('/', "."))
        .map_err(|e| e.to_string())?;
    let class_name_obj = JObject::from(class_name_java);
    let class_obj = env
        .call_method(
            class_loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&class_name_obj)],
        )
        .and_then(|v| v.l())
        .map_err(|e| e.to_string())?;
    Ok(JClass::from(class_obj))
}

#[cfg(target_os = "android")]
fn request_health_permission_from_rust() -> Result<(), String> {
    let jvm = spottedcat::android::get_jvm().ok_or_else(|| "JVM unavailable".to_string())?;
    let activity_ref =
        spottedcat::android::get_activity().ok_or_else(|| "Activity unavailable".to_string())?;
    let mut env = jvm.attach_current_thread().map_err(|e| e.to_string())?;
    env.call_method(
        activity_ref.as_obj(),
        "requestHealthConnectPermissionFromRust",
        "()V",
        &[],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(target_os = "android")]
fn fetch_health_history_from_rust() -> Result<String, String> {
    let provider = "com.google.android.apps.healthdata";
    let jvm = spottedcat::android::get_jvm().ok_or_else(|| "JVM unavailable".to_string())?;
    let activity_ref =
        spottedcat::android::get_activity().ok_or_else(|| "Activity unavailable".to_string())?;
    let mut env = jvm.attach_current_thread().map_err(|e| e.to_string())?;
    let activity = activity_ref.as_obj();

    let health_client_class =
        load_android_class(&mut env, activity, "androidx/health/connect/client/HealthConnectClient")?;
    let provider_java = env.new_string(provider).map_err(|e| e.to_string())?;
    let provider_obj = JObject::from(provider_java);
    let sdk_available = env
        .get_static_field(&health_client_class, "SDK_AVAILABLE", "I")
        .and_then(|v| v.i())
        .map_err(|e| e.to_string())?;
    let sdk_status = env
        .call_static_method(
            &health_client_class,
            "getSdkStatus",
            "(Landroid/content/Context;Ljava/lang/String;)I",
            &[
                JValue::Object(activity),
                JValue::Object(&provider_obj),
            ],
        )
        .and_then(|v| v.i())
        .map_err(|e| e.to_string())?;
    if sdk_status != sdk_available {
        return Err("Health Connect SDK unavailable".to_string());
    }

    let client = env
        .call_static_method(
            &health_client_class,
            "getOrCreate",
            "(Landroid/content/Context;Ljava/lang/String;)Landroidx/health/connect/client/HealthConnectClient;",
            &[
                JValue::Object(activity),
                JValue::Object(&provider_obj),
            ],
        )
        .and_then(|v| v.l())
        .map_err(|e| e.to_string())?;

    let steps_record_class = load_android_class(
        &mut env,
        activity,
        "androidx/health/connect/client/records/StepsRecord",
    )?;
    let count_total_metric = env
        .get_static_field(
            &steps_record_class,
            "COUNT_TOTAL",
            "Landroidx/health/connect/client/aggregate/AggregateMetric;",
        )
        .and_then(|v| v.l())
        .map_err(|e| e.to_string())?;

    let hash_set_class = env.find_class("java/util/HashSet").map_err(|e| e.to_string())?;
    let metric_set = env
        .new_object(&hash_set_class, "()V", &[])
        .map_err(|e| e.to_string())?;
    env.call_method(
        &metric_set,
        "add",
        "(Ljava/lang/Object;)Z",
        &[JValue::Object(&count_total_metric)],
    )
    .map_err(|e| e.to_string())?;

    let local_date_class = env.find_class("java/time/LocalDate").map_err(|e| e.to_string())?;
    let today = env
        .call_static_method(&local_date_class, "now", "()Ljava/time/LocalDate;", &[])
        .and_then(|v| v.l())
        .map_err(|e| e.to_string())?;
    let start_date = env
        .call_method(&today, "minusDays", "(J)Ljava/time/LocalDate;", &[JValue::Long(6)])
        .and_then(|v| v.l())
        .map_err(|e| e.to_string())?;
    let end_date = env
        .call_method(&today, "plusDays", "(J)Ljava/time/LocalDate;", &[JValue::Long(1)])
        .and_then(|v| v.l())
        .map_err(|e| e.to_string())?;
    let start_time = env
        .call_method(
            &start_date,
            "atStartOfDay",
            "()Ljava/time/LocalDateTime;",
            &[],
        )
        .and_then(|v| v.l())
        .map_err(|e| e.to_string())?;
    let end_time = env
        .call_method(&end_date, "atStartOfDay", "()Ljava/time/LocalDateTime;", &[])
        .and_then(|v| v.l())
        .map_err(|e| e.to_string())?;

    let time_range_filter_class = load_android_class(
        &mut env,
        activity,
        "androidx/health/connect/client/time/TimeRangeFilter",
    )?;
    let time_range_filter = env
        .call_static_method(
            &time_range_filter_class,
            "between",
            "(Ljava/time/LocalDateTime;Ljava/time/LocalDateTime;)Landroidx/health/connect/client/time/TimeRangeFilter;",
            &[
                JValue::Object(&start_time),
                JValue::Object(&end_time),
            ],
        )
        .and_then(|v| v.l())
        .map_err(|e| e.to_string())?;

    let period_class = env.find_class("java/time/Period").map_err(|e| e.to_string())?;
    let one_day_period = env
        .call_static_method(&period_class, "ofDays", "(I)Ljava/time/Period;", &[JValue::Int(1)])
        .and_then(|v| v.l())
        .map_err(|e| e.to_string())?;

    let collections_class = env
        .find_class("java/util/Collections")
        .map_err(|e| e.to_string())?;
    let empty_set = env
        .call_static_method(&collections_class, "emptySet", "()Ljava/util/Set;", &[])
        .and_then(|v| v.l())
        .map_err(|e| e.to_string())?;

    let request_class = load_android_class(
        &mut env,
        activity,
        "androidx/health/connect/client/request/AggregateGroupByPeriodRequest",
    )?;
    let request = env
        .new_object(
            &request_class,
            "(Ljava/util/Set;Landroidx/health/connect/client/time/TimeRangeFilter;Ljava/time/Period;Ljava/util/Set;)V",
            &[
                JValue::Object(&metric_set),
                JValue::Object(&time_range_filter),
                JValue::Object(&one_day_period),
                JValue::Object(&empty_set),
            ],
        )
        .map_err(|e| e.to_string())?;

    let result_list = env
        .call_method(
            &client,
            "aggregateGroupByPeriod",
            "(Landroidx/health/connect/client/request/AggregateGroupByPeriodRequest;)Ljava/util/List;",
            &[JValue::Object(&request)],
        )
        .and_then(|v| v.l())
        .map_err(|e| e.to_string())?;

    let size = env
        .call_method(&result_list, "size", "()I", &[])
        .and_then(|v| v.i())
        .map_err(|e| e.to_string())?;

    let mut counts_by_day = BTreeMap::new();
    for idx in 0..size {
        let entry = env
            .call_method(
                &result_list,
                "get",
                "(I)Ljava/lang/Object;",
                &[JValue::Int(idx)],
            )
            .and_then(|v| v.l())
            .map_err(|e| e.to_string())?;
        let start_time = env
            .call_method(
                &entry,
                "getStartTime",
                "()Ljava/time/LocalDateTime;",
                &[],
            )
            .and_then(|v| v.l())
            .map_err(|e| e.to_string())?;
        let local_date = env
            .call_method(&start_time, "toLocalDate", "()Ljava/time/LocalDate;", &[])
            .and_then(|v| v.l())
            .map_err(|e| e.to_string())?;
        let label = env
            .call_method(&local_date, "toString", "()Ljava/lang/String;", &[])
            .and_then(|v| v.l())
            .and_then(|v| Ok(JString::from(v)))
            .map_err(|e| e.to_string())?;
        let label: String = env.get_string(&label).map_err(|e| e.to_string())?.into();

        let aggregate_result = env
            .call_method(
                &entry,
                "getResult",
                "()Landroidx/health/connect/client/aggregate/AggregationResult;",
                &[],
            )
            .and_then(|v| v.l())
            .map_err(|e| e.to_string())?;
        let value_obj = env
            .call_method(
                &aggregate_result,
                "get",
                "(Landroidx/health/connect/client/aggregate/AggregateMetric;)Ljava/lang/Object;",
                &[JValue::Object(&count_total_metric)],
            )
            .and_then(|v| v.l())
            .map_err(|e| e.to_string())?;
        let count = if value_obj.is_null() {
            0
        } else {
            env.call_method(&value_obj, "longValue", "()J", &[])
                .and_then(|v| v.j())
                .map_err(|e| e.to_string())?
        };
        counts_by_day.insert(label, count);
    }

    let mut lines = vec!["Health Connect (7 days, Rust JNI)".to_string()];
    let mut total = 0i64;
    for offset in (0..=6).rev() {
        let date = env
            .call_method(&today, "minusDays", "(J)Ljava/time/LocalDate;", &[JValue::Long(offset)])
            .and_then(|v| v.l())
            .map_err(|e| e.to_string())?;
        let label = env
            .call_method(&date, "toString", "()Ljava/lang/String;", &[])
            .and_then(|v| v.l())
            .and_then(|v| Ok(JString::from(v)))
            .map_err(|e| e.to_string())?;
        let label: String = env.get_string(&label).map_err(|e| e.to_string())?.into();
        let count = *counts_by_day.get(&label).unwrap_or(&0);
        total += count;
        lines.push(format!("{label}: {count}"));
    }
    lines.push(format!("7d total: {total}"));
    Ok(lines.join("\n"))
}

#[cfg(target_os = "android")]
fn draw_text_block(ctx: &mut Context, text: &Text, x: f32, y: &mut f32, gap_after: f32) {
    spottedcat::text::draw(
        ctx,
        text,
        DrawOption::default().with_position([Pt::from(x), Pt::from(*y)]),
    );
    let (_, height): (f32, f32) = spottedcat::text::measure(ctx, text);
    *y += height.max(1.0) + gap_after;
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
        yesterday_step_text: Text,
        history_text: Text,
        health_status_text: Text,
        bridge_text: Text,
        step_count: f32,
        yesterday_step_count: f32,
        step_detected_timer: f32,
        requested_health_permission: bool,
        permission_granted: bool,
        history_loaded: bool,
        touch_pos: Option<(Pt, Pt)>,
        last_fps_time: std::time::Instant,
        frame_count: u32,
        current_fps: f32,
        update_count: u32,
        current_ups: f32,
        sensor_text_refresh_accum: f32,
        update_log_accum: f32,
        touch_log_cooldown: f32,
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
                spottedcat::image::create(ctx, Pt::from(img.width()), Pt::from(img.height()), &img).unwrap();

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
            let step_text =
                Text::new("Today's Steps: 0", font_id).with_font_size(Pt::from(20.0));
            let yesterday_step_text =
                Text::new("Yesterday's Steps: 0", font_id).with_font_size(Pt::from(20.0));
            let history_text = Text::new(
                "History: tap the screen to request Health Connect history",
                font_id,
            )
            .with_font_size(Pt::from(16.0))
            .with_color([0.8, 0.9, 1.0, 1.0]);
            let health_status_text =
                Text::new("Health: tap the screen to request permission", font_id)
                .with_font_size(Pt::from(18.0))
                .with_color([0.5, 1.0, 0.8, 1.0]);

            // Setup 3D scene
            spottedcat::set_ambient_light(ctx, [0.2, 0.2, 0.2, 1.0]);
            spottedcat::set_light(ctx, 0, [10.0, 10.0, 10.0, 0.0], [1.0, 1.0, 1.0, 1.0]);
            spottedcat::set_camera_pos(ctx, [0.0, 0.0, 5.0]);

            let model = spottedcat::model::create_cube(ctx, 1.5).unwrap();

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
                yesterday_step_text,
                history_text,
                health_status_text,
                bridge_text: Text::new("Bridge: No Event", font_id)
                    .with_font_size(Pt::from(20.0))
                    .with_color([0.5, 1.0, 0.5, 1.0]),
                step_count: 0.0,
                yesterday_step_count: 0.0,
                step_detected_timer: 0.0,
                requested_health_permission: false,
                permission_granted: false,
                history_loaded: false,
                touch_pos: None,
                last_fps_time: std::time::Instant::now(),
                frame_count: 0,
                current_fps: 0.0,
                update_count: 0,
                current_ups: 0.0,
                sensor_text_refresh_accum: 0.0,
                update_log_accum: 0.0,
                touch_log_cooldown: 0.0,
                model,
                rotation_anim: 0.0,
                gyro_data: [0.0; 3],
                accel_data: [0.0; 3],
                mag_data: [0.0; 3],
            }
        }

        fn update(&mut self, ctx: &mut Context, dt: std::time::Duration) {
            self.update_count += 1;
            let dt_secs = dt.as_secs_f32();
            self.update_log_accum += dt_secs;
            self.sensor_text_refresh_accum += dt_secs;
            self.touch_log_cooldown = (self.touch_log_cooldown - dt_secs).max(0.0);

            if self.update_log_accum >= 1.0 {
                eprintln!("[spot][android] update loop running");
                self.update_log_accum = 0.0;
            }

            self.rotation_anim += dt_secs * 1.5;

            // Update sensor data
            self.gyro_data = spottedcat::gyroscope(ctx).unwrap_or([0.0; 3]);
            self.accel_data = spottedcat::accelerometer(ctx).unwrap_or([0.0; 3]);
            self.mag_data = spottedcat::magnetometer(ctx).unwrap_or([0.0; 3]);
            self.rot_data = spottedcat::rotation(ctx).unwrap_or([0.0; 4]);

            if self.sensor_text_refresh_accum >= 0.1 {
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
                self.sensor_text_refresh_accum = 0.0;
            }
            
            self.step_count = spottedcat::today_step_count(ctx).unwrap_or(0.0);
            self.yesterday_step_count = spottedcat::yesterday_step_count(ctx).unwrap_or(0.0);
            if spottedcat::step_detected(ctx) {
                self.step_detected_timer = 0.5;
            }
            self.step_detected_timer = (self.step_detected_timer - dt_secs).max(0.0);
            self.step_text
                .set_content(format!("Today's Steps: {:.0}", self.step_count));
            self.yesterday_step_text
                .set_content(format!("Yesterday's Steps: {:.0}", self.yesterday_step_count));
            if self.step_detected_timer > 0.0 {
                self.step_text.set_color([1.0, 1.0, 0.0, 1.0]); // Yellow on detection
            } else {
                self.step_text.set_color([1.0, 1.0, 1.0, 1.0]);
            }

            // --- Platform Bridge Example ---
            for event in spottedcat::poll_platform_events(ctx) {
                let PlatformEvent::Event(t, d) = event;
                match t.as_str() {
                    "health_steps_history" => self.history_text.set_content(d),
                    "health_connect_status" => self.health_status_text.set_content(format!("Health: {}", d)),
                    "health_connect_permission" => {
                        self.permission_granted = d == "granted";
                        if d != "granted" {
                            self.history_loaded = false;
                        }
                    }
                    _ => self.bridge_text.set_content(format!("{}: {}", t, d)),
                }
            }

            if self.permission_granted && !self.history_loaded {
                match fetch_health_history_from_rust() {
                    Ok(history) => {
                        self.history_text.set_content(history);
                        self.health_status_text
                            .set_content("Health: Rust JNI history loaded".to_string());
                        self.history_loaded = true;
                    }
                    Err(err) => {
                        self.health_status_text
                            .set_content(format!("Health: Rust JNI history failed: {err}"));
                    }
                }
            }

            // Touch to trigger Kotlin method
            let tapped_this_frame = spottedcat::touches(ctx)
                .iter()
                .any(|touch| touch.phase == TouchPhase::Started);
            if tapped_this_frame {
                if !self.requested_health_permission {
                    match request_health_permission_from_rust() {
                        Ok(()) => {
                            self.health_status_text
                                .set_content("Health: requesting Health Connect permission");
                            self.requested_health_permission = true;
                        }
                        Err(err) => {
                            self.health_status_text
                                .set_content(format!("Health: permission request failed: {err}"));
                        }
                    }
                }

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
            if !current_touches.is_empty() && self.touch_log_cooldown <= 0.0 {
                eprintln!(
                    "[spot][android] active touches count: {}",
                    current_touches.len()
                );
                self.touch_log_cooldown = 0.25;
            }

            for touch in current_touches {
                // Any active touch updates the position
                if self.touch_pos.is_none()
                    || (touch.position.0 - self.touch_pos.unwrap().0)
                        .as_f32()
                        .abs()
                        > 8.0
                {
                    if self.touch_log_cooldown <= 0.0 {
                        eprintln!("[spot][android] touch detected at: {:?}", touch.position);
                        self.touch_log_cooldown = 0.25;
                    }
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
                self.current_ups = self.update_count as f32 / elapsed.as_secs_f32();
                self.fps_text
                    .set_content(format!("FPS: {:.1} | UPS: {:.1}", self.current_fps, self.current_ups));
                self.last_fps_time = now;
                self.frame_count = 0;
                self.update_count = 0;
            }

            let (window_w, window_h) = spottedcat::window_size(ctx);
            let width = window_w.as_f32().max(320.0);
            let height = window_h.as_f32().max(320.0);
            let compact_layout = width < 520.0 || height < 760.0;
            let narrow_layout = width < 420.0;
            let landscape = width > height;

            let side_padding = (width * 0.05).clamp(18.0, 40.0);
            let top_padding = (height * 0.07).clamp(28.0, 64.0);
            let content_width = if landscape {
                (width * 0.36).clamp(280.0, 460.0)
            } else {
                (width - side_padding * 2.0).clamp(240.0, 560.0)
            };
            let content_width_pt = Pt::from(content_width);
            let panel_x = side_padding;

            self.text
                .set_font_size(Pt::from(if narrow_layout { 24.0 } else if compact_layout { 28.0 } else { 32.0 }));
            self.fps_text
                .set_font_size(Pt::from(if narrow_layout { 16.0 } else if compact_layout { 18.0 } else { 22.0 }));
            self.gyro_text
                .set_font_size(Pt::from(if narrow_layout { 14.0 } else if compact_layout { 16.0 } else { 18.0 }));
            self.accel_text
                .set_font_size(Pt::from(if narrow_layout { 14.0 } else if compact_layout { 16.0 } else { 18.0 }));
            self.mag_text
                .set_font_size(Pt::from(if narrow_layout { 14.0 } else if compact_layout { 16.0 } else { 18.0 }));
            self.rot_text
                .set_font_size(Pt::from(if narrow_layout { 14.0 } else if compact_layout { 16.0 } else { 18.0 }));
            self.step_text
                .set_font_size(Pt::from(if narrow_layout { 17.0 } else { 20.0 }));
            self.yesterday_step_text
                .set_font_size(Pt::from(if narrow_layout { 17.0 } else { 20.0 }));
            self.health_status_text
                .set_font_size(Pt::from(if narrow_layout { 15.0 } else if compact_layout { 17.0 } else { 18.0 }));
            self.history_text
                .set_font_size(Pt::from(if narrow_layout { 14.0 } else { 16.0 }));
            self.bridge_text
                .set_font_size(Pt::from(if narrow_layout { 14.0 } else { 16.0 }));

            self.text.set_max_width(Some(content_width_pt));
            self.fps_text.set_max_width(Some(content_width_pt));
            self.gyro_text.set_max_width(Some(content_width_pt));
            self.accel_text.set_max_width(Some(content_width_pt));
            self.mag_text.set_max_width(Some(content_width_pt));
            self.rot_text.set_max_width(Some(content_width_pt));
            self.step_text.set_max_width(Some(content_width_pt));
            self.yesterday_step_text.set_max_width(Some(content_width_pt));
            self.health_status_text.set_max_width(Some(content_width_pt));
            self.history_text.set_max_width(Some(content_width_pt));
            self.bridge_text.set_max_width(Some(content_width_pt));

            // Draw 3D model with gyroscope tilt
            let opts_3d = DrawOption3D::default()
                .with_position([
                    if landscape { 0.9 } else { 0.0 },
                    if landscape { 0.0 } else { 0.25 },
                    0.0,
                ])
                .with_scale([
                    if compact_layout { 0.9 } else { 1.0 },
                    if compact_layout { 0.9 } else { 1.0 },
                    if compact_layout { 0.9 } else { 1.0 },
                ])
                .with_rotation([
                    self.gyro_data[0] * 0.5, 
                    self.rotation_anim + self.gyro_data[1] * 0.5, 
                    self.gyro_data[2] * 0.5
                ]);
            spottedcat::model::draw(ctx, &self.model, opts_3d);

            let mut cursor_y = top_padding;
            let section_gap = if compact_layout { 8.0 } else { 10.0 };
            let block_gap = if compact_layout { 12.0 } else { 16.0 };

            draw_text_block(ctx, &self.text, panel_x, &mut cursor_y, block_gap);
            draw_text_block(ctx, &self.fps_text, panel_x, &mut cursor_y, section_gap);
            draw_text_block(ctx, &self.step_text, panel_x, &mut cursor_y, section_gap);
            draw_text_block(ctx, &self.yesterday_step_text, panel_x, &mut cursor_y, section_gap);
            draw_text_block(ctx, &self.health_status_text, panel_x, &mut cursor_y, section_gap);
            draw_text_block(ctx, &self.history_text, panel_x, &mut cursor_y, block_gap);
            draw_text_block(ctx, &self.gyro_text, panel_x, &mut cursor_y, section_gap);
            draw_text_block(ctx, &self.accel_text, panel_x, &mut cursor_y, section_gap);
            draw_text_block(ctx, &self.mag_text, panel_x, &mut cursor_y, section_gap);
            draw_text_block(ctx, &self.rot_text, panel_x, &mut cursor_y, section_gap);

            let (_, bridge_height): (f32, f32) = spottedcat::text::measure(ctx, &self.bridge_text);
            let bridge_y = if cursor_y + bridge_height + section_gap <= height - side_padding {
                cursor_y
            } else {
                (height - side_padding - bridge_height).max(top_padding)
            };
            spottedcat::text::draw(
                ctx,
                &self.bridge_text,
                DrawOption::default().with_position([Pt::from(panel_x), Pt::from(bridge_y)]),
            );

            let hud_bottom = if bridge_y >= cursor_y {
                bridge_y + bridge_height
            } else {
                cursor_y
            }
            .max(top_padding);
            let image_left = if landscape {
                panel_x + content_width + side_padding
            } else {
                side_padding
            };
            let image_top = if landscape {
                top_padding
            } else {
                (hud_bottom + block_gap).min(height - 120.0)
            };
            let image_right = (width - side_padding).max(image_left + 120.0);
            let image_bottom = (height - side_padding).max(image_top + 120.0);
            let available_width = (image_right - image_left).max(120.0);
            let available_height = (image_bottom - image_top).max(120.0);

            let image_width = self.happy_tree.width().as_f32().max(1.0);
            let image_height = self.happy_tree.height().as_f32().max(1.0);
            let image_scale = (available_width / image_width)
                .min(available_height / image_height)
                .clamp(0.35, 1.0);
            let scaled_width = image_width * image_scale;
            let scaled_height = image_height * image_scale;

            let default_center_x = if landscape {
                image_left + available_width * 0.5
            } else {
                width * 0.5
            };
            let default_center_y = image_top + available_height * 0.5;
            let (desired_x, desired_y) = self
                .touch_pos
                .map(|(x, y)| (x.as_f32(), y.as_f32()))
                .unwrap_or((default_center_x, default_center_y));

            let center_x = desired_x.clamp(
                image_left + scaled_width * 0.5,
                image_right - scaled_width * 0.5,
            );
            let center_y = desired_y.clamp(
                image_top + scaled_height * 0.5,
                image_bottom - scaled_height * 0.5,
            );

            let img_opts = DrawOption::default()
                .with_position([
                    Pt::from(center_x - scaled_width * 0.5),
                    Pt::from(center_y - scaled_height * 0.5),
                ])
                .with_scale([image_scale, image_scale]);
            spottedcat::image::draw(ctx, self.happy_tree, img_opts);
        }

        fn resumed(&mut self, _ctx: &mut Context) {
            eprintln!("[spot][android] resumed called");
            self.requested_health_permission = false;
            self.permission_granted = false;
            self.history_loaded = false;
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
            spottedcat::text::draw(
                ctx,
                &self.text,
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
