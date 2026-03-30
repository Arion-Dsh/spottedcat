# Spottedcat Engine: AI Game Generation Guide

This guide is designed to help AI models (like LLMs) generate high-quality Rust code using the `spottedcat` engine. 

## 1. Core Architecture: The `Spot` Trait

Every application or "scene" in `spottedcat` must implement the `Spot` trait. This is the heart of the game loop.

```rust
use spottedcat::{Context, Spot, WindowConfig};
use std::time::Duration;

struct MyGame {
    // Your game state here
}

impl Spot for MyGame {
    /// Initialized once when the game starts.
    fn initialize(context: &mut Context) -> Self {
        // Load assets, initialize state
        Self { }
    }

    /// Called every frame for logic updates.
    /// dt: Delta time (time elapsed since last frame).
    fn update(&mut self, context: &mut Context, dt: Duration) {
        // Handle input, move entities, etc.
    }

    /// Called every frame for rendering.
    fn draw(&mut self, context: &mut Context) {
        // Issue draw commands here
    }

    /// Called when the app enters the foreground (e.g., from background).
    fn resumed(&mut self, context: &mut Context) { }

    /// Called when the app enters the background (e.g., screen locked, app switched).
    fn suspended(&mut self, context: &mut Context) { }

    /// (Optional) Called when the scene is being removed.
    fn remove(&self) { }
}

fn main() {
    spottedcat::run::<MyGame>(WindowConfig {
        title: "My Spottedcat Game".to_string(),
        ..Default::default()
    });
}
```

## 2. Drawing 2D (Images & Text)

### Images
Images are loaded from RGBA8 buffers or files. Use `DrawOption` for transformations.
```rust
use spottedcat::{Image, DrawOption, Pt};

let image = Image::new_from_rgba8(width, height, &rgba_data).unwrap();
image.draw(context, DrawOption::default()
    .with_position([Pt(100.0), Pt(100.0)])
    .with_scale([2.0, 2.0])
    .with_rotation(45.0f32.to_radians())
    .with_layer(1) // Higher layers are drawn on top
);
```

### Text
```rust
use spottedcat::{Text, DrawOption};

let text = Text::new("Hello World", 24.0); // string, font size
text.draw(context, DrawOption::default().with_position([Pt(0.0), Pt(0.0)]));
```

## 3. Drawing 3D (Models & Instancing)

### Basic Models
```rust
use spottedcat::{Model, DrawOption3D};

let cube = Model::cube(1.0).unwrap();
cube.draw(context, DrawOption3D::default()
    .with_position([0.0, 0.0, -5.0])
    .with_rotation([0.0, 1.0, 0.0])); // rotation as [x, y, z] axis
```

### Instanced Rendering (Performance)
Use this for drawing many copies of the same model efficiently.
```rust
let transforms: Vec<[[f32; 4]; 4]> = vec![...]; // Array of 4x4 matrices
model.draw_instanced(context, DrawOption3D::default(), &transforms);
```

## 4. Input Management

Access input via the `context`.
```rust
// Keyboard
if spottedcat::key_down(context, spottedcat::Key::Space) { ... }

// Mouse
if let Some((x, y)) = spottedcat::cursor_position(context) { ... }
if spottedcat::mouse_button_pressed(context, spottedcat::MouseButton::Left) { ... }

// Touch (Mobile)
for touch in spottedcat::touches(context) {
    match touch.phase {
        spottedcat::TouchPhase::Started => { ... }
        _ => {}
    }
}

// Sensors (Mobile) - Requires "sensors" feature
if let Some(gyro) = spottedcat::gyroscope(context) { ... } // [x, y, z]
```

## 5. UI Elements & Positioning

Use `Pt` for resolution-independent units.
- `context.vw(100.0)` -> 100% of window width.
- `context.vh(100.0)` -> 100% of window height.

```rust
let center_x = context.vw(50.0);
let center_y = context.vh(50.0);
```

## 6. Resources & Asset Loading

### Loading Files (Cross-platform)
```rust
let data = spottedcat::load_asset("assets/texture.png").expect("Failed to load");
```

### Resource Storage
Store shared objects in the `Context`.
```rust
context.insert_resource(Rc::new(my_resource));
if let Some(res) = context.get_resource::<MyResourceType>() { ... }
```

## 7. Scene Switching
```rust
spottedcat::switch_scene::<OtherScene>(); // Transitions to a new "Spot" implementation
```

## 8. Lifecycle & Platform Events

### Lifecycle
On mobile (iOS/Android), handle app background/foreground transitions:
- `resumed`: App is active. Restore timers, audio, or network state.
- `suspended`: App is in background. Pause expensive logic, save state, or mute audio.

### Native Platform Events
Listen for events from native iOS/Android code (e.g., JNI callbacks):
```rust
for event in spottedcat::poll_platform_events(context) {
    match event {
        spottedcat::PlatformEvent::Event(name, data) => {
            println!("Received event: {} with data: {}", name, data);
        }
    }
}
```

## 9. Development Tips
- **Coordinate System**: 2D origin (0,0) is TOP-LEFT. 3D uses standard RHS (X: right, Y: up, Z: back).
- **Transparency**: Set `WindowConfig.transparent = true` for background transparency (Android/Desktop).
- **Debugging**: Run with `SPOT_PROFILE_RENDER=1` to see performance statistics.
- **Delta Time**: Always multiply movement by `dt.as_secs_f32()` in `update`.
