use bytemuck::Pod;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShaderOpts {
    bytes: [u8; crate::image_raw::ImageRenderer::GLOBALS_SIZE_BYTES],
}

impl Default for ShaderOpts {
    fn default() -> Self {
        Self {
            bytes: [0u8; crate::image_raw::ImageRenderer::GLOBALS_SIZE_BYTES],
        }
    }
}

impl ShaderOpts {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.set_opacity(opacity);
        self
    }

    pub fn set_opacity(&mut self, opacity: f32) {
        let opacity = opacity.clamp(0.0, 1.0);
        let bytes = opacity.to_le_bytes();
        let end = self.bytes.len();
        self.bytes[end - 4..end].copy_from_slice(&bytes);
    }

    pub fn from_bytes(src: &[u8]) -> Self {
        let mut out = Self::default();
        let n = src.len().min(out.bytes.len());
        out.bytes[..n].copy_from_slice(&src[..n]);
        out
    }

    pub fn from_pod<T: Pod>(value: &T) -> Self {
        Self::from_bytes(bytemuck::bytes_of(value))
    }

    pub fn as_vec4_mut(&mut self) -> Vec<[f32; 4]> {
        self.bytes
            .chunks_exact(16)
            .map(|chunk| {
                let mut array = [0.0f32; 4];
                for i in 0..4 {
                    array[i] = f32::from_le_bytes([
                        chunk[i * 4],
                        chunk[i * 4 + 1],
                        chunk[i * 4 + 2],
                        chunk[i * 4 + 3],
                    ]);
                }
                array
            })
            .collect()
    }

    pub fn set_vec4(&mut self, index: usize, value: [f32; 4]) {
        if index < 16 {
            let start = index * 16;
            for i in 0..4 {
                let bytes = value[i].to_le_bytes();
                self.bytes[start + i*4..start + i*4 + 4].copy_from_slice(&bytes);
            }
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}
