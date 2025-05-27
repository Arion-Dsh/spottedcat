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
// use symphonia::core::units::Time; // Not strictly needed for this duration calculation method

#[derive(Debug, Clone)]
pub struct Sample {
    pub data: Vec<f32>,
    pub duration: Duration,
    pub rate: u32,
    pub channels: u16,
    pub volume: f32,
}


impl Sample {
    pub fn from_path(path_str: &str) -> Result<Self, Error> {
        let src_file = File::open(path_str).map_err(Error::IoError)?;
        let mss = MediaSourceStream::new(Box::new(src_file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = Path::new(path_str)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
        {
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

        Ok(Sample {
            data: all_samples,
            duration: calculated_duration,
            rate: sample_rate_u32,
            channels: channels_u16,
            volume: 1.0,
        })
    }
}



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
    pub fn mix(samples_to_mix: &[&Sample]) -> Result<Sample, MixerError> {
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

        Ok(Sample {
            data: mixed_data,
            duration: mixed_duration,
            rate: target_rate,
            channels: target_channels,
            volume: 1.0,
        })
    }
}