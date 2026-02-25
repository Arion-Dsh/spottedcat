use anyhow::{Context as _, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct AudioSystem(pub(crate) Arc<AudioSystemInner>);

impl Clone for AudioSystem {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

pub(crate) struct AudioSystemInner {
    #[allow(dead_code)]
    stream: cpal::Stream,
    handler: Arc<Mutex<MixerHandler>>,
}

// cpal::Stream is safe to send and sync on most platforms.
// We explicitly implement Send and Sync to allow storing AudioSystem in a global static/Arc.
unsafe impl Send for AudioSystemInner {}
unsafe impl Sync for AudioSystemInner {}

impl fmt::Debug for AudioSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AudioSystem").finish()
    }
}

impl fmt::Debug for AudioSystemInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AudioSystemInner").finish()
    }
}

pub(crate) struct MixerHandler {
    sample_rate: u32,
    channels: u16,
    next_play_id: u64,
    next_sound_id: u32,
    sound_registry: HashMap<u32, SoundData>,
    sounds: Vec<PlayingSound>,
}

impl fmt::Debug for MixerHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MixerHandler").finish()
    }
}

impl MixerHandler {
    fn process(&mut self, output: &mut [f32]) {
        let channels = self.channels.max(1) as usize;
        let frames = output.len() / channels;
        for frame in 0..frames {
            let mut mix = 0.0f32;
            for sound in &mut self.sounds {
                if sound.finished || sound.paused {
                    continue;
                }
                mix += sound.next_sample(self.sample_rate);
            }
            let mix = mix.clamp(-1.0, 1.0);
            let base = frame * channels;
            for ch in 0..channels {
                output[base + ch] = mix;
            }
        }
        self.sounds.retain(|sound| !sound.finished);
    }

    fn unregister_sound(&mut self, sound_id: u32) {
        self.sound_registry.remove(&sound_id);
    }
}

impl AudioSystem {
    pub fn new() -> Result<Self> {
        Ok(Self(Arc::new(AudioSystemInner::new()?)))
    }

    pub(crate) fn play_sine(&self, freq: f32, volume: f32) -> Option<u64> {
        self.0.play_sine(freq, volume)
    }

    pub(crate) fn play_registered_sound_with_options(
        &self,
        sound_id: u32,
        options: PlayOptions,
    ) -> Option<u64> {
        self.0.play_registered_sound_with_options(sound_id, options)
    }

    pub(crate) fn play_sound_with_options(
        &self,
        sound: &SoundData,
        options: PlayOptions,
    ) -> Option<u64> {
        self.0.play_sound_with_options(sound, options)
    }

    pub(crate) fn register_sound(&self, sound_data: SoundData) -> u32 {
        self.0.register_sound(sound_data)
    }

    pub(crate) fn pause_play_id(&self, play_id: u64) {
        self.0.pause_play_id(play_id);
    }

    pub(crate) fn resume_play_id(&self, play_id: u64) {
        self.0.resume_play_id(play_id);
    }

    pub(crate) fn stop_play_id(&self, play_id: u64) {
        self.0.stop_play_id(play_id);
    }

    pub(crate) fn fade_in_play_id(&self, play_id: u64, duration: Duration) {
        self.0.fade_in_play_id(play_id, duration);
    }

    pub(crate) fn fade_out_play_id(&self, play_id: u64, duration: Duration) {
        self.0.fade_out_play_id(play_id, duration);
    }

    pub(crate) fn set_volume_play_id(&self, play_id: u64, volume: f32) {
        self.0.set_volume_play_id(play_id, volume);
    }

    pub(crate) fn is_playing_play_id(&self, play_id: u64) -> bool {
        self.0.is_playing_play_id(play_id)
    }

    pub(crate) fn unregister_sound(&self, sound_id: u32) {
        self.0.unregister_sound(sound_id);
    }

    /// Try to resume the audio stream (useful for WASM autoplay policy).
    pub(crate) fn try_resume(&self) {
        let _ = self.0.stream.play();
    }
}

