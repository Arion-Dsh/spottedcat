//! Configurable CPU/GPU frame profiling and present mode selection utilities.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::thread;
use std::time::Duration;
use std::time::Instant;

static PROFILE_RENDER: OnceLock<bool> = OnceLock::new();
static PROFILE_STATE: OnceLock<Mutex<RenderProfiler>> = OnceLock::new();
static PROFILE_FRAME_ID: AtomicU64 = AtomicU64::new(0);
static PENDING_SCENE_TIMES: OnceLock<Mutex<SceneTimes>> = OnceLock::new();
static GPU_READBACK_WARNED: AtomicBool = AtomicBool::new(false);

const FRAME_CSV_HEADER: &str = "frame,sample,frame_interval_ms,engine_ms,wait_ms,work_ms,prepare_ms,targets_ms,shadow_ms,main3d_ms,overlay_ms,present_ms,update_ms,draw_ms,gpu_ms,rss_mb";
const SUMMARY_CSV_HEADER: &str = "scenario,samples,gpu_samples,mean_frame_interval_ms,p50_frame_interval_ms,p95_frame_interval_ms,p99_frame_interval_ms,max_frame_interval_ms,mean_engine_ms,p50_engine_ms,p95_engine_ms,p99_engine_ms,max_engine_ms,mean_work_ms,p95_work_ms,p99_work_ms,mean_update_ms,p95_update_ms,mean_draw_ms,p95_draw_ms,mean_gpu_ms,p50_gpu_ms,p95_gpu_ms,p99_gpu_ms,max_gpu_ms,mean_rss_mb,max_rss_mb";
const MAX_GPU_TIMESTAMP_QUERIES: u32 = 512;
const GPU_TIMESTAMP_BUFFER_SIZE: u64 = MAX_GPU_TIMESTAMP_QUERIES as u64 * 8;

#[derive(Clone, Debug)]
struct ProfileConfig {
    warmup_frames: u64,
    sample_frames: u64,
    report_every: u64,
    max_samples: usize,
    frame_csv: Option<PathBuf>,
    summary_csv: Option<PathBuf>,
    exit_after_sample: bool,
    scenario: String,
}

