#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

impl MouseButton {
    pub const STANDARD_COUNT: usize = 5;

    pub(crate) fn bit_index(self) -> Option<u8> {
        match self {
            MouseButton::Left => Some(0),
            MouseButton::Right => Some(1),
            MouseButton::Middle => Some(2),
            MouseButton::Back => Some(3),
            MouseButton::Forward => Some(4),
            MouseButton::Other(_) => None,
        }
    }

    pub(crate) fn from_winit(button: winit::event::MouseButton) -> Self {
        use winit::event::MouseButton as W;
        match button {
            W::Left => MouseButton::Left,
            W::Right => MouseButton::Right,
            W::Middle => MouseButton::Middle,
            W::Back => MouseButton::Back,
            W::Forward => MouseButton::Forward,
            W::Other(v) => MouseButton::Other(v),
        }
    }
}
