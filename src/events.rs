use std::collections::{HashMap, HashSet};
use std::time::Instant;
use winit::event::{
    DeviceEvent, ElementState, KeyEvent, MouseButton, MouseScrollDelta, Touch, TouchPhase,
    WindowEvent, Event,
};
use winit::keyboard::{Key, ModifiersKeyState, PhysicalKey};
use crate::keycode::{Keycode, Modifiers};

// event manager instance
// safy! because it is only used in the main thread.
pub(crate)static mut EVENT_MANAGER: Option<EventManager> = None;


/// Keyboard input information, to avoid winit::event::KeyEvent's lifetime issues
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RawKeyboardInput {
    key_code: Option<Keycode>, 
    logical_key: Option<Key>,
    state: ElementState,
    is_synthetic: bool,
    is_repeat: bool,
}

/// Mouse event (now only contains state updates, not directly as event queue elements)
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct MouseEvent {
    button: MouseButton,
    state: ElementState,
    position: (f64, f64),
}

/// Touch event
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TouchEvent {
    id: u64,
    phase: TouchPhase,
    position: (f64, f64),
    force: Option<f64>, // winit's force is Option<Force>, Force has normalized() method
}

/// Touch information
#[derive(Debug, Clone)]
pub struct TouchInfo {
    position: (f64, f64),
    force: Option<f64>,
    #[allow(dead_code)]
    start_position: (f64, f64),
    #[allow(dead_code)]
    start_time: Instant,
    phase: TouchPhase, // Current touch state
}

/// Keyboard state
#[derive(Default)]
struct KeyboardState {
    pressed_keys: HashSet<Keycode>,     // Currently pressed physical keys
    just_pressed_keys: HashSet<Keycode>,// Keys just pressed this frame
    just_released_keys: HashSet<Keycode>,// Keys just released this frame
}

/// Mouse state
#[derive(Default)]
struct MouseState {
    position: (f64, f64),
    delta: (f64, f64),
    buttons: HashMap<MouseButton, ElementState>,
    wheel_delta_x: f32, // X-axis wheel delta
    wheel_delta_y: f32, // Y-axis wheel delta
    #[allow(dead_code)]
    cursor_entered: bool, // Is mouse inside the window? TODO: implement this
}

/// Touch state
#[derive(Default)]
struct TouchState {
    active_touches: HashMap<u64, TouchInfo>, // Currently active touch points
    started_touches: HashSet<u64>,           // IDs of touches newly started this frame
    ended_touches: HashSet<u64>,             // IDs of touches ended this frame
    moved_touches: HashSet<u64>,             // IDs of touches moved this frame
}

/// Window state
#[derive(Default)]
struct WindowState {
    size: (u32, u32),
    position: (i32, i32),
    is_focused: bool,
    is_resized: bool,
    #[allow(dead_code)]
    is_minimized: bool, //TODO: implement this
}

/// Event Manager (now primarily a state manager)
pub(crate)struct EventManager {
    keyboard_state: KeyboardState,
    mouse_state: MouseState,
    touch_state: TouchState,
    window_state: WindowState,
    modifiers: Modifiers,
    current_keyboard_event: Option<KeyEvent>,
    current_keycode: Option<Keycode>,
}

impl EventManager {
    /// Create a new event manager
    pub(crate) fn new() -> Self {
        Self {
            keyboard_state: KeyboardState::default(),
            mouse_state: MouseState::default(),
            touch_state: TouchState::default(),
            window_state: WindowState::default(),
            modifiers: Modifiers::default(),
            current_keyboard_event: None,
            current_keycode: None,
        }
    }

    // Process a Winit Event
    //TODO: Remove this method
    #[allow(dead_code)]
    pub(crate) fn process_winit_event(&mut self, event: Event<()>) {
        match event {
            Event::WindowEvent { event, .. } => self.process_window_event(event),
            Event::DeviceEvent { event, .. } => self.process_device_event(event),
            Event::NewEvents(_) => { /* State is cleared before this, no dispatch needed */ }
            Event::AboutToWait => { /* This is where the application loop will query state */ }
            Event::LoopExiting => { /* Application cleanup */ }
            Event::Suspended => { /* Handle application suspension */ }
            Event::Resumed => { /* Handle application resumption */ }
            _ => {
                // println!("Unhandled Winit Event: {:?}", event); // For debugging unhandled events
            }
        }
    }

