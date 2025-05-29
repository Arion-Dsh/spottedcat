use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, SampleFormat, StreamConfig};
// use std::sync::mpsc::{Receiver, Sender};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::Track;

/// Global counter for generating unique image IDs

#[derive(Debug, PartialEq, Clone)]
pub enum PlaybackCommand {
    Play,
    Pause,
    Stop,
    Seek(u32), // Seek to a specific sample index
}
pub struct Player {
    track_id_counter: u32,
    tracks: Arc<Mutex<HashMap<u32, Arc<Track>>>>, // Store tracks in HashMap for O(1) lookup
    command_senders: Vec<Sender<(u32, PlaybackCommand)>>, // Sender for (track_id, command)
    _stream: cpal::Stream,                        // Hold the stream to keep it alive
    #[allow(dead_code)]
    main_rx_command: Receiver<(u32, PlaybackCommand)>, // Receiver for commands from main thread
    #[allow(dead_code)]
    audio_tx_command: Sender<(u32, PlaybackCommand)>, // Sender for commands to audio callback
    output_config: StreamConfig,
}

impl Player {
    pub fn new() -> Result<Self, anyhow::Error> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("No default output device available");
        let config = device.default_output_config()?;

        println!("AudioEngine playing on device: {}", device.name()?);
        println!("AudioEngine default output stream config: {:?}", config);

        let sample_format = config.sample_format();
        let cpal_config = StreamConfig {
            channels: config.channels(), // Use device's default channels for output
            sample_rate: config.sample_rate(), // Use device's default sample rate
            buffer_size: BufferSize::Default,
        };

        let tracks: Arc<Mutex<HashMap<u32, Arc<Track>>>> = Arc::new(Mutex::new(HashMap::new()));
        let (main_tx_command, audio_rx_command): (
            Sender<(u32, PlaybackCommand)>,
            Receiver<(u32, PlaybackCommand)>,
        ) = unbounded();
        let (audio_tx_command, main_rx_command): (
            Sender<(u32, PlaybackCommand)>,
            Receiver<(u32, PlaybackCommand)>,
        ) = unbounded();

        let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

