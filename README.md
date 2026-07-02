# Spottedcat

**Lightweight cross-platform 2D/3D game engine for Rust.**

Spottedcat is small, fast, and a little wild—named after the rusty-spotted cat and built for desktop, Web/WASM, iOS, Android, and AI-assisted game creation.

> [!NOTE]
> **Spottedcat 1.0 is stable.** Public API breakage is reserved for future major versions; minor and patch releases focus on compatibility, fixes, and additive improvements.

## Why Spottedcat?

- **Small core API**: build around `Spot`, `Context`, `Image`, `Text`, `Model`, and `run`.
- **2D and 3D**: draw images, text, custom shaders, primitives, GLTF models, billboards, instanced models, and foggy 3D scenes.
- **Cross-platform**: target desktop, Web/WASM, iOS, and Android from one Rust codebase.
- **AI-friendly**: stable lifecycle, focused examples, and a dedicated guide make it easier for AI tools to generate runnable game prototypes.
- **Practical by default**: start with zero default features for lean 2D apps; enable 3D, effects, GLTF, image helpers, or sensors only when needed.

## Links

- Documentation: [rustyspottedcat.dev](https://rustyspottedcat.dev)
- API reference: [docs.rs/spottedcat](https://docs.rs/spottedcat)
- Crate: [crates.io/crates/spottedcat](https://crates.io/crates/spottedcat)
- AI generation guide: [AI_GAME_GENERATION_GUIDE.md](AI_GAME_GENERATION_GUIDE.md)

## Install

Minimal 2D core:

```toml
[dependencies]
spottedcat = "1.0.1"
```

Common feature set for richer projects:

```toml
[dependencies]
spottedcat = { version = "1.0.1", features = ["model-3d", "utils", "gltf", "effects", "sensors"] }
```

Feature guide:

| Feature | Use when you need |
| --- | --- |
| `model-3d` | 3D models, primitives, billboards, cameras, and model shaders |
| `effects` | 3D effects such as fog |
| `utils` | PNG/JPEG/WebP helper loading and async image loading |
| `gltf` | GLTF model loading; also enables `model-3d` and `utils` |
| `sensors` | Motion and step APIs on supported mobile platforms |

## Minimal app

Spottedcat apps are ordinary Rust types that implement `Spot`:

```rust
use spottedcat::{Context, DrawOption, Image, Pt, Spot, WindowConfig, run};
use std::time::Duration;

struct Game {
    player: Image,
}

impl Spot for Game {
    fn initialize(ctx: &mut Context) -> Self {
        let rgba = vec![255; 32 * 32 * 4];
        let player = Image::new(ctx, Pt::from(32.0), Pt::from(32.0), &rgba)
            .expect("create player image");

        Self { player }
    }

    fn update(&mut self, _ctx: &mut Context, _dt: Duration) {
        // Move your world forward here.
    }

    fn draw(&mut self, ctx: &mut Context, screen: Image) {
        let (w, h) = spottedcat::window_size(ctx);
        screen.draw(
            ctx,
            &self.player,
            DrawOption::default().with_position([w / 2.0, h / 2.0]),
        );
    }

    fn remove(&mut self, _ctx: &mut Context) {}
}

fn main() {
    run::<Game>(WindowConfig {
        title: "Spottedcat".to_string(),
        ..Default::default()
    });
}
```

Run an example:

```bash
cargo run --example input_example
cargo run --example fog_world --features model-3d,effects
cargo run --example gltf_loader --features gltf
```

## AI-assisted creation

Spottedcat is intentionally easy for AI coding tools to read and extend:

- the game loop is explicit: `initialize`, `update`, `draw`, `remove`
- examples are small enough to use as generation anchors
- the 1.0 API gives AI tools a stable surface to target

If you use Codex or another AI coding tool, a good prompt shape is:

```text
Use Spottedcat 1.0 to make a tiny playable 2D dodge game.
Start from one Spot scene, read keyboard input, draw simple images,
show score text, and keep the code close to the existing examples.
```

See [AI_GAME_GENERATION_GUIDE.md](AI_GAME_GENERATION_GUIDE.md) for LLM-oriented API guidance.

## Examples

Useful starting points:

| Goal | Example |
| --- | --- |
| Input and HUD | `examples/input_example.rs` |
| Image loading | `examples/rgb_image.rs`, `examples/async_loading_example.rs` |
| Text rendering | `examples/centered_text_test.rs` |
| Audio | `examples/audio_test.rs` |
| 2D shaders | `examples/image_shader_template.rs` |
| 3D world | `examples/fog_world.rs` |
| GLTF models | `examples/gltf_loader.rs`, `examples/animated_gltf.rs` |
| Instancing | `examples/instancing_test.rs` |
| WASM demos | `examples/wasm/` |
| Mobile wrappers | `examples/ios/`, `examples/android/` |

For a guided mini-game, see the Flappy Cat guide on [rustyspottedcat.dev](https://rustyspottedcat.dev/guide/flappy-cat).

## Core concepts

### `Spot`

`Spot` is the application or scene lifecycle:

- `initialize` loads assets and creates state.
- `update` runs fixed-step gameplay logic.
- `draw` renders the current frame.
- `resumed` and `suspended` handle platform lifecycle when needed.
- `remove` cleans up scene-specific state.

Keep gameplay mutation in `update` and rendering in `draw`. Use interpolation when fixed updates and display refresh do not line up.

### Rendering

- `Image` handles 2D textures, render targets, sub-images, layers, transforms, clipping, and custom 2D shaders.
- `Text` handles font-backed text rendering, wrapping, color, and stroke.
- `Model` handles 3D primitives, GLTF content, billboards, instancing, lighting, fog, and custom model shaders when `model-3d` is enabled.

### Platform support

Declared support:

- **Desktop**: Windows, macOS, Linux
- **Web**: WASM builds
- **iOS**: UIKit integration and native sensor access
- **Android**: Android GameActivity integration and native sensor access

Generated platform outputs such as `target/`, `.gradle/`, `pkg/`, `.xcframework/`, and IDE caches are intentionally excluded from version control.

## Stability

Spottedcat 1.0 treats the main user-facing surfaces as stable:

- `Context`
- `Spot`
- `Image`
- `Text`
- `Model`
- `WindowConfig`
- `run`

Minor and patch releases should preserve public API compatibility wherever possible. Pin an exact crate version if you need fully reproducible builds.

## Name

The name comes from the **rusty-spotted cat** (*Prionailurus rubiginosus*): tiny, quick, and a little wild. It also nods to Rust, the language behind the engine.

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT License

at your option.
