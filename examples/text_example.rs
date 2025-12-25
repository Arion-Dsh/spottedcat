use spottedcat::{Context, Spot, Text, DrawOption, load_font_from_bytes};

struct TextApp {}

impl Spot for TextApp {
    fn initialize(_context: &mut Context) -> Self {
        Self {}
    }

    fn draw(&mut self, context: &mut Context) {
        // 示例：优先尝试 macOS 自带中文字体（多数为 .ttc，可能不被当前字体解析器支持）
        // 如果加载失败则回退到仓库内的 DejaVuSans.ttf（仅用于演示，中文会变方块）。
        const FALLBACK_FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");

        let font_opts = [
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/STHeiti Medium.ttc",
            "/System/Library/Fonts/Supplemental/Songti.ttc",
        ];

        let mut font_data: Option<Vec<u8>> = None;
        let mut chosen_path: Option<&str> = None;
        for p in font_opts {
            if let Ok(data) = spottedcat::load_font_from_file(p) {
                font_data = Some(data);
                chosen_path = Some(p);
                break;
            }
        }

        let font_data = font_data.unwrap_or_else(|| load_font_from_bytes(FALLBACK_FONT));

        // 使用 Text::draw() - 传入字体数据
        let mut opts = DrawOption::new();
        opts.position = [spottedcat::Pt::from(100.0), spottedcat::Pt::from(100.0)];
        Text::new("Provided Font", font_data.clone())
            .with_font_size(spottedcat::Pt::from(32.0))
            .with_color([1.0, 1.0, 1.0, 1.0])
            .draw(context, opts);
        
        let mut custom_opts = DrawOption::new();
        custom_opts.position = [spottedcat::Pt::from(100.0), spottedcat::Pt::from(150.0)];
        Text::new("使用嵌入字体 - Embedded Font", font_data.clone())
            .with_font_size(spottedcat::Pt::from(28.0))
            .with_color([1.0, 0.5, 0.0, 1.0])
            .with_stroke_width(spottedcat::Pt::from(2.0))
            .with_stroke_color([1.0, 1.0, 1.0, 1.0])
            .draw(context, custom_opts);

        // 从文件加载也可以（同样会写入 font_data）
        let file_font = match chosen_path {
            Some(p) => spottedcat::load_font_from_file(p).unwrap_or_else(|_| font_data.clone()),
            None => spottedcat::load_font_from_file("assets/DejaVuSans.ttf")
                .unwrap_or_else(|_| font_data.clone()),
        };
        let mut file_opts = DrawOption::new();
        file_opts.position = [spottedcat::Pt::from(100.0), spottedcat::Pt::from(200.0)];
        Text::new("从文件加载字体 - Loaded from File", file_font)
            .with_font_size(spottedcat::Pt::from(24.0))
            .with_color([0.0, 1.0, 0.5, 1.0])
            .draw(context, file_opts);

        // 使用 Text::draw() - 不同颜色和大小
        let mut small_opts = DrawOption::new();
        small_opts.position = [spottedcat::Pt::from(100.0), spottedcat::Pt::from(250.0)];
        Text::new("小字体 - Small Font Size", font_data.clone())
            .with_font_size(spottedcat::Pt::from(18.0))
            .with_color([0.5, 0.5, 1.0, 1.0])
            .draw(context, small_opts);

        let mut large_opts = DrawOption::new();
        large_opts.position = [spottedcat::Pt::from(100.0), spottedcat::Pt::from(300.0)];
        Text::new("大字体 - Large Font", font_data)
            .with_font_size(spottedcat::Pt::from(48.0))
            .with_color([1.0, 0.0, 0.5, 1.0])
            .draw(context, large_opts);
    }

    fn update(&mut self, _context: &mut Context, _dt: std::time::Duration) {}
    fn remove(&self) {}
}

fn main() {
    spottedcat::run::<TextApp>(spottedcat::WindowConfig::default());
}
