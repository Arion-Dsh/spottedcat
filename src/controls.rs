use crate::{Context, Key, MouseButton, PlatformEvent, Pt, TouchInfo};

pub fn key_down(ctx: &Context, key: Key) -> bool {
    ctx.input().key_down(key)
}

pub fn key_pressed(ctx: &Context, key: Key) -> bool {
    ctx.input().key_pressed(key)
}

pub fn key_released(ctx: &Context, key: Key) -> bool {
    ctx.input().key_released(key)
}

pub fn mouse_button_down(ctx: &Context, button: MouseButton) -> bool {
    ctx.input().mouse_down(button)
}

pub fn mouse_button_pressed(ctx: &Context, button: MouseButton) -> bool {
    ctx.input().mouse_pressed(button)
}

pub fn mouse_button_released(ctx: &Context, button: MouseButton) -> bool {
    ctx.input().mouse_released(button)
}

pub fn mouse_button_pressed_position(ctx: &Context, button: MouseButton) -> Option<(Pt, Pt)> {
    if mouse_button_pressed(ctx, button) {
        cursor_position(ctx)
    } else {
        None
    }
}

pub fn window_size(ctx: &Context) -> (Pt, Pt) {
    ctx.window_logical_size()
}

pub fn cursor_position(ctx: &Context) -> Option<(Pt, Pt)> {
    ctx.input().cursor_position()
}

pub fn text_input_enabled(ctx: &Context) -> bool {
    ctx.input().text_input_enabled()
}

pub fn set_text_input_enabled(ctx: &mut Context, enabled: bool) {
    ctx.input_mut().set_text_input_enabled(enabled);
}

pub fn text_input(ctx: &Context) -> &str {
    ctx.input().text_input()
}

pub fn get_input(ctx: &Context) -> &str {
    ctx.input().text_input()
}

pub fn touches(ctx: &Context) -> &[TouchInfo] {
    ctx.input().touches()
}

#[cfg(feature = "sensors")]
pub fn gyroscope(ctx: &Context) -> Option<[f32; 3]> {
    ctx.input().gyroscope()
}

#[cfg(feature = "sensors")]
pub fn accelerometer(ctx: &Context) -> Option<[f32; 3]> {
    ctx.input().accelerometer()
}

#[cfg(feature = "sensors")]
pub fn magnetometer(ctx: &Context) -> Option<[f32; 3]> {
    ctx.input().magnetometer()
}

#[cfg(feature = "sensors")]
pub fn rotation(ctx: &Context) -> Option<[f32; 4]> {
    ctx.input().rotation()
}

#[cfg(feature = "sensors")]
/// Returns the current day's step count when the platform provides it.
///
/// On iOS this is sourced from `CMPedometer` starting at the beginning of the
/// current local day. On Android this is derived from the step counter sensor
/// while the app keeps the sensor registered.
pub fn today_step_count(ctx: &Context) -> Option<f32> {
    ctx.input().today_step_count()
}

#[cfg(feature = "sensors")]
/// Compatibility alias for `today_step_count`.
///
/// This is not a lifetime or historical total.
pub fn step_count(ctx: &Context) -> Option<f32> {
    today_step_count(ctx)
}

#[cfg(feature = "sensors")]
pub fn yesterday_step_count(ctx: &Context) -> Option<f32> {
    ctx.input().yesterday_step_count()
}

#[cfg(feature = "sensors")]
pub fn step_detected(ctx: &Context) -> bool {
    ctx.input().step_detected()
}

pub fn poll_platform_events(_ctx: &Context) -> Vec<PlatformEvent> {
    crate::platform_events::poll_events()
}

pub fn push_platform_event(event: PlatformEvent) {
    crate::platform_events::push_event(event);
}

pub fn touch_down(ctx: &Context) -> bool {
    !ctx.input().touches().is_empty()
}

pub fn ime_preedit(ctx: &Context) -> Option<&str> {
    ctx.input().ime_preedit()
}
