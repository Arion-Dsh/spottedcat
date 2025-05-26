use std::borrow::Cow;
use std::cmp;
use std::{fmt::Debug, sync::Arc};
use anyhow::*;
use image::GenericImageView;

#[allow(dead_code)]
#[derive(Clone)]
pub(crate) struct Texture {
    pub width: u32,
    pub height: u32,
    pub texture: Arc<wgpu::Texture>,
    pub view: Arc<wgpu::TextureView>,
    pub sampler: Arc<wgpu::Sampler>,
}

impl Texture {

    pub(crate) fn msaa_texture_view(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> wgpu::TextureView {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let msaa_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("MSAA Render Target"),
            size,
            mip_level_count: 1, // MSAA 纹理通常没有 mipmaps
            sample_count: 4, // <--- 关键：设置多重采样
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT, // 必须可作为渲染附件
            view_formats: &[],
        });
        let msaa_texture_view = msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());
        msaa_texture_view
    }
    pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub(crate) fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
       
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING| wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            compare: None,
            lod_min_clamp: 0.0,
            lod_max_clamp: 200.0,
            ..Default::default()
        });
     

        Self {
            width: size.width,
            height: size.height,
            texture: Arc::new(texture),
            view: Arc::new(view),
            sampler: Arc::new(sampler),
        }
    }

    pub(crate)  fn  from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> Result<Self> {
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        //let mip_level_count = Self::mip_level_count(dimensions.0, dimensions.1);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count:1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });


        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 1 as f32,
            compare: None,
            anisotropy_clamp: 1,
            ..Default::default()
        });
        
        // Self::generate_mipmaps(device, queue, &texture, format, mip_level_count);

        Ok(Self {
            width: dimensions.0,
            height: dimensions.1,
            texture: Arc::new(texture),
            view: Arc::new(view),
            sampler: Arc::new(sampler),
        })
    }
    #[allow(dead_code)]
    fn mip_level_count(width: u32, height: u32) -> u32 {
        let max_dim = cmp::max(width, height) as f32; // 将最大维度转换为 f32
        if max_dim == 0.0 {
            // 避免 log2(0) 导致的问题，虽然实际纹理尺寸不会是0
            return 1;
        }
        
        // 计算 log2，然后取整，最后加 1
        // log2(1) = 0, +1 = 1 (对于 1x1 纹理，只有级别 0)
        // log2(2) = 1, +1 = 2 (对于 2x2 纹理，有级别 0, 1)
        // log2(3) approx 1.58, floor = 1, +1 = 2 (对于 3x3 纹理，有级别 0, 1)
        // log2(4) = 2, +1 = 3 (对于 4x4 纹理，有级别 0, 1, 2)
        (max_dim.log2().floor() as u32) + 1
    }
    #[allow(dead_code)]
    pub fn generate_mipmaps(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        format: wgpu::TextureFormat,
        mip_level_count: u32,
    ) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("mip.wgsl"))),
        });
        
    
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mipmap_generator_pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })
                ],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let bind_group_layout = pipeline.get_bind_group_layout(0);
    
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mip"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
    
      
        let views = (0..mip_level_count)
            .map(|mip| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("mip"),
                    format: None,
                    dimension: None,
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: mip,
                    mip_level_count: Some(1),
                    base_array_layer: 0,
                    array_layer_count: None,
                })
            })
            .collect::<Vec<_>>();
        let query_sets = if device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::PIPELINE_STATISTICS_QUERY)
        {
            let mip_passes = mip_level_count - 1;

            let timestamp = device.create_query_set(&wgpu::QuerySetDescriptor {
                count: mip_passes * 2,
                ty: wgpu::QueryType::Timestamp,
                label: Some("timestamp"),
            });
            let timestamp_period = queue.get_timestamp_period();

            let pipeline_statistics = device.create_query_set(&wgpu::QuerySetDescriptor {
                count: mip_passes,
                ty: wgpu::QueryType::PipelineStatistics(
                    wgpu::PipelineStatisticsTypes::FRAGMENT_SHADER_INVOCATIONS,
                ),
                label: Some("pipeline_statistics"),
            });

            let data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("query buffer"),
                size: mip_passes as wgpu::BufferAddress
                    * 3
                    * std::mem::size_of::<u64>() as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            Some(QuerySets {
                timestamp,
                timestamp_period,
                pipeline_statistics,
                data_buffer,
            })
        } else {
            None
        };

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("mipmap_generator_encoder"),
        });

        for target_mip in 1..mip_level_count as usize {
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&views[target_mip - 1]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
                label: None,
            });

            let pipeline_query_index_base = target_mip as u32 - 1;
            let timestamp_query_index_base = (target_mip as u32 - 1) * 2;

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &views[target_mip],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if let Some(ref query_sets) = query_sets {
                rpass.write_timestamp(&query_sets.timestamp, timestamp_query_index_base);
                rpass.begin_pipeline_statistics_query(
                    &query_sets.pipeline_statistics,
                    pipeline_query_index_base,
                );
            }
            rpass.set_pipeline(&pipeline);
            rpass.set_bind_group(0, &bind_group, &[]);
            rpass.draw(0..4, 0..1);
            if let Some(ref query_sets) = query_sets {
                rpass.write_timestamp(&query_sets.timestamp, timestamp_query_index_base + 1);
                rpass.end_pipeline_statistics_query();
            }
        }
        let mip_passes = mip_level_count - 1;
        if let Some(ref query_sets) = query_sets {
            let timestamp_query_count = mip_passes * 2;
            encoder.resolve_query_set(
                &query_sets.timestamp,
                0..timestamp_query_count,
                &query_sets.data_buffer,
                0,
            );
            encoder.resolve_query_set(
                &query_sets.pipeline_statistics,
                0..mip_passes,
                &query_sets.data_buffer,
                (timestamp_query_count * std::mem::size_of::<u64>() as u32) as wgpu::BufferAddress,
            );
        }
    
        queue.submit(Some(encoder.finish()));
    }
    

}

impl  Debug for Texture{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texture").field("texture", &self.texture).field("view", &self.view).field("sampler", &self.sampler).finish()
    }
}

struct QuerySets {
    timestamp: wgpu::QuerySet,
    #[allow(dead_code)]
    timestamp_period: f32,
    pipeline_statistics: wgpu::QuerySet,
    data_buffer: wgpu::Buffer,
}