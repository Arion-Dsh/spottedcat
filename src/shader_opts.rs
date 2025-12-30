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

    pub fn from_bytes(src: &[u8]) -> Self {
        let mut out = Self::default();
        let n = src.len().min(out.bytes.len());
        out.bytes[..n].copy_from_slice(&src[..n]);
        out
    }

    pub fn from_pod<T: Pod>(value: &T) -> Self {
        Self::from_bytes(bytemuck::bytes_of(value))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}
