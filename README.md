# spottedcat

Spottedcat is a lightweight cross-platform 2D/3D game engine built with Rust and wgpu.
It provides a simple API for rendering, input, audio, text, and scene management across desktop, web, iOS, and Android.
Designed for fast prototyping and creative interactive projects, it aims to stay small, practical, and easy to use.

> [!WARNING]
> **ALPHA VERSION**: This library is currently in an alpha state. The API is subject to frequent breaking changes and significant refactoring. Use with caution in production environments.

## Stability

- Stable-ish core direction: `Context`, `Spot`, `Image`, `Model`, `Text`, and `run` are the primary surfaces the crate is trying to converge around.
- Still volatile: scene payload internals, shader extension points, platform-specific behavior, audio internals, and lower-level rendering details may change between minor releases.
- Release expectation: until `1.0`, minor versions may include breaking API changes, behavior fixes, and platform-specific adjustments.
- Production guidance: pin an exact crate version if you ship with `spottedcat` today, and review changelogs before upgrading.

## Why spottedcat?

The library is named after the **Rusty-spotted cat** (*Prionailurus rubiginosus*), the world's smallest wild cat. Just like its namesake, this library aims to be tiny, agile, and remarkably efficient.


## Features

- **Simple API**: Minimal core types to learn: `Context`, `Spot`, `Image`, `Model`, `Text`, and `run`.
- **GPU-accelerated**: Built on wgpu for high-performance rendering.
- **2D & 3D Support**: Draw 2D UI and images, or load and render 3D models with PBR materials and skeletal animation.
- **3D Billboards**: Easily render 2D textures as 3D billboards that properly depth-sort with 3D objects.
- **Instanced Rendering**: Draw thousands of identical 3D models (grass, particles, crowds) natively in a single CPU draw call via `draw_instanced`.
- **Custom Shaders**: Inject custom WGSL code into the rendering pipeline for both 2D and 3D.
- **Image operations**: Load from files, create from raw data, extract sub-images.
- **Text rendering**: Custom font support, text wrapping, and styling (color, stroke).
- **Audio support**: Play sounds, sine waves, and handle fades/volume.
- **Input management**: High-level API for Keyboard, Mouse, and Touch events.
- **Scene management**: Easy switching between game scenes with payload support.
- **Resource management**: Built-in dependency injection for shared resources.
- **Cross-platform**: Support for Desktop, Web (WASM), iOS, and Android.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
spottedcat = "0.9.0"
```

By default, only the 2D core is enabled for maximum efficiency. To use 3D models or asset loaders (PNG/GLTF), enable the corresponding features:

```toml
[dependencies]
spottedcat = { version = "0.9.0", features = ["model-3d", "utils", "gltf", "effects", "sensors"] }
```

### Basic Example

```rust
use spottedcat::{Context, Spot, Image, DrawOption, Pt, WindowConfig};
use std::time::Duration;

struct MyApp {
    image: Image,
}

impl Spot for MyApp {
    fn initialize(ctx: &mut Context) -> Self {
        // Create an image from raw RGBA8 data
        let rgba = vec![255u8; 64 * 64 * 4]; // Red square
        let image = Image::new(ctx, Pt::from(64.0), Pt::from(64.0), &rgba)
            .expect("Failed to create image");
        Self { image }
    }

    fn update(&mut self, _ctx: &mut Context, _dt: Duration) {
        // Handle logic here
    }

    fn draw(&mut self, ctx: &mut Context, screen: Image) {
        let (w, h) = spottedcat::window_size(ctx);
        
        // Draw image at center
        let opts = DrawOption::default()
            .with_position([w / 2.0, h / 2.0])
            .with_scale([2.0, 2.0]);
            
        screen.draw(ctx, &self.image, opts);
    }

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    spottedcat::run::<MyApp>(WindowConfig {
        title: "SpottedCat Example".to_string(),
        ..Default::default()
    });
}
```

### Built-in Intro Scene

If you want the game to show a branded startup intro before entering your own
scene, wrap your root scene with `OneShotSplash<T>`:

```rust
use spottedcat::{Context, OneShotSplash, Spot, WindowConfig, run};

