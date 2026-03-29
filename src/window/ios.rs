#[cfg(all(target_os = "ios", feature = "sensors"))]
use objc2::{msg_send, rc::Retained};
#[cfg(all(target_os = "ios", feature = "sensors"))]
use objc2_core_motion::{
    CMMotionManager, CMPedometer, CMPedometerData,
};
#[cfg(all(target_os = "ios", feature = "sensors"))]
use objc2_foundation::MainThreadMarker;
#[cfg(all(target_os = "ios", feature = "sensors"))]
use std::sync::{Arc, Mutex};

#[cfg(all(target_os = "ios", feature = "sensors"))]
pub(crate) struct IosSensorState {
    manager: Retained<CMMotionManager>,
    pedometer: Retained<CMPedometer>,
    latest_steps: Arc<Mutex<Option<f32>>>,
}

#[cfg(all(target_os = "ios", feature = "sensors"))]
impl IosSensorState {
    pub fn new() -> Self {
        let _mtm =
            MainThreadMarker::new().expect("must be on main thread to initialize CMMotionManager");
        
        let manager = unsafe { CMMotionManager::new() };

        // Settings intervals (50Hz)
        unsafe {
            let _: () = msg_send![&manager, setAccelerometerUpdateInterval: 0.02f64];
            let _: () = msg_send![&manager, setGyroUpdateInterval: 0.02f64];
            let _: () = msg_send![&manager, setMagnetometerUpdateInterval: 0.02f64];
            let _: () = msg_send![&manager, setDeviceMotionUpdateInterval: 0.02f64];
        }

        let pedometer = unsafe { CMPedometer::new() };

        Self {
            manager,
            pedometer,
            latest_steps: Arc::new(Mutex::new(None)),
        }
    }

    pub fn enable(&self) {
        unsafe {
            let accel_avail: bool = msg_send![&self.manager, isAccelerometerAvailable];
            if accel_avail {
                let _: () = msg_send![&self.manager, startAccelerometerUpdates];
            }
            let gyro_avail: bool = msg_send![&self.manager, isGyroAvailable];
            if gyro_avail {
                let _: () = msg_send![&self.manager, startGyroUpdates];
            }
            let mag_avail: bool = msg_send![&self.manager, isMagnetometerAvailable];
            if mag_avail {
                let _: () = msg_send![&self.manager, startMagnetometerUpdates];
            }
            let motion_avail: bool = msg_send![&self.manager, isDeviceMotionAvailable];
            if motion_avail {
                let _: () = msg_send![&self.manager, startDeviceMotionUpdates];
            }

            let pedo_avail: bool = msg_send![objc2::class!(CMPedometer), isStepCountingAvailable];
            if pedo_avail {
                let latest_steps = self.latest_steps.clone();
                let now: Retained<objc2_foundation::NSDate> = msg_send![objc2::class!(NSDate), date];
                let handler = block2::RcBlock::new(
                    move |data: *mut CMPedometerData, _error: *mut objc2::runtime::AnyObject| {
                        if !data.is_null() {
                            let data = &*data;
                            // Get steps via msg_send! to avoid needing NSNumber feature
                            let steps_obj: *mut objc2::runtime::AnyObject = msg_send![data, numberOfSteps];
                            if !steps_obj.is_null() {
                                let steps: i32 = msg_send![steps_obj, intValue];
                                if let Ok(mut latest) = latest_steps.lock() {
                                    *latest = Some(steps as f32);
                                }
                            }
                        }
                    },
                );
                let _: () = msg_send![&self.pedometer, startPedometerUpdatesFromDate: &*now, withHandler: &*handler];
            }
        }
    }

    pub fn disable(&self) {
        unsafe {
            let _: () = msg_send![&self.manager, stopAccelerometerUpdates];
            let _: () = msg_send![&self.manager, stopGyroUpdates];
            let _: () = msg_send![&self.manager, stopMagnetometerUpdates];
            let _: () = msg_send![&self.manager, stopDeviceMotionUpdates];
            let _: () = msg_send![&self.pedometer, stopPedometerUpdates];
        }
    }

    pub fn poll(&self, input: &mut crate::InputManager) {
        unsafe {
            if let Some(data) = self.manager.accelerometerData() {
                let accel = data.acceleration();
                input.handle_accelerometer(accel.x as f32, accel.y as f32, accel.z as f32);
            }

            if let Some(data) = self.manager.gyroData() {
                let rate = data.rotationRate();
                input.handle_gyroscope(rate.x as f32, rate.y as f32, rate.z as f32);
            }

            if let Some(data) = self.manager.magnetometerData() {
                let field = data.magneticField();
                input.handle_magnetometer(field.x as f32, field.y as f32, field.z as f32);
            }

            if let Some(data) = self.manager.deviceMotion() {
                let attitude = data.attitude();
                let quat = attitude.quaternion();
                input.handle_rotation(quat.x as f32, quat.y as f32, quat.z as f32, quat.w as f32);
            }
        }

        if let Ok(steps) = self.latest_steps.lock() {
            if let Some(count) = *steps {
                input.handle_step_counter(count);
            }
        }
    }
}
