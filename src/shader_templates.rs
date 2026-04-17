use crate::image_shader::{ImageShaderBlendMode, ImageShaderDesc};

/// A builder for creating data-driven image shaders with automatic boilerplate injection.
///
/// `ImageShaderTemplate` provides a high-level API to customize the vertex and fragment
/// stages without writing full WGSL from scratch. It automatically injects:
/// - Standard structs: `VsIn`, `VsOut`, `EngineGlobals`.
/// - Core variables: `screen`, `opacity`, `scale_factor`.
/// - Bindings: Source texture (`tex`/`samp`) and semantic extra textures (`t_history`, `t_screen`, etc.)
///
/// Use [`ImageShaderTemplate::build_desc`] to get a descriptor ready for registration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImageShaderTemplate {
    uses_extra_textures: bool,
    blend_mode: ImageShaderBlendMode,
    shared: String,
    vertex_body: String,
    fragment_body: String,
    extra_texture_names: [Option<String>; 4],
    user_global_names: [Option<String>; 16],
    history_slot: Option<usize>,
    screen_slot: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelShaderTemplate {
    shared: String,
    fragment_body: String,
}

const IMAGE_SHADER_TEMPLATE_PREFIX: &str = r#"struct EngineGlobals {
    screen: vec4<f32>,
    opacity: f32,
    shader_opacity: f32,
    scale_factor: f32,
    _padding: f32,
};

struct VsIn {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) pos: vec2<f32>,
    @location(1) rotation: f32,
    @location(2) size: vec2<f32>,
    @location(3) uv_rect: vec4<f32>,
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) local_uv: vec2<f32>,
    @location(2) uv_scale: vec2<f32>,
};

@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
"#;

const MODEL_SHADER_TEMPLATE: &str = include_str!("shaders/templates/model_template.wgsl");

impl ImageShaderTemplate {
    pub fn new() -> Self {
        Self {
            uses_extra_textures: false,
            blend_mode: ImageShaderBlendMode::Alpha,
            shared: String::new(),
            vertex_body: String::new(),
            fragment_body: String::from("return vec4<f32>(src.rgb, src.a * opacity);"),
            extra_texture_names: Default::default(),
            user_global_names: Default::default(),
            history_slot: None,
            screen_slot: None,
        }
    }

    pub fn with_extra_textures(mut self, enabled: bool) -> Self {
        self.uses_extra_textures = enabled;
        self
    }

    pub fn with_blend_mode(mut self, blend_mode: ImageShaderBlendMode) -> Self {
        self.blend_mode = blend_mode;
        self
    }

    pub fn with_shared(mut self, shared: impl Into<String>) -> Self {
        self.shared = shared.into();
        self
    }

    pub fn with_vertex_body(mut self, vertex_body: impl Into<String>) -> Self {
        self.vertex_body = vertex_body.into();
        self
    }

    pub fn with_fragment_body(mut self, fragment_body: impl Into<String>) -> Self {
        self.fragment_body = fragment_body.into();
        self
    }

    /// Provides a custom name for an extra texture slot when using the internal prelude.
    ///
    /// This name can then be used in the shader body (e.g., `textureSample(custom_name, ...)`).
    pub fn with_texture_alias(mut self, slot: usize, name: impl Into<String>) -> Self {
        if slot < 4 {
            self.extra_texture_names[slot] = Some(name.into());
        }
        self
    }

    pub fn with_user_global_alias(mut self, slot: usize, name: impl Into<String>) -> Self {
        if slot < 16 {
            self.user_global_names[slot] = Some(name.into());
        }
        self
    }

    /// Registers the history texture semantic to a specific slot.
    ///
    /// When using the internal prelude, this slot will be automatically aliased to `t_history`.
    pub fn with_history_at(mut self, slot: usize) -> Self {
        if slot < 4 {
            self.history_slot = Some(slot);
        }
        self
    }

    /// Registers the screen snapshot semantic to a specific slot.
    ///
    /// When using the internal prelude, this slot will be automatically aliased to `t_screen`.
    pub fn with_screen_at(mut self, slot: usize) -> Self {
        if slot < 4 {
            self.screen_slot = Some(slot);
        }
        self
    }

    pub fn build(self) -> String {
        image_shader_template_from_slots(&self)
    }