    // Process a Winit WindowEvent
    pub(crate) fn process_window_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::Resized(size) => {
                self.handle_window_resize(size);
            }
            WindowEvent::Moved(position) => {
                self.handle_window_move(position);
            }
            WindowEvent::CloseRequested => {
                // In a real application, you'd set an "exit_requested" flag here
            }
            WindowEvent::Focused(focused) => {
                self.handle_window_focus(focused);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_event(event);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                if modifiers.lcontrol_state() == ModifiersKeyState::Pressed {
                    self.modifiers.ctrl = true;
                } else {
                    self.modifiers.ctrl = false;
                }
                if modifiers.lalt_state() == ModifiersKeyState::Pressed {
                    self.modifiers.alt = true;
                } else {
                    self.modifiers.alt = false;
                }
                if modifiers.lsuper_state() == ModifiersKeyState::Pressed {
                    self.modifiers.super_key = true;
                } else {
                    self.modifiers.super_key = false;
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.handle_mouse_move(position);
            }
            WindowEvent::MouseInput { button, state, .. } => {
                self.handle_mouse_button(button, state);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_mouse_wheel(delta);
            }
            WindowEvent::Touch(touch) => {
                let _phase = touch.phase; // Keep this to determine what touch state changed
                self.handle_touch_event(touch);
            }
            _ => {
                // println!("Unhandled WindowEvent: {:?}", event);
            }
        }
    }

    /// Process a Winit DeviceEvent (if needed, this can be further processed here)
    #[allow(dead_code)]
    fn process_device_event(&mut self, _event: DeviceEvent) {
        // println!("DeviceEvent: {:?}", event);
    }

    /// Handle keyboard event
    fn handle_keyboard_event(&mut self, event: KeyEvent) {
        self.current_keyboard_event = Some(event.clone());

        if let PhysicalKey::Code(key_code) = event.physical_key {
            let keycode = Keycode::from(key_code);
            self.current_keycode = Some(keycode);
            let keycode = Keycode::from(key_code);
            self.current_keycode = Some(keycode);

            match event.state {
                ElementState::Pressed => {
                    if self.keyboard_state.pressed_keys.insert(keycode) {
                        // Only add to just_pressed if it wasn't already pressed
                        self.keyboard_state.just_pressed_keys.insert(keycode);
                    }
                }
                ElementState::Released => {
                    if self.keyboard_state.pressed_keys.remove(&keycode) {
                        // Only add to just_released if it was previously pressed
                        self.keyboard_state.just_released_keys.insert(keycode);
                    }
                }
            }
        }
    }

    /// Handle mouse movement
    fn handle_mouse_move(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        self.mouse_state.delta = (
            position.x - self.mouse_state.position.0,
            position.y - self.mouse_state.position.1,
        );
        self.mouse_state.position = (position.x, position.y);
    }

    /// Handle mouse button events
    fn handle_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        self.mouse_state.buttons.insert(button, state);
    }

    /// Handle mouse wheel events
    fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        match delta {
            MouseScrollDelta::LineDelta(x, y) => {
                self.mouse_state.wheel_delta_x += x;
                self.mouse_state.wheel_delta_y += y;
            }
            MouseScrollDelta::PixelDelta(pos) => {
                self.mouse_state.wheel_delta_x += pos.x as f32;
                self.mouse_state.wheel_delta_y += pos.y as f32;
            }
        }
    }

    /// Handle touch events
    fn handle_touch_event(&mut self, touch: Touch) {
        let touch_id = touch.id;
        let position = (touch.location.x, touch.location.y);
        let force = touch.force.map(|f| f.normalized()); // Convert to normalized f64

        match touch.phase {
            TouchPhase::Started => {
                self.touch_state.active_touches.insert(
                    touch_id,
                    TouchInfo {
                        position,
                        force,
                        start_position: position,
                        start_time: Instant::now(),
                        phase: touch.phase,
                    },
                );
                self.touch_state.started_touches.insert(touch_id);
            }
            TouchPhase::Moved => {
                if let Some(touch_info) = self.touch_state.active_touches.get_mut(&touch_id) {
                    touch_info.position = position;
                    touch_info.force = force;
                    touch_info.phase = touch.phase;
                    self.touch_state.moved_touches.insert(touch_id);
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                self.touch_state.active_touches.remove(&touch_id);
                self.touch_state.ended_touches.insert(touch_id); // Record for this frame
                // Do not clear `moved_touches` here, it's cleared at the end of the frame
            }
        }
    }

    /// Handle window resize
    fn handle_window_resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.window_state.size = (size.width, size.height);
        self.window_state.is_resized = true;
    }

    /// Handle window move
    fn handle_window_move(&mut self, position: winit::dpi::PhysicalPosition<i32>) {
        self.window_state.position = (position.x, position.y);
    }

    /// Handle window focus
    fn handle_window_focus(&mut self, focused: bool) {
        self.window_state.is_focused = focused;
    }

    /// Clear frame state (call at the beginning of each frame)
    #[allow(dead_code)]
    pub(crate) fn clear_frame_state(&mut self) {
        self.keyboard_state.just_pressed_keys.clear();
        self.keyboard_state.just_released_keys.clear();
        self.mouse_state.delta = (0.0, 0.0);
        self.mouse_state.wheel_delta_x = 0.0;
        self.mouse_state.wheel_delta_y = 0.0;
        self.touch_state.started_touches.clear();
        self.touch_state.ended_touches.clear();
        self.touch_state.moved_touches.clear();
        self.window_state.is_resized = false;
    }

    
}

