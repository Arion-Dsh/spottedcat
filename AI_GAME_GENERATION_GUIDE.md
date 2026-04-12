# Spottedcat Engine: AI Game Generation Guide

This guide is designed to help AI models (like LLMs) generate high-quality Rust code using the `spottedcat` engine. 

> [!WARNING]
> **ALPHA VERSION**: This library is in an alpha state. The API is subject to frequent breaking changes. Use with caution.

## 1. Core Architecture: The `Spot` Trait

Every application or "scene" in `spottedcat` must implement the `Spot` trait. This is the heart of the game loop.

> [!IMPORTANT]
> Always use the `&mut Context` provided in the lifecycle methods. It is synchronized with the engine's internal state.

```rust
use spottedcat::{Context, Spot, WindowConfig, Pt};
use std::time::Duration;

struct MyGame {
    // Your game state here
}

impl Spot for MyGame {
    /// Initialized once when the game starts.
    fn initialize(ctx: &mut Context) -> Self {
        // Load assets, initialize state
        // Use the passed-in ctx for registration
        Self { }
    }

    /// Called every frame for logic updates.
    fn update(&mut self, ctx: &mut Context, dt: Duration) {
        // Handle input, move entities, etc.
    }

    /// Called every frame for rendering.
    fn draw(&mut self, ctx: &mut Context, screen: spottedcat::Image) {
        // Issue draw commands here using the provided 'screen' as the target
    }

    /// Called when the app enters the foreground (e.g., from background).
    /// Critical for restoring GPU resources on mobile.
    fn resumed(&mut self, ctx: &mut Context) { }

    /// Called when the app enters the background.
    fn suspended(&mut self, ctx: &mut Context) { }

    /// Called when the scene is being removed.
    fn remove(&mut self, _ctx: &mut Context) { }
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
Images are created via `Image::new(...)`. Use `Pt` for logical units. When the source asset is an encoded PNG/JPEG/WebP and the `utils` feature is enabled, prefer `spottedcat::utils::image::from_image(...)` or `from_rgba_image(...)` so the engine keeps the asset's pixel dimensions and derives the default logical size from the current `scale_factor`.
```rust
use spottedcat::{Image, DrawOption, Pt};

// Registering requires the ctx to synchronize with the GPU
let image = Image::new(ctx, Pt(width), Pt(height), &rgba_data).unwrap();

// Draw using the target (e.g. screen)
target.draw(ctx, &image, DrawOption::default()
    .with_position([Pt(100.0), Pt(100.0)])
    .with_scale([2.0, 2.0])
    .with_rotation(45.0f32.to_radians())
    .with_layer(1) // Higher layers are drawn on top
);
```

```rust
#[cfg(feature = "utils")]
{
    let decoded = image::load_from_memory(encoded_bytes).unwrap();
    let image = spottedcat::utils::image::from_image(ctx, &decoded).unwrap();
}
```

### Text
Text requires a registered font ID.
```rust
use spottedcat::{Text, DrawOption, Pt};

// 1. Register a font (returns a u32 font_id)
let font_id = spottedcat::register_font(ctx, font_data_vec);

// 2. Create the Text object with content and font_id
let text = Text::new("Hello World", font_id).with_font_size(Pt(24.0));
target.draw(ctx, &text, DrawOption::default().with_position([Pt(50.0), Pt(50.0)]));
```

## 3. Drawing 3D (Models & Instancing)

### Basic Models
```rust
use spottedcat::{Model, DrawOption3D};

let cube = spottedcat::model::create_cube(ctx, 1.0).unwrap();
// 3D drawing example (if model-3d feature enabled)
target.draw(ctx, &cube, DrawOption3D::default()
    .with_position([0.0, 0.0, -5.0])
    .with_rotation([0.0, 1.0, 0.0])); // rotation as [x, y, z] axis
```

### Instanced Rendering (Performance)
Use this for drawing many copies of the same model efficiently.
```rust
let transforms: Vec<[[f32; 4]; 4]> = vec![...]; // Array of 4x4 matrices
spottedcat::model::draw_instanced(ctx, target, &model, DrawOption3D::default(), &transforms);
```

## 4. Input Management

Access input via helper functions or trait methods using the `ctx`.
```rust
// Keyboard
if spottedcat::key_down(ctx, spottedcat::Key::Space) { ... }

// Mouse
if let Some((x, y)) = spottedcat::mouse_pos(ctx) { ... }
if spottedcat::mouse_down(ctx, spottedcat::MouseButton::Left) { ... }

// Touch (Mobile)
for touch in spottedcat::touches(ctx) {
    match touch.phase {
        spottedcat::TouchPhase::Started => { ... }
        _ => {}
    }
}
```

## 5. Viewport & Scaling

Use `Pt` for resolution-independent units.
- `spottedcat::window_size(ctx)` -> Returns `(Pt(width), Pt(height))`.
- `spottedcat::vw(ctx, 100.0)` -> 100% of window width in `Pt`.
- `spottedcat::vh(ctx, 100.0)` -> 100% of window height in `Pt`.

## 6. Audio System

Access audio via helper functions using the `ctx`.
```rust
// 1. Register a sound (returns a u32 sound_id)
let sound_id = spottedcat::register_sound(ctx, sound_data_vec);

// 2. Play a sound
spottedcat::play_sound(ctx, sound_id, spottedcat::SoundOptions::default());

// 3. Simple sine wave
spottedcat::play_sine(ctx, 440.0, 0.5);
```

## 7. Resources & Persistence

### Resource Storage
Store shared objects in the `Context`. The `Context` is persistent across app pauses/resumes.
```rust
spottedcat::insert_resource(ctx, Rc::new(my_resource));
if let Some(res) = spottedcat::get_resource::<MyResourceType>(ctx) { ... }
```

### Asset Registration Persistence
Shaders, fonts, and images created or registered through the `Context` are persistent. When the GPU device is lost (Android lifecycle), the engine automatically restores these registration-based assets using high-level metadata stored in the `Context`.

## 8. Custom Shaders

Inject custom WGSL hooks into the rendering pipeline.
```rust
// 2D Shader
let shader_source_str = &spottedcat::image_shader_template();
spottedcat::register_image_shader_desc(ctx, spottedcat::ImageShaderDesc::from_wgsl(shader_source_str));

// 3D Shader
let shader_source_str = spottedcat::model_shader_template();
spottedcat::register_model_shader(ctx, shader_source_str);

// shader_source_str must be a full WGSL source that defines:
// - vs_main
// - vs_main_instanced
// - fs_main
```

## 9. Development Tips

- **The Context Rule**: **ALWAYS** use the passed-in `Context` (usually named `ctx`). 
- **Coordinate System**: 2D origin (0,0) is **TOP-LEFT**. 3D uses RHS (X: right, Y: up, Z: back).
- **Transparency**: Set `WindowConfig.transparent = true` for background transparency (Android/Desktop).
- **Delta Time**: Always multiply movement/animations by `dt.as_secs_f32()` in `update`.
- **Pt Construction**: Use `Pt::from(f32)` or the `Pt(f32)` tuple struct constructor. Positions in `DrawOption` are `[Pt; 2]`.
