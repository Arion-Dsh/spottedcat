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
spottedcat = "0.7.0"
```

By default, only the 2D core is enabled for maximum efficiency. To use 3D models or asset loaders (PNG/GLTF), enable the corresponding features:

```toml
[dependencies]
spottedcat = { version = "0.7.0", features = ["model-3d", "utils", "gltf", "effects", "sensors"] }
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
        // Create an image from raw RGBA8 data (or use the 'image' crate to load pixels)
        let rgba = vec![255u8; 64 * 64 * 4]; // Red square
        let image = Image::new_from_rgba8(ctx, Pt::from(64.0), Pt::from(64.0), &rgba)
            .expect("Failed to create image");
        Self { image }
    }

    fn update(&mut self, _ctx: &mut Context, _dt: Duration) {
        // Handle logic here
    }

    fn draw(&mut self, ctx: &mut Context) {
        let (w, h) = spottedcat::window_size(ctx);
        
        // Draw image at center
        let opts = DrawOption::default()
            .with_position([w / 2.0, h / 2.0])
            .with_scale([2.0, 2.0]);
        self.image.draw(ctx, opts);
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

## AI Assistant Guide

For comprehensive guidance on generating games and working with the `spottedcat` engine, please refer to the dedicated [AI Game Generation Guide](AI_GAME_GENERATION_GUIDE.md).

## API Overview

### Core Components

- **`Context`**: Central state for managing draw commands, input, audio, and resources.
- **`Spot`**: Trait defining application lifecycle (`initialize`, `update`, `draw`, `remove`).
- **`Image`**: GPU texture handle for 2D drawing. Supports sub-images and raw data creation.
- **`Model`**: 3D model handle for rendering meshes, PBR materials, and skeletal animations. Supports extreme performance Instanced Rendering (`draw_instanced`).
- **`Text`**: High-level text rendering with font registration and layout.
- **`DrawOption`**: Unified configuration for layer, position, rotation, scale, and clipping in 2D.
- **`DrawOption3D`**: Configuration for 3D model placement (position, rotation, scale).

### Key Systems

- **Input**: Check keys with `key_down(ctx, ...)`, mouse with `mouse_button_pressed(ctx, ...)`, or get `touches(ctx)`.
- **Audio**: Load and play sounds with `play_sound(ctx, ...)`, or generate tones with `play_sine(ctx, ...)`.
- **Scenes**: Transition between states using `switch_scene::<NewScene>()`.
- **Resources**: Share data between systems via `ctx.get_resource::<T>()`.

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
