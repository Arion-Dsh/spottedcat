use std::sync::Arc;

#[derive(Clone)]
pub struct Texture(pub(crate) Arc<AnyTexture>);

pub(crate) struct AnyTexture {
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl Drop for AnyTexture {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}

impl AnyTexture {
    pub fn from_rgba8_with_format(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        rgba: &[u8],
        format: wgpu::TextureFormat,
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

        let (data, format) = match format {
            wgpu::TextureFormat::Rgba8UnormSrgb => (rgba.to_vec(), format),
            wgpu::TextureFormat::Bgra8UnormSrgb => {
                let mut bgra = rgba.to_vec();
                for p in bgra.chunks_exact_mut(4) {
                    p.swap(0, 2);
                }
                (bgra, format)
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "unsupported texture format for rgba8 upload: {:?}",
                    format
                ));
            }
        };

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
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data,
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
            format,
            texture,
            view,
        })
    }

    pub fn from_rgba8(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> anyhow::Result<Self> {
        Self::from_rgba8_with_format(
            device,
            queue,
            width,
            height,
            rgba,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        )
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

    pub fn from_rgba8_with_format(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        rgba: &[u8],
        format: wgpu::TextureFormat,
    ) -> anyhow::Result<Self> {
        Ok(Self(Arc::new(AnyTexture::from_rgba8_with_format(
            device, queue, width, height, rgba, format,
        )?)))
    }
}
