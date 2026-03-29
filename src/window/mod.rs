use crate::platform;
use crate::{
    Context, Spot, WindowConfig,
};
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web_time::Instant;

pub(crate) type GraphicsInitState = platform::GraphicsInitState;

#[cfg(target_os = "android")]
pub mod android;
#[cfg(target_os = "ios")]
pub mod ios;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub mod wasm;
#[cfg(all(not(target_os = "android"), not(target_os = "ios"), not(all(target_arch = "wasm32", target_os = "unknown"))))]
pub mod desktop;

#[cfg(target_os = "android")]
pub(crate) use self::android::PlatformData;
#[cfg(target_os = "ios")]
pub(crate) use self::desktop::PlatformData;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub(crate) use self::wasm::PlatformData;
#[cfg(all(not(target_os = "android"), not(target_os = "ios"), not(all(target_arch = "wasm32", target_os = "unknown"))))]
pub(crate) use self::desktop::PlatformData;

pub(crate) struct App {
    pub(crate) platform: PlatformData,
    pub(crate) instance: wgpu::Instance,
    pub(crate) surface: Option<wgpu::Surface<'static>>,
    pub(crate) context: Context,
    pub(crate) spot: Option<Box<dyn Spot>>,
    pub(crate) scene_factory: Box<dyn Fn(&mut Context) -> Box<dyn Spot> + Send + Sync>,
    #[cfg(not(target_os = "android"))]
    pub(crate) window_config: WindowConfig,
    pub(crate) init_state: GraphicsInitState,
    pub(crate) scale_factor: f64,
    pub(crate) previous: Option<Instant>,
    pub(crate) lag: Duration,
    pub(crate) fixed_dt: Duration,
}

impl App {
    pub(crate) fn new<T: Spot + 'static>(window_config: WindowConfig) -> Self {
        #[cfg(target_os = "android")]
        let _ = window_config;
        let instance = platform::create_wgpu_instance();
        let audio = crate::audio::AudioSystem::new().expect("failed to initialize audio system");
        let _ = platform::set_global_audio(audio);

        Self {
            platform: PlatformData::new(),
            instance,
            surface: None,
            context: Context::new(),
            spot: None,
            scene_factory: Box::new(|ctx| Box::new(T::initialize(ctx))),
            #[cfg(not(target_os = "android"))]
            window_config,
            init_state: GraphicsInitState::NotStarted,
            scale_factor: 1.0,
            previous: None,
            lag: Duration::ZERO,
            fixed_dt: Duration::from_secs_f64(1.0 / 60.0),
        }
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    pub(crate) fn new_wasm<T: Spot + 'static>(
        window_config: WindowConfig,
        canvas_id: Option<String>,
    ) -> Self {
        let instance = platform::create_wgpu_instance();
        Self {
            platform: PlatformData::new_wasm(canvas_id),
            instance,
            surface: None,
            context: Context::new(),
            spot: None,
            scene_factory: Box::new(|ctx| Box::new(T::initialize(ctx))),
            #[cfg(not(target_os = "android"))]
            window_config,
            init_state: GraphicsInitState::NotStarted,
            scale_factor: 1.0,
            previous: None,
            lag: Duration::ZERO,
            fixed_dt: Duration::from_nanos(8_333_333),
        }
    }
}