impl AudioSystemInner {
    fn new() -> Result<Self> {
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .context("no output device available")?;
        let config = device.default_output_config()?;
        let sample_rate = config.sample_rate();
        let channels = config.channels();

        let handler = Arc::new(Mutex::new(MixerHandler {
            sample_rate,
            channels,
            next_play_id: 1,
            next_sound_id: 1,
            sound_registry: HashMap::new(),
            sounds: Vec::new(),
        }));

        let handler_clone = Arc::clone(&handler);
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if let Ok(mut h) = handler_clone.lock() {
                        h.process(data);
                    }
                },
                |err| eprintln!("audio stream error: {}", err),
                None,
            )?,
            _ => return Err(anyhow::anyhow!("Unsupported sample format")),
        };

        // On WASM, stream.play() may fail due to browser autoplay policy.
        // The AudioContext will be resumed automatically after user interaction.
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            if let Err(e) = stream.play() {
                web_sys::console::warn_1(
                    &format!("[spot][audio] play deferred (autoplay policy): {e:?}").into(),
                );
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        stream.play()?;

        Ok(Self { stream, handler })
    }

    fn play_sine(&self, freq: f32, volume: f32) -> Option<u64> {
        let sample_rate = {
            let Ok(h) = self.handler.lock() else {
                return None;
            };
            h.sample_rate
        };
        let duration = Duration::from_millis(600);
        let total_samples = (duration.as_secs_f64() * sample_rate as f64) as usize;
        let mut samples = Vec::with_capacity(total_samples);
        for i in 0..total_samples {
            let t = i as f32 / sample_rate as f32;
            samples.push((t * freq * std::f32::consts::TAU).sin());
        }
        let sound = SoundData {
            samples: Arc::new(samples),
            sample_rate,
            channels: 1,
        };
        self.play_sound_with_options(
            &sound,
            PlayOptions {
                volume,
                fade_in: Duration::from_millis(20),
                fade_out: Some(Duration::from_millis(80)),
                start_paused: false,
            },
        )
    }

    fn play_registered_sound_with_options(
        &self,
        sound_id: u32,
        options: PlayOptions,
    ) -> Option<u64> {
        let Ok(mut handler) = self.handler.lock() else {
            return None;
        };
        let sound = handler.sound_registry.get(&sound_id)?.clone();
        Some(Self::add_playing_sound_locked(
            &mut handler,
            &sound,
            options,
        ))
    }

    fn play_sound_with_options(&self, sound: &SoundData, options: PlayOptions) -> Option<u64> {
        if sound.samples.is_empty() || sound.sample_rate == 0 {
            return None;
        }
        if let Ok(mut handler) = self.handler.lock() {
            Some(Self::add_playing_sound_locked(&mut handler, sound, options))
        } else {
            None
        }
    }

    fn register_sound(&self, sound_data: SoundData) -> u32 {
        let Ok(mut handler) = self.handler.lock() else {
            return 0;
        };
        let sound_id = handler.next_sound_id;
        handler.next_sound_id = handler.next_sound_id.saturating_add(1).max(1);
        handler.sound_registry.insert(sound_id, sound_data);
        sound_id
    }

    fn unregister_sound(&self, sound_id: u32) {
        if let Ok(mut handler) = self.handler.lock() {
            handler.unregister_sound(sound_id);
        }
    }

    fn add_playing_sound_locked(
        handler: &mut MixerHandler,
        sound: &SoundData,
        options: PlayOptions,
    ) -> u64 {
        let play_id = handler.next_play_id;
        handler.next_play_id = handler.next_play_id.wrapping_add(1).max(1);
        let source_rate = sound.sample_rate;
        let step = source_rate as f64 / handler.sample_rate as f64;
        let total_frames = if step > 0.0 {
            (sound.samples.len() as f64 / step).ceil() as u64
        } else {
            sound.samples.len() as u64
        };
        let mut playing = PlayingSound::new(play_id, Arc::clone(&sound.samples), source_rate);
        playing.volume = options.volume.max(0.0);
        playing.paused = options.start_paused;
        playing.total_frames = total_frames;
        if options.fade_in > Duration::ZERO {
            playing.fade_gain = 0.0;
            playing.fade = Some(FadeState::new(
                0.0,
                1.0,
                duration_to_frames(options.fade_in, handler.sample_rate),
                false,
            ));
        }
        if let Some(fade_out) = options.fade_out {
            let frames = duration_to_frames(fade_out, handler.sample_rate);
            if frames > 0 {
                playing.fade_out_on_end = Some(FadeOnEnd {
                    frames,
                    started: false,
                });
            }
        }
        handler.sounds.push(playing);
        play_id
    }

    fn pause_play_id(&self, play_id: u64) {
        self.update_playing(play_id, |sound| sound.paused = true);
    }

    fn resume_play_id(&self, play_id: u64) {
        self.update_playing(play_id, |sound| sound.paused = false);
    }

    fn stop_play_id(&self, play_id: u64) {
        self.update_playing(play_id, |sound| sound.finished = true);
    }

    fn fade_in_play_id(&self, play_id: u64, duration: Duration) {
        let frames = duration_to_frames(duration, self.sample_rate());
        if frames == 0 {
            return;
        }
        self.update_playing(play_id, |sound| {
            sound.fade_gain = 0.0;
            sound.fade = Some(FadeState::new(0.0, 1.0, frames, false));
        });
    }

    fn fade_out_play_id(&self, play_id: u64, duration: Duration) {
        let frames = duration_to_frames(duration, self.sample_rate());
        if frames == 0 {
            self.stop_play_id(play_id);
            return;
        }
        self.update_playing(play_id, |sound| {
            let start = sound.fade_gain;
            sound.fade = Some(FadeState::new(start, 0.0, frames, true));
        });
    }

    fn set_volume_play_id(&self, play_id: u64, volume: f32) {
        self.update_playing(play_id, |sound| sound.volume = volume.max(0.0));
    }

    fn is_playing_play_id(&self, play_id: u64) -> bool {
        let Ok(handler) = self.handler.lock() else {
            return false;
        };
        handler
            .sounds
            .iter()
            .find(|sound| sound.id == play_id)
            .map(|sound| !sound.finished && !sound.paused)
            .unwrap_or(false)
    }

    fn sample_rate(&self) -> u32 {
        let Ok(handler) = self.handler.lock() else {
            return 0;
        };
        handler.sample_rate
    }

    fn update_playing(&self, play_id: u64, f: impl FnOnce(&mut PlayingSound)) {
        let Ok(mut handler) = self.handler.lock() else {
            return;
        };
        if let Some(sound) = handler.sounds.iter_mut().find(|sound| sound.id == play_id) {
            f(sound);
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PlayOptions {
    pub volume: f32,
    pub fade_in: Duration,
    pub fade_out: Option<Duration>,
    pub start_paused: bool,
}

impl Default for PlayOptions {
    fn default() -> Self {
        Self {
            volume: 1.0,
            fade_in: Duration::ZERO,
            fade_out: None,
            start_paused: false,
        }
    }
}

#[derive(Clone)]
pub(crate) struct SoundData {
    pub samples: Arc<Vec<f32>>,
    pub sample_rate: u32,
    #[allow(dead_code)]
    pub channels: u16,
}

struct PlayingSound {
    id: u64,
    samples: Arc<Vec<f32>>,
    source_rate: u32,
    position: f64,
    volume: f32,
    paused: bool,
    fade_gain: f32,
    fade: Option<FadeState>,
    fade_out_on_end: Option<FadeOnEnd>,
    total_frames: u64,
    frames_played: u64,
    finished: bool,
}

impl PlayingSound {
    fn new(id: u64, samples: Arc<Vec<f32>>, source_rate: u32) -> Self {
        Self {
            id,
            samples,
            source_rate,
            position: 0.0,
            volume: 1.0,
            paused: false,
            fade_gain: 1.0,
            fade: None,
            fade_out_on_end: None,
            total_frames: 0,
            frames_played: 0,
            finished: false,
        }
    }

    fn next_sample(&mut self, output_rate: u32) -> f32 {
        if self.finished || output_rate == 0 || self.source_rate == 0 {
            return 0.0;
        }

        if let Some(fade_out) = &mut self.fade_out_on_end {
            if !fade_out.started {
                let start_at = self.total_frames.saturating_sub(fade_out.frames);
                if self.frames_played >= start_at {
                    let start_gain = self.fade_gain;
                    self.fade = Some(FadeState::new(start_gain, 0.0, fade_out.frames, true));
                    fade_out.started = true;
                }
            }
        }

        let sample = self.sample_at_position();

        if let Some(fade) = &mut self.fade {
            let gain = fade.next_gain();
            self.fade_gain = gain;
            if fade.finished() {
                let stop = fade.stop_on_end && fade.end <= 0.0001;
                self.fade = None;
                if stop {
                    self.finished = true;
                }
            }
        }

        let step = self.source_rate as f64 / output_rate as f64;
        self.position += step;
        self.frames_played = self.frames_played.saturating_add(1);

        if self.position >= self.samples.len() as f64 {
            self.finished = true;
        }

        sample * self.volume * self.fade_gain
    }

    fn sample_at_position(&self) -> f32 {
        let len = self.samples.len();
        if len == 0 {
            return 0.0;
        }
        let idx = self.position.floor() as usize;
        if idx >= len {
            return 0.0;
        }
        let next_idx = idx + 1;
        let frac = (self.position - idx as f64) as f32;
        let s0 = self.samples[idx];
        let s1 = if next_idx < len {
            self.samples[next_idx]
        } else {
            0.0
        };
        s0 + (s1 - s0) * frac
    }
}

struct FadeState {
    current: f32,
    step: f32,
    remaining: u64,
    end: f32,
    stop_on_end: bool,
}

impl FadeState {
    fn new(start: f32, end: f32, frames: u64, stop_on_end: bool) -> Self {
        if frames == 0 {
            return Self {
                current: end,
                step: 0.0,
                remaining: 0,
                end,
                stop_on_end,
            };
        }
        let step = (end - start) / frames as f32;
        Self {
            current: start,
            step,
            remaining: frames,
            end,
            stop_on_end,
        }
    }

    fn next_gain(&mut self) -> f32 {
        if self.remaining == 0 {
            return self.end;
        }
        let gain = self.current;
        self.current += self.step;
        self.remaining = self.remaining.saturating_sub(1);
        if self.remaining == 0 {
            self.current = self.end;
        }
        gain
    }

    fn finished(&self) -> bool {
        self.remaining == 0
    }
}

struct FadeOnEnd {
    frames: u64,
    started: bool,
}

fn duration_to_frames(duration: Duration, sample_rate: u32) -> u64 {
    if duration == Duration::ZERO || sample_rate == 0 {
        return 0;
    }
    (duration.as_secs_f64() * sample_rate as f64).round() as u64
}

pub(crate) fn decode_sound_from_bytes(bytes: Vec<u8>) -> Result<SoundData> {
    let src = std::io::Cursor::new(bytes);
    let mss = MediaSourceStream::new(Box::new(src), Default::default());
    let hint = Hint::new();

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .context("no supported audio track found")?;

    let dec_opts = DecoderOptions::default();
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &dec_opts)?;

    let track_id = track.id;
    let mut sample_rate = 0;
    let mut channels = 0;
    let mut samples = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::IoError(ref err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(err) => return Err(err.into()),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = decoder.decode(&packet)?;

        if sample_rate == 0 {
            let spec = decoded.spec();
            sample_rate = spec.rate;
            channels = spec.channels.count() as u16;
        }

        match decoded {
            AudioBufferRef::F32(buf) => {
                let channel_count = buf.spec().channels.count();
                for i in 0..buf.frames() {
                    let mut sum = 0.0f32;
                    for ch in 0..channel_count {
                        sum += buf.chan(ch)[i];
                    }
                    samples.push(sum / channel_count as f32);
                }
            }
            AudioBufferRef::U8(buf) => {
                let channel_count = buf.spec().channels.count();
                for i in 0..buf.frames() {
                    let mut sum = 0.0f32;
                    for ch in 0..channel_count {
                        sum += (buf.chan(ch)[i] as f32 - 128.0) / 128.0;
                    }
                    samples.push(sum / channel_count as f32);
                }
            }
            AudioBufferRef::S16(buf) => {
                let channel_count = buf.spec().channels.count();
                for i in 0..buf.frames() {
                    let mut sum = 0.0f32;
                    for ch in 0..channel_count {
                        sum += buf.chan(ch)[i] as f32 / 32768.0;
                    }
                    samples.push(sum / channel_count as f32);
                }
            }
            AudioBufferRef::S32(buf) => {
                let channel_count = buf.spec().channels.count();
                for i in 0..buf.frames() {
                    let mut sum = 0.0f32;
                    for ch in 0..channel_count {
                        sum += buf.chan(ch)[i] as f32 / 2147483648.0;
                    }
                    samples.push(sum / channel_count as f32);
                }
            }
            _ => {}
        }
    }

    Ok(SoundData {
        samples: Arc::new(samples),
        sample_rate,
        channels,
    })
}
