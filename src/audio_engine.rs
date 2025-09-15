use crate::fm_synth::FmSynthesizer;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub struct AudioEngine {
    _stream: cpal::Stream,
    _underrun_counter: Arc<AtomicUsize>,
}

impl AudioEngine {
    pub fn new_with_synth_setup() -> (Self, Arc<Mutex<FmSynthesizer>>) {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("No output device available");

        let config = device
            .default_output_config()
            .expect("Failed to get default output config");

        let sample_rate = config.sample_rate().0 as f32;

        // Create synthesizer with correct sample rate
        let synthesizer = Arc::new(Mutex::new(FmSynthesizer::new_with_sample_rate(sample_rate)));

        let underrun_counter = Arc::new(AtomicUsize::new(0));
        let audio_engine = Self::new(synthesizer.clone(), underrun_counter.clone());

        (audio_engine, synthesizer)
    }

    pub fn new(synthesizer: Arc<Mutex<FmSynthesizer>>, underrun_counter: Arc<AtomicUsize>) -> Self {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("No output device available");

        let config = device
            .default_output_config()
            .expect("Failed to get default output config");

        let sample_rate = config.sample_rate().0;

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => Self::build_stream::<f32>(
                &device,
                &config.into(),
                synthesizer,
                underrun_counter.clone(),
            ),
            cpal::SampleFormat::I16 => Self::build_stream::<i16>(
                &device,
                &config.into(),
                synthesizer,
                underrun_counter.clone(),
            ),
            cpal::SampleFormat::U16 => Self::build_stream::<u16>(
                &device,
                &config.into(),
                synthesizer,
                underrun_counter.clone(),
            ),
            _ => panic!("Unsupported sample format"),
        };

        stream.play().expect("Failed to start audio stream");

        println!(
            "Audio engine initialized with {} Hz sample rate",
            sample_rate
        );

        Self {
            _stream: stream,
            _underrun_counter: underrun_counter,
        }
    }

    fn build_stream<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        synthesizer: Arc<Mutex<FmSynthesizer>>,
        underrun_counter: Arc<AtomicUsize>,
    ) -> cpal::Stream
    where
        T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
    {
        let channels = config.channels as usize;

        device
            .build_output_stream(
                config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    // Use try_lock but with better limiting and reduced dropouts
                    match synthesizer.try_lock() {
                        Ok(mut synth) => {
                            for frame in data.chunks_mut(channels) {
                                let sample = synth.process();
                                // Apply better soft limiting to prevent clipping
                                let limited_sample = Self::soft_limit(sample);
                                let value = T::from_sample(limited_sample);

                                for channel_sample in frame.iter_mut() {
                                    *channel_sample = value;
                                }
                            }
                        }
                        Err(_) => {
                            // Reduced underrun logging frequency for less console spam
                            let underrun_count = underrun_counter.fetch_add(1, Ordering::Relaxed);
                            if underrun_count % 500 == 0 {
                                eprintln!(
                                    "AUDIO WARNING: {} buffer underruns detected",
                                    underrun_count
                                );
                            }

                            // Fill with silence
                            for frame in data.chunks_mut(channels) {
                                let value = T::from_sample(0.0);
                                for channel_sample in frame.iter_mut() {
                                    *channel_sample = value;
                                }
                            }
                        }
                    }
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            )
            .expect("Failed to build output stream")
    }

    /// Improved soft limiting using a smooth S-curve instead of harsh tanh
    fn soft_limit(sample: f32) -> f32 {
        const THRESHOLD: f32 = 0.8;  // Start limiting at 80% to prevent harsh clipping
        const KNEE: f32 = 0.1;      // Smooth knee transition

        if sample.abs() <= THRESHOLD {
            sample
        } else {
            let sign = sample.signum();
            let abs_sample = sample.abs();

            // Smooth compression above threshold
            let excess = abs_sample - THRESHOLD;
            let compressed_excess = excess / (1.0 + excess / KNEE);
            let limited = THRESHOLD + compressed_excess;

            // Final hard limit to prevent any overshoot
            sign * limited.min(0.95)
        }
    }
}
