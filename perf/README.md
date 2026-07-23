# Spottedcat performance suite

Spottedcat's internal, unpublished performance suite lives in `perf/`. Files under `examples/` demonstrate APIs and are not benchmark targets.

## Coverage

| Benchmark | Scenario | Primary pressure |
| --- | --- | --- |
| `cpu` | Matrix multiplication | Transform math |
| `cpu` | 10,000 transform updates | Per-frame simulation |
| `cpu` | Shader option writes | Uniform preparation |
| `cpu` | Draw option construction | Command setup |
| `cpu` | 1,000 text changes | Dynamic text update |
| `cpu` | OBJ grid parsing | Model import with `model-3d` |
| `render_2d` | `sprite_batch` | 20,000 sprites sharing one texture |
| `render_2d` | `sprite_state_changes` | 20,000 sprites alternating 64 textures |
| `render_2d` | `text_cached` | 800 stable multilingual text objects |
| `render_2d` | `text_dynamic` | 800 text objects changed every update |
| `render_2d` | `offscreen` | 8,000 sprites across eight render targets |
| `render_3d` | `render_state` | Opaque/transparent draws and material changes |
| `render_3d` | `shader_switches` | Alternating default and custom model shaders |
| `render_3d` | `instancing` | 10,000 animated instances |
| `render_3d` | `offscreen` | 3D passes across eight render targets |

The renderer profiler records frame pacing, scene update/draw CPU time, resource preparation, render-target, shadow, main 3D, overlay, and present CPU time. On adapters supporting `wgpu::Features::TIMESTAMP_QUERY`, it asynchronously reads timestamps around each render pass and reports the GPU frame span from the earliest valid start to the latest valid end. This avoids double-counting overlapping passes; copy commands outside those timestamped passes are not included.

Optional low-frequency RSS sampling records process memory without adding a query to the timed render path.

## Run the suite

Use a release build, close unrelated GPU-heavy applications, connect laptops to power, and keep display resolution and power mode unchanged between runs.

Quick smoke run:

```bash
make perf-quick
```

Full baseline run, with 300 warm-up frames and 1,800 measured frames per render scenario:

```bash
make perf
```

Choose an explicit output directory for a reusable baseline:

```bash
bash scripts/run_performance.sh full target/perf/baseline-macos-m3
```

Run an individual CPU benchmark:

```bash
cargo bench --manifest-path perf/Cargo.toml --target-dir target \
  --bench cpu --features model-3d -- \
  --output target/perf/cpu.csv
```

Run one render benchmark with automatic termination:

```bash
SPOT_PERF_SCENARIO=sprite_batch \
SPOT_PROFILE_RENDER=1 \
SPOT_PROFILE_WARMUP_FRAMES=120 \
SPOT_PROFILE_SAMPLE_FRAMES=600 \
SPOT_PROFILE_EXIT_AFTER_SAMPLE=1 \
SPOT_PROFILE_CSV=target/perf/sprite.frames.csv \
SPOT_PROFILE_SUMMARY=target/perf/sprite.summary.csv \
SPOT_PRESENT_MODE=auto_no_vsync \
cargo bench --manifest-path perf/Cargo.toml --target-dir target --bench render_2d
```

Render benchmarks open a native window and require a desktop session; they are not headless unit tests.

## Compare against a baseline

Create baseline and candidate runs on the same machine under the same conditions, then run:

```bash
target/release/spot-perf-compare \
  target/perf/baseline-macos-m3 \
  target/perf/candidate-macos-m3 \
  --tolerance 10
```

The command exits non-zero if a scenario's P95 frame interval, P95 engine CPU time, P95 non-wait work time, P95 GPU time, peak RSS, or a CPU microbenchmark's P95 time regresses beyond the tolerance. Missing scenarios also fail. Label baselines with hardware, OS, power mode, resolution, and present mode because results are machine-specific.

## Outputs

Each render scenario produces:

- `*.frames.csv`: one row per sampled frame for plots and detailed analysis
- `*.summary.csv`: mean, P50, P95, P99, and maximum metrics used by the comparator

The CPU suite produces `cpu.csv` with calibrated iterations, mean/P50/P95/P99 nanoseconds, and throughput. GPU fields remain empty when timestamp queries are unsupported; other metrics remain valid.

## Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `SPOT_PROFILE_RENDER` | off | Enable profiling |
| `SPOT_PROFILE_GPU` | on while profiling | Request asynchronous GPU timestamps |
| `SPOT_PROFILE_MEMORY` | off | Enable background RSS sampling |
| `SPOT_PROFILE_MEMORY_INTERVAL_MS` | `1000` | RSS sampling interval |
| `SPOT_PROFILE_WARMUP_FRAMES` | `0` | Frames discarded before measurement |
| `SPOT_PROFILE_SAMPLE_FRAMES` | `0` | Frames to sample; zero means until exit |
| `SPOT_PROFILE_REPORT_EVERY` | `120` | Console reporting interval |
| `SPOT_PROFILE_MAX_SAMPLES` | `36000` | In-memory frame ring capacity |
| `SPOT_PROFILE_EXIT_AFTER_SAMPLE` | off | Quit after the sample count |
| `SPOT_PROFILE_CSV` | unset | Per-frame CSV path |
| `SPOT_PROFILE_SUMMARY` | unset | Summary CSV path |
| `SPOT_PROFILE_SCENARIO` | `unnamed` | Scenario label written to output |
| `SPOT_PRESENT_MODE` | engine default | `auto_no_vsync`, `immediate`, `mailbox`, `fifo`, or `auto_vsync` |
| `SPOT_PERF_SCENARIO` | benchmark-specific | Render workload to run |
| `SPOT_PERF_OBJECTS` | scenario-specific | Override workload size |
| `SPOT_PERF_UPDATE_HZ` | `60` | Fixed update frequency for render benchmarks |
| `SPOT_CPU_BENCH_SAMPLES` | `50` | CPU sample count |
| `SPOT_CPU_BENCH_SAMPLE_MS` | `10` | Target duration of each CPU sample |

Use `auto_no_vsync` for maximum-throughput comparisons and `auto_vsync` or `fifo` for user-visible frame-pacing tests. With VSync, a frame interval near 16.67 ms at 60 Hz is expected; compare engine work and GPU time rather than treating the display wait as a rendering regression.

After the suite identifies a regression, use platform profilers for call stacks and driver-level detail—for example Time Profiler, Allocations, and Metal System Trace on macOS.
