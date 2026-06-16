use crate::input::InputManager;

/// Stable identifier assigned to a connected gamepad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GamepadId(pub u32);

/// Basic information about a gamepad known to the input system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GamepadInfo {
    pub id: GamepadId,
    pub name: String,
    pub connected: bool,
}

/// Logical gamepad buttons exposed by spottedcat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    South,
    East,
    West,
    North,
    C,
    Z,
    LeftShoulder,
    LeftTrigger,
    RightShoulder,
    RightTrigger,
    Select,
    Start,
    Mode,
    LeftThumb,
    RightThumb,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    Other(u16),
}

/// Logical gamepad axes exposed by spottedcat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadAxis {
    LeftX,
    LeftY,
    LeftTrigger,
    RightX,
    RightY,
    RightTrigger,
    DPadX,
    DPadY,
    Other(u16),
}

pub(crate) trait GamepadBackend {
    fn poll(&mut self, input: &mut InputManager);
}

pub(crate) struct GamepadRuntime {
    backend: Box<dyn GamepadBackend>,
}

impl GamepadRuntime {
    pub(crate) fn new() -> Self {
        Self {
            backend: default_backend(),
        }
    }

    pub(crate) fn poll(&mut self, input: &mut InputManager) {
        self.backend.poll(input);
    }
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn default_backend() -> Box<dyn GamepadBackend> {
    Box::new(gilrs_backend::GilrsGamepadBackend::new())
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn default_backend() -> Box<dyn GamepadBackend> {
    Box::new(NoopGamepadBackend)
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
struct NoopGamepadBackend;

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
impl GamepadBackend for NoopGamepadBackend {
    fn poll(&mut self, _input: &mut InputManager) {}
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
mod gilrs_backend {
    use super::{GamepadAxis, GamepadBackend, GamepadButton, GamepadId};
    use crate::input::InputManager;
    use gilrs::{
        Axis as GilrsAxis, Button as GilrsButton, Event, EventType, GamepadId as GilrsGamepadId,
        Gilrs,
    };
    use std::collections::HashMap;

    pub(super) struct GilrsGamepadBackend {
        gilrs: Option<Gilrs>,
        ids: HashMap<GilrsGamepadId, GamepadId>,
        next_id: u32,
    }

    impl GilrsGamepadBackend {
        pub(super) fn new() -> Self {
            let gilrs = match Gilrs::new() {
                Ok(gilrs) => Some(gilrs),
                Err(err) => {
                    eprintln!("[spot][gamepad] gilrs initialization failed: {err}");
                    None
                }
            };

            let mut backend = Self {
                gilrs,
                ids: HashMap::new(),
                next_id: 0,
            };
            backend.seed_connected_gamepads();
            backend
        }

        fn seed_connected_gamepads(&mut self) {
            let Some(gilrs) = self.gilrs.as_ref() else {
                return;
            };

            let ids: Vec<_> = gilrs
                .gamepads()
                .map(|(gilrs_id, _gamepad)| gilrs_id)
                .collect();
            for gilrs_id in ids {
                self.ensure_id(gilrs_id);
            }
        }

        fn ensure_id(&mut self, gilrs_id: GilrsGamepadId) -> GamepadId {
            if let Some(id) = self.ids.get(&gilrs_id) {
                return *id;
            }

            let id = GamepadId(self.next_id);
            self.next_id = self.next_id.saturating_add(1);
            self.ids.insert(gilrs_id, id);
            id
        }

        fn gamepad_name(&self, gilrs_id: GilrsGamepadId) -> String {
            self.gilrs
                .as_ref()
                .map(|gilrs| gilrs.gamepad(gilrs_id).name().to_string())
                .unwrap_or_else(|| "Gamepad".to_string())
        }

        fn handle_event(&mut self, event: Event, input: &mut InputManager) {
            let id = self.ensure_id(event.id);
            match event.event {
                EventType::Connected => {
                    input.handle_gamepad_connected(id, self.gamepad_name(event.id));
                }
                EventType::Disconnected => {
                    input.handle_gamepad_disconnected(id);
                }
                EventType::ButtonPressed(button, _) => {
                    input.handle_gamepad_button(id, map_button(button), true);
                }
                EventType::ButtonReleased(button, _) => {
                    input.handle_gamepad_button(id, map_button(button), false);
                }
                EventType::ButtonChanged(button, value, _) => {
                    input.handle_gamepad_button(id, map_button(button), value >= 0.5);
                }
                EventType::AxisChanged(axis, value, _) => {
                    input.handle_gamepad_axis(id, map_axis(axis), value);
                }
                _ => {}
            }
        }
    }

    impl GamepadBackend for GilrsGamepadBackend {
        fn poll(&mut self, input: &mut InputManager) {
            let Some(gilrs) = self.gilrs.as_ref() else {
                return;
            };

            let connected: Vec<_> = gilrs
                .gamepads()
                .map(|(gilrs_id, gamepad)| (gilrs_id, gamepad.name().to_string()))
                .collect();
            for (gilrs_id, name) in connected {
                let id = self.ensure_id(gilrs_id);
                input.handle_gamepad_connected(id, name);
            }

            loop {
                let event = self.gilrs.as_mut().and_then(|gilrs| gilrs.next_event());
                let Some(event) = event else {
                    break;
                };
                self.handle_event(event, input);
            }
        }
    }

    fn map_button(button: GilrsButton) -> GamepadButton {
        match button {
            GilrsButton::South => GamepadButton::South,
            GilrsButton::East => GamepadButton::East,
            GilrsButton::West => GamepadButton::West,
            GilrsButton::North => GamepadButton::North,
            GilrsButton::C => GamepadButton::C,
            GilrsButton::Z => GamepadButton::Z,
            GilrsButton::LeftTrigger => GamepadButton::LeftShoulder,
            GilrsButton::LeftTrigger2 => GamepadButton::LeftTrigger,
            GilrsButton::RightTrigger => GamepadButton::RightShoulder,
            GilrsButton::RightTrigger2 => GamepadButton::RightTrigger,
            GilrsButton::Select => GamepadButton::Select,
            GilrsButton::Start => GamepadButton::Start,
            GilrsButton::Mode => GamepadButton::Mode,
            GilrsButton::LeftThumb => GamepadButton::LeftThumb,
            GilrsButton::RightThumb => GamepadButton::RightThumb,
            GilrsButton::DPadUp => GamepadButton::DPadUp,
            GilrsButton::DPadDown => GamepadButton::DPadDown,
            GilrsButton::DPadLeft => GamepadButton::DPadLeft,
            GilrsButton::DPadRight => GamepadButton::DPadRight,
            _ => GamepadButton::Other(0),
        }
    }

    fn map_axis(axis: GilrsAxis) -> GamepadAxis {
        match axis {
            GilrsAxis::LeftStickX => GamepadAxis::LeftX,
            GilrsAxis::LeftStickY => GamepadAxis::LeftY,
            GilrsAxis::LeftZ => GamepadAxis::LeftTrigger,
            GilrsAxis::RightStickX => GamepadAxis::RightX,
            GilrsAxis::RightStickY => GamepadAxis::RightY,
            GilrsAxis::RightZ => GamepadAxis::RightTrigger,
            GilrsAxis::DPadX => GamepadAxis::DPadX,
            GilrsAxis::DPadY => GamepadAxis::DPadY,
            _ => GamepadAxis::Other(0),
        }
    }
}
