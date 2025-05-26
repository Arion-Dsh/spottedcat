
use spottedcat::{Spot, Image};

struct MySpot {
    tree: Image,
    track: Image,
}

impl Spot for MySpot {
    fn preload(&mut self) {
        // Your drawing logic here
        // This is where you would implement your rendering logic
        self.track.load();
        self.tree.load();
    }
    fn update(&mut self, _dt: f32) {
        // Your drawing logic here
        // This is where you would implement your rendering logic
    }
    fn draw(&mut self, _screen: &mut Image) {
        let _ = _screen.draw(self.tree.clone());
        let _ = _screen.draw(self.track.clone());
    }
    fn release(&mut self) {
        // Your drawing logic here
        // This is where you would implement your rendering logic
    }
}

fn main() {
    let img = Image::new_from_path("happy-tree.png");
    let track = Image::new_from_path("track.png");
    spottedcat::run(MySpot { tree: img, track });
}
