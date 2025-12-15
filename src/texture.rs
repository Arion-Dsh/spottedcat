use std::sync::Arc;

#[derive(Clone)]
pub struct Texture(pub(crate) Arc<AnyTexture>);

pub(crate) struct AnyTexture {
    pub width: u32,
    pub height: u32,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl Drop for AnyTexture {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}

impl AnyTexture {
    pub fn from_rgba8(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> anyhow::Result<Self> {
        let expected = width
            .checked_mul(height)
            .and_then(|v| v.checked_mul(4))
            .ok_or_else(|| anyhow::anyhow!("invalid texture size"))? as usize;
        if rgba.len() != expected {
            return Err(anyhow::anyhow!(
                "invalid rgba length: got {}, expected {}",
                rgba.len(),
                expected
            ));
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("any_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Ok(Self {
            width,
            height,
            texture,
            view,
        })
    }
}

impl Texture {
    pub fn from_rgba8(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> anyhow::Result<Self> {
        Ok(Self(Arc::new(AnyTexture::from_rgba8(
            device, queue, width, height, rgba,
        )?)))
    }
}
