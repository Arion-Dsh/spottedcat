//! Performance profiling and present mode selection utilities.

use std::sync::Mutex;
use std::sync::OnceLock;

#[allow(dead_code)]
pub(crate) static PROFILE_RENDER: OnceLock<bool> = OnceLock::new();

#[allow(dead_code)]
pub(crate) struct RenderProfileStats {
    pub frame: u64,
    pub sum_total_ms: f64,
    pub sum_wait_ms: f64,
    pub sum_work_ms: f64,
    pub min_total_ms: f64,
    pub max_total_ms: f64,
}

impl Default for RenderProfileStats {
    fn default() -> Self {
        Self {
            frame: 0,
            sum_total_ms: 0.0,
            sum_wait_ms: 0.0,
            sum_work_ms: 0.0,
            min_total_ms: f64::INFINITY,
            max_total_ms: 0.0,
        }
    }
}

#[allow(dead_code)]
pub(crate) static PROFILE_STATS: OnceLock<Mutex<RenderProfileStats>> = OnceLock::new();

pub(crate) fn render_profiling_enabled() -> bool {
    *PROFILE_RENDER.get_or_init(|| {
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            false
        }
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        {
            std::env::var("SPOT_PROFILE_RENDER")
                .map(|value| {
                    let value = value.trim().to_ascii_lowercase();
                    !matches!(value.as_str(), "" | "0" | "false" | "off")
                })
                .unwrap_or(false)
        }
    })
}

pub(crate) fn record_render_frame(wait_ms: f64, total_ms: f64) {
    if !render_profiling_enabled() {
        return;
    }

    let work_ms = (total_ms - wait_ms).max(0.0);

    let stats = PROFILE_STATS.get_or_init(|| Mutex::new(RenderProfileStats::default()));
    let mut stats = stats.lock().unwrap();
    stats.frame += 1;
    stats.sum_total_ms += total_ms;
    stats.sum_wait_ms += wait_ms;
    stats.sum_work_ms += work_ms;
    stats.min_total_ms = stats.min_total_ms.min(total_ms);
    stats.max_total_ms = stats.max_total_ms.max(total_ms);

    if stats.frame % 120 == 0 {
        let frames = stats.frame as f64;
        eprintln!(
            "[spot][profile] frames={} avg_total={:.2}ms avg_wait={:.2}ms avg_work={:.2}ms min_total={:.2}ms max_total={:.2}ms",
            stats.frame,
            stats.sum_total_ms / frames,
            stats.sum_wait_ms / frames,
            stats.sum_work_ms / frames,
            stats.min_total_ms,
            stats.max_total_ms
        );
    }
}

pub(crate) fn parse_present_mode_from_env() -> Option<wgpu::PresentMode> {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    let v: Option<String> = None;
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    let v: Option<String> = std::env::var("SPOT_PRESENT_MODE").ok();
    
    let v = v?;
    let v = v.trim().to_ascii_lowercase();
    if v.is_empty() {
        return None;
    }
    match v.as_str() {
        "immediate" => Some(wgpu::PresentMode::Immediate),
        "mailbox" => Some(wgpu::PresentMode::Mailbox),
        "fifo" => Some(wgpu::PresentMode::Fifo),
        "auto" => Some(wgpu::PresentMode::AutoVsync),
        "auto_vsync" => Some(wgpu::PresentMode::AutoVsync),
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
