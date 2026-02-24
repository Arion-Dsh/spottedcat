# spottedcat

A simple, clean 2D graphics library for drawing images using Rust and wgpu.

## Why spottedcat?

The library is named after the **Rusty-spotted cat** (*Prionailurus rubiginosus*), the world's smallest wild cat. Just like its namesake, this library aims to be tiny, agile, and remarkably efficient.


## Features

- **Simple API**: Minimal core types to learn: `Context`, `Spot`, `Image`, `Text`, and `run`.
- **GPU-accelerated**: Built on wgpu for high-performance rendering.
- **Image operations**: Load from files, create from raw data, extract sub-images.
- **Text rendering**: Custom font support, text wrapping, and styling (color, stroke).
- **Audio support**: Play sounds (PNG/WAV/etc.), sine waves, and handle fades/volume.
- **Input management**: High-level API for Keyboard, Mouse, and Touch events.
- **Scene management**: Easy switching between game scenes with payload support.
- **Resource management**: Built-in dependency injection for shared resources.
- **Cross-platform**: Support for Desktop, Web (WASM), and Android.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
spottedcat = "0.2.9"
```

### Basic Example

```rust
use spottedcat::{Context, Spot, Image, DrawOption, Pt, WindowConfig};
use std::time::Duration;

struct MyApp {
    image: Image,
}

impl Spot for MyApp {
    fn initialize(context: &mut Context) -> Self {
        // Create an image from raw RGBA8 data (or use the 'image' crate to load pixels)
        let rgba = vec![255u8; 64 * 64 * 4]; // Red square
        let image = Image::new_from_rgba8(Pt::from(64.0), Pt::from(64.0), &rgba)
            .expect("Failed to create image");
        Self { image }
    }

    fn update(&mut self, _context: &mut Context, _dt: Duration) {
        // Handle logic here
    }

    fn draw(&mut self, context: &mut Context) {
        let (w, h) = spottedcat::window_size(context);
        
        // Draw image at center
        let opts = DrawOption::default()
            .with_position([w / 2.0, h / 2.0])
            .with_scale([2.0, 2.0]);
        self.image.draw(context, opts);
    }

    fn remove(&self) {}
}

fn main() {
    spottedcat::run::<MyApp>(WindowConfig {
        title: "SpottedCat Example".to_string(),
        ..Default::default()
    });
}
```

## API Overview

### Core Components

- **`Context`**: Central state for managing draw commands, input, audio, and resources.
- **`Spot`**: Trait defining application lifecycle (`initialize`, `update`, `draw`, `remove`).
- **`Image`**: GPU texture handle for drawing. Supports sub-images and raw data creation.
- **`Text`**: High-level text rendering with font registration and layout.
- **`DrawOption`**: Unified configuration for position, rotation, scale, and clipping.

### Key Systems

- **Input**: Check keys with `key_down`, mouse with `mouse_button_pressed`, or get `touches`.
- **Audio**: Load and play sounds with `play_sound`, or generate tones with `play_sine`.
- **Scenes**: Transition between states using `switch_scene::<NewScene>()`.
- **Resources**: Share data between systems via `context.get_resource::<T>()`.

## Platform Support

- **Desktop**: Windows, macOS, Linux.
- **Web**: Compile to WASM with `wasm-pack`. See `canvas_id` in `WindowConfig`.
- **Android**: Integrated with `winit`'s android-activity.

## License

This project is licensed under either of:

- Apache License, Version 2.0
- MIT license

at your option.

