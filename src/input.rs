use std::collections::{HashMap, HashSet};

#[cfg(not(target_os = "android"))]
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, Touch};
#[cfg(not(target_os = "android"))]
use winit::keyboard::PhysicalKey;

use crate::Key;
use crate::MouseButton as SpotMouseButton;
use crate::Pt;
use crate::gamepad::{GamepadAxis, GamepadButton, GamepadId, GamepadInfo};
use crate::touch::{TouchInfo, TouchPhase};

#[derive(Debug, Clone)]
struct GamepadInputState {
    info: GamepadInfo,
    buttons_down: HashSet<GamepadButton>,
    buttons_pressed: HashSet<GamepadButton>,
    buttons_released: HashSet<GamepadButton>,
    axes: HashMap<GamepadAxis, f32>,
}

impl GamepadInputState {
    #[allow(dead_code)]
    fn new(id: GamepadId, name: String) -> Self {
        Self {
            info: GamepadInfo {
                id,
                name,
                connected: true,
            },
            buttons_down: HashSet::new(),
            buttons_pressed: HashSet::new(),
            buttons_released: HashSet::new(),
            axes: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
/// Manages the state of all input devices (keyboard, mouse, touch, sensors).
///
/// `InputManager` tracks which keys and buttons are currently held down, which ones
/// were just pressed or released in the current frame, and aggregates touch/sensor data.
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
    gamepads: Vec<GamepadInputState>,
    #[cfg(feature = "sensors")]
    gyroscope: Option<[f32; 3]>,
    #[cfg(feature = "sensors")]
    accelerometer: Option<[f32; 3]>,
    #[cfg(feature = "sensors")]
    magnetometer: Option<[f32; 3]>,
    #[cfg(feature = "sensors")]
    rotation: Option<[f32; 4]>,
    #[cfg(feature = "sensors")]
    step_count: Option<f32>,
    #[cfg(feature = "sensors")]
    yesterday_step_count: Option<f32>,
    #[cfg(feature = "sensors")]
    step_detected: bool,
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
            gamepads: Vec::new(),
            #[cfg(feature = "sensors")]
            gyroscope: None,
            #[cfg(feature = "sensors")]
            accelerometer: None,
            #[cfg(feature = "sensors")]
            magnetometer: None,
            #[cfg(feature = "sensors")]
            rotation: None,
            #[cfg(feature = "sensors")]
            step_count: None,
            #[cfg(feature = "sensors")]
            yesterday_step_count: None,
            #[cfg(feature = "sensors")]
            step_detected: false,
        }
    }
}

fn key_word_bit(key: Key) -> (usize, u64) {
    let idx = key.as_index();
    (idx / 64, 1u64 << (idx % 64))
}

impl InputManager {
    /// Creates a new InputManager with default state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if text input (IME) is currently enabled.
    pub fn text_input_enabled(&self) -> bool {
        self.text_input_enabled
    }

    /// Enables or disables text input (IME). When disabled, text events are ignored.
    pub fn set_text_input_enabled(&mut self, enabled: bool) {
        self.text_input_enabled = enabled;
        if !enabled {
            self.text_input.clear();
            self.ime_preedit = None;
        }
    }

    /// Returns true if the window currently has input focus.
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Returns the current hardware cursor position in logical coordinates.
    pub fn cursor_position(&self) -> Option<(Pt, Pt)> {
        self.cursor_position
    }

    /// Returns the scroll wheel delta since the last frame.
    pub fn scroll_delta(&self) -> (f32, f32) {
        self.scroll_delta
    }

    /// Returns the accumulated text input string for the current frame.
    pub fn text_input(&self) -> &str {
        &self.text_input
    }

    /// Returns the current IME pre-edit string (uncommitted text).
    pub fn ime_preedit(&self) -> Option<&str> {
        self.ime_preedit.as_deref()
    }

    /// Returns a slice of active touch points.
    pub fn touches(&self) -> &[TouchInfo] {
        &self.touches
    }

    /// Returns basic information for every gamepad known to the input system.
    pub fn gamepads(&self) -> Vec<GamepadInfo> {
        self.gamepads
            .iter()
            .map(|state| state.info.clone())
            .collect()
    }

