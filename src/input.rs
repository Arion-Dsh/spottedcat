use std::collections::HashSet;

use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, Touch};
use winit::keyboard::PhysicalKey;

use crate::Key;
use crate::MouseButton as SpotMouseButton;
use crate::Pt;
use crate::touch::{TouchInfo, TouchPhase};

#[derive(Debug, Clone)]
pub struct InputManager {
    keys_down: [u64; Key::WORDS],
    keys_pressed: [u64; Key::WORDS],
    keys_released: [u64; Key::WORDS],

    mouse_down: u8,
    mouse_pressed: u8,
    mouse_released: u8,
    mouse_other_down: HashSet<u16>,
    mouse_other_pressed: HashSet<u16>,
    mouse_other_released: HashSet<u16>,

    cursor_position: Option<(Pt, Pt)>,
    scroll_delta: (f32, f32),
    focused: bool,

    text_input_enabled: bool,

    text_input: String,
    ime_preedit: Option<String>,

    touches: Vec<TouchInfo>,
}

impl Default for InputManager {
    fn default() -> Self {
        Self {
            keys_down: [0u64; Key::WORDS],
            keys_pressed: [0u64; Key::WORDS],
            keys_released: [0u64; Key::WORDS],

            mouse_down: 0,
            mouse_pressed: 0,
            mouse_released: 0,
            mouse_other_down: HashSet::new(),
            mouse_other_pressed: HashSet::new(),
            mouse_other_released: HashSet::new(),

            cursor_position: None,
            scroll_delta: (0.0, 0.0),
            focused: false,

            text_input_enabled: false,

            text_input: String::new(),
            ime_preedit: None,

            touches: Vec::new(),
        }
    }
}

fn key_word_bit(key: Key) -> (usize, u64) {
    let idx = key.as_index();
    (idx / 64, 1u64 << (idx % 64))
}

