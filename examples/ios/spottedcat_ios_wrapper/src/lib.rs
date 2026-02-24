use spottedcat::{Context, DrawOption, Image, Pt, Spot, WindowConfig, Text};

#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn spottedcat_ios_start() {
    struct IosFfiSpot {
        grandpa: Image,
        father: Image,
        son: Image,
        text: Text,
    }

    impl Spot for IosFfiSpot {
        fn initialize(_: &mut Context) -> Self {
            let grandpa_rgba = vec![100, 100, 100, 255].repeat(300 * 300);
            let grandpa =
                Image::new_from_rgba8(Pt::from(300.0), Pt::from(300.0), &grandpa_rgba).unwrap();

            let father_rgba = vec![0, 0, 255, 255].repeat(200 * 200);
            let father =
                Image::new_from_rgba8(Pt::from(200.0), Pt::from(200.0), &father_rgba).unwrap();

            let son_rgba = vec![255, 0, 0, 255].repeat(100 * 100);
            let son = Image::new_from_rgba8(Pt::from(100.0), Pt::from(100.0), &son_rgba).unwrap();

            const FALLBACK_FONT: &[u8] = include_bytes!("../../../../assets/DejaVuSans.ttf");
            let font_id = spottedcat::register_font(FALLBACK_FONT.to_vec());
            let text = Text::new("Clipped Text", font_id)
                .with_font_size(Pt::from(24.0))
                .with_color([1.0, 1.0, 1.0, 1.0]);

            Self {
                grandpa,
                father,
                son,
                text,
            }
        }

        fn draw(&mut self, context: &mut Context) {
            let grandpa_opts = DrawOption::default()
                .with_position([Pt::from(50.0), Pt::from(50.0)]);
            self.grandpa.draw(context, grandpa_opts);

            let father_opts = DrawOption::default()
                .with_position([Pt::from(150.0), Pt::from(150.0)]);

            self.grandpa.with_clip_scope(context, grandpa_opts, |context| {
                self.father.draw(context, father_opts);

                let son_opts = DrawOption::default()
                    .with_position([Pt::from(100.0), Pt::from(100.0)]);

                self.father.with_clip_scope(context, father_opts, |context| {
                    self.son.draw(context, son_opts);

                    let text_opts = DrawOption::default()
                        .with_position([Pt::from(50.0), Pt::from(50.0)]);
                    self.text.clone().draw(context, text_opts);
                });
            });
        }

        fn update(&mut self, _: &mut Context, _dt: std::time::Duration) {}

        fn remove(&self) {}
    }

    spottedcat::run::<IosFfiSpot>(WindowConfig::default());
}
