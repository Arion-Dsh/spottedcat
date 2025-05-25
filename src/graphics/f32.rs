// 如果是手动实现：
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
#[repr(C)] // 确保内存布局与 C 兼容，与 WGSL 匹配
pub struct Vec2 { pub x: f32, pub y: f32 }
impl Vec2 { pub fn new(x: f32, y: f32) -> Self { Self { x, y } } }

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
#[repr(C)]
pub struct Vec4 { pub x: f32, pub y: f32, pub z: f32, pub w: f32 }
impl Vec4 { pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self { Self { x, y, z, w } } }

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
#[repr(C)]
pub struct Mat4 { pub m: [[f32; 4]; 4] } // 4x4 矩阵，假定为列主序 (column-major)
impl Mat4 {
    // 单位矩阵
    pub fn identity() -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    // 矩阵乘法 (A * B) - 假设是列主序
    pub fn mul(&self, other: &Self) -> Self {
        let mut result = Mat4::identity();
        for i in 0..4 { // Column of result
            for j in 0..4 { // Row of result
                result.m[i][j] = self.m[0][j] * other.m[i][0]
                               + self.m[1][j] * other.m[i][1]
                               + self.m[2][j] * other.m[i][2]
                               + self.m[3][j] * other.m[i][3];
            }
        }
        result
    }

    // 矩阵-向量乘法 (Mat4 * Vec4) - 假设 Mat4 是列主序
    pub fn mul_vec4(&self, vec: Vec4) -> Vec4 {
        Vec4 {
            x: self.m[0][0] * vec.x + self.m[1][0] * vec.y + self.m[2][0] * vec.z + self.m[3][0] * vec.w,
            y: self.m[0][1] * vec.x + self.m[1][1] * vec.y + self.m[2][1] * vec.z + self.m[3][1] * vec.w,
            z: self.m[0][2] * vec.x + self.m[1][2] * vec.y + self.m[2][2] * vec.z + self.m[3][2] * vec.w,
            w: self.m[0][3] * vec.x + self.m[1][3] * vec.y + self.m[2][3] * vec.z + self.m[3][3] * vec.w,
        }
    }

    // 缩放矩阵
    pub fn from_scale(x: f32, y: f32, z: f32) -> Self {
        let mut m = Self::identity();
        m.m[0][0] = x;
        m.m[1][1] = y;
        m.m[2][2] = z;
        m
    }

    // Z 轴旋转矩阵 (2D)
    pub fn from_rotation_z(angle_radians: f32) -> Self {
        let cos_a = angle_radians.cos();
        let sin_a = angle_radians.sin();
        let mut m = Self::identity();
        m.m[0][0] = cos_a;  m.m[1][0] = -sin_a;
        m.m[0][1] = sin_a;  m.m[1][1] = cos_a;
        m
    }

    // 平移矩阵
    pub fn from_translation(x: f32, y: f32, z: f32) -> Self {
        let mut m = Self::identity();
        m.m[3][0] = x;
        m.m[3][1] = y;
        m.m[3][2] = z;
        m
    }
}

