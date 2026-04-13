#[cfg(target_os = "ios")]
use block2::RcBlock;
#[cfg(target_os = "ios")]
use objc2::{msg_send, rc::Retained};
#[cfg(target_os = "ios")]
use objc2_core_motion::CMPedometer;
#[cfg(target_os = "ios")]
use objc2_foundation::{MainThreadMarker, NSDate};

#[cfg(target_os = "ios")]
const STEP_PERMISSION_EVENT: &str = "step_permission";
#[cfg(target_os = "ios")]
const MOTION_PERMISSION: &str = "motion";
#[cfg(target_os = "ios")]
const CM_AUTHORIZED: isize = 3;

#[cfg(target_os = "ios")]
fn motion_authorization_status() -> isize {
    unsafe { msg_send![objc2::class!(CMPedometer), authorizationStatus] }
}

#[cfg(target_os = "ios")]
pub fn has_runtime_permission(permission: &str) -> Result<bool, String> {
    match permission {
        MOTION_PERMISSION => Ok(motion_authorization_status() == CM_AUTHORIZED),
        _ => Err(format!("unsupported iOS permission: {permission}")),
    }
}

#[cfg(target_os = "ios")]
pub fn request_runtime_permission(permission: &str) -> Result<(), String> {
    if permission != MOTION_PERMISSION {
        return Err(format!("unsupported iOS permission: {permission}"));
    }

    let _mtm = MainThreadMarker::new()
        .ok_or_else(|| "motion permission request must run on the iOS main thread".to_string())?;

    unsafe {
        if !CMPedometer::isStepCountingAvailable() {
            return Err("step counting is unavailable on this device".to_string());
        }

        let pedometer = CMPedometer::new();
        let pedometer_for_handler: Retained<CMPedometer> = pedometer.clone();
        let start = NSDate::date();
        let handler = RcBlock::new(
            move |_data: *mut objc2_core_motion::CMPedometerData,
                  _error: *mut objc2_foundation::NSError| {
                let granted = motion_authorization_status() == CM_AUTHORIZED;
                pedometer_for_handler.stopPedometerUpdates();
                crate::push_platform_event(crate::PlatformEvent::Event(
                    STEP_PERMISSION_EVENT.to_string(),
                    if granted { "granted" } else { "denied" }.to_string(),
                ));
            },
        );

        pedometer.startPedometerUpdatesFromDate_withHandler(&start, &*handler);
    }

    Ok(())
}
