@group(0) @binding(0)
var tex: texture_2d<f32>;

@group(0) @binding(1)
var samp: sampler;

// User shader globals - @group(1) for user
@group(1) @binding(0)
var<uniform> user_globals: array<vec4<f32>, 16>;

// Engine globals - @group(2) for system use
// globals[0].xy = [2.0/logical_w, 2.0/logical_h] (sw_inv_2, sh_inv_2)
// globals[0].zw = [1.0/logical_w, 1.0/logical_h] (sw_inv, sh_inv)
struct EngineGlobals {
    screen: vec4<f32>,
    opacity: f32,
    shader_opacity: f32,
    scale_factor: f32,
    _padding: f32,
};

@group(2) @binding(0)
var<uniform> _sp_internal: EngineGlobals;

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


@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    
    // Triangle Strip Quad: BL, BR, TL, TR
    var pos_arr = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0,  1.0)
    );
    // UVs follow pos: (0,1), (1,1), (0,0), (1,0)
    var uv_arr = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0)
    );
    
    let local_pos = pos_arr[in.vertex_index];
    let uv = uv_arr[in.vertex_index];

    // MVP calculation moved to GPU
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
    
    // local_uv is always 0..1 within the quad
    out.local_uv = uv;

    // UVs: u = u0 + uv.x * w, v = v0 + uv.y * h
    out.uv = vec2<f32>(
        in.uv_rect.x + uv.x * in.uv_rect.z,
        in.uv_rect.y + uv.y * in.uv_rect.w
    );

    // uv_scale is the atlas-space size of this image region
    out.uv_scale = in.uv_rect.zw;

    // uv_scale is the atlas-space size of this image region
    out.uv_scale = in.uv_rect.zw;

    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, in.uv);
    let opacity = _sp_internal.opacity * _sp_internal.shader_opacity;
    var color = vec4<f32>(c.rgb, c.a * opacity);
    return color;
}
