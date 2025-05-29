use std::sync::{Arc, Mutex};
use std::fs::File;
use std::path::Path;
use std::time::Duration;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::PlaybackCommand;


// static GLOBAL_THREAD_COUNT: AtomicUsize = AtomicUsize::new(0);


#[derive(Debug, Clone)]
pub struct Track {
    pub id: u32, // Unique ID for the track
    pub data: Arc<Vec<f32>>,
    pub duration: Duration,
    pub current_sample_index: Arc<Mutex<u32>>,
    pub playback_state: Arc<Mutex<PlaybackCommand>>,
    pub channels: u16,
    pub rate: u32,
    pub volume: Arc<Mutex<f32>>,
}

impl Track {

    pub fn from_path(path_str: &str) -> Result<Self, Error> {
        let src_file = File::open(path_str).map_err(Error::IoError)?;
        let mss = MediaSourceStream::new(Box::new(src_file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = Path::new(path_str)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
        {
            println!("Extension: {}", ext);
            hint.with_extension(ext);

        }

        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &fmt_opts, &meta_opts)
            .map_err(|err| {
                // Workaround for Error::Unsupported expecting &'static str
                // This leaks memory for each dynamic error message.
                // Consider a more robust error handling strategy for production code.
                let msg = format!("Unsupported format or error probing: {}", err);
                Error::Unsupported(Box::leak(msg.into_boxed_str()))
            })?;

        let mut format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|t| {
                t.codec_params.codec != CODEC_TYPE_NULL && t.codec_params.sample_rate.is_some()
            })
            .ok_or(Error::Unsupported("No suitable audio track found."))?;

        let sample_rate_u32 = track.codec_params.sample_rate.ok_or(Error::Unsupported(
            "Missing sample rate in codec parameters",
        ))?;

        let channels_spec = track.codec_params.channels.ok_or(Error::Unsupported(
            "Missing channels specification in codec parameters",
        ))?;
        let channels_u16: u16 = channels_spec
            .count()
            .try_into()
            .map_err(|_| Error::Unsupported("Channel count out of range for u16"))?;

        if channels_u16 == 0 {
            return Err(Error::Unsupported("Audio track has zero channels."));
        }

        let dec_opts: DecoderOptions = Default::default();
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &dec_opts)
            .map_err(|err| {
                let msg = format!("Unsupported codec or error creating decoder: {}", err);
                Error::Unsupported(Box::leak(msg.into_boxed_str()))
            })?;

        let track_id = track.id;
        let mut all_samples: Vec<f32> = Vec::new();

        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(Error::IoError(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(Error::ResetRequired) => {
                    return Err(Error::Unsupported(
                        "Media stream changed (ResetRequired), not handled.",
                    ));
                }
                Err(err) => {
                    return Err(err);
                }
            };

            while !format.metadata().is_latest() {
                format.metadata().pop();
            }

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => {
                    let mut sample_buf =
                        SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
                    sample_buf.copy_interleaved_ref(decoded);
                    let samples = sample_buf.samples();
                    for sample in samples {
                        all_samples.push(*sample);
                    }
                }
                Err(Error::IoError(_)) => {
                    continue;
                }
                Err(Error::DecodeError(_)) => {
                    continue;
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }

        let total_frames_per_channel = if channels_u16 > 0 {
            all_samples.len() / (channels_u16 as usize)
        } else {
            0
        };

        let calculated_duration = if sample_rate_u32 > 0 {
            Duration::from_secs_f64(total_frames_per_channel as f64 / sample_rate_u32 as f64)
        } else {
            Duration::from_secs(0)
        };

        Ok(Track {
            id: track_id,
            data: Arc::new(all_samples),
            duration: calculated_duration,
            current_sample_index: Arc::new(Mutex::new(0)),
            playback_state: Arc::new(Mutex::new(PlaybackCommand::Pause)),
            channels: channels_u16,
            rate: sample_rate_u32,
            volume: Arc::new(Mutex::new(1.0)),
        })
    }
    pub fn get_next_sample(&self) -> f32 {
        let mut index_guard = self.current_sample_index.lock().unwrap();
        let mut state_guard = self.playback_state.lock().unwrap();

        if *state_guard == PlaybackCommand::Play {
            if *index_guard < self.data.len() as u32 {
                let sample_value = self.data[*index_guard as usize];
                *index_guard += 1;
                // Optional: apply volume here if you added a volume field
                // sample_value * (*self.volume.lock().unwrap())
                sample_value
            } else {
                // End of track, transition to Stop
                *state_guard = PlaybackCommand::Stop;
                println!("Track {} finished playback.", self.id);
                0.0 // Return silence
            }
        } else {
            0.0 // Return silence if paused or stopped
        }
    }
    pub fn handle_command(&self, command: PlaybackCommand) {
        let mut state_guard = self.playback_state.lock().unwrap();
        let mut index_guard = self.current_sample_index.lock().unwrap();

        match command {
            PlaybackCommand::Play => {
                *state_guard = PlaybackCommand::Play;
            }
            PlaybackCommand::Pause => {
                *state_guard = PlaybackCommand::Pause;
            }
            PlaybackCommand::Stop => {
                *state_guard = PlaybackCommand::Stop;
                *index_guard = 0; // Reset index on stop
            }
            PlaybackCommand::Seek(idx) => {
                *index_guard = idx.min(self.data.len() as u32); // Ensure index is within bounds
            }
        }
        println!("Track {} received command: {:?}", self.id, command);
    }