    /// Returns true if the gamepad is currently connected.
    pub fn gamepad_connected(&self, id: GamepadId) -> bool {
        self.gamepad_state(id)
            .map(|state| state.info.connected)
            .unwrap_or(false)
    }

    /// Returns true if the specified gamepad button is currently held.
    pub fn gamepad_button_down(&self, id: GamepadId, button: GamepadButton) -> bool {
        self.gamepad_state(id)
            .map(|state| state.buttons_down.contains(&button))
            .unwrap_or(false)
    }

    /// Returns true if the specified gamepad button was pressed this frame.
    pub fn gamepad_button_pressed(&self, id: GamepadId, button: GamepadButton) -> bool {
        self.gamepad_state(id)
            .map(|state| state.buttons_pressed.contains(&button))
            .unwrap_or(false)
    }

    /// Returns true if the specified gamepad button was released this frame.
    pub fn gamepad_button_released(&self, id: GamepadId, button: GamepadButton) -> bool {
        self.gamepad_state(id)
            .map(|state| state.buttons_released.contains(&button))
            .unwrap_or(false)
    }

    /// Returns the current value of a gamepad axis, or 0.0 if unavailable.
    pub fn gamepad_axis(&self, id: GamepadId, axis: GamepadAxis) -> f32 {
        self.gamepad_state(id)
            .and_then(|state| state.axes.get(&axis).copied())
            .unwrap_or(0.0)
    }

    #[cfg(feature = "sensors")]
    pub fn gyroscope(&self) -> Option<[f32; 3]> {
        self.gyroscope
    }

    #[cfg(feature = "sensors")]
    pub fn accelerometer(&self) -> Option<[f32; 3]> {
        self.accelerometer
    }

    #[cfg(feature = "sensors")]
    pub fn magnetometer(&self) -> Option<[f32; 3]> {
        self.magnetometer
    }

    #[cfg(feature = "sensors")]
    pub fn rotation(&self) -> Option<[f32; 4]> {
        self.rotation
    }

    #[cfg(feature = "sensors")]
    pub fn step_count(&self) -> Option<f32> {
        self.step_count
    }

    #[cfg(feature = "sensors")]
    pub fn today_step_count(&self) -> Option<f32> {
        self.step_count
    }

    #[cfg(feature = "sensors")]
    pub fn yesterday_step_count(&self) -> Option<f32> {
        self.yesterday_step_count
    }

    #[cfg(feature = "sensors")]
    pub fn step_detected(&self) -> bool {
        self.step_detected
    }

    /// Returns true if the specified key is currently held down.
    pub fn key_down(&self, key: Key) -> bool {
        let (w, m) = key_word_bit(key);
        (self.keys_down[w] & m) != 0
    }

    /// Returns true if the specified key was just pressed this frame.
    pub fn key_pressed(&self, key: Key) -> bool {
        let (w, m) = key_word_bit(key);
        (self.keys_pressed[w] & m) != 0
    }

    /// Returns true if the specified key was just released this frame.
    pub fn key_released(&self, key: Key) -> bool {
        let (w, m) = key_word_bit(key);
        (self.keys_released[w] & m) != 0
    }

    /// Returns true if the specified mouse button is currently held down.
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

    /// Returns true if the specified mouse button was just pressed this frame.
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

