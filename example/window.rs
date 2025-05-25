
use spottedcat::{Spot, Image};

struct MySpot {
    tree: Image,
}

impl Spot for MySpot {
    fn preload(&mut self) {
        // Your drawing logic here
        // This is where you would implement your rendering logic
        self.tree.load();
    }
    fn update(&mut self, _dt: f32) {
        // Your drawing logic here
        // This is where you would implement your rendering logic
    }
    fn draw(&self, _screen: &mut Image) {
        _screen.draw(self.tree.clone());
    }
    fn release(&self) {
        // Your drawing logic here
        // This is where you would implement your rendering logic
    }
}

fn main() {
    let img = Image::new_from_path("happy-tree.png");
    spottedcat::run(MySpot { tree: img });
}