    pub fn is_stopped(&self) -> bool {
        *self.playback_state.lock().unwrap() == PlaybackCommand::Stop
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }
        /// Resamples the track's audio data to the target sample rate using linear interpolation.
        pub fn resample_linear(&mut self, target_sample_rate: u32) -> Result<(), anyhow::Error> {
            if self.rate == target_sample_rate {
                return Ok(()); // No resampling needed
            }
    
            let source_sample_rate_f = self.rate as f32;
            let target_sample_rate_f = target_sample_rate as f32;
            let ratio = source_sample_rate_f / target_sample_rate_f; // How many source samples per target sample
    
            let current_num_frames = (self.data.len() / self.channels as usize) as f32;
            let estimated_output_frames = (current_num_frames * (target_sample_rate_f / source_sample_rate_f)).ceil() as usize;
            let estimated_output_samples = estimated_output_frames * self.channels as usize;
    
            let mut new_data = Vec::with_capacity(estimated_output_samples);
    
            for i in 0..estimated_output_frames {
                let source_frame_f = i as f32 * ratio; // Floating point index in source frames
                let source_frame_idx = source_frame_f.floor() as usize;
                let alpha = source_frame_f - source_frame_idx as f32; // Interpolation factor (0.0 to 1.0)
    
                for c in 0..self.channels as usize {
                    let current_sample_idx = source_frame_idx * self.channels as usize + c;
                    let next_sample_idx = (source_frame_idx + 1) * self.channels as usize + c;
    
                    let sample1 = if current_sample_idx < self.data.len() {
                        self.data[current_sample_idx]
                    } else {
                        0.0
                    };
    
                    let sample2 = if next_sample_idx < self.data.len() {
                        self.data[next_sample_idx]
                    } else {
                        sample1 // If at end, just use the last valid sample
                    };
    
                    // Linear interpolation formula: value = sample1 * (1 - alpha) + sample2 * alpha
                    let interpolated_sample = sample1 * (1.0 - alpha) + sample2 * alpha;
                    new_data.push(interpolated_sample);
                }
            }
    
            self.data = Arc::new(new_data);
            self.rate = target_sample_rate;
            let new_num_frames = (self.data.len() / self.channels as usize) as f64;
            self.duration = Duration::from_secs_f64(new_num_frames / self.rate as f64);
            *self.current_sample_index.lock().unwrap() = 0; // Reset index after resampling
            Ok(())
        }
    
        /// Converts the track's audio channels to the target number of channels.
        /// This is a basic implementation:
        /// - Mono to Stereo: Duplicates the mono channel to both stereo channels.
        /// - Stereo to Mono: Averages the stereo channels.
        /// - Other conversions would require more complex logic.
        pub fn convert_channels(&mut self, target_channels: u16) -> Result<(), anyhow::Error> {
            if self.channels == target_channels {
                return Ok(()); // No conversion needed
            }
    
            let mut new_data = Vec::new();
            let current_data_len = self.data.len();
            let current_channels = self.channels as usize;
            let target_channels_usize = target_channels as usize;
    
            match (current_channels, target_channels_usize) {
                (1, 2) => { // Mono to Stereo
                    for i in 0..current_data_len {
                        let sample = self.data[i];
                        new_data.push(sample); // Left
                        new_data.push(sample); // Right
                    }
                }
                (2, 1) => { // Stereo to Mono
                    for i in (0..current_data_len).step_by(2) {
                        let left = self.data[i];
                        let right = self.data[i + 1];
                        new_data.push((left + right) / 2.0); // Average
                    }
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Unsupported channel conversion: from {} to {}",
                        self.channels,
                        target_channels
                    ));
                }
            }
            self.data = Arc::new(new_data);
            self.channels = target_channels;
            let new_num_frames = (self.data.len() / self.channels as usize) as f64;
            self.duration = Duration::from_secs_f64(new_num_frames / self.rate as f64);
            *self.current_sample_index.lock().unwrap() = 0; // Reset index after channel conversion
            Ok(())
        }
}