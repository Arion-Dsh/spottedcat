 

fn main() {
    struct DrawSubDemo {
        a: spottedcat::Image,
        b: spottedcat::Image,
        c: spottedcat::Image,
    }

    impl spottedcat::Spot for DrawSubDemo {
        fn initialize(_context: &mut spottedcat::Context) -> Self {
            let mut a_rgba = vec![0u8; 400 * 400 * 4];
            for i in 0..(400 * 400) {
                a_rgba[i * 4] = 50;
                a_rgba[i * 4 + 1] = 50;
                a_rgba[i * 4 + 2] = 200;
                a_rgba[i * 4 + 3] = 255;
            }
            let a = spottedcat::Image::new_from_rgba8(
                spottedcat::Pt::from(400.0),
                spottedcat::Pt::from(400.0),
                &a_rgba,
            )
            .expect("failed to create A");

            let mut b_rgba = vec![0u8; 220 * 220 * 4];
            for i in 0..(220 * 220) {
                b_rgba[i * 4] = 80;
                b_rgba[i * 4 + 1] = 200;
                b_rgba[i * 4 + 2] = 80;
                b_rgba[i * 4 + 3] = 255;
            }
            let b = spottedcat::Image::new_from_rgba8(
                spottedcat::Pt::from(220.0),
                spottedcat::Pt::from(220.0),
                &b_rgba,
            )
            .expect("failed to create B");

            let mut c_rgba = vec![0u8; 80 * 80 * 4];
            for i in 0..(80 * 80) {
                c_rgba[i * 4] = 255;
                c_rgba[i * 4 + 1] = 80;
                c_rgba[i * 4 + 2] = 80;
                c_rgba[i * 4 + 3] = 255;
            }
            let c = spottedcat::Image::new_from_rgba8(
                spottedcat::Pt::from(80.0),
                spottedcat::Pt::from(80.0),
                &c_rgba,
            )
            .expect("failed to create C");

            Self {
                a,
                b,
                c,
            }
        }

        fn draw(&mut self, context: &mut spottedcat::Context) {
            self.a
                .clear([50.0 / 255.0, 50.0 / 255.0, 200.0 / 255.0, 1.0])
                .expect("failed to clear A");
            self.b
                .clear([80.0 / 255.0, 200.0 / 255.0, 80.0 / 255.0, 1.0])
                .expect("failed to clear B");
            self.c
                .clear([1.0, 80.0 / 255.0, 80.0 / 255.0, 1.0])
                .expect("failed to clear C");

            let mut a_on_screen = spottedcat::DrawOption::new();
            a_on_screen.position = [spottedcat::Pt::from(20.0), spottedcat::Pt::from(20.0)];
            self.a.draw(context, a_on_screen);

            let mut b_in_a = spottedcat::DrawOption::new();
            b_in_a.position = [spottedcat::Pt::from(60.0), spottedcat::Pt::from(60.0)];
            self.a
                .draw_sub(context, spottedcat::DrawAble::Image(self.b), b_in_a)
                .expect("failed to draw B onto A");

            let mut c_in_b = spottedcat::DrawOption::new();
            c_in_b.position = [spottedcat::Pt::from(40.0), spottedcat::Pt::from(40.0)];
            self.b
                .draw_sub(context, spottedcat::DrawAble::Image(self.c), c_in_b)
                .expect("failed to draw C onto B");
        }

        fn update(&mut self, _context: &mut spottedcat::Context, _dt: std::time::Duration) {}

        fn remove(&self) {}
    }

    spottedcat::run::<DrawSubDemo>(spottedcat::WindowConfig::default());
}
