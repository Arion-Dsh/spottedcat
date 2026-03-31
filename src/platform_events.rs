use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::Mutex;
use std::sync::OnceLock;

/// Events received from the native platform (e.g. Android JNI or iOS ObjC).
#[derive(Debug, Clone)]
pub enum PlatformEvent {
    /// A general purpose event from the platform.
    /// (event_type, data) - 'data' can be a JSON string or any format the app expects.
    Event(String, String),
}

static PENDING_EVENTS: OnceLock<Mutex<Vec<PlatformEvent>>> = OnceLock::new();

/// Pushes a new event from the platform to the engine.
///
/// This can be called from any thread (e.g. from a JNI callback).
pub fn push_event(event: PlatformEvent) {
    let mutex = PENDING_EVENTS.get_or_init(|| Mutex::new(Vec::new()));
    if let Ok(mut events) = mutex.lock() {
        events.push(event);
    }
}

/// Polls all pending platform events and clears the queue.
pub fn poll_events() -> Vec<PlatformEvent> {
    if let Some(mutex) = PENDING_EVENTS.get()
        && let Ok(mut events) = mutex.lock()
    {
        return events.drain(..).collect();
    }
    Vec::new()
}

/// C-compatible API for pushing events from native code (e.g. iOS Objective-C).
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn spottedcat_push_platform_event(
    event_type: *const c_char,
    data: *const c_char,
) {
    if event_type.is_null() || data.is_null() {
        return;
    }

    let t = unsafe { CStr::from_ptr(event_type as *const _) }
        .to_string_lossy()
        .into_owned();
    let d = unsafe { CStr::from_ptr(data as *const _) }
        .to_string_lossy()
        .into_owned();

    push_event(PlatformEvent::Event(t, d));
}
