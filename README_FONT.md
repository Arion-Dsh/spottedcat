# 字体功能 (Font Feature)

## 快速开始

### 1. 下载默认字体

在项目根目录运行：

```bash
curl -L -o assets/DejaVuSans.ttf "https://raw.githubusercontent.com/prawnpdf/prawn/master/data/fonts/DejaVuSans.ttf"
```

### 2. 使用字体 API

Spot 提供了三种方式来加载和使用字体：

#### 方法 1: 传入字体（必须）

```rust
use spot::{Text, TextOptions, load_font_from_bytes};

const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");

let opts = TextOptions::new(load_font_from_bytes(FONT));
Text::new("Hello, World!").draw(context, opts);
```

#### 方法 2: 从文件加载字体

```rust
use spot::TextOptions;

let opts = TextOptions::from_file("path/to/font.ttf")
    .expect("Failed to load font");
Text::new("Custom Font Text").draw(context, opts);
```

#### 方法 3: 从字节数组加载字体

```rust
use spot::{TextOptions, load_font_from_bytes};

const MY_FONT: &[u8] = include_bytes!("../assets/MyFont.ttf");
let font_data = load_font_from_bytes(MY_FONT);

let opts = TextOptions::new(font_data);
Text::new("Embedded Font Text").draw(context, opts);
```

## API 说明

### `TextOptions` 结构体

```rust
pub struct TextOptions {
    pub position: [f32; 2],      // 文本位置
    pub font_size: f32,           // 字体大小
    pub color: [f32; 4],          // RGBA 颜色
    pub scale: [f32; 2],          // 缩放比例
    pub font_data: Vec<u8>, // 字体数据
}
```

### Text 类型

- `Text::new(content)` - 创建文本实例
- `.draw(context, options)` - 绘制文本到 context

### TextOptions 方法

- `TextOptions::new(font_data)` - 创建配置（必须提供字体数据）
- `TextOptions::from_file(path)` - 从文件加载字体并创建配置
- `.with_font_from_file(path: &str)` - 从文件加载字体
- `.with_font_from_bytes(data: Vec<u8>)` - 从字节数组加载字体

### 全局函数

- `load_font_from_file(path: &str) -> Result<Vec<u8>>` - 加载字体文件
- `load_font_from_bytes(bytes: &[u8]) -> Vec<u8>` - 转换字节数组

## 完整示例

参见 `examples/text_example.rs`

## 注意事项

1. 字体数据会被自动缓存，相同的字体不会重复加载
2. 建议在 `initialize` 方法中加载字体，而不是在 `draw` 方法中
3. **必须提供** `font_data`，否则 `Text::draw` 会 panic
