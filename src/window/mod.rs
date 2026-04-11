use crate::platform;
use crate::scenes::{SceneFactory, ScenePayloadTypeId, Spot, take_scene_switch_request};
use crate::{Context, WindowConfig};
use std::pin::Pin;
use std::rc::Rc;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web_time::Instant;

pub(crate) type GraphicsInitState = platform::GraphicsInitState;

#[cfg(target_os = "android")]
pub mod android;
#[cfg(all(
    not(target_os = "android"),
    any(
        not(all(target_arch = "wasm32", target_os = "unknown")),
        all(target_arch = "wasm32", target_os = "unknown")
    )
))]
pub mod desktop;
#[cfg(target_os = "ios")]
pub mod ios;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub mod wasm;

#[cfg(target_os = "android")]
pub(crate) use self::android::PlatformData;
#[cfg(all(
    not(target_os = "android"),
    any(
        not(all(target_arch = "wasm32", target_os = "unknown")),
        all(target_arch = "wasm32", target_os = "unknown")
    )
))]
pub(crate) use self::desktop::PlatformData;

/// A helper for running game logic at a fixed frequency while rendering at a variable rate.
///
/// It maintains a 'lag' accumulator of elapsed time and executes updates in discrete steps.
/// This decoupling allows the game simulation to remain deterministic and consistent
/// regardless of the display refresh rate.
pub(crate) struct FixedTimestep {
    previous: Option<Instant>,
    lag: Duration,
    step: Duration,
}

impl FixedTimestep {
    pub(crate) fn new(step: Duration) -> Self {
        Self {
            previous: None,
            lag: Duration::ZERO,
            step,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.previous = Some(Instant::now());
        self.lag = Duration::ZERO;
    }

    pub(crate) fn run_updates(
        &mut self,
        max_updates: usize,
        mut update: impl FnMut(Duration),
    ) -> usize {
        let now = Instant::now();
        if let Some(previous) = self.previous.replace(now) {
            let elapsed = now.duration_since(previous);
            self.lag = self.lag.saturating_add(elapsed);

            let mut updates = 0;
            while self.lag >= self.step && updates < max_updates {
                update(self.step);
                self.lag = self.lag.saturating_sub(self.step);
                updates += 1;
            }

            if self.lag >= self.step {
                self.lag = Duration::ZERO;
            }

            updates
        } else {
            0
        }
    }

    pub(crate) fn next_deadline(&self) -> Instant {
        let now = Instant::now();
        match self.previous {
            Some(previous) => previous + self.step.saturating_sub(self.lag),
            None => now,
        }
    }

    pub(crate) fn alpha(&self) -> f32 {
        (self.lag.as_secs_f64() / self.step.as_secs_f64()).min(1.0) as f32
    }
}

pub(crate) struct SceneHost {
    spot: Option<Box<dyn Spot>>,
    factory: SceneFactory,
    is_floating_scene: bool,
}

impl SceneHost {
    pub(crate) fn new<T: Spot + 'static>() -> Self {
        Self {
            spot: None,
            factory: Box::new(|ctx| Box::new(T::initialize(ctx))),
            is_floating_scene: false,
        }
    }

    pub(crate) fn spot_mut(&mut self) -> Option<&mut Box<dyn Spot>> {
        self.spot.as_mut()
    }

    pub(crate) fn has_active_scene(&self) -> bool {
        self.spot.is_some()
    }

    #[cfg(target_os = "android")]
    pub(crate) fn needs_initial_scene(&self) -> bool {
        self.spot.is_none()
    }

    #[cfg(target_os = "android")]
    pub(crate) fn is_floating_scene(&self) -> bool {
        self.is_floating_scene
    }

    #[cfg(target_os = "android")]
    pub(crate) fn mark_floating(&mut self) {
        self.is_floating_scene = true;
    }

    #[cfg(target_os = "android")]
    pub(crate) fn clear_floating(&mut self) {
        self.is_floating_scene = false;
    }

    #[cfg(target_os = "android")]
    pub(crate) fn set_active_scene(&mut self, spot: Box<dyn Spot>) {
        self.spot = Some(spot);
    }

    pub(crate) fn remove_current(&mut self, ctx: &mut Context) {
        if let Some(mut spot) = self.spot.take() {
            spot.remove(ctx);
        }
    }

    #[cfg(target_os = "android")]
    pub(crate) fn restore_root_scene(&mut self, ctx: &mut Context) {
        self.remove_current(ctx);
        self.spot = Some((self.factory)(ctx));
        self.is_floating_scene = false;
    }

    #[cfg(not(target_os = "android"))]
    pub(crate) fn initialize_if_missing(&mut self, ctx: &mut Context) {
        if self.spot.is_none() {
            self.spot = Some((self.factory)(ctx));
        }
    }

    pub(crate) fn apply_pending_switch(&mut self, ctx: &mut Context) -> bool {
        let Some(request) = take_scene_switch_request() else {
            return false;
        };

        clear_scene_payload(ctx);
        if let Some(payload) = request.payload {
            ctx.insert_resource_dyn(payload.type_id, payload.value);
            ctx.insert_resource(Rc::new(ScenePayloadTypeId(payload.type_id)));
        }

        self.remove_current(ctx);
        self.spot = Some((request.factory)(ctx));
        self.is_floating_scene = false;
        true
    }
}

