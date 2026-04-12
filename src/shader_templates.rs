use crate::image_shader::{ImageShaderBlendMode, ImageShaderDesc};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImageShaderTemplate {
    uses_extra_textures: bool,
    blend_mode: ImageShaderBlendMode,
    shared: String,
    vertex_body: String,
    fragment_body: String,
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

    pub fn build(self) -> String {
        image_shader_template_from_slots(&self)
    }

    pub fn build_desc(self) -> ImageShaderDesc {
        let uses_extra_textures = self.uses_extra_textures;
        let blend_mode = self.blend_mode;
        ImageShaderDesc::from_wgsl(self.build())
            .with_extra_textures(uses_extra_textures)
            .with_blend_mode(blend_mode)
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
    let uses_extra_textures = template.uses_extra_textures;
    let user_group = if uses_extra_textures { 2 } else { 1 };
    let engine_group = if uses_extra_textures { 3 } else { 2 };

    let mut wgsl = String::from(IMAGE_SHADER_TEMPLATE_PREFIX);

    if uses_extra_textures {
        wgsl.push_str(
            r#"
@group(1) @binding(0) var t0: texture_2d<f32>;
@group(1) @binding(1) var t1: texture_2d<f32>;
@group(1) @binding(2) var t2: texture_2d<f32>;
@group(1) @binding(3) var t3: texture_2d<f32>;
@group(1) @binding(4) var extra_samp: sampler;
"#,
        );
    }

    wgsl.push_str(&format!(
        "\n@group({user_group}) @binding(0) var<uniform> user_globals: array<vec4<f32>, 16>;\n"
    ));
    wgsl.push_str(&format!(
        "@group({engine_group}) @binding(0) var<uniform> _sp_internal: EngineGlobals;\n"
    ));

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
    let sw_inv_2 = _sp_internal.screen.x;
    let sh_inv_2 = _sp_internal.screen.y;
    let sw_inv = _sp_internal.screen.z;
    let sh_inv = _sp_internal.screen.w;

    let tx = in.pos.x * sw_inv_2 - 1.0;
    let ty = 1.0 - in.pos.y * sh_inv_2;
    let sx = in.size.x * sw_inv;
    let sy = in.size.y * sh_inv;

    let c = cos(in.rotation);
    let s = sin(in.rotation);
    let dx = tx - (c * sx * -1.0 - s * sy * 1.0);
    let dy = ty - (s * sx * -1.0 + c * sy * 1.0);

    let x = local_pos.x * (c * sx) + local_pos.y * (-s * sy) + dx;
    let y = local_pos.x * (s * sx) + local_pos.y * (c * sy) + dy;

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
"#,
    );

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

    #[test]
    fn model_shader_template_includes_shared_and_fragment_slots() {
        let shader = ModelShaderTemplate::new()
            .with_shared(
                "fn tint(c: vec3<f32>) -> vec3<f32> { return c * vec3<f32>(0.8, 0.9, 1.0); }",
            )
            .with_fragment_body("return vec4<f32>(tint(src.rgb), src.a * model_globals.extra.x);")
            .build();
        assert!(shader.contains("fn tint(c: vec3<f32>) -> vec3<f32>"));
        assert!(shader.contains("return vec4<f32>(tint(src.rgb), src.a * model_globals.extra.x);"));
        assert!(!shader.contains("MODEL_SHARED_SLOT"));
        assert!(!shader.contains("MODEL_FRAGMENT_BODY_SLOT"));
    }
}
