#[cfg(all(target_os = "ios", feature = "sensors"))]
use objc2_core_motion::{CMMotionManager, CMAccelerometerData, CMGyroData, CMMagnetometerData, CMDeviceMotion, CMPedometer};
#[cfg(all(target_os = "ios", feature = "sensors"))]
use objc2_foundation::{NSOperationQueue, MainThreadMarker, NSDate};
#[cfg(all(target_os = "ios", feature = "sensors"))]
use objc2::rc::Retained;
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
        let mtm = MainThreadMarker::new().expect("must be on main thread to initialize CMMotionManager");
        let manager = CMMotionManager::new(mtm);
        
        // Settings intervals (50Hz)
        manager.setAccelerometerUpdateInterval(0.02);
        manager.setGyroUpdateInterval(0.02);
        manager.setMagnetometerUpdateInterval(0.02);
        manager.setDeviceMotionUpdateInterval(0.02);
        
        let pedometer = CMPedometer::new();
        
        Self { 
            manager, 
            pedometer, 
            latest_steps: Arc::new(Mutex::new(None)),
        }
    }

    pub fn enable(&self) {
        if self.manager.isAccelerometerAvailable() { self.manager.startAccelerometerUpdates(); }
        if self.manager.isGyroAvailable() { self.manager.startGyroUpdates(); }
        if self.manager.isMagnetometerAvailable() { self.manager.startMagnetometerUpdates(); }
        if self.manager.isDeviceMotionAvailable() { self.manager.startDeviceMotionUpdates(); }
        
        if CMPedometer::isStepCountingAvailable() {
            let latest_steps = self.latest_steps.clone();
            let now = NSDate::date();
            unsafe {
                self.pedometer.startPedometerUpdatesFromDate_withHandler(&now, move |data, error| {
                    if let Some(data) = data {
                        let steps = data.numberOfSteps().intValue();
                        if let Ok(mut latest) = latest_steps.lock() {
                            *latest = Some(steps as f32);
                        }
                    }
                });
            }
        }
    }

    pub fn disable(&self) {
        self.manager.stopAccelerometerUpdates();
        self.manager.stopGyroUpdates();
        self.manager.stopMagnetometerUpdates();
        self.manager.stopDeviceMotionUpdates();
        self.pedometer.stopPedometerUpdates();
    }

    pub fn poll(&self, input: &mut crate::InputManager) {
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
            let quat = data.attitude().quaternion();
            // CoreMotion quaternion is (x, y, z, w) where w is scalar
            input.handle_rotation(quat.x as f32, quat.y as f32, quat.z as f32, quat.w as f32);
        }
        
        if let Ok(steps) = self.latest_steps.lock() {
            if let Some(count) = *steps {
                input.handle_step_counter(count);
            }
        }
    }
}
