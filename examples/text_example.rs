use spottedcat::{Context, Spot, Text, TextOptions, load_font_from_bytes};

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
            if let Ok(opts) = TextOptions::from_file(p) {
                font_data = Some(opts.font_data);
                chosen_path = Some(p);
                break;
            }
        }

        let font_data = font_data.unwrap_or_else(|| load_font_from_bytes(FALLBACK_FONT));

        // 使用 Text::draw() - 传入字体数据
        let mut opts = TextOptions::new(font_data.clone());
        opts.position = [spottedcat::Pt(100), spottedcat::Pt(100)];
        opts.font_size = spottedcat::Pt(32);
        opts.color = [1.0, 1.0, 1.0, 1.0];
        Text::new("Provided Font").draw(context, opts);
        
        let mut custom_opts = TextOptions::new(font_data.clone());
        custom_opts.position = [spottedcat::Pt(100), spottedcat::Pt(150)];
        custom_opts.font_size = spottedcat::Pt(28);
        custom_opts.color = [1.0, 0.5, 0.0, 1.0]; // 橙色
        custom_opts.stroke_width = spottedcat::Pt(2);
        custom_opts.stroke_color = [1.0, 1.0, 1.0, 1.0];
        Text::new("使用嵌入字体 - Embedded Font").draw(context, custom_opts);

        // 从文件加载也可以（同样会写入 font_data）
        let mut file_opts = match chosen_path {
            Some(p) => TextOptions::from_file(p).unwrap_or_else(|_| TextOptions::new(font_data.clone())),
            None => TextOptions::from_file("assets/DejaVuSans.ttf")
                .unwrap_or_else(|_| TextOptions::new(font_data.clone())),
        };
        file_opts.position = [spottedcat::Pt(100), spottedcat::Pt(200)];
        file_opts.font_size = spottedcat::Pt(24);
        file_opts.color = [0.0, 1.0, 0.5, 1.0]; // 青绿色
        Text::new("从文件加载字体 - Loaded from File").draw(context, file_opts);

        // 使用 Text::draw() - 不同颜色和大小
        let mut small_opts = TextOptions::new(font_data.clone());
        small_opts.position = [spottedcat::Pt(100), spottedcat::Pt(250)];
        small_opts.font_size = spottedcat::Pt(18);
        small_opts.color = [0.5, 0.5, 1.0, 1.0]; // 淡蓝色
        Text::new("小字体 - Small Font Size").draw(context, small_opts);

        let mut large_opts = TextOptions::new(font_data);
        large_opts.position = [spottedcat::Pt(100), spottedcat::Pt(300)];
        large_opts.font_size = spottedcat::Pt(48);
        large_opts.color = [1.0, 0.0, 0.5, 1.0]; // 粉红色
        Text::new("大字体 - Large Font").draw(context, large_opts);
    }

    fn update(&mut self, _context: &mut Context, _dt: std::time::Duration) {}
    fn remove(&self) {}
}

fn main() {
    spottedcat::run::<TextApp>(spottedcat::WindowConfig::default());
}
