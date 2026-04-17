@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

@group(1) @binding(0) var<uniform> user_globals: array<vec4<f32>, 16>;

struct EngineGlobals {
    screen: vec4<f32>,
    opacity: f32,
    shader_opacity: f32,
    scale_factor: f32,
    _padding: f32,
};

@group(2) @binding(0) var<uniform> _sp_internal: EngineGlobals;

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
};

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
    let c = cos(in.rotation);
    let s = sin(in.rotation);

    let ox = (local_pos.x + 1.0) * 0.5 * in.size.x;
    let oy = (1.0 - local_pos.y) * 0.5 * in.size.y;
    let rx = c * ox + s * oy;
    let ry = c * oy - s * ox;

    let x = tx + rx * sw_inv_2;
    let y = ty - ry * sh_inv_2;

    out.clip_pos = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(
        in.uv_rect.x + uv.x * in.uv_rect.z,
        in.uv_rect.y + uv.y * in.uv_rect.w,
    );
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let color = textureSample(tex, samp, in.uv);
    let tint = user_globals[0];
    return vec4<f32>(color.rgb * tint.rgb, color.a * tint.a * _sp_internal.opacity * _sp_internal.shader_opacity);
}
