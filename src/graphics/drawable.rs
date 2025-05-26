use std::sync::Arc;

use super::{ImageState, Texture, TextureUniformState};

#[derive(Clone)]
pub(crate) struct DrawItem {
    pub(crate) size: [f32; 3],
    pub(crate) state: Arc<ImageState>,
    #[allow(dead_code)]
    pub(crate) texture: Arc<Texture>,
    #[allow(dead_code)]
    pub(crate) texture_uniform_state: Arc<TextureUniformState>,
    pub(crate) options: DrawOptions,
}

#[derive(Clone, Copy, Default)]
pub struct DrawOptions {
    pub(crate) gmo_matrix: GeoMartrix,
    pub(crate) color_matrix: ColorMatrix,
    pub(crate) need_update_color_matrix: bool,
}

#[derive(Clone, Copy)]
pub struct GeoMartrix {
    pub(crate) pos: [f32; 3],
    pub(crate) scale: [f32; 3],
    pub(crate) rotation_angle: f32,
    pub(crate) opacity: f32,
    pub(crate) z_index: f32,
}

impl Default for GeoMartrix {
    fn default() -> Self {
        Self {
            pos: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            rotation_angle: 0.0,
            opacity: 1.0,
            z_index: 0.0,
        }
    }
}

impl DrawOptions {
    pub fn translate(&mut self, x: f32, y: f32, z: f32) {
        self.gmo_matrix.pos[0] += x;
        self.gmo_matrix.pos[1] += y;
        self.gmo_matrix.pos[2] += z;
    }

    pub fn scale(&mut self, x: f32, y: f32, z: f32) {
        self.gmo_matrix.scale[0] *= x;
        self.gmo_matrix.scale[1] *= y;
        self.gmo_matrix.scale[2] *= z;
    }

    pub fn rotate(&mut self, angle: f32) {
        self.gmo_matrix.rotation_angle += angle;
    }

    pub fn opacity(&mut self, opacity: f32) {
        self.gmo_matrix.opacity = opacity;
    }

    pub fn z_index(&mut self, z_index: f32) {
        self.gmo_matrix.z_index = z_index;
    }
}

#[derive(Clone, Copy)]
pub struct ColorMatrix {
    pub matrix: [[f32; 4]; 4],
    pub transform: [f32; 4],
}

impl Default for ColorMatrix {
    fn default() -> Self {
        Self {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            transform: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

impl DrawOptions {
    // 颜色变换相关方法
    pub fn set_color_offset(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.color_matrix.transform = [r, g, b, a];
        self.need_update_color_matrix = true;
    }

    pub fn set_color_matrix(&mut self, matrix: [[f32; 4]; 4]) {
        self.color_matrix.matrix = matrix;
        self.need_update_color_matrix = true;
    }

    pub fn reset_color_matrix(&mut self) {
        self.color_matrix = ColorMatrix::default();
        self.need_update_color_matrix = true;
    }

    pub fn apply_color(&self, color: [f32; 4]) -> [f32; 4] {
        let mut result = [0.0; 4];
        for i in 0..4 {
            result[i] = self.color_matrix.transform[i];
            for j in 0..4 {
                result[i] += self.color_matrix.matrix[i][j] * color[j];
            }
        }
        result
    }

    pub fn set_grayscale(&mut self, amount: f32) {
        let amount = amount / 100.0;
        let r = 0.2126 + 0.7874 * (1.0 - amount);
        let g = 0.7152 - 0.7152 * (1.0 - amount);
        let b = 0.0722 - 0.0722 * (1.0 - amount);
        self.color_matrix.matrix = [
            [r, g, b, 0.0],
            [r, g, b, 0.0],
            [r, g, b, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        self.need_update_color_matrix = true;
    }

    pub fn set_brightness(&mut self, amount: f32) {
        let amount = amount / 100.0;
        self.color_matrix.transform = [amount, amount, amount, 0.0];
        self.need_update_color_matrix = true;
    }

    pub fn set_contrast(&mut self, amount: f32) {
        let amount = amount / 100.0;
        let factor = (amount + 1.0) / (1.01 - amount);
        self.color_matrix.matrix = [
            [factor, 0.0, 0.0, 0.0],
            [0.0, factor, 0.0, 0.0],
            [0.0, 0.0, factor, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        self.color_matrix.transform = [factor * -0.5, factor * -0.5, factor * -0.5, 0.0];
        self.need_update_color_matrix = true;
    }
}
