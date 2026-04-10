use crate::{Image, Pt};
use std::sync::Arc;

/// Internal structure porting the binary tree packing algorithm from packing.go
#[derive(Default, Clone)]
struct Node {
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    split: bool,
    is_end: bool,
}

impl Node {
    fn insert_node(&mut self, rect_w: i32, rect_h: i32) -> Option<(i32, i32)> {
        if self.split {
            if let Some(left) = self.left.as_mut() {
                if let Some(pos) = left.insert_node(rect_w, rect_h) {
                    return Some(pos);
                }
            }
            if let Some(right) = self.right.as_mut() {
                return right.insert_node(rect_w, rect_h);
            }
            return None;
        }

        if self.is_end || self.w < rect_w || self.h < rect_h {
            return None;
        }

        self.split = true;

        if self.w == rect_w && self.h == rect_h {
            self.is_end = true;
            return Some((self.x, self.y));
        }

        let dw = self.w - rect_w;
        let dh = self.h - rect_h;

        let mut left_node;
        let down_node;
        let right_node;

        if dw > dh {
            left_node = Node {
                x: self.x,
                y: self.y,
                w: self.w,
                h: self.h - rect_h,
                split: true,
                ..Default::default()
            };
            down_node = Node {
                x: self.x,
                y: self.y + rect_h,
                w: self.w - rect_w,
                h: self.h - rect_h,
                ..Default::default()
            };
            right_node = Node {
                x: self.x + rect_w,
                y: self.y,
                w: self.w - rect_w,
                h: self.h,
                ..Default::default()
            };
        } else {
            left_node = Node {
                x: self.x,
                y: self.y,
                w: rect_w,
                h: self.h,
                split: true,
                ..Default::default()
            };
            down_node = Node {
                x: self.x,
                y: self.y + rect_h,
                w: self.w,
                h: self.h - rect_h,
                ..Default::default()
            };
            right_node = Node {
                x: self.x + rect_w,
                y: self.y,
                w: self.w - rect_w,
                h: rect_h,
                ..Default::default()
            };
        }

        let rect_x = right_node.x - rect_w;
        let rect_y = down_node.y - rect_h;

        let mut final_rect = Node {
            x: rect_x,
            y: rect_y,
            w: rect_w,
            h: rect_h,
            is_end: true,
            ..Default::default()
        };

        left_node.left = Some(Box::new(std::mem::take(&mut final_rect)));
        left_node.right = Some(Box::new(down_node));
        self.left = Some(Box::new(left_node));
        self.right = Some(Box::new(right_node));

        Some((rect_x, rect_y))
    }
}

pub(crate) struct Packer {
    root: Node,
}

impl Packer {
    pub fn new(w: i32, h: i32) -> Self {
        Self {
            root: Node {
                x: 0,
                y: 0,
                w,
                h,
                ..Node::default()
            },
        }
    }

    pub fn insert(&mut self, w: i32, h: i32) -> Option<(i32, i32)> {
        self.root.insert_node(w, h)
    }
}

pub(crate) struct AtlasPage {
    pub texture_id: u32,
    pub packer: Packer,
    pub buffer: Vec<u8>,
    pub pixel_width: u32,
    pub pixel_height: u32,
}

/// A dynamic texture atlas that can grow across multiple pages.
pub(crate) struct DynamicAtlas {
    pub pages: Vec<AtlasPage>,
    pub max_dim: u32,
}

impl DynamicAtlas {
    pub fn new(max_dim: u32) -> Self {
        Self {
            pages: Vec::new(),
            max_dim,
        }
    }

    pub fn add_region(
        &mut self,
        registry: &mut crate::context::ResourceRegistry,
        scale_factor: f64,
        logical_w: Pt,
        logical_h: Pt,
        w_px: u32,
        h_px: u32,
        rgba: &[u8],
    ) -> anyhow::Result<Image> {
        let padding = 1;
        let total_w = w_px + padding;
        let total_h = h_px + padding;

        if total_w > self.max_dim || total_h > self.max_dim {
            anyhow::bail!("Region too large for atlas: {}x{}", w_px, h_px);
        }

        // Try current page
        if let Some(page_idx) = self.find_page_for_region(total_w, total_h) {
            let page = &mut self.pages[page_idx];
            if let Some((x, y)) = page.packer.insert(total_w as i32, total_h as i32) {
                return self.write_to_page(registry, scale_factor, page_idx, x as u32, y as u32, logical_w, logical_h, w_px, h_px, rgba);
            }
        }

        // If no page works, create a new one
        let new_w = (total_w.next_power_of_two()).max(256).min(self.max_dim);
        let new_h = (total_h.next_power_of_two()).max(256).min(self.max_dim);

        let page_idx = self.create_page(registry, scale_factor, new_w, new_h);
        let page = &mut self.pages[page_idx];
        if let Some((x, y)) = page.packer.insert(total_w as i32, total_h as i32) {
            self.write_to_page(registry, scale_factor, page_idx, x as u32, y as u32, logical_w, logical_h, w_px, h_px, rgba)
        } else {
            anyhow::bail!("Failed to insert region into new atlas page");
        }
    }