impl ProfileConfig {
    fn from_env() -> Self {
        Self {
            warmup_frames: env_u64("SPOT_PROFILE_WARMUP_FRAMES", 0),
            sample_frames: env_u64("SPOT_PROFILE_SAMPLE_FRAMES", 0),
            report_every: env_u64("SPOT_PROFILE_REPORT_EVERY", 120),
            max_samples: env_u64("SPOT_PROFILE_MAX_SAMPLES", 36_000).max(1) as usize,
            frame_csv: env_path("SPOT_PROFILE_CSV"),
            summary_csv: env_path("SPOT_PROFILE_SUMMARY"),
            exit_after_sample: env_bool("SPOT_PROFILE_EXIT_AFTER_SAMPLE", false),
            scenario: std::env::var("SPOT_PROFILE_SCENARIO")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "unnamed".to_string()),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FrameProfileInput {
    pub frame_id: u64,
    pub engine_ms: f64,
    pub wait_ms: f64,
    pub prepare_ms: f64,
    pub targets_ms: f64,
    pub shadow_ms: f64,
    pub main_3d_ms: f64,
    pub overlay_ms: f64,
    pub present_ms: f64,
}

#[derive(Clone, Debug, Default)]
struct FrameSample {
    frame: u64,
    sample: u64,
    frame_interval_ms: f64,
    engine_ms: f64,
    wait_ms: f64,
    work_ms: f64,
    prepare_ms: f64,
    targets_ms: f64,
    shadow_ms: f64,
    main_3d_ms: f64,
    overlay_ms: f64,
    present_ms: f64,
    update_ms: f64,
    draw_ms: f64,
    gpu_ms: Option<f64>,
    rss_mb: Option<f64>,
}

#[derive(Default)]
struct SceneTimes {
    update_ms: f64,
    draw_ms: f64,
}

struct RenderProfiler {
    config: ProfileConfig,
    rendered_frames: u64,
    sampled_frames: u64,
    samples: VecDeque<FrameSample>,
    last_frame_at: Option<Instant>,
    sample_complete: bool,
    warned_capacity: bool,
    memory_sampler: Option<MemorySampler>,
}

struct MemorySampler {
    latest_kb: Arc<AtomicU64>,
}

impl MemorySampler {
    fn start() -> Self {
        let latest_kb = Arc::new(AtomicU64::new(0));
        let output = latest_kb.clone();
        let interval_ms = env_u64("SPOT_PROFILE_MEMORY_INTERVAL_MS", 1_000).max(100);
        let _ = thread::Builder::new()
            .name("spot-profile-memory".to_string())
            .spawn(move || {
                loop {
                    if let Some(kb) = resident_memory_kb() {
                        output.store(kb, Ordering::Relaxed);
                    }
                    thread::sleep(Duration::from_millis(interval_ms));
                }
            });
        Self { latest_kb }
    }

    fn rss_mb(&self) -> Option<f64> {
        let kb = self.latest_kb.load(Ordering::Relaxed);
        (kb > 0).then(|| kb as f64 / 1024.0)
    }
}

impl RenderProfiler {
    fn new() -> Self {
        let config = ProfileConfig::from_env();
        eprintln!(
            "[spot][profile] enabled scenario={} warmup_frames={} sample_frames={} report_every={} gpu_requested={}",
            config.scenario,
            config.warmup_frames,
            config.sample_frames,
            config.report_every,
            gpu_profiling_requested()
        );
        let memory_sampler = env_bool("SPOT_PROFILE_MEMORY", false).then(MemorySampler::start);
        Self {
            config,
            rendered_frames: 0,
            sampled_frames: 0,
            samples: VecDeque::new(),
            last_frame_at: None,
            sample_complete: false,
            warned_capacity: false,
            memory_sampler,
        }
    }

    fn record(&mut self, input: FrameProfileInput, scene: SceneTimes) -> bool {
        let now = Instant::now();
        let frame_interval_ms = self
            .last_frame_at
            .replace(now)
            .map(|last| now.duration_since(last).as_secs_f64() * 1000.0)
            .unwrap_or(input.engine_ms);
        self.rendered_frames += 1;

        if self.rendered_frames <= self.config.warmup_frames || self.sample_complete {
            if self.rendered_frames == self.config.warmup_frames {
                eprintln!("[spot][profile] warmup complete; sampling starts on the next frame");
            }
            return false;
        }

        self.sampled_frames += 1;
        let sample = FrameSample {
            frame: input.frame_id,
            sample: self.sampled_frames,
            frame_interval_ms,
            engine_ms: input.engine_ms,
            wait_ms: input.wait_ms,
            work_ms: (input.engine_ms - input.wait_ms).max(0.0),
            prepare_ms: input.prepare_ms,
            targets_ms: input.targets_ms,
            shadow_ms: input.shadow_ms,
            main_3d_ms: input.main_3d_ms,
            overlay_ms: input.overlay_ms,
            present_ms: input.present_ms,
            update_ms: scene.update_ms,
            draw_ms: scene.draw_ms,
            gpu_ms: None,
            rss_mb: self.memory_sampler.as_ref().and_then(MemorySampler::rss_mb),
        };

        if self.samples.len() == self.config.max_samples {
            self.samples.pop_front();
            if !self.warned_capacity {
                self.warned_capacity = true;
                eprintln!(
                    "[spot][profile] sample ring reached {}; oldest samples will be discarded",
                    self.config.max_samples
                );
            }
        }
        self.samples.push_back(sample);

        if self.config.report_every > 0
            && self.sampled_frames.is_multiple_of(self.config.report_every)
        {
            self.print_report(false);
        }

        if self.config.sample_frames > 0 && self.sampled_frames >= self.config.sample_frames {
            self.sample_complete = true;
            return self.config.exit_after_sample;
        }
        false
    }

    fn record_gpu(&mut self, frame_id: u64, gpu_ms: f64) {
        if let Some(sample) = self
            .samples
            .iter_mut()
            .rev()
            .find(|sample| sample.frame == frame_id)
        {
            sample.gpu_ms = Some(gpu_ms);
        }
    }

    fn print_report(&self, final_report: bool) {
        if self.samples.is_empty() {
            return;
        }
        let frame = values(&self.samples, |s| Some(s.frame_interval_ms));
        let engine = values(&self.samples, |s| Some(s.engine_ms));
        let work = values(&self.samples, |s| Some(s.work_ms));
        let update = values(&self.samples, |s| Some(s.update_ms));
        let draw = values(&self.samples, |s| Some(s.draw_ms));
        let gpu = values(&self.samples, |s| s.gpu_ms);
        let rss = values(&self.samples, |s| s.rss_mb);
        let label = if final_report { "final" } else { "report" };
        eprintln!(
            "[spot][profile][{}] samples={} gpu_samples={} frame_mean={:.2}ms frame_p50={:.2}ms frame_p95={:.2}ms frame_p99={:.2}ms frame_max={:.2}ms engine_mean={:.2}ms engine_p95={:.2}ms work_mean={:.2}ms work_p95={:.2}ms update_mean={:.2}ms draw_mean={:.2}ms gpu_mean={} gpu_p95={} rss_max={}",
            label,
            self.samples.len(),
            gpu.len(),
            mean(&frame),
            percentile(&frame, 0.50),
            percentile(&frame, 0.95),
            percentile(&frame, 0.99),
            max(&frame),
            mean(&engine),
            percentile(&engine, 0.95),
            mean(&work),
            percentile(&work, 0.95),
            mean(&update),
            mean(&draw),
            optional_ms(mean_optional(&gpu)),
            optional_ms(percentile_optional(&gpu, 0.95)),
            rss.last()
                .map(|_| format!("{:.1}MB", max(&rss)))
                .unwrap_or_else(|| "n/a".to_string()),
        );
    }

    fn write_outputs(&self) -> std::io::Result<()> {
        if let Some(path) = self.config.frame_csv.as_deref() {
            write_frame_csv(path, &self.samples)?;
            eprintln!("[spot][profile] wrote frame samples to {}", path.display());
        }
        if let Some(path) = self.config.summary_csv.as_deref() {
            write_summary_csv(path, &self.config.scenario, &self.samples)?;
            eprintln!("[spot][profile] wrote summary to {}", path.display());
        }
        Ok(())
    }
}

pub(crate) fn render_profiling_enabled() -> bool {
    *PROFILE_RENDER.get_or_init(|| {
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            false
        }
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        {
            env_bool("SPOT_PROFILE_RENDER", false)
        }
    })
}

pub(crate) fn gpu_profiling_requested() -> bool {
    render_profiling_enabled() && env_bool("SPOT_PROFILE_GPU", true)
}

pub(crate) fn next_render_frame_id() -> u64 {
    PROFILE_FRAME_ID.fetch_add(1, Ordering::Relaxed) + 1
}

pub(crate) fn record_scene_update(elapsed_ms: f64) {
    if !render_profiling_enabled() {
        return;
    }
    let mut times = PENDING_SCENE_TIMES
        .get_or_init(|| Mutex::new(SceneTimes::default()))
        .lock()
        .unwrap();
    times.update_ms += elapsed_ms;
}

pub(crate) fn record_scene_draw(elapsed_ms: f64) {
    if !render_profiling_enabled() {
        return;
    }
    let mut times = PENDING_SCENE_TIMES
        .get_or_init(|| Mutex::new(SceneTimes::default()))
        .lock()
        .unwrap();
    times.draw_ms += elapsed_ms;
}

pub(crate) fn record_render_frame(input: FrameProfileInput) {
    if !render_profiling_enabled() {
        return;
    }
    let scene = PENDING_SCENE_TIMES
        .get_or_init(|| Mutex::new(SceneTimes::default()))
        .lock()
        .map(|mut times| std::mem::take(&mut *times))
        .unwrap_or_default();
    let should_exit = PROFILE_STATE
        .get_or_init(|| Mutex::new(RenderProfiler::new()))
        .lock()
        .map(|mut profiler| profiler.record(input, scene))
        .unwrap_or(false);
    if should_exit {
        crate::scenes::quit();
    }
}

fn record_gpu_frame(frame_id: u64, gpu_ms: f64) {
    if let Some(state) = PROFILE_STATE.get()
        && let Ok(mut profiler) = state.lock()
    {
        profiler.record_gpu(frame_id, gpu_ms);
    }
}

pub(crate) fn finalize_render_profiling() {
    if let Some(state) = PROFILE_STATE.get()
        && let Ok(profiler) = state.lock()
    {
        profiler.print_report(true);
        if let Err(error) = profiler.write_outputs() {
            eprintln!("[spot][profile] failed to write output: {error}");
        }
    }
}

pub(crate) struct GpuTimestampProfiler {
    slots: Vec<GpuTimestampSlot>,
    next_slot: usize,
    period_ns: f64,
    sender: mpsc::Sender<(u64, f64)>,
    receiver: mpsc::Receiver<(u64, f64)>,
}

struct GpuTimestampSlot {
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    read_buffer: wgpu::Buffer,
    busy: Arc<AtomicBool>,
}

pub(crate) struct GpuFrameQuery {
    pub query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    read_buffer: wgpu::Buffer,
    busy: Arc<AtomicBool>,
    sender: mpsc::Sender<(u64, f64)>,
    frame_id: u64,
    period_ns: f64,
    used_queries: u32,
}

impl GpuTimestampProfiler {
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        const SLOT_COUNT: usize = 8;
        let mut slots = Vec::with_capacity(SLOT_COUNT);
        for index in 0..SLOT_COUNT {
            slots.push(GpuTimestampSlot {
                query_set: device.create_query_set(&wgpu::QuerySetDescriptor {
                    label: Some(&format!("spot_profile_timestamp_queries_{index}")),
                    ty: wgpu::QueryType::Timestamp,
                    count: MAX_GPU_TIMESTAMP_QUERIES,
                }),
                resolve_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("spot_profile_timestamp_resolve_{index}")),
                    size: GPU_TIMESTAMP_BUFFER_SIZE,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
                read_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("spot_profile_timestamp_read_{index}")),
                    size: GPU_TIMESTAMP_BUFFER_SIZE,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }),
                busy: Arc::new(AtomicBool::new(false)),
            });
        }
        let (sender, receiver) = mpsc::channel();
        eprintln!(
            "[spot][profile] GPU timestamp profiling enabled (period {:.3}ns)",
            queue.get_timestamp_period()
        );
        Self {
            slots,
            next_slot: 0,
            period_ns: queue.get_timestamp_period() as f64,
            sender,
            receiver,
        }
    }

    pub(crate) fn begin_frame(
        &mut self,
        frame_id: u64,
        device: &wgpu::Device,
    ) -> Option<GpuFrameQuery> {
        let _ = device.poll(wgpu::PollType::Poll);
        self.drain_results();
        for offset in 0..self.slots.len() {
            let index = (self.next_slot + offset) % self.slots.len();
            let slot = &self.slots[index];
            if slot
                .busy
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                self.next_slot = (index + 1) % self.slots.len();
                return Some(GpuFrameQuery {
                    query_set: slot.query_set.clone(),
                    resolve_buffer: slot.resolve_buffer.clone(),
                    read_buffer: slot.read_buffer.clone(),
                    busy: slot.busy.clone(),
                    sender: self.sender.clone(),
                    frame_id,
                    period_ns: self.period_ns,
                    used_queries: 0,
                });
            }
        }
        None
    }

    pub(crate) fn drain_results(&mut self) {
        while let Ok((frame_id, gpu_ms)) = self.receiver.try_recv() {
            record_gpu_frame(frame_id, gpu_ms);
        }
    }

    pub(crate) fn finish(&mut self, device: &wgpu::Device) {
        for _ in 0..=self.slots.len() {
            let _ = device.poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            });
            self.drain_results();
            if self
                .slots
                .iter()
                .all(|slot| !slot.busy.load(Ordering::Acquire))
            {
                break;
            }
        }
    }
}

