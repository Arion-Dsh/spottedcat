use crate::{Spot, WindowConfig};

#[cfg(target_os = "android")]
use android_activity::AndroidApp;

pub(crate) trait WindowBackend {
    #[cfg(not(target_os = "android"))]
    fn run<T: Spot + 'static>(window: WindowConfig);

    #[cfg(target_os = "android")]
    fn run<T: Spot + 'static>(window: WindowConfig, app: AndroidApp);
}
