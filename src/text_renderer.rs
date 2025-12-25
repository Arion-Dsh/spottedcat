use crate::{DrawOption, Text};
use ab_glyph::{Font as _, FontArc, Glyph, GlyphId, PxScale, ScaleFont as _};
use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TextUniforms {
    screen_size: [f32; 2],
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GlyphInstance {
    pos: [f32; 2],
    size: [f32; 2],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    color: [f32; 4],
}

impl GlyphInstance {
    const ATTRS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        2 => Float32x2, // pos
        3 => Float32x2, // size
        4 => Float32x2, // uv_min
        5 => Float32x2, // uv_max
        6 => Float32x4, // color
    ];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GlyphInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphKey {
    font_hash: u64,
    px_size_bits: u32,
    glyph_id: u32,
}

#[derive(Debug, Clone, Copy)]
struct AtlasRect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

impl AtlasRect {
    fn uv(&self, atlas_w: u32, atlas_h: u32) -> ([f32; 2], [f32; 2]) {
        let u0 = self.x as f32 / atlas_w as f32;
        let v0 = self.y as f32 / atlas_h as f32;
        let u1 = (self.x + self.w) as f32 / atlas_w as f32;
        let v1 = (self.y + self.h) as f32 / atlas_h as f32;
        ([u0, v0], [u1, v1])
    }
}

#[derive(Debug, Clone, Copy)]
struct GlyphEntry {
    rect: AtlasRect,
    bmin: [f32; 2],
    bmax: [f32; 2],
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut h);
    h.finish()
}

pub struct TextRenderer {
    pipeline: wgpu::RenderPipeline,

    quad_vb: wgpu::Buffer,
    quad_ib: wgpu::Buffer,
    quad_index_count: u32,

    uniform_buffer: wgpu::Buffer,
    uniform_bg: wgpu::BindGroup,

    atlas_texture: wgpu::Texture,
    atlas_bg: wgpu::BindGroup,

    atlas_w: u32,
    atlas_h: u32,
    next_x: u32,
    next_y: u32,
    row_h: u32,

    instances: Vec<GlyphInstance>,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,

    font_cache: HashMap<u64, FontArc>,
    glyph_cache: HashMap<GlyphKey, GlyphEntry>,
}

