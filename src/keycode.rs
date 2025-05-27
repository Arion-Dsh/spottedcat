use winit::keyboard::KeyCode;

/// 键盘按键枚举，对应 winit 的 KeyCode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keycode {
    /// 数字键 0-9
    Num0, Num1, Num2, Num3, Num4,
    Num5, Num6, Num7, Num8, Num9,

    /// 字母键 A-Z
    A, B, C, D, E, F, G, H, I, J,
    K, L, M, N, O, P, Q, R, S, T,
    U, V, W, X, Y, Z,

    /// 功能键 F1-F24
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10,
    F11, F12, F13, F14, F15, F16, F17, F18, F19, F20,
    F21, F22, F23, F24,

    /// 方向键
    Up, Down, Left, Right,

    /// 特殊键
    Escape, Tab, Backspace, Enter, Insert, Delete,
    Home, End, PageUp, PageDown,

    /// 控制键
    CapsLock, ScrollLock, NumLock, PrintScreen, Pause,

    /// 修饰键
    LeftShift, RightShift, LeftControl, RightControl,
    LeftAlt, RightAlt, LeftSuper, RightSuper,

    /// 数字小键盘
    Kp0, Kp1, Kp2, Kp3, Kp4, Kp5, Kp6, Kp7, Kp8, Kp9,
    KpDecimal, KpDivide, KpMultiply, KpSubtract, KpAdd,
    KpEnter, KpEqual,

    /// 符号键
    Space, Comma, Period, Slash, Semicolon, Quote,
    LeftBracket, RightBracket, Backslash, Grave,
    Minus, Equal,

    /// 其他键
    Menu, Unknown,
}