    /// Returns true if the specified mouse button was just released this frame.
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
        for gamepad in &mut self.gamepads {
            gamepad.buttons_pressed.clear();
            gamepad.buttons_released.clear();
        }
        #[cfg(feature = "sensors")]
        {
            self.step_detected = false;
        }
    }

    pub(crate) fn clear_transient_state(&mut self) {
        self.keys_down = [0u64; Key::WORDS];
        self.keys_pressed = [0u64; Key::WORDS];
        self.keys_released = [0u64; Key::WORDS];
        self.mouse_down = 0;
        self.mouse_pressed = 0;
        self.mouse_released = 0;
        self.mouse_other_down.clear();
        self.mouse_other_pressed.clear();
        self.mouse_other_released.clear();
        self.cursor_position = None;
        self.scroll_delta = (0.0, 0.0);
        self.text_input.clear();
        self.ime_preedit = None;
        self.touches.clear();
        for gamepad in &mut self.gamepads {
            gamepad.buttons_down.clear();
            gamepad.buttons_pressed.clear();
            gamepad.buttons_released.clear();
            gamepad.axes.clear();
        }
        #[cfg(feature = "sensors")]
        {
            self.step_detected = false;
        }
    }

    #[allow(dead_code)]
    pub(crate) fn handle_focus(&mut self, focused: bool) {
        self.focused = focused;
        if !focused {
            self.clear_transient_state();
        }
    }

    #[allow(dead_code)]
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

    #[cfg(not(target_os = "android"))]
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

    #[allow(dead_code)]
    pub(crate) fn handle_cursor_moved(&mut self, x: Pt, y: Pt) {
        self.cursor_position = Some((x, y));
    }

    #[cfg(not(target_os = "android"))]
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

    #[cfg(not(target_os = "android"))]
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

    #[cfg(not(target_os = "android"))]
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

    #[cfg(not(target_os = "android"))]
    pub(crate) fn handle_touch(&mut self, touch: Touch, scale_factor: f64) {
        let x = Pt::from_physical_px(touch.location.x, scale_factor);
        let y = Pt::from_physical_px(touch.location.y, scale_factor);
        let pos = (x, y);
        let phase = TouchPhase::from_winit(touch.phase);

        self.handle_touch_raw(touch.id, pos, phase);
    }

    pub(crate) fn handle_touch_raw(&mut self, id: u64, position: (Pt, Pt), phase: TouchPhase) {
        match phase {
            TouchPhase::Started => {
                // Ensure we don't have duplicates
                self.touches.retain(|t| t.id != id);
                self.touches.push(TouchInfo {
                    id,
                    position,
                    phase,
                });
            }
            TouchPhase::Moved => {
                if let Some(t) = self.touches.iter_mut().find(|t| t.id == id) {
                    t.position = position;
                    t.phase = phase;
                } else {
                    self.touches.push(TouchInfo {
                        id,
                        position,
                        phase,
                    });
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                self.touches.retain(|t| t.id != id);
            }
        }
    }

    fn gamepad_state(&self, id: GamepadId) -> Option<&GamepadInputState> {
        self.gamepads.iter().find(|state| state.info.id == id)
    }

    #[allow(dead_code)]
    fn gamepad_state_mut(&mut self, id: GamepadId) -> Option<&mut GamepadInputState> {
        self.gamepads.iter_mut().find(|state| state.info.id == id)
    }

    #[allow(dead_code)]
    fn ensure_gamepad_state(&mut self, id: GamepadId, name: String) -> &mut GamepadInputState {
        if let Some(idx) = self.gamepads.iter().position(|state| state.info.id == id) {
            let state = &mut self.gamepads[idx];
            state.info.connected = true;
            if state.info.name != name {
                state.info.name = name;
            }
            return state;
        }

        self.gamepads.push(GamepadInputState::new(id, name));
        self.gamepads.last_mut().unwrap()
    }

    #[allow(dead_code)]
    pub(crate) fn handle_gamepad_connected(&mut self, id: GamepadId, name: String) {
        self.ensure_gamepad_state(id, name);
    }

    #[allow(dead_code)]
    pub(crate) fn handle_gamepad_disconnected(&mut self, id: GamepadId) {
        if let Some(state) = self.gamepad_state_mut(id) {
            state.info.connected = false;
            state.buttons_down.clear();
            state.buttons_pressed.clear();
            state.buttons_released.clear();
            state.axes.clear();
        }
    }

    #[allow(dead_code)]
    pub(crate) fn handle_gamepad_button(
        &mut self,
        id: GamepadId,
        button: GamepadButton,
        pressed: bool,
    ) {
        let state = self.ensure_gamepad_state(id, "Gamepad".to_string());
        if pressed {
            if state.buttons_down.insert(button) {
                state.buttons_pressed.insert(button);
            }
            state.buttons_released.remove(&button);
        } else {
            if state.buttons_down.remove(&button) {
                state.buttons_released.insert(button);
            }
            state.buttons_pressed.remove(&button);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn handle_gamepad_axis(&mut self, id: GamepadId, axis: GamepadAxis, value: f32) {
        let state = self.ensure_gamepad_state(id, "Gamepad".to_string());
        let value = if value.abs() < 0.001 { 0.0 } else { value };
        state.axes.insert(axis, value.clamp(-1.0, 1.0));
    }

    #[cfg(feature = "sensors")]
    #[allow(dead_code)]
    pub(crate) fn handle_gyroscope(&mut self, x: f32, y: f32, z: f32) {
        self.gyroscope = Some([x, y, z]);
    }

    #[cfg(feature = "sensors")]
    #[allow(dead_code)]
    pub(crate) fn handle_accelerometer(&mut self, x: f32, y: f32, z: f32) {
        self.accelerometer = Some([x, y, z]);
    }

    #[cfg(feature = "sensors")]
    #[allow(dead_code)]
    pub(crate) fn handle_magnetometer(&mut self, x: f32, y: f32, z: f32) {
        self.magnetometer = Some([x, y, z]);
    }

    #[cfg(feature = "sensors")]
    #[allow(dead_code)]
    pub(crate) fn handle_rotation(&mut self, x: f32, y: f32, z: f32, w: f32) {
        self.rotation = Some([x, y, z, w]);
    }

    #[cfg(feature = "sensors")]
    #[allow(dead_code)]
    pub(crate) fn handle_step_counter(&mut self, count: f32) {
        self.step_count = Some(count);
    }

    #[cfg(feature = "sensors")]
    #[allow(dead_code)]
    pub(crate) fn handle_yesterday_step_counter(&mut self, count: f32) {
        self.yesterday_step_count = Some(count);
    }

    #[cfg(feature = "sensors")]
    #[allow(dead_code)]
    pub(crate) fn handle_step_detector(&mut self) {
        self.step_detected = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gamepad::{GamepadAxis, GamepadButton, GamepadId};

    #[test]
    fn losing_focus_clears_active_touches() {
        let mut input = InputManager::new();
        input.handle_touch_raw(7, (Pt::from(12.0), Pt::from(24.0)), TouchPhase::Started);

        assert_eq!(input.touches().len(), 1);

        input.handle_focus(false);

        assert!(input.touches().is_empty());
    }

    #[test]
    fn clear_transient_state_preserves_focus_flag() {
        let mut input = InputManager::new();
        input.handle_focus(true);
        input.handle_touch_raw(7, (Pt::from(12.0), Pt::from(24.0)), TouchPhase::Started);

        input.clear_transient_state();

        assert!(input.is_focused());
        assert!(input.touches().is_empty());
    }

    #[test]
    fn gamepad_button_edges_and_axes_are_tracked() {
        let mut input = InputManager::new();
        let id = GamepadId(0);

        input.handle_gamepad_connected(id, "pad".to_string());
        input.handle_gamepad_button(id, GamepadButton::South, true);
        input.handle_gamepad_axis(id, GamepadAxis::LeftX, 0.75);

        assert!(input.gamepad_connected(id));
        assert!(input.gamepad_button_down(id, GamepadButton::South));
        assert!(input.gamepad_button_pressed(id, GamepadButton::South));
        assert_eq!(input.gamepad_axis(id, GamepadAxis::LeftX), 0.75);

        input.end_frame();

        assert!(input.gamepad_button_down(id, GamepadButton::South));
        assert!(!input.gamepad_button_pressed(id, GamepadButton::South));

        input.handle_gamepad_button(id, GamepadButton::South, false);

        assert!(!input.gamepad_button_down(id, GamepadButton::South));
        assert!(input.gamepad_button_released(id, GamepadButton::South));

        input.handle_gamepad_disconnected(id);

        assert!(!input.gamepad_connected(id));
        assert_eq!(input.gamepad_axis(id, GamepadAxis::LeftX), 0.0);
    }
}