struct MyGame;

impl Spot for MyGame {
    fn initialize(_ctx: &mut Context) -> Self {
        Self
    }

    fn update(&mut self, _ctx: &mut Context, _dt: std::time::Duration) {}

    fn draw(&mut self, _ctx: &mut Context) {}
}

fn main() {
    run::<OneShotSplash<MyGame>>(WindowConfig::default());
}
```

The default intro uses a font-free pixel-art Rusty-spotted cat logo and
automatically switches to your main scene after a short delay. Players can
also skip with Space, Enter, mouse click, or touch. The splash is shown once
per process, so Android surface restoration resumes directly into your game.

## AI Assistant Guide

For comprehensive guidance on generating games and working with the `spottedcat` engine, please refer to the dedicated [AI Game Generation Guide](AI_GAME_GENERATION_GUIDE.md).

## API Overview

### Core Components

- **`Context`**: Central state for managing draw commands, input, audio, and resources. Hides internal methods to ensure consistency.
- **`Spot`**: Trait defining application lifecycle (`initialize`, `update`, `draw`, `remove`).
- **`Image`**: GPU texture handle for 2D drawing. Created via `Image::new(ctx, ...)` and rendered via `target.draw(ctx, &image, ...)`. Use the provided `screen` in `Spot::draw` as the default target.
- **`Model`**: 3D model handle created via `spottedcat::model::create(...)` and rendered via `spottedcat::model::*`.
- **`Text`**: High-level text structure for 2D layout.
- **`Interpolated<T>`**: Wrapper for game state that provides smooth interpolation between fixed logic updates.
- **`DrawOption`**: Unified configuration for layer, position, rotation, scale, and clipping in 2D.
- **`DrawOption3D`**: Configuration for 3D model placement (position, rotation, scale).

### API Style

- **Context-based operations (`ctx:*`)** live at crate top-level `spottedcat::*` (for example: `register_font`, `set_window_size`, `key_down`, `play_sound`), while image creation and drawing live on `Image` methods and model creation lives at `spottedcat::model::create`.
- **Assets**
 
Forces pending asset rebuild/re-upload to GPU to run immediately with `spottedcat::rebuild_assets(ctx)`.
- **Resource operations** stay inside their domains:
  - `Image` methods for creation, sub-image extraction, and target-based drawing.
  - `Texture` for managing underlying GPU textures and creating render targets.
  - `spottedcat::model::*` for model create/draw/instancing/shader draw (via `target.draw`).
  - `spottedcat::text::*` for text draw/measure (via `target.draw`).

- For encoded PNG/JPEG/WebP bytes, enable the `utils` feature and use `spottedcat::utils::image::from_image(...)` or `from_rgba_image(...)` after decoding with the `image` crate. These helpers preserve pixel dimensions and derive the default logical size from the current `scale_factor`.

### Key Systems

- **Input**: Check keys with `spottedcat::key_down(ctx, ...)`, mouse with `spottedcat::mouse_down(ctx, ...)`, or get `spottedcat::mouse_pos(ctx)`.
- **Audio**: Play sounds with `spottedcat::play_sound(ctx, ...)`.
- **Scenes**: Transition between states using `spottedcat::switch_scene::<NewScene>()`.
- **Resources**: Share data between systems via `spottedcat::get_resource::<T>(ctx)`.

### Sensors

Enable the `sensors` feature to access motion and step APIs.

- `today_step_count(ctx)` returns the current day's steps when the platform can provide them.
- `step_detected(ctx)` reports whether a new step was observed during the current frame.

Step semantics are intentionally limited to "today" for cross-platform consistency:

- **iOS** uses `CMPedometer` updates starting from the beginning of the current local day.
- **Android** derives today's steps from `TYPE_STEP_COUNTER` while the sensor stays registered.
- Neither API should be treated as a lifetime or historical total. Historical fitness data belongs in HealthKit or Health Connect integration.

### Model 3D

The `model-3d` feature gates the 3D model stack, including `Model`, `DrawOption3D`, custom model shaders, mesh loaders, and lighting. This feature is **disabled by default** to minimize the engine's footprint when only 2D features are needed.

## Performance & Timing

Spottedcat uses a decoupled main loop to ensure consistent game logic across different hardware while maintaining smooth visuals:

- **Fixed-Step Update (`UPS`)**: `Spot::update` is called at a fixed frequency (defaults to 60Hz). All physics and gameplay logic should happen here. The `dt` provided is constant.
- **Variable-Rate Draw (`FPS`)**: `Spot::draw` is called as fast as the display refresh rate or OS allows.
- **State Interpolation**: To prevent "stutter" when FPS and UPS don't match, use the `Interpolated<T>` wrapper for game state (like positions). It automatically smooths out values in `draw` calls using the engine's internal interpolation factor.

```rust
// In your Spot implementation
fn draw(&mut self, ctx: &mut Context, screen: Image) {
    // value(ctx) returns the smoothly interpolated position
    let pos = self.player_pos.value(ctx); 
    screen.draw(ctx, &self.player_img, DrawOption::new().with_position(pos));
}
```

## Advanced 2D Features

### Automatic Texture Atlasing
For performance efficiency, Spottedcat automatically manages small images (<512px) in a shared internal texture atlas. This reduces GPU state changes and draw calls under the hood without requiring any manual sprite sheet management from the developer.

## Custom Shaders

You can inject custom WGSL code into the fragment shader using `register_image_shader` (2D) and `register_model_shader` (3D).

### 3D Shader Exposed Variables

When using custom shaders for 3D models, your code is injected at `USER_FS_HOOK` at the end of the fragment shader. The following variables are available to read or modify:

- `final_color: vec4<f32>`: The computed PBR color (RGB) and opacity (A). You can modify this to change the final output.
- `in: VertexOutput`: Contains `in.uv`, `in.normal`, `in.world_pos`, and `in.clip_position`.
- `user_globals: array<vec4<f32>, 16>`: Custom uniform data passed from your Rust code via `ShaderOpts`.
- `scene: SceneGlobals`: Contains `scene.camera_pos`, `scene.ambient_color`, and `scene.lights`.
- `model_globals: ModelGlobals`: Contains `model_globals.mvp`, `model_globals.model`, `model_globals.extra` (x: opacity), and UV transforms.
- **Textures** (with `s_sampler`): `t_albedo`, `t_pbr`, `t_normal`, `t_ao`, `t_emissive`.

Example 3D shader hook:
```wgsl
// Make the model pulse based on the extra opacity parameter
let pulse = (sin(model_globals.extra.x * 10.0) + 1.0) * 0.5;
final_color = vec4<f32>(final_color.rgb * pulse, final_color.a);
```

## Platform Support

Declared support:

- **Desktop**: Windows, macOS, Linux.
- **Web**: Compile to WASM with `wasm-pack`. See `canvas_id` in `WindowConfig`.
- **Android**: Integrated with `winit`'s android-activity.
- **iOS**: Support for UIKit and native sensor access.


- **WASM example**: [`examples/wasm/web`](examples/wasm/web) and [`examples/wasm/wasm_demo`](examples/wasm/wasm_demo)
- **Android example**: [`examples/android/GameActivityExample`](examples/android/GameActivityExample) and [`examples/android/spottedcat_android_wrapper`](examples/android/spottedcat_android_wrapper)
- **iOS example**: [`examples/ios/SpottedcatIosSimulatorExample`](examples/ios/SpottedcatIosSimulatorExample) and [`examples/ios/spottedcat_ios_wrapper`](examples/ios/spottedcat_ios_wrapper)

Generated outputs for these examples such as `target/`, `.gradle/`, `pkg/`, `.xcframework/`, and IDE caches are intentionally excluded from version control.

## License

This project is licensed under either of:

- Apache License, Version 2.0
- MIT license

at your option.
