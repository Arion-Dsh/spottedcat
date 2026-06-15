/// Represents a button on a mouse or similar pointing device.
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

    pub(crate) fn from_sdl_button(button: u8) -> Self {
        match button {
            1 => MouseButton::Left,
            2 => MouseButton::Middle,
            3 => MouseButton::Right,
            4 => MouseButton::Back,
            5 => MouseButton::Forward,
            v => MouseButton::Other(v as u16),
        }
    }
}
