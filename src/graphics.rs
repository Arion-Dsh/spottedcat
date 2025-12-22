use crate::{Context, DrawCommand, DrawOption, ImageDrawOptions};
use crate::image::{Bounds, Image, ImageEntry};
use crate::image_raw::{ImageRenderer, ImageTransform, InstanceData};
use crate::texture::Texture;
use crate::text_renderer::TextRenderer;

fn mvp_from_draw_options(
    sw: f32,
    sh: f32,
    base_w_px: f32,
    base_h_px: f32,
    opts: ImageDrawOptions,
) -> [[f32; 4]; 4] {
    // `position` is the desired top-left corner in screen pixels (origin at top-left).
    let (px, py) = (opts.position[0].as_f32(), opts.position[1].as_f32());
    let (w_px, h_px) = (
        base_w_px * opts.scale[0],
        base_h_px * opts.scale[1],
    );

    // Target top-left in clip-space.
    let tx = (px / sw) * 2.0 - 1.0;
    let ty = 1.0 - (py / sh) * 2.0;

    // Our quad is in local space [-1, 1]. Width/height = 2.
    // To get a clip-space width of (w_px / sw) * 2, we need sx = w_px / sw.
    let sx = w_px / sw;
    let sy = h_px / sh;

    let (c, s) = (opts.rotation.cos(), opts.rotation.sin());

    // Anchor is local top-left corner of the quad.
    // With our vertex layout, top-left is (-1, +1).
    let p_tl_x = -1.0;
    let p_tl_y = 1.0;

    // Compute where the local top-left ends up after R*S, then choose translation so it lands on (tx,ty).
    // v = R * (S * p)
    let spx = sx * p_tl_x;
    let spy = sy * p_tl_y;
    let v_tl_x = c * spx - s * spy;
    let v_tl_y = s * spx + c * spy;

    let dx = tx - v_tl_x;
    let dy = ty - v_tl_y;

    // Column-major affine matrix used by WGSL (mvp * vec4(pos,0,1)).
    // MVP = T * R * S with T chosen so that (-1,+1) maps to (tx,ty).
    [
        [c * sx, -s * sy, 0.0, 0.0],
        [s * sx, c * sy, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [dx, dy, 0.0, 1.0],
    ]
}

/// MVP calculation for rendering to texture (offscreen).
/// Position is relative to texture's top-left corner in pixels.
/// (0, 0) = top-left of texture, (tw, th) = bottom-right of texture.
fn mvp_for_texture(
    tw: f32,
    th: f32,
    base_w_px: f32,
    base_h_px: f32,
    opts: ImageDrawOptions,
) -> [[f32; 4]; 4] {
    mvp_from_draw_options(tw, th, base_w_px, base_h_px, opts)
}

pub struct Graphics {
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    image_renderer: ImageRenderer,
    images: Vec<Option<ImageEntry>>,
    text_renderer: TextRenderer,
}

impl Graphics {
    pub async fn new(
        instance: &wgpu::Instance,
        surface: &wgpu::Surface<'_>,
        width: u32,
        height: u32,
    ) -> anyhow::Result<Self> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    experimental_features: wgpu::ExperimentalFeatures::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                    trace: wgpu::Trace::Off,
                },
            )
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: width.max(1),
            height: height.max(1),
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let image_renderer = ImageRenderer::new(&device, config.format, 4096);

        let text_renderer = TextRenderer::new(&device, config.format);

        Ok(Self {
            device,
            queue,
            config,
            image_renderer,
            images: Vec::new(),
            text_renderer,
        })
    }

    pub(crate) fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub fn resize(&mut self, surface: &wgpu::Surface<'_>, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        surface.configure(&self.device, &self.config);
    }

     pub(crate) fn device_queue(&self) -> (&wgpu::Device, &wgpu::Queue) {
         (&self.device, &self.queue)
     }

     pub(crate) fn create_texture_bind_group_from_texture(
         &self,
         texture: &Texture,
     ) -> wgpu::BindGroup {
         self.image_renderer
             .create_texture_bind_group(&self.device, &texture.0.view)
     }

     pub(crate) fn insert_image_entry(&mut self, entry: ImageEntry) -> Image {
         let id = Image(self.images.len());
         self.images.push(Some(entry));
         id
     }

     pub(crate) fn insert_sub_image(
         &mut self,
         image: Image,
         bounds: Option<Bounds>,
     ) -> anyhow::Result<Image> {
         let (texture, src_bounds, src_bg) = {
             let src = self
                 .images
                 .get(image.0)
                 .and_then(|v| v.as_ref())
                 .ok_or_else(|| anyhow::anyhow!("invalid source image"))?;
             (src.texture.clone(), src.bounds, src.texture_bind_group.clone())
         };

         let tex_w = texture.0.width;
         let tex_h = texture.0.height;

         let b = match bounds {
             None => src_bounds,
             Some(b) => {
                 let x1 = b
                     .x
                     .checked_add(b.width)
                     .ok_or_else(|| anyhow::anyhow!("bounds overflow"))?;
                 let y1 = b
                     .y
                     .checked_add(b.height)
                     .ok_or_else(|| anyhow::anyhow!("bounds overflow"))?;

                 if x1 > tex_w || y1 > tex_h {
                     return Err(anyhow::anyhow!("sub_image bounds out of range"));
                 }

                 b
             }
         };

         Ok(self.insert_image_entry(ImageEntry::new_with_bounds(
             texture, src_bg, b,
         )))
     }

     pub(crate) fn take_image_entry(&mut self, image: Image) -> Option<ImageEntry> {
         self.images.get_mut(image.0)?.take()
     }

    pub fn draw_context(
        &mut self,
        surface: &wgpu::Surface<'_>,
        context: &Context,
    ) -> Result<(), wgpu::SurfaceError> {
        self.draw_drawables(surface, context.draw_list())
    }

    pub fn draw_drawables(
        &mut self,
        surface: &wgpu::Surface<'_>,
        drawables: &[DrawCommand],
    ) -> Result<(), wgpu::SurfaceError> {
        let frame = surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("graphics_encoder"),
            });

        self.image_renderer.begin_frame();
        self.text_renderer
            .begin_frame(self.config.width, self.config.height, &self.queue);
        let (sw, sh) = (self.config.width as f32, self.config.height as f32);

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("graphics_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            let mut current_key: Option<usize> = None;
            let mut current_bg: Option<wgpu::BindGroup> = None;
            let mut batch: Vec<InstanceData> = Vec::new();

            for drawable in drawables {
                match drawable {
                    DrawCommand::Image(id, opts) => {
                        self.text_renderer
                            .flush(&self.device, &mut rpass, &self.queue);

                        let Some(Some(img)) = self.images.get(id.0) else {
                            continue;
                        };
                        if !img.visible {
                            continue;
                        }

                        let tex_key = std::sync::Arc::as_ptr(&img.texture.0) as usize;
                        if current_key.is_some() && current_key != Some(tex_key) {
                            if !batch.is_empty() {
                                let Some(bind_group) = current_bg.clone() else {
                                    batch.clear();
                                    continue;
                                };
                                let range_opt = {
                                    let renderer = &mut self.image_renderer;
                                    match renderer.upload_instances(&self.queue, batch.as_slice()) {
                                        Ok(r) => Some(r),
                                        Err(_) => None,
                                    }
                                };
                                if let Some(range) = range_opt {
                                    let renderer = &self.image_renderer;
                                    renderer.draw_batch(&mut rpass, &bind_group, range);
                                }
                                batch.clear();
                            }
                        }
                        if current_key != Some(tex_key) {
                            current_key = Some(tex_key);
                            current_bg = Some(img.texture_bind_group.clone());
                        }

                        let base_w_px = img.bounds.width as f32;
                        let base_h_px = img.bounds.height as f32;
                        let t = ImageTransform {
                            mvp: mvp_from_draw_options(sw, sh, base_w_px, base_h_px, *opts),
                            uvp: img.uvp,
                            color: ImageTransform::default().color,
                        };
                        batch.push(InstanceData::from(t));
                    }
                    DrawCommand::Text(text, opts) => {
                        if !batch.is_empty() {
                            if let Some(bind_group) = current_bg.clone() {
                                let range_opt = {
                                    let renderer = &mut self.image_renderer;
                                    match renderer.upload_instances(&self.queue, batch.as_slice()) {
                                        Ok(r) => Some(r),
                                        Err(_) => None,
                                    }
                                };
                                if let Some(range) = range_opt {
                                    let renderer = &self.image_renderer;
                                    renderer.draw_batch(&mut rpass, &bind_group, range);
                                }
                            }
                            batch.clear();
                        }
                        current_key = None;
                        current_bg = None;
                        self.text_renderer
                            .queue_text(&text.clone().to_string(), opts, &self.queue)
                            .expect("Text draw requires valid font_data");
                    }
                }
            }

            if !batch.is_empty() {
                if let Some(bind_group) = current_bg.clone() {
                    let range_opt = {
                        let renderer = &mut self.image_renderer;
                        match renderer.upload_instances(&self.queue, batch.as_slice()) {
                            Ok(r) => Some(r),
                            Err(_) => None,
                        }
                    };
                    if let Some(range) = range_opt {
                        let renderer = &self.image_renderer;
                        renderer.draw_batch(&mut rpass, &bind_group, range);
                    }
                }
                batch.clear();
            }

            current_key = None;

            self.text_renderer
                .flush(&self.device, &mut rpass, &self.queue);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }

    pub(crate) fn copy_image(&mut self, dst: Image, src: Image) -> anyhow::Result<()> {
        let (dst_tex, dst_bounds, dst_format) = {
            let Some(Some(d)) = self.images.get(dst.0) else {
                return Err(anyhow::anyhow!("invalid dst image"));
            };
            (&d.texture.0.texture, d.bounds, d.texture.0.format)
        };
        let (src_tex, src_bounds, src_format) = {
            let Some(Some(s)) = self.images.get(src.0) else {
                return Err(anyhow::anyhow!("invalid src image"));
            };
            (&s.texture.0.texture, s.bounds, s.texture.0.format)
        };

        if dst_bounds.width != src_bounds.width || dst_bounds.height != src_bounds.height {
            return Err(anyhow::anyhow!(
                "image size mismatch: dst {}x{}, src {}x{}",
                dst_bounds.width,
                dst_bounds.height,
                src_bounds.width,
                src_bounds.height
            ));
        }
        if dst_format != src_format {
            return Err(anyhow::anyhow!(
                "image format mismatch: dst {:?}, src {:?}",
                dst_format,
                src_format
            ));
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("graphics_copy_image_encoder"),
            });

        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: src_tex,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: src_bounds.x,
                    y: src_bounds.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: dst_tex,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: dst_bounds.x,
                    y: dst_bounds.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: src_bounds.width,
                height: src_bounds.height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }

    pub(crate) fn clear_image(&mut self, target: Image, color: [f32; 4]) -> anyhow::Result<()> {
        let target_view = {
            let Some(Some(target_entry)) = self.images.get(target.0) else {
                return Err(anyhow::anyhow!("invalid target image"));
            };
            target_entry.texture.0.view.clone()
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("graphics_clear_image_encoder"),
            });

        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("graphics_clear_image_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: color[0] as f64,
                            g: color[1] as f64,
                            b: color[2] as f64,
                            a: color[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }

    pub(crate) fn draw_drawables_to_image(

        &mut self,
        target: Image,
        drawables: &[DrawCommand],
        _option: DrawOption,
    ) -> anyhow::Result<()> {
        if drawables
            .iter()
            .any(|d| matches!(d, DrawCommand::Image(id, _) if *id == target))
        {
            return Err(anyhow::anyhow!(
                "cannot draw an image into itself; use a separate target image"
            ));
        }

        let (target_view, target_bounds) = {
            let Some(Some(target_entry)) = self.images.get(target.0) else {
                return Err(anyhow::anyhow!("invalid target image"));
            };
            (target_entry.texture.0.view.clone(), target_entry.bounds)
        };

        let (tw, th) = (target_bounds.width as f32, target_bounds.height as f32);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("graphics_offscreen_encoder"),
            });

        let ops = wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
        };

        self.image_renderer.begin_frame();
        self.text_renderer
            .begin_frame(target_bounds.width, target_bounds.height, &self.queue);

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("graphics_offscreen_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_viewport(
                target_bounds.x as f32,
                target_bounds.y as f32,
                tw,
                th,
                0.0,
                1.0,
            );
            rpass.set_scissor_rect(
                target_bounds.x,
                target_bounds.y,
                target_bounds.width,
                target_bounds.height,
            );

            let mut current_key: Option<usize> = None;
            let mut current_bg: Option<wgpu::BindGroup> = None;
            let mut batch: Vec<InstanceData> = Vec::new();

            for drawable in drawables {
                match drawable {
                    DrawCommand::Image(id, opts) => {
                        self.text_renderer
                            .flush(&self.device, &mut rpass, &self.queue);

                        let Some(Some(img)) = self.images.get(id.0) else {
                            continue;
                        };
                        if !img.visible {
                            continue;
                        }

                        let tex_key = std::sync::Arc::as_ptr(&img.texture.0) as usize;
                        if current_key.is_some() && current_key != Some(tex_key) {
                            if !batch.is_empty() {
                                if let Some(bind_group) = current_bg.clone() {
                                    let range = {
                                        let renderer = &mut self.image_renderer;
                                        renderer.upload_instances(&self.queue, batch.as_slice())?
                                    };
                                    let renderer = &self.image_renderer;
                                    renderer.draw_batch(&mut rpass, &bind_group, range);
                                }
                                batch.clear();
                            }
                        }

                        if current_key != Some(tex_key) {
                            current_key = Some(tex_key);
                            current_bg = Some(img.texture_bind_group.clone());
                        }

                        let base_w_px = img.bounds.width as f32;
                        let base_h_px = img.bounds.height as f32;
                        let t = ImageTransform {
                            mvp: mvp_for_texture(tw, th, base_w_px, base_h_px, *opts),
                            uvp: img.uvp,
                            color: ImageTransform::default().color,
                        };
                        batch.push(InstanceData::from(t));
                    }
                    DrawCommand::Text(text, opts) => {
                        if !batch.is_empty() {
                            if let Some(bind_group) = current_bg.clone() {
                                let range = {
                                    let renderer = &mut self.image_renderer;
                                    renderer.upload_instances(&self.queue, batch.as_slice())?
                                };
                                let renderer = &self.image_renderer;
                                renderer.draw_batch(&mut rpass, &bind_group, range);
                            }
                            batch.clear();
                        }
                        current_key = None;
                        current_bg = None;

                        self.text_renderer
                            .queue_text(&text.to_string(), opts, &self.queue)
                            .expect("Text draw requires valid font_data");
                    }
                }
            }

            if !batch.is_empty() {
                if let Some(bind_group) = current_bg.clone() {
                    let range = {
                        let renderer = &mut self.image_renderer;
                        renderer.upload_instances(&self.queue, batch.as_slice())?
                    };
                    let renderer = &self.image_renderer;
                    renderer.draw_batch(&mut rpass, &bind_group, range);
                }
                batch.clear();
            }

            self.text_renderer
                .flush(&self.device, &mut rpass, &self.queue);
        }

        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }
}