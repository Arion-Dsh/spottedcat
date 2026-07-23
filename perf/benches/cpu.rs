use spottedcat::{DrawOption, Pt, ShaderOpts, Text};
use std::fs::File;
use std::hint::black_box;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug)]
struct BenchResult {
    name: &'static str,
    samples: usize,
    iterations: u64,
    items_per_iteration: u64,
    mean_ns: f64,
    p50_ns: f64,
    p95_ns: f64,
    p99_ns: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = output_path();
    let sample_count = std::env::var("SPOT_CPU_BENCH_SAMPLES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(50usize)
        .max(10);
    let target_sample_time = Duration::from_millis(
        std::env::var("SPOT_CPU_BENCH_SAMPLE_MS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(10u64)
            .max(1),
    );

    eprintln!(
        "[spot][cpu-bench] samples={sample_count} target_sample_ms={}",
        target_sample_time.as_millis()
    );

    let mut results = Vec::new();

    let matrix_a = spottedcat::math::mat4::from_rotation([0.31, 0.73, 1.17]);
    let matrix_b = spottedcat::math::mat4::from_translation([10.0, -4.0, 19.0]);
    results.push(run_benchmark(
        "mat4_multiply",
        1,
        sample_count,
        target_sample_time,
        || black_box(spottedcat::math::mat4::multiply(matrix_a, matrix_b)),
    ));

    let mut transforms = vec![[[0.0f32; 4]; 4]; 10_000];
    let mut phase = 0.0f32;
    results.push(run_benchmark(
        "transform_update_10000",
        transforms.len() as u64,
        sample_count,
        target_sample_time,
        || {
            phase += 0.0001;
            for (index, transform) in transforms.iter_mut().enumerate() {
                let angle = phase + index as f32 * 0.001;
                let (sin, cos) = angle.sin_cos();
                *transform = [
                    [cos, 0.0, -sin, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [sin, 0.0, cos, 0.0],
                    [index as f32 * 0.01, angle.sin() * 2.0, -20.0, 1.0],
                ];
            }
            black_box(transforms[phase.to_bits() as usize % transforms.len()])
        },
    ));

    let mut shader_opts = ShaderOpts::default();
    let mut shader_tick = 0.0f32;
    results.push(run_benchmark(
        "shader_opts_16_vec4_writes",
        16,
        sample_count,
        target_sample_time,
        || {
            shader_tick += 0.001;
            for index in 0..16 {
                shader_opts.set_vec4(index, [shader_tick, index as f32, 0.5, 1.0]);
            }
            black_box(shader_opts.as_bytes()[0])
        },
    ));

    let mut draw_tick = 0.0f32;
    results.push(run_benchmark(
        "draw_option_build_1000",
        1_000,
        sample_count,
        target_sample_time,
        || {
            let mut sum = 0.0;
            draw_tick += 0.001;
            for index in 0..1_000 {
                let option = DrawOption::default()
                    .with_position([
                        Pt::from((index % 100) as f32),
                        Pt::from((index / 100) as f32),
                    ])
                    .with_rotation(draw_tick + index as f32 * 0.01)
                    .with_scale([1.0 + index as f32 * 0.0001, 1.0]);
                sum += option.rotation() + option.position()[0].as_f32();
            }
            black_box(sum)
        },
    ));

    let mut texts: Vec<Text> = (0..1_000)
        .map(|index| Text::new(format!("text {index}"), 1))
        .collect();
    let mut text_tick = 0u64;
    results.push(run_benchmark(
        "text_content_update_1000",
        texts.len() as u64,
        sample_count,
        target_sample_time,
        || {
            text_tick = text_tick.wrapping_add(1);
            for (index, text) in texts.iter_mut().enumerate() {
                text.set_content(format!("动态 text {index} {}", text_tick % 10_000));
            }
            black_box(text_tick)
        },
    ));

    #[cfg(feature = "model-3d")]
    {
        let obj = make_grid_obj(32);
        results.push(run_benchmark(
            "obj_parse_grid_32",
            32 * 32 * 2,
            sample_count,
            target_sample_time,
            || {
                let parsed = spottedcat::utils::obj::parse_obj_data(black_box(obj.as_bytes()))
                    .expect("generated OBJ should parse");
                black_box((parsed.0.len(), parsed.1.len()))
            },
        ));
    }

    write_results(&output, &results)?;
    eprintln!("[spot][cpu-bench] wrote {}", output.display());
    Ok(())
}

fn run_benchmark<T>(
    name: &'static str,
    items_per_iteration: u64,
    sample_count: usize,
    target_sample_time: Duration,
    mut operation: impl FnMut() -> T,
) -> BenchResult {
    let warmup_until = Instant::now() + Duration::from_millis(100);
    while Instant::now() < warmup_until {
        black_box(operation());
    }

    let mut iterations = 1u64;
    loop {
        let started = Instant::now();
        for _ in 0..iterations {
            black_box(operation());
        }
        let elapsed = started.elapsed();
        if elapsed >= target_sample_time || iterations >= 1 << 30 {
            break;
        }
        let estimated = (target_sample_time.as_secs_f64() / elapsed.as_secs_f64().max(1e-9))
            .ceil()
            .clamp(2.0, 16.0) as u64;
        iterations = iterations.saturating_mul(estimated);
    }

    let mut ns_per_iteration = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        let started = Instant::now();
        for _ in 0..iterations {
            black_box(operation());
        }
        ns_per_iteration.push(started.elapsed().as_nanos() as f64 / iterations as f64);
    }
    ns_per_iteration.sort_by(f64::total_cmp);
    let result = BenchResult {
        name,
        samples: sample_count,
        iterations,
        items_per_iteration,
        mean_ns: ns_per_iteration.iter().sum::<f64>() / ns_per_iteration.len() as f64,
        p50_ns: percentile(&ns_per_iteration, 0.50),
        p95_ns: percentile(&ns_per_iteration, 0.95),
        p99_ns: percentile(&ns_per_iteration, 0.99),
    };
    eprintln!(
        "[spot][cpu-bench] {:<30} mean={:>10.2}ns p95={:>10.2}ns iterations={}",
        result.name, result.mean_ns, result.p95_ns, result.iterations
    );
    result
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let rank = p.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    sorted[lower] + (sorted[upper] - sorted[lower]) * (rank - lower as f64)
}

fn output_path() -> PathBuf {
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--output" {
            return args
                .next()
                .map(PathBuf::from)
                .expect("--output requires a path");
        }
    }
    std::env::var_os("SPOT_CPU_BENCH_CSV")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/perf/cpu.csv"))
}

