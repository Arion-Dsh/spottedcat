use std::borrow::Borrow;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, SurfaceError};
use bytemuck::{Pod, Zeroable};


use super::{ColorUniform, ImageState, ScreenAndPQRSUniform, TextureUniform};
pub(crate) use super::texture::Texture;

pub struct Graphics {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    depth_stencil: Texture,
}

impl Graphics {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, config: &wgpu::SurfaceConfiguration) -> Graphics {
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

        Graphics {
            device,
            queue,
            vertex_buffer,
            index_buffer,
            depth_stencil,
        }
    }


    pub fn draw(&self, surface: &wgpu::Surface, imgs:Vec<(Arc<ImageState>, Arc<Texture>, bool)>){
        let output = surface.get_current_texture().unwrap();
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        // Add rendering code here
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment:Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_stencil.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            
            for (state, texture, is_sub) in imgs {

                let texture_bind_group = state.texture_bind_group(&self.device, &texture.view, &texture.sampler);
                //TODO: 判断是否需要更新uniform
                let mvp_uniform = ScreenAndPQRSUniform::new_a();
                let uvp_uniform = TextureUniform::new_a();
                let color_uniform = ColorUniform::default();
                // let (mvp, uvp)  = calculate_matrices(&mvp_uniform, &uvp_uniform);

                self.queue.write_buffer(&state.uniform_buffer, 0, bytemuck::cast_slice(&[mvp_uniform]));    
                self.queue.write_buffer(&state.texture_uniform_buffer, 0, bytemuck::cast_slice(&[uvp_uniform]));
                self.queue.write_buffer(&state.color_uniform_buffer, 0, bytemuck::cast_slice(&[color_uniform]));

                
                render_pass.set_pipeline(&state.pipeline);
                render_pass.set_bind_group(0, &texture_bind_group, &[]);
                render_pass.set_bind_group(1, &*state.uniform_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..QUAD_INDICES.len() as u32, 0, 0..1);
            }
        }       
        
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
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
    Vertex { position: [-1.0, 1.0], tex_coords: [0.0, 1.0] },
    Vertex { position: [1.0, 1.0], tex_coords: [1.0, 1.0] },
    Vertex { position: [1.0, -1.0], tex_coords: [1.0, 0.0] },
    Vertex { position: [-1.0, -1.0], tex_coords: [0.0, 0.0] },
];

const QUAD_INDICES: [u16; 6] = [
    0, 1, 2,
    0, 2, 3,
];

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