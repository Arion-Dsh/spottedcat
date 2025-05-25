// Uniform 结构体，需要与 Rust 中的 ScreenAndPQRSUniform 匹配
struct ScreenAndPQRSUniform {
    screen_size: vec2<f32>,         // 屏幕的尺寸 (像素)
    position: vec2<f32>,           // 矩形左上角的位置 (像素坐标)
    size: vec2<f32>,          // 矩形的大小 (像素坐标)
    scale: vec2<f32>,         // 缩放因子
    rotation_angle: f32,      // 旋转角度 (弧度)
    overall_alpha: f32,              // 整体的透明度
    z_index: f32,                   // Z 深度值 (影响绘制顺序和深度测试)
    _padding: f32,                  // 填充（为了内存对齐）
};

// 统一声明 TextureUniform 和 ColorTransform 结构体，因为它们在这里被绑定
struct TextureUniform {
    t_size: vec2<f32>,      // 纹理的原始尺寸 (像素)
    uv_offset: vec2<f32>,   // 纹理 UV 坐标的偏移量
    uv_size: vec2<f32>,     // 纹理 UV 坐标的有效区域大小
    _padding: vec2<f32>,
};

// 颜色变换相关的 Uniform 结构体
struct ColorUniform {
    matrix: mat4x4<f32>,    // 颜色变换矩阵
    transform: vec4<f32>,   // 颜色加法向量 (值在 0-255 之间)
    use_uniform: f32,        // 1.0 表示应用 matrix 变换，0.0 表示跳过
};

// 绑定组 1，绑定点 0：用于屏幕和矩形参数
@group(1) @binding(0)
var<uniform> u_screen_and_pqrs: ScreenAndPQRSUniform;

// 顶点输入结构：定义了从 CPU 传入的每个顶点的属性
struct VertexInput {
    @location(0) position: vec2<f32>, // 归一化设备坐标 (NDC)，例如 (-1,-1) 到 (1,1)
    @location(1) tex_coords: vec2<f32>, // 纹理坐标，例如 (0,0) 到 (1,1)
};

// 顶点输出结构：定义了从顶点着色器传递到片段着色器的插值数据
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) alpha: f32, // 将整体透明度传递给片元着色器
    @location(2) screen_size: vec2<f32>, // 屏幕尺寸
    @location(7) overall_alpha: f32, // 整体透明度
};


@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    // 1. 将标准化设备坐标 ([-1, 1]) 转换为相对于矩形左上角的像素坐标
    // 原始矩形顶点是 [-1, 1] 到 [1, -1]
    // 假设原始的 [-1, 1] 是左上角，[1, -1] 是右下角
    // 那么，通过 (in.position + 1.0) / 2.0 得到 [0, 1] 范围的相对坐标
    // 再乘以矩形自身的尺寸 (u_screen_and_pqrs.size) 得到像素坐标
    let local_pixel_pos = (in.position * 0.5 + 0.5) * u_screen_and_pqrs.size;

    // 2. 缩放 (以左上角为基准)
    // 直接乘以缩放因子
    let scaled_pos = local_pixel_pos * u_screen_and_pqrs.scale;

    // 3. 旋转 (以左上角为基准)
    let cos_angle = cos(u_screen_and_pqrs.rotation_angle);
    let sin_angle = sin(u_screen_and_pqrs.rotation_angle);
    let rotated_pos_x = scaled_pos.x * cos_angle - scaled_pos.y * sin_angle;
    let rotated_pos_y = scaled_pos.x * sin_angle + scaled_pos.y * cos_angle;
    let rotated_pos = vec2<f32>(rotated_pos_x, rotated_pos_y);

    // 4. 移动 (加上矩形左上角的位置)
    let final_pixel_pos = rotated_pos + u_screen_and_pqrs.position;

    // 5. 将像素坐标转换为标准化设备坐标 (NDC)
    // NDC 范围是 [-1, 1]，所以需要进行如下转换：
    // (2.0 * final_pixel_pos / screen_size) - 1.0
    // 注意 Y 轴在屏幕坐标系通常是向下为正，而在 NDC 中是向上为正，所以 Y 轴需要反转
    let ndc_x = (2.0 * final_pixel_pos.x / u_screen_and_pqrs.screen_size.x) - 1.0;
    let ndc_y = 1.0 - (2.0 * final_pixel_pos.y / u_screen_and_pqrs.screen_size.y); // 注意 Y 轴反转

    // 6. 设置 Z 深度
    let z_ndc = u_screen_and_pqrs.z_index; // 假设 z_index 已经在 [0, 1] 范围内或者可以被映射到这个范围

    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.tex_coords = in.tex_coords;
    out.overall_alpha = u_screen_and_pqrs.overall_alpha;
    out.screen_size = u_screen_and_pqrs.screen_size;

    return out;
}

// 绑定组 0，绑定点 0：纹理
@group(0) @binding(0)
var t_texture: texture_2d<f32>;
// 绑定组 0，绑定点 1：采样器
@group(0) @binding(1)
var s_texture_sampler: sampler;
// 绑定组 0，绑定点 2：纹理参数 Uniform
@group(0) @binding(2)
var<uniform>u_texture_params: TextureUniform;
// 绑定组 0，绑定点 3：颜色变换参数 Uniform
@group(0) @binding(3)
var<uniform>u_color_uniform: ColorUniform;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let overall_alpha = in.overall_alpha;

    //归一化
    let normalized_uv_offset = u_texture_params.uv_offset / u_texture_params.t_size;
    let normalized_uv_size = u_texture_params.uv_size / u_texture_params.t_size;

    let adjusted_uv = normalized_uv_offset + (in.tex_coords * normalized_uv_size);
    var color = textureSample(t_texture, s_texture_sampler, adjusted_uv);

    if (u_color_uniform.use_uniform == 1.0) {
        color = u_color_uniform.matrix * color;
        let normalized_color_add = u_color_uniform.transform / 255.0;
        color = color + normalized_color_add;
    }

    color.a = color.a * overall_alpha;
    return color;
}