fn clear_scene_payload(ctx: &mut Context) {
    if let Some(last) = ctx.take_resource::<ScenePayloadTypeId>() {
        ctx.take_resource_dyn(last.0);
    }
}

pub(crate) struct App {
    pub(crate) platform: PlatformData,
    pub(crate) instance: wgpu::Instance,
    pub(crate) surface: Option<wgpu::Surface<'static>>,
    pub(crate) ctx: Pin<Box<Context>>,
    pub(crate) scene: SceneHost,
    pub(crate) window_config: WindowConfig,
    pub(crate) init_state: GraphicsInitState,
    pub(crate) scale_factor: f64,
    pub(crate) timing: FixedTimestep,
}

pub(crate) fn make_screen_target(ctx: &Context) -> crate::Image {
    let (width, height) = ctx.window_logical_size();
    crate::Image {
        id: 0,
        texture_id: 0,
        x: crate::Pt(0.0),
        y: crate::Pt(0.0),
        width,
        height,
        pixel_bounds: crate::image::PixelBounds {
            x: 0,
            y: 0,
            width: width.to_u32_clamped().max(1),
            height: height.to_u32_clamped().max(1),
        },
    }
}

impl App {
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub(crate) fn new<T: Spot + 'static>(window_config: WindowConfig) -> Self {
        let instance = platform::create_wgpu_instance();

        Self {
            platform: PlatformData::new(),
            instance,
            surface: None,
            ctx: Box::pin(Context::new()),
            scene: SceneHost::new::<T>(),
            window_config,
            init_state: GraphicsInitState::NotStarted,
            scale_factor: 1.0,
            timing: FixedTimestep::new(Duration::from_secs_f64(1.0 / 60.0)),
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
            ctx: Box::pin(Context::new()),
            scene: SceneHost::new::<T>(),
            window_config,
            init_state: GraphicsInitState::NotStarted,
            scale_factor: 1.0,
            timing: FixedTimestep::new(Duration::from_nanos(8_333_333)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{switch_scene, switch_scene_with};

    struct RootScene;
    struct PayloadScene;
    struct FinalScene;

    impl Spot for RootScene {
        fn initialize(_ctx: &mut Context) -> Self {
            Self
        }

        fn draw(&mut self, _ctx: &mut Context, _screen: crate::Image) {}

        fn update(&mut self, _ctx: &mut Context, _dt: Duration) {}
    }

    impl Spot for PayloadScene {
        fn initialize(_ctx: &mut Context) -> Self {
            Self
        }

        fn draw(&mut self, _ctx: &mut Context, _screen: crate::Image) {}

        fn update(&mut self, _ctx: &mut Context, _dt: Duration) {}
    }

    impl Spot for FinalScene {
        fn initialize(_ctx: &mut Context) -> Self {
            Self
        }

        fn draw(&mut self, _ctx: &mut Context, _screen: crate::Image) {}

        fn update(&mut self, _ctx: &mut Context, _dt: Duration) {}
    }

    #[derive(Debug, PartialEq)]
    struct PayloadA(&'static str);

    #[derive(Debug, PartialEq)]
    struct PayloadB(&'static str);

    #[test]
    fn scene_switch_replaces_previous_payload() {
        let _ = take_scene_switch_request();

        let mut ctx = Context::new();
        let mut host = SceneHost::new::<RootScene>();
        host.initialize_if_missing(&mut ctx);

        switch_scene_with::<PayloadScene, _>(PayloadA("first"));
        assert!(host.apply_pending_switch(&mut ctx));
        assert_eq!(
            ctx.get_resource::<PayloadA>().as_deref().map(|p| p.0),
            Some("first")
        );

        switch_scene_with::<FinalScene, _>(PayloadB("second"));
        assert!(host.apply_pending_switch(&mut ctx));

        assert!(ctx.get_resource::<PayloadA>().is_none());
        assert_eq!(
            ctx.get_resource::<PayloadB>().as_deref().map(|p| p.0),
            Some("second")
        );
    }

    #[test]
    fn scene_switch_without_payload_clears_previous_payload() {
        let _ = take_scene_switch_request();

        let mut ctx = Context::new();
        let mut host = SceneHost::new::<RootScene>();
        host.initialize_if_missing(&mut ctx);

        switch_scene_with::<PayloadScene, _>(PayloadA("first"));
        assert!(host.apply_pending_switch(&mut ctx));
        assert!(ctx.get_resource::<PayloadA>().is_some());

        switch_scene::<FinalScene>();
        assert!(host.apply_pending_switch(&mut ctx));

        assert!(ctx.get_resource::<PayloadA>().is_none());
        assert!(ctx.get_resource::<ScenePayloadTypeId>().is_none());
    }

    #[test]
    fn app_context_address_stays_stable_when_app_moves() {
        let app = App::new::<RootScene>(crate::WindowConfig::default());
        let before_move = app.ctx.as_ref().get_ref() as *const Context;

        let app = Some(app).unwrap();
        let after_move = app.ctx.as_ref().get_ref() as *const Context;

        assert_eq!(before_move, after_move);
    }
}
