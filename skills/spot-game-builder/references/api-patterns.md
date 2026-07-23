# Spottedcat consumer API patterns

Use these patterns as a starting point, then verify them against the version selected in the consumer project's `Cargo.lock`.

## Minimal scene

```rust
use spottedcat::{Context, Image, Spot, WindowConfig};
use std::time::Duration;

struct Game;

impl Spot for Game {
    fn initialize(_ctx: &mut Context) -> Self {
        Self
    }

    fn update(&mut self, _ctx: &mut Context, _dt: Duration) {}

    fn draw(&mut self, _ctx: &mut Context, _screen: Image) {}
}

fn main() {
    spottedcat::run::<Game>(WindowConfig {
        title: "My Game".into(),
        update_hz: 60,
        ..Default::default()
    });
}
```

`update_hz` controls the fixed simulation step. Rendering follows the platform/display independently.

## 2D images and input

Create or register images once and store their handles:

```rust
let pixels = vec![255_u8; 32 * 32 * 4];
let player = spottedcat::Image::new(ctx, spottedcat::Pt(32.0), spottedcat::Pt(32.0), &pixels)?;
```

Advance positions in `update`:

```rust
let seconds = dt.as_secs_f32();
if spottedcat::key_down(ctx, spottedcat::Key::Right) {
    self.x += self.speed * seconds;
}
if spottedcat::key_pressed(ctx, spottedcat::Key::Space) {
    self.jump();
}
```

Draw through the target supplied to `draw`:

```rust
screen.draw(
    ctx,
    &self.player,
    spottedcat::DrawOption::default()
        .with_position([spottedcat::Pt(self.x), spottedcat::Pt(self.y)])
        .with_layer(10),
);
```

Use `window_size`, `vw`, and `vh` for logical responsive layout. `mouse_pos` returns logical `Pt` coordinates; `touches` exposes active touches.

## Text

Register font bytes in `initialize`, construct stable `Text` values once, and mutate only when their value changes:

```rust
let font_id = spottedcat::register_font(ctx, include_bytes!("../assets/font.ttf").to_vec());
let score = spottedcat::Text::new("Score: 0", font_id)
    .with_font_size(spottedcat::Pt(24.0));

// Later, in update when the score changes:
self.score.set_content(format!("Score: {}", self.points));
```

Draw text with the same `screen.draw(ctx, &self.score, DrawOption)` pattern as images.

Spottedcat does not provide an implicit default font: `Text` always needs a font ID registered from real TTF/OTF bytes. Reuse a font asset or loader already present in the consumer project. If none exists, make the missing asset requirement explicit. Do not hard-code operating-system font paths for a cross-platform game unless the target platform is intentionally restricted and that tradeoff is stated.

## Encoded and asynchronous images

Enable `utils` for PNG/JPEG/WebP helpers. Decode with the `image` crate and use `spottedcat::utils::image::from_image` or `from_rgba_image` to preserve source pixel dimensions and scale-factor-aware logical sizing.

For background file loading, keep an `AsyncImageLoader` in scene state, call `load` once, and poll `get`, `is_ready`, `is_done`, or `progress_ratio` from `update`. Do not start loads repeatedly in `draw`.

## Audio

Register encoded sound bytes once:

```rust
let sound_id = spottedcat::register_sound(ctx, bytes).expect("supported audio data");
spottedcat::play_sound(ctx, sound_id, spottedcat::SoundOptions::default());
```

Use `play_sine` only for simple generated feedback or smoke tests.

## 3D

Enable `model-3d`, configure the camera during initialization or update, and draw models through the same target:

```rust
let cube = spottedcat::model::create_cube(ctx, 1.0)?;
spottedcat::set_camera_pos(ctx, [0.0, 2.0, 6.0]);
spottedcat::set_camera_target(ctx, 0.0, 0.0, 0.0);

screen.draw(
    ctx,
    &self.cube,
    spottedcat::DrawOption3D::default()
        .with_position([0.0, 0.0, -2.0])
        .with_rotation([0.0, self.angle, 0.0]),
);
```

For many copies of one model, prefer `spottedcat::model::draw_instanced` over many individual draw calls. Enable `effects` for fog and clear scene-global fog in `remove` when the next scene should not inherit it.

## Scenes and resources

- `switch_scene::<NextScene>()` replaces the current scene.
- `switch_scene_with::<NextScene, _>(payload)` passes a typed payload; retrieve it with `take_resource` during initialization.
- `insert_resource` and `get_resource` store shared `Rc<T>` values in `Context`.
- Keep ordinary scene-local mutable state on the `Spot` implementation.

## Custom shaders

Prefer `ImageShaderTemplate` and `ModelShaderTemplate` for common effects. Use full WGSL descriptors only when the template cannot express the effect. Keep shader handles in scene state and bind screen/history textures through `ImageShaderBindings` semantics rather than hard-coded assumptions.

## Common failure checks

- Missing item: confirm its feature flag and selected Spottedcat version.
- `Pt` mismatch: convert logical values explicitly with `Pt(value)` or `Pt::from(value)`.
- Blank image: confirm RGBA byte length, readiness, position, scale, opacity, and layer.
- Stuttering movement: mutate in fixed `update`; interpolate only for presentation.
- Slow frame: stop allocating text/assets in `draw`; batch repeated 3D models with instancing.
- Platform build failure: validate desktop/core first, then isolate wrapper/toolchain errors.
