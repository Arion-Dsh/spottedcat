

use std::sync::{Arc, Mutex};
pub(crate) use std::time::Duration;

use super::{PlaybackCommand, Track};

#[derive(Debug, PartialEq)]
pub enum MixerError {
    NoSamplesToMix,
    InconsistentSampleRates,
    InconsistentChannelCounts,
    InvalidInputSample, // E.g., channels = 0
}

pub struct Mixer;

impl Mixer {
    /// Mixes multiple `Sample` structs into a single `Sample`.
    ///
    /// All input samples must have the same sample rate and channel count.
    /// Shorter samples will be padded with silence to match the duration of the longest sample.
    /// The audio data is mixed by summing corresponding samples and then averaging
    /// them (dividing by the number of tracks) to help prevent clipping.
    /// The duration of the output sample will be equal to the longest input sample.
    /// After mixing and averaging, the output is normalized to prevent clipping
    /// and maintain a consistent amplitude.
    ///
    /// # Arguments
    /// * `samples_to_mix`: A slice of references to `Sample` structs to be mixed.
    ///
    /// # Returns
    /// A `Result` containing the mixed `Sample` or a `MixerError` if mixing fails.
    pub fn mix(samples_to_mix: &[&Track]) -> Result<Track, MixerError> {
        if samples_to_mix.is_empty() {
            return Err(MixerError::NoSamplesToMix);
        }

        // Get properties from the first sample and validate consistency
        let first_sample = samples_to_mix[0];
        let target_rate = first_sample.rate;
        let target_channels = first_sample.channels;

        if target_channels == 0 {
            return Err(MixerError::InvalidInputSample);
        }

        for sample in samples_to_mix.iter().skip(1) {
            if sample.rate != target_rate {
                return Err(MixerError::InconsistentSampleRates);
            }
            if sample.channels != target_channels {
                return Err(MixerError::InconsistentChannelCounts);
            }
            if sample.channels == 0 {
                return Err(MixerError::InvalidInputSample);
            }
        }

        // Determine the maximum length of the data vector among all samples
        let mut max_data_len = 0;
        for sample in samples_to_mix.iter() {
            if sample.data.len() > max_data_len {
                max_data_len = sample.data.len();
            }
        }
        
        // Initialize output data buffer with zeros
        let mut mixed_data = vec![0.0f32; max_data_len];
        let num_tracks_to_mix = samples_to_mix.len() as f32;

        // Sum samples from all tracks, padding shorter ones with silence
        for sample_ref in samples_to_mix.iter() {
            // Iterate only up to the length of the current sample's data
            for (i, &sample_value) in sample_ref.data.iter().enumerate() {
                // `mixed_data` is already initialized with zeros, so values beyond
                // `sample_ref.data.len()` will remain zero, effectively padding with silence.
                mixed_data[i] += sample_value;
            }
        }

        // Average the summed samples by dividing by the number of tracks
        if num_tracks_to_mix > 0.0 {
            for sample_value in mixed_data.iter_mut() {
                *sample_value /= num_tracks_to_mix;
            }
        }

        // Normalization Step
        let mut max_amplitude = 0.0f32;
        for &value in mixed_data.iter() {
            let abs_value = value.abs();
            if abs_value > max_amplitude {
                max_amplitude = abs_value;
            }
        }

        if max_amplitude > 1.0f32 {
            let normalization_factor = 1.0f32 / max_amplitude;
            for sample_value in mixed_data.iter_mut() {
                *sample_value *= normalization_factor;
            }
        }

        // Calculate the duration of the mixed sample
        let num_frames = if target_channels > 0 {
            max_data_len / (target_channels as usize)
        } else {
            0
        };
        
        let mixed_duration = if target_rate > 0 {
            Duration::from_secs_f64(num_frames as f64 / target_rate as f64)
        } else {
            Duration::from_secs(0)
        };

        Ok(Track {
            data: mixed_data.into(),
            duration: mixed_duration,
            rate: target_rate,
            channels: target_channels,
            volume: Arc::new(Mutex::new(1.0)),
            id: 0,
            current_sample_index: Arc::new(Mutex::new(0)),
            playback_state: Arc::new(Mutex::new(PlaybackCommand::Pause)),
        })
    }
}