impl GpuFrameQuery {
    pub(crate) fn timestamp_writes(&mut self) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        if self.used_queries + 2 > MAX_GPU_TIMESTAMP_QUERIES {
            return None;
        }
        let beginning = self.used_queries;
        self.used_queries += 2;
        Some(wgpu::RenderPassTimestampWrites {
            query_set: &self.query_set,
            beginning_of_pass_write_index: Some(beginning),
            end_of_pass_write_index: Some(beginning + 1),
        })
    }

    pub(crate) fn resolve_and_map(self, encoder: &mut wgpu::CommandEncoder) {
        if self.used_queries == 0 {
            self.busy.store(false, Ordering::Release);
            return;
        }
        let byte_size = self.used_queries as u64 * 8;
        encoder.resolve_query_set(
            &self.query_set,
            0..self.used_queries,
            &self.resolve_buffer,
            0,
        );
        encoder.copy_buffer_to_buffer(&self.resolve_buffer, 0, &self.read_buffer, 0, byte_size);
        let callback_buffer = self.read_buffer.clone();
        let busy = self.busy;
        let sender = self.sender;
        let frame_id = self.frame_id;
        let period_ns = self.period_ns;
        encoder.map_buffer_on_submit(
            &self.read_buffer,
            wgpu::MapMode::Read,
            0..byte_size,
            move |result| {
                if result.is_ok() {
                    let mapped = callback_buffer.get_mapped_range(0..byte_size);
                    if let Some(frame_ticks) = gpu_timestamp_span_ticks(&mapped) {
                        let gpu_ms = frame_ticks as f64 * period_ns / 1_000_000.0;
                        let _ = sender.send((frame_id, gpu_ms));
                    } else if !GPU_READBACK_WARNED.swap(true, Ordering::Relaxed) {
                        eprintln!(
                            "[spot][profile] GPU timestamp query returned no valid pass pairs"
                        );
                    }
                    drop(mapped);
                    callback_buffer.unmap();
                } else if !GPU_READBACK_WARNED.swap(true, Ordering::Relaxed) {
                    eprintln!("[spot][profile] GPU timestamp readback mapping failed");
                }
                busy.store(false, Ordering::Release);
            },
        );
    }
}

