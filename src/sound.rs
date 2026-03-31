use crate::Context;
use crate::audio;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SoundOptions {
    pub volume: f32,
    pub fade_in: Duration,
    pub fade_out: Option<Duration>,
    pub start_paused: bool,
}

impl Default for SoundOptions {
    fn default() -> Self {
        Self {
            volume: 1.0,
            fade_in: Duration::ZERO,
            fade_out: None,
            start_paused: false,
        }
    }
}

pub fn register_sound(ctx: &mut Context, bytes: Vec<u8>) -> Option<u32> {
    ctx.with_audio(|a| a.register_sound(bytes))
}

pub fn play_sound(ctx: &mut Context, sound_id: u32, options: SoundOptions) -> Option<u64> {
    let opts = audio::PlayOptions {
        volume: options.volume,
        fade_in: options.fade_in,
        fade_out: options.fade_out,
        start_paused: options.start_paused,
    };
    ctx.with_audio(|a| a.play_registered_sound_with_options(sound_id, opts))
        .flatten()
}

pub fn play_sound_simple(ctx: &mut Context, sound_id: u32) -> Option<u64> {
    ctx.with_audio(|a| {
        a.play_registered_sound_with_options(sound_id, audio::PlayOptions::default())
    })
    .flatten()
}

pub fn pause_sound(ctx: &mut Context, play_id: u64) {
    ctx.with_audio(|a| a.pause_play_id(play_id));
}

pub fn resume_sound(ctx: &mut Context, play_id: u64) {
    ctx.with_audio(|a| a.resume_play_id(play_id));
}

pub fn stop_sound(ctx: &mut Context, play_id: u64) {
    ctx.with_audio(|a| a.stop_play_id(play_id));
}

pub fn fade_in_sound(ctx: &mut Context, play_id: u64, duration: Duration) {
    ctx.with_audio(|a| a.fade_in_play_id(play_id, duration));
}

pub fn fade_out_sound(ctx: &mut Context, play_id: u64, duration: Duration) {
    ctx.with_audio(|a| a.fade_out_play_id(play_id, duration));
}

pub fn set_sound_volume(ctx: &mut Context, play_id: u64, volume: f32) {
    ctx.with_audio(|a| a.set_volume_play_id(play_id, volume));
}

pub fn is_sound_playing(ctx: &Context, play_id: u64) -> bool {
    ctx.runtime
        .audio
        .as_ref()
        .map(|a| a.is_playing_play_id(play_id))
        .unwrap_or(false)
}

pub fn unregister_sound(ctx: &mut Context, sound_id: u32) {
    ctx.with_audio(|a| a.unregister_sound(sound_id));
}

pub fn play_sine(ctx: &mut Context, freq: f32, volume: f32) -> Option<u64> {
    ctx.with_audio(|a| a.play_sine(freq, volume)).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_sound_returns_none_when_audio_is_unavailable() {
        let mut ctx = Context::new();

        assert_eq!(register_sound(&mut ctx, vec![1, 2, 3]), None);
    }

    #[test]
    fn play_sound_simple_returns_none_when_audio_is_unavailable() {
        let mut ctx = Context::new();

        assert_eq!(play_sound_simple(&mut ctx, 42), None);
    }
}