impl TextRenderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let uniforms = TextUniforms {
            screen_size: [1.0, 1.0],
            _pad: [0.0, 0.0],
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("text_uniform_buffer"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("text_uniform_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(
                        std::num::NonZeroU64::new(std::mem::size_of::<TextUniforms>() as u64).unwrap(),
                    ),
                },
                count: None,
            }],
        });

        let uniform_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("text_uniform_bg"),
            layout: &uniform_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let atlas_w = 1024u32;
        let atlas_h = 1024u32;
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("text_atlas"),
            size: wgpu::Extent3d {
                width: atlas_w,
                height: atlas_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("text_atlas_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let atlas_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("text_atlas_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let atlas_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("text_atlas_bg"),
            layout: &atlas_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                },
            ],
        });

        #[repr(C)]
        #[derive(Clone, Copy, Pod, Zeroable)]
        struct QuadVertex {
            pos: [f32; 2],
            uv: [f32; 2],
        }

        impl QuadVertex {
            const ATTRS: [wgpu::VertexAttribute; 2] =
                wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];

            fn layout() -> wgpu::VertexBufferLayout<'static> {
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &Self::ATTRS,
                }
            }
        }

        let vertices: [QuadVertex; 4] = [
            QuadVertex { pos: [0.0, 0.0], uv: [0.0, 0.0] },
            QuadVertex { pos: [1.0, 0.0], uv: [1.0, 0.0] },
            QuadVertex { pos: [1.0, 1.0], uv: [1.0, 1.0] },
            QuadVertex { pos: [0.0, 1.0], uv: [0.0, 1.0] },
        ];
        let indices: [u16; 6] = [0, 1, 2, 2, 3, 0];

        let quad_vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("text_quad_vb"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let quad_ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("text_quad_ib"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("text_shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
struct Uniforms {
    screen_size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;

@group(1) @binding(0)
var atlas: texture_2d<f32>;

@group(1) @binding(1)
var samp: sampler;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,

    @location(2) i_pos: vec2<f32>,
    @location(3) i_size: vec2<f32>,
    @location(4) i_uv_min: vec2<f32>,
    @location(5) i_uv_max: vec2<f32>,
    @location(6) i_color: vec4<f32>,
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    let p = in.i_pos + in.pos * in.i_size;

    let x = (p.x / u.screen_size.x) * 2.0 - 1.0;
    let y = 1.0 - (p.y / u.screen_size.y) * 2.0;

    out.clip_pos = vec4<f32>(x, y, 0.0, 1.0);

    out.uv = mix(in.i_uv_min, in.i_uv_max, in.uv);
    out.color = in.i_color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let a = textureSample(atlas, samp, in.uv).r;
    return vec4<f32>(in.color.rgb, in.color.a * a);
}
"#
                .into(),
            ),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("text_pipeline_layout"),
            bind_group_layouts: &[&uniform_bgl, &atlas_bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("text_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[QuadVertex::layout(), GlyphInstance::layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        let instance_capacity = 2048usize;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text_instance_buffer"),
            size: (instance_capacity * std::mem::size_of::<GlyphInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            quad_vb,
            quad_ib,
            quad_index_count: indices.len() as u32,
            uniform_buffer,
            uniform_bg,
            atlas_texture,
            atlas_bg,
            atlas_w,
            atlas_h,
            next_x: 0,
            next_y: 0,
            row_h: 0,
            instances: Vec::new(),
            instance_buffer,
            instance_capacity,
            font_cache: HashMap::new(),
            glyph_cache: HashMap::new(),
        }
    }

    fn get_font(&mut self, font_data: &Vec<u8>) -> anyhow::Result<(u64, FontArc)> {
        let h = hash_bytes(font_data);
        if let Some(f) = self.font_cache.get(&h) {
            return Ok((h, f.clone()));
        }
        let font = FontArc::try_from_vec(font_data.clone())
            .map_err(|e| anyhow::anyhow!("Failed to parse font: {}", e))?;
        self.font_cache.insert(h, font.clone());
        Ok((h, font))
    }

    fn alloc_atlas(&mut self, w: u32, h: u32) -> Option<AtlasRect> {
        if w == 0 || h == 0 || w > self.atlas_w || h > self.atlas_h {
            return None;
        }

        if self.next_x + w > self.atlas_w {
            self.next_x = 0;
            self.next_y = self.next_y.saturating_add(self.row_h);
            self.row_h = 0;
        }

        if self.next_y + h > self.atlas_h {
            return None;
        }

        let rect = AtlasRect {
            x: self.next_x,
            y: self.next_y,
            w,
            h,
        };
        self.next_x += w;
        self.row_h = self.row_h.max(h);
        Some(rect)
    }

    fn upload_alpha(
        &self,
        queue: &wgpu::Queue,
        rect: AtlasRect,
        alpha: &[u8],
        width: u32,
        height: u32,
    ) {
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u32;
        let bytes_per_row = width;
        let aligned_bpr = ((bytes_per_row + align - 1) / align) * align;

        if aligned_bpr == bytes_per_row {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.atlas_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: rect.x,
                        y: rect.y,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                alpha,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            return;
        }

        let mut padded = vec![0u8; (aligned_bpr * height) as usize];
        for row in 0..height {
            let src0 = (row * bytes_per_row) as usize;
            let src1 = src0 + bytes_per_row as usize;
            let dst0 = (row * aligned_bpr) as usize;
            let dst1 = dst0 + bytes_per_row as usize;
            padded[dst0..dst1].copy_from_slice(&alpha[src0..src1]);
        }

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: rect.x,
                    y: rect.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &padded,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(aligned_bpr),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn begin_frame(&mut self, screen_w: u32, screen_h: u32, queue: &wgpu::Queue) {
        let u = TextUniforms {
            screen_size: [screen_w as f32, screen_h as f32],
            _pad: [0.0, 0.0],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&u));
        self.instances.clear();
    }

    pub fn queue_text(
        &mut self,
        text: &Text,
        opts: &DrawOption,
        queue: &wgpu::Queue,
    ) -> anyhow::Result<()> {
        let (font_hash, font) = self.get_font(&text.font_data)?;
        let px_size = (text.font_size.as_f32() * opts.scale[1]).max(1.0);
        let scale = PxScale::from(px_size);
        let scaled = font.as_scaled(scale);

        let mut caret_x = 0.0f32;
        let baseline_y = scaled.ascent();
        let mut prev: Option<GlyphId> = None;

        for ch in text.content.chars() {
            let id = scaled.glyph_id(ch);
            if let Some(p) = prev {
                caret_x += scaled.kern(p, id);
            }

            // Ensure cached in atlas (upload if needed)
            let entry = {
                let key = GlyphKey {
                    font_hash,
                    px_size_bits: px_size.to_bits(),
                    glyph_id: id.0 as u32,
                };
                if let Some(e) = self.glyph_cache.get(&key) {
                    *e
                } else {
                    // Re-outline at origin for stable bitmap.
                    let g0: Glyph = Glyph {
                        id,
                        scale,
                        position: ab_glyph::point(0.0, 0.0),
                    };
                    let outlined0 = match scaled.outline_glyph(g0) {
                        None => {
                            // Non-drawable glyph (e.g. space). Skip caching and drawing.
                            caret_x += scaled.h_advance(id);
                            prev = Some(id);
                            continue;
                        }
                        Some(o) => o,
                    };
                    let b0 = outlined0.px_bounds();
                    let w0 = (b0.max.x - b0.min.x).ceil().max(1.0) as u32;
                    let h0 = (b0.max.y - b0.min.y).ceil().max(1.0) as u32;
                    let rect = self
                        .alloc_atlas(w0, h0)
                        .ok_or_else(|| anyhow::anyhow!("text atlas full"))?;
                    let mut alpha = vec![0u8; (w0 * h0) as usize];
                    outlined0.draw(|x, y, v| {
                        if x >= w0 || y >= h0 {
                            return;
                        }
                        let idx = (y * w0 + x) as usize;
                        let a = (v * 255.0).round().clamp(0.0, 255.0) as u8;
                        alpha[idx] = alpha[idx].max(a);
                    });
                    self.upload_alpha(queue, rect, &alpha, w0, h0);
                    let e = GlyphEntry {
                        rect,
                        bmin: [b0.min.x, b0.min.y],
                        bmax: [b0.max.x, b0.max.y],
                    };
                    self.glyph_cache.insert(key, e);
                    e
                }
            };

            let (uv_min, uv_max) = entry.rect.uv(self.atlas_w, self.atlas_h);

            let w = (entry.bmax[0] - entry.bmin[0]).ceil().max(1.0);
            let h = (entry.bmax[1] - entry.bmin[1]).ceil().max(1.0);

            let base_pos = [
                opts.position[0].as_f32() + caret_x + entry.bmin[0],
                opts.position[1].as_f32() + baseline_y + entry.bmin[1],
            ];

            let stroke_w = text.stroke_width.as_f32();
            if stroke_w > 0.0 {
                let r = stroke_w.ceil().max(1.0) as i32;
                for dy in -r..=r {
                    for dx in -r..=r {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        if (dx * dx + dy * dy) as f32 > stroke_w * stroke_w {
                            continue;
                        }
                        self.instances.push(GlyphInstance {
                            pos: [base_pos[0] + dx as f32, base_pos[1] + dy as f32],
                            size: [w, h],
                            uv_min,
                            uv_max,
                            color: text.stroke_color,
                        });
                    }
                }
            }

            self.instances.push(GlyphInstance {
                pos: base_pos,
                size: [w, h],
                uv_min,
                uv_max,
                color: text.color,
            });

            caret_x += scaled.h_advance(id);
            prev = Some(id);
        }

        Ok(())
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        pass: &mut wgpu::RenderPass<'_>,
        queue: &wgpu::Queue,
    ) {
        if self.instances.is_empty() {
            return;
        }

        if self.instances.len() > self.instance_capacity {
            self.instance_capacity = self.instances.len().next_power_of_two();
            self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("text_instance_buffer_grow"),
                size: (self.instance_capacity * std::mem::size_of::<GlyphInstance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&self.instances),
        );

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniform_bg, &[]);
        pass.set_bind_group(1, &self.atlas_bg, &[]);
        pass.set_vertex_buffer(0, self.quad_vb.slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        pass.set_index_buffer(self.quad_ib.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..self.quad_index_count, 0, 0..self.instances.len() as u32);
    }

    pub fn flush(
        &mut self,
        device: &wgpu::Device,
        pass: &mut wgpu::RenderPass<'_>,
        queue: &wgpu::Queue,
    ) {
        self.draw(device, pass, queue);
        self.instances.clear();
    }
}
