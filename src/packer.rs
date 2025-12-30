/// 矩形区域定义
#[derive(Debug, Clone, Copy)]
pub struct PackerRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// 二叉树节点
struct Node {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
    filled: bool,
}

impl Node {
    fn new(x: u32, y: u32, w: u32, h: u32) -> Self {
        Node {
            x, y, w, h,
            left: None,
            right: None,
            filled: false,
        }
    }

    fn insert(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        if let Some(ref mut left) = self.left {
            if let Some(pos) = left.insert(w, h) {
                return Some(pos);
            }
            return self.right.as_mut().unwrap().insert(w, h);
        }

        if self.filled || w > self.w || h > self.h {
            return None;
        }

        if w == self.w && h == self.h {
            self.filled = true;
            return Some((self.x, self.y));
        }

        let dw = self.w - w;
        let dh = self.h - h;

        if dw > dh {
            self.left = Some(Box::new(Node::new(self.x, self.y, w, self.h)));
            self.right = Some(Box::new(Node::new(self.x + w, self.y, dw, self.h)));
        } else {
            self.left = Some(Box::new(Node::new(self.x, self.y, self.w, h)));
            self.right = Some(Box::new(Node::new(self.x, self.y + h, self.w, dh)));
        }

        self.left.as_mut().unwrap().insert(w, h)
    }
}

/// 图集打包器：支持 2 像素 Padding 和边缘拉伸适配
pub struct AtlasPacker {
    width: u32,
    height: u32,
    padding: u32,
    root: Node,
}

impl AtlasPacker {
    /// 创建一个新的打包器，通常大小为 2048x2048 或 4096x4096
    pub fn new(width: u32, height: u32, padding: u32) -> Self {
        Self {
            width,
            height,
            padding,
            root: Node::new(0, 0, width, height),
        }
    }

    /// 核心插入：传入图片的原始宽高
    /// 返回的 Rect 是物理分配的矩形（包含了四周的 Padding 区域）
    pub fn insert_raw(&mut self, sprite_w: u32, sprite_h: u32) -> Option<PackerRect> {
        let needed_w = sprite_w + self.padding * 2;
        let needed_h = sprite_h + self.padding * 2;

        self.root.insert(needed_w, needed_h).map(|(x, y)| PackerRect {
            x,
            y,
            w: needed_w,
            h: needed_h,
        })
    }

    /// 获取 queue.write_texture 所需的物理参数
    /// 直接返回 (x, y, 宽度, 高度)
    pub fn get_write_info(&self, rect: &PackerRect) -> (u32, u32, u32, u32) {
        (rect.x, rect.y, rect.w, rect.h)
    }

    pub fn get_uv_param(&self, rect: &PackerRect) -> [f32; 4] {
        let fw = self.width as f32;
        let fh = self.height as f32;
        let content_w = rect.w - self.padding * 2;
        let content_h = rect.h - self.padding * 2;

        [
            (rect.x + self.padding) as f32 / fw,
            (rect.y + self.padding) as f32 / fh,
            content_w as f32 / fw,
            content_h as f32 / fh,
        ]
    }

    /// 工具方法：在 CPU 端对图片进行边缘拉伸填充（解决 Bleeding）
    pub fn extrude_rgba8(
        &self, 
        raw_rgba: &[u8], 
        w: u32, 
        h: u32
    ) -> Vec<u8> {
        let p = self.padding as i32;
        let new_w = w + self.padding * 2;
        let new_h = h + self.padding * 2;
        let mut out = vec![0u8; (new_w * new_h * 4) as usize];

        for y in 0..new_h {
            for x in 0..new_w {
                // 关键点：通过 clamp 实现边缘像素向外拉伸
                let src_x = (x as i32 - p).clamp(0, w as i32 - 1) as u32;
                let src_y = (y as i32 - p).clamp(0, h as i32 - 1) as u32;

                let src_idx = ((src_y * w + src_x) * 4) as usize;
                let out_idx = ((y * new_w + x) * 4) as usize;

                out[out_idx..out_idx + 4].copy_from_slice(&raw_rgba[src_idx..src_idx + 4]);
            }
        }
        out
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}