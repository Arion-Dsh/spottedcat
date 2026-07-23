use spottedcat::{
    Context, DrawOption, DrawOption3D, Image, Model, ModelShaderTemplate, Pt, ShaderOpts, Spot,
    Texture, WindowConfig, register_model_shader_template,
};
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    RenderState,
    ShaderSwitches,
    Instancing,
    Offscreen,
}

impl Scenario {
    fn from_env() -> Self {
        match std::env::var("SPOT_PERF_SCENARIO").as_deref() {
            Ok("render_state") => Self::RenderState,
            Ok("shader_switches") => Self::ShaderSwitches,
            Ok("instancing") => Self::Instancing,
            Ok("offscreen") => Self::Offscreen,
            Ok(other) => panic!(
                "unknown SPOT_PERF_SCENARIO={other}; expected render_state, shader_switches, instancing, or offscreen"
            ),
            Err(_) => Self::RenderState,
        }
    }

    fn default_objects(self) -> usize {
        match self {
            Self::RenderState | Self::ShaderSwitches => 1_024,
            Self::Instancing => 10_000,
            Self::Offscreen => 256,
        }
    }
}

struct Performance3D {
    scenario: Scenario,
    object_count: usize,
    models: Vec<Model>,
    transparent_plane: Model,
    instanced_cube: Model,
    transforms: Vec<[[f32; 4]; 4]>,
    targets: Vec<Image>,
    shader_id: u32,
    time: f32,
}

fn checker_texture(ctx: &mut Context, index: usize) -> Image {
    let colors = [
        ([255, 96, 96, 255], [160, 32, 32, 255]),
        ([96, 255, 160, 255], [24, 120, 64, 255]),
        ([96, 168, 255, 255], [24, 56, 144, 255]),
        ([255, 220, 96, 255], [160, 120, 24, 255]),
    ];
    let (a, b) = colors[index % colors.len()];
    let rgba = [
        a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3], b[0], b[1], b[2], b[3], a[0], a[1], a[2],
        a[3],
    ];
    Image::new(ctx, Pt::from(2.0), Pt::from(2.0), &rgba).expect("checker texture")
}

fn make_transforms(count: usize) -> Vec<[[f32; 4]; 4]> {
    let columns = (count as f32).sqrt().ceil() as usize;
    (0..count)
        .map(|index| {
            let x = index % columns;
            let y = index / columns;
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [
                    (x as f32 - columns as f32 * 0.5) * 1.2,
                    (y as f32 - columns as f32 * 0.5) * 1.2,
                    -50.0,
                    1.0,
                ],
            ]
        })
        .collect()
}