// Public methods to query state (unchanged from previous versions)

    /// Check if a key is currently pressed down.
    pub fn is_key_pressed(keycode: Keycode) -> bool {
        #[allow(static_mut_refs)]
        unsafe { EVENT_MANAGER.as_ref().unwrap() }.keyboard_state.pressed_keys.contains(&keycode)
    }

    /// Check if a key was just pressed this frame.
    pub fn is_key_just_pressed(keycode: Keycode) -> bool {
        #[allow(static_mut_refs)]
        unsafe { EVENT_MANAGER.as_ref().unwrap() }.keyboard_state.just_pressed_keys.contains(&keycode)
    }

    /// Check if a key was just released this frame.
    pub fn is_key_just_released(keycode: Keycode) -> bool {
        #[allow(static_mut_refs)]
        unsafe { EVENT_MANAGER.as_ref().unwrap() }.keyboard_state.just_released_keys.contains(&keycode)
    }

    /// Get the current mouse position.
    pub fn mouse_position() -> (f64, f64) {
        #[allow(static_mut_refs)]
        unsafe { EVENT_MANAGER.as_ref().unwrap() }.mouse_state.position
    }

    /// Get the mouse delta movement for the current frame.
    pub fn mouse_delta() -> (f64, f64) {
        #[allow(static_mut_refs)]
        unsafe { EVENT_MANAGER.as_ref().unwrap() }.mouse_state.delta
    }

    /// Check if a mouse button is currently pressed.
    #[allow(static_mut_refs)]
    pub fn is_mouse_button_pressed(button: MouseButton) -> bool {
        unsafe { EVENT_MANAGER.as_ref().unwrap() }.mouse_state.buttons.get(&button) == Some(&ElementState::Pressed)
    }

    /// Get the mouse wheel scroll delta for the current frame.
    pub fn mouse_wheel_delta() -> (f32, f32) {
        #[allow(static_mut_refs)]
        (unsafe { EVENT_MANAGER.as_ref().unwrap() }.mouse_state.wheel_delta_x, unsafe { EVENT_MANAGER.as_ref().unwrap() }.mouse_state.wheel_delta_y)
    }

    /// Get information about active touches.
    pub fn active_touches() -> HashMap<u64, TouchInfo> {
        #[allow(static_mut_refs)]
        unsafe { EVENT_MANAGER.as_ref().unwrap() }.touch_state.active_touches.clone()
    }

    /// Get IDs of touches that started this frame.
    pub fn just_started_touches() -> HashSet<u64> {
        #[allow(static_mut_refs)]
        unsafe { EVENT_MANAGER.as_ref().unwrap() }.touch_state.started_touches.clone()
    }

    /// Get IDs of touches that ended this frame.
    pub fn just_ended_touches() -> HashSet<u64> {
        #[allow(static_mut_refs)]
        unsafe { EVENT_MANAGER.as_ref().unwrap() }.touch_state.ended_touches.clone()
    }

    /// Get IDs of touches that moved this frame.
    pub fn just_moved_touches() -> HashSet<u64> {
        #[allow(static_mut_refs)]
        unsafe { EVENT_MANAGER.as_ref().unwrap() }.touch_state.moved_touches.clone()
    }

    /// Get the current window size.
    pub fn window_size() -> (u32, u32) {
        #[allow(static_mut_refs)]
        unsafe { EVENT_MANAGER.as_ref().unwrap() }.window_state.size
    }

    /// Check if the window was resized this frame.
    pub fn was_window_resized() -> bool {
        #[allow(static_mut_refs)]
        unsafe { EVENT_MANAGER.as_ref().unwrap() }.window_state.is_resized
    }

    /// Check if the window is currently focused.
    pub fn is_window_focused() -> bool {
        #[allow(static_mut_refs)]
        unsafe { EVENT_MANAGER.as_ref().unwrap() }.window_state.is_focused
    }

    /// Get the current modifier keys state.
    pub fn modifiers() -> Modifiers {
        #[allow(static_mut_refs)]
        unsafe { EVENT_MANAGER.as_ref().unwrap() }.modifiers
    }