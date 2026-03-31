use bytemuck::Pod;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShaderOpts {
    pub(crate) bytes: [u8; crate::image_raw::ImageRenderer::GLOBALS_SIZE_BYTES],
    pub(crate) opacity: f32,
}

impl Default for ShaderOpts {
    fn default() -> Self {
        Self {
            bytes: [0u8; crate::image_raw::ImageRenderer::GLOBALS_SIZE_BYTES],
            opacity: 1.0,
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
        self.opacity = opacity.clamp(0.0, 1.0);
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
                for (i, item) in array.iter_mut().enumerate() {
                    *item = f32::from_le_bytes([
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
            for (i, &v) in value.iter().enumerate() {
                let bytes = v.to_le_bytes();
                self.bytes[start + i * 4..start + i * 4 + 4].copy_from_slice(&bytes);
            }
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}