impl Spot for Performance3D {
    fn initialize(ctx: &mut Context) -> Self {
        let scenario = Scenario::from_env();
        let object_count = std::env::var("SPOT_PERF_OBJECTS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| scenario.default_objects())
            .clamp(1, 65_536);

        let materials: Vec<_> = (0..4).map(|index| checker_texture(ctx, index)).collect();
        let models = vec![
            spottedcat::model::create_cube(ctx, 0.8)
                .expect("cube")
                .with_material(materials[0]),
            spottedcat::model::create_sphere(ctx, 0.5)
                .expect("sphere")
                .with_material(materials[1]),
            spottedcat::model::create_cube(ctx, 0.55)
                .expect("cube")
                .with_material(materials[2]),
            spottedcat::model::create_sphere(ctx, 0.35)
                .expect("sphere")
                .with_material(materials[3]),
        ];
        let transparent_plane = spottedcat::model::create_plane(ctx, 1.8, 1.8)
            .expect("plane")
            .with_material(materials[2]);
        let instanced_cube = spottedcat::model::create_cube(ctx, 0.35)
            .expect("instanced cube")
            .with_material(materials[3]);
        let shader_id = register_model_shader_template(
            ctx,
            ModelShaderTemplate::new().with_fragment_body(
                r#"
let pulse = 0.6 + user_globals[2].x * 0.4;
let rim = pow(1.0 - max(dot(normalize(in.normal), normalize(scene.camera_pos.xyz - in.world_pos)), 0.0), 3.0);
return vec4<f32>(src.rgb * pulse + vec3<f32>(rim * 0.35), src.a * model_globals.extra.x);
"#,
            ),
        );
        let transforms = make_transforms(object_count);
        let targets = if matches!(scenario, Scenario::Offscreen) {
            (0..8)
                .map(|_| Texture::new_render_target(ctx, Pt::from(512.0), Pt::from(512.0)).view())
                .collect()
        } else {
            Vec::new()
        };
        eprintln!(
            "[spot][perf-scene] scenario={scenario:?} objects={object_count} targets={}",
            targets.len()
        );
        Self {
            scenario,
            object_count,
            models,
            transparent_plane,
            instanced_cube,
            transforms,
            targets,
            shader_id,
            time: 0.0,
        }
    }

    fn update(&mut self, ctx: &mut Context, dt: Duration) {
        self.time += dt.as_secs_f32();
        spottedcat::set_camera_pos(ctx, [0.0, 12.0, 28.0]);
        spottedcat::set_camera_target(ctx, 0.0, 0.0, -24.0);

        if matches!(self.scenario, Scenario::Instancing) {
            let columns = (self.object_count as f32).sqrt().ceil() as usize;
            for (index, transform) in self.transforms.iter_mut().enumerate() {
                let x = index % columns;
                let y = index / columns;
                let angle = self.time + index as f32 * 0.001;
                let (sin, cos) = angle.sin_cos();
                *transform = [
                    [cos, 0.0, -sin, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [sin, 0.0, cos, 0.0],
                    [
                        (x as f32 - columns as f32 * 0.5) * 1.2,
                        (y as f32 - columns as f32 * 0.5) * 1.2,
                        -50.0 + (angle * 3.0).sin() * 2.0,
                        1.0,
                    ],
                ];
            }
        }
    }

    fn draw(&mut self, ctx: &mut Context, screen: Image) {
        match self.scenario {
            Scenario::RenderState | Scenario::ShaderSwitches => {
                let columns = (self.object_count as f32).sqrt().ceil() as usize;
                for index in 0..self.object_count {
                    let x = index % columns;
                    let y = index / columns;
                    let opts = DrawOption3D::default()
                        .with_position([
                            (x as f32 - columns as f32 * 0.5) * 1.5,
                            (y as f32 - columns as f32 * 0.5) * 1.15,
                            -6.0 - (index % 7) as f32 * 1.2,
                        ])
                        .with_rotation([self.time * 0.3, self.time + index as f32 * 0.02, 0.0]);
                    let model = &self.models[(index * 5 + y * 3) % self.models.len()];
                    if matches!(self.scenario, Scenario::ShaderSwitches) && index % 2 == 1 {
                        let mut shader_opts = ShaderOpts::default();
                        shader_opts.set_vec4(2, [0.5 + x as f32 / columns as f32, 0.0, 0.0, 0.0]);
                        spottedcat::model::draw_with_shader(
                            ctx,
                            screen,
                            model,
                            self.shader_id,
                            opts,
                            shader_opts,
                            None,
                        );
                    } else {
                        screen.draw(ctx, model, opts);
                    }
                }
                if matches!(self.scenario, Scenario::RenderState) {
                    for index in 0..32 {
                        screen.draw(
                            ctx,
                            &self.transparent_plane,
                            DrawOption3D::default()
                                .with_position([
                                    -16.0 + index as f32,
                                    0.0,
                                    -2.0 - index as f32 * 0.2,
                                ])
                                .with_opacity(0.35),
                        );
                    }
                }
            }
            Scenario::Instancing => {
                spottedcat::model::draw_instanced(
                    ctx,
                    screen,
                    &self.instanced_cube,
                    DrawOption3D::default(),
                    &self.transforms,
                );
            }
            Scenario::Offscreen => {
                for (target_index, target) in self.targets.iter().copied().enumerate() {
                    let columns = (self.object_count as f32).sqrt().ceil() as usize;
                    for index in 0..self.object_count {
                        let x = index % columns;
                        let y = index / columns;
                        target.draw(
                            ctx,
                            &self.models[(index + target_index) % self.models.len()],
                            DrawOption3D::default()
                                .with_position([
                                    (x as f32 - columns as f32 * 0.5) * 1.3,
                                    (y as f32 - columns as f32 * 0.5) * 1.0,
                                    -4.0 - ((index + target_index) % 8) as f32 * 1.2,
                                ])
                                .with_rotation([
                                    self.time * 0.3,
                                    self.time + index as f32 * 0.02,
                                    0.0,
                                ]),
                        );
                    }
                    screen.draw(
                        ctx,
                        &target,
                        DrawOption::default()
                            .with_position([
                                Pt::from((target_index % 4) as f32 * 320.0),
                                Pt::from((target_index / 4) as f32 * 360.0),
                            ])
                            .with_scale([0.625, 0.625]),
                    );
                }
            }
        }
    }
}

fn main() {
    unsafe {
        std::env::set_var("SPOT_PROFILE_RENDER", "1");
    }
    let update_hz = std::env::var("SPOT_PERF_UPDATE_HZ")
        .map(|value| {
            value
                .parse::<u32>()
                .expect("SPOT_PERF_UPDATE_HZ must be an integer")
        })
        .unwrap_or(60);
    spottedcat::run::<Performance3D>(WindowConfig {
        title: "Spottedcat 3D Render Benchmark".to_string(),
        width: Pt::from(1280.0),
        height: Pt::from(720.0),
        update_hz,
        ..Default::default()
    });
}
