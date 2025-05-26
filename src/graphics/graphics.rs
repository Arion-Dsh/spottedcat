use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue};

pub(crate) use super::texture::Texture;
use super::{ColorUniform, DrawItem, ImageBaseUniform};

pub struct Graphics {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    depth_stencil: Texture,
    msaa_texture_view: wgpu::TextureView,
    screen_size: [f32; 2],
}

impl Graphics {
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        config: &wgpu::SurfaceConfiguration,
    ) -> Graphics {
        // Create vertex buffer

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Create index buffer
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let depth_stencil = Texture::create_depth_texture(&device, config, "DepthStencil");
        let msaa_texture_view = Texture::msaa_texture_view(&device, config);

        Graphics {
            device,
            queue,
            vertex_buffer,
            index_buffer,
            depth_stencil,
            msaa_texture_view,
            screen_size: [config.width as f32, config.height as f32],
        }
    }

    pub fn draw(&self, surface: &wgpu::Surface, imgs: Vec<DrawItem>) {
        let output = surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        // Add rendering code here
        {

            let color_attachment = wgpu::RenderPassColorAttachment {
                view: &self.msaa_texture_view,
                resolve_target: Some(&view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            };
            let depth_attachment = wgpu::RenderPassDepthStencilAttachment {
                view: self.depth_stencil.view.as_ref(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            };

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(color_attachment)],
                depth_stencil_attachment: Some(depth_attachment),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for item in imgs {
               

                let options = item.options;
                //TODO: 判断是否需要更新uniform
                let mvp_uniform = ImageBaseUniform::new(
                    self.screen_size,
                    [options.gmo_matrix.pos[0], options.gmo_matrix.pos[1]],
                    [item.texture.width as f32, item.texture.height as f32],
                    [options.gmo_matrix.scale[0], options.gmo_matrix.scale[1]],
                    options.gmo_matrix.rotation_angle,
                    options.gmo_matrix.opacity,
                    options.gmo_matrix.z_index,
                );
                let color_uniform = ColorUniform::default();

                self.queue.write_buffer(
                    &item.state.uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[mvp_uniform]),
                );
                self.queue.write_buffer(
                    &item.state.color_uniform_buffer,
                    0,
                    bytemuck::cast_slice(&[color_uniform]),
                );

                let texture_uniform_group = item.state.texture_bind_group(
                    &self.device,
                    &item.texture.view,
                    &item.texture.sampler,
                );

                render_pass.set_pipeline(&item.state.pipeline);
                render_pass.set_bind_group(0, &*item.state.uniform_bind_group, &[]);
                render_pass.set_bind_group(1, &texture_uniform_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..QUAD_INDICES.len() as u32, 0, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    pub(crate) fn resize(&mut self, config: &wgpu::SurfaceConfiguration) {
        self.depth_stencil = Texture::create_depth_texture(&self.device, config, "DepthStencil");
        self.msaa_texture_view = Texture::msaa_texture_view(&self.device, config);
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub(crate) struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

// Define quad vertices and indices
const QUAD_VERTICES: [Vertex; 4] = [
    Vertex {
        position: [-1.0, 1.0],
        tex_coords: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0],
        tex_coords: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, -1.0],
        tex_coords: [1.0, 0.0],
    },
    Vertex {
        position: [-1.0, -1.0],
        tex_coords: [0.0, 0.0],
    },
];

const QUAD_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}
