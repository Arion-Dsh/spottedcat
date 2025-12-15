# Scene Management in Spot

Spot supports scene management using Rust's enum pattern. A **Scene** represents a complete game screen or state (like Main Menu, Game Play, Pause Menu, etc.), not individual UI components or entities.

## Scene vs Components

- **Scene** = Complete screen/state (Main Menu, Game Play, Settings, etc.)
- **Components** = Elements within a scene (buttons, player, enemies, UI widgets, etc.)

## Recommended Approach: Scene Enum

The way to handle scene switching in Spot is to use an enum to represent all your scenes:

```rust
use spot::{Context, Spot, Image, DrawOptions};

// Define your scenes as an enum
enum MyScene {
    Menu(MenuScene),
    Game(GameScene),
    GameOver(GameOverScene),
}

// Implement Spot for the enum
impl Spot for MyScene {
    fn initialize(context: Context) -> Self {
        // Start with the menu scene
        MyScene::Menu(MenuScene::initialize(context))
    }

    fn draw(&mut self, context: &mut Context) {
        match self {
            MyScene::Menu(scene) => scene.draw(context),
            MyScene::Game(scene) => scene.draw(context),
            MyScene::GameOver(scene) => scene.draw(context),
        }
    }

    fn update(&self, event: spot::Event) {
        match self {
            MyScene::Menu(scene) => scene.update(event),
            MyScene::Game(scene) => scene.update(event),
            MyScene::GameOver(scene) => scene.update(event),
        }
    }

    fn remove(&self) {
        match self {
            MyScene::Menu(scene) => scene.remove(),
            MyScene::Game(scene) => scene.remove(),
            MyScene::GameOver(scene) => scene.remove(),
        }
    }
}

// Individual scene implementations
struct MenuScene {
    background: Image,
}

impl MenuScene {
    fn initialize(context: Context) -> Self {
        let background = Image::new_from_file("menu_bg.png").unwrap();
        Self { background }
    }

    fn draw(&mut self, context: &mut Context) {
        let mut opts = DrawOptions::default();
        opts.position = [0.0, 0.0];
        opts.size = [800.0, 600.0];
        self.background.draw(context, opts);
        
        // Check for user input to switch scenes
        // In a real app, you'd handle this in update() with actual events
        // For now, we'll switch after some condition
    }

    fn update(&self, _event: spot::Event) {}
    fn remove(&self) {}
}

struct GameScene {
    player: Image,
}

impl GameScene {
    fn initialize(context: Context) -> Self {
        let player = Image::new_from_file("player.png").unwrap();
        Self { player }
    }

    fn draw(&mut self, context: &mut Context) {
        let mut opts = DrawOptions::default();
        opts.position = [100.0, 100.0];
        opts.size = [64.0, 64.0];
        self.player.draw(context, opts);
    }

    fn update(&self, _event: spot::Event) {}
    fn remove(&self) {}
}

struct GameOverScene {
    game_over_text: Image,
}

impl GameOverScene {
    fn initialize(context: Context) -> Self {
        let game_over_text = Image::new_from_file("game_over.png").unwrap();
        Self { game_over_text }
    }

    fn draw(&mut self, context: &mut Context) {
        let mut opts = DrawOptions::default();
        opts.position = [200.0, 200.0];
        opts.size = [400.0, 100.0];
        self.game_over_text.draw(context, opts);
    }

    fn update(&self, _event: spot::Event) {}
    fn remove(&self) {}
}

fn main() {
    spot::run::<MyScene>();
}
```

## Scene Switching with State Management

For more complex applications, you can add state management to handle scene transitions:

```rust
enum MyScene {
    Menu(MenuScene),
    Game(GameScene),
    GameOver(GameOverScene),
}

impl MyScene {
    // Helper method to transition between scenes
    fn transition_to(&mut self, new_scene: MyScene) {
        // Call remove on the old scene
        match self {
            MyScene::Menu(scene) => scene.remove(),
            MyScene::Game(scene) => scene.remove(),
            MyScene::GameOver(scene) => scene.remove(),
        }
        // Replace with new scene
        *self = new_scene;
    }
}

impl Spot for MyScene {
    fn initialize(context: Context) -> Self {
        MyScene::Menu(MenuScene::initialize(context))
    }

    fn draw(&mut self, context: &mut Context) {
        // Check for scene transitions
        let should_transition = match self {
            MyScene::Menu(scene) => {
                scene.draw(context);
                scene.should_start_game() // Returns Option<MyScene>
            }
            MyScene::Game(scene) => {
                scene.draw(context);
                scene.check_game_over() // Returns Option<MyScene>
            }
            MyScene::GameOver(scene) => {
                scene.draw(context);
                scene.should_return_to_menu() // Returns Option<MyScene>
            }
        };

        // Perform transition if needed
        if let Some(next_scene) = should_transition {
            self.transition_to(next_scene);
        }
    }

    fn update(&self, event: spot::Event) {
        match self {
            MyScene::Menu(scene) => scene.update(event),
            MyScene::Game(scene) => scene.update(event),
            MyScene::GameOver(scene) => scene.update(event),
        }
    }

    fn remove(&self) {
        match self {
            MyScene::Menu(scene) => scene.remove(),
            MyScene::Game(scene) => scene.remove(),
            MyScene::GameOver(scene) => scene.remove(),
        }
    }
}
```

