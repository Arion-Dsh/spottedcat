# Spottedcat

**A lightweight cross-platform 2D/3D game engine for Rust.**

[![Crates.io](https://img.shields.io/crates/v/spottedcat.svg)](https://crates.io/crates/spottedcat)
[![docs.rs](https://docs.rs/spottedcat/badge.svg)](https://docs.rs/spottedcat)
[![Example Checks](https://github.com/arion-dsh/spottedcat/actions/workflows/examples.yml/badge.svg)](https://github.com/arion-dsh/spottedcat/actions/workflows/examples.yml)
[![License](https://img.shields.io/crates/l/spottedcat.svg)](https://github.com/arion-dsh/spottedcat#license)

Spottedcat provides a small `Spot` lifecycle, 2D images and text, optional 3D models and effects, input, audio, and desktop/Web/iOS/Android targets.

- [Documentation](https://rustyspottedcat.dev)
- [API reference](https://docs.rs/spottedcat)
- [Examples](https://rustyspottedcat.dev/examples/)
- [AI-assisted creation](https://rustyspottedcat.dev/ai/)

## Install

```toml
[dependencies]
spottedcat = "1.0.3"
```

Enable optional capabilities as needed:

```toml
spottedcat = { version = "1.0.3", features = ["model-3d", "utils", "gltf", "effects", "sensors"] }
```

See the [feature guide](https://rustyspottedcat.dev/guide/core-concepts#choosing-features) for details.

## Quick start

```rust
use spottedcat::{Context, DrawOption, Image, Pt, Spot, WindowConfig};
use std::time::Duration;

struct Game {
    tile: Image,
}

impl Spot for Game {
    fn initialize(ctx: &mut Context) -> Self {
        let pixels = vec![255; 64 * 64 * 4];
        Self {
            tile: Image::new(ctx, Pt(64.0), Pt(64.0), &pixels).unwrap(),
        }
    }

    fn update(&mut self, _ctx: &mut Context, _dt: Duration) {}

    fn draw(&mut self, ctx: &mut Context, screen: Image) {
        let (width, height) = spottedcat::window_size(ctx);
        screen.draw(
            ctx,
            &self.tile,
            DrawOption::default().with_position([width / 2.0, height / 2.0]),
        );
    }

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    spottedcat::run::<Game>(WindowConfig::default());
}
```

`update` uses a configurable fixed step (60 Hz by default), while rendering follows the display refresh rate. See [Core Concepts](https://rustyspottedcat.dev/guide/core-concepts) for lifecycle and timing details.

## Development

```bash
cargo test --all-features
make check-examples
```

## License

Licensed under either Apache-2.0 or MIT, at your option.