fn write_results(path: &Path, results: &[BenchResult]) -> std::io::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut writer = BufWriter::new(File::create(path)?);
    writeln!(
        writer,
        "benchmark,samples,iterations_per_sample,items_per_iteration,mean_ns,p50_ns,p95_ns,p99_ns,throughput_items_per_sec"
    )?;
    for result in results {
        let throughput = result.items_per_iteration as f64 * 1_000_000_000.0 / result.mean_ns;
        writeln!(
            writer,
            "{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.3}",
            result.name,
            result.samples,
            result.iterations,
            result.items_per_iteration,
            result.mean_ns,
            result.p50_ns,
            result.p95_ns,
            result.p99_ns,
            throughput
        )?;
    }
    writer.flush()
}

#[cfg(feature = "model-3d")]
fn make_grid_obj(size: usize) -> String {
    let mut obj = String::new();
    for y in 0..=size {
        for x in 0..=size {
            obj.push_str(&format!("v {x} 0 {y}\nvt 0 0\nvn 0 1 0\n"));
        }
    }
    for y in 0..size {
        for x in 0..size {
            let a = y * (size + 1) + x + 1;
            let b = a + 1;
            let c = a + size + 2;
            let d = a + size + 1;
            obj.push_str(&format!(
                "f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c} {d}/{d}/{d}\n"
            ));
        }
    }
    obj
}
