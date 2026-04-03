use crate::Context;
use crate::audio;
use std::time::Duration;

/// Configuration options for playing a sound.
#[derive(Debug, Clone)]
pub struct SoundOptions {
    /// Volume multiplier (0.0 to 1.0).
    pub volume: f32,
    /// Duration to fade in the sound from zero volume.
    pub fade_in: Duration,
    /// Optional duration to fade out the sound when stopped.
    pub fade_out: Option<Duration>,
    /// Whether the sound should start in a paused state.
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

/// Registers a sound from raw bytes (supported formats depend on backend).
///
/// Returns a unique sound ID if successful.
pub fn register_sound(ctx: &mut Context, bytes: Vec<u8>) -> Option<u32> {
    ctx.with_audio(|a| a.register_sound(bytes))
}

/// Plays a registered sound with the specified options.
///
/// Returns a unique play ID that can be used to control the playing sound.
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

/// A convenience function to play a registered sound with default options.
pub fn play_sound_simple(ctx: &mut Context, sound_id: u32) -> Option<u64> {
    ctx.with_audio(|a| {
        a.play_registered_sound_with_options(sound_id, audio::PlayOptions::default())
    })
    .flatten()
}

/// Pauses a currently playing sound by its play ID.
pub fn pause_sound(ctx: &mut Context, play_id: u64) {
    ctx.with_audio(|a| a.pause_play_id(play_id));
}

/// Resumes a paused sound by its play ID.
pub fn resume_sound(ctx: &mut Context, play_id: u64) {
    ctx.with_audio(|a| a.resume_play_id(play_id));
}

/// Stops a playing sound immediately by its play ID.
pub fn stop_sound(ctx: &mut Context, play_id: u64) {
    ctx.with_audio(|a| a.stop_play_id(play_id));
}

/// Stops all currently playing sounds.
pub fn stop_all_sounds(ctx: &mut Context) {
    ctx.with_audio(|a| a.stop_all_sounds());
}

/// Initiates a fade-in for a playing sound.
pub fn fade_in_sound(ctx: &mut Context, play_id: u64, duration: Duration) {
    ctx.with_audio(|a| a.fade_in_play_id(play_id, duration));
}

/// Initiates a fade-out for a playing sound.
pub fn fade_out_sound(ctx: &mut Context, play_id: u64, duration: Duration) {
    ctx.with_audio(|a| a.fade_out_play_id(play_id, duration));
}

/// Sets the volume of a playing sound directly.
pub fn set_sound_volume(ctx: &mut Context, play_id: u64, volume: f32) {
    ctx.with_audio(|a| a.set_volume_play_id(play_id, volume));
}

/// Returns true if the sound associated with the play ID is still active.
pub fn is_sound_playing(ctx: &Context, play_id: u64) -> bool {
    ctx.runtime
        .audio
        .as_ref()
        .map(|a| a.is_playing_play_id(play_id))
        .unwrap_or(false)
}

/// Unregisters a sound and frees its resources.
pub fn unregister_sound(ctx: &mut Context, sound_id: u32) {
    ctx.with_audio(|a| a.unregister_sound(sound_id));
}

/// A debug function to play a simple sine wave at the specified frequency.
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
