use crate::{Context, Key, MouseButton, PlatformEvent, Pt};

/// Returns true if the specified key was just released this frame.
pub fn key_released(ctx: &Context, key: Key) -> bool {
    ctx.input().key_released(key)
}

/// Returns true if the specified mouse button is currently held down.
pub fn mouse_button_down(ctx: &Context, button: MouseButton) -> bool {
    ctx.input().mouse_down(button)
}

/// Returns true if the specified mouse button was just pressed this frame.
pub fn mouse_button_pressed(ctx: &Context, button: MouseButton) -> bool {
    ctx.input().mouse_pressed(button)
}

/// Returns true if the specified mouse button was just released this frame.
pub fn mouse_button_released(ctx: &Context, button: MouseButton) -> bool {
    ctx.input().mouse_released(button)
}

/// Returns the cursor position if the mouse button was just pressed this frame.
pub fn mouse_button_pressed_position(ctx: &Context, button: MouseButton) -> Option<(Pt, Pt)> {
    if mouse_button_pressed(ctx, button) {
        cursor_position(ctx)
    } else {
        None
    }
}

/// Returns the current hardware cursor position in logical coordinates.
pub fn cursor_position(ctx: &Context) -> Option<(Pt, Pt)> {
    ctx.input().cursor_position()
}

/// Returns true if text input (IME) is currently enabled.
pub fn text_input_enabled(ctx: &Context) -> bool {
    ctx.input().text_input_enabled()
}

/// Enables or disables text input (IME) for the window.
pub fn set_text_input_enabled(ctx: &mut Context, enabled: bool) {
    ctx.input_mut().set_text_input_enabled(enabled);
}

/// Returns the accumulated text input string for the current frame.
pub fn text_input(ctx: &Context) -> &str {
    ctx.input().text_input()
}

/// Compatibility alias for `text_input`.
pub fn get_input(ctx: &Context) -> &str {
    ctx.input().text_input()
}

#[cfg(feature = "sensors")]
/// Returns the current gyroscope readings [x, y, z] if available.
pub fn gyroscope(ctx: &Context) -> Option<[f32; 3]> {
    ctx.input().gyroscope()
}

#[cfg(feature = "sensors")]
/// Returns the current accelerometer readings [x, y, z] if available.
pub fn accelerometer(ctx: &Context) -> Option<[f32; 3]> {
    ctx.input().accelerometer()
}

#[cfg(feature = "sensors")]
/// Returns the current magnetometer readings [x, y, z] if available.
pub fn magnetometer(ctx: &Context) -> Option<[f32; 3]> {
    ctx.input().magnetometer()
}

#[cfg(feature = "sensors")]
/// Returns the current device rotation as a quaternion [x, y, z, w].
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
/// Returns the step count for the previous local day.
pub fn yesterday_step_count(ctx: &Context) -> Option<f32> {
    ctx.input().yesterday_step_count()
}

#[cfg(feature = "sensors")]
/// Returns true if a single step was detected this frame.
pub fn step_detected(ctx: &Context) -> bool {
    ctx.input().step_detected()
}

/// Returns a list of raw platform events received this frame.
pub fn poll_platform_events(_ctx: &Context) -> Vec<PlatformEvent> {
    crate::platform_events::poll_events()
}

/// Pushes a custom platform event into the event queue.
pub fn push_platform_event(event: PlatformEvent) {
    crate::platform_events::push_event(event);
}

/// Returns true if any touch point is currently active.
pub fn touch_down(ctx: &Context) -> bool {
    !ctx.input().touches().is_empty()
}

/// Returns the current IME pre-edit string (uncommitted text).
pub fn ime_preedit(ctx: &Context) -> Option<&str> {
    ctx.input().ime_preedit()
}