    pub fn build_desc(self) -> ImageShaderDesc {
        let uses_extra_textures = self.uses_extra_textures;
        let blend_mode = self.blend_mode;
        let history_slot = self.history_slot;
        let screen_slot = self.screen_slot;
        
        let mut desc = ImageShaderDesc::from_wgsl(self.build())
            .with_extra_textures(uses_extra_textures)
            .with_blend_mode(blend_mode);
            
        if let Some(slot) = history_slot {
            desc = desc.with_history_slot(slot);
        }
        if let Some(slot) = screen_slot {
            desc = desc.with_screen_slot(slot);
        }
        desc
    }
}

impl ModelShaderTemplate {
    pub fn new() -> Self {
        Self {
            shared: String::new(),
            fragment_body: String::from(
                "return vec4<f32>(src.rgb, src.a * model_globals.extra.x);",
            ),
        }
    }

    pub fn with_shared(mut self, shared: impl Into<String>) -> Self {
        self.shared = shared.into();
        self
    }

    pub fn with_fragment_body(mut self, fragment_body: impl Into<String>) -> Self {
        self.fragment_body = fragment_body.into();
        self
    }

    pub fn build(self) -> String {
        model_shader_template_from_slots(&self)
    }
}

fn image_shader_template_from_slots(template: &ImageShaderTemplate) -> String {
    let mut wgsl = image_shader_prelude_with_full_metadata_internal(
        template.uses_extra_textures, 
        &template.extra_texture_names,
        template.history_slot,
        template.screen_slot
    );

    if !template.shared.trim().is_empty() {
        wgsl.push('\n');
        wgsl.push_str(template.shared.trim());
        wgsl.push('\n');
    }

    wgsl.push_str(
        r#"
@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;

    var pos_arr = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, 1.0),
    );
    var uv_arr = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
    );

    let local_pos = pos_arr[in.vertex_index];
    let uv = uv_arr[in.vertex_index];
    
    // Inject core variables
    let screen = _sp_internal.screen;
    let opacity = _sp_internal.opacity * _sp_internal.shader_opacity;
    let scale_factor = _sp_internal.scale_factor;

    let sw_inv_2 = screen.x;
    let sh_inv_2 = screen.y;
    let sw_inv = screen.z;
    let sh_inv = screen.w;

    let tx = in.pos.x * sw_inv_2 - 1.0;
    let ty = 1.0 - in.pos.y * sh_inv_2;
    let c = cos(in.rotation);
    let s = sin(in.rotation);

    let ox = (local_pos.x + 1.0) * 0.5 * in.size.x;
    let oy = (1.0 - local_pos.y) * 0.5 * in.size.y;
    let rx = c * ox + s * oy;
    let ry = c * oy - s * ox;

    let x = tx + rx * sw_inv_2;
    let y = ty - ry * sh_inv_2;

    out.clip_pos = vec4<f32>(x, y, 0.0, 1.0);
    out.local_uv = uv;
    out.uv = vec2<f32>(
        in.uv_rect.x + uv.x * in.uv_rect.z,
        in.uv_rect.y + uv.y * in.uv_rect.w,
    );
    out.uv_scale = in.uv_rect.zw;
"#,
    );

    let vertex_body = template.vertex_body.trim();
    if !vertex_body.is_empty() {
        for line in vertex_body.lines() {
            wgsl.push_str("    ");
            wgsl.push_str(line);
            wgsl.push('\n');
        }
    }

    wgsl.push_str(
        r#"    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let src = textureSample(tex, samp, in.uv);
    let opacity = _sp_internal.opacity * _sp_internal.shader_opacity;
    let screen = _sp_internal.screen;
    let scale_factor = _sp_internal.scale_factor;
"#,
    );

    // Inject user global aliases
    for (i, name) in template.user_global_names.iter().enumerate() {
        if let Some(name) = name {
            wgsl.push_str(&format!("    let {name} = user_globals[{i}];\n"));
        }
    }

    let body = template.fragment_body.trim();
    if body.is_empty() {
        wgsl.push_str("    return vec4<f32>(src.rgb, src.a * opacity);\n");
    } else {
        for line in body.lines() {
            wgsl.push_str("    ");
            wgsl.push_str(line);
            wgsl.push('\n');
        }
    }
    wgsl.push_str("}\n");
    wgsl
}