    fn find_page_for_region(&self, w: u32, h: u32) -> Option<usize> {
        for (i, page) in self.pages.iter().enumerate() {
            if page.pixel_width >= w && page.pixel_height >= h {
                return Some(i);
            }
        }
        None
    }

    fn create_page(
        &mut self,
        registry: &mut crate::context::ResourceRegistry,
        scale_factor: f64,
        w_px: u32,
        h_px: u32,
    ) -> usize {
        let buffer = vec![0u8; (w_px * h_px * 4) as usize];
        let scale_factor = scale_factor.max(1.0);
        let logical_w = Pt::from_physical_px(w_px as f64, scale_factor);
        let logical_h = Pt::from_physical_px(h_px as f64, scale_factor);

        // We need to register the texture manually in the registry
        let texture_id = registry.next_texture_id;
        registry.next_texture_id += 1;
        let image_id = registry.next_image_id;
        registry.next_image_id += 1;

        while registry.textures.len() <= texture_id as usize {
            registry.textures.push(None);
        }
        registry.textures[texture_id as usize] = Some(crate::graphics::texture::TextureEntry::new_sampled(
            logical_w,
            logical_h,
            w_px,
            h_px,
            image_id,
            std::sync::Arc::from(buffer.as_slice()),
        ));

        let bounds = crate::image::Bounds::new(Pt(0.0), Pt(0.0), logical_w, logical_h);
        while registry.images.len() <= image_id as usize {
            registry.images.push(None);
        }
        registry.images[image_id as usize] = Some(crate::image::ImageEntry::new(
            texture_id,
            bounds,
            crate::image::PixelBounds {
                x: 0,
                y: 0,
                width: w_px,
                height: h_px,
            },
        ));

        registry.dirty_assets = true;

        let page = AtlasPage {
            texture_id,
            packer: Packer::new(w_px as i32, h_px as i32),
            buffer,
            pixel_width: w_px,
            pixel_height: h_px,
        };
        self.pages.push(page);
        self.pages.len() - 1
    }

    fn write_to_page(
        &mut self,
        registry: &mut crate::context::ResourceRegistry,
        scale_factor: f64,
        page_idx: usize,
        x: u32,
        y: u32,
        logical_w: Pt,
        logical_h: Pt,
        w: u32,
        h: u32,
        rgba: &[u8],
    ) -> anyhow::Result<Image> {
        let page = &mut self.pages[page_idx];
        
        for row in 0..h {
            let src_idx = (row * w * 4) as usize;
            let dst_idx = ((y + row) * page.pixel_width + x) as usize * 4;
            page.buffer[dst_idx..dst_idx + (w * 4) as usize].copy_from_slice(&rgba[src_idx..src_idx + (w * 4) as usize]);
        }

        // Update the texture data in Registry
        if let Some(entry) = registry.textures.get_mut(page.texture_id as usize).and_then(|v| v.as_mut()) {
            entry.raw_data = Some(Arc::from(page.buffer.as_slice()));
            registry.dirty_assets = true;
        }

        let scale_factor = scale_factor.max(1.0);
        let logical_x = Pt::from_physical_px(x as f64, scale_factor);
        let logical_y = Pt::from_physical_px(y as f64, scale_factor);

        // Manual sub-image registration
        let view_id = registry.next_image_id;
        registry.next_image_id += 1;

        let entry = crate::image::ImageEntry::new(
            page.texture_id,
            crate::image::Bounds::new(logical_x, logical_y, logical_w, logical_h),
            crate::image::PixelBounds {
                x,
                y,
                width: w,
                height: h,
            },
        );

        while registry.images.len() <= view_id as usize {
            registry.images.push(None);
        }
        registry.images[view_id as usize] = Some(entry);
        registry.dirty_assets = true;

        Ok(Image {
            id: view_id,
            texture_id: page.texture_id,
            x: logical_x,
            y: logical_y,
            width: logical_w,
            height: logical_h,
            pixel_bounds: crate::image::PixelBounds {
                x,
                y,
                width: w,
                height: h,
            },
        })
    }
}
