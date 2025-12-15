# 字体使用说明 (Font Usage Guide)

## 概述

Spot 支持自定义字体渲染。你可以使用默认字体，也可以为每个文本指定自定义字体。

## 基本用法

### 使用默认字体

```rust
use spot::{Text, TextOptions, load_font_from_bytes};

const FONT: &[u8] = include_bytes!("assets/DejaVuSans.ttf");

let mut opts = TextOptions::new(load_font_from_bytes(FONT));
opts.position = [100.0, 100.0];
opts.font_size = 32.0;
opts.color = [1.0, 1.0, 1.0, 1.0]; // RGBA (白色)
Text::new("Hello, World!").draw(context, opts);
```

### 从文件加载字体

```rust
use spot::{Text, TextOptions, load_font_from_file};

// 方法 1: 使用 with_font_from_file
let opts = TextOptions::from_file("path/to/font.ttf")
    .expect("Failed to load font");
Text::new("自定义字体文本").draw(context, opts);

// 方法 2: 使用 load_font_from_file 函数
let font_data = load_font_from_file("path/to/font.ttf").expect("Failed to load font");
let opts = TextOptions::new(font_data);
Text::new("自定义字体文本").draw(context, opts);
```

### 从字节数组加载字体

```rust
use spot::{Text, TextOptions, load_font_from_bytes};

// 嵌入字体文件
const CUSTOM_FONT: &[u8] = include_bytes!("../assets/CustomFont.ttf");

let font_data = load_font_from_bytes(CUSTOM_FONT);
let opts = TextOptions::new(font_data);
Text::new("嵌入字体文本").draw(context, opts);
```

## TextOptions 字段说明

- `position: [f32; 2]` - 文本位置 (x, y)，默认 [100.0, 100.0]
- `font_size: f32` - 字体大小，默认 24.0
- `color: [f32; 4]` - 颜色 RGBA，范围 0.0-1.0，默认白色 [1.0, 1.0, 1.0, 1.0]
- `scale: [f32; 2]` - 缩放比例 (x, y)，默认 [1.0, 1.0]
- `font_data: Vec<u8>` - 字体数据（必填）

## 完整示例

```rust
use spot::{Context, Spot, Text, TextOptions, load_font_from_file};

struct MyApp {
    custom_font: Option<Vec<u8>>,
}

impl Spot for MyApp {
    fn initialize(_context: Context) -> Self {
        let custom_font = load_font_from_file("assets/MyFont.ttf").ok();
        Self { custom_font }
    }

    fn draw(&mut self, context: &mut Context) {
        // 字体必须提供
        let mut default_opts = TextOptions::from_file("assets/MyFont.ttf")
            .expect("Failed to load font");
        default_opts.position = [50.0, 50.0];
        default_opts.font_size = 24.0;
        Text::new("默认字体").draw(context, default_opts);

        // 自定义字体
        if let Some(font) = &self.custom_font {
            let mut custom_opts = TextOptions::new(font.clone());
            custom_opts.position = [50.0, 100.0];
            custom_opts.font_size = 32.0;
            custom_opts.color = [1.0, 0.5, 0.0, 1.0]; // 橙色
            Text::new("自定义字体").draw(context, custom_opts);
        }
    }

    fn update(&self, _event: spot::Event) {}
    fn remove(&self) {}
}

fn main() {
    spot::run::<MyApp>();
}
```

## 注意事项

1. **字体缓存**: 相同的字体数据会被自动缓存，避免重复加载
2. **性能**: 建议在 `initialize` 中加载字体，而不是每帧加载
3. **字体格式**: 支持 TrueType (.ttf) 字体格式

## 获取字体

你可以从以下来源获取免费字体：
- [Google Fonts](https://fonts.google.com/)
- [DejaVu Fonts](https://dejavu-fonts.github.io/)
- [Font Squirrel](https://www.fontsquirrel.com/)

下载 DejaVuSans.ttf 作为示例字体：
```bash
curl -L -o assets/DejaVuSans.ttf "https://raw.githubusercontent.com/prawnpdf/prawn/master/data/fonts/DejaVuSans.ttf"
```
