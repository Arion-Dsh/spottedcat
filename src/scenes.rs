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

pub fn quit() {
    QUIT_REQUEST.with(|request| *request.borrow_mut() = true);
}

pub(crate) fn take_quit_request() -> bool {
    QUIT_REQUEST.with(|request| request.replace(false))
}

pub fn switch_scene<T: Spot + 'static>() {
    request_scene_switch(|ctx| Box::new(T::initialize(ctx)));
}

pub fn switch_scene_with<T: Spot + 'static, P: Any>(payload: P) {
    request_scene_switch_with(
        |ctx| Box::new(T::initialize(ctx)),
        ScenePayload {
            type_id: TypeId::of::<P>(),
            value: Rc::new(payload),
        },
    );
}

pub trait Spot {
    fn initialize(ctx: &mut Context) -> Self
    where
        Self: Sized;

    fn draw(&mut self, ctx: &mut Context);

    fn update(&mut self, ctx: &mut Context, dt: Duration);

    fn resumed(&mut self, _ctx: &mut Context) {}

    fn suspended(&mut self, _ctx: &mut Context) {}

    fn remove(&mut self, _ctx: &mut Context) {}
}
