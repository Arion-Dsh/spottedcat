use crate::Context;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

pub(crate) type SceneFactory = Box<dyn Fn(&mut Context) -> Box<dyn Spot> + Send + Sync>;

pub(crate) struct ScenePayload {
    pub(crate) type_id: TypeId,
    pub(crate) value: Rc<dyn Any>,
}

pub(crate) struct ScenePayloadTypeId(pub(crate) TypeId);

pub(crate) struct SceneSwitchRequest {
    pub(crate) factory: SceneFactory,
    pub(crate) payload: Option<ScenePayload>,
}

thread_local! {
    static SCENE_SWITCH_REQUEST: RefCell<Option<SceneSwitchRequest>> = const { RefCell::new(None) };
    static QUIT_REQUEST: RefCell<bool> = const { RefCell::new(false) };
}

fn request_scene_switch<F>(factory: F)
where
    F: Fn(&mut Context) -> Box<dyn Spot> + Send + Sync + 'static,
{
    SCENE_SWITCH_REQUEST.with(|request| {
        *request.borrow_mut() = Some(SceneSwitchRequest {
            factory: Box::new(factory),
            payload: None,
        });
    });
}

fn request_scene_switch_with<F>(factory: F, payload: ScenePayload)
where
    F: Fn(&mut Context) -> Box<dyn Spot> + Send + Sync + 'static,
{
    SCENE_SWITCH_REQUEST.with(|request| {
        *request.borrow_mut() = Some(SceneSwitchRequest {
            factory: Box::new(factory),
            payload: Some(payload),
        });
    });
}

pub(crate) fn take_scene_switch_request() -> Option<SceneSwitchRequest> {
    SCENE_SWITCH_REQUEST.with(|request| request.borrow_mut().take())
}

/// Signals the engine to quit the application.
pub fn quit() {
    QUIT_REQUEST.with(|request| *request.borrow_mut() = true);
}

pub(crate) fn take_quit_request() -> bool {
    QUIT_REQUEST.with(|request| request.replace(false))
}

/// Switches to a new scene of type `T`.
///
/// The current scene will be removed and the new scene will be initialized.
pub fn switch_scene<T: Spot + 'static>() {
    request_scene_switch(|ctx| Box::new(T::initialize(ctx)));
}

/// Switches to a new scene of type `T` and passes a payload.
///
/// The payload can be retrieved in the new scene's `initialize` method
/// using `ctx.take_resource::<P>()`.
pub fn switch_scene_with<T: Spot + 'static, P: Any>(payload: P) {
    request_scene_switch_with(
        |ctx| Box::new(T::initialize(ctx)),
        ScenePayload {
            type_id: TypeId::of::<P>(),
            value: Rc::new(payload),
        },
    );
}

/// The core trait for defining application logic and rendering.
///
/// Implement this trait on your application state struct to handle lifecycle
/// events, updates, and drawing.
pub trait Spot {
    /// Initializes the scene. This is called once when the scene is created.
    fn initialize(ctx: &mut Context) -> Self
    where
        Self: Sized;

    /// Called every frame to draw the scene.
    fn draw(&mut self, ctx: &mut Context);

    /// Called every frame to update the scene logic.
    ///
    /// # Arguments
    /// * `dt` - The time elapsed since the last frame.
    fn update(&mut self, ctx: &mut Context, dt: Duration);

    /// Called when the application is resumed (e.g., from background).
    fn resumed(&mut self, _ctx: &mut Context) {}

    /// Called when the application is suspended (e.g., to background).
    fn suspended(&mut self, _ctx: &mut Context) {}

    /// Called when the scene is being removed or the application is quitting.
    fn remove(&mut self, _ctx: &mut Context) {}
}
