//! Performance profiling and present mode selection utilities.

use std::sync::Mutex;
use std::sync::OnceLock;

pub(crate) static PROFILE_RENDER: OnceLock<bool> = OnceLock::new();

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

pub(crate) static PROFILE_STATS: OnceLock<Mutex<RenderProfileStats>> = OnceLock::new();

pub(crate) fn parse_present_mode_from_env() -> Option<wgpu::PresentMode> {
    let v = std::env::var("SPOT_PRESENT_MODE").ok()?;
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
    if let Some(requested) = parse_present_mode_from_env() {
        if surface_caps.present_modes.iter().any(|m| *m == requested) {
            return requested;
        }
    }

    if surface_caps
        .present_modes
        .iter()
        .any(|m| *m == wgpu::PresentMode::Immediate)
    {
        wgpu::PresentMode::Immediate
    } else {
        surface_caps.present_modes[0]
    }
}
