//! Shader registration for custom image effects.

use crate::image_raw::InstanceData;

use super::Graphics;

impl Graphics {
    pub(crate) fn register_image_shader(&mut self, user_functions: &str) -> u32 {
        let shader_id = self.next_image_shader_id;
        self.next_image_shader_id = self.next_image_shader_id.saturating_add(1);

        // Hook-function injection.
        // User provides WGSL code snippets:
        // They will be inserted at markers in vs_main and fs_main.
        let base_template = include_str!("../shaders/image.wgsl");
        let mut combined_shader = base_template.to_string();

        if let Some(vs_start) = user_functions.find("fn user_vs_hook") {
            let vs_body_start = user_functions[vs_start..]
                .find('{')
                .map(|i| vs_start + i + 1)
                .unwrap_or(vs_start);
            let vs_end = user_functions[vs_body_start..]
                .find("fn user_fs_hook")
                .map(|rel| vs_body_start + rel)
                .unwrap_or(user_functions.len());
            let vs_body_end = user_functions[..vs_end].rfind('}').unwrap_or(vs_end);
            let vs_src = user_functions[vs_body_start..vs_body_end].trim();

            if !vs_src.is_empty() {
                let marker = "// USER_VS_HOOK";
                if let Some(pos) = combined_shader.rfind(marker) {
                    combined_shader.insert_str(pos + marker.len(), &format!("\n{}", vs_src));
                }
            }
        }

        if let Some(fs_start) = user_functions.find("fn user_fs_hook") {
            let fs_body_start = user_functions[fs_start..]
                .find('{')
                .map(|i| fs_start + i + 1)
                .unwrap_or(fs_start);
            let fs_end = user_functions.len();
            let fs_body_end = user_functions[..fs_end].rfind('}').unwrap_or(fs_end);
            let fs_src = user_functions[fs_body_start..fs_body_end].trim();

            if !fs_src.is_empty() {
                let marker = "// USER_FS_HOOK";
                if let Some(pos) = combined_shader.rfind(marker) {
                    combined_shader.insert_str(pos + marker.len(), &format!("\n{}", fs_src));
                }
            }
        }

        if std::env::var("SPOT_DEBUG_SHADER").is_ok() {
            let vs_marker = "// USER_VS_HOOK";
            let fs_marker = "// USER_FS_HOOK";

            let vs_block = if let Some(pos) = combined_shader.find(vs_marker) {
                let end = combined_shader[pos..]
                    .find("return")
                    .map(|i| pos + i)
                    .unwrap_or(combined_shader.len());
                &combined_shader[pos..end]
            } else {
                "<missing vs hook marker>"
            };
            let fs_block = if let Some(pos) = combined_shader.find(fs_marker) {
                let end = combined_shader[pos..]
                    .find("return")
                    .map(|i| pos + i)
                    .unwrap_or(combined_shader.len());
                &combined_shader[pos..end]
            } else {
                "<missing fs hook marker>"
            };

            eprintln!(
                "[spot][debug][shader] register_image_shader id={}\n{}\n{}",
                shader_id, vs_block, fs_block
            );
        }

        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("custom_image_shader"),
                source: wgpu::ShaderSource::Wgsl(combined_shader.into()),
            });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("custom_image_pipeline_layout"),
                bind_group_layouts: &[
                    &self.image_renderer.texture_bind_group_layout,
                    &self.image_renderer.user_globals_bind_group_layout,
                    &self.image_renderer.engine_globals_bind_group_layout,
                ],
                immediate_size: 0,
            });

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("custom_image_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[InstanceData::layout()],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
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
                        format: self.config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });

        self.image_pipelines.insert(shader_id, pipeline);
        shader_id
    }
}