pub fn image_shader_prelude_with_full_metadata_internal(
    uses_extra_textures: bool,
    texture_names: &[Option<String>; 4],
    history_slot: Option<usize>,
    screen_slot: Option<usize>,
) -> String {
    let mut final_names = [None, None, None, None];
    for i in 0..4 {
        final_names[i] = texture_names[i].clone();
    }
    
    if let Some(slot) = history_slot {
        if final_names[slot].is_none() {
            final_names[slot] = Some("t_history".to_string());
        }
    }
    if let Some(slot) = screen_slot {
        if final_names[slot].is_none() {
            final_names[slot] = Some("t_screen".to_string());
        }
    }

    let user_group = if uses_extra_textures { 2 } else { 1 };
    let engine_group = if uses_extra_textures { 3 } else { 2 };

    let mut wgsl = String::from(IMAGE_SHADER_TEMPLATE_PREFIX);

    if uses_extra_textures {
        wgsl.push('\n');
        for i in 0..4 {
            let name = final_names[i].as_deref().unwrap_or_else(|| match i {
                0 => "t0",
                1 => "t1",
                2 => "t2",
                3 => "t3",
                _ => unreachable!(),
            });
            wgsl.push_str(&format!("@group(1) @binding({i}) var {name}: texture_2d<f32>;\n"));
        }
        wgsl.push_str("@group(1) @binding(4) var extra_samp: sampler;\n");
    }

    wgsl.push_str(&format!(
        "\n@group({user_group}) @binding(0) var<uniform> user_globals: array<vec4<f32>, 16>;\n"
    ));
    wgsl.push_str(&format!(
        "@group({engine_group}) @binding(0) var<uniform> _sp_internal: EngineGlobals;\n"
    ));

    wgsl
}

/// Returns a full WGSL image shader template that matches Spot's current image pipeline contract.
pub fn image_shader_template() -> String {
    ImageShaderTemplate::new().build()
}

/// Returns a full WGSL model shader template that matches Spot's current 3D pipeline contract.
///
/// The generated shader defines `vs_main`, `vs_main_instanced`, and `fs_main`.
pub fn model_shader_template() -> &'static str {
    MODEL_SHADER_TEMPLATE
}

fn model_shader_template_from_slots(template: &ModelShaderTemplate) -> String {
    let shared = if template.shared.trim().is_empty() {
        String::new()
    } else {
        format!("{}\n", template.shared.trim())
    };

    let fragment_body = if template.fragment_body.trim().is_empty() {
        String::from("    return vec4<f32>(src.rgb, src.a * model_globals.extra.x);")
    } else {
        template
            .fragment_body
            .trim()
            .lines()
            .map(|line| format!("    {line}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    MODEL_SHADER_TEMPLATE
        .replace("// MODEL_SHARED_SLOT\n", &shared)
        .replace("    // MODEL_FRAGMENT_BODY_SLOT", &fragment_body)
}

#[cfg(test)]
mod tests {
    use super::{
        ImageShaderTemplate, ModelShaderTemplate, image_shader_template, model_shader_template,
    };

    #[test]
    fn image_shader_template_without_extra_textures_uses_default_groups() {
        let shader = image_shader_template();
        assert!(shader.contains("@group(1) @binding(0) var<uniform> user_globals"));
        assert!(shader.contains("@group(2) @binding(0) var<uniform> _sp_internal"));
        assert!(!shader.contains("var t0: texture_2d<f32>;"));
    }

    #[test]
    fn image_shader_template_with_extra_textures_shifts_groups() {
        let shader = ImageShaderTemplate::new().with_extra_textures(true).build();
        assert!(shader.contains("@group(1) @binding(0) var t0: texture_2d<f32>;"));
        assert!(shader.contains("@group(2) @binding(0) var<uniform> user_globals"));
        assert!(shader.contains("@group(3) @binding(0) var<uniform> _sp_internal"));
    }

    #[test]
    fn model_shader_template_has_required_entry_points() {
        let shader = model_shader_template();
        assert!(shader.contains("fn vs_main("));
        assert!(shader.contains("fn vs_main_instanced("));
        assert!(shader.contains("fn fs_main("));
    }

    #[test]
    fn image_shader_template_includes_shared_and_fragment_slots() {
        let shader = ImageShaderTemplate::new()
            .with_shared("fn boost(c: vec3<f32>) -> vec3<f32> { return c * 2.0; }")
            .with_fragment_body("return vec4<f32>(boost(src.rgb), src.a * opacity);")
            .build();
        assert!(shader.contains("fn boost(c: vec3<f32>) -> vec3<f32>"));
        assert!(shader.contains("return vec4<f32>(boost(src.rgb), src.a * opacity);"));
    }

    #[test]
    fn image_shader_template_includes_vertex_body_slot() {
        let shader = ImageShaderTemplate::new()
            .with_vertex_body("out.local_uv = out.local_uv * 0.5;")
            .build();
        assert!(shader.contains("out.local_uv = out.local_uv * 0.5;"));
    }
}