## Benefits of the Enum Approach

1. **Type Safety**: All scenes are known at compile time
2. **Pattern Matching**: Rust's exhaustive pattern matching ensures you handle all scenes
3. **No Runtime Overhead**: No dynamic dispatch for scene selection
4. **Clear State Machine**: The enum clearly shows all possible scenes
5. **Easy Refactoring**: Adding or removing scenes is straightforward

## How Scene Switching Works

### Method 1: Using the `switch_scene` Function

Spot provides a top-level `switch_scene` function that automatically handles scene transitions:

```rust
use spot::{switch_scene, Context, Spot, Image, DrawOptions};

struct MenuScene {
    background: Image,
}

impl Spot for MenuScene {
    fn initialize(context: Context) -> Self {
        let background = Image::new_from_file("menu.png").unwrap();
        Self { background }
    }

    fn draw(&mut self, context: &mut Context) {
        let mut opts = DrawOptions::default();
        opts.position = [0.0, 0.0];
        opts.size = [800.0, 600.0];
        self.background.draw(context, opts);
        
        // Check for user input (e.g., button press)
        if user_pressed_start_button() {
            // Simply call switch_scene - it will handle everything!
            switch_scene::<GameScene>();
        }
    }

    fn update(&self, _event: spot::Event) {}
    fn remove(&self) {
        println!("Menu scene cleaned up");
    }
}

struct GameScene {
    player: Image,
}

impl Spot for GameScene {
    fn initialize(context: Context) -> Self {
        let player = Image::new_from_file("player.png").unwrap();
        Self { player }
    }

    fn draw(&mut self, context: &mut Context) {
        let mut opts = DrawOptions::default();
        opts.position = [100.0, 100.0];
        opts.size = [64.0, 64.0];
        self.player.draw(context, opts);
        
        // Check for game over
        if game_is_over() {
            switch_scene::<GameOverScene>();
        }
    }

    fn update(&self, _event: spot::Event) {}
    fn remove(&self) {
        println!("Game scene cleaned up");
    }
}

fn main() {
    spot::run::<MenuScene>();
}

// Helper functions (implement based on your input system)
fn user_pressed_start_button() -> bool { false }
fn game_is_over() -> bool { false }
```

**How it works:**
- Call `switch_scene::<NewScene>()` anywhere in your scene
- The switch happens at the end of the current frame
- Old scene's `remove()` is called automatically
- New scene's `initialize()` is called automatically
- The running scene in `App.spot` is replaced with the new scene

### Method 2: Manual Scene Management

Scene switching can also be handled internally by your scene enum. When a scene needs to transition, it modifies itself:

```rust
impl Spot for MyScene {
    fn draw(&mut self, context: &mut Context) {
        // Draw current scene
        match self {
            MyScene::Menu(scene) => scene.draw(context),
            MyScene::Game(scene) => scene.draw(context),
            MyScene::GameOver(scene) => scene.draw(context),
        }
        
        // Check for transitions and update self
        if let Some(next) = self.check_transition() {
            *self = next;
        }
    }
}

impl MyScene {
    fn check_transition(&mut self) -> Option<Self> {
        match self {
            MyScene::Menu(scene) if scene.start_pressed() => {
                Some(MyScene::Game(GameScene::initialize(Context::new())))
            }
            MyScene::Game(scene) if scene.is_game_over() => {
                Some(MyScene::GameOver(GameOverScene::initialize(Context::new())))
            }
            _ => None
        }
    }
}
```

## Best Practices

1. **Use enums for scene management** - Most flexible and type-safe
2. **Keep scenes independent** - Each scene should manage its own resources
3. **Clean up resources** - Implement `remove()` to free resources when switching
4. **Avoid deep nesting** - Keep scene hierarchies flat when possible
5. **Use state machines** - For complex scene transitions, consider a formal state machine pattern
