use crate::fm_synth::SynthEngine;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub struct AudioEngine {
    _stream: cpal::Stream,
    _underrun_counter: Arc<AtomicUsize>,
}

impl AudioEngine {
    pub fn new(engine: Arc<Mutex<SynthEngine>>, underrun_counter: Arc<AtomicUsize>) -> Self {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("No output device available");

        let config = device
            .default_output_config()
            .expect("Failed to get default output config");

        let sample_rate = config.sample_rate().0;

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                Self::build_stream::<f32>(&device, &config.into(), engine, underrun_counter.clone())
            }
            cpal::SampleFormat::I16 => {
                Self::build_stream::<i16>(&device, &config.into(), engine, underrun_counter.clone())
            }
            cpal::SampleFormat::U16 => {
                Self::build_stream::<u16>(&device, &config.into(), engine, underrun_counter.clone())
            }
            format => panic!("Unsupported sample format: {:?}", format),
        };

        stream.play().expect("Failed to start audio stream");

        log::info!(
            "Audio engine initialized with {} Hz sample rate",
            sample_rate
        );

        Self {
            _stream: stream,
            _underrun_counter: underrun_counter,
        }
    }

    /// Get the default sample rate from the audio device
    pub fn get_default_sample_rate() -> f32 {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("No output device available");

        let config = device
            .default_output_config()
            .expect("Failed to get default output config");

        config.sample_rate().0 as f32
    }

    fn build_stream<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        engine: Arc<Mutex<SynthEngine>>,
        underrun_counter: Arc<AtomicUsize>,
    ) -> cpal::Stream
    where
        T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
    {
        let channels = config.channels as usize;
        let mut samples_since_snapshot = 0u32;
        let snapshot_interval = 1024; // Update snapshot every N samples

        device
            .build_output_stream(
                config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    match engine.try_lock() {
                        Ok(mut synth) => {
                            // Process commands at the start of each buffer
                            synth.process_commands();

                            for frame in data.chunks_mut(channels) {
                                let (left, right) = synth.process_stereo();

                                if channels >= 2 {
                                    frame[0] = T::from_sample(left);
                                    frame[1] = T::from_sample(right);
                                } else {
                                    frame[0] = T::from_sample((left + right) * 0.5);
                                }

                                samples_since_snapshot += 1;
                            }

                            // Update snapshot periodically (not every sample)
                            if samples_since_snapshot >= snapshot_interval {
                                synth.update_snapshot();
                                samples_since_snapshot = 0;
                            }
                        }
                        Err(_) => {
                            let underrun_count = underrun_counter.fetch_add(1, Ordering::Relaxed);
                            if underrun_count.is_multiple_of(500) {
                                log::warn!(
                                    "AUDIO WARNING: {} buffer underruns detected",
                                    underrun_count
                                );
                            }

                            for frame in data.chunks_mut(channels) {
                                let value = T::from_sample(0.0);
                                for channel_sample in frame.iter_mut() {
                                    *channel_sample = value;
                                }
                            }
                        }
                    }
                },
                |err| log::error!("Audio stream error: {}", err),
                None,
            )
            .expect("Failed to build output stream")
    }
}