fn gpu_timestamp_span_ticks(bytes: &[u8]) -> Option<u64> {
    let mut first_start = u64::MAX;
    let mut last_end = 0u64;
    let mut valid_passes = 0usize;
    for pair in bytes.chunks_exact(16) {
        let start = u64::from_le_bytes(pair[0..8].try_into().unwrap());
        let end = u64::from_le_bytes(pair[8..16].try_into().unwrap());
        if start > 0 && end >= start {
            first_start = first_start.min(start);
            last_end = last_end.max(end);
            valid_passes += 1;
        }
    }
    if valid_passes > 0 && last_end >= first_start {
        Some(last_end - first_start)
    } else {
        None
    }
}

fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            !matches!(value.as_str(), "" | "0" | "false" | "off" | "no")
        })
        .unwrap_or(default)
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(default)
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[cfg(target_os = "linux")]
fn resident_memory_kb() -> Option<u64> {
    std::fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

#[cfg(all(unix, not(target_os = "linux")))]
fn resident_memory_kb() -> Option<u64> {
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()?;
    std::str::from_utf8(&output.stdout)
        .ok()?
        .trim()
        .parse()
        .ok()
}

#[cfg(not(unix))]
fn resident_memory_kb() -> Option<u64> {
    None
}

fn values(
    samples: &VecDeque<FrameSample>,
    select: impl Fn(&FrameSample) -> Option<f64>,
) -> Vec<f64> {
    samples
        .iter()
        .filter_map(select)
        .filter(|value| value.is_finite())
        .collect()
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn max(values: &[f64]) -> f64 {
    values.iter().copied().reduce(f64::max).unwrap_or(0.0)
}

fn percentile(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = percentile.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let fraction = rank - lower as f64;
        sorted[lower] + (sorted[upper] - sorted[lower]) * fraction
    }
}

fn mean_optional(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| mean(values))
}

