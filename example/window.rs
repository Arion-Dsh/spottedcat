use spottedcat::{DrawOpt, Image, Spot};
struct MySpot {
    tree: Image,
    track: Image,
}

impl Spot for MySpot {
    fn preload(&mut self) {
        // Your drawing logic here
        // This is where you would implement your rendering logic
        // self.track.load();
        // self.tree.load();
    }
    fn update(&mut self, _dt: f32) {
        // Your drawing logic here
        // This is where you would implement your rendering logic
    }

    fn draw(&mut self, _screen: &mut Image) {
        let mut opt = DrawOpt::default();
        opt.translate(100.0, 100.0, 0.0);
        let _ = _screen.draw(self.tree.clone(), opt);
        let _ = _screen.draw(self.track.clone(), DrawOpt::default());
    }
    fn release(&mut self) {
        // Your drawing logic here
        // This is where you would implement your rendering logic
    }
}

fn main() {
    let img = Image::new_from_path("happy-tree.png").expect("Failed to load tree image");
    let track = Image::new_from_path("track.png").expect("Failed to load track image");
    spottedcat::run(MySpot { tree: img, track });
}