        let stream = match sample_format {
            SampleFormat::F32 => Self::build_float_multi_track_stream(
                &device,
                &cpal_config,
                tracks.clone(),
                audio_rx_command,
                audio_tx_command.clone(),
                err_fn,
            ),
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported sample format: {:?}",
                    sample_format
                ))
            }
        }?;
        stream.play()?;

        Ok(Player {
            track_id_counter: 0,
            tracks,
            command_senders: vec![main_tx_command], // This one sender handles all tracks now
            _stream: stream,
            main_rx_command,
            audio_tx_command,
            output_config: cpal_config,
        })
    }

    /// Starts the audio stream playback
    pub fn start_stream(&self) -> Result<(), anyhow::Error> {
        self._stream.play()?;
        Ok(())
    }

    /// Adds a new `Sample` as a track to the engine.
    /// Returns the ID of the newly added track.
    pub fn add_track(&mut self, mut track: Track) -> Result<u32, anyhow::Error> {
        println!("Adding track: {}", track.id);
        let mut tracks_guard = self.tracks.lock().unwrap();
        let track_id = self.track_id_counter.clone();
        self.track_id_counter += 1;

        let output_sample_rate = self.output_config.sample_rate.0;
        let output_channels = self.output_config.channels;

        // Handle sample rate conversion
        track.resample_linear(output_sample_rate)?;

        track.convert_channels(output_channels)?;

        tracks_guard.insert(track_id, Arc::new(track));

        Ok(track_id)
    }

    /// Sends a command to a specific track.
    pub fn send_command(
        &self,
        track_id: u32,
        command: PlaybackCommand,
    ) -> Result<(), anyhow::Error> {
        // We use the single sender from the Player to send (track_id, command)
        // to the audio callback.
        self.command_senders[0].send((track_id, command))?;
        Ok(())
    }

    /// Checks if all tracks are stopped.
    pub fn all_tracks_stopped(&self) -> bool {
        let tracks = self.tracks.lock().unwrap();
        tracks.values().all(|t| t.is_stopped())
    }

    /// Removes a track by its ID.
    /// Returns an error if the track ID is not found.
    pub fn remove_track(&self, track_id: u32) -> Result<(), anyhow::Error> {
        let mut tracks = self.tracks.lock().unwrap();
        if tracks.contains_key(&(track_id)) {

            // Remove the track
            tracks.remove(&(track_id as u32));
            Ok(())
        } else {
            Err(anyhow::anyhow!("Track with ID {} not found", track_id))
        }
    }

    // Helper functions for stream building (modified to handle multiple tracks)
    fn build_float_multi_track_stream(
        device: &cpal::Device,
        config: &StreamConfig,
        tracks: Arc<Mutex<HashMap<u32, Arc<Track>>>>,
        rx_command: Receiver<(u32, PlaybackCommand)>,
        tx_completion: Sender<(u32, PlaybackCommand)>, // For sending completion signals back
        err_fn: impl Fn(cpal::StreamError) + Send + 'static,
    ) -> Result<cpal::Stream, anyhow::Error> {
        let config = config.clone();
        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // Process commands for specific tracks
                while let Ok((track_id, command)) = rx_command.try_recv() {
                    let tracks_guard = tracks.lock().unwrap();
                    if let Some(track) = tracks_guard.get(&(track_id as u32)) {
                        track.handle_command(command.clone());
                        // If a track stops, send a completion signal
                        if command == PlaybackCommand::Stop {
                            let _ = tx_completion.send((track_id, PlaybackCommand::Stop));
                        }
                    } else {
                        eprintln!("Command for non-existent track ID: {}", track_id);
                    }
                }

                // Zero out the buffer before mixing
                for sample_out in data.iter_mut() {
                    *sample_out = 0.0;
                }

                // Sum samples from all active tracks
                let tracks_guard = tracks.lock().unwrap();
                let num_channels = config.channels;
                let mut active_tracks_count = 0; // Count tracks that are actually contributing audio

                for (_track_id, track) in tracks_guard.iter() {
                    if *track.playback_state.lock().unwrap() == PlaybackCommand::Play {
                        active_tracks_count += 1;
                        for i in 0..data.len() {
                            // Ensure the track has enough samples for the current output buffer chunk
                            // and that its channels match the output channels.
                            // If a track's channels don't match, you'd need more complex channel mapping/resampling.
                            // For simplicity, we assume channels match, or handle gracefully with 0.0.
                            if i < track.data.len() && track.channels() == num_channels as u16 {
                                data[i] += track.get_next_sample();
                            } else {
                                // If channels don't match or data runs out, ensure it contributes silence
                                // after its own data ends, or if its channels are incompatible.
                                // For simplicity, we'll let get_next_sample handle end-of-track silence.
                                data[i] = 0.0;
                            }
                        }
                    }
                }

                // Average the mixed samples to prevent clipping, only if there are active tracks
                if active_tracks_count > 0 {
                    let divisor = active_tracks_count as f32; // Average by number of tracks actively playing
                    for sample_out in data.iter_mut() {
                        *sample_out /= divisor;
                    }
                }

                // Optional: Apply a master normalization if needed after mixing multiple tracks
                // This is a simple soft-clipping or limiting,
                // A full compressor/limiter would be more advanced.
                // Normalization Step
                let mut max_amplitude = 0.0f32;
                for &value in data.iter() {
                    let abs_value = value.abs();
                    if abs_value > max_amplitude {
                        max_amplitude = abs_value;
                    }
                }

                if max_amplitude > 1.0f32 {
                    // 只有在检测到可能削波时才归一化
                    let normalization_factor = 1.0f32 / max_amplitude;
                    for sample_value in data.iter_mut() {
                        *sample_value *= normalization_factor;
                    }
                }

                for sample_out in data.iter_mut() {
                    *sample_out = sample_out.max(-1.0).min(1.0);
                }
            },
            err_fn,
            None,
        )?;
        Ok(stream)
    }
}