impl From<KeyCode> for Keycode {
    fn from(code: KeyCode) -> Self {
        match code {
            KeyCode::Digit0 => Self::Num0,
            KeyCode::Digit1 => Self::Num1,
            KeyCode::Digit2 => Self::Num2,
            KeyCode::Digit3 => Self::Num3,
            KeyCode::Digit4 => Self::Num4,
            KeyCode::Digit5 => Self::Num5,
            KeyCode::Digit6 => Self::Num6,
            KeyCode::Digit7 => Self::Num7,
            KeyCode::Digit8 => Self::Num8,
            KeyCode::Digit9 => Self::Num9,

            KeyCode::KeyA => Self::A,
            KeyCode::KeyB => Self::B,
            KeyCode::KeyC => Self::C,
            KeyCode::KeyD => Self::D,
            KeyCode::KeyE => Self::E,
            KeyCode::KeyF => Self::F,
            KeyCode::KeyG => Self::G,
            KeyCode::KeyH => Self::H,
            KeyCode::KeyI => Self::I,
            KeyCode::KeyJ => Self::J,
            KeyCode::KeyK => Self::K,
            KeyCode::KeyL => Self::L,
            KeyCode::KeyM => Self::M,
            KeyCode::KeyN => Self::N,
            KeyCode::KeyO => Self::O,
            KeyCode::KeyP => Self::P,
            KeyCode::KeyQ => Self::Q,
            KeyCode::KeyR => Self::R,
            KeyCode::KeyS => Self::S,
            KeyCode::KeyT => Self::T,
            KeyCode::KeyU => Self::U,
            KeyCode::KeyV => Self::V,
            KeyCode::KeyW => Self::W,
            KeyCode::KeyX => Self::X,
            KeyCode::KeyY => Self::Y,
            KeyCode::KeyZ => Self::Z,

            KeyCode::F1 => Self::F1,
            KeyCode::F2 => Self::F2,
            KeyCode::F3 => Self::F3,
            KeyCode::F4 => Self::F4,
            KeyCode::F5 => Self::F5,
            KeyCode::F6 => Self::F6,
            KeyCode::F7 => Self::F7,
            KeyCode::F8 => Self::F8,
            KeyCode::F9 => Self::F9,
            KeyCode::F10 => Self::F10,
            KeyCode::F11 => Self::F11,
            KeyCode::F12 => Self::F12,
            KeyCode::F13 => Self::F13,
            KeyCode::F14 => Self::F14,
            KeyCode::F15 => Self::F15,
            KeyCode::F16 => Self::F16,
            KeyCode::F17 => Self::F17,
            KeyCode::F18 => Self::F18,
            KeyCode::F19 => Self::F19,
            KeyCode::F20 => Self::F20,
            KeyCode::F21 => Self::F21,
            KeyCode::F22 => Self::F22,
            KeyCode::F23 => Self::F23,
            KeyCode::F24 => Self::F24,

            KeyCode::ArrowUp => Self::Up,
            KeyCode::ArrowDown => Self::Down,
            KeyCode::ArrowLeft => Self::Left,
            KeyCode::ArrowRight => Self::Right,

            KeyCode::Escape => Self::Escape,
            KeyCode::Tab => Self::Tab,
            KeyCode::Backspace => Self::Backspace,
            KeyCode::Enter => Self::Enter,
            KeyCode::Insert => Self::Insert,
            KeyCode::Delete => Self::Delete,
            KeyCode::Home => Self::Home,
            KeyCode::End => Self::End,
            KeyCode::PageUp => Self::PageUp,
            KeyCode::PageDown => Self::PageDown,

            KeyCode::CapsLock => Self::CapsLock,
            KeyCode::ScrollLock => Self::ScrollLock,
            KeyCode::NumLock => Self::NumLock,
            KeyCode::PrintScreen => Self::PrintScreen,
            KeyCode::Pause => Self::Pause,

            KeyCode::ShiftLeft => Self::LeftShift,
            KeyCode::ShiftRight => Self::RightShift,
            KeyCode::ControlLeft => Self::LeftControl,
            KeyCode::ControlRight => Self::RightControl,
            KeyCode::AltLeft => Self::LeftAlt,
            KeyCode::AltRight => Self::RightAlt,
            KeyCode::SuperLeft => Self::LeftSuper,
            KeyCode::SuperRight => Self::RightSuper,

            KeyCode::Numpad0 => Self::Kp0,
            KeyCode::Numpad1 => Self::Kp1,
            KeyCode::Numpad2 => Self::Kp2,
            KeyCode::Numpad3 => Self::Kp3,
            KeyCode::Numpad4 => Self::Kp4,
            KeyCode::Numpad5 => Self::Kp5,
            KeyCode::Numpad6 => Self::Kp6,
            KeyCode::Numpad7 => Self::Kp7,
            KeyCode::Numpad8 => Self::Kp8,
            KeyCode::Numpad9 => Self::Kp9,
            KeyCode::NumpadDecimal => Self::KpDecimal,
            KeyCode::NumpadDivide => Self::KpDivide,
            KeyCode::NumpadMultiply => Self::KpMultiply,
            KeyCode::NumpadSubtract => Self::KpSubtract,
            KeyCode::NumpadAdd => Self::KpAdd,
            KeyCode::NumpadEnter => Self::KpEnter,
            KeyCode::NumpadEqual => Self::KpEqual,

            KeyCode::Space => Self::Space,
            KeyCode::Comma => Self::Comma,
            KeyCode::Period => Self::Period,
            KeyCode::Slash => Self::Slash,
            KeyCode::Semicolon => Self::Semicolon,
            KeyCode::Quote => Self::Quote,
            KeyCode::BracketLeft => Self::LeftBracket,
            KeyCode::BracketRight => Self::RightBracket,
            KeyCode::Backslash => Self::Backslash,
            KeyCode::Backquote => Self::Grave,
            KeyCode::Minus => Self::Minus,
            KeyCode::Equal => Self::Equal,

            KeyCode::ContextMenu => Self::Menu,
            _ => Self::Unknown,
        }
    }
}


/// 键盘修饰键状态
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub super_key: bool,
}

impl Modifiers {
    /// 创建新的修饰键状态
    pub fn new() -> Self {
        Self::default()
    }

    /// 检查是否按下 Shift 键
    pub fn is_shift(&self) -> bool {
        self.shift
    }

    /// 检查是否按下 Ctrl 键
    pub fn is_ctrl(&self) -> bool {
        self.ctrl
    }

    /// 检查是否按下 Alt 键
    pub fn is_alt(&self) -> bool {
        self.alt
    }

    /// 检查是否按下 Super 键
    pub fn is_super(&self) -> bool {
        self.super_key
    }

    /// 检查是否按下任意修饰键
    pub fn any(&self) -> bool {
        self.shift || self.ctrl || self.alt || self.super_key
    }
}