fn percentile_optional(values: &[f64], p: f64) -> Option<f64> {
    (!values.is_empty()).then(|| percentile(values, p))
}

fn optional_ms(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}ms"))
        .unwrap_or_else(|| "n/a".to_string())
}

fn ensure_parent(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn write_frame_csv(path: &Path, samples: &VecDeque<FrameSample>) -> std::io::Result<()> {
    ensure_parent(path)?;
    let mut writer = BufWriter::new(File::create(path)?);
    writeln!(writer, "{FRAME_CSV_HEADER}")?;
    for s in samples {
        writeln!(
            writer,
            "{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{}",
            s.frame,
            s.sample,
            s.frame_interval_ms,
            s.engine_ms,
            s.wait_ms,
            s.work_ms,
            s.prepare_ms,
            s.targets_ms,
            s.shadow_ms,
            s.main_3d_ms,
            s.overlay_ms,
            s.present_ms,
            s.update_ms,
            s.draw_ms,
            s.gpu_ms.map(|v| format!("{v:.6}")).unwrap_or_default(),
            s.rss_mb.map(|v| format!("{v:.6}")).unwrap_or_default()
        )?;
    }
    writer.flush()
}

fn write_summary_csv(
    path: &Path,
    scenario: &str,
    samples: &VecDeque<FrameSample>,
) -> std::io::Result<()> {
    ensure_parent(path)?;
    let mut writer = BufWriter::new(File::create(path)?);
    let frame = values(samples, |s| Some(s.frame_interval_ms));
    let engine = values(samples, |s| Some(s.engine_ms));
    let work = values(samples, |s| Some(s.work_ms));
    let update = values(samples, |s| Some(s.update_ms));
    let draw = values(samples, |s| Some(s.draw_ms));
    let gpu = values(samples, |s| s.gpu_ms);
    let rss = values(samples, |s| s.rss_mb);
    writeln!(writer, "{SUMMARY_CSV_HEADER}")?;
    writeln!(
        writer,
        "{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{},{},{},{},{},{}",
        csv_field(scenario),
        samples.len(),
        gpu.len(),
        mean(&frame),
        percentile(&frame, 0.50),
        percentile(&frame, 0.95),
        percentile(&frame, 0.99),
        max(&frame),
        mean(&engine),
        percentile(&engine, 0.50),
        percentile(&engine, 0.95),
        percentile(&engine, 0.99),
        max(&engine),
        mean(&work),
        percentile(&work, 0.95),
        percentile(&work, 0.99),
        mean(&update),
        percentile(&update, 0.95),
        mean(&draw),
        percentile(&draw, 0.95),
        csv_optional(mean_optional(&gpu)),
        csv_optional(percentile_optional(&gpu, 0.50)),
        csv_optional(percentile_optional(&gpu, 0.95)),
        csv_optional(percentile_optional(&gpu, 0.99)),
        csv_optional((!gpu.is_empty()).then(|| max(&gpu))),
        csv_optional(mean_optional(&rss)),
        csv_optional((!rss.is_empty()).then(|| max(&rss))),
    )?;
    writer.flush()
}

fn csv_field(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn csv_optional(value: Option<f64>) -> String {
    value.map(|v| format!("{v:.6}")).unwrap_or_default()
}

pub(crate) fn parse_present_mode_from_env() -> Option<wgpu::PresentMode> {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    let value: Option<String> = None;
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    let value: Option<String> = std::env::var("SPOT_PRESENT_MODE").ok();

    match value?.trim().to_ascii_lowercase().as_str() {
        "immediate" => Some(wgpu::PresentMode::Immediate),
        "mailbox" => Some(wgpu::PresentMode::Mailbox),
        "fifo" => Some(wgpu::PresentMode::Fifo),
        "auto" | "auto_vsync" => Some(wgpu::PresentMode::AutoVsync),
        "auto_no_vsync" => Some(wgpu::PresentMode::AutoNoVsync),
        _ => None,
    }
}

pub(crate) fn pick_present_mode(surface_caps: &wgpu::SurfaceCapabilities) -> wgpu::PresentMode {
    if let Some(requested) = parse_present_mode_from_env()
        && surface_caps.present_modes.contains(&requested)
    {
        return requested;
    }

    for preferred in [
        wgpu::PresentMode::AutoVsync,
        wgpu::PresentMode::Fifo,
        wgpu::PresentMode::Mailbox,
        wgpu::PresentMode::AutoNoVsync,
        wgpu::PresentMode::Immediate,
    ] {
        if surface_caps.present_modes.contains(&preferred) {
            return preferred;
        }
    }
    surface_caps.present_modes[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_interpolates_and_handles_empty_input() {
        assert_eq!(percentile(&[], 0.95), 0.0);
        assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0], 0.50), 2.5);
        assert!((percentile(&[1.0, 2.0, 3.0, 4.0], 0.95) - 3.85).abs() < 0.0001);
    }

    #[test]
    fn csv_field_escapes_quotes() {
        assert_eq!(csv_field("a\"b"), "\"a\"\"b\"");
    }

    #[test]
    fn gpu_timestamp_span_does_not_double_count_overlapping_passes() {
        let mut bytes = Vec::new();
        for timestamp in [100u64, 200, 150, 260, 0, 0, 300, 250] {
            bytes.extend_from_slice(&timestamp.to_le_bytes());
        }
        assert_eq!(gpu_timestamp_span_ticks(&bytes), Some(160));
        assert_eq!(gpu_timestamp_span_ticks(&[]), None);
    }
}
