#[derive(Debug)]
pub(crate) struct ImagePipeline {
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) uses_extra_textures: bool,
}
