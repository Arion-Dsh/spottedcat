# Spottedcat upstream reference map

Use this file only when an upstream example, engine source detail, or platform guide is needed. Paths refer to the Spottedcat engine repository, not the consumer game project.

## Documentation

- Main guide: <https://rustyspottedcat.dev>
- API reference: <https://docs.rs/spottedcat>
- Examples: <https://github.com/Arion-Dsh/spottedcat/tree/main/examples>
- 2D rendering: <https://rustyspottedcat.dev/graphics/2d>
- 3D rendering: <https://rustyspottedcat.dev/graphics/3d>
- Custom shaders: <https://rustyspottedcat.dev/graphics/shaders>
- Web and mobile: <https://rustyspottedcat.dev/platforms/>

When the consumer uses an older release, verify signatures against that release's docs or locally downloaded source rather than copying `main` blindly.

## Example chooser

| Need | Upstream example |
| --- | --- |
| Keyboard movement and text HUD | `examples/input_example.rs` |
| Audio | `examples/audio_test.rs` |
| Decoded image loading | `examples/happy_tree_desktop.rs` |
| Asynchronous images | `examples/async_loading_example.rs` |
| 2D shader template | `examples/image_shader_template.rs` |
| 3D camera, lighting, and fog | `examples/fog_world.rs` |
| Repeated 3D models | `examples/instancing_test.rs` |
| glTF loading | `examples/gltf_loader.rs` |
| 2D art in a 3D scene | `examples/billboard.rs` |
| Browser wrapper | `examples/wasm/` |
| Android wrapper | `examples/android/` |
| iOS wrapper | `examples/ios/` |

## Resolve the installed API

Use the consumer project first:

```bash
cargo tree -p spottedcat
cargo metadata --format-version 1
```

For a local path dependency, inspect its `src/lib.rs`, `Cargo.toml`, and nearest example. For a registry dependency, inspect docs.rs for the selected release or the dependency source identified by Cargo metadata. Search exact symbols instead of guessing names.

## Consumer-project validation

Run commands from the game project and use its existing aliases or task runner when present:

```bash
cargo fmt --check
cargo check
cargo test
cargo run
```

For WASM, Android, and iOS, follow the consumer project's wrapper and build scripts. Upstream wrapper directories are structural references, not drop-in commands for every game.
