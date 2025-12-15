# Spot

A simple, clean 2D graphics library for drawing images using Rust and wgpu.

## Features

- **Simple API**: Only 4 main types to learn: `Context`, `Spot`, `Image`, and `run`
- **GPU-accelerated**: Built on wgpu for high-performance rendering
- **Image operations**: Load from files, create from raw data, extract sub-images
- **Flexible drawing**: Position, scale, rotate images with ease

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
spot = { path = "." }
```

### Basic Example

```rust
use spot::{Context, Spot, Image, DrawOptions};

struct MyApp {
    image: Image,
}

impl Spot for MyApp {
    fn initialize(_context: Context) -> Self {
        let image = Image::new_from_file("image.png")
            .expect("Failed to load image");
        Self { image }
    }

    fn draw(&mut self, context: &mut Context) {
        let mut opts = DrawOptions::default();
        opts.position = [100.0, 100.0];
        opts.scale = [2.0, 2.0];
        context.draw_image(self.image, opts);
    }

    fn update(&self, _event: spot::Event) {}
    fn remove(&self) {}
}

fn main() {
    spot::run(|ctx| Box::new(MyApp::initialize(ctx)));
}
```

## API Overview

### Core Types

#### `Context`
Drawing context for managing render commands. Accumulates drawing operations during a frame.

**Methods:**
- `new()` - Create a new context
- `begin_frame()` - Clear previous frame's commands
- `draw_image(image, options)` - Queue an image for drawing

#### `Spot` (trait)
Main application trait defining the lifecycle of your app.

**Required methods:**
- `initialize(context)` - Set up initial state and load resources
- `draw(&mut context)` - Render the current frame
- `update(event)` - Handle events (reserved for future use)
- `remove()` - Cleanup on shutdown

#### `Image`
Handle to a GPU texture that can be drawn to the screen.

**Methods:**
- `new_from_rgba8(width, height, rgba)` - Create from raw pixel data
- `new_from_file(path)` - Load from image file (PNG, JPEG, etc.)
- `new_from_image(image)` - Clone an existing image
- `sub_image(image, bounds)` - Extract a region from an image
- `destroy()` - Free GPU resources

#### `DrawOptions`
Options for controlling how images are rendered.

**Fields:**
- `position: [f32; 2]` - Top-left corner in screen pixels
- `rotation: f32` - Rotation in radians
- `scale: [f32; 2]` - Scale factors (applied after the image's intrinsic size)

#### `Bounds`
Rectangle for defining sub-regions of images.

**Fields:**
- `x: u32` - X coordinate
- `y: u32` - Y coordinate  
- `width: u32` - Width
- `height: u32` - Height

### Functions

#### `run(init)`
Main entry point. Creates a window, initializes graphics, and runs the event loop.

**Arguments:**
- `init: fn(Context) -> Box<dyn Spot>` - Function to create your app

## Advanced Usage

### Creating Sub-Images

Extract regions from existing images without duplicating GPU memory:

```rust
let full_image = Image::new_from_file("spritesheet.png")?;
let sprite = Image::sub_image(
    full_image,
    Bounds::new(0, 0, 32, 32)
)?;
```

### Drawing with Transformations

```rust
let mut opts = DrawOptions::default();
opts.position = [400.0, 300.0];
opts.rotation = std::f32::consts::PI / 4.0; // 45 degrees
opts.scale = [2.0, 2.0]; // Double size
context.draw_image(image, opts);
```

### Creating Images from Raw Data

```rust
let mut rgba = vec![0u8; 64 * 64 * 4];
// Fill with your pixel data...
let image = Image::new_from_rgba8(64, 64, &rgba)?;
```

## Project Structure

```
spot/
├── src/
│   ├── lib.rs          # Public API and core types
│   ├── graphics.rs     # Graphics system and rendering
│   ├── image.rs        # Image resource management
│   ├── image_raw.rs    # Low-level image rendering
│   ├── texture.rs      # GPU texture wrapper
│   ├── drawable.rs     # Drawing primitives
│   ├── window.rs       # Window and event loop
│   └── main.rs         # Example application
├── Cargo.toml
└── README.md
```

## Architecture

The library is organized into clear layers:

1. **Public API** (`lib.rs`): Clean, minimal surface exposing only essential types
2. **Graphics System** (`graphics.rs`): Manages GPU state and rendering pipeline
3. **Resource Management** (`image.rs`, `texture.rs`): Handles image lifecycle
4. **Rendering** (`image_raw.rs`): Low-level GPU rendering operations
5. **Application** (`window.rs`): Window management and event loop

## Dependencies

- `wgpu` - Modern GPU API
- `winit` - Cross-platform windowing
- `image` - Image loading and decoding
- `bytemuck` - Safe type casting
- `anyhow` - Error handling

## License

See LICENSE file for details.