impl InputManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text_input_enabled(&self) -> bool {
        self.text_input_enabled
    }

    pub fn set_text_input_enabled(&mut self, enabled: bool) {
        self.text_input_enabled = enabled;
        if !enabled {
            self.text_input.clear();
            self.ime_preedit = None;
        }
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn cursor_position(&self) -> Option<(Pt, Pt)> {
        self.cursor_position
    }

    pub fn scroll_delta(&self) -> (f32, f32) {
        self.scroll_delta
    }

    pub fn text_input(&self) -> &str {
        &self.text_input
    }

    pub fn ime_preedit(&self) -> Option<&str> {
        self.ime_preedit.as_deref()
    }

    pub fn touches(&self) -> &[TouchInfo] {
        &self.touches
    }

    pub fn key_down(&self, key: Key) -> bool {
        let (w, m) = key_word_bit(key);
        (self.keys_down[w] & m) != 0
    }

    pub fn key_pressed(&self, key: Key) -> bool {
        let (w, m) = key_word_bit(key);
        (self.keys_pressed[w] & m) != 0
    }

    pub fn key_released(&self, key: Key) -> bool {
        let (w, m) = key_word_bit(key);
        (self.keys_released[w] & m) != 0
    }

    pub fn mouse_down(&self, button: SpotMouseButton) -> bool {
        match button.bit_index() {
            Some(i) => (self.mouse_down & (1u8 << i)) != 0,
            None => {
                let SpotMouseButton::Other(v) = button else {
                    return false;
                };
                self.mouse_other_down.contains(&v)
            }
        }
    }

    pub fn mouse_pressed(&self, button: SpotMouseButton) -> bool {
        match button.bit_index() {
            Some(i) => (self.mouse_pressed & (1u8 << i)) != 0,
            None => {
                let SpotMouseButton::Other(v) = button else {
                    return false;
                };
                self.mouse_other_pressed.contains(&v)
            }
        }
    }

    pub fn mouse_released(&self, button: SpotMouseButton) -> bool {
        match button.bit_index() {
            Some(i) => (self.mouse_released & (1u8 << i)) != 0,
            None => {
                let SpotMouseButton::Other(v) = button else {
                    return false;
                };
                self.mouse_other_released.contains(&v)
            }
        }
    }

    pub fn end_frame(&mut self) {
        self.keys_pressed = [0u64; Key::WORDS];
        self.keys_released = [0u64; Key::WORDS];
        self.mouse_pressed = 0;
        self.mouse_released = 0;
        self.mouse_other_pressed.clear();
        self.mouse_other_released.clear();
        self.scroll_delta = (0.0, 0.0);
        self.text_input.clear();
    }

    pub(crate) fn handle_focus(&mut self, focused: bool) {
        self.focused = focused;
        if !focused {
            self.keys_down = [0u64; Key::WORDS];
            self.keys_pressed = [0u64; Key::WORDS];
            self.keys_released = [0u64; Key::WORDS];
            self.mouse_down = 0;
            self.mouse_pressed = 0;
            self.mouse_released = 0;
            self.mouse_other_down.clear();
            self.mouse_other_pressed.clear();
            self.mouse_other_released.clear();

            self.text_input.clear();
            self.ime_preedit = None;
        }
    }

    pub(crate) fn handle_received_character(&mut self, ch: char) {
        if !self.text_input_enabled {
            return;
        }
        // Ignore control characters; keep printable/unicode characters.
        if ch.is_control() {
            return;
        }
        self.text_input.push(ch);
    }

    pub(crate) fn handle_ime(&mut self, ime: Ime) {
        match ime {
            Ime::Preedit(value, _cursor) => {
                if !self.text_input_enabled {
                    self.ime_preedit = None;
                    return;
                }

                self.ime_preedit = if value.is_empty() { None } else { Some(value) };
            }
            Ime::Commit(value) => {
                if self.text_input_enabled && !value.is_empty() {
                    self.text_input.push_str(&value);
                }
                self.ime_preedit = None;
            }
            Ime::Enabled | Ime::Disabled => {
                self.ime_preedit = None;
            }
        }
    }

    pub(crate) fn handle_cursor_moved(&mut self, x: Pt, y: Pt) {
        self.cursor_position = Some((x, y));
    }

    pub(crate) fn handle_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        let button = SpotMouseButton::from_winit(button);
        match (state, button.bit_index(), button) {
            (ElementState::Pressed, Some(i), _) => {
                let mask = 1u8 << i;
                if (self.mouse_down & mask) == 0 {
                    self.mouse_down |= mask;
                    self.mouse_pressed |= mask;
                }
            }
            (ElementState::Released, Some(i), _) => {
                let mask = 1u8 << i;
                self.mouse_down &= !mask;
                self.mouse_released |= mask;
            }
            (ElementState::Pressed, None, SpotMouseButton::Other(v)) => {
                if self.mouse_other_down.insert(v) {
                    self.mouse_other_pressed.insert(v);
                }
            }
            (ElementState::Released, None, SpotMouseButton::Other(v)) => {
                self.mouse_other_down.remove(&v);
                self.mouse_other_released.insert(v);
            }
            _ => {}
        }
    }

    pub(crate) fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        match delta {
            MouseScrollDelta::LineDelta(x, y) => {
                self.scroll_delta.0 += x;
                self.scroll_delta.1 += y;
            }
            MouseScrollDelta::PixelDelta(p) => {
                self.scroll_delta.0 += p.x as f32;
                self.scroll_delta.1 += p.y as f32;
            }
        }
    }

    pub(crate) fn handle_keyboard_input(&mut self, state: ElementState, physical_key: PhysicalKey) {
        let PhysicalKey::Code(code) = physical_key else {
            return;
        };

        let Some(key) = Key::from_winit_key_code(code) else {
            return;
        };

        let (w, mask) = key_word_bit(key);

        match state {
            ElementState::Pressed => {
                if (self.keys_down[w] & mask) == 0 {
                    self.keys_down[w] |= mask;
                    self.keys_pressed[w] |= mask;
                }
            }
            ElementState::Released => {
                self.keys_down[w] &= !mask;
                self.keys_released[w] |= mask;
            }
        }
    }

    pub(crate) fn handle_touch(&mut self, touch: Touch, scale_factor: f64) {
        let x = Pt::from_physical_px(touch.location.x, scale_factor);
        let y = Pt::from_physical_px(touch.location.y, scale_factor);
        let pos = (x, y);
        let phase = TouchPhase::from_winit(touch.phase);

        match phase {
            TouchPhase::Started => {
                // Ensure we don't have duplicates
                self.touches.retain(|t| t.id != touch.id);
                self.touches.push(TouchInfo {
                    id: touch.id,
                    position: pos,
                    phase,
                });
            }
            TouchPhase::Moved => {
                if let Some(t) = self.touches.iter_mut().find(|t| t.id == touch.id) {
                    t.position = pos;
                    t.phase = phase;
                } else {
                    self.touches.push(TouchInfo {
                        id: touch.id,
                        position: pos,
                        phase,
                    });
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                self.touches.retain(|t| t.id != touch.id);
            }
        }
    }